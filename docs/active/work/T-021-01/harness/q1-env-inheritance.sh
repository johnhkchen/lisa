#!/usr/bin/env bash
# Q1 — Env inheritance.
# Does a wrapper-launched `codex exec` child (and the shell codex spawns for a
# tool call) see LISA_PANE_ID? This underpins deterministic pane attribution:
# if the var does NOT survive the launch → codex → tool-shell chain, the whole
# "wrapper inherits env" premise (doc 05 §reframe) collapses and we fall back to
# passing the pane id in the prompt or via a temp file.
#
# Method: lisa sets `LISA_PANE_ID=<n>` on the launched command. We emulate that
# by exporting it here, then (a) dump the wrapper's own env, and (b) ask codex to
# run `env | grep LISA` as a shell tool call and capture what IT sees.
PROBE=q1
. "$(dirname "$0")/00-common.sh"

OUT="$(probe_out q1)"
require_codex "$OUT"

export LISA_PANE_ID=7
export LISA_TICKET_ID=T-021-01

# (a) Wrapper-level: this must show 7 or the emulation itself is wrong.
env | grep '^LISA_' > "$OUT/wrapper-env.txt" || true
log "wrapper sees: $(cat "$OUT/wrapper-env.txt")"

# (b) codex-child level: force a shell tool call that echoes the var. Under
# -a never + workspace-write codex may or may not run the command; the JSONL
# command_execution item (if present) carries the aggregated_output we grep.
codex exec --json -a never -s workspace-write --skip-git-repo-check \
  "Run exactly this shell command and report its full output verbatim: env | grep LISA_PANE_ID" \
  > "$OUT/stdout.jsonl" 2> "$OUT/stderr.log"
echo "$?" > "$OUT/exit-code"

# Extract what the codex-spawned shell actually saw.
grep -o 'LISA_PANE_ID=[0-9]*' "$OUT/stdout.jsonl" | sort -u > "$OUT/child-saw.txt" || true

log "VERDICT INPUT: child-saw.txt = [$(cat "$OUT/child-saw.txt")], exit=$(cat "$OUT/exit-code")"
log "PASS if child-saw.txt contains LISA_PANE_ID=7"
