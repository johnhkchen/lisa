#!/usr/bin/env bash
set -euo pipefail

# Ask the live tap whether each formula names the release its channel means
# (T-069-01-05).
#
#   lisa          the newest release that is not a prerelease
#   lisa-nightly  the release packaging/apt/nightly-tag.txt names on main
#   lisa-canary   the newest release of any kind
#
# The publisher derives all three on every run, so this is the check that the
# derivation reached the tap -- after a release, after a repair, or when a desk
# reports `brew outdated` saying something that cannot be true.
#
# Read-only, on purpose. It asks GitHub and compares; it writes nothing, here or
# there, so it is safe to run against a live tap from anywhere.
#
# Exit codes:
#   0  every formula names the release its channel means
#   1  a formula does not
#   2  it could not look -- no gh, no network, no auth -- which is not a verdict
#      on the tap

source_repo=${LISA_SOURCE_REPO:-johnhkchen/lisa}
tap_repo=${LISA_TAP_REPO:-johnhkchen/homebrew-lisa}

cannot_look() {
    echo "verify-live-tap: $*" >&2
    exit 2
}

command -v gh >/dev/null 2>&1 || cannot_look "gh is required to read the tap"
command -v awk >/dev/null 2>&1 || cannot_look "awk is required"
command -v base64 >/dev/null 2>&1 || cannot_look "base64 is required"

newest_tag=$(gh release list --repo "$source_repo" --exclude-drafts --limit 1 \
    --json tagName --jq '.[0].tagName') ||
    cannot_look "could not read $source_repo's release list"
[[ -n $newest_tag ]] || cannot_look "$source_repo has published no releases"

newest_stable_tag=$(gh release list --repo "$source_repo" \
    --exclude-drafts --exclude-pre-releases --limit 1 \
    --json tagName --jq '.[0].tagName') ||
    cannot_look "could not read $source_repo's release list"

# The promotion pointer, read from main the same way the publish reads it.
# `stable` means nothing has been promoted yet and nightly carries what stable
# carries.
pointer=$(gh api -H "Accept: application/vnd.github.raw" \
    "repos/$source_repo/contents/packaging/apt/nightly-tag.txt?ref=main" |
    tr -d '[:space:]') ||
    cannot_look "could not read packaging/apt/nightly-tag.txt on main"
[[ -n $pointer ]] || cannot_look "packaging/apt/nightly-tag.txt is empty on main"

if [[ $pointer == stable ]]; then
    nightly_tag=$newest_stable_tag
else
    nightly_tag=$pointer
fi

# Absent, rather than empty: a formula the tap does not carry at all.
live_version() {
    local formula=$1 encoded
    encoded=$(gh api "repos/$tap_repo/contents/Formula/$formula.rb" --jq .content 2>/dev/null) || {
        printf 'absent'
        return
    }
    printf '%s' "$encoded" | tr -d '\n' | base64 --decode |
        awk -F'"' '/^  version "/ { print $2; exit }'
}

wrong=0
printf '%-14s %-14s %s\n' formula carries 'should carry'
for pair in "lisa:$newest_stable_tag" "lisa-nightly:$nightly_tag" "lisa-canary:$newest_tag"; do
    formula=${pair%%:*}
    tag=${pair#*:}
    want=${tag:+${tag#v}}
    want=${want:-absent}
    got=$(live_version "$formula")
    got=${got:-absent}
    printf '%-14s %-14s %s\n' "$formula" "$got" "$want"
    [[ $got == "$want" ]] || wrong=$((wrong + 1))
done

if [[ $wrong -ne 0 ]]; then
    echo "verify-live-tap: $wrong formula(e) name a release their channel does not mean;" \
        "see docs/knowledge/nightly-promotion.md#when-the-tap-is-already-wrong" >&2
    exit 1
fi

echo "verify-live-tap: $tap_repo is correct on all three channels (promotion pointer: $pointer)"
