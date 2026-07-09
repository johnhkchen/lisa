# T-023-02 · Structure — file-level blueprint

The shape of the change: files, boundaries, interfaces, ordering. Not code.
Grounded in `design.md`.

## Files

### Modified

- **`crates/lisa-core/src/types.rs`** (`PluginConfig`)
  - Add field `pub lisa_bin: Option<String>` to `struct PluginConfig` (`:470`).
  - Default `None` in `PluginConfig::new()` (`:543`).
  - Parse key `lisa_bin` in `from_config_map` (`:559`): `if let Some(v) =
    config.get("lisa_bin") { if !v.is_empty() { result.lisa_bin = Some(v.clone()); } }`.
  - One unit test asserting the key round-trips (present → `Some`, absent →
    `None`, empty → `None`).

- **`crates/lisa-cli/src/loop_cmd.rs`** (layout generation)
  - `run_loop`: capture `let lisa_bin = std::env::current_exe().ok();` and pass to
    `generate_layout`.
  - `run_dry`: same capture (so the printed layout is representative); may pass
    `None` — dry-run does not launch, so an absent path is acceptable, but
    capturing keeps the preview honest.
  - `generate_layout(wasm_path, config)` → `generate_layout(wasm_path, lisa_bin:
    Option<&Path>, config)`. Build a `lisa_bin_line` string —
    `"                lisa_bin \"{}\"\n"` when `Some`, empty when `None` — and
    interpolate it into the plugin block alongside the existing keys.
  - Update the 3 existing `generate_layout` call sites in `#[cfg(test)]` +
    `run_dry` for the new arg; add one test asserting `lisa_bin "<path>"` appears
    when a path is supplied and is absent when `None`.

- **`crates/lisa-plugin/src/adapter.rs`** (the adapter itself)
  - Add `struct CodexAdapter { lisa_bin: String }` + `impl CodexAdapter { fn
    new(lisa_bin: Option<&str>) -> Self }` (falls back to `"lisa"`).
  - `impl AgentAdapter for CodexAdapter` — the five methods per design Decision 1.
  - Add `enum Provider { Claude, Codex }` and `fn provider_for(_ticket: &Ticket)
    -> Provider` (returns `Claude`; the seam).
  - Add `pub(crate) fn build_adapter(provider: Provider, lisa_bin: Option<&str>)
    -> Box<dyn AgentAdapter>` (the test-only Codex resolution path).
  - Change `resolve_adapter(ticket)` → `resolve_adapter(ticket, lisa_bin:
    Option<&str>)`, delegating to `build_adapter(provider_for(ticket), lisa_bin)`.
  - Change `resolve_adapter_or_native(ticket)` →
    `resolve_adapter_or_native(ticket, lisa_bin: Option<&str>)`.
  - New `#[cfg(test)]` cases (see Test surface).

- **`crates/lisa-plugin/src/lib.rs`** (scheduler wiring)
  - Four `resolve_adapter_or_native(...)` call sites (`:579`, `:1375`, `:1467`,
    `:1527`) gain the second arg `self.config.lisa_bin.as_deref()`.
  - **Fill the `FreshExec` arm** (`:607-609`): replace `unreachable!` with
    `let cmd = adapter.launch_command(&ctx); self.send_line_to_pane(&cmd,
    PaneId::Terminal(pane_id)); launch_cmd = cmd;` — no `WaitingForClear`.
  - **Fill the `SpawnCommand` arm** (`:1538-1540`): replace `unreachable!` with
    `self.send_line_to_pane(&cmd, PaneId::Terminal(pane_id));`.
  - No new `State`/`AgentSlot` fields. No new imports beyond what the arms need
    (already have `PaneId`, `send_line_to_pane`).

### Created

- None. (Both seams pre-exist; only their bodies and one config field are new.)

### Deleted

- None.

## Interface delta (crate-internal)

```
// adapter.rs
pub(crate) struct CodexAdapter { lisa_bin: String }
enum Provider { Claude, Codex }
fn provider_for(ticket: &Ticket) -> Provider            // seam, returns Claude
pub(crate) fn build_adapter(Provider, Option<&str>) -> Box<dyn AgentAdapter>
pub(crate) fn resolve_adapter(&Ticket, Option<&str>) -> Box<dyn AgentAdapter>       // +arg
pub(crate) fn resolve_adapter_or_native(Option<&Ticket>, Option<&str>) -> Box<...>  // +arg

// lisa-core types.rs
PluginConfig { …, pub lisa_bin: Option<String> }        // +field
```

