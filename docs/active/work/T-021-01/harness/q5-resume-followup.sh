#!/usr/bin/env bash
# Q5 — Follow-up via `codex exec resume`.
# lisa's Claude path sends a `finish_up_prompt` into a stuck Review pane to nudge
# it to completion (build_notify / finish_up_prompt in lisa-plugin). The Codex
# analog is a fresh turn against the SAME thread: `codex exec resume <thread_id>`
# with new instructions. This must reliably continue the session (see prior state)
# rather than start cold — otherwise T-023-02 has no follow-up mechanism.
#
# Method: run turn 1, capture thread_id from `thread.started`, then resume with a
# follow-up that can only be answered correctly if turn-1 context survived.
PROBE=q5
. "$(dirname "$0")/00-common.sh"

OUT="$(probe_out q5)"
require_codex "$OUT"

WORK="$(mktemp -d)"; ( cd "$WORK" && git init -q )

# Turn 1 — establish a fact in-session.
codex exec --json -a never -s workspace-write -C "$WORK" \
  "Remember this codeword: MARMALADE. Acknowledge and stop." \
  > "$OUT/turn1.jsonl" 2> "$OUT/turn1.stderr.log"
echo "$?" > "$OUT/turn1.exit"

THREAD_ID="$(grep -o '"thread_id"[[:space:]]*:[[:space:]]*"[^"]*"' "$OUT/turn1.jsonl" \
  | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
printf '%s\n' "$THREAD_ID" > "$OUT/thread-id.txt"
[ -n "$THREAD_ID" ] || fail "no thread_id captured from turn 1 — resume cannot be tested"
log "captured thread_id: $THREAD_ID"

# Turn 2 — resume and ask for the remembered fact.
codex exec resume "$THREAD_ID" --json -a never -s workspace-write -C "$WORK" \
  "What was the codeword I asked you to remember? Answer in one word and stop." \
  > "$OUT/turn2.jsonl" 2> "$OUT/turn2.stderr.log"
echo "$?" > "$OUT/turn2.exit"

grep -io 'MARMALADE' "$OUT/turn2.jsonl" | head -1 > "$OUT/recall.txt" || true

log "VERDICT INPUT: recall.txt = [$(cat "$OUT/recall.txt")], turn2 exit=$(cat "$OUT/turn2.exit")"
log "PASS if turn 2 recalls MARMALADE → resume carries context → finish_up analog works."
log "Also try 'codex exec resume --last' as an alternative if thread_id capture is flaky."
rm -rf "$WORK"
