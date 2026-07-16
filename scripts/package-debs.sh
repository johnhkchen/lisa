#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
distrib_dir=${LISA_DISTRIB_DIR:-"$repo_root/target/distrib"}
output_dir=${LISA_DEB_OUTPUT_DIR:-"$repo_root"}
runtime_manifest="$repo_root/crates/lisa-cli/data/managed-runtime-sha256.json"

nfpm_version=2.47.0
nfpm_asset=nfpm_2.47.0_Linux_x86_64.tar.gz
nfpm_sha256=0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783

if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    echo "Debian package assembly requires a Linux x86_64 build host" >&2
    exit 1
fi

for command in cargo curl jq sha256sum tar; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Debian package assembly requires $command" >&2
        exit 1
    fi
done

lisa_version=${LISA_VERSION:-}
if [[ -z $lisa_version ]]; then
    lisa_version=$(cargo metadata --no-deps --format-version 1 --manifest-path "$repo_root/Cargo.toml" |
        jq -er '.packages[] | select(.name == "lisa-cli") | .version')
fi
if [[ -z $lisa_version || $lisa_version == null ]]; then
    echo "Could not determine the lisa-cli package version" >&2
    exit 1
fi

mkdir -p "$output_dir"
work_dir=$(mktemp -d "${TMPDIR:-/tmp}/lisa-debs.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT

verify_sha256() {
    local file=$1
    local expected=$2
    local actual
    actual=$(sha256sum "$file" | awk '{print $1}')
    if [[ $actual != "$expected" ]]; then
        echo "SHA-256 mismatch for $file: expected $expected, got $actual" >&2
        exit 1
    fi
}

nfpm_archive="$work_dir/$nfpm_asset"
curl --proto '=https' --tlsv1.2 -LsSf \
    "https://github.com/goreleaser/nfpm/releases/download/v${nfpm_version}/${nfpm_asset}" \
    -o "$nfpm_archive"
verify_sha256 "$nfpm_archive" "$nfpm_sha256"
mkdir -p "$work_dir/nfpm"
tar -xzf "$nfpm_archive" -C "$work_dir/nfpm"
nfpm_binary="$work_dir/nfpm/nfpm"
if [[ ! -x $nfpm_binary ]]; then
    echo "Authenticated nFPM archive did not contain an executable nfpm" >&2
    exit 1
fi

package_architecture() {
    local rust_target=$1
    local deb_arch=$2
    local lisa_archive="$distrib_dir/lisa-cli-${rust_target}.tar.gz"
    local stage="$work_dir/$deb_arch"
    local lisa_binary
    local lisa_count
    local runtime_url
    local runtime_sha256
    local zellij_version
    local runtime_archive
    local runtime_members
    local zellij_binary

    if [[ ! -f $lisa_archive ]]; then
        echo "Missing cargo-dist Lisa archive: $lisa_archive" >&2
        exit 1
    fi

    mkdir -p "$stage/lisa"
    tar -xzf "$lisa_archive" -C "$stage/lisa"
    lisa_count=$(find "$stage/lisa" -type f -name lisa | wc -l | tr -d ' ')
    if [[ $lisa_count != 1 ]]; then
        echo "Expected exactly one lisa binary in $lisa_archive, found $lisa_count" >&2
        exit 1
    fi
    lisa_binary=$(find "$stage/lisa" -type f -name lisa -print)

    runtime_url=$(jq -er --arg target "$rust_target" \
        '.artifacts[] | select(.target == $target) | .url' "$runtime_manifest")
    runtime_sha256=$(jq -er --arg target "$rust_target" \
        '.artifacts[] | select(.target == $target) | .sha256' "$runtime_manifest")
    zellij_version=$(jq -er '.version' "$runtime_manifest")
    runtime_archive="$stage/zellij.tar.gz"
    curl --proto '=https' --tlsv1.2 -LsSf "$runtime_url" -o "$runtime_archive"
    verify_sha256 "$runtime_archive" "$runtime_sha256"

    runtime_members=$(tar -tzf "$runtime_archive")
    if [[ $runtime_members != zellij ]]; then
        echo "Expected exactly one top-level zellij file in $runtime_url" >&2
        exit 1
    fi
    mkdir -p "$stage/runtime"
    tar -xzf "$runtime_archive" -C "$stage/runtime" zellij
    zellij_binary="$stage/runtime/zellij"
    chmod 0755 "$lisa_binary" "$zellij_binary"

    NFPM_ARCH=$deb_arch \
        LISA_VERSION=$lisa_version \
        LISA_BINARY=$lisa_binary \
        "$nfpm_binary" package --packager deb \
        --config "$repo_root/packaging/nfpm/lisa.yaml" \
        --target "$output_dir/lisa-${deb_arch}.deb"
    NFPM_ARCH=$deb_arch \
        LISA_VERSION=$lisa_version \
        ZELLIJ_VERSION=$zellij_version \
        ZELLIJ_BINARY=$zellij_binary \
        "$nfpm_binary" package --packager deb \
        --config "$repo_root/packaging/nfpm/lisa-runtime-zellij.yaml" \
        --target "$output_dir/lisa-runtime-zellij-${deb_arch}.deb"
}

package_architecture x86_64-unknown-linux-musl amd64
package_architecture aarch64-unknown-linux-musl arm64

echo "Built Lisa $lisa_version Debian packages for amd64 and arm64 in $output_dir"
