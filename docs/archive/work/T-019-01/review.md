# T-019-01 Review — Plugin notification emit

## Summary

The plugin now fires the user's `.lisa/hooks/on-notify` script at the two moments
S-019 targets: `complete` (DAG drained) and `attention` (agent idle in a
non-Implement phase without its expected artifact). Both go through Zellij's
host-side `run_command`, are `test -x`-guarded into a silent no-op when the hook
is absent, and carry the S-019 env/argument contract. All changes are confined to
`crates/lisa-plugin/src/lib.rs`. `just check` is green and clippy is clean.

## Files changed

- **`crates/lisa-plugin/src/lib.rs`** (only file):
  - `State`: +3 fields (`project_root`, `notified_attention`, `loop_started_at`).
  - `impl State`: +`build_notify_command` (pure builder) and +`fire_notify`
    (host-call wrapper with empty-root guard).
  - `load()`: +`RunCommandResult` subscription, +`RunCommands` permission,
    +capture of `get_plugin_ids().initial_cwd` and loop-start time.
  - `poll_tick()`: `complete` fire site (counts Done tickets; `LISA_TICKETS_DONE`,
    optional `LISA_DURATION_SECS`).
  - `check_idle_signals()`: lifted `idle_pane_id`; debounced `attention` fire in
    the IdleWithoutArtifact branch.
  - `check_heartbeat_signals()`: clears the pane's attention-debounce entry.
  - `update()`: `Event::RunCommandResult` arm (Info/Warning by exit code).
  - tests: +4 unit tests.

No files created or deleted. No `lisa-core` or CLI changes.

## Acceptance criteria checklist

- [x] `PermissionType::RunCommands` added; `EventType::RunCommandResult`
  subscribed.
- [x] `Event::RunCommandResult` handled — logs success (Info) / failure (Warning,
  with exit code) as `ActivityEvent`. Missing/non-executable hook is a clean exit
  0 (the `if [ -x ]` guard) → logged as Info, **not** an error.
- [x] Hook command built host-absolute and invoked via `run_command*`, keeping the
  executable guard and passing `event` + `detail` positionally, with env per the
  S-019 contract. *(See divergence notes below — uses the `_and_cwd` API and
  `initial_cwd` instead of `strip_host_prefix`, both stronger choices.)*
- [x] `complete` fired right after `log_activity(AllTicketsDone)` and before the
  early `return`. Env: `LISA_EVENT=complete`, `LISA_PROJECT`, `LISA_TICKETS_DONE`,
  `LISA_DURATION_SECS`. Fires once per completion; re-fires after `keep_working()`.
- [x] `attention` fired in the IdleWithoutArtifact branch. Env:
  `LISA_EVENT=attention`, `LISA_PROJECT`, `LISA_PANE_ID`, `LISA_TICKET`,
  `LISA_REASON=idle-without-artifact`; detail `"<ticket> idle in <phase> without
  <artifact>"`.
- [x] Debounce: `HashSet<u32>` of notified panes; skip while stalled; cleared on
  heartbeat in `check_heartbeat_signals`.
- [x] No scheduling/phase-advancement/`send_line_to_pane`/Implement-idle changes —
  purely additive.
- [x] Tests: debounce add/skip/clear; env+arg map for both events; host call
  compiles natively but is not exercised (empty-root guard).
- [x] `just check` passes.

## Divergences from the ticket wording (intentional, see design.md)

1. **`run_command_with_env_variables_and_cwd`**, not the no-cwd
   `run_command_with_env_variables` — the latter does not exist in zellij-tile
   0.43.
2. **`get_plugin_ids().initial_cwd`** for the absolute host root, not
   `strip_host_prefix` (which yields a relative path and no absolute
   `LISA_PROJECT`). cwd is set to that root too.
3. **`if [ -x "$LISA_HOOK" ]; then … fi`** guard, not `test -x … && …` — the `&&`
   form exits 1 on an absent hook, which would have been logged as a failure.

These are documented in `design.md` and `progress.md`.

## Test coverage

- **Unit (native):** `build_notify_command` argv/env for both `complete` and
  `attention`; debounce add → skip → clear; `fire_notify` no-op on empty root.
  The pre-existing `test_idle_signal_research_without_artifact_alerts` now also
  exercises the new attention branch (insert into debounce set + guarded
  `fire_notify`) and still passes.
- **Compile-only (wasm):** permission/subscription/event-arm/host call typecheck
  against 0.43. WASM build succeeds.
- **Counts:** lisa-plugin 160 → 164 tests; full workspace green; clippy clean.

## Gaps / open concerns for the reviewer

1. **No end-to-end runtime test.** Zellij actually spawning `on-notify` under a
   live `lisa loop` is not unit-testable (same limitation as every existing
   `write_*`/`set_timeout` call). Recommend a one-time manual smoke test: create
   an executable `.lisa/hooks/on-notify` that appends `$1`/env to a log, run a
   tiny loop to completion, and confirm both `complete` and a forced `attention`
   land. The on-notify.sample from T-019-02 plus T-019-03's guide support this.
2. **New permission prompt on upgrade.** Adding `RunCommands` changes the
   permission set, so Zellij will re-prompt users once on next launch. Worth a
   line in release notes.
3. **`LISA_DURATION_SECS` measures plugin-load → completion**, not the human's
   wall-clock since invoking `lisa loop` (a few seconds of startup differ). Close
   enough for a notification; flagged for transparency.
4. **Legacy `{ticket}.idle` signals carry no pane id**, so the `attention` notify
   (and its debounce) is skipped for them — they keep today's alert/warning-only
   behavior. Acceptable: legacy hooks are deprecated; current hooks emit
   `pane-{id}.idle`.
5. **Activity-log volume:** at most one Info per fired event plus one
   RunCommandResult line; failures are Warning. No spam risk given the debounce.

## Risk assessment

Low. Additive, single-file, behind a new permission, with a host-call guard that
keeps all native tests inert. No change to the scheduler hot path. The main thing
a human should verify is the one-time end-to-end smoke test (gap #1).
</content>
