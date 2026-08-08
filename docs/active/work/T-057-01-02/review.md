# Review — T-057-01-02 the-scheduler-stops-counting-phases

The scheduler no longer publishes `progress.md`, and the rule that `review.md` is what moves a
ticket off Implement now lives in a named method instead of an `if` in the middle of a detector.
Three commits. `just check` exit 0.

---

## What changed

### `2a55e7e` — Name the artifact that completes a phase

`crates/lisa-core/src/types.rs`: new `Phase::completion_artifact()`, plus one test.

```rust
pub fn completion_artifact(&self) -> Option<&'static str> {
    match self {
        Phase::Implement | Phase::Review => Some("review.md"),
        Phase::Ready | Phase::Done => None,
    }
}
```

Beside the existing `artifact_filename()`, which is untouched. The two answer different questions
— *"what does this phase produce?"* versus *"what file appearing means this phase is over?"* — and
they diverge at exactly one phase. Implement produces nothing; the file that ends it is Review's
artifact, written before the agent enters Review.

### `c4772bd` — Stop publishing progress.md and read the phase edge from one place

`crates/lisa-plugin/src/lib.rs`, four production edits:

| Site | Before | After |
|---|---|---|
| `check_artifact_advances` ~5978 | 20-line `progress.md` durability admission | deleted |
| `check_artifact_advances` ~5999 | `if current_phase == Phase::Implement { "review.md" } else { … }` | `match current_phase.completion_artifact()` |
| `check_idle_signals`, Implement arm ~6568 | a second `progress.md` admission | deleted |
| `inspected_paths` ~2273 | `progress.md` among an operator's citable evidence | deleted |
| `to_ui_state` ~9859 | `artifact_filename().unwrap_or("artifact.md")` | the ticket's work directory when the phase has no artifact |

The detector's doc comment was rewritten to state the rule the code now carries. After this
change, `check_artifact_advances` names exactly one phase in its whole body — `Phase::Done`, at
the completion dispatch.

### `201fe77` — Pin the four-phase board

`crates/lisa-plugin/src/ui.rs`: **test-only.** One new board test, three stale fixture strings
retargeted (`design.md` / `research.md` → `review.md` or the bare work directory). No production
line changed, because T-057-01-01 had already rewritten every `ui::Phase` arm — see "AC 5" below.

Nothing created, nothing deleted at file level.

---

## Test coverage

Every acceptance criterion has a named test, and each was checked by mutation — applied, observed
red, reverted. Green alone would not distinguish a load-bearing test from a decorative one, and
T-057-01-01's review recorded four fixtures that silently stopped proving anything during the
phase collapse.

| AC | Test | Mutation that proves it bites |
|---|---|---|
| 1 — Implement advances on `review.md`; re-deriving from `artifact_filename()` must fail it | `test_check_artifact_advances_implement_to_review_via_review_md` (strengthened) + `phase_completion_artifact_diverges_from_what_the_phase_produces` (new, `lisa-core`) | detector → `artifact_filename()`: **14 plugin tests fail**. `completion_artifact()` → delegate: the core test fails first. |
| 2 — no code path publishes `progress.md` | `implement_does_not_publish_progress_md` (inverted) + `idle_signal_at_implement_does_not_publish_progress_md` (new) | both assert the *absence of a published path*, not merely that the phase did not move — the weaker form would pass with publication restored |
| 3 — idle arm is `Phase::Review` alone, body unchanged | `test_idle_signal_review_with_artifact_advances_to_done`, **unmodified and green** | — |
| 4 — no `unwrap_or("artifact.md")` on an operator surface | `parked_thread_at_implement_cites_its_work_directory` (new) | asserts both the Implement path (directory) and the Review path (`review.md`), and that no parked citation anywhere contains `artifact.md` |
| 5 — four-phase board, no dead arms, no gap | `render_threads_draws_a_four_phase_board` (new) | `Implement.short_name()` → `"PLN"`: fails on the retired-vocabulary half |
| 6 — transition log emits only new-chain edges | `phase_transitions_logged_are_exactly_the_new_chain` (new) | asserts the detector's emitted edges are exactly `[(Implement, Review)]` and that each is a real `next()` edge |
| 7 — `just check` green | exit code 0, run before and after the final commit | — |

**Counts:** `lisa-core` 313 (+1), `lisa-plugin` 580 (+4), `lisa-cli` 396 (unchanged). Zero
failures.

### The one test whose strength is easy to overstate

`phase_transitions_logged_are_exactly_the_new_chain` drives the artifact detector only, so it
observes `Implement → Review` and asserts nothing else was emitted. It does **not** reach
`Review → Done`, which is logged at `lib.rs` ~3486 on completion-commit success and needs the
completion transaction to actually run. What covers that edge instead: the four-variant `Phase`
makes a retired phase inexpressible at the type level, and the call site derives its `from` from
`pending.prior_phase`. The test does check the one place production code writes an edge as a
literal pair (`Implement → Review`, idle handler) against `Phase::next()`, which is the part a
type system cannot check.

---

## Decisions a reviewer may want to push back on

