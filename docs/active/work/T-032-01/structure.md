# Structure: T-032-01 Zellij pane lifecycle names

## Change boundary

The implementation is confined to the plugin crate because pane naming is a Zellij host
operation driven by scheduler-owned state. Core ticket and routing types already expose every
required input. The CLI layout already creates the panes and needs no naming behavior.

Ticket-owned source paths:

- Create `crates/lisa-plugin/src/pane_name.rs`.
- Modify `crates/lisa-plugin/src/lib.rs`.

Workflow artifacts live under `docs/active/work/T-032-01/` and are left for Lisa's final
completion transaction, per the workflow contract.

No files are deleted.

## New `pane_name` module

`crates/lisa-plugin/src/pane_name.rs` is a small pure module with no Zellij host dependency.
It imports only `lisa_core::client::AgentClient`.

Public-within-crate surface:

```rust
pub(crate) const MAX_PANE_NAME_CHARS: usize = 80;

pub(crate) enum PaneName<'a> {
    Assigned {
        agent: AgentClient,
        ticket_id: &'a str,
        title: &'a str,
    },
    Idle {
        resident_agent: Option<AgentClient>,
    },
}

pub(crate) fn format_pane_name(input: PaneName<'_>) -> String;
```

The enum makes the formatter exhaustive across the two lifecycle modes and prevents callers
from assembling provider-specific strings independently.

Internal helpers:

- `sanitize_title(&str) -> String` replaces controls, collapses whitespace, trims, and
  returns `untitled` for an empty result.
- `truncate_assigned(prefix, title) -> String` performs Unicode-scalar counting and title-
  only ellipsis truncation.

The module documents that the 80-character normal bound assumes canonical Lisa ticket IDs;
complete provider and ticket scan keys take priority for malformed overlong IDs.

## Formatter tests

The module contains its own `#[cfg(test)]` test module. Tests are pure and table-oriented.

Cases:

- Claude assigned output.
- Codex assigned output.
- Resident Claude idle output.
- Resident Codex idle output.
- Empty shell idle output.
- Newline, carriage return, tab, escape, and Unicode control removal.
- Whitespace collapse and trimming.
- Empty/control-only title fallback.
- No truncation at the limit.
- Ellipsis truncation above the limit.
- Unicode input is cut at scalar boundaries.
- Full agent and ticket ID remain present after truncation.

Actual-vs-requested routing is most naturally tested in the scheduler module, where a parsed
ticket is resolved and the assigned `PaneName` is constructed from `route.agent`.

## Plugin module registration

`crates/lisa-plugin/src/lib.rs` adds:

```rust
mod pane_name;
```

It imports `format_pane_name` and `PaneName` for scheduler use. The module is not re-exported
outside the plugin crate because pane titles are a plugin presentation contract, not a core
domain type or CLI API.

## State additions

`State` gains:

```rust
last_pane_names: HashMap<u32, String>
```

The existing `#[derive(Default)]` initializes this cache empty. It is keyed by physical
terminal pane ID, matching Zellij's rename API and remaining stable through ticket and
provider transitions.

The cache deliberately does not live on `Thread`: idle slots have no thread, and naming is
about pane lifecycle rather than run provenance.

The cache deliberately does not live on `AgentSlot`: a state-level map avoids adding a field
to every existing slot fixture while still binding the applied value to a discovered pane.

## Rename gate

Add a private method on `State`:

```rust
fn rename_slot(&mut self, pane_id: u32, name: String) -> bool;
```

Behavior:

1. Verify `pane_id` occurs in `agent_slots`; unknown panes return `false`.
2. Compare `last_pane_names[pane_id]` with `name`.
3. If equal, return `false` and make no host call.
4. Insert the new cached value.
5. Call `rename_terminal_pane(pane_id, &name)`.
6. Return `true`.

The method is the only call site for Zellij's rename API. Tests observe the cache and return
value; the existing native host stub absorbs the plugin command during unit tests.

