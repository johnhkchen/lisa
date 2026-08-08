# Progress — T-057-01-01 four-phases-become-one

## Status: complete. `just check` exits 0.

---

## Step 1 — Retire `auto_advance` ✅ committed `aabdb59`

Landed as planned across seven files. The two bidirectional coupling tests
(`verify_readme_config_table`, `flag_audit_covers_live_cli_config_and_prompts`) both pass, which
is the proof that the catalog entry, the README row, and the flag-audit row went together.

New test: `config::tests::retired_auto_advance_key_loads_and_is_ignored`.

**Deviation — `loop_cmd.rs` withheld from this commit.** `crates/lisa-cli/src/loop_cmd.rs` was
being modified concurrently by T-057-01-03 (the `CLAUDE.md` → `.lisa.toml` startup check).
`lisa commit-ticket --include` commits whole file contents, so including it would have swept
another ticket's uncommitted work into this ticket's commit. The three lines of `auto_advance`
removal were left in the working tree instead. T-057-01-03 has since committed the file
(`c044e26`), carrying those three lines with it. The change is durable and correct; only its
attribution landed on the neighbouring commit. Recorded in `review.md` as an open note.

**Deviation — one stray `cargo fmt --all`.** Early in this step `cargo fmt --all` reformatted
`crates/lisa-cli/src/init.rs`, a file owned by T-057-01-03. Formatting only, no semantic change;
every later format ran as `cargo fmt -p lisa-core -p lisa-plugin`.

---

## Step 2 — Collapse the enum in `lisa-core` ✅

`types.rs`, `ticket.rs`, `dag.rs` as blueprinted. `parse_phase` now delegates to
`Phase::from_name`, so the mapping table exists exactly once.

New/rewritten tests, all five acceptance criteria:

| Test | File |
|---|---|
| `test_phase_next` — walks `Ready → Implement → Review → Done → None`, then re-walks it from the front and asserts equality with `all()` | `types.rs` |
| `test_phase_artifact_filename` — rewritten: `Some("review.md")` for Review, `None` for the other three | `types.rs` |
| `retired_phase_names_map_forward_through_both_parsers` | `ticket.rs` |
| `unknown_phase_is_still_rejected` | `ticket.rs` |
| `ticket_at_retired_phase_is_rewritten_as_implement` | `ticket.rs` |
| `test_phase_to_string` — extended to assert no retired name can be emitted for any variant | `ticket.rs` |

### Deviation — a third migration surface the ticket did not name

`test_thread_deserializes_without_run_meta` failed on a `Thread` JSON carrying
`"current_phase": "research"`. Tracing it: **`Phase` is serde-persisted**, and
`crates/lisa-core/src/completion_journal.rs` records `prior_phase: Phase` on every completion
record. Journal replay is fail-closed — a replay failure fences all scheduling, which is exactly
the "loop looks dead" failure mode. A 0.4 journal line would have failed to deserialize and
bricked the board on upgrade.

The ticket names two parsers. There were three: `parse_phase`, `Phase::from_name`, and serde.

Fix, one line, same forward-mapping promise:

```rust
#[serde(alias = "research", alias = "design", alias = "structure", alias = "plan")]
Implement,
```

Serialization is unchanged — only `implement` is ever written. Pinned by a new test,
`retired_phase_names_deserialize_forward_and_are_never_reserialized`, which also asserts
`"speculate"` still fails so the widening does not become permissiveness.

---

## Step 3 — Make `lisa-plugin` compile ✅

Folded into Step 2's commit rather than committed separately, under the deviation guard written
into `plan.md`: the intermediate state (core collapsed, plugin red) is not a shippable tree, and
committing it would have left `just check` red at a commit boundary.

Three production edits, all compile-forced, exactly as scoped in `design.md` D4:

1. `lib.rs` module header — no longer recites five phases.
2. `lib.rs` ~5814 — the Ready spawn sentinel advances to `Implement`, not `Research`.
3. `lib.rs` ~6624 — the idle-signal arm `Research | Design | Structure | Plan | Review` is now
   just `Review`.

