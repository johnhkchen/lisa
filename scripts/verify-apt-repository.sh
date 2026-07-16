#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
artifacts_dir=${1:-"$repo_root/target/distrib"}
old_version=0.0.0-1

fail() {
    echo "Signed apt repository verification failed: $*" >&2
    exit 1
}

command -v docker >/dev/null 2>&1 || fail "docker is required"
[[ -d $artifacts_dir ]] || fail "artifact directory does not exist: $artifacts_dir"
artifacts_dir=$(cd "$artifacts_dir" && pwd -P)

for architecture in amd64 arm64; do
    for name in lisa lisa-runtime-zellij; do
        package="$artifacts_dir/$name-$architecture.deb"
        [[ -s $package ]] || fail "missing Debian package: $package"
    done
done

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/lisa-apt-verification.XXXXXX")
tool_container=
client_container=
cleanup() {
    if [[ -n $client_container ]]; then
        docker rm -f "$client_container" >/dev/null 2>&1 || true
    fi
    if [[ -n $tool_container ]]; then
        docker rm -f "$tool_container" >/dev/null 2>&1 || true
    fi
    rm -rf "$work_dir"
}
trap cleanup EXIT

tool_container=$(docker create --platform linux/amd64 \
    --mount "type=bind,source=$repo_root,target=/source,readonly" \
    --mount "type=bind,source=$artifacts_dir,target=/packages,readonly" \
    --mount "type=bind,source=$work_dir,target=/fixture" \
    debian:bookworm-slim sleep infinity)
docker start "$tool_container" >/dev/null
docker exec -e DEBIAN_FRONTEND=noninteractive "$tool_container" sh -eu -c '
    apt-get -qq update
    apt-get install -y -qq apt-utils gnupg >/dev/null
    mkdir -m 0700 /fixture/gnupg
'

printf '%s\n' \
    'Key-Type: RSA' \
    'Key-Length: 2048' \
    'Key-Usage: sign' \
    'Name-Real: Lisa Apt Verification' \
    'Name-Email: apt-verification@example.invalid' \
    'Expire-Date: 0' \
    '%no-protection' \
    '%commit' | docker exec -i -e GNUPGHOME=/fixture/gnupg \
        "$tool_container" gpg --batch --generate-key

fingerprint=$(docker exec -e GNUPGHOME=/fixture/gnupg "$tool_container" sh -eu -c \
    "gpg --batch --with-colons --list-secret-keys | awk -F: '\$1 == \"fpr\" { print \$10; exit }'")
[[ $fingerprint =~ ^[0-9A-F]{40}$ ]] || fail "ephemeral key has invalid fingerprint"

docker exec -e GNUPGHOME=/fixture/gnupg "$tool_container" sh -eu -c \
    "gpg --batch --armor --export '$fingerprint' > /fixture/lisa-archive-keyring.asc"

docker exec -e OLD_VERSION="$old_version" "$tool_container" sh -eu -c '
    mkdir -p /fixture/old /fixture/all /fixture/unpacked
    for architecture in amd64 arm64; do
        for name in lisa lisa-runtime-zellij; do
            source="/packages/$name-$architecture.deb"
            root="/fixture/unpacked/$name-$architecture"
            dpkg-deb --raw-extract "$source" "$root"
            sed -i "s/^Version:.*/Version: $OLD_VERSION/" "$root/DEBIAN/control"
            dpkg-deb --build "$root" "/fixture/old/$name-$architecture.deb" >/dev/null
            cp "$source" "/fixture/all/$name-$architecture.deb"
            cp "/fixture/old/$name-$architecture.deb" \
                "/fixture/all/$name-$architecture-old.deb"
        done
    done
    dpkg-deb --field /packages/lisa-amd64.deb Version > /fixture/current-lisa-version
    dpkg-deb --field /packages/lisa-runtime-zellij-amd64.deb Version \
        > /fixture/current-runtime-version
'

docker exec -e GNUPGHOME=/fixture/gnupg "$tool_container" \
    bash /source/scripts/build-apt-repository.sh \
    /fixture/old /fixture/site "$fingerprint" /fixture/lisa-archive-keyring.asc

