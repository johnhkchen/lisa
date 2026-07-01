# shellcheck shell=bash
# T-021-01 spike harness — shared setup. Sourced by every q*.sh probe.
#
# THIS IS SPIKE STUB CODE. It lives under docs/active/work/ and must never be
# merged into crates/. It exists to produce evidence (event dumps, exit codes,
# env dumps) for the wrapper verdict — not to become the wrapper.
#
# Pinned target: codex rust-v0.142.5 (see PINNED_VERSION). Every probe records
# the codex version it actually ran against so a verdict can be trusted or
# rejected on version grounds.

set -u

PINNED_VERSION="rust-v0.142.5"

# Where evidence lands. Each probe writes <OUT>/qN/... (stdout.jsonl, stderr.log,
# exit-code, notes).
HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${OUT_DIR:-$HARNESS_DIR/out}"

# Isolated CODEX_HOME so Q4 (directory trust) starts from a known-empty state and
# no probe pollutes the operator's real ~/.codex. Overridable per probe.
SANDBOX_HOME="${SANDBOX_HOME:-$HARNESS_DIR/.codex-sandbox}"

log()  { printf '[%s] %s\n' "${PROBE:-harness}" "$*" >&2; }
fail() { log "FAIL: $*"; exit 1; }

# Confirm codex exists and record the running version next to the evidence.
# A verdict captured against a version other than PINNED_VERSION is provisional.
require_codex() {
  command -v codex >/dev/null 2>&1 || fail "codex not on PATH — install $PINNED_VERSION before running the spike"
  local v
  v="$(codex --version 2>&1 || true)"
  log "codex version: $v (pinned target: $PINNED_VERSION)"
  case "$v" in
    *"$PINNED_VERSION"*) : ;;
    *) log "WARNING: running version does not match $PINNED_VERSION — verdict is provisional" ;;
  esac
  printf '%s\n' "$v" > "$1/codex-version.txt"
}

# mkdir the per-probe evidence dir and echo its path.
probe_out() {
  local d="$OUT_DIR/$1"
  mkdir -p "$d"
  printf '%s' "$d"
}
