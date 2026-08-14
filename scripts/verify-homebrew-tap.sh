#!/usr/bin/env bash
set -euo pipefail

# Rehearse the three-channel Homebrew tap against the just-built release
# artifacts, the way verify-shell-installer.sh and verify-apt-repository.sh
# rehearse their own channels (T-069-01-01).
#
# Two things are checked, and they are different things:
#
#   routing   which formula files a release writes. A prerelease reaches
#             lisa-canary and never lisa; a release never rewrites lisa-nightly,
#             which belongs to the soak promotion.
#   install   a real Homebrew, in a throwaway prefix, installing each formula
#             from a local copy of this build's archive: the version it lands
#             on, the lisa binary it puts on PATH, and brew refusing the second
#             channel instead of letting PATH order decide.

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
artifacts_dir=${1:-"$repo_root/target/distrib"}
publisher="$repo_root/scripts/publish-tap-formulae.sh"
# A version no real release can carry, so the routing assertions cannot pass by
# accident when a formula was left alone.
rehearsal_prerelease_version=999.0.0-rc.1

fail() {
    echo "Homebrew tap verification failed: $*" >&2
    exit 1
}

for tool in git tar awk; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"
done

if command -v sha256sum >/dev/null 2>&1; then
    sha256_of() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    fail "sha256sum or shasum is required"
fi

[[ -x $publisher ]] || fail "missing formula generator: $publisher"
[[ -d $artifacts_dir ]] || fail "artifact directory does not exist: $artifacts_dir"
artifacts_dir=$(cd "$artifacts_dir" && pwd -P)

dist_formula="$artifacts_dir/lisa.rb"
[[ -s $dist_formula ]] || fail "missing cargo-dist formula: $dist_formula"

release_version=$(awk -F'"' '/^  version "/ { print $2; exit }' "$dist_formula")
[[ -n $release_version ]] || fail "could not read the version out of $dist_formula"

case "$(uname -s)-$(uname -m)" in
Darwin-arm64) host_triple=aarch64-apple-darwin ;;
Darwin-x86_64) host_triple=x86_64-apple-darwin ;;
Linux-x86_64) host_triple=x86_64-unknown-linux-musl ;;
Linux-aarch64 | Linux-arm64) host_triple=aarch64-unknown-linux-musl ;;
*) fail "unsupported host for tap verification: $(uname -s)-$(uname -m)" ;;
esac

