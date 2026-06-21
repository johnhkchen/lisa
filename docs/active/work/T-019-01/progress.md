# T-019-01 Progress — Plugin notification emit

Status: **complete**. All plan steps executed; `just check` green.

## Steps completed

- [x] **Step 1 — State fields.** Added `project_root: PathBuf`,
  `notified_attention: HashSet<u32>`, `loop_started_at: Option<SystemTime>` to
  `State`. All `Default`-able; `#[derive(Default)]` unchanged.
- [x] **Step 2 — `build_notify_command`.** Pure host-free associated fn returning
  `(Vec<String>, BTreeMap<String,String>)`. Builds absolute hook path + env, and
  the `sh -c` argv with the `if [ -x ]` guard (exit 0 when hook absent).
- [x] **Step 3 — `fire_notify`.** Wraps `run_command_with_env_variables_and_cwd`.
  Early-returns when `project_root` is empty (keeps native tests host-free).
  Tags the call with `context = {"lisa_notify": event}`.
- [x] **Step 4 — `load()`.** Added `EventType::RunCommandResult` subscription,
  `PermissionType::RunCommands` permission, and capture of
  `get_plugin_ids().initial_cwd` + loop-start timestamp.
- [x] **Step 5 — `complete` fire site.** In `poll_tick`, between
  `log_activity(AllTicketsDone)` and `terminated = true`: counts Done tickets,
  adds `LISA_TICKETS_DONE` + optional `LISA_DURATION_SECS`, fires `complete`.
- [x] **Step 6 — `attention` fire site + debounce.** Lifted `idle_pane_id:
  Option<u32>` in `check_idle_signals`; debounced `fire_notify("attention", …)`
  in the IdleWithoutArtifact branch; `notified_attention.remove(&pane_id)` in
  `check_heartbeat_signals`.
- [x] **Step 7 — Result handling.** `Event::RunCommandResult` arm in `update()`:
  Info on exit 0, Warning on non-zero/None, filtered by the `lisa_notify` context
  key.
- [x] **Step 8 — Tests.** Added `test_build_notify_command_complete`,
  `test_build_notify_command_attention`, `test_attention_debounce_add_skip_and_clear`,
  `test_fire_notify_noop_when_project_root_empty`.
- [x] **Step 9 — Gate.** `just check` passes; `cargo clippy -p lisa-plugin
  --target wasm32-wasip1` clean.

## Deviations from the plan / design

1. **API: `run_command_with_env_variables_and_cwd`, not
   `run_command_with_env_variables`.** The no-cwd helper named in the ticket does
   not exist in zellij-tile 0.43 (only `run_command` and the `_and_cwd` form).
   Used the `_and_cwd` variant with cwd = `project_root`. (Decided in design;
   recording here as a divergence from the ticket's literal wording.)

2. **Absolute paths via `get_plugin_ids().initial_cwd`, not
   `strip_host_prefix`.** The ticket suggested
   `strip_host_prefix(host.join(".lisa/hooks/on-notify"))`, which yields a
   *relative* path and cannot supply an absolute `LISA_PROJECT`. `initial_cwd` is
   the real host root, giving absolute hook path, `LISA_PROJECT`, and cwd.
   `strip_host_prefix` is left untouched (still used elsewhere).

3. **Guard uses `if [ -x ]; then …; fi`, not `test -x … && …`.** The ticket's
   `&&` example exits 1 when the hook is absent, which would surface as a spurious
   failure in `RunCommandResult`. The `if` form exits 0 on an absent hook (true
   silent no-op) while preserving the hook's real exit code on failure.

4. **`fire_notify` empty-root guard.** Added an early return when `project_root`
   is empty so that existing native tests reaching the attention branch (which
   build `State` directly, leaving `project_root` default) never invoke the host
   stub. Documented and covered by `test_fire_notify_noop_when_project_root_empty`.

5. **`LISA_DURATION_SECS` implemented, not deferred.** `SystemTime::now()` is
   already used throughout the plugin, so capturing `loop_started_at` in `load()`
   was low-risk; the env var is included on `complete` when available and omitted
   otherwise.

## Verification

- `cargo build -p lisa-plugin --target wasm32-wasip1` — compiles.
- `cargo test --workspace` — 165 + 106 + 164 + 0 doctests pass (lisa-plugin now
  164, +4 new).
- `cargo clippy -p lisa-plugin --target wasm32-wasip1` — no warnings.
- `just check` — green.

## Not committed

Per the loop workflow, changes are staged in the working tree but not committed
(the operator/Lisa handles commits). One logical change confined to
`crates/lisa-plugin/src/lib.rs`.
</content>
