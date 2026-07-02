# T-026-01 · Review — routing frontmatter

Self-assessment of the completed work. Handoff for a human reviewer.

## What changed

### lisa-core

- **`types.rs`** — `Ticket` gains `agent: Option<String>` and
  `model: Option<String>` (raw, unvalidated routing hints); `Ticket::new` inits
  both `None`. `Thread` gains `route: Option<ResolvedRoute>` (`#[serde(default)]`),
  set at spawn; `Thread::new` → `None`. New serde back-compat test for `Thread`.
- **`ticket.rs`** — parser gains two lenient `match` arms for `agent`/`model`
  (trimmed scalar, empty = absent, **never errors** — the "invalid route can't
  fail the ticket" contract). 5 new parser tests (inline, absent, invalid-raw,
  empty-as-absent, coexistence with a multiline `depends_on`).
- **`route.rs`** (new) — `ResolvedRoute { agent, model, requested_agent,
  substituted, note }` + `resolve_route(&Ticket, AgentClient)` + `display_cell`.
  Encodes precedence (ticket → loop default → native Claude) and fallback
  (invalid agent → default, `substituted=true`, actionable `note`). 7 unit
  tests. Registered `pub mod route`.
- **`dag.rs` / `diagnostics.rs`** — added `agent: None, model: None` to two
  test-helper struct literals (compile fix from the new required fields).

### lisa-plugin

- **`lib.rs`** — `build_claude_command` gains `model: Option<&str>` (`--model`
  appended only when `Some`; absent = byte-for-byte the old line). Spawn site
  destructures `(adapter, route)`, logs `ActivityEvent::Warning` on
  substitution, stores `route` on the thread and sets `thread.client =
  route.agent`. Three reuse sites take `(adapter, _route)`. `ActiveThread`
  populated with `route.display_cell()`.
- **`adapter.rs`** — `ClaudeCodeAdapter { model }` (+ `new`/`Default`),
  `CodexAdapter { lisa_bin, model }` (+ `model_flag`). `adapter_for_route`
  replaces `adapter_for_client`; both resolvers return
  `(Box<dyn AgentAdapter>, ResolvedRoute)`. New tests: model→flag (both
  providers), agent override, invalid→fallback, mixed-route heterogeneous
  resolution in one loop.
- **`ui.rs`** — `ActiveThread.route: Option<String>`; `render_threads` gains an
  `AGENT` column (route cell / `—`), separator widened to 70. New route-render
  test; existing `ActiveThread` literals updated with `route`.

## How the acceptance criteria are met

1. **Fields parsed in lisa-core, tolerated by old versions** — `agent`/`model`
   parsed in `ticket.rs`; unknown-field tolerance already covered old binaries,
   and these are lenient (no new error paths).
2. **Spawn resolver: ticket → loop default → native Claude, per ticket, mixed** —
   `resolve_route` + `resolve_adapter`; `mixed_route_resolves_heterogeneous_
   adapters_in_one_loop` proves two tickets resolve to different adapters under
   one default.
3. **Invalid → loop default, logged + dashboard + provenance** — `substituted`
   flag drives the spawn `Warning` log and the `*` marker in the AGENT cell;
   `ResolvedRoute` (requested + actual) is stored on the thread for T-027-01.
4. **Dashboard surfaces `(provider, model)`** — the `AGENT` column renders
   `claude` / `codex/gpt-5` / `codex/gpt-5*`.
5. **Tests** — parsing (inline + multiline coexistence), precedence, fallback,
   and a mixed-route resolution test all present and green.

## Test coverage & verification

- `cargo test --workspace`: **lisa-cli 218 ✓, lisa-core 140 ✓, lisa-plugin 215 ✓**
  (+1 failure owned by T-027-01, below).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` ✓ — no host-only
  dependency leaked into the WASM plugin (resolution stayed in `lisa-core`).
- `cargo fmt` applied.
- **Zero-regression proof**: `build_claude_command(.., None)` asserts no
  `--model`, and `native_launch_matches_free_fn` ties the default adapter to the
  unchanged line.

Gaps / not covered:

- No **plugin-level end-to-end spawn** test for mixed routing (two panes
  actually launched in one `poll_tick`). The adapter-level
  `mixed_route_…` test covers the resolution contract; a full scheduling test
  would need the Zellij host harness the other codex tests use. The resolver is
  where all the routing logic lives, so coverage is meaningful, but a reviewer
  wanting belt-and-suspenders could add a `codex_state_with_dag`-style spawn
  test.

## Open concerns / for human attention

1. **Cross-ticket test failure (NOT this ticket's bug).** T-027-01's uncommitted
   provenance WIP shares the working tree. Its test
   `provenance_emitted_on_error_signal` (`lib.rs`) builds a thread with
   `Thread::new(...)` (client defaults to Claude) but asserts the record's
   method is `codex`, and it never spawns — so no version of the routing spawn
   code touches it. The two sibling provenance tests set `thread.client`
   explicitly; this one omits it. It fails independently of T-026-01. **This
   ticket deliberately does not edit T-027-01's test** (avoids conflicting with
   the concurrent agent); flagged here for whoever integrates the two.
2. **`thread.client = route.agent` at spawn is load-bearing for provenance.**
   With per-pane routing the *actual* client can differ from `config.client`;
   recording `route.agent` (not the loop default) is what makes T-027-01's
   "actual" field correct for a routed pane. If T-027-01 later reads the route
   directly, `client` and `route.agent` stay in sync (both set here).
3. **"Unavailable" routes are only partially handled** (design decision, stated
   honestly): an *invalid* provider (not claude|codex) falls back with a
   surfaced substitution. An *unavailable* one (codex binary missing, or a model
   the provider rejects) cannot be probed from WASM at spawn — a missing binary
   stays a `lisa doctor` concern, and a bad model surfaces at runtime in the
   signal files / provenance. No fabricated availability check was added.
4. **No loop-level default model.** Only the agent has a loop default
   (T-025-01). A `None` model runs the provider default — intentional, keeps
   today's behaviour; a future ticket could add `[agent].model` if wanted.
5. **Formatting of a few auto-edited test literals** was normalised by
   `cargo fmt`; no functional impact.

## Follow-ups (out of scope here)

- **T-026-02** — provider-aware concurrency / rate-limit pools + 16-agent mixed
  stress.
- **T-027-01** — write the provenance record from the `ResolvedRoute` this
  ticket now stores on the thread (requested vs. actual).
