# T-022-01 · Structure — file-level blueprint

The shape of the change. Not code — the files, module boundaries, public
interfaces, and ordering. Blueprint for `plan.md`.

## Files

### Created

- **`crates/lisa-plugin/src/adapter.rs`** (new, ~120 lines)
  - The `AgentAdapter` trait + doc comment (AC 4: documents how native Codex and
    ACP fit without redesign).
  - Supporting types: `SpawnContext<'a>`, `FollowUpContext<'a>`,
    `ResetStrategy`, `FollowUp`, `SignalCapabilities`.
  - `struct ClaudeCodeAdapter` + `impl AgentAdapter for ClaudeCodeAdapter`,
    delegating to the existing free functions in `lib.rs`.
  - `fn resolve_adapter(ticket: &Ticket) -> Box<dyn AgentAdapter>` → MVP returns
    `Box::new(ClaudeCodeAdapter)` unconditionally, ignoring the ticket.
  - Unit tests (`#[cfg(test)] mod tests`) proving the native adapter's outputs
    equal the free functions and the resolver returns a working adapter.

### Modified

- **`crates/lisa-plugin/src/lib.rs`**
  - Add `mod adapter;` and `use adapter::{AgentAdapter, ClaudeCodeAdapter,
    ResetStrategy, FollowUp, SignalCapabilities, SpawnContext, FollowUpContext,
    resolve_adapter};` near the top (line ~7).
  - Change visibility of the three shared free functions from `fn` to
    `pub(crate) fn` so `adapter.rs` can call them **without moving them** (keeps
    every existing test reference — they resolve via `super::*` — unchanged):
    - `ticket_prompt` (`:34`)
    - `build_claude_command` (`:53`)
    - `finish_up_prompt` (`:63`)
  - Import `Ticket` for the resolver signature: extend the
    `use lisa_core::types::{…}` at `:17` (or `use lisa_core::ticket::Ticket` /
    wherever `Ticket` is defined — confirm during Plan; `Dag::get_ticket` returns
    `&Ticket`).
  - Rewire four call sites to go through a resolved adapter (see below).

### Deleted

- None.

## Module boundary

`adapter.rs` is a leaf module: it depends **on** `lib.rs`'s free functions and
`lisa_core::types::Ticket`, and is depended on **by** `lib.rs`'s scheduler code.
No cycle (Rust modules in one crate share a namespace; the `pub(crate)` free
functions are visible to the submodule). The adapter performs **no host I/O** —
it returns strings/enums; only `lib.rs` calls `send_line_to_pane` /
`run_command`. This keeps the WASM-constraint boundary explicit and the module
unit-testable without a Zellij host.

## Public (crate-internal) interface of `adapter.rs`

```
pub(crate) trait AgentAdapter {
    fn launch_command(&self, ctx: &SpawnContext) -> String;
    fn reset_strategy(&self) -> ResetStrategy;
    fn reuse_prompt(&self, ctx: &SpawnContext) -> String;
    fn follow_up(&self, ctx: &FollowUpContext) -> FollowUp;
    fn signals(&self) -> SignalCapabilities;
}
pub(crate) struct SpawnContext<'a> { pub ticket_dir: &'a Path, pub ticket_id: &'a str, pub pane_id: u32 }
pub(crate) struct FollowUpContext<'a> { pub ticket_dir: &'a Path, pub work_dir: &'a Path, pub ticket_id: &'a str, pub pane_id: u32 }
pub(crate) enum ResetStrategy { ClearHandshake, FreshExec }
pub(crate) enum FollowUp { TypeIntoPane(String), SpawnCommand(String) }
pub(crate) struct SignalCapabilities { pub idle: bool, pub awaiting: bool, pub cleared: bool }
pub(crate) struct ClaudeCodeAdapter;
pub(crate) fn resolve_adapter(ticket: &Ticket) -> Box<dyn AgentAdapter>;
```

