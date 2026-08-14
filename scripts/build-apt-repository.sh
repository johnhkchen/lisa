#!/usr/bin/env bash
set -euo pipefail

# Three suites, one pool, one key. `stable`, `nightly` and `canary` are the
# channel: a box joins one by writing that word into
# /etc/apt/sources.list.d/lisa.list. Every suite indexes a subset of the same
# shared pool and is signed by the same archive key, so changing channel never
# means trusting a second key, and every version any channel has ever carried
# stays installable by exact version.

suites=(stable nightly canary)

suite_description() {
    case "$1" in
        stable) echo "Lisa stable apt channel: released versions only" ;;
        nightly) echo "Lisa nightly apt channel: releases that have soaked" ;;
        canary) echo "Lisa canary apt channel: every release, candidates included" ;;
        *) echo "Lisa apt channel" ;;
    esac
}

usage() {
    echo "Usage: $0 SUITE_INPUT_ROOT OUTPUT_DIR SIGNING_FINGERPRINT PUBLIC_KEY" >&2
    echo "SUITE_INPUT_ROOT holds one directory per suite: ${suites[*]}" >&2
    exit 2
}

fail() {
    echo "Apt repository build failed: $*" >&2
    exit 1
}

[[ $# -eq 4 ]] || usage

input_arg=$1
output_arg=${2%/}
signing_fingerprint=${3//[[:space:]]/}
public_key_arg=$4

[[ -d $input_arg ]] || fail "Debian input directory does not exist: $input_arg"
[[ -f $public_key_arg ]] || fail "public key does not exist: $public_key_arg"
[[ $signing_fingerprint =~ ^[0-9A-Fa-f]{40}$ ]] ||
    fail "signing fingerprint must contain exactly 40 hexadecimal characters"
signing_fingerprint=${signing_fingerprint^^}

for command in apt-ftparchive awk dpkg-deb find gpg grep gzip install mktemp sha256sum sort; do
    command -v "$command" >/dev/null 2>&1 || fail "required command is missing: $command"
done

input_dir=$(cd "$input_arg" && pwd -P)
public_key=$(cd "$(dirname "$public_key_arg")" && pwd -P)/$(basename "$public_key_arg")
mkdir -p "$(dirname "$output_arg")"
output_parent=$(cd "$(dirname "$output_arg")" && pwd -P)
output_name=$(basename "$output_arg")
[[ -n $output_name && $output_name != . && $output_name != .. ]] ||
    fail "unsafe output directory: $output_arg"
output_dir="$output_parent/$output_name"

case "$output_dir/" in
    "$input_dir/"*) fail "output directory must not be inside the input directory" ;;
esac
case "$input_dir/" in
    "$output_dir/"*) fail "input directory must not be inside the output directory" ;;
esac

for suite in "${suites[@]}"; do
    [[ -d "$input_dir/$suite" ]] ||
        fail "input root is missing the $suite suite directory: $input_dir/$suite"
done

public_fingerprints=$(gpg --batch --with-colons --show-keys "$public_key" 2>/dev/null |
    awk -F: '$1 == "fpr" { print toupper($10) }')
public_primary_count=$(gpg --batch --with-colons --show-keys "$public_key" 2>/dev/null |
    awk -F: '$1 == "pub" { count++ } END { print count + 0 }')
[[ $public_primary_count -eq 1 ]] || fail "public key must contain exactly one primary key"
grep -Fxq "$signing_fingerprint" <<<"$public_fingerprints" ||
    fail "public key does not contain signing fingerprint $signing_fingerprint"

secret_fingerprints=$(gpg --batch --with-colons --list-secret-keys "$signing_fingerprint" 2>/dev/null |
    awk -F: '$1 == "fpr" { print toupper($10) }')
grep -Fxq "$signing_fingerprint" <<<"$secret_fingerprints" ||
    fail "GNUPGHOME does not contain signing key $signing_fingerprint"

stage=$(mktemp -d "$output_parent/.lisa-apt-repository.XXXXXX")
work=$(mktemp -d "$output_parent/.lisa-apt-work.XXXXXX")
verify_home=
cleanup() {
    rm -rf "$stage" "$work"
    if [[ -n $verify_home ]]; then
        rm -rf "$verify_home"
    fi
}
trap cleanup EXIT

pool="$stage/pool/main/l/lisa"
mkdir -p "$pool"

# The pool is shared by every suite. A version that entered through canary is
# still on disk after it has been promoted or superseded, which is what makes
# `apt-get install lisa=<version>` a real rollback with no extra machinery.
declare -A identities=()
declare -A membership=()
package_count=0

for suite in "${suites[@]}"; do
    suite_count=0
    declare -A required=()

    while IFS= read -r -d '' package; do
        name=$(dpkg-deb --field "$package" Package)
        version=$(dpkg-deb --field "$package" Version)
        architecture=$(dpkg-deb --field "$package" Architecture)

        case "$name" in
            lisa|lisa-runtime-zellij) ;;
            *) fail "unexpected package name '$name' in $package" ;;
        esac
        case "$architecture" in
            amd64|arm64) ;;
            *) fail "unexpected architecture '$architecture' in $package" ;;
        esac
        [[ -n $version && $version =~ ^[0-9A-Za-z.+~_-]+$ ]] ||
            fail "unsafe or empty package version '$version' in $package"

        identity="${name}_${version}_${architecture}.deb"
        checksum=$(sha256sum "$package" | awk '{ print $1 }')
        if [[ -n ${identities[$identity]:-} ]]; then
            [[ ${identities[$identity]} == "$checksum" ]] ||
                fail "conflicting package bytes for $identity"
        else
            install -m 0644 "$package" "$pool/$identity"
            identities[$identity]=$checksum
            ((package_count += 1))
        fi

        required["$name:$architecture"]=1
        if [[ ${membership["$suite:$identity"]:-} != 1 ]]; then
            membership["$suite:$identity"]=1
            ((suite_count += 1))
        fi
    done < <(find "$input_dir/$suite" -type f -name '*.deb' -print0 | sort -z)

    [[ $suite_count -gt 0 ]] || fail "no Debian packages found beneath $input_dir/$suite"
    # lisa-runtime-zellij carries the same version as lisa and pins the Zellij
    # that release was built against, so it rides in every suite next to the
    # lisa it belongs to. A single stable runtime shared by all three would let
    # apt pair a canary lisa with a stale runtime, because lisa only
    # *recommends* it and does so without a version.
    for architecture in amd64 arm64; do
        for name in lisa lisa-runtime-zellij; do
            [[ ${required["$name:$architecture"]:-} == 1 ]] ||
                fail "the $suite suite is missing $name for $architecture"
        done
    done
    unset required
