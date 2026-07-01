# T-023-02 · Plan — ordered implementation steps

Executable steps with verification. Each is small enough to commit atomically.
Grounded in `structure.md`.

## Step 1 — `lisa_bin` config field (lisa-core)

- `crates/lisa-core/src/types.rs`:
  - Add `pub lisa_bin: Option<String>` to `PluginConfig`.
  - Init `lisa_bin: None` in `new()`.
  - In `from_config_map`, after the `wind_down_secs` branch: parse `lisa_bin`
    (non-empty → `Some`).
  - Test `lisa_bin_round_trips`: map with `lisa_bin` → `Some`; without → `None`;
    empty string → `None`.
- **Verify**: `cargo test -p lisa-core`.

## Step 2 — `CodexAdapter` + provider resolver (lisa-plugin/adapter.rs)

- Add `struct CodexAdapter { lisa_bin: String }` and `CodexAdapter::new(Option
  <&str>)` (fallback `"lisa"`).
- Implement `AgentAdapter`:
  - `launch_command` / `reuse_prompt`: the wrapper line with `ticket_prompt`
    (identical outputs).
  - `reset_strategy` → `FreshExec`.
  - `follow_up` → `SpawnCommand(--resume line with finish_up_prompt)`.
  - `signals` → all `false`.
- Add `enum Provider { Claude, Codex }`, `fn provider_for(&Ticket) -> Provider`
  (returns `Claude`), `pub(crate) fn build_adapter(Provider, Option<&str>)`.
- Widen `resolve_adapter` and `resolve_adapter_or_native` with `lisa_bin:
  Option<&str>`; delegate through `build_adapter`.
- Add the seven adapter tests (structure.md Test surface). Update the existing
  `resolve_adapter*` tests to pass the new arg.
- **Verify**: compiles only once Step 3 supplies the new arg from lib.rs; run
  `cargo test -p lisa-plugin` after Step 3.

## Step 3 — Wire the scheduler (lisa-plugin/lib.rs)

- Add the second arg `self.config.lisa_bin.as_deref()` to the four
  `resolve_adapter_or_native` calls (`:579`, `:1375`, `:1467`, `:1527`).
- Replace the `FreshExec` `unreachable!` (`:607-609`) with the immediate-send,
  no-handshake body.
- Replace the `SpawnCommand` `unreachable!` (`:1538-1540`) with `send_line_to_pane`.
- **Verify**: `cargo test -p lisa-plugin`; specifically the transition tests and
  `test_build_claude_command*` (must still pass — Claude path unchanged).

## Step 4 — Emit `lisa_bin` in the layout (lisa-cli/loop_cmd.rs)

- Capture `std::env::current_exe().ok()` in `run_loop` (and `run_dry`).
- Widen `generate_layout` with `lisa_bin: Option<&Path>`; build the conditional
  `lisa_bin_line`; interpolate into the plugin block.
- Fix the 3 in-test call sites + the `run_dry` call site.
- Add `test_generate_layout_includes_lisa_bin` (present when supplied, absent when
  `None`).
- **Verify**: `cargo test -p lisa-cli`.

## Step 5 — Cross-target + full suite

- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — the new arm and
  `Box<dyn>` must compile for WASM.
- `cargo test --workspace` — all green.
- `cargo clippy` on touched crates — clean.

## Testing strategy

| Concern | Test kind | Where |
|---|---|---|
| Codex command construction (launch/reuse/follow-up) | unit, string-anchor | `adapter.rs` |
| Reuse-without-handshake (`reuse == launch`, `FreshExec`) | unit | `adapter.rs` |
| `signals()` all false | unit | `adapter.rs` |
| Resolver reaches Codex; production still Claude | unit | `adapter.rs` |
| `lisa_bin` config round-trip | unit | `types.rs` |
| Layout emits `lisa_bin` | unit | `loop_cmd.rs` |
| Claude path unchanged | existing suite | all crates |
| End-to-end pane run (launch → artifacts → `.stopped`/`.error`) | manual, needs live codex | — |

The one path CI cannot cover — a real codex pane run — is the same gap T-023-01
documented (no codex in CI). Every decision-bearing branch (command strings,
`FreshExec`, `SpawnCommand`, config threading) is unit-covered; only the live
subprocess stream is manual.

## Verification criteria (maps to AC)

- ✅ Codex adapter implements launch/reuse/follow-up/signals; resolvable via
  `build_adapter` (test-only path); production resolves Claude.
- ✅ `lisa loop` passes its absolute binary path (`current_exe`) into plugin
  config for wrapper invocation.
- ✅ `FreshExec`/`SpawnCommand` arms live; `.error` fails the thread promptly
  (existing T-022-02 consumer, unchanged).
- ✅ Claude behaviour untouched (existing tests green).
- ✅ Native tests cover command construction + reuse-without-handshake.

## Risks & mitigations

- **R1 signature ripple** (resolver +arg): mechanical; the compiler enumerates
  every call site. Land Steps 2+3 together so the crate always builds.
- **R2 `AgentSlot`/`State` literals**: none touched (Decision 4 avoids per-slot
  fields), so no test-literal churn.
- **R3 `current_exe` failure**: `Option` + `"lisa"` fallback; never panics.
- **R4 WASM compile**: `Box<dyn>` already used; new arm is `send_line_to_pane`
  (already WASM-side). Step 5 gates it.

## Rollback

Each step is one commit. Reverting Step 3 restores the `unreachable!` seams
(Claude still works); reverting Step 1/4 drops the config key (adapter falls back
to bare `lisa`). No destructive changes; no data migration.
</content>
