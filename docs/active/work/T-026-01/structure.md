# T-026-01 · Structure — file-level blueprint

The shape of the change, not the code. Ordering matters: core types/parser →
core resolver → plugin adapter → plugin spawn wiring → dashboard. Each layer
compiles and tests green before the next.

## New files

### `crates/lisa-core/src/route.rs` (new)

Public surface:

- `struct ResolvedRoute { agent: AgentClient, model: Option<String>,
  requested_agent: Option<String>, substituted: bool, note: Option<String> }`
  — `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` (Serialize
  so T-027-01 can embed it in the JSONL record).
- `fn resolve_route(ticket: &Ticket, default_agent: AgentClient) -> ResolvedRoute`
  — the precedence + fallback engine (Design Decision 3). Pure.
- Convenience: `ResolvedRoute::display_cell(&self) -> String` → `"claude"` or
  `"codex/gpt-5"` (model appended with `/`), with a trailing `*` when
  `substituted`. Used by the dashboard so formatting lives with the type.
- Register in `lib.rs`: `pub mod route;`

Unit tests (in-file `#[cfg(test)]`): none/some/valid/invalid agent, model
pass-through, precedence to default, substituted flag + note content,
`display_cell` variants.

## Modified files

### `crates/lisa-core/src/types.rs`

`Ticket` gains two fields (Design Decision 2), placed after `blocks`:

```
#[serde(default, skip_serializing_if = "Option::is_none")]
pub agent: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub model: Option<String>,
```

- Update `Ticket::new` to initialise both to `None`.
- No behavioural change to any existing method; `PartialEq` derive picks up the
  new fields automatically (existing `Ticket::new`-based tests still pass since
  both default to `None`).

### `crates/lisa-core/src/ticket.rs`

In `parse_frontmatter_into_ticket`:

- Add `let mut agent: Option<String> = None; let mut model: Option<String> = None;`
- Two new `match key` arms: `"agent" => agent = Some(value.to_string())`,
  `"model" => model = Some(value.to_string())`. Values are already trimmed by
  `parse_yaml_line`. **Lenient** — no `parse_*` helper, no error (Design
  Decision 2). Empty value → store `None` (guard `if !value.is_empty()`), so a
  stray `agent:` with no value is treated as absent.
- Populate the two fields in the returned `Ticket { … }`.
- New tests: inline `agent: codex` / `model: gpt-5`; both absent → `None`;
  present alongside multiline `depends_on` (the "inline + multiline" criterion);
  invalid `agent: gpt` still *parses* (stored raw, no error).

### `crates/lisa-plugin/src/adapter.rs`

- `use lisa_core::route::{resolve_route, ResolvedRoute};`
- `adapter_for_client(client, lisa_bin)` → add `model: Option<&str>` param;
  thread it into both arms.
- `ClaudeCodeAdapter` → `ClaudeCodeAdapter { model: Option<String> }`
  (constructor or struct literal); `launch_command`/`reuse_prompt` pass the model
  into the `lib.rs` builders.
- `CodexAdapter` → add `model: Option<String>`; `agent_exec_line` emits the model
  flag onto the `lisa agent-exec` line.
- **`resolve_adapter(ticket, default_client, lisa_bin)`** — replace the
  `let _ = ticket;` stub: `let route = resolve_route(ticket, default_client);`
  then build the adapter from `route.agent` + `route.model`. **Return the route
  too** so the caller can log/surface/store it. Change signature to return
  `(Box<dyn AgentAdapter>, ResolvedRoute)` — or add a sibling
  `resolve_route_and_adapter`. Decision: change the two existing resolvers to
  return the tuple; update the four call sites. `resolve_adapter_or_native` with
  `None` ticket returns a `ResolvedRoute` built from the default (not
  substituted).
- Update existing adapter tests for the new signature/tuple; add tests: ticket
  with `agent: codex` resolves the Codex adapter under a Claude default; invalid
  `agent` falls back to default with `substituted = true`; model flows into the
  launch command for both providers.

### `crates/lisa-plugin/src/lib.rs`

- `build_claude_command(ticket_dir, ticket_id, pane_id)` → add
  `model: Option<&str>`; append ` --model <m>` before the quoted prompt only when
  `Some`. Absent = current output verbatim.
- The Claude adapter's `launch_command` passes its stored model through.
- **Spawn site (~586)** and the three reuse sites (1388/1480/1540): consume the
  new tuple. At spawn, when `route.substituted`, push
  `ActivityEvent::Warning { message: route.note }` (or an `Info`) via the
  existing activity path. Store `route` on the `Thread` (see below) for
  provenance + dashboard.
- **`Thread`** (`lisa-core/types.rs:329`) gains `#[serde(default)] route:
  Option<ResolvedRoute>` — set at spawn from the resolver result. `Thread::new`
  leaves it `None` (back-compat with persisted threads). This is the hand-off
  surface for T-027-01 and the dashboard.
- Populate `ui::ActiveThread` (`lib.rs:2865`): map `thread.route` →
  `route_cell: Option<String>` via `ResolvedRoute::display_cell`.

### `crates/lisa-plugin/src/ui.rs`

- `ActiveThread` gains `route: Option<String>` (the pre-formatted cell).
- `render_threads`: add an `AGENT` column to the header/rows. Widen the
  separator; render `route` (or `—` when `None`). Keep it compact to respect the
  fixed-width table. Parked rows may show the same cell if available, else `—`.
- Update `render_threads` tests + the `ActiveThread` literal at ~1310/1420.

### `crates/lisa-core/src/types.rs` (Thread + serde)

As above: `Thread.route: Option<ResolvedRoute>`. Because `ResolvedRoute`
derives `Serialize/Deserialize`, thread persistence round-trips. Add a test that
a `Thread` with a route serialises and a legacy thread JSON without the field
deserialises (default `None`).

## Ordering of changes (each independently committable)

1. **core types + parser** (`types.rs` fields, `ticket.rs` arms) + parser tests.
2. **core `route.rs`** (`ResolvedRoute` + `resolve_route` + tests) + `mod route`.
3. **`Thread.route`** field + serde back-compat test.
4. **plugin adapter** (`resolve_route` wired into `resolve_adapter*`, model into
   adapters, `build_claude_command` model arg) + adapter tests.
5. **spawn/reuse wiring** in `lib.rs` (tuple consumption, substitution
   Warning event, store route on Thread) + a mixed-route scheduling test.
6. **dashboard** (`ActiveThread.route`, `render_threads` column) + UI tests.

## Interfaces / boundaries preserved

- Resolver stays **vocabulary-only**; adapters own model→flag mapping.
- lisa never writes routing frontmatter (read-only).
- No-opt-in path (`agent`/`model` absent, Claude default) produces byte-for-byte
  the current launch command → the zero-regression invariant, asserted by test.
- `ResolvedRoute` is the single shared shape for surfacing + provenance.

## Out of scope (deferred, noted so reviewers don't expect them)

- Provider-aware concurrency / rate-limit pools — **T-026-02**.
- Writing the provenance JSONL record — **T-027-01** (this ticket only exposes
  the data).
- Availability probing of provider binaries / model validity at spawn (Design
  Decision 4).
- A loop-level default *model* / policy routing by type/phase.
