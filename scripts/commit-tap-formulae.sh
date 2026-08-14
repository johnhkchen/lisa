#!/usr/bin/env bash
set -euo pipefail

# Style the tap's channel formulae and commit only the ones that really changed.
#
# Two callers write into johnhkchen/homebrew-lisa -- a release publish, which
# moves lisa and lisa-canary, and the soak promotion, which moves lisa-nightly
# (T-069-01-03). They share this so the tap's history has one grammar, and so
# neither can leave a commit behind for a formula whose contents did not move.
#
# Style first and ask git after: `brew style --fix` rewrites the file, so what
# is committed is the styled version and a publish that changes nothing really
# does leave no commit.
#
# Usage:
#   commit-tap-formulae.sh <tap-checkout> [note]
#
#   <tap-checkout>  a checkout of the tap, with Formula/*.rb already rendered
#   [note]          one line added to each commit body saying why it moved

formulae=(lisa lisa-nightly lisa-canary)

fail() {
    echo "commit-tap-formulae: $*" >&2
    exit 1
}

[[ $# -ge 1 && $# -le 2 ]] || fail "usage: $(basename "$0") <tap-checkout> [note]"

tap_checkout=$1
note=${2:-}

[[ -d $tap_checkout ]] || fail "tap checkout does not exist: $tap_checkout"
command -v brew >/dev/null 2>&1 || fail "brew is required to style the formulae"

committed=0
for formula in "${formulae[@]}"; do
    path="Formula/$formula.rb"
    [[ -e "$tap_checkout/$path" ]] || continue

    # We avoid reformatting user-provided data such as the app description and
    # homepage.
    brew style --except-cops FormulaAudit/Homepage,FormulaAudit/Desc,FormulaAuditStrict \
        --fix "$tap_checkout/$path" || true

    if [[ -z $(git -C "$tap_checkout" status --porcelain -- "$path") ]]; then
        echo "unchanged $path"
        continue
    fi

    version=$(awk -F'"' '/^  version "/ { print $2; exit }' "$tap_checkout/$path")
    [[ -n $version ]] || fail "no version line in $path"

    git -C "$tap_checkout" add "$path"
    if [[ -n $note ]]; then
        git -C "$tap_checkout" commit -m "$formula $version" -m "$note"
    else
        git -C "$tap_checkout" commit -m "$formula $version"
    fi
    echo "committed $path ($version)"
    committed=$((committed + 1))
done

echo "commit-tap-formulae: $committed formula commit(s)"
