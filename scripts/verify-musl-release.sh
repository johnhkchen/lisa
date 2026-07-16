#!/usr/bin/env bash
set -euo pipefail

target=${1:-}
case "$target" in
    x86_64-unknown-linux-musl)
        expected_runtime_sha=bac0728945e8f5a28f2647e2b9b0cfe4591d71abfe227336b1318937241f071d
        ;;
    aarch64-unknown-linux-musl)
        expected_runtime_sha=8ced877df27a8fe9112607dd3d772442aefa5e42359cda1baba53e78c4ae46aa
        ;;
    *)
        echo "usage: $0 <x86_64-unknown-linux-musl|aarch64-unknown-linux-musl>" >&2
        exit 2
        ;;
esac

distrib_dir=${LISA_DISTRIB_DIR:-target/distrib}
archive="$distrib_dir/lisa-cli-$target.tar.gz"
if [[ ! -f "$archive" ]]; then
    echo "missing musl release archive: $archive" >&2
    exit 1
fi
archive=$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

# Extract inside a container matching the Chromebook fixture's decompressor
# floor (tar+gzip, no xz) so "unpacking needs no extra host packages" is a
# checked release property, not an accident of runner tooling (T-046-03-03).
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --mount "type=bind,source=$archive,target=/archive.tar.gz,readonly" \
    --mount "type=bind,source=$work_dir,target=/extract" \
    debian:bookworm-slim sh -eu -c '
        if command -v xz >/dev/null 2>&1; then
            echo "no-xz extraction proof is vacuous: container image ships xz" >&2
            exit 1
        fi
        tar -xzf /archive.tar.gz -C /extract
    '

mapfile -d '' binaries < <(find "$work_dir" -type f -name lisa -print0)
if [[ ${#binaries[@]} -ne 1 ]]; then
    echo "expected exactly one lisa binary in $archive, found ${#binaries[@]}" >&2
    exit 1
fi
binary=${binaries[0]}

file_output=$(file "$binary")
if [[ ! "$file_output" =~ ELF ]] ||
    [[ ! "$file_output" =~ (statically\ linked|static-pie\ linked) ]]; then
    echo "musl artifact is not a static ELF binary: $file_output" >&2
    exit 1
fi

if readelf -l "$binary" | grep -q 'INTERP'; then
    echo "musl artifact unexpectedly contains an ELF interpreter" >&2
    exit 1
fi

ldd_output=$(ldd "$binary" 2>&1 || true)
if [[ ! "$ldd_output" =~ (not\ a\ dynamic\ executable|statically\ linked) ]]; then
    echo "ldd did not identify a static executable: $ldd_output" >&2
    exit 1
fi

python3 - "$binary" "$expected_runtime_sha" <<'PY'
import pathlib
import sys

binary = pathlib.Path(sys.argv[1]).read_bytes()
expected_runtime_sha = sys.argv[2].encode()
if b"\0asm\x01\0\0\0" not in binary:
    raise SystemExit("packaged lisa binary does not contain a non-empty WASM module")
if expected_runtime_sha not in binary:
    raise SystemExit("packaged lisa binary does not contain its managed-runtime checksum")
PY

version_output=$(docker run --rm -v "$binary:/lisa:ro" debian:bullseye-slim /lisa --version)
if [[ ! "$version_output" =~ ^lisa\  ]]; then
    echo "unexpected bullseye version output: $version_output" >&2
    exit 1
fi

echo "verified $target: no-xz gzip extraction, static ELF, embedded assets, bullseye execution ($version_output)"