`ResetStrategy` / `FollowUp` derive `Debug, PartialEq, Eq` for test assertions.

## Call-site rewiring in `lib.rs` (all no-op for native)

1. **`schedule_ready_tickets` fresh branch (`:580-586`)**
   - Resolve `let adapter = resolve_adapter(ticket);` (ticket from
     `self.dag.get_ticket(&ticket_id)` — already fetched at `:594`; hoist the
     lookup slightly earlier or resolve from a cloned handle).
   - `!has_session` → `adapter.launch_command(&SpawnContext { ticket_dir:
     &host_ticket_dir, ticket_id: &ticket_id, pane_id })`. Native == today's
     `build_claude_command(...)`.

2. **`schedule_ready_tickets` reuse branch (`:568-579`)**
   - `match adapter.reset_strategy()`:
     - `ClearHandshake` → today's exact code: `send_line_to_pane("/clear", …)`,
       `transition_state = WaitingForClear`, stamp `transition_started_at`,
       `launch_cmd = adapter.reuse_prompt(&ctx)` (== `ticket_prompt`).
     - `FreshExec` → `unreachable!("no FreshExec adapter in MVP")` (documented
       seam; no adapter returns it yet, so never hit).

3. **`handle_cleared_signal` (`:1266-1268`)**
   - Replace `ticket_prompt(&host_ticket_dir, &ticket_id)` with
     `resolve_adapter(ticket).reuse_prompt(&ctx)`. Needs the ticket; fetch via
     `self.dag.get_ticket(&ticket_id)`. Native == today.
   - **Note:** if the ticket is momentarily missing from the DAG, fall back to
     the native adapter (see Plan risk R2) so behaviour never regresses.

4. **`check_transition_timeouts` clear-timeout fallback (`:1350-1353`)**
   - Same substitution as (3): `reuse_prompt` in place of `ticket_prompt`.

5. **`check_review_timeouts` (`:1400-1403`)**
   - `match resolve_adapter(ticket).follow_up(&FollowUpContext { … })`:
     - `TypeIntoPane(s)` → `send_line_to_pane(&s, PaneId::Terminal(pane_id))`
       (== today, since native yields `finish_up_prompt`).
     - `SpawnCommand(_)` → documented Codex seam; in MVP native never returns it.

`SignalCapabilities` is **declared** by the native adapter (all `true`) but has
no *behavioural* consumer added in this ticket — its consumers are T-022-02
(`.error`) and the Codex adapter. Wiring a live consumer now would risk a
no-op violation for no MVP benefit; the type + native declaration + trait method
are the deliverable (the "expected-signal-set" AC is met by the declaration and
its doc-documented purpose). This is called out explicitly so a reviewer knows
the omission is intentional.

## Ordering of changes

1. Make the three free functions `pub(crate)` (mechanical, no behaviour change;
   run tests — still green).
2. Add `adapter.rs` with trait, types, `ClaudeCodeAdapter`, `resolve_adapter`,
   and its own unit tests. Compiles standalone; not yet wired. Run tests.
3. Wire fresh-launch (call site 1). Run `test_build_claude_command*` + full
   suite.
4. Wire reuse + cleared + clear-timeout (call sites 2, 3, 4). Run transition
   tests.
5. Wire follow-up (call site 5). Run `test_check_review_timeouts_*`.
6. WASM build check (`cargo build -p lisa-plugin --target wasm32-wasip1
   --release`) — `Box<dyn>` and the trait must compile under `wasm32-wasip1`.

Each step is independently compilable and testable, so a regression is bisectable
to one call site.

## Invariants preserved

- `State` and `AgentSlot` gain **no fields** → all `State::default()` /
  `AgentSlot { … }` test literals unchanged.
- The three free functions keep identical signatures and bodies → string-anchor
  tests unchanged.
- `TransitionState` FSM and all signal-file scanning unchanged.
- No new dependencies; `Box<dyn Trait>` is core `alloc`, WASM-safe.
