# T-023-02 · Progress

Implementation log. What was done, in order, and the one deviation from the plan.

## Deviation from plan (important) — concurrent T-025-01 landed the selection seam

When Implement began, a **concurrent Lisa thread (T-025-01, client-selection-
config)** had already modified the shared files on this branch:

- Created `crates/lisa-core/src/client.rs` — `enum AgentClient { Claude, Codex }`
  + `parse`/`as_str`/`Display` + tests.
- Added `client: AgentClient` to `PluginConfig` + `from_config_map` parsing + the
  `client "…"` key in `generate_layout` + `ResolvedConfig.client` + doctor codex
  checks.
- Rewrote `adapter.rs`'s resolver to take a `default_client: AgentClient` and
  added `adapter_for_client(client)` — with the **Codex arm stubbed to
  `Box::new(ClaudeCodeAdapter)`** and an explicit `// T-023-02: return the native
  Codex adapter here` marker.

This is the "missing dependency edge" the RDSPI concurrency model warns about, but
it resolved cleanly as a **division of labour**: T-025-01 owns *selection*
(the `client` toggle), T-023-02 owns the *Codex adapter body* + `lisa_bin`
threading + filling the two scheduler seams. So the plan's `Provider` enum was
**dropped** in favour of the already-landed `AgentClient`, and `resolve_adapter`'s
already-changed signature was *extended* (add `lisa_bin`) rather than rewritten.

## Steps executed

1. **`lisa_bin` config field** (`crates/lisa-core/src/types.rs`)
   - Added `pub lisa_bin: Option<String>` (+ `#[serde(default)]`), `None` in
     `new()`, and a non-empty parse branch in `from_config_map`.
   - Test `test_config_lisa_bin_round_trip` (present → `Some`, absent/empty →
     `None`). ✅

2. **`CodexAdapter`** (`crates/lisa-plugin/src/adapter.rs`)
   - Added `struct CodexAdapter { lisa_bin: String }` + `new(Option<&str>)`
     (empty/None → bare `"lisa"`), and `agent_exec_line` helper.
   - `impl AgentAdapter`: `launch_command`/`reuse_prompt` → identical wrapper
     line (`LISA_PANE_ID=… LISA_TICKET_ID=… <lisa> agent-exec "<ticket_prompt>"`);
     `reset_strategy` → `FreshExec`; `follow_up` → `SpawnCommand(… agent-exec
     --resume "<finish_up_prompt>")`; `signals` → all `false`.
   - Filled `adapter_for_client`'s Codex arm → `Box::new(CodexAdapter::new(
     lisa_bin))`; threaded `lisa_bin: Option<&str>` through `adapter_for_client`,
     `resolve_adapter`, `resolve_adapter_or_native`.
   - Rewrote T-025-01's placeholder test `resolver_codex_falls_back_to_claude_…`
     → `resolver_codex_resolves_native_codex_adapter` (now asserts `FreshExec`);
     updated the two Claude resolver tests for the new arg; added 6 `CodexAdapter`
     tests (launch shape, reuse==launch, reset, follow-up, signals, bare-lisa
     fallback). ✅

3. **Scheduler wiring** (`crates/lisa-plugin/src/lib.rs`)
   - Added `self.config.lisa_bin.as_deref()` to all four
     `resolve_adapter_or_native` call sites.
   - Filled the `ResetStrategy::FreshExec` reuse arm (`schedule_ready_tickets`):
     send `launch_command` immediately, leave `transition_state` Idle (no
     `/clear`, no `WaitingForClear`).
   - Filled the `FollowUp::SpawnCommand` arm (`check_review_timeouts`): route
     through `send_line_to_pane` (the only pane I/O the WASM plugin has). ✅

4. **Layout emission** (`crates/lisa-cli/src/loop_cmd.rs`)
   - `run_loop`/`run_dry` capture `std::env::current_exe().ok()`.
   - `generate_layout` gained `lisa_bin: Option<&Path>` and emits a conditional
     `lisa_bin "<path>"` line. Updated all 5 test call sites; added
     present/absent layout tests. ✅

## Verification

- `cargo test --workspace` → **526 passed, 0 failed** (215 plugin + 117 cli + 194
  core + 0 doc). All new tests green; every pre-existing test unchanged and green.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → clean.
- `cargo clippy -p lisa-core -p lisa-plugin -p lisa-cli` → clean, no warnings.

## Not done (out of scope / follow-up)

- `SignalCapabilities` still has no live scheduler consumer (design Decision 4 —
  Codex never writes the Claude-only signals, so gating is unnecessary; the
  `WaitingForClear` avoidance is structural via `FreshExec`).
- End-to-end pane run against a live `codex` (no codex in CI; same gap as
  T-023-01). String construction + both seams are unit-covered.
- No commit made (per instructions — Lisa handles the rest).
</content>
