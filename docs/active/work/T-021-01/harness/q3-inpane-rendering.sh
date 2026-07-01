#!/usr/bin/env bash
# Q3 — In-pane rendering under `--json`.
# The observability requirement (doc 05 §Observability) needs the pane to SHOW the
# conversation. The wrapper reads stdout JSON; the question is what codex still
# prints on STDERR under --json, and at what granularity. This single answer picks:
#   - tee-stderr        (stderr already carries rich human progress)  → cheapest
#   - render-from-JSON  (stderr is only a spinner)                    → wrapper renders
#   - escalate to app-server (need token-by-token streaming)          → heavier
#
# Also probes: does exec emit PARTIAL assistant text (deltas) or only COMPLETED
# messages? We grep the JSONL for *delta* item types to decide.
#
# Method: run with stdout piped to a file (as the wrapper would) and stderr piped
# to a SEPARATE file so we can inspect exactly what a Zellij pane would show if we
# tee'd stderr. We do NOT inherit stderr to a tty here — we capture it for review.
PROBE=q3
. "$(dirname "$0")/00-common.sh"

OUT="$(probe_out q3)"
require_codex "$OUT"

WORK="$(mktemp -d)"; ( cd "$WORK" && git init -q )

codex exec --json -a never -s workspace-write -C "$WORK" \
  "Briefly explain, in three sentences, what a topological sort is. Then stop." \
  > "$OUT/stdout.jsonl" 2> "$OUT/stderr.log"
echo "$?" > "$OUT/exit-code"

# How much and what kind of content landed on stderr?
{
  echo "stderr bytes: $(wc -c < "$OUT/stderr.log")"
  echo "stderr lines: $(wc -l < "$OUT/stderr.log")"
  echo "stderr non-blank sample (first 20 lines):"
  grep -v '^[[:space:]]*$' "$OUT/stderr.log" | head -20
} > "$OUT/stderr-analysis.txt"

# Partial vs. completed assistant text on stdout.
{
  echo "delta-style item events (partial text):"
  grep -oE '"[a-z_./]*delta[a-z_./]*"' "$OUT/stdout.jsonl" | sort | uniq -c
  echo "completed assistant/agent message items:"
  grep -cE 'agent_message|assistant.*completed|item.completed' "$OUT/stdout.jsonl"
} > "$OUT/granularity.txt"

log "VERDICT INPUT: stderr-analysis.txt (rich vs spinner) + granularity.txt (delta vs completed)"
log "DECISION: rich stderr → tee-stderr; spinner-only + completed-only items → render-from-JSON;"
log "          need deltas but exec gives none → flag app-server escalation for T-023-01."
rm -rf "$WORK"
