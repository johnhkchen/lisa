#!/usr/bin/env bash
set -euo pipefail

# Rehearse a soak promotion against a throwaway tap (T-069-01-03).
#
# The decision -- which release has soaked, which has been superseded, which was
# yanked -- is Rust, and `cargo test` covers it. What this covers is the half
# that is shell and git: given a promotion pointer, does the tap end up with
# lisa-nightly on that release, does the release publish leave lisa-nightly
# alone, and does a run with nothing to say leave no commit behind it.
#
# That last one is the criterion this exists for. A tap whose history is full of
# no-op commits is one nobody will read during an incident, and it is the kind
# of regression that is invisible until someone goes looking during one.
#
# No network, no Homebrew, no release artifacts: `brew style` is stubbed out
# here, and the real thing runs in the release workflow's tap verification.

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
publisher="$repo_root/scripts/publish-tap-formulae.sh"
committer="$repo_root/scripts/commit-tap-formulae.sh"

released=9.9.9-rc.1 # the newest release: a candidate, so canary and never stable
soaked=9.9.8        # what nightly carries before the promotion

fail() {
    echo "nightly promotion verification failed: $*" >&2
    exit 1
}

for tool in git awk; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"
done
[[ -s $publisher ]] || fail "missing formula generator: $publisher"
[[ -s $committer ]] || fail "missing tap committer: $committer"

work=$(mktemp -d "${TMPDIR:-/tmp}/lisa-promotion.XXXXXX")
trap 'rm -rf "$work"' EXIT

# A stand-in for `brew style --fix`, which the committer runs before it asks git
# whether anything moved. Styling real formulae needs a real Homebrew; what is
# being checked here is what git is asked afterwards.
mkdir -p "$work/bin"
cat >"$work/bin/brew" <<'STUB'
#!/usr/bin/env bash
exit 0
STUB
chmod +x "$work/bin/brew"
export PATH="$work/bin:$PATH"

# A formula shaped like the one cargo-dist writes: the generator renames the
# class, appends the channel to the description, and anchors conflicts_with on
# the license line, so those three have to be here.
formula_for() {
    local version=$1 path=$2
    cat >"$path" <<RB
class Lisa < Formula
  desc "Runs my coding agents"
  homepage "https://github.com/johnhkchen/lisa"
  version "$version"
  license "MIT"

  def install
    bin.install "lisa"
  end
end
RB
}

formula_for "$released" "$work/release.rb"
formula_for "$soaked" "$work/soaked.rb"

tap="$work/tap"
mkdir -p "$tap"
git -C "$tap" init --quiet
git -C "$tap" config user.email "verify@example.invalid"
git -C "$tap" config user.name "tap verification"

version_of() {
    awk -F'"' '/^  version "/ { print $2; exit }' "$1"
}

commits_in() {
    git -C "$tap" rev-list --count HEAD 2>/dev/null || echo 0
}

expect_version() {
    local formula=$1 want=$2 got
    got=$(version_of "$tap/Formula/$formula.rb" 2>/dev/null || true)
    [[ $got == "$want" ]] ||
        fail "$formula should be $want, is ${got:-absent}"
}

# 1. A release publish: canary takes the candidate, stable refuses it, and
#    nightly carries whatever the promotion pointer named.
bash "$publisher" "$tap" "$work/release.rb" true "$work/soaked.rb" >/dev/null
expect_version lisa-canary "$released"
expect_version lisa-nightly "$soaked"
[[ ! -e "$tap/Formula/lisa.rb" ]] ||
    fail "a release candidate reached the stable formula"

bash "$committer" "$tap" >/dev/null
first=$(commits_in)
[[ $first -eq 2 ]] || fail "the first publish should commit two formulae, committed $first"

# 2. The same publish again. Nothing moved, so nothing is committed: this is the
#    no-op criterion, and it has to hold for the release path as well as the
#    promotion one.
bash "$publisher" "$tap" "$work/release.rb" true "$work/soaked.rb" >/dev/null
bash "$committer" "$tap" >/dev/null
[[ $(commits_in) -eq $first ]] ||
    fail "a publish with nothing to say left $(($(commits_in) - first)) commit(s) in the tap"

# 3. The promotion: the pointer now names the candidate, and the release half of
#    the run is unchanged. Only lisa-nightly may move.
canary_before=$(git -C "$tap" log -1 --format=%H -- Formula/lisa-canary.rb)
note="promoted after a 24h soak; superseded nothing"
bash "$publisher" "$tap" "$work/release.rb" true "$work/release.rb" >/dev/null
bash "$committer" "$tap" "$note" >/dev/null

expect_version lisa-nightly "$released"
expect_version lisa-canary "$released"
[[ $(commits_in) -eq $((first + 1)) ]] ||
    fail "a promotion should leave exactly one commit, left $(($(commits_in) - first))"
[[ $(git -C "$tap" log -1 --format=%H -- Formula/lisa-canary.rb) == "$canary_before" ]] ||
    fail "a promotion rewrote lisa-canary"

# 4. The commit says which formula moved and why, because that line is where a
#    person reads what nightly is on without asking a machine.
subject=$(git -C "$tap" log -1 --format=%s)
[[ $subject == "lisa-nightly $released" ]] ||
    fail "the promotion commit subject should name the formula and version, is: $subject"
git -C "$tap" log -1 --format=%b | grep -Fq "$note" ||
    fail "the promotion commit does not carry the reason it moved"

# 5. And a stable release afterwards writes lisa, leaves nightly where the
#    promotion put it, and needs no second promotion to do it.
formula_for 9.9.10 "$work/stable.rb"
bash "$publisher" "$tap" "$work/stable.rb" false "$work/release.rb" >/dev/null
bash "$committer" "$tap" >/dev/null
expect_version lisa 9.9.10
expect_version lisa-canary 9.9.10
expect_version lisa-nightly "$released"

echo "nightly promotion verification passed"
