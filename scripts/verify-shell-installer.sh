#!/usr/bin/env bash
set -euo pipefail

# Rehearse the generated shell installer on a no-xz Debian bookworm container:
# the same download -> checksum -> extract -> place path the README one-liner
# takes, pointed at the just-built release artifacts instead of a published
# GitHub release (T-046-03-03). The client container mirrors the Chromebook
# fixture's floor: tar+gzip and curl, with xz/Rust/compilers absent.

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
distrib_dir=${LISA_DISTRIB_DIR:-"$repo_root/target/distrib"}
distrib_dir=$(cd "$distrib_dir" && pwd)

for command in docker grep; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "shell installer verification requires $command" >&2
        exit 1
    fi
done

installer="$distrib_dir/lisa-cli-installer.sh"
if [[ ! -s $installer ]]; then
    echo "missing generated shell installer: $installer" >&2
    exit 1
fi

# Generation-time assertions: the installer must serve gzip archives for both
# Linux architectures and must not reference xz-compressed artifacts at all.
if grep -q 'tar\.xz' "$installer"; then
    echo "generated installer still references .tar.xz artifacts" >&2
    exit 1
fi
for target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
    if ! grep -Fq "lisa-cli-$target.tar.gz" "$installer"; then
        echo "generated installer does not offer lisa-cli-$target.tar.gz" >&2
        exit 1
    fi
    if [[ ! -s "$distrib_dir/lisa-cli-$target.tar.gz" ]]; then
        echo "missing release archive: $distrib_dir/lisa-cli-$target.tar.gz" >&2
        exit 1
    fi
done

# Runtime rehearsal on the host architecture. The installer's own platform
# detection chooses the archive, so the run covers exactly what a user on
# this architecture gets.
suffix=$$
network="lisa-installer-verify-$suffix"
server_container=
client_container=
cleanup() {
    if [[ -n $client_container ]]; then
        docker rm -f "$client_container" >/dev/null 2>&1 || true
    fi
    if [[ -n $server_container ]]; then
        docker rm -f "$server_container" >/dev/null 2>&1 || true
    fi
    docker network rm "$network" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker network create "$network" >/dev/null

server_container=$(docker create --network "$network" --network-alias artifacts \
    --mount "type=bind,source=$distrib_dir,target=/distrib,readonly" \
    python:3-slim \
    python3 -m http.server 8000 --bind 0.0.0.0 --directory /distrib)
docker start "$server_container" >/dev/null

client_container=$(docker create --network "$network" \
    debian:bookworm-slim sleep infinity)
docker start "$client_container" >/dev/null

# Mirror the Chromebook fixture floor: transport plus certificates, nothing
# else. The forbidden-tool assertion runs after the install so package drift
# cannot quietly reintroduce a decompressor or toolchain dependency.
docker exec -e DEBIAN_FRONTEND=noninteractive "$client_container" sh -eu -c '
    apt-get -qq update
    apt-get install -y -qq --no-install-recommends ca-certificates curl >/dev/null
    useradd --create-home --shell /bin/bash tester
    for i in 1 2 3 4 5 6 7 8 9 10; do
        if curl -fsS -o /dev/null http://artifacts:8000/lisa-cli-installer.sh; then
            exit 0
        fi
        sleep 1
    done
    echo "artifact server did not become reachable" >&2
    exit 1
'

install_log=$(docker exec --user tester \
    -e HOME=/home/tester \
    -e INSTALLER_DOWNLOAD_URL=http://artifacts:8000 \
    "$client_container" sh -c \
    "curl --proto '=http,https' -LsSf http://artifacts:8000/lisa-cli-installer.sh | sh" 2>&1)
printf '%s\n' "$install_log"

if grep -Fq 'no checksums to verify' <<<"$install_log"; then
    echo "installer skipped integrity verification: no embedded checksums" >&2
    exit 1
fi
if ! grep -Fq '.local/bin' <<<"$install_log"; then
    echo "installer output does not carry the ~/.local/bin PATH guidance" >&2
    exit 1
fi

docker exec --user tester -e HOME=/home/tester "$client_container" sh -eu -c '
    for binary in xz rustc cargo rustup gcc cc g++ make git; do
        if command -v "$binary" >/dev/null 2>&1; then
            echo "rehearsal invariant failed: $binary must be absent" >&2
            exit 1
        fi
    done
    test -x "$HOME/.local/bin/lisa"
    version=$("$HOME/.local/bin/lisa" --version)
    case "$version" in
        lisa\ *) echo "installed: $version" ;;
        *)
            echo "unexpected installed version output: $version" >&2
            exit 1
            ;;
    esac
'

echo "verified shell installer: no-xz bookworm install to ~/.local/bin with checksum verification"
