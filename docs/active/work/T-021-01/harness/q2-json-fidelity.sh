#!/usr/bin/env bash
# Q2 — `--json` fidelity under a real RDSPI-style ticket.
# Runs one representative ticket prompt through `codex exec --json` with MCP/tools
# active and records EVERY event plus the exit code. We are testing three claims:
#   - #15451: `--json` events silently dropped when MCP/tools are active.
#   - #14691: item statuses misreport (abandoned / wrong status) at turn end.
#   - the anchor rule: turn.completed/turn.failed + process exit are authoritative,
#     item.* is best-effort.
#
# Method: feed fixtures/rdspi-ticket-prompt.txt (a small but real multi-step task
# that forces file reads, a shell command, and a write — i.e. tool activity), then
# tally event types and cross-check the final turn event against the exit code.
PROBE=q2
. "$(dirname "$0")/00-common.sh"

OUT="$(probe_out q2)"
require_codex "$OUT"

PROMPT_FILE="$HARNESS_DIR/fixtures/rdspi-ticket-prompt.txt"
[ -f "$PROMPT_FILE" ] || fail "missing fixture: $PROMPT_FILE"

# Run inside a throwaway git repo so workspace-write + tool calls behave like a
# real ticket working tree.
WORK="$(mktemp -d)"
( cd "$WORK" && git init -q )

codex exec --json -a never -s workspace-write -C "$WORK" \
  "$(cat "$PROMPT_FILE")" \
  > "$OUT/stdout.jsonl" 2> "$OUT/stderr.log"
echo "$?" > "$OUT/exit-code"

# Tally event types (the "type"/event-name field of each JSONL line).
grep -oE '"(type|msg)"[[:space:]]*:[[:space:]]*"[^"]+"' "$OUT/stdout.jsonl" \
  | sort | uniq -c > "$OUT/event-histogram.txt" || true

# Presence checks for the anchor events + the item-status bug surface.
{
  echo "thread.started: $(grep -c 'thread.started' "$OUT/stdout.jsonl")"
  echo "turn.started:   $(grep -c 'turn.started'   "$OUT/stdout.jsonl")"
  echo "turn.completed: $(grep -c 'turn.completed' "$OUT/stdout.jsonl")"
  echo "turn.failed:    $(grep -c 'turn.failed'    "$OUT/stdout.jsonl")"
  echo "item.completed: $(grep -c 'item.completed' "$OUT/stdout.jsonl")"
  echo "command_execution items: $(grep -c 'command_execution' "$OUT/stdout.jsonl")"
  echo "exit-code:      $(cat "$OUT/exit-code")"
} > "$OUT/anchor-check.txt"

log "VERDICT INPUT: see anchor-check.txt + event-histogram.txt"
log "PASS if a terminal turn.completed/turn.failed exists AND agrees with exit-code,"
log "and if tool activity (command_execution items) is present rather than silently dropped."
rm -rf "$WORK"
