---
id: T-019-01
story: S-019
title: plugin-notification-emit
type: feature
status: open
priority: high
phase: done
depends_on: [T-019-02]
---

## Context

Make the plugin fire the user's `.lisa/hooks/on-notify` script at two moments
via Zellij's `run_command` host API: when the whole loop completes, and when an
agent stalls needing input (`IdleWithoutArtifact`). This is the plugin half of
S-019; it does not require the hook to exist (the script is `test -x` guarded on
the host), so it can land independently of the CLI scaffolding in T-019-02.

Touches `crates/lisa-plugin/src/lib.rs` only. The plugin currently has **no**
command-execution capability — it only writes chars to panes — so this adds a new
permission and a result subscription.

Key anchors (verify before editing; line numbers drift):
- Permission request list — `lib.rs:2299` (currently `WriteToStdin`,
  `ChangeApplicationState`, `ReadApplicationState`).
- Event subscription — `lib.rs:2291`.
- Completion transition — `lib.rs:1548-1553` (`check_all_done()` true →
  `log_activity(AllTicketsDone)`, `terminated = true`, return without re-arming).
- Idle-without-artifact — `lib.rs:879-889` (pushes `idle_alerts`, logs `Warning`).
- Host-path helper — `strip_host_prefix()` at `lib.rs:89`; `host`/`signal_dir`
  built at `lib.rs:2288`.

## Acceptance Criteria

- Add `PermissionType::RunCommands` to the permission request at `lib.rs:2299`,
  and subscribe to `EventType::RunCommandResult` at `lib.rs:2291`.
- Handle `Event::RunCommandResult` in `update()`: log success/failure (exit code)
  as an `ActivityEvent` so hook failures are visible in the dashboard/activity log.
  A missing/non-executable `on-notify` must be a silent no-op (the `test -x` guard
  handles this — a clean exit, not an error).
- Build the hook command as host-absolute:
  `strip_host_prefix(host.join(".lisa/hooks/on-notify"))`, invoked as
  `run_command_with_env_variables(["sh", "-c", "test -x \"$LISA_HOOK\" && \"$LISA_HOOK\" \"$1\" \"$2\"", "sh", <event>, <detail>], env, context)`
  (or equivalent that keeps the `test -x` guard and passes event + detail). Set
  env vars per the S-019 contract.
- **`complete`** fired at `lib.rs:1549`, right after `log_activity(AllTicketsDone)`
  and before the early `return`. Env: `LISA_EVENT=complete`, `LISA_PROJECT`,
  `LISA_TICKETS_DONE` (count of Done tickets), `LISA_DURATION_SECS` (loop wall-clock
  if tracked; omit if not readily available). Fires once per completion — the timer
  isn't re-armed, so the block can't re-run unless `keep_working()` resets
  `terminated`, in which case a later completion correctly re-fires.
- **`attention`** fired in the `IdleWithoutArtifact` branch (`lib.rs:879`). Env:
  `LISA_EVENT=attention`, `LISA_PROJECT`, `LISA_PANE_ID`, `LISA_TICKET=<ticket id>`,
  `LISA_REASON=idle-without-artifact`. Detail string e.g. `"<ticket> idle in <phase> without <artifact>"`.
- **Debounce**: add a `HashSet<PaneId>` (or per-slot flag) of panes already notified
  for `attention`; skip re-firing while the pane stays in that state. Clear a pane's
  entry when its activity is bumped (next heartbeat in `check_heartbeat_signals`,
  `lib.rs:679`) so a resumed-then-re-stalled agent can notify again. This prevents a
  60s-repeating `idle_prompt` from spamming.
- No change to scheduling behavior: do not alter phase advancement, `send_line_to_pane`,
  or the Implement-phase idle handling. Purely additive notification calls.
- Tests (native): debounce set add/clear logic; env/arg map construction for both
  events given a fake project path + ticket. The `run_command_with_env_variables`
  call compiles on the native target (zellij_tile stubs) but is not exercised in
  tests — consistent with existing `set_timeout` / `write_chars_to_pane_id` calls.
- `just check` passes (WASM check + workspace tests).

## Implementation notes

- `run_command` runs on the host; cwd is not guaranteed to be the project root, so
  pass the hook by absolute path and export `LISA_PROJECT` for scripts that need it.
- If `LISA_DURATION_SECS` requires new loop-start timekeeping, it's acceptable to
  defer that single env var to a follow-up rather than expand this ticket.
