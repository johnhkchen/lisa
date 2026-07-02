# T-026-01 · Progress

All seven plan steps complete.

- [x] Step 1 — Ticket `agent`/`model` fields + lenient parsing (lisa-core)
      `types.rs` (fields + `Ticket::new`), `ticket.rs` (two lenient `match`
      arms), 5 new parser tests. Also fixed two struct-literal `Ticket`
      constructions in `dag.rs`/`diagnostics.rs` test helpers.
- [x] Step 2 — `ResolvedRoute` + `resolve_route` (lisa-core)
      New `route.rs`; precedence + fallback + `display_cell`; `pub mod route`;
      7 unit tests.
- [x] Step 3 — `Thread.route` hand-off field (lisa-core)
      `Thread.route: Option<ResolvedRoute>` (`#[serde(default)]`), `Thread::new`
      sets `None`; serde back-compat test. Coexists with T-027-01's `client` /
      `concurrency_at_spawn` fields already on `Thread`.
- [x] Step 4 — model into command builders + adapters (lisa-plugin)
      `build_claude_command` gains `model: Option<&str>`; `ClaudeCodeAdapter`
      and `CodexAdapter` carry a model; `adapter_for_route` threads it; new
      command-shape tests (with-model + zero-regression without).
- [x] Step 5 — wire `resolve_route` into resolvers + spawn (lisa-plugin)
      `resolve_adapter`/`resolve_adapter_or_native` now return
      `(Box<dyn AgentAdapter>, ResolvedRoute)`; 4 call sites updated; spawn
      stores the route on the thread, sets `thread.client = route.agent`, and
      logs an `ActivityEvent::Warning` on substitution; mixed-route +
      fallback + override adapter tests.
- [x] Step 6 — dashboard surfacing (lisa-plugin)
      `ActiveThread.route`, new `AGENT` column in `render_threads`, populated
      from `thread.route.display_cell()`; route-rendering UI test.
- [x] Step 7 — full verification
      `cargo test --workspace`: lisa-cli 218 ✓, lisa-core 140 ✓, lisa-plugin
      215 ✓ / 1 pre-existing failure owned by T-027-01 (see review.md).
      `cargo build --target wasm32-wasip1 --release` ✓. `cargo fmt` applied.

## Notes / deviations

- **No loop-level default *model*** (only agent has a loop default, per
  T-025-01). A `None` model = the provider's own default, preserving today's
  behaviour. Matches the design decision; not a deviation from the plan.
- **Codex model flag** reuses the wrapper's existing `--model` passthrough
  (`agent_exec.rs`), so no CLI change was needed — the adapter just emits the
  flag on the `lisa agent-exec` line.
- **One cross-ticket test failure** in T-027-01's uncommitted provenance WIP
  (`provenance_emitted_on_error_signal`) — analysed in review.md; not caused by
  and not owned by this ticket.
