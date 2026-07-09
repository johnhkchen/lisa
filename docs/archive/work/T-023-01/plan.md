# T-023-01 Plan — agent-exec wrapper

Ordered, independently-verifiable steps. Each is small enough to commit atomically.
Testing strategy is called out per step. Verification commands:
`cargo test -p lisa-cli`, `cargo build -p lisa-cli --release`, `just check`.

## Step 1 — Pure vocabulary + translator core (Layers A & B)

Create `crates/lisa-cli/src/agent_exec.rs` with:
- `SignalKind` (+ `suffix`), `StreamEffect`, `Outcome` (+ `signals`).
- `Translator` struct + `observe` + `finalize`.
- Field-pluck helpers (`event_type`, `extract_thread_id`, `extract_usage`,
  `item_of`, `item_kind`, `item_text`) and `render_event`.
- `mod tests` covering `observe` per event type, `finalize` anchor-rule cases.

**Test:** unit tests only, no IO. `cargo test -p lisa-cli agent_exec::tests`.
**Verify:** heartbeat on `item.*`; terminal signal only from `finalize`; anchor
rule (no `turn.completed` OR non-zero exit ⇒ Failure); `turn.failed` ⇒ Failure
even on exit 0; unknown line ⇒ no panic, no heartbeat.
**Commit:** "agent-exec: pure JSONL→signal translator core + tests".

## Step 2 — Signal & artifact writers (Layer C)

Add `SignalWriter` (guarded on pane id), `persist_run_artifacts`, `read_thread_id`.
Timestamp = epoch seconds (no date dep). `mod tests` with `tempfile::tempdir`.

**Test:** `SignalWriter` writes `pane-<id>.<suffix>`; `pane_id: None` writes
nothing; artifacts round-trip (`persist` then `read_thread_id`); `usage.json`
parses back to the stored `usage`.
**Verify:** filenames match the hook format exactly (`pane-7.heartbeat` etc.).
**Commit:** "agent-exec: signal + run-artifact writers with tempdir tests".

## Step 3 — Command shell + CLI wiring (Layer D + main.rs)

- `AgentExecArgs`, `run_agent_exec` (spawn, line loop, heartbeat writes, render,
  wait, finalize, write terminal signals, persist artifacts, `--resume`
  resolution).
- `main.rs`: `mod agent_exec;`, `AgentExec` variant, match arm.

**Test:** compile-level + a `--help`/parse smoke test (`assert_cmd`-free: just
construct `AgentExecArgs` and check argv-building if factored into a pure
`build_codex_argv(&args, resolved_thread) -> Vec<String>` helper — add that helper
so argv construction is unit-testable without spawning).
**Verify:** `cargo build -p lisa-cli --release`; `lisa agent-exec --help` lists the
flags; argv-builder test asserts `exec --json --skip-git-repo-check -C … -a never
-s workspace-write … <prompt>` and the `--resume`/`--bypass-sandbox` variants.
**Commit:** "agent-exec: codex spawn + render loop, wired into the CLI".

## Step 4 — Fixtures + full-stream integration of the pure core

Add `crates/lisa-cli/tests/fixtures/codex-success.jsonl` and
`codex-turn-failed.jsonl` (representative, `[PROVISIONAL]` shape matching doc 05).
`include_str!` them into translator tests that feed the whole stream line-by-line.

**Test:** success fixture ⇒ ≥1 heartbeat, `thread_id` = expected, `usage` captured,
`finalize(true)` ⇒ `[Stopped]`; turn-failed fixture ⇒ `finalize(true)` ⇒
`[Error, Stopped]`.
**Verify:** these are the AC's "recorded event streams, no live codex" tests.
**Commit:** "agent-exec: recorded-stream fixtures + end-to-end translator tests".

## Step 5 — Full workspace verification + docs note

- `cargo test --workspace` green.
- `cargo build -p lisa-cli --release` green.
- `just check` (WASM check + tests) green.
- Confirm no plugin/core files changed (`git diff --stat`).

**Commit:** "agent-exec: verify build/test; T-023-01 wrapper complete".

## Testing strategy summary

| Concern | Level | Where |
|---|---|---|
| JSONL→signal mapping (AC core) | unit, pure, fixture-driven | `observe`/`finalize` tests, Steps 1 & 4 |
| Anchor rule (turn events × exit) | unit | `finalize` tests, Step 1 |
| Degrade-safely (no `LISA_PANE_ID`) | unit | `SignalWriter` `pane_id: None`, Step 2 |
| Signal filename parity with hooks | unit, tempdir | `SignalWriter`, Step 2 |
| thread_id/usage persistence | unit, tempdir | `persist`/`read_thread_id`, Step 2 |
| argv construction (flags/resume) | unit | `build_codex_argv`, Step 3 |
| Live codex spawn/render | **manual only** | out of CI (no live codex; T-021-01 gap) |

The live spawn path is deliberately un-unit-tested (no codex in CI — T-021-01's
documented reality). Everything decision-bearing is behind the pure translator and
argv-builder, so CI covers all logic that can be wrong. Manual verification
(`lisa agent-exec "…"` against a real codex) is a post-merge check gated on the
same `rust-v0.142.5` availability T-021-01 flagged.

## Risks & mitigations carried into Implement

- **Schema drift** (`[PROVISIONAL]` JSON): defensive parse + "unknown ⇒ render raw,
  no crash" (Design 2). If a real run shows different field names, only the pluck
  helpers change; the translator/tests structure holds.
- **`.error` no-consumer:** compat `.stopped` written alongside `.error` (Design
  4). Verified by `Outcome::signals` test.
- **Resume without thread id:** fall back to `--last`; error clearly only if
  `--resume` is set with neither a persisted id nor a ticket id.
</content>
