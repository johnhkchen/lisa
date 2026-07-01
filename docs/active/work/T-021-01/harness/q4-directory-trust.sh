#!/usr/bin/env bash
# Q4 — Directory trust, headless.
# A fresh CODEX_HOME has no trusted projects. Does `codex exec -a never` on an
# untrusted repo BLOCK (prompt / refuse) or proceed? If it blocks, `lisa doctor`
# must pre-seed trust for the working tree before the wrapper runs. We test three
# states so doctor knows exactly what to write:
#   A. fresh CODEX_HOME, no seeding           → expect block (hypothesis)
#   B. fresh CODEX_HOME + pre-seeded trust     → expect proceed
#   C. fresh CODEX_HOME + bypass flag          → expect proceed (escape hatch)
# (open bug #14345 context: trust behaviour in exec is unstable across versions.)
PROBE=q4
. "$(dirname "$0")/00-common.sh"

WORK="$(mktemp -d)"; ( cd "$WORK" && git init -q )

run_case() {
  local name="$1"; shift
  local home="$1"; shift
  local d; d="$(probe_out "q4/$name")"
  require_codex "$d"
  CODEX_HOME="$home" codex exec --json -a never -s workspace-write -C "$WORK" "$@" \
    "Print the word READY and stop." \
    > "$d/stdout.jsonl" 2> "$d/stderr.log"
  echo "$?" > "$d/exit-code"
  log "$name: exit=$(cat "$d/exit-code"); trust/approval mentions:"
  grep -iE 'trust|approv|not trusted|sandbox' "$d/stderr.log" "$d/stdout.jsonl" | head -5 >&2 || true
}

# Case A — fresh, unseeded.
A_HOME="$(mktemp -d)"
run_case A-fresh-unseeded "$A_HOME"

# Case B — fresh + seeded trust for the working tree.
B_HOME="$(mktemp -d)"
cat > "$B_HOME/config.toml" <<EOF
[projects."$WORK"]
trust_level = "trusted"
EOF
run_case B-seeded-trusted "$B_HOME"
cp "$B_HOME/config.toml" "$(probe_out q4/B-seeded-trusted)/seeded-config.toml"

# Case C — fresh + bypass flag (last resort; also disables sandbox).
C_HOME="$(mktemp -d)"
run_case C-bypass "$C_HOME" --dangerously-bypass-approvals-and-sandbox

log "VERDICT INPUT: compare exit codes across A/B/C."
log "If A blocks and B proceeds → doctor pre-seeds [projects.<path>].trust_level=trusted."
log "If A already proceeds → no seeding needed on this version (record the version!)."
rm -rf "$WORK" "$A_HOME" "$B_HOME" "$C_HOME"