## Idle-name helper

Add a private pure-ish helper on `State` or a short local derivation:

```rust
fn idle_pane_name(slot: &AgentSlot) -> String
```

It passes `Some(last_client)` only when `has_session` is true. Any inconsistent or empty-shell
combination uses `resident_agent: None`, producing `lisa · idle`.

This makes session truth, not stale provider memory, authoritative.

## Slot discovery modification

`discover_slots` currently pushes each non-plugin pane immediately. To avoid a mutable borrow
collision, it will collect or record each new pane ID, push the slot, then call `rename_slot`
after the slot exists.

Each discovered slot starts with:

- `ticket_id = None`
- `has_session = false`
- `last_client = None`
- desired name `lisa · idle`

Repeated `PaneUpdate` events are already suppressed by `slots_discovered`; the rename cache
also makes the operation safe if discovery behavior changes later.

## Scheduling modification

Within `schedule_ready_tickets`:

1. Resolve `(adapter, route)` as today.
2. Retain access to the parsed ticket title from `self.dag` long enough to clone it into a
   local `String` before mutable calls.
3. Pass `route.agent`, ticket ID, and title to `format_pane_name`.
4. Call `rename_slot` after all admission/awaiting guards but before branching on recycle,
   same-provider reuse, or fresh launch.
5. Continue existing branch behavior unchanged.

The title clone avoids holding an immutable DAG borrow while mutating `State`. The assigned
rename therefore precedes `/exit`, `/clear`, fresh launch, or any prompt submission.

The route is moved into `Thread` only after its `agent` has been used for naming and slot
state, matching current ownership flow.

## Release modification

`release_slot_for_ticket` needs to avoid calling a `&mut self` helper while iterating through
`&mut self.agent_slots`. It will:

1. Find and mutate the matching slot.
2. Capture `(pane_id, idle_name)` in a local option.
3. End the slot borrow.
4. Call `rename_slot` with the captured name.
5. Log the existing release event.

Because the slot's `ticket_id` is cleared before formatting, the title reflects released
state. `has_session` and `last_client` remain unchanged under normal release, yielding the
resident provider's idle form.

## Clean-shell recovery modification

In the `WaitingForExit` timeout branch where `ticket_id` is absent, existing code resets:

- `transition_state = Idle`
- `transition_started_at = None`
- `has_session = false`
- `last_client = None`

After the mutable slot borrow ends, it calls the common formatter/gate for `lisa · idle`.
No other timeout branch changes pane naming.

## Completion behavior

No new code is required in `handle_completion_result` beyond release behavior:

- Failure returns without calling `release_slot_for_ticket`; cache remains assigned.
- Success verifies durable Done, then calls `release_slot_for_ticket`; cache becomes idle.
- Durable verification failure reinserts pending completion and returns; cache remains
  assigned.

Tests add explicit cache assertions to protect this ordering contract.

## Test helper additions

The plugin test module may add small helpers:

- `assigned_name(state, pane_id)` to read `last_pane_names`.
- A ticket/DAG fixture with a long or malicious title where lifecycle integration matters.

Existing `codex_state_with_dag`, `codex_slot`, and scheduling fixtures should be reused.
Assertions are added near existing fresh launch, provider reuse, recycling, and completion
tests rather than duplicating entire state-machine scenarios.

## Verification boundary

Focused verification:

- `cargo test -p lisa-plugin pane_name`
- Focused lifecycle test filters for new test names.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`

Repository verification:

- `cargo test --workspace`
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`

The final diff check must show only the two ticket-owned source paths outside work artifacts.
The isolated source commit includes exactly those source paths.

## Ordering of edits

1. Add the formatter module and pure tests.
2. Register the module and add the state cache/gate.
3. Initialize empty-shell names at discovery.
4. Apply assigned names before scheduler input.
5. Apply idle names after release and clean-shell recovery.
6. Add lifecycle and routing assertions.
7. Format, test, lint, build, and commit the exact source paths.