done

[[ $package_count -gt 0 ]] || fail "no Debian packages found beneath $input_dir"

# Index the whole pool once per architecture, then cut each suite's index out of
# it. A suite sees a package only when that suite's input carried it, so a box
# pinned to stable never sees a release candidate sitting in canary even though
# the candidate's bytes are in the same pool.
for architecture in amd64 arm64; do
    (
        cd "$stage"
        apt-ftparchive --arch "$architecture" packages pool
    ) > "$work/Packages.$architecture"
done

for suite in "${suites[@]}"; do
    members="$work/members-$suite.txt"
    for key in "${!membership[@]}"; do
        case "$key" in
            "$suite:"*) printf '%s\n' "${key#"$suite":}" ;;
        esac
    done | sort > "$members"

    for architecture in amd64 arm64; do
        index_dir="$stage/dists/$suite/main/binary-$architecture"
        mkdir -p "$index_dir"

        awk -v listfile="$members" '
            function base(path,   count, parts) {
                count = split(path, parts, "/")
                return parts[count]
            }
            BEGIN {
                while ((getline line < listfile) > 0) {
                    if (line != "") want[line] = 1
                }
                close(listfile)
                RS = ""
                FS = "\n"
            }
            {
                keep = 0
                for (field = 1; field <= NF; field++) {
                    if (substr($field, 1, 10) == "Filename: " &&
                        base(substr($field, 11)) in want) {
                        keep = 1
                    }
                }
                if (keep) {
                    if (emitted) printf "\n"
                    printf "%s\n", $0
                    emitted = 1
                }
            }
        ' "$work/Packages.$architecture" > "$index_dir/Packages"

        expected=$(grep -c "_${architecture}\.deb\$" "$members" || true)
        actual=$(grep -c '^Filename: ' "$index_dir/Packages" || true)
        [[ $actual -eq $expected ]] ||
            fail "$suite $architecture index lists $actual packages, expected $expected"
        for name in lisa lisa-runtime-zellij; do
            grep -Fxq "Package: $name" "$index_dir/Packages" ||
                fail "$suite $architecture Packages index is missing $name"
        done
        gzip -n -9 -c "$index_dir/Packages" > "$index_dir/Packages.gz"
    done

    (
        cd "$stage"
        apt-ftparchive \
            -o APT::FTPArchive::Release::Origin=Lisa \
            -o APT::FTPArchive::Release::Label=Lisa \
            -o APT::FTPArchive::Release::Suite="$suite" \
            -o APT::FTPArchive::Release::Codename="$suite" \
            -o APT::FTPArchive::Release::Architectures="amd64 arm64" \
            -o APT::FTPArchive::Release::Components=main \
            -o APT::FTPArchive::Release::Description="$(suite_description "$suite")" \
            release "dists/$suite"
    ) > "$stage/dists/$suite/Release"

    gpg --batch --yes --armor --local-user "${signing_fingerprint}!" \
        --digest-algo SHA256 --clearsign \
        --output "$stage/dists/$suite/InRelease" "$stage/dists/$suite/Release"
    gpg --batch --yes --armor --local-user "${signing_fingerprint}!" \
        --digest-algo SHA256 --detach-sign \
        --output "$stage/dists/$suite/Release.gpg" "$stage/dists/$suite/Release"
done

verify_home=$(mktemp -d "$output_parent/.lisa-apt-verify.XXXXXX")
chmod 0700 "$verify_home"
GNUPGHOME=$verify_home gpg --batch --quiet --import "$public_key"
for suite in "${suites[@]}"; do
    GNUPGHOME=$verify_home gpg --batch --verify \
        "$stage/dists/$suite/InRelease" >/dev/null 2>&1 ||
        fail "the $suite InRelease does not verify against the archive key"
    GNUPGHOME=$verify_home gpg --batch --verify \
        "$stage/dists/$suite/Release.gpg" "$stage/dists/$suite/Release" >/dev/null 2>&1 ||
        fail "the $suite Release.gpg does not verify against the archive key"
done

install -m 0644 "$public_key" "$stage/lisa-archive-keyring.asc"
: > "$stage/.nojekyll"

# A published static apt site is world-readable by definition, and apt's
# sandboxed `_apt` user must be able to read every index it fetches — gpg
# writes some outputs 0600 regardless of umask, which broke file: repo
# verification (v0.4.2 release run 29550709595). Normalize the whole site.
chmod -R a+rX "$stage"

rm -rf "$output_dir"
mv "$stage" "$output_dir"
stage="$output_parent/.lisa-apt-repository.consumed"

echo "Built signed Lisa apt repository with $package_count pooled packages" \
    "across suites ${suites[*]} at $output_dir"
