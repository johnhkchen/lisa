#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $0 DEB_INPUT_DIR OUTPUT_DIR SIGNING_FINGERPRINT PUBLIC_KEY" >&2
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
verify_home=
cleanup() {
    rm -rf "$stage"
    if [[ -n $verify_home ]]; then
        rm -rf "$verify_home"
    fi
}
trap cleanup EXIT

pool="$stage/pool/main/l/lisa"
mkdir -p "$pool"

declare -A identities=()
declare -A required=()
package_count=0

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
        continue
    fi

    install -m 0644 "$package" "$pool/$identity"
    identities[$identity]=$checksum
    required["$name:$architecture"]=1
    ((package_count += 1))
done < <(find "$input_dir" -type f -name '*.deb' -print0 | sort -z)

[[ $package_count -gt 0 ]] || fail "no Debian packages found beneath $input_dir"
for architecture in amd64 arm64; do
    for name in lisa lisa-runtime-zellij; do
        [[ ${required["$name:$architecture"]:-} == 1 ]] ||
            fail "repository input is missing $name for $architecture"
    done
done

for architecture in amd64 arm64; do
    index_dir="$stage/dists/stable/main/binary-$architecture"
    mkdir -p "$index_dir"
    (
        cd "$stage"
        apt-ftparchive --arch "$architecture" packages pool
    ) > "$index_dir/Packages"
    for name in lisa lisa-runtime-zellij; do
        grep -Fxq "Package: $name" "$index_dir/Packages" ||
            fail "$architecture Packages index is missing $name"
    done
    gzip -n -9 -c "$index_dir/Packages" > "$index_dir/Packages.gz"
done

(
    cd "$stage"
    apt-ftparchive \
        -o APT::FTPArchive::Release::Origin=Lisa \
        -o APT::FTPArchive::Release::Label=Lisa \
        -o APT::FTPArchive::Release::Suite=stable \
        -o APT::FTPArchive::Release::Codename=stable \
        -o APT::FTPArchive::Release::Architectures="amd64 arm64" \
        -o APT::FTPArchive::Release::Components=main \
        -o APT::FTPArchive::Release::Description="Lisa stable apt repository" \
        release dists/stable
) > "$stage/dists/stable/Release"

gpg --batch --yes --armor --local-user "${signing_fingerprint}!" \
    --digest-algo SHA256 --clearsign \
    --output "$stage/dists/stable/InRelease" "$stage/dists/stable/Release"
gpg --batch --yes --armor --local-user "${signing_fingerprint}!" \
    --digest-algo SHA256 --detach-sign \
    --output "$stage/dists/stable/Release.gpg" "$stage/dists/stable/Release"

verify_home=$(mktemp -d "$output_parent/.lisa-apt-verify.XXXXXX")
chmod 0700 "$verify_home"
GNUPGHOME=$verify_home gpg --batch --quiet --import "$public_key"
GNUPGHOME=$verify_home gpg --batch --verify \
    "$stage/dists/stable/InRelease" >/dev/null 2>&1
GNUPGHOME=$verify_home gpg --batch --verify \
    "$stage/dists/stable/Release.gpg" "$stage/dists/stable/Release" >/dev/null 2>&1

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

echo "Built signed Lisa apt repository with $package_count packages at $output_dir"
