# Plan — T-057-01-02 the-scheduler-stops-counting-phases

Five steps, three commits. Each step is verifiable on its own; the acceptance criterion it
discharges is named.

---

## Step 0 — Baseline

```
just check
git status --porcelain
```

Establishes that the tree is green before any edit (so a later red is mine) and that no sibling
thread has uncommitted work in `crates/lisa-core/src/types.rs`,
`crates/lisa-plugin/src/lib.rs`, or `crates/lisa-plugin/src/ui.rs`.

**Verify:** `just check` exits 0; none of the three files appear in `git status`.

---

## Step 1 — `Phase::completion_artifact()` — *commit 1*

Files: `crates/lisa-core/src/types.rs`.

1. Add the method after `artifact_filename()` (structure F1.1), exhaustive `match`, doc comment
   naming the divergence and its consequence.
2. Add `phase_completion_artifact_diverges_from_what_the_phase_produces` after
   `test_phase_artifact_filename` (F1.2).

**Verify:** `cargo test -p lisa-core` green; the new test names both methods in the same
assertions. `cargo clippy -p lisa-core -- -D warnings` green (a `pub` method on a library type is
not dead code).

**Commit:** `lisa commit-ticket --ticket-id T-057-01-02 --message "Name the artifact that
completes a phase" --include crates/lisa-core/src/types.rs`

**Discharges:** the enum half of AC 1.

---

## Step 2 — The plugin subtraction — *commit 2*

Files: `crates/lisa-plugin/src/lib.rs`.

Order within the step matters only for keeping the compiler useful:

1. **F2.2 detector.** Delete the `progress.md` admission block (~5978–5998); replace the
   `if/else` artifact-name selection with the `completion_artifact()` lookup; rewrite the
   function's doc comment.
2. **F2.3 idle handler.** Delete the `progress.md` admission from the `Phase::Implement` arm.
   Leave the `Phase::Review` arm byte-identical.
3. **F2.1 citation.** Drop `work_dir.join("progress.md")` from `inspected_paths`.
4. **F2.4 parked label.** Replace `unwrap_or("artifact.md")` with the `match` on
   `artifact_filename()` that falls back to the work directory.
5. Run `cargo test -p lisa-plugin`. Expect exactly two failures — the two tests that assert the old
   behaviour — and read each before touching it. **An unexpected third failure is a finding, not a
   fixture to update.**
6. **F2.5 tests.** Invert `test_check_artifact_advances_implement_ignores_progress_md` →
   `implement_does_not_publish_progress_md`. Strengthen
   `test_check_artifact_advances_implement_to_review_via_review_md` with the
   `artifact_filename() == None` assertion and its explanatory message. Re-prove
   `operator_override_cites_review_and_progress_only_when_they_exist` through `review.md` and
   rename it. Add `idle_signal_at_implement_does_not_publish_progress_md`,
   `phase_transitions_logged_are_exactly_the_new_chain`, and
   `parked_thread_at_implement_cites_its_work_directory`.
7. `grep -n "progress.md" crates/lisa-plugin/src/lib.rs` — the only survivors must be inside the
   two negative tests.
8. `grep -n "artifact.md" crates/lisa-plugin/src/` — no hits.

**Verify:** `cargo test -p lisa-plugin` green; `cargo check -p lisa-plugin --target wasm32-wasip1`
green; both greps as specified above.

**Commit:** `lisa commit-ticket --ticket-id T-057-01-02 --message "Stop publishing progress.md and
read the phase edge from one place" --include crates/lisa-plugin/src/lib.rs`

**Discharges:** AC 1 (behavioural half), AC 2, AC 3 (by leaving the Review arm alone and showing
its test still passes unmodified), AC 4, AC 6.

---

## Step 3 — The four-phase board test — *commit 3*

Files: `crates/lisa-plugin/src/ui.rs`.

1. Add `render_threads_draws_a_four_phase_board` (F3.1): four slots, one per phase, positive
   assertions on `RDY`/`IMP`/`REV`/`DON` (each exactly once) and negative assertions on every
   retired short name and full word.