The `AgentAdapter` trait, `SpawnContext`, `FollowUpContext`, `ResetStrategy`,
`FollowUp`, `SignalCapabilities` are **unchanged** — the whole point of T-022-01.

## Command strings the adapter emits (the contract under test)

- launch / reuse (`SpawnContext`):
  `LISA_PANE_ID={pane_id} LISA_TICKET_ID={ticket_id} {lisa_bin} agent-exec "{ticket_prompt(ticket_dir, ticket_id)}"`
- follow-up (`FollowUpContext`):
  `LISA_PANE_ID={pane_id} LISA_TICKET_ID={ticket_id} {lisa_bin} agent-exec --resume "{finish_up_prompt(ticket_dir, work_dir, ticket_id)}"`

`{lisa_bin}` = the struct field (`current_exe` path, else `"lisa"`).

## Module boundary (unchanged from T-022-01)

`adapter.rs` remains a leaf: depends on `lib.rs`'s `pub(crate)` free functions and
`lisa_core::types::Ticket`; performs **no host I/O** (returns strings/enums). Only
`lib.rs` calls `send_line_to_pane`. The `SpawnCommand` arm typing into a pane
keeps the WASM boundary explicit — the adapter never spawns.

## Ordering of changes (each independently compilable + testable)

1. `types.rs`: add `lisa_bin` field + parse + test. `cargo test -p lisa-core`.
2. `adapter.rs`: add `CodexAdapter`, `Provider`, `build_adapter`; widen the two
   resolvers' signatures; add adapter tests. Compiles standalone (resolvers'
   new arg not yet supplied by lib.rs → do 3 in the same step to keep the crate
   building).
3. `lib.rs`: thread `lisa_bin.as_deref()` into the four call sites; fill both
   `unreachable!` arms. `cargo test -p lisa-plugin` + native suite.
4. `loop_cmd.rs`: capture `current_exe`, widen `generate_layout`, emit the key,
   fix call sites + test. `cargo test -p lisa-cli`.
5. WASM build check: `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
   (`Box<dyn>` + new arm must compile under `wasm32-wasip1`).
6. Full `cargo test --workspace` + `just check`.

Steps 2 and 3 are coupled (the resolver signature change ripples into lib.rs), so
they land together; everything else is independently bisectable.

## Test surface (new)

`adapter.rs`:
- `codex_launch_command_shape` — contains `agent-exec`, `LISA_PANE_ID=7`,
  `LISA_TICKET_ID=T-…`, the `lisa_bin` path, and the full `ticket_prompt`.
- `codex_reuse_equals_launch` — the **reuse-without-handshake** proof:
  `reuse_prompt(ctx) == launch_command(ctx)` (a fresh wrapper line, not a bare
  prompt), mirroring `test_build_claude_command*`'s string anchoring.
- `codex_reset_is_fresh_exec` — `reset_strategy() == FreshExec`.
- `codex_follow_up_is_spawn_command` — `SpawnCommand` with `--resume` and the
  `finish_up_prompt`.
- `codex_signals_all_false`.
- `codex_new_falls_back_to_lisa` — `CodexAdapter::new(None)` uses bare `lisa`.
- `build_adapter_codex_resolves` — `build_adapter(Provider::Codex, Some(bin))`
  yields a `FreshExec` adapter; `provider_for(any)` still `Claude` (production
  unchanged), and `resolve_adapter(t, None).reset_strategy() == ClearHandshake`.

`types.rs`: `lisa_bin` round-trips (present/absent/empty).
`loop_cmd.rs`: `lisa_bin "<path>"` present when supplied, absent when `None`.

## Invariants preserved

- `AgentSlot`/`State` gain **no fields** → all test literals unchanged.
- `ClaudeCodeAdapter`, free functions, FSM, signal readers byte-identical.
- Every existing `resolve_adapter*` caller in lib.rs still resolves Claude
  (only the arg list grew; `provider_for` returns Claude) → existing tests green.
- No new dependency; WASM-safe.
</content>
