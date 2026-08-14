#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
artifacts_dir=${1:-"$repo_root/target/distrib"}
old_version=0.0.0-1
# Newer than any real Lisa version and shaped like a candidate. It exists only
# in canary, so a box on stable or nightly must never be offered it.
rc_version=9.9.9~rc1-1

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
# mktemp creates 0700. On a Linux host that mode rides the bind mount into the
# client container, where apt's sandboxed `_apt` user must traverse /fixture to
# read the file: repository — 0700 denies it and `apt-get update` fails with
# EACCES on the index fetch. (Docker Desktop on macOS remaps ownership, which
# is why this only bites on Linux CI runners.)
chmod 0755 "$work_dir"
tool_container=
client_container=
cleanup() {
    if [[ -n $client_container ]]; then
        docker rm -f "$client_container" >/dev/null 2>&1 || true
    fi
    if [[ -n $tool_container ]]; then
        docker rm -f "$tool_container" >/dev/null 2>&1 || true
    fi
    # Container-root-owned files inside the fixture are not removable by the
    # host user on Linux; loosen them from inside a throwaway container first.
    docker run --rm \
        --mount "type=bind,source=$work_dir,target=/fixture" \
        debian:bookworm-slim chmod -R a+rwX /fixture >/dev/null 2>&1 || true
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

# Three package generations: an old release every suite carries, the real
# current release, and a candidate that only canary is allowed to see.
docker exec -e OLD_VERSION="$old_version" -e RC_VERSION="$rc_version" \
    "$tool_container" sh -eu -c '
    mkdir -p /fixture/pkgs/old /fixture/pkgs/current /fixture/pkgs/rc /fixture/unpacked
    for architecture in amd64 arm64; do
        for name in lisa lisa-runtime-zellij; do
            source="/packages/$name-$architecture.deb"
            cp "$source" "/fixture/pkgs/current/$name-$architecture.deb"
            for generation in old rc; do
                case "$generation" in
                    old) version=$OLD_VERSION ;;
                    rc) version=$RC_VERSION ;;
                esac
                root="/fixture/unpacked/$generation-$name-$architecture"
                dpkg-deb --raw-extract "$source" "$root"
                sed -i "s/^Version:.*/Version: $version/" "$root/DEBIAN/control"
                dpkg-deb --build "$root" \
                    "/fixture/pkgs/$generation/$name-$architecture.deb" >/dev/null
            done
        done
    done
    dpkg-deb --field /packages/lisa-amd64.deb Version > /fixture/current-lisa-version
    dpkg-deb --field /packages/lisa-runtime-zellij-amd64.deb Version \
        > /fixture/current-runtime-version

    # Bootstrap: every suite carries only the old release.
    for suite in stable nightly canary; do
        mkdir -p "/fixture/bootstrap/$suite"
        cp -r /fixture/pkgs/old "/fixture/bootstrap/$suite/old"
    done

    # Steady state: stable and nightly stop at the current release; the
    # candidate reaches canary and no further.
    for suite in stable nightly canary; do
        mkdir -p "/fixture/full/$suite"
        cp -r /fixture/pkgs/old "/fixture/full/$suite/old"
        cp -r /fixture/pkgs/current "/fixture/full/$suite/current"
    done
    cp -r /fixture/pkgs/rc /fixture/full/canary/rc
    cp -r /fixture/pkgs/rc /fixture/full/stable/rc
'

docker exec -e GNUPGHOME=/fixture/gnupg "$tool_container" \
    bash /source/scripts/build-apt-repository.sh \
    /fixture/bootstrap /fixture/site "$fingerprint" /fixture/lisa-archive-keyring.asc

docker exec "$tool_container" sh -eu -c '
    export GNUPGHOME=/fixture/public-only
    mkdir -m 0700 "$GNUPGHOME"
    gpg --batch --quiet --import /fixture/lisa-archive-keyring.asc
    suites=$(ls /fixture/site/dists | sort | tr "\n" " ")
    if [ "$suites" != "canary nightly stable " ]; then
        echo "published suites are \"$suites\", expected canary nightly stable" >&2
        exit 1
    fi
    # One key, one keyring, three suites: a channel change must never mean
    # trusting a second key, so every suite verifies against this one import.
    for suite in stable nightly canary; do
        gpg --batch --verify "/fixture/site/dists/$suite/InRelease" >/dev/null 2>&1
        gpg --batch --verify "/fixture/site/dists/$suite/Release.gpg" \
            "/fixture/site/dists/$suite/Release" >/dev/null 2>&1
    done
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
    /fixture/full /fixture/site "$fingerprint" /fixture/lisa-archive-keyring.asc

# The candidate's bytes are in the shared pool. That is what makes one signing
# key and one pool possible; the suite index is what keeps it out of stable.
docker exec -e RC_VERSION="$rc_version" "$tool_container" sh -eu -c '
    test -f "/fixture/site/pool/main/l/lisa/lisa_${RC_VERSION}_amd64.deb"
    if grep -q "$RC_VERSION" /fixture/site/dists/stable/main/binary-amd64/Packages; then
        echo "the stable index lists candidate $RC_VERSION" >&2
        exit 1
    fi
    if grep -q "$RC_VERSION" /fixture/site/dists/nightly/main/binary-amd64/Packages; then
        echo "the nightly index lists candidate $RC_VERSION" >&2
        exit 1
    fi
    grep -q "$RC_VERSION" /fixture/site/dists/canary/main/binary-amd64/Packages
'