docker exec "$tool_container" sh -eu -c '
    export GNUPGHOME=/fixture/public-only
    mkdir -m 0700 "$GNUPGHOME"
    gpg --batch --quiet --import /fixture/lisa-archive-keyring.asc
    gpg --batch --verify /fixture/site/dists/stable/InRelease >/dev/null 2>&1
    gpg --batch --verify /fixture/site/dists/stable/Release.gpg \
        /fixture/site/dists/stable/Release >/dev/null 2>&1
    if grep -R -q "BEGIN PGP PRIVATE KEY" /fixture/site; then
        echo "generated site contains private key material" >&2
        exit 1
    fi
'

client_container=$(docker create --platform linux/amd64 \
    --mount "type=bind,source=$work_dir,target=/fixture,readonly" \
    debian:bookworm-slim sleep infinity)
docker start "$client_container" >/dev/null
docker exec -e DEBIAN_FRONTEND=noninteractive -e OLD_VERSION="$old_version" \
    "$client_container" sh -eu -c '
        apt-get -qq update
        apt-get install -y -qq ca-certificates gnupg >/dev/null
        gpg --batch --dearmor \
            --output /usr/share/keyrings/lisa-archive-keyring.gpg \
            /fixture/lisa-archive-keyring.asc
        printf "%s\n" \
            "deb [signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] file:/fixture/site stable main" \
            > /etc/apt/sources.list.d/lisa.list
        apt-get -qq update
        apt-get install -y -qq lisa lisa-runtime-zellij
        test "$(dpkg-query -W -f="\${Version}" lisa)" = "$OLD_VERSION"
        test "$(dpkg-query -W -f="\${Version}" lisa-runtime-zellij)" = "$OLD_VERSION"
    '

docker exec -e GNUPGHOME=/fixture/gnupg "$tool_container" \
    bash /source/scripts/build-apt-repository.sh \
    /fixture/all /fixture/site "$fingerprint" /fixture/lisa-archive-keyring.asc

docker exec -e DEBIAN_FRONTEND=noninteractive "$client_container" sh -eu -c '
    apt-get -qq update
    apt-get upgrade -y -qq
    lisa_version=$(dpkg-query -W -f="\${Version}" lisa)
    runtime_version=$(dpkg-query -W -f="\${Version}" lisa-runtime-zellij)
    expected_lisa=$(cat /fixture/current-lisa-version)
    expected_runtime=$(cat /fixture/current-runtime-version)
    test "$lisa_version" = "$expected_lisa"
    test "$runtime_version" = "$expected_runtime"
    test "$(apt-cache policy lisa | awk "/Candidate:/ { print \$2 }")" = "$expected_lisa"
    test "$(apt-cache policy lisa-runtime-zellij | awk "/Candidate:/ { print \$2 }")" \
        = "$expected_runtime"
    printf "%s\n" "#!/bin/sh" "echo claude 1.0.0" > /usr/local/bin/claude
    chmod 0755 /usr/local/bin/claude
    mkdir -p /tmp/lisa-doctor-project
'

networks=$(docker inspect --format '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{"\n"}}{{end}}' \
    "$client_container")
while IFS= read -r network; do
    if [[ -n $network ]]; then
        docker network disconnect "$network" "$client_container"
    fi
done <<<"$networks"
if [[ $(docker inspect --format '{{len .NetworkSettings.Networks}}' "$client_container") != 0 ]]; then
    fail "could not isolate the apt verification container from the network"
fi

set +e
doctor_output=$(docker exec "$client_container" \
    /usr/bin/lisa doctor --path /tmp/lisa-doctor-project 2>&1)
doctor_status=$?
set -e
printf '%s\n' "$doctor_output"
[[ $doctor_status -eq 0 ]] || fail "upgraded lisa doctor exited $doctor_status without a network"
grep -Fq 'mode packaged' <<<"$doctor_output" ||
    fail "doctor did not report packaged Zellij provenance"
grep -Fq 'path /usr/libexec/lisa/zellij' <<<"$doctor_output" ||
    fail "doctor did not report the companion runtime path"
grep -Fq 'All dependencies satisfied.' <<<"$doctor_output" ||
    fail "doctor did not report satisfied dependencies"

echo "Verified signed apt install and upgrade from $old_version to $(tr -d '\n' < "$work_dir/current-lisa-version")"