2. Retarget the three stale `artifact_path` fixtures (F3.2).
3. Confirm no production change was needed — if the new test passes on first run, that *is* the
   evidence for AC 5's "no dead arms and no gap", and it goes in `review.md` as such.

**Verify:** `cargo test -p lisa-plugin ui::` green; the new test fails if `IMP` is removed from
`short_name` (checked by hand-editing and reverting, not committed).

**Commit:** `lisa commit-ticket --ticket-id T-057-01-02 --message "Pin the four-phase board"
--include crates/lisa-plugin/src/ui.rs`

**Discharges:** AC 5.

---

## Step 4 — Gate and review

1. `cargo fmt -p lisa-core -p lisa-plugin` — scoped, so no sibling ticket's file is reformatted.
   If it changes anything, amend into the owning commit by re-running `lisa commit-ticket` on the
   same path.
2. `just check` — the full gate, by exit code, not by reading output.
3. `git status --porcelain` — no ticket-owned file left staged, modified, or untracked.
4. Write `review.md` and `review-disposition.json`; run `lisa check-disposition T-057-01-02`.

**Discharges:** AC 7.

---

## Testing strategy

**Unit (lisa-core).** One test, on the new method, asserting the divergence rather than the values
in isolation. The value assertions alone would survive a refactor that made
`completion_artifact()` delegate to `artifact_filename()` — the paired assertion is what does not.

**Behavioural (lisa-plugin, in-process `State`).** Every acceptance criterion that is about the
scheduler is tested through `check_artifact_advances` / `check_idle_signals` against a real
tempdir, real ticket file, and a real attempt lease — the same fixture shape the surrounding ~40
tests use. No mocking of the admission layer: a test that stages `review.md` and asserts the ticket
file on disk says `phase: review` is the only kind that would have caught this ticket's named
regression.

**Rendering (lisa-plugin `ui`).** String assertions over `render_threads`, positive and negative.
No golden snapshot — the rows carry ANSI codes and elapsed-time cells, so a byte-exact file would
fail on unrelated formatting work and train the next person to regenerate without reading.

**Verification criteria, restated as the questions each test answers:**

| Question | Test |
|---|---|
| Does a ticket at Implement advance when `review.md` appears? | `test_check_artifact_advances_implement_to_review_via_review_md` |
| Would re-deriving the name from `artifact_filename()` break that? | same test, via its `assert_eq!(…artifact_filename(), None)` preamble |
| Is `progress.md` still published by the artifact detector? | `implement_does_not_publish_progress_md` |
| …by the idle handler? | `idle_signal_at_implement_does_not_publish_progress_md` |
| Does the idle Review→Done path still work, untouched? | `test_idle_signal_review_with_artifact_advances_to_done`, unmodified |
| What does an operator read for a parked Implement thread? | `parked_thread_at_implement_cites_its_work_directory` |
| Does the board draw four phases and only four? | `render_threads_draws_a_four_phase_board` |
| Does the feed report only real edges? | `phase_transitions_logged_are_exactly_the_new_chain` |

## Risks

1. **A test that compiles and proves less.** T-057-01-01's review records four fixtures that
   silently weakened when two phases collapsed into one operand. The same hazard applies to the
   test inversions here: `implement_does_not_publish_progress_md` must assert the *absence of a
   path*, not merely that the phase did not advance — the latter would pass even if publication
   came back. Guarded by writing the negative assertion first and watching it fail against the
   unmodified code.
2. **Sweeping a sibling's file into a commit.** `--include` commits whole file contents. Mitigated
   by confining paths to `lisa-core`/`lisa-plugin`, checking `git status` before each commit, and
   scoping `cargo fmt`.
3. **`record_artifact_ownership` losing a call site.** Reasoned through in structure F2.2: the
   remaining artifact path is `review.md`, and the primary ownership paths are assignment-time and
   untouched. If a plugin test asserting ownership-from-progress.md exists, it will fail in Step
   2.5 and is a finding to report, not a fixture to edit.