`ui.rs`: four display impls collapsed to four arms each; every retained value byte-identical.
Legend now reads `Phases: ○ Rdy ● Imp ◎ Rev ✓ Don`.

**Not touched, per D4 and left for T-057-01-02:** the `progress.md` durability admission
(~5981–5998) and the `if current_phase == Phase::Implement { "review.md" }` special case
(~6000–6008). Both still compile and behave identically — confirmed by the advance tests
passing unchanged in substance.

### Test fixtures — what needed judgement rather than substitution

Roughly 110 substitutions. The ones that were not mechanical:

- `test_check_artifact_advances_research_to_design` → `..._implement_to_review`, driven by
  `review.md`.
- `research_state_reachable_by_both_detectors` → `implement_state_reachable_by_both_detectors`.
- `test_idle_signal_research_with_artifact_advances` **deleted.** After the collapse it was a
  byte-equivalent duplicate of the pre-existing `test_idle_signal_implement_advances_to_review`
  (the compiler caught this as a duplicate fn name). The case it used to cover — idle + artifact
  on a non-Implement phase — survives as `test_idle_signal_review_with_artifact_advances_to_done`.
- `test_idle_signal_research_without_artifact_alerts` → `..._review_without_artifact_alerts`;
  Review is now the only phase that stalls waiting for an artifact.
- `test_codex_dag_advances_all_phases_via_artifacts`: the four-step artifact walk is replaced by
  an assertion that **nothing** advances until `review.md` lands, then it cascades to Done in one
  fixpoint pass.
- **Four contrast tests the blanket substitution silently broke, and the test suite caught:**
  `test_dag_status_color_is_independent_of_phase` and `test_render_threads_all_running` both need
  two *different* phases; collapsing both operands to `Implement` would have left them green in
  form and vacuous in substance. Restored to `Ready`/`Implement` and `Implement`/`Review`.
  `four_status_state` and `test_dag_ticket_ids_keep_phase_color` were re-tabled so every row's
  phase colour still differs from its status colour.
- `signal_consumer_characterization.rs`: the legacy fixture keeps its `phase: research`
  frontmatter — it is now a migration specimen — and gained an assertion that it loads as
  `Implement`. The thread moves to `Review` so the test stays about the signal *body* being
  ignored.

---

## Step 4 — Doc-comment sweep and the full gate ✅

Corrected prose that the compiler cannot reach: `types.rs` 117 and the `is_active` doc,
`dag.rs` `get_in_progress_tickets`, `lib.rs` module header, `check_idle_signals`, and the
transition-dedup doc's `Research -> Design` example.

**One further deviation.** `just lint` runs `clippy -D warnings`, and removing the Design phase
made `ui.rs`'s `MAGENTA` constant dead code — a hard error at the gate. Deleted the constant; its
only other use was a `visible_width` test that needs *some* escape code, not that one, so it now
uses `CYAN`. `YELLOW`, `BLUE` and `CYAN` remain live and were left alone.

`crates/lisa-cli/src/run_summary.rs` had a single `Phase::Plan` test fixture — one line,
surfaced by `clippy --all-targets`.

---

## Final gate

`just check` → exit **0**: `cargo check -p lisa-plugin --target wasm32-wasip1`,
`cargo fmt --all --check`, three `clippy -D warnings` invocations, `cargo test --workspace`.
Workspace totals: 312 `lisa-core`, 576 `lisa-plugin`, 396 `lisa-cli`, plus integration suites.
Zero failures, zero ignored beyond the one pre-existing ignore.

## Commits

| Commit | Contents |
|---|---|
| `aabdb59` | Retire `auto_advance` (`types.rs`, `config.rs`, `setup_guide.rs`, plugin dump, README, flag-audit) |
| _this step_ | The phase collapse: `types.rs`, `ticket.rs`, `dag.rs`, `lib.rs`, `ui.rs`, `signal_consumer_characterization.rs`, `run_summary.rs` |
| `c044e26` (T-057-01-03) | Carried this ticket's three-line `loop_cmd.rs` change — see Step 1 deviation |
