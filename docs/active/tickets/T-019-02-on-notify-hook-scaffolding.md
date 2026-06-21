---
id: T-019-02
story: S-019
title: on-notify-hook-scaffolding
type: feature
status: open
priority: high
phase: done
depends_on: []
---

## Context

Add the user-owned `on-notify` hook and the Claude Code `Notification` binding
that fires it for permission/attention prompts, then wire both into `lisa init`
so new and existing projects get the full hook set. This is the CLI half of S-019.
It touches `crates/lisa-cli/src/templates.rs` and `crates/lisa-cli/src/init.rs`
only — no overlap with T-019-01 (plugin crate), so the two run in parallel.

The user also wants `lisa init` to reliably set up **the existing hooks**
(on-idle/on-stop/on-clear/on-heartbeat), not just the new one. `init` already does
this via a plan-then-execute pattern with content-aware merge — this ticket extends
that set and confirms idempotent re-run on an already-configured project.

Key anchors (verify before editing):
- Hook constants — `templates.rs:11/25/39/55` (`ON_IDLE/STOP/CLEAR/HEARTBEAT_HOOK`),
  `LISA_GITIGNORE` at `templates.rs:68`.
- `settings_local_json()` — `templates.rs:74-123`; `Notification[idle_prompt]` array
  at `templates.rs:108-118`.
- `ensure_hook()` — `templates.rs:129-199`; `merge_hooks()` — `templates.rs:204-243`
  (four `ensure_hook` calls, last at `templates.rs:240`).
- init hook-scripts array — `init.rs:320-350`; chmod loop — `init.rs:476-492`;
  settings merge — `init.rs:366-409`; validate expected-keys/filenames —
  `init.rs:647-651` / `init.rs:675`.
- init plan-count test — `init.rs:946` (`creates.len() == 17`, will need bumping).

## Acceptance Criteria

- **`ON_NOTIFY_HOOK`** const added after `templates.rs:65`. A POSIX `sh` script
  scaffolded to `.lisa/hooks/on-notify.sample` (NOT `on-notify`, so the `test -x`
  guard stays inert until the user opts in). It must:
  - Document the `on-notify <event> [detail]` contract and the env vars from S-019
    in comments.
  - Contain a **commented** ntfy.sh example, e.g.:
    ```sh
    # case "$1" in
    #   complete)  msg="lisa done: $LISA_TICKETS_DONE tickets in ${LISA_DURATION_SECS}s" ;;
    #   attention) msg="lisa needs you ($LISA_REASON): $2" ;;
    # esac
    # curl -s -d "$msg" ntfy.sh/your-topic-here
    ```
  - Reference ntfy only as a commented example. Lisa core never names it.
- **Attention Notification binding** added to `settings_local_json()` and
  `merge_hooks()`:
  - A second `Notification` entry that catches non-idle payloads (permission/attention).
    Since matcher semantics for permission payloads aren't guaranteed, use a catch-all
    `Notification` entry (no `matcher`) whose command **skips idle** and otherwise fires
    the user hook, e.g.:
    `command: "test -x .lisa/hooks/on-notify && in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) .lisa/hooks/on-notify attention \"$in\" ;; esac"`
    (read stdin once; do not double-handle `idle_prompt`, which the existing on-idle
    hook + plugin already cover).
  - Set `LISA_EVENT=attention` / `LISA_REASON=permission` via the script or inline env
    where practical (pane already exports `LISA_PANE_ID`).
  - Add a fifth `ensure_hook(...)` call in `merge_hooks()` after `templates.rs:240`.
    Confirm `ensure_hook`'s dedup distinguishes the new no-matcher `Notification` entry
    from the existing `idle_prompt`-matched one (different matcher/script path) and does
    not collapse them — add a unit test covering "merge into settings that already has
    the idle_prompt Notification hook" → both entries present, idempotent on re-run.
- **`lisa init` wiring**:
  - Add `("on-notify.sample", templates::ON_NOTIFY_HOOK)` to the hook-scripts array
    (`init.rs:320-350`); ensure it flows through the chmod-executable loop only if you
    want it runnable — a `.sample` need not be `+x`; the *copied* `on-notify` is what the
    user chmods. Decide and document (recommend: scaffold `.sample` non-executable;
    guide tells the user to `cp on-notify.sample on-notify && chmod +x on-notify`).
  - Settings merge already covers the new binding via `merge_hooks`/`settings_local_json`.
  - Update validate expected-keys (`init.rs:647-651`) and filenames (`init.rs:675`) to
    include the new Notification binding / `on-notify.sample`.
  - Re-running `init` on a project that already has the four hooks + settings must
    `Skip` the unchanged ones and only `CreateFile` the new `on-notify.sample` +
    `UpdateFile` settings.local.json (verifies "set up existing ones" is idempotent).
- Bump the `creates.len()` assertion in `test_plan_init_actions_empty_dir`
  (`init.rs:946`) and any other count-based init tests to match the added artifact(s).
- `just check` passes.

## Implementation notes

- Mirror exactly how `on-heartbeat.sh` was previously added (per project memory: the
  same validate/filename/test-count spots needed updating) — that is the checklist.
- Keep the catch-all `Notification` command POSIX `sh`-only: no `jq`, no bashisms.
