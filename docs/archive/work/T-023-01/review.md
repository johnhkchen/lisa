# T-023-01 Review — agent-exec wrapper

Handoff for a human reviewer. What changed, how it was tested, and the open
concerns a reviewer must weigh.

## What this ticket delivered

A `lisa agent-exec` subcommand — the Codex-client signal producer, the host-side
Rust analog of Claude Code's shell hooks. It runs `codex exec --json …`, streams
the JSONL event stream, writes lisa's `.lisa/signals/pane-<id>.{heartbeat,stopped,
error}` files, renders a chunked human-readable conversation to its own stdout
(the pane), persists the `thread_id` for `--resume`, and captures
`turn.completed.usage` for provenance.

## Files changed (my footprint — CLI only)

| File | Change | Lines |
|---|---|---|
| `crates/lisa-cli/src/agent_exec.rs` | **created** | 845 |
| `crates/lisa-cli/src/main.rs` | **modified** | +60 (`mod agent_exec;`, `AgentExec` variant, match arm) |
| `crates/lisa-cli/tests/fixtures/codex-success.jsonl` | **created** | recorded stream |
| `crates/lisa-cli/tests/fixtures/codex-turn-failed.jsonl` | **created** | recorded stream |

No dependency changes (`serde_json`/`serde`/`clap`/`tempfile` already present).
**No files under `crates/lisa-core/` or `crates/lisa-plugin/`** were touched by
this ticket — the pane-launch plumbing and the plugin-side `.error` consumer are
T-023-02's scope.

> **Note for the reviewer on working-tree state:** `git status` also shows
> `crates/lisa-plugin/src/adapter.rs` (new) and `crates/lisa-plugin/src/lib.rs`
> (modified). **These are not part of T-023-01** — they appeared during this
> session and are consistent with a concurrent thread building T-023-02 (the
> Codex adapter) on the same branch (lisa's concurrency model: multiple threads,
> one branch, disjoint files). They compile alongside my change but should be
> reviewed under their own ticket.

## Architecture (four layers, pure → IO)

- **Layer A/B — pure translator core** (`Translator::observe`/`finalize`,
  `render_event`, field-pluck helpers). Turns `(event lines, exit code)` into a
  signal decision + render lines with **zero IO**. This is the tested heart.
- **Layer C — writers** (`SignalWriter`, `persist_run_artifacts`,
  `read_thread_id`). Thin, tempdir-testable.
- **Layer D — command shell** (`build_codex_argv`, `run_agent_exec`). Spawns
  codex, runs the read loop, applies the anchor rule at exit. The argv builder is
  pure and unit-tested; only the spawn/stream body is un-testable in CI.

## Key decisions realised (see design.md)

1. **Defensive parse** — keys on the string `type` with prefix matching, plucks
   fields best-effort across candidate key names, renders unknown events raw
   rather than erroring. Chosen because the Codex JSON shape is `[PROVISIONAL]`
   (T-021-01 never ran against live codex). When the schema is pinned, only the
   pluck helpers change.
2. **Anchor rule** — item statuses are heartbeat-only; the terminal signal is
   decided in `finalize(exit_success)` from `turn.completed`/`turn.failed` crossed
   with the process exit code. `turn.completed && !turn.failed && exit 0` ⇒
   Success; anything else ⇒ Failure.
3. **Failure writes `.error` + `.stopped`** — resolves T-021-01 review
   Open-concern #1. `.error` is canonical (for T-023-02's future consumer +
   provenance); the compat `.stopped` keeps today's scheduler (no `.error` reader)
   from hanging on a dead pane.
4. **Degrade-safely** — `SignalWriter` no-ops when `LISA_PANE_ID` is absent,
   mirroring the hooks' `[ -n "$LISA_PANE_ID" ]` guard; codex still runs and
   renders.
5. **thread_id/usage persisted per ticket** under `.lisa/codex/<key>.thread` and
   `.usage.json` (`key` = ticket id, else `pane-<id>`, else `last`), surviving the
   plugin's deletion of `.stopped` on read.

## Test coverage

`cargo test --workspace` → **all green** (197 + 106 + 184 + 22 passing; the 22 are
this module). WASM plugin build and `cargo build -p lisa-cli` both clean, no
warnings.

Covered (22 tests in `agent_exec::tests`):
- `observe` per event type; nested/renamed `thread_id` shapes.
- Anchor rule: success requires completed + exit 0; missing `turn.completed` ⇒
  failure even on exit 0; `turn.failed`/top-level `error` ⇒ failure even on exit 0;
  non-zero exit overrides a completed turn.
- `Outcome::signals` sets (`[Stopped]` vs `[Error, Stopped]`).
- Unknown/garbage line ⇒ no heartbeat, no panic, rendered raw.
- **Recorded-stream fixtures** (the AC's "no live codex in CI"): success stream ⇒
  heartbeats + thread_id + usage captured ⇒ `[Stopped]`; same stream with
  non-zero exit ⇒ `[Error, Stopped]`; turn-failed stream ⇒ `[Error, Stopped]`.
- Writers (tempdir): filename parity (`pane-7.heartbeat` etc.); no-pane-id no-op;
  artifact round-trip; absent thread id ⇒ None.
- argv builder: default flags, bypass-sandbox, resume-with-thread, resume→--last,
  pass-through `--codex-arg`.

## Acceptance criteria check

- ✅ `lisa agent-exec` subcommand takes the prompt + codex flags, runs
  `codex exec --json`.
- ✅ writes `.heartbeat` on item events, `.stopped` on successful turn+exit,
  `.error` on failure — the exact files the plugin polls.
- ✅ renders the conversation to stdout, chunked (render-from-JSON per T-021-01 Q3).
- ✅ persists `thread_id` where a `--resume` follow-up finds it (`.lisa/codex/`).
- ✅ exposes `turn.completed.usage` to a per-run artifact (`<key>.usage.json`).
- ✅ degrades safely: no `LISA_PANE_ID` ⇒ runs + renders, writes no signals.
- ✅ unit tests over JSONL→signal translation from recorded streams, no live codex.

## Open concerns / for human attention

1. **Codex JSON shape is unconfirmed (`[PROVISIONAL]`).** T-021-01's harness never
   ran against `rust-v0.142.5` (codex not installed). The fixtures encode a
   *plausible* shape from doc 05, not a captured one. **Before T-023-02's adapter
   or T-027-02's cost capture hardcode anything, run T-021-01's harness and
   reconcile the real event names / `usage` placement against `render_event`,
   `extract_usage`, and the fixtures.** The defensive parser degrades gracefully
   to raw-render on drift, but the *render quality* and *usage capture* depend on
   the pluck keys matching reality. This is the single most important follow-up.
2. **Live spawn/stream path is not covered in CI** (no codex in CI — T-021-01's
   documented reality). All decision-bearing logic sits behind the pure translator
   and argv builder, which *are* covered. A manual `lisa agent-exec "…"` against a
   real codex is the remaining verification, gated on codex availability.
3. **Compat `.stopped` on failure** is a bridge until T-023-02 adds the `.error`
   consumer. Once that lands, T-023-02 may choose to gate off the compat `.stopped`
   so failures surface distinctly. The `Outcome::signals` mapping is the single
   place to change.
4. **`signal_dir` defaults to relative `.lisa/signals`** (matching the hooks),
   resolved against the wrapper's CWD = the pane's working tree. T-023-02's pane
   launch must keep the wrapper's CWD at the working tree (as it does for Claude)
   for attribution to land in the right directory.

## Bottom line

The wrapper is complete, self-contained to `lisa-cli`, dependency-free, and fully
tested at every layer that can be wrong without a live codex. It faithfully
reproduces lisa's signal contract and resolves the one structural gap T-021-01
flagged (`.error` no-consumer). The honest residual is the same one the spike
carried: the Codex event *shape* is reasoned, not captured — reconcile it against a
real `rust-v0.142.5` run before the downstream adapter/cost tickets bake in field
names.
</content>
