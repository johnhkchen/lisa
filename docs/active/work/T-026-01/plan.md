# T-026-01 · Plan — implementation steps

Ordered, each step compiles + tests green and is atomically committable. Verify
between steps with `cargo test -p lisa-core` / `cargo test -p lisa-plugin` and a
final `cargo test --workspace` + WASM check.

## Step 1 — Ticket routing fields + lenient parsing (lisa-core)

1. `types.rs`: add `agent: Option<String>`, `model: Option<String>` to `Ticket`
   (after `blocks`, `#[serde(default, skip_serializing_if="Option::is_none")]`);
   init both `None` in `Ticket::new`.
2. `ticket.rs`: in `parse_frontmatter_into_ticket` add `agent`/`model`
   accumulators, two `match` arms storing the trimmed value when non-empty, and
   populate the returned struct.
3. Tests (`ticket.rs`): `agent: codex`+`model: gpt-5` parse; both absent → `None`;
   invalid `agent: gpt` still parses (raw, no error); fields coexist with a
   multiline `depends_on` block (inline+multiline criterion).

**Verify:** `cargo test -p lisa-core`. Commit: "T-026-01: parse agent/model routing frontmatter".

## Step 2 — `ResolvedRoute` + `resolve_route` (lisa-core)

1. New `route.rs`: `ResolvedRoute` struct (derives incl. Serialize/Deserialize),
   `resolve_route(&Ticket, AgentClient) -> ResolvedRoute`, `display_cell`.
2. Precedence + fallback per Design Decision 3; `note` on substitution reuses the
   `AgentClient::parse` error text.
3. `lib.rs`: `pub mod route;`.
4. Tests: none → default (not substituted); valid ticket agent wins over default;
   invalid → default + `substituted=true` + `note` contains the bad value and the
   default; model passes through untouched in every branch; `display_cell` for
   `claude`, `codex/gpt-5`, and substituted (`*`).

**Verify:** `cargo test -p lisa-core`. Commit: "T-026-01: resolve_route precedence + fallback".

## Step 3 — `Thread.route` hand-off field (lisa-core)

1. `types.rs`: `Thread` gains `#[serde(default)] pub route: Option<ResolvedRoute>`;
   `Thread::new` sets `None`.
2. Test: a legacy `Thread` JSON without `route` deserialises to `None`; a thread
   with a route round-trips.

**Verify:** `cargo test -p lisa-core`. Commit: "T-026-01: carry ResolvedRoute on Thread".

## Step 4 — Model into command builders + adapters (lisa-plugin)

1. `lib.rs`: `build_claude_command` gains `model: Option<&str>`; append
   ` --model <m>` before the prompt only when `Some`. Update its callers/tests.
2. `adapter.rs`: `ClaudeCodeAdapter { model }` and `CodexAdapter { lisa_bin, model }`;
   `launch_command`/`reuse_prompt`/`agent_exec_line` thread the model. Codex line
   emits the model flag (via the `agent-exec` passthrough already supported).
3. `adapter_for_client(client, model, lisa_bin)` signature updated.
4. Tests: Claude command with `Some("opus")` contains `--model opus`; with `None`
   equals today's line (zero-regression assert); Codex line with a model contains
   the flag.

**Verify:** `cargo test -p lisa-plugin`. Commit: "T-026-01: thread model through adapters".

## Step 5 — Wire `resolve_route` into the resolvers + spawn (lisa-plugin)

1. `adapter.rs`: `resolve_adapter`/`resolve_adapter_or_native` return
   `(Box<dyn AgentAdapter>, ResolvedRoute)`; build the adapter from
   `route.agent`+`route.model`; `None` ticket → default route (not substituted).
2. `lib.rs`: update the 4 call sites to destructure the tuple. At spawn, on
   `route.substituted` push `ActivityEvent::Warning { message: note }`. Store
   `route` on the new `Thread`.
3. Tests: mixed-route scheduling — two tickets, one `agent: codex` and one absent,
   under a Claude default, resolve to different adapters concurrently
   (`reset_strategy` FreshExec vs ClearHandshake) with independent routes; invalid
   agent under a Codex default falls back to Codex + substituted.

**Verify:** `cargo test -p lisa-plugin`. Commit: "T-026-01: per-ticket route resolution at spawn + substitution log".

## Step 6 — Dashboard surfacing (lisa-plugin)

1. `ui.rs`: `ActiveThread.route: Option<String>`; add `AGENT` column to
   `render_threads` header/rows (compact; `—` when `None`); widen separator.
2. `lib.rs:2865`: populate `route` from `thread.route.map(|r| r.display_cell())`.
3. Tests: `render_threads` output includes the route cell for a routed thread and
   `—`/absent for an unrouted one; header contains `AGENT`.

**Verify:** `cargo test -p lisa-plugin`. Commit: "T-026-01: surface (provider, model) in the thread table".

## Step 7 — Full verification

1. `cargo test --workspace`.
2. `cargo build -p lisa-plugin --target wasm32-wasip1 --release` (WASM still
   compiles — no host-only deps leaked into the plugin).
3. `just check` if available.
4. Write `progress.md` (running) and `review.md`.

## Testing strategy summary

- **Unit (lisa-core):** parser (inline + coexistence with multiline), resolver
  precedence + fallback + note, `display_cell`, Thread serde back-compat. All
  host-free — this is why resolution lives in core.
- **Unit (lisa-plugin):** command builders (model on/off, zero-regression), adapter
  selection per route, dashboard rendering.
- **Integration-ish (lisa-plugin):** the mixed-route scheduling test — the
  acceptance criterion that two panes run different `(provider, model)` in one
  loop and the scheduler stays client-agnostic.
- **Regression guard:** the "`None` model + Claude default == today's exact
  command" assertion is the zero-regression proof the epic demands.

## Risks / watch-items

- Changing the resolver signature touches 4 call sites — mechanical but must all
  destructure the tuple; the reuse sites don't need the route surfaced, only the
  adapter, so ignore `route` there (`let (adapter, _) = …`).
- `Thread` gaining a field: ensure every `Thread` construction/`PartialEq` test
  still holds (only `Thread::new` constructs them; default `None` keeps equality).
- Dashboard width: keep the new column narrow to avoid wrapping the fixed table.
