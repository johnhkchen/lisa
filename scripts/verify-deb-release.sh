#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
distrib_dir=${LISA_DISTRIB_DIR:-"$repo_root/target/distrib"}

for command in docker dpkg-deb grep; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Debian package verification requires $command" >&2
        exit 1
    fi
done

field() {
    dpkg-deb --field "$1" "$2"
}

assert_equal() {
    local actual=$1
    local expected=$2
    local description=$3
    if [[ $actual != "$expected" ]]; then
        echo "$description: expected '$expected', got '$actual'" >&2
        exit 1
    fi
}

assert_contents() {
    local package=$1
    local path=$2
    if ! dpkg-deb --contents "$package" | grep -Eq "^-rwxr-xr-x .* \.$path$"; then
        echo "$package does not install executable $path with mode 0755" >&2
        exit 1
    fi
}

declare -A lisa_versions
for architecture in amd64 arm64; do
    lisa_package="$distrib_dir/lisa-${architecture}.deb"
    runtime_package="$distrib_dir/lisa-runtime-zellij-${architecture}.deb"
    for package in "$lisa_package" "$runtime_package"; do
        if [[ ! -s $package ]]; then
            echo "Missing Debian package: $package" >&2
            exit 1
        fi
    done

    assert_equal "$(field "$lisa_package" Package)" lisa "CLI package name"
    assert_equal "$(field "$runtime_package" Package)" lisa-runtime-zellij \
        "runtime package name"
    assert_equal "$(field "$lisa_package" Architecture)" "$architecture" \
        "CLI package architecture"
    assert_equal "$(field "$runtime_package" Architecture)" "$architecture" \
        "runtime package architecture"

    lisa_versions[$architecture]=$(field "$lisa_package" Version)
    assert_equal "$(field "$runtime_package" Version)" "${lisa_versions[$architecture]}" \
        "companion package version"
    if ! field "$lisa_package" Depends | grep -Eq '(^|, )git([ (]|$)'; then
        echo "$lisa_package does not depend on git" >&2
        exit 1
    fi
    if ! field "$lisa_package" Recommends | grep -Eq '(^|, )lisa-runtime-zellij([ (]|$)'; then
        echo "$lisa_package does not recommend lisa-runtime-zellij" >&2
        exit 1
    fi

    assert_contents "$lisa_package" /usr/bin/lisa
    assert_contents "$runtime_package" /usr/libexec/lisa/zellij
done

assert_equal "${lisa_versions[arm64]}" "${lisa_versions[amd64]}" \
    "cross-architecture package version"

container=
cleanup() {
    if [[ -n $container ]]; then
        docker rm -f "$container" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

container=$(docker create --platform linux/amd64 \
    --mount "type=bind,source=$distrib_dir,target=/packages,readonly" \
    debian:bookworm-slim sleep infinity)
docker start "$container" >/dev/null
docker exec -e DEBIAN_FRONTEND=noninteractive "$container" sh -eu -c '
    apt-get -qq update
    apt-get install -y -qq /packages/lisa-amd64.deb /packages/lisa-runtime-zellij-amd64.deb
    printf "%s\n" "#!/bin/sh" "echo claude 1.0.0" > /usr/local/bin/claude
    chmod 0755 /usr/local/bin/claude
    mkdir -p /tmp/lisa-doctor-project
    # Since 0.4.4, doctor in an uninitialized folder exits nonzero with the
    # "Run: lisa init" guidance — the fixture must be a real project.
    /usr/bin/lisa init --no-history --path /tmp/lisa-doctor-project
'

docker network disconnect bridge "$container"
if [[ $(docker inspect --format '{{len .NetworkSettings.Networks}}' "$container") != 0 ]]; then
    echo "Could not isolate the package verification container from the network" >&2
    exit 1
fi

set +e
doctor_output=$(docker exec "$container" \
    /usr/bin/lisa doctor --path /tmp/lisa-doctor-project 2>&1)
doctor_status=$?
set -e
printf '%s\n' "$doctor_output"
if [[ $doctor_status -ne 0 ]]; then
    echo "Packaged lisa doctor exited $doctor_status without a network" >&2
    exit 1
fi
if ! grep -Fq 'mode packaged' <<<"$doctor_output"; then
    echo "Doctor did not report packaged Zellij provenance" >&2
    exit 1
fi
if ! grep -Fq 'path /usr/libexec/lisa/zellij' <<<"$doctor_output"; then
    echo "Doctor did not report the companion runtime path" >&2
    exit 1
fi
if ! grep -Fq 'All dependencies satisfied.' <<<"$doctor_output"; then
    echo "Doctor did not report satisfied dependencies" >&2
    exit 1
fi

docker exec "$container" /usr/libexec/lisa/zellij --version
echo "Verified Lisa ${lisa_versions[amd64]} Debian packages on bookworm without runtime network"
