#!/usr/bin/env bash
# T-024-01 — Codex loop parity validation harness.
#
# Stands up a throwaway, real-`lisa init` project configured for the Codex client
# with a small multi-ticket DAG, then prints the run command + a PASS/FAIL
# runbook. The scheduler-side parity behaviours are already proven automatically
# by the `tests::test_codex_*` composition tests in crates/lisa-plugin/src/lib.rs;
# this harness covers the irreducible live remainder — the real `codex exec`
# spawn/stream, real signal files, real pane rendering, and `--resume` re-entry —
# which no CI can run (no `codex` binary in CI). It asserts nothing itself: the
# live run is intrinsic, judged from durable artifacts after the fact.
#
# No production code is touched (validation only).
#
# Usage:   ./validate-codex-loop.sh [DEST_DIR]
#          (default DEST_DIR=/tmp/lisa-codex-dryrun)
set -euo pipefail

DEST="${1:-/tmp/lisa-codex-dryrun}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Repo root = four levels up from docs/active/work/T-024-01/
REPO="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

echo "==> Building WASM plugin (release) then CLI so the embedded plugin is current…"
# Plugin FIRST (build.rs embeds target/wasm32-wasip1/release/lisa.wasm); the touch
# forces cargo to re-copy the fresh WASM into the CLI. Mirrors `just build-cli`.
# Without this the dry run could exercise a stale plugin and mis-report PASS/FAIL.
( cd "$REPO" \
  && cargo build -q -p lisa-plugin --target wasm32-wasip1 --release \
  && touch target/wasm32-wasip1/release/lisa.wasm \
  && cargo build -q -p lisa-cli --release )
LISA="$REPO/target/release/lisa"
[ -x "$LISA" ] || { echo "build failed: $LISA missing"; exit 1; }

echo "==> Scaffolding throwaway project at $DEST"
rm -rf "$DEST"; mkdir -p "$DEST"; cd "$DEST"
git init -q
git config user.email codex@test.local; git config user.name codex
cat > CLAUDE.md <<'MD'
# Codex Dry-Run

Throwaway project for T-024-01 Codex loop parity validation.

### Build and Test
```bash
true
```
MD
mkdir -p docs/active/tickets docs/active/work

echo "==> lisa init"
"$LISA" init >/dev/null

# Select the Codex client in .lisa.toml (the S-025 "documented toggle"). Insert
# `client = "codex"` immediately after the [agent] section header so it lands
# under the right table, not as a stray top-level key.
awk '{ print } /^\[agent\]$/ { print "client = \"codex\"" }' .lisa.toml > .lisa.toml.new
mv .lisa.toml.new .lisa.toml
echo "==> .lisa.toml [agent] client:"; grep -A1 '^\[agent\]' .lisa.toml

# --- Small multi-ticket DAG: T-CDX-02 depends on T-CDX-01 ----------------------
cat > docs/active/tickets/T-CDX-01-first.md <<'MD'
---
id: T-CDX-01
title: codex-first
type: task
status: open
priority: high
phase: research
depends_on: []
---

## Context

Codex parity smoke ticket #1. Work the RDSPI phases normally and write each
artifact to docs/active/work/T-CDX-01/. Keep each phase small — this is a
liveness/transition smoke test, not real work.

## Acceptance Criteria

- All six RDSPI artifacts produced; the loop advances the phase on each.
MD

cat > docs/active/tickets/T-CDX-02-second.md <<'MD'
---
id: T-CDX-02
title: codex-second
type: task
status: open
priority: high
phase: research
depends_on: [T-CDX-01]
---

## Context

Codex parity smoke ticket #2. Blocked until T-CDX-01 reaches Done — proves the
DAG gate holds under the Codex adapter. Same "keep it small" guidance.

## Acceptance Criteria

- Does not start until T-CDX-01 is Done; then completes all six phases.
MD

# --- Preflight: warn (do not fail) if codex is missing -------------------------
if ! command -v codex >/dev/null 2>&1; then
  echo "!! WARNING: 'codex' is not on PATH. The scaffold is ready, but the live"
  echo "!! run below needs a real codex binary (rust-v0.142.5+). Install it first."
