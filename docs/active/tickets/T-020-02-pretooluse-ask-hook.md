---
id: T-020-02
story: S-020
title: pretooluse-ask-hook
type: feature
status: open
priority: high
phase: done
depends_on: [T-020-01]
---

## Context

Bind a `PreToolUse[AskUserQuestion]` Claude Code hook that fires the S-019
`on-notify` script when an agent asks the user a question, and writes a
`pane-<id>.awaiting` signal for the plugin (consumed in T-020-03). This is the
hook/notify half of S-020; the spike (T-020-01) confirmed the gate is **GO** and
captured a real payload (`docs/active/work/T-020-01/`).

The notify fires **from the Claude Code hook directly** (like the permission
catch-all path), not from the plugin — so no `run_command` plumbing is needed.
Reuses the S-019 `on-notify` contract verbatim, adding a new
`LISA_REASON=question` value (no new user hook).

Touches `crates/lisa-cli/src/templates.rs` and `init.rs` only.

Key anchors (verify before editing; from spike design):
- lisa binds **no** `PreToolUse` today — `settings_local_json()` (~`templates.rs:74-123`,
  spike cites `:116`) and `merge_hooks()` (~`templates.rs:204-243`, spike cites `:296`).
- Existing attention catch-all command to mirror — `NOTIFY_ATTENTION_COMMAND` style
  (spike cites `templates.rs:110`).
- validate expected-keys/filenames — `init.rs:654`, `init.rs:680-708`.
- The `on-notify` contract + env vars live in S-019; `hooks-guide.md` documents them.

## Acceptance Criteria

- **Step 1 (gate-closer, do first):** validate interactively under a **real `lisa loop`**
  that an agent's `AskUserQuestion` (a) fires the `PreToolUse` hook, (b) blocks the pane
  until answered, and (c) resumes with a `PostToolUse` heartbeat after answering. The
  spike proved the hook fires under `bypassPermissions` headless, but not the interactive
  block/answer cycle (T-020-01 design Q2/Q4 residual risk). Record the result in this
  ticket's `progress.md`. If blocking/resume does not behave as designed, stop and
  reassess before the plugin ticket.
- New hook command constant (e.g. `ON_PRETOOL_ASK_*` / inline command) added to
  `templates.rs`. The command must, POSIX `sh`-only (no `jq`, no bashisms):
  - Write `.lisa/signals/pane-$LISA_PANE_ID.awaiting` (timestamp line, mirroring the other
    signal hooks).
  - Best-effort extract the first question text via `sed` (design Q3):
    ```sh
    in=$(cat)
    q=$(printf '%s' "$in" | sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p')
    [ -z "$q" ] && q="agent is asking a question"
    ```
    Degrade to a generic "needs you" message on extraction miss — never a hard failure.
  - Fire `test -x .lisa/hooks/on-notify && .lisa/hooks/on-notify attention "$q"` with
    `LISA_REASON=question` (and `LISA_EVENT=attention`, `LISA_PANE_ID` already exported).
- Sixth hook binding added to both `settings_local_json()` and `merge_hooks()`:
  a **matcher `"AskUserQuestion"` `PreToolUse`** entry. In `merge_hooks`, dedup by command
  substring (like the Notification catch-all) so it doesn't collide with the future
  matcher-less heartbeat `PostToolUse`. Add a unit test: merge into settings that already
  has all five hooks → the new PreToolUse entry is added, present alongside the others,
  idempotent on re-run.
- `lisa init` scaffolds the new hook script (if a separate `.sh` file) through the
  hook-scripts array + chmod loop; settings merge covers the binding automatically.
- Update `lisa validate` expected-keys/filenames (`init.rs:654`, `init.rs:680-708`) and the
  init plan-count test(s) to include the new artifact(s).
- Update `crates/lisa-cli/data/hooks-guide.md` to document the `question` reason and the
  PreToolUse hook (the doc belongs with the guide from T-019-03).
- Tests: `templates.rs` merge/idempotency + the payload-extraction `sed` against the
  captured `pretooluse-payload-sample.json` (a question with and without an escaped quote).
- `just check` passes.

## Implementation notes

- Do **not** over-parse the payload in the hook (design Q3): extract a short best-effort
  detail; the user hook can inspect more if it wants. A `question` containing `\"` will
  truncate the greedy-free `[^"]*` — that's acceptable as long as it degrades to the
  generic message.
- This ticket only *writes* the `.awaiting` signal; the plugin does not consume it until
  T-020-03. Writing the signal early is harmless (an unread file the plugin ignores).
