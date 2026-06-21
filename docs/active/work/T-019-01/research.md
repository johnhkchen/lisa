# T-019-01 Research — Plugin notification emit

## Goal (restated, descriptive)

The plugin (`crates/lisa-plugin/src/lib.rs`) must invoke the user's
`.lisa/hooks/on-notify` script at two moments via Zellij's `run_command` host API:

1. `complete` — when the whole loop finishes (DAG drained).
2. `attention` — when an agent stalls in a non-Implement phase without producing
   its expected artifact (`IdleWithoutArtifact`).

This is the *plugin half* of S-019. The CLI half (T-019-02, already `done`)
scaffolds the `on-notify.sample` hook and the Claude Code `Notification` binding.
The plugin does not require the hook to exist — the invocation is `test -x`-guarded
on the host, so a missing/non-executable hook is a clean no-op.

## Current plugin capabilities

The plugin today has **no command-execution capability**. It only:
- writes characters/Enter to agent panes (`write_chars_to_pane_id`,
  `write_to_pane_id`) — `lib.rs:253,262`;
- reads/writes ticket files via `std::fs` (host mounted at `/host`);
- arms timers (`set_timeout`) and reacts to events.

Permissions requested in `load()` (`lib.rs:2299`): `WriteToStdin`,
`ChangeApplicationState`, `ReadApplicationState`. Event subscriptions
(`lib.rs:2291`): `PaneUpdate`, `PermissionRequestResult`, `Timer`, `Key`.

To run a host command the plugin needs **`PermissionType::RunCommands`** and, to
observe the result, a subscription to **`EventType::RunCommandResult`**.

## Zellij 0.43 API reality (verified against the crate source)

`Cargo.toml` pins `zellij-tile = "0.43"` (resolved 0.43.1). Inspecting
`zellij-tile-0.43.1/src/shim.rs` and `zellij-utils-0.43.1/src/data.rs`:

- **No `run_command_with_env_variables`** (the no-cwd helper the ticket text
  suggests) exists in 0.43. Only two functions are available:
  - `run_command(cmd: &[&str], context: BTreeMap<String,String>)` — internally
    passes `env = {}` and `cwd = PathBuf::from(".")`.
  - `run_command_with_env_variables_and_cwd(cmd: &[&str], env:
    BTreeMap<String,String>, cwd: PathBuf, context: BTreeMap<String,String>)`.
  → We must use the **`_and_cwd`** variant (we need env vars).
- `PermissionType::RunCommands` exists (`data.rs:977`).
- `Event::RunCommandResult(Option<i32> exit_code, Vec<u8> stdout, Vec<u8>
  stderr, BTreeMap<String,String> context)` exists (`data.rs:919`).
- `EventType` is a `strum_discriminants` discriminant of `Event`, so
  `EventType::RunCommandResult` is a valid subscription variant.

The `context` map round-trips from the `run_command*` call back to the
`RunCommandResult` event — the standard way to correlate a result with its call.

## Host path resolution (key constraint)

Commands launched by `run_command` execute **on the host**, not inside the WASI
sandbox. Inside the sandbox the project root is mounted at `/host`; on the host
that path is meaningless. The existing `strip_host_prefix()` (`lib.rs:89`) turns
`/host/.lisa/hooks/on-notify` into the *relative* `.lisa/hooks/on-notify`, which
only resolves if the command's cwd is the project root.

`run_command` defaults cwd to `PathBuf::from(".")`, which Zellij resolves to the
directory it was launched from — the project root. **Better:** `get_plugin_ids()`
(`shim.rs:59`) returns `PluginIds { initial_cwd: PathBuf, .. }`, the real
absolute host project root. Capturing `initial_cwd` once in `load()` lets us pass
an **absolute** hook path, an absolute `LISA_PROJECT`, and an absolute cwd —
removing all relative-path fragility. This is the cleanest source of the host
root; `PluginConfig` carries no absolute project path.

## The two fire sites

### `complete` — `lib.rs:1548-1553` (in `poll_tick`)
```rust
if self.check_all_done() {
    self.log_activity(ActivityEvent::AllTicketsDone);
    self.terminated = true;
    return;                 // timer NOT re-armed → fires once
}
```
Fires once per completion because the timer is not re-armed. If `keep_working()`
(`lib.rs:2254`) resets `terminated = false`, a later completion correctly
re-fires. Done-ticket count is derivable from `self.dag.tickets()` filtered on
`phase == Phase::Done`. No loop-start timestamp exists today, so
`LISA_DURATION_SECS` requires a new field (optional per the ticket).

### `attention` — `lib.rs:879-889` (in `check_idle_signals`)
The `IdleWithoutArtifact` branch currently pushes `idle_alerts` and logs a
`Warning`. Scope note: `pane_id` is a `u32` bound **only inside** the
`pane-{id}.idle` parse branch (`lib.rs:738`); the legacy `{ticket}.idle` else
branch (`lib.rs:754`) has no pane id. To use the pane id (debounce key +
`LISA_PANE_ID`) in the match arm, it must be lifted to an `Option<u32>` declared
before the if/else. `current_phase`, `ticket_id`, and `artifact_name` are in
scope at the branch.

Idle prompts repeat (~60s) — without debounce, each repeated `.idle` would
re-fire `attention`. `bump_pane_activity()` (`lib.rs:659`) is called on every
idle/heartbeat/stop/cleared signal — including the idle signal that triggers this
very branch — so it is **not** a safe place to clear a debounce set. Heartbeats
(`check_heartbeat_signals`, `lib.rs:679,699`) prove genuine progress and are the
correct place to clear, letting a resumed-then-re-stalled agent notify again.

## State struct & test conventions

`State` (`lib.rs:156`) derives `#[derive(Default)]` (`lib.rs:154`). Tests build it
with `State { ..State::default() }` and set `signal_dir`, `agent_slots`,
`threads` directly. New fields must be `Default`-able: `project_root: PathBuf`,
`notified_attention: HashSet<u32>`, `loop_started_at: Option<SystemTime>` all
qualify. Host functions (`subscribe`, `request_permission`, `run_command*`) are
only called from `load()`/event handlers, never in tests — tests never call
`load()`. So a host-free, pure builder for the argv+env is the testable seam,
mirroring how `set_timeout`/`write_chars_to_pane_id` calls are compiled but never
exercised natively.

## Environment / argument contract (from S-019)

`on-notify <event> [detail]`, plus env:
- `LISA_EVENT` (`complete`|`attention`), `LISA_PROJECT` (abs root).
- complete: `LISA_TICKETS_DONE`, `LISA_DURATION_SECS`.
- attention: `LISA_PANE_ID`, `LISA_TICKET`, `LISA_REASON=idle-without-artifact`.

## Constraints / assumptions

- POSIX `sh` only in the guard command (matches T-019-02's hook posture).
- The guard must **exit 0 when the hook is absent** (silent no-op). `test -x X &&
  Y` returns 1 when the test fails — so an `if [ -x ]; then ...; fi` form (or a
  `|| true` tail) is required to avoid logging a spurious failure.
- Purely additive: no change to scheduling, phase advancement, `send_line_to_pane`,
  or Implement-phase idle handling.
- `RunCommandResult` handling should keep hook *failures* visible (non-zero exit)
  without spamming the activity log.
</content>
</invoke>