fi

echo
echo "======================================================================"
echo " T-024-01 — Codex loop parity: LIVE RUN + PASS/FAIL RUNBOOK"
echo "======================================================================"
echo
echo "Run it:"
echo "    cd $DEST && $LISA loop"
echo "  (client is set in .lisa.toml; equivalently: $LISA loop --client codex)"

# Quoted delimiter: the runbook body is literal (its backticks/keywords must NOT
# be executed or expanded by the shell).
cat <<'RUNBOOK'

Watch two Codex panes come up (T-CDX-01 first; T-CDX-02 unblocks after it is
Done). Judge PASS/FAIL from these observables — durable files, not a glance:

 1. SPAWN / STREAM / RENDER
    PASS: each pane runs `lisa agent-exec` → `codex exec --json`; the pane shows
          the chunked human render ("$ cmd", "~ file", "— turn complete …").
    FAIL: pane errors immediately, or shows raw JSON only (render pluck-key drift
          — reconcile event names vs agent_exec.rs, T-023-01 open-concern #1).

 2. PHASES ADVANCE ON ARTIFACTS (all six)
    Observe: docs/active/work/T-CDX-01/{research,design,structure,plan,progress,
             review}.md appear; the ticket's `phase:` field walks forward.
    PASS: phase advances as each artifact lands (no reliance on any .idle).
    FAIL: artifacts present but phase stuck.

 3. .stopped → REVIEW AUTO-COMPLETE, DEPS RESPECTED
    PASS: after Review, T-CDX-01 flips to phase: done / status: done, and only
          THEN does T-CDX-02 start.
    FAIL: T-CDX-02 starts before T-CDX-01 is Done, or Review never completes.

 4. LIVENESS (heartbeat honest; genuine hang reclaimed)
    PASS: a long tool-free stretch (a slow codex turn) does NOT trip a stuck
          alert while item.* heartbeats flow (watch .lisa/signals/pane-*.heartbeat
          mtime advance). A genuinely killed codex (Ctrl-C the pane) IS reclaimed
          within ~2× stuck_threshold and the ticket re-enters the ready set.
    FAIL: an actively-streaming pane is reclaimed, or a dead pane never is.

 5. FORCED FAILURE (.error fails promptly)
    Force it: in a pane, run a prompt that makes codex exit non-zero / turn.failed.
    PASS: .lisa/signals/pane-<id>.error appears; the dashboard shows ✗ FAILED for
          that ticket within a poll or two (NOT after ~40 min of silence); the
          slot is released and the ticket re-schedules.
    FAIL: failure only surfaces via the silence timeout.

 6. REVIEW-TIMEOUT FINISH-UP via agent-exec --resume
    Trigger: let a Review session go quiet past review_timeout_secs + wind_down.
    PASS: the pane receives a `lisa agent-exec --resume "…finish up…"` line; codex
          re-enters the persisted thread (.lisa/codex/<key>.thread) and completes.
    FAIL: a Claude-shaped prompt is typed, or no resume line appears.

 7. DASHBOARD SANE — NO PHANTOM "AWAITING"
    PASS: no Codex pane ever shows the [AWAITING] marker (Codex emits no .awaiting).
    FAIL: an [AWAITING] marker appears for a Codex pane.

Mixed Claude+Codex: single-loop client mixing is loop-wide-`client`-gated (S-026).
Validate attribution by running this loop (codex) and a second Claude loop in a
separate project; confirm each pane's signals land in its own .lisa/signals/
pane-<id>.* and never cross. (Per-pane attribution is unit-proven by
tests::test_mixed_panes_error_attributed_per_pane.)

Durable evidence to keep: the ticket files' final `phase:`/`status:`, the
.lisa/signals/ directory contents over time, and .lisa/codex/<key>.thread.
The scaffold is left in place for re-inspection; the next run rm -rf's it.

Any behaviour that violates the contract above is a bug blocking S-025's
"documented toggle" claim — file it and reference this runbook.
======================================================================
RUNBOOK
