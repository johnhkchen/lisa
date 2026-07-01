#!/usr/bin/env bash
# T-021-01 spike driver. Runs every probe against the installed codex and leaves
# all evidence under ./out/. Intended to be run ONCE on a machine with the pinned
# codex rust-v0.142.5 installed. Prints a compact PASS/FAIL-input summary; the
# actual verdicts are written up by hand in ../design.md from the evidence.
#
# SPIKE STUB — never merged into crates/.
set -u
HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$HARNESS_DIR"

if ! command -v codex >/dev/null 2>&1; then
  echo "codex not installed — cannot run the spike. Install rust-v0.142.5 first." >&2
  echo "Every probe is a no-op until then; design.md verdicts stay PROVISIONAL." >&2
  exit 1
fi

for p in q1-env-inheritance q2-json-fidelity q3-inpane-rendering q4-directory-trust q5-resume-followup; do
  echo "=== running $p ==="
  bash "$HARNESS_DIR/$p.sh" || echo "!!! $p exited non-zero (may still have captured evidence)"
done

echo
echo "Evidence written under: $HARNESS_DIR/out/"
echo "Now transcribe verdicts into ../design.md, pinned to the codex version in each out/*/codex-version.txt."
