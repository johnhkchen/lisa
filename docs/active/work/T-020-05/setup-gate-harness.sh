#!/usr/bin/env bash
# T-020-05 — S-020 interactive-gate dry-run harness.
# Builds a throwaway, instrumented project so a real `lisa loop` exercises the
# AskUserQuestion block/resume cycle observably. No production code is touched.
#
# Usage:   ./setup-gate-harness.sh [DEST_DIR]
#          (default DEST_DIR=/tmp/lisa-gate-dryrun)
set -euo pipefail

DEST="${1:-/tmp/lisa-gate-dryrun}"
# Repo root = three levels up from docs/active/work/T-020-05/
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

echo "==> Building WASM plugin (release) then CLI so the embedded plugin is current…"
# Must build the plugin FIRST (build.rs embeds target/wasm32-wasip1/release/lisa.wasm);
# the touch forces cargo to re-copy the fresh WASM into the CLI. Mirrors `just build-cli`.
( cd "$REPO" \
  && cargo build -q -p lisa-plugin --target wasm32-wasip1 --release \
  && touch target/wasm32-wasip1/release/lisa.wasm \
  && cargo build -q -p lisa-cli --release )
LISA="$REPO/target/release/lisa"
[ -x "$LISA" ] || { echo "build failed: $LISA missing"; exit 1; }

echo "==> Scaffolding throwaway project at $DEST"
rm -rf "$DEST"; mkdir -p "$DEST"; cd "$DEST"
git init -q
git config user.email gate@test.local; git config user.name gate
cat > CLAUDE.md <<'MD'
# Gate Dry-Run

Throwaway project for the S-020 AskUserQuestion interactive gate.

### Build and Test
```bash
true
```
MD
mkdir -p docs/active/tickets docs/active/work

echo "==> lisa init"
"$LISA" init >/dev/null

# --- Trigger ticket: force an AskUserQuestion as the very first action ----------
cat > docs/active/tickets/T-GATE-01-ask.md <<'MD'
---
id: T-GATE-01
title: ask-the-operator
type: task
status: open
priority: high
phase: ready
depends_on: []
---

## Context

This is a gate test. Your **very first action**, before reading any files or doing
any research, MUST be to call the `AskUserQuestion` tool to ask the operator:
"Gate test: choose approach A or approach B?" with two options (A and B). Do not take
any other action until the operator answers. After they answer, simply acknowledge
the choice in one sentence and stop.

## Acceptance Criteria

- The `AskUserQuestion` tool is invoked as the first action.
- After the answer, the agent acknowledges the selected option and stops.
MD

# --- Logging on-notify: capture attention/question + completion -----------------
cat > .lisa/hooks/on-notify <<'EOF'
#!/bin/sh
echo "$(date -u +%H:%M:%SZ) on-notify EVENT=$1 LISA_REASON=$LISA_REASON DETAIL=$2" >> .lisa/on-notify.log
EOF
chmod +x .lisa/hooks/on-notify

# --- Instrument the scaffolded lifecycle hooks with a persistent trace line -----
# Each on-*.sh keeps its normal signal-write; we append a timestamped trace so the
# question -> block -> resume sequence is reviewable after the run.
for f in on-idle on-stop on-clear on-heartbeat; do
  hook=".lisa/hooks/$f.sh"
  [ -f "$hook" ] || continue
  if ! grep -q "GATE-TRACE" "$hook"; then
    printf '\n# GATE-TRACE\necho "$(date -u +%%H:%%M:%%SZ) %s pane=$LISA_PANE_ID" >> .lisa/trace.log\n' "$f" >> "$hook"
  fi
done

: > .lisa/trace.log; : > .lisa/on-notify.log

cat <<EOF

================================================================================
 Gate dry-run ready at: $DEST
================================================================================

 RUN IT:
     cd "$DEST" && "$LISA" loop

 WHAT TO WATCH (live, in the zellij dashboard):
   1. The agent for T-GATE-01 calls AskUserQuestion → its pane shows the question.
   2. That pane gets an [AWAITING] marker on the dashboard.
   3. lisa does NOT type /clear or a prompt over the question. If a timeout path
      tries, the activity log shows: "Suppressed injection into pane N (awaiting human)".
   4. Answer the question in the pane.
   5. The agent resumes and the [AWAITING] marker clears.

 CHECK AFTER (persistent evidence):
     cat "$DEST/.lisa/on-notify.log"   # expect a line: on-notify EVENT=attention LISA_REASON=question
     cat "$DEST/.lisa/trace.log"       # expect a 'heartbeat pane=N' line AFTER you answered (= resume)

 PASS  = block (no clobber) + marker appears + marker clears + post-answer heartbeat.
 FAIL  = pane /clear'd or prompt typed over the question, OR no [AWAITING] marker,
         OR marker never clears after answering.
================================================================================
EOF
