# T-019-01 Plan — Plugin notification emit

Ordered, independently-verifiable steps. Single file: `crates/lisa-plugin/src/lib.rs`.
Verification gate throughout: `cargo build -p lisa-plugin --target wasm32-wasip1`
(WASM compile) and `cargo test --workspace` (native) — i.e. `just check`.

## Step 1 — State fields
Add `project_root: PathBuf`, `notified_attention: HashSet<u32>`,
`loop_started_at: Option<SystemTime>` to `State` (`lib.rs:156`).
**Verify:** `cargo build` for native + wasm compiles (Default derive covers them).
Atomic commit candidate.

## Step 2 — Pure builder `build_notify_command`
Add the host-free associated fn (Structure §C) in `impl State`.
**Verify:** compiles; write tests 1 & 2 (Step 7) can come now or later. Pure
function, no host deps — safe to land alone.

## Step 3 — `fire_notify` wrapper
Add `fire_notify` (Structure §D) calling
`run_command_with_env_variables_and_cwd`.
**Verify:** WASM compile resolves the symbol; native compiles (symbol exists in
zellij_tile stubs). Not called yet → dead-code warning is acceptable mid-step, or
land together with Steps 5–6 to avoid the warning.

## Step 4 — Permission + subscription + root capture in `load()`
- Add `EventType::RunCommandResult` to `subscribe(...)` (`lib.rs:2291`).
- Add `PermissionType::RunCommands` to `request_permission(...)` (`lib.rs:2299`).
- Capture `self.project_root = get_plugin_ids().initial_cwd;` and
  `self.loop_started_at = Some(SystemTime::now());` after `signal_dir` setup.
**Verify:** WASM compile. (Runtime: Zellij will re-prompt for the new permission
on next launch — expected; documented in review.)

## Step 5 — `complete` fire site
Insert the `complete` block in `poll_tick` between `AllTicketsDone` log and
`terminated = true` (Structure §E). Compute `tickets_done` + env locals first.
**Verify:** WASM + native compile; existing `check_all_done`/termination tests
still pass (they don't assert log length at that point).

## Step 6 — `attention` fire site + debounce
- Lift `idle_pane_id: Option<u32>` in `check_idle_signals` (Structure §F.1).
- Add the debounced `fire_notify("attention", ...)` in the IdleWithoutArtifact
  branch (Structure §F.2).
- Add `notified_attention.remove(&pane_id)` in `check_heartbeat_signals`
  (Structure §G).
**Verify:** native — existing idle-without-artifact test
(`test_idle_signal_*_without_artifact`) still passes (alert + warning unchanged;
notify path is host-only and inert in tests because `project_root` is empty and
the host call is never executed natively... see note). WASM compile.

> Note: in native tests `fire_notify` would call the host `run_command` fn which
> is a stub that may panic. The existing idle-without-artifact test reaches this
> branch. To keep it host-free, the notify call only runs when `idle_pane_id`
> is `Some` AND we are not in a unit test. We avoid a test-only flag by relying on
> the fact that the existing failing-path tests use pane-based signals → they WILL
> hit `fire_notify`. **Mitigation:** gate the actual host call inside
> `fire_notify` so the builder is exercised but the host call is skipped when
> `project_root` is empty (tests leave it empty/default). I.e. `fire_notify`
> early-returns if `self.project_root.as_os_str().is_empty()`. This is a natural
> guard (no real project → nothing to invoke) and keeps every existing native
> test host-free. Document this in review.

## Step 7 — Result handling in `update()`
Add the `Event::RunCommandResult` arm (Structure §H).
**Verify:** WASM + native compile.

## Step 8 — Tests
Add tests 1–3 (Structure §I):
- `test_build_notify_command_complete`
- `test_build_notify_command_attention`
- `test_attention_debounce_add_and_clear`
Also add a regression test that the idle-without-artifact path with a non-empty
`project_root` left empty (default) does NOT panic — i.e. the existing
`..._without_artifact` test continues to pass, proving the empty-root guard.
**Verify:** `cargo test --workspace` green.

## Step 9 — Full gate
Run `just check` (WASM check + workspace tests) and `cargo clippy` on the plugin.
Resolve any warnings introduced (unused imports, dead code). Confirm no
behavioral change to scheduling/phase advancement.

## Testing strategy

- **Unit (native):** builder argv/env for both events; debounce set
  add/skip/clear; empty-root guard prevents host call in the existing
  idle-without-artifact test.
- **Compile-only (wasm):** permission/subscription/event-arm/host-call typecheck
  against zellij-tile 0.43 — the real runtime behavior (Zellij actually spawning
  the hook) is not unit-testable, consistent with all existing `write_*`/`set_timeout`
  usage.
- **Not covered (documented):** end-to-end firing of a real `on-notify` script
  under a live `lisa loop` — manual/integration, out of scope for native tests.

## Risks & mitigations

- *New permission prompt on upgrade:* Zellij re-requests on the changed permission
  set; users approve once. Note in review.
- *Native test panic via host call:* mitigated by the empty-`project_root` guard
  in `fire_notify` (Step 6 note).
- *Borrow conflicts at fire sites:* mitigated by computing owned locals before the
  `self.fire_notify` call.
- *Log noise:* one Info per fired event; failures as Warning. Acceptable volume.
</content>