# A box on stable upgrades to the current release and cannot see the candidate,
# while every version this channel has ever carried stays installable.
docker exec -e DEBIAN_FRONTEND=noninteractive -e OLD_VERSION="$old_version" \
    -e RC_VERSION="$rc_version" "$client_container" sh -eu -c '
    apt-get -qq update
    apt-get upgrade -y -qq
    expected_lisa=$(cat /fixture/current-lisa-version)
    expected_runtime=$(cat /fixture/current-runtime-version)
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$expected_lisa"
    test "$(dpkg-query -W -f="\${Version}" lisa-runtime-zellij)" = "$expected_runtime"
    test "$(apt-cache policy lisa | awk "/Candidate:/ { print \$2 }")" = "$expected_lisa"
    test "$(apt-cache policy lisa-runtime-zellij | awk "/Candidate:/ { print \$2 }")" \
        = "$expected_runtime"
    if apt-cache madison lisa | grep -q "$RC_VERSION"; then
        echo "a box on stable can see candidate $RC_VERSION" >&2
        exit 1
    fi
    if apt-get install -s -y "lisa=$RC_VERSION" >/dev/null 2>&1; then
        echo "a box on stable can install candidate $RC_VERSION" >&2
        exit 1
    fi
    # Rollback with no extra machinery: the pool still holds the old release.
    apt-cache madison lisa | grep -q "$OLD_VERSION"
    apt-get install -y -qq --allow-downgrades \
        "lisa=$OLD_VERSION" "lisa-runtime-zellij=$OLD_VERSION"
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$OLD_VERSION"
    apt-get install -y -qq "lisa=$expected_lisa" "lisa-runtime-zellij=$expected_runtime"
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$expected_lisa"
'

# Changing channel is editing one word in one file. nightly is a real suite of
# its own, carrying the same current release and no candidate.
docker exec -e DEBIAN_FRONTEND=noninteractive -e RC_VERSION="$rc_version" \
    "$client_container" sh -eu -c '
    sed -i "s#/fixture/site [a-z]* main#/fixture/site nightly main#" \
        /etc/apt/sources.list.d/lisa.list
    apt-get -qq update
    expected_lisa=$(cat /fixture/current-lisa-version)
    expected_runtime=$(cat /fixture/current-runtime-version)
    test "$(apt-cache policy lisa | awk "/Candidate:/ { print \$2 }")" = "$expected_lisa"
    test "$(apt-cache policy lisa-runtime-zellij | awk "/Candidate:/ { print \$2 }")" \
        = "$expected_runtime"
    if apt-cache madison lisa | grep -q "$RC_VERSION"; then
        echo "a box on nightly can see candidate $RC_VERSION" >&2
        exit 1
    fi
'

docker exec -e DEBIAN_FRONTEND=noninteractive -e RC_VERSION="$rc_version" \
    "$client_container" sh -eu -c '
    sed -i "s#/fixture/site [a-z]* main#/fixture/site canary main#" \
        /etc/apt/sources.list.d/lisa.list
    apt-get -qq update
    test "$(apt-cache policy lisa | awk "/Candidate:/ { print \$2 }")" = "$RC_VERSION"
    apt-get upgrade -y -qq
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$RC_VERSION"
    test "$(dpkg-query -W -f="\${Version}" lisa-runtime-zellij)" = "$RC_VERSION"
'

# Coming back down a channel is a downgrade, and apt will not perform one until
# it is told to. This is the operator note in docs/knowledge/mac-mini-nightly.md.
docker exec -e DEBIAN_FRONTEND=noninteractive -e RC_VERSION="$rc_version" \
    "$client_container" sh -eu -c '
    sed -i "s#/fixture/site [a-z]* main#/fixture/site stable main#" \
        /etc/apt/sources.list.d/lisa.list
    apt-get -qq update
    expected_lisa=$(cat /fixture/current-lisa-version)
    expected_runtime=$(cat /fixture/current-runtime-version)
    if apt-cache madison lisa | grep -q "$RC_VERSION"; then
        echo "a box back on stable can still see candidate $RC_VERSION" >&2
        exit 1
    fi
    apt-get upgrade -y -qq
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$RC_VERSION"
    if apt-get install -y -qq "lisa=$expected_lisa" >/dev/null 2>&1; then
        echo "apt downgraded lisa without --allow-downgrades" >&2
        exit 1
    fi
    apt-get install -y -qq --allow-downgrades \
        "lisa=$expected_lisa" "lisa-runtime-zellij=$expected_runtime"
    test "$(dpkg-query -W -f="\${Version}" lisa)" = "$expected_lisa"
    test "$(dpkg-query -W -f="\${Version}" lisa-runtime-zellij)" = "$expected_runtime"
'

docker exec -e DEBIAN_FRONTEND=noninteractive "$client_container" sh -eu -c '
    printf "%s\n" "#!/bin/sh" "echo claude 1.0.0" > /usr/local/bin/claude
    chmod 0755 /usr/local/bin/claude
    mkdir -p /tmp/lisa-doctor-project
    # Since 0.4.4, doctor in an uninitialized folder exits nonzero with the
    # "Run: lisa init" guidance — the fixture must be a real project.
    /usr/bin/lisa init --no-history --path /tmp/lisa-doctor-project
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

echo "Verified stable, nightly and canary against one archive key:" \
    "install and upgrade from $old_version to $(tr -d '\n' < "$work_dir/current-lisa-version")," \
    "rollback by exact version, a channel change by one word, and stable never" \
    "seeing the $rc_version candidate that sits in canary"