host_archive="$artifacts_dir/lisa-cli-$host_triple.tar.gz"
[[ -s $host_archive ]] || fail "missing release archive for this host: $host_archive"

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/lisa-homebrew-verification.XXXXXX")
cleanup() {
    chmod -R u+w "$work_dir" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT

formula_version() {
    awk -F'"' '/^  version "/ { print $2; exit }' "$1"
}

# ---------------------------------------------------------------------------
# Routing: which files a release writes, and which it must leave alone.
# ---------------------------------------------------------------------------

prerelease_formula="$work_dir/lisa-prerelease.rb"
sed "s/^  version \".*\"\$/  version \"$rehearsal_prerelease_version\"/" \
    "$dist_formula" >"$prerelease_formula"
[[ $(formula_version "$prerelease_formula") == "$rehearsal_prerelease_version" ]] ||
    fail "could not build the prerelease rehearsal formula"

: >"$work_dir/routing.log"
route() {
    local tap=$1 release_formula=$2 prerelease=$3 nightly_formula=$4
    "$publisher" "$tap" "$release_formula" "$prerelease" "$nightly_formula" |
        tee -a "$work_dir/routing.log"
}

# A release that is not a prerelease, with nothing promoted yet: nightly carries
# what stable carries, which is what the promotion pointer resolves to before
# anything has been promoted.
release_tap="$work_dir/tap"
mkdir -p "$release_tap"
route "$release_tap" "$dist_formula" false "$dist_formula" >/dev/null

for name in lisa lisa-nightly lisa-canary; do
    [[ -s $release_tap/Formula/$name.rb ]] || fail "a stable release did not write Formula/$name.rb"
    [[ $(formula_version "$release_tap/Formula/$name.rb") == "$release_version" ]] ||
        fail "Formula/$name.rb did not land on $release_version"
done

# The behaviour change this ticket is about: with a stable tap already in place,
# the next release candidate moves canary and touches nothing else. Nightly is
# handed the same source as before, standing in for a promotion that has not
# happened.
rc_tap="$work_dir/tap-after-rc"
mkdir -p "$rc_tap/Formula"
cp "$release_tap"/Formula/*.rb "$rc_tap/Formula/"
rc_log=$(route "$rc_tap" "$prerelease_formula" true "$dist_formula")

[[ $(formula_version "$rc_tap/Formula/lisa-canary.rb") == "$rehearsal_prerelease_version" ]] ||
    fail "a release candidate did not reach lisa-canary"
[[ $(formula_version "$rc_tap/Formula/lisa.rb") == "$release_version" ]] ||
    fail "a release candidate moved lisa; stable must stay on $release_version"
[[ $(formula_version "$rc_tap/Formula/lisa-nightly.rb") == "$release_version" ]] ||
    fail "a release candidate moved lisa-nightly; only the soak promotion may move it"
grep -Fq "unchanged Formula/lisa-nightly.rb" <<<"$rc_log" ||
    fail "a promotion with nothing to do rewrote lisa-nightly.rb; the tap's history must stay readable"

# And a prerelease into an empty tap leaves no stable formula at all, rather
# than seeding one from a release candidate.
fresh_tap="$work_dir/tap-from-prerelease"
mkdir -p "$fresh_tap"
route "$fresh_tap" "$prerelease_formula" true "$prerelease_formula" >/dev/null
[[ ! -e $fresh_tap/Formula/lisa.rb ]] ||
    fail "a prerelease wrote Formula/lisa.rb; stable must never take a release candidate"
for name in lisa-canary lisa-nightly; do
    [[ -s $fresh_tap/Formula/$name.rb ]] || fail "a prerelease did not write Formula/$name.rb"
done

cat "$work_dir/routing.log"

for name in lisa lisa-nightly lisa-canary; do
    grep -Fq 'conflicts_with' "$release_tap/Formula/$name.rb" ||
        fail "Formula/$name.rb declares no conflicts_with"
    grep -Fq 'Do not edit by hand' "$release_tap/Formula/$name.rb" ||
        fail "Formula/$name.rb is not marked as generated"
done

echo "routing verified: a prerelease reaches lisa-canary only, and never lisa or lisa-nightly"

# ---------------------------------------------------------------------------
# Install: a real Homebrew in a throwaway prefix, serving this build's archive.
# ---------------------------------------------------------------------------

# Homebrew resolves conflicts_with by loading the sibling formula, so all three
# must live in one tap checkout. Point every url this host would fetch at the
# local archive instead of a GitHub release that does not exist yet.
prefix="$work_dir/prefix"
tap_dir="$work_dir/rehearsal-tap"
mkdir -p "$tap_dir/Formula"

host_archive_sha=$(sha256_of "$host_archive")
for name in lisa lisa-nightly lisa-canary; do
    awk -v archive="$host_archive" -v digest="$host_archive_sha" \
        -v basename="lisa-cli-$host_triple.tar.gz" '
        index($0, "/" basename "\"") > 0 && $0 ~ /^ *url "/ {
            sub(/url ".*"/, "url \"file://" archive "\"")
            print
            pending = 1
            next
        }
        pending == 1 && $0 ~ /^ *sha256 "/ {
            sub(/sha256 ".*"/, "sha256 \"" digest "\"")
            print
            pending = 0
            rewritten = 1
            next
        }
        { print }
        END {
            if (rewritten != 1) {
                print "no url/sha256 pair rewritten for " basename > "/dev/stderr"
                exit 1
            }
        }
    ' "$release_tap/Formula/$name.rb" >"$tap_dir/Formula/$name.rb" ||
        fail "could not point Formula/$name.rb at the local archive"
done

git -C "$tap_dir" init -q .
git -C "$tap_dir" add -A
git -C "$tap_dir" -c user.email=tap-verification@example.invalid \
    -c user.name='Lisa Tap Verification' commit -qm "rehearsal $release_version"

git clone --quiet --depth=1 https://github.com/Homebrew/brew "$prefix" ||
    fail "could not clone Homebrew into the throwaway prefix"
mkdir -p "$prefix/Library/Taps/johnhkchen"
mv "$tap_dir" "$prefix/Library/Taps/johnhkchen/homebrew-lisa"

export HOMEBREW_NO_ANALYTICS=1
export HOMEBREW_NO_AUTO_UPDATE=1
export HOMEBREW_NO_ENV_HINTS=1
export HOMEBREW_NO_EMOJI=1
export HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK=1
export HOMEBREW_CACHE="$work_dir/cache"
export HOMEBREW_LOGS="$work_dir/logs"
# Homebrew refuses a prefix that sits inside its own temporary directory, and on
# macOS that directory defaults to /private/tmp -- where mktemp -d puts us.
export HOMEBREW_TEMP="$work_dir/temp"
# Tap trust is recorded per user, in $HOME/.homebrew/trust.json, not per prefix.
# Without this the rehearsal reads -- and writes -- the trust list of whoever is
# running it, and passes on a machine that already trusts the tap.
export HOME="$work_dir/home"
mkdir -p "$HOMEBREW_TEMP" "$HOME"
unset HOMEBREW_PREFIX HOMEBREW_CELLAR HOMEBREW_REPOSITORY HOMEBREW_LIBRARY
brew="$prefix/bin/brew"
[[ -x $brew ]] || fail "the throwaway Homebrew has no bin/brew"

# Homebrew requires a non-official tap to be trusted before it will load a
# formula that was not named on the command line, and conflicts_with loads the
# two sibling channels. So `brew trust johnhkchen/lisa` is part of installing
# Lisa from the tap, and the README says so. Prove it is really needed rather
# than repeating it as folklore -- but if Homebrew stops needing it, say so
# instead of failing a release over an instruction that became unnecessary.
set +e
untrusted_output=$("$brew" install johnhkchen/lisa/lisa 2>&1)
untrusted_status=$?
set -e
if [[ $untrusted_status -eq 0 ]]; then
    echo "notice: brew installed from the untrusted tap; the README's 'brew trust' step may no longer be required" >&2
    "$brew" uninstall johnhkchen/lisa/lisa >/dev/null 2>&1 || true
else
    grep -Fq 'untrusted tap johnhkchen/lisa' <<<"$untrusted_output" ||
        {
            printf '%s\n' "$untrusted_output" >&2
            fail "brew refused the untrusted tap for some reason other than trust"
        }
    echo "untrusted tap refused, as the README's 'brew trust johnhkchen/lisa' step says it is"
fi

"$brew" trust johnhkchen/lisa >"$work_dir/trust.log" 2>&1 ||
    {
        cat "$work_dir/trust.log" >&2
        fail "brew trust johnhkchen/lisa failed; the README's install instructions cannot work"
    }

for channel in lisa lisa-nightly lisa-canary; do
    "$brew" install "johnhkchen/lisa/$channel" >"$work_dir/install-$channel.log" 2>&1 ||
        {
            cat "$work_dir/install-$channel.log" >&2
            fail "brew install $channel failed"
        }

    [[ -x $prefix/bin/lisa ]] ||
        fail "$channel did not put a lisa binary on the prefix's PATH"
    installed=$("$prefix/bin/lisa" --version 2>&1) ||
        fail "the lisa installed by $channel would not run"
    case "$installed" in
    *"$release_version"*) ;;
    *) fail "$channel landed on '$installed' instead of $release_version" ;;
    esac
    echo "installed $channel -> $installed"

    for other in lisa lisa-nightly lisa-canary; do
        [[ $other == "$channel" ]] && continue
        set +e
        conflict_output=$("$brew" install "johnhkchen/lisa/$other" 2>&1)
        conflict_status=$?
        set -e
        if [[ $conflict_status -eq 0 ]]; then
            fail "brew installed $other alongside $channel; the channels must conflict"
        fi
        grep -Fq 'conflicting formulae are installed' <<<"$conflict_output" ||
            {
                printf '%s\n' "$conflict_output" >&2
                fail "brew refused $other alongside $channel without naming the conflict"
            }
        grep -Fq "$channel:" <<<"$conflict_output" ||
            {
                printf '%s\n' "$conflict_output" >&2
                fail "brew's refusal of $other did not name $channel as the installed channel"
            }
        echo "  brew refuses $other while $channel is installed"
    done

    "$brew" uninstall "johnhkchen/lisa/$channel" >>"$work_dir/install-$channel.log" 2>&1 ||
        fail "brew uninstall $channel failed"
done

echo "Verified the three-channel tap: lisa, lisa-nightly and lisa-canary each install $release_version, and brew refuses the second one"