### 1. The fix landed in `lisa-core`, not only in `lisa-plugin`

The ticket names three sites, all in `lib.rs`, and asks that whatever replaces the Implement
special case "deserves a test that fails if someone later simplifies it back into the generic
lookup." The minimal version of that is a `match current_phase { Phase::Implement => "review.md", … }`
left in the detector. I put the fact in `lisa-core` instead, as a method with a name and a doc
comment, because a bare literal in the middle of a 130-line function *is* the shape that invites
the simplification the ticket is guarding against — and because it lets the anti-regression
assertion be stated as one line beside the phase chain rather than only as a behavioural test 8000
lines away.

Cost: one file outside the ticket's named boundary. `crates/lisa-core/src/types.rs` was
unmodified before this commit and no in-flight sibling touches `lisa-core` (T-057-01-03 is
`lisa-cli`), verified with `git status` immediately before committing.

### 2. A second `progress.md` publication site, not named in the ticket

The ticket body names one admission block. There were two: `check_idle_signals`' Implement arm
(~6568) published `progress.md` as well. AC 2 is written about behaviour — *"No code path writes
or publishes `progress.md`"* — so both went. Leaving the idle one would have meant an agent that
goes idle still gets the file published, which is the outcome AC 2's test forbids.

The ticket's sentence *"The body is unchanged"* refers to the five-pattern
`Research | Design | Structure | Plan | Review` arm, which T-057-01-01 had already narrowed to
`Phase::Review`. That arm is byte-identical, and its test passes unmodified.

### 3. A parked thread at Implement cites a directory, not a filename

`unwrap_or("artifact.md")` reached an operator through a `ReviewWait` desk card's evidence
citation (`lib.rs` ~9698) as `docs/active/work/T-xxx/artifact.md` — a filename this codebase has
never written. Options were `review.md` unconditionally (which lies: a thread parked mid-implement
has not written one) or the ticket's work directory. I took the directory, matching the precedent
`inspected_paths` already sets two thousand lines up: *"The file is not there; the directory the
operator opened is."* One convention, used in both places.

This field reads `artifact_filename()` and deliberately **not** `completion_artifact()` — it
answers "what did this phase write?", and Implement's honest answer is "nothing of its own".

### 4. `ui.rs` needed no production change

AC 5 reads like a code criterion. It is not, any more: T-057-01-01 rewrote `ui::Phase` to four
variants with four arms in each of `short_name`/`full_name`/`color_code`/`indicator`, a four-entry
DAG legend, and a `render_threads` whose column widths come from a format string rather than a
phase count. The new board test passing on first run is the evidence, and the mutation check is
what makes that evidence mean something.

---

## Open concerns

### A. `record_artifact_ownership` lost one of its two call sites

Removing the `progress.md` admission removes the *earlier* of the two artifact-driven seat
ownership promotions; `review.md`'s remains. This is a real change in when a seat can flip to
`Owned` for an agent that writes `progress.md` early and `review.md` late.

Assessed as safe and not compensated for: `record_artifact_ownership` is documented in the code as
"the weaker, bounded ownership fallback". The primary paths — `admit_codex_ack` (~4176) and
`admit_assignment_claim` (~4214) — fire at assignment time and are untouched. No test asserted
ownership-from-`progress.md`; the full suite is green. Flagging it because "an ownership signal got
later" is the kind of change that shows up as a rare timing symptom rather than a red test.

### B. The workflow document still asks agents for `progress.md`

`docs/knowledge/rdspi-workflow.md` and `crates/lisa-cli/data/rdspi-workflow.md` still instruct
every agent to write it. That is T-057-01-05's file, and out of scope here by the ticket's own
text. The consequence in the meantime is benign and is what AC 2's test pins: an agent writes
`progress.md` into its private attempt directory, and it stays there. This attempt wrote one.

### C. `commit_transaction.rs` still lists six artifact names in a fixture

`crates/lisa-cli/src/commit_transaction.rs` ~1850/1875 seeds a work directory with
`research.md`…`review.md` to prove `complete-ticket` commits the whole directory. The sweep is
name-agnostic, the test is correct, and the file belongs to `lisa-cli`. Left alone.

### D. Not touched, by scope

The assignment prompt (`lib.rs` ~120–172) still recites six phases — T-057-01-04.

---

## Verification

```
just check → 0
  cargo check -p lisa-plugin --target wasm32-wasip1
  cargo fmt --all -- --check
  cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
  cargo clippy -p lisa-core -- -D warnings
  cargo clippy -p lisa-cli -- -D warnings
  cargo test --workspace
```

Run at baseline (green before any edit) and after the final commit (green). `git status` carries
no ticket-owned source file staged, modified, or untracked. `cargo fmt` was run scoped to
`-p lisa-core -p lisa-plugin` throughout, so no sibling ticket's file was reformatted.

Residual greps, both intentional:

- `progress.md` in `lisa-plugin/src/lib.rs` — only inside the two tests that assert its absence
  and the citation test that names it as retired.
- `artifact.md` in `crates/lisa-plugin/` — only the negative assertion in
  `parked_thread_at_implement_cites_its_work_directory`.
