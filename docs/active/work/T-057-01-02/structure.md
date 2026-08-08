# Structure — T-057-01-02 the-scheduler-stops-counting-phases

Three files modified. None created, none deleted. Ordering matters at exactly one seam and is
called out at the end.

---

## F1 — `crates/lisa-core/src/types.rs`

### F1.1 New method `Phase::completion_artifact()` (insert after `artifact_filename`, ~177)

Public API addition on an existing type. No signature elsewhere changes.

```rust
/// The artifact whose appearance completes this phase.
///
/// Not the same question as [`Phase::artifact_filename`], which answers
/// "what does this phase produce?". The two diverge at exactly one phase,
/// and the divergence is a scheduling rule rather than a detail:
/// `Implement` produces nothing of its own, and the file that moves a
/// ticket off it is `review.md` — Review's artifact, written by the agent
/// before it enters Review. Fold this back into `artifact_filename()` and
/// every ticket strands at Implement forever.
pub fn completion_artifact(&self) -> Option<&'static str> {
    match self {
        Phase::Implement | Phase::Review => Some("review.md"),
        Phase::Ready | Phase::Done => None,
    }
}
```

Exhaustive `match` on `Phase` (no `_` arm) so a fifth variant is a compile error, not a silent
`None`.

### F1.2 New test `phase_completion_artifact_diverges_from_what_the_phase_produces`

Placed immediately after `test_phase_artifact_filename` (~1162) so the two read together.

Asserts, with the reason in the message:

- `Phase::Implement.completion_artifact() == Some("review.md")` **and**
  `Phase::Implement.artifact_filename() == None` — the divergence, stated in one test.
- `Phase::Review.completion_artifact() == Some("review.md")`.
- `Phase::Ready` / `Phase::Done` → `None` on both.
- Every phase with a `completion_artifact` has a `next()`, i.e. no phase can complete into nothing.

---

## F2 — `crates/lisa-plugin/src/lib.rs`

Four edits, listed in file order.

### F2.1 `inspected_paths` — drop the retired citation entry (~2267–2278)

```rust
OverriddenAsk::Block { .. } => [
    self.review_disposition_path(ticket_id),
    work_dir.join("review.md"),
    // work_dir.join("progress.md"),   ← removed
]
```

The array literal narrows from 3 to 2 elements; the `.filter(|path| path.exists())` chain below is
unchanged. Nothing else in the function moves.

### F2.2 `check_artifact_advances` — the detector (~5951–6008)

**Doc comment (~5951–5965).** The two paragraphs explaining `progress.md` as a living document are
replaced by one that states the rule the code now carries:

> Each phase advances when its completion artifact appears — see
> [`Phase::completion_artifact`]. Implement and Review share `review.md`, which is why
> the phase edge is read from that method and not from `artifact_filename()`.

**Body.** Delete lines ~5978–5998 in full (the `if current_phase == Phase::Implement { … }`
`progress.md` admission, including its two-line comment). Replace ~5999–6008 with:

```rust
let artifact_name = match current_phase.completion_artifact() {
    Some(name) => name,
    None => continue,
};
```

Net: the loop body opens directly on the artifact lookup. `Phase` is now named exactly once in the
whole function — at the `next_phase == Phase::Done` completion dispatch (~6034).

**Consequence to note, not to fix:** `record_artifact_ownership` loses one of its two call sites.
The remaining artifact-driven ownership promotion is `review.md`'s; the primary paths
(`admit_codex_ack`, `admit_assignment_claim`) are untouched and fire at assignment time, which is
where seat ownership is actually established.

### F2.3 `check_idle_signals` — the Implement arm (~6567–6577)

Delete the `admit_artifact(..., "progress.md")` call and its error-logging `if let Err` block, plus
the `// Idle signal alone is the completion signal for Implement` comment's now-orphaned
predecessor. The arm then opens on:

```rust
Phase::Implement => {
    // Idle signal alone is the completion signal for Implement
    let file_path = self.dag.get_ticket(&ticket_id).map(|t| t.file_path.clone());
    …
```

Everything from `let file_path` onward is byte-identical to today, including the
`log_phase_transition(Implement, Review)` and the same-tick `review.md` completion dispatch.

**`Phase::Review` arm (~6625 onward): not touched.** AC 3 requires its body unchanged, and its
`current_phase.artifact_filename()` lookup is correct there (`Review → Some("review.md")`). It is
deliberately *not* migrated to `completion_artifact()` — same value, and the ticket asks for that
arm to stay still.

### F2.4 The parked-thread label (~9852–9860)

```rust
ui::ParkedThread {
    ticket_id: t.ticket_id.clone(),
    phase: phase_to_ui_phase(t.current_phase),
    artifact_path: match t.current_phase.artifact_filename() {
        Some(name) => format!("{}/{}/{}", self.config.work_dir.display(), t.ticket_id, name),
        // The phase wrote no artifact of its own — name the directory the
        // operator would open, the same answer `inspected_paths` gives when
        // review.md is not on file.
        None => format!("{}/{}", self.config.work_dir.display(), t.ticket_id),
    },
    …
}
```

`unwrap_or("artifact.md")` disappears from the codebase.

### F2.5 Tests in `lib.rs`

| Test | Action |
|---|---|
| `test_check_artifact_advances_implement_ignores_progress_md` | **Invert and rename** → `implement_does_not_publish_progress_md`. Keeps the "does not advance" half; the assertion that `work_dir/T-002/progress.md` holds `# Progress` becomes an assertion that the path does not exist, and that the ticket's canonical work directory contains no `progress.md`. AC 2's test. |
| `test_check_artifact_advances_implement_to_review_via_review_md` | **Strengthen.** Add, at the top, `assert_eq!(Phase::Implement.artifact_filename(), None, …)` with a message naming why the test is load-bearing: if the detector is ever re-derived from `artifact_filename()` alone, Implement yields `None`, the loop `continue`s, and the advance below fails. AC 1's test. |
| `operator_override_cites_review_and_progress_only_when_they_exist` | **Rename** → `operator_override_cites_only_the_evidence_that_exists`, and re-prove the property through `review.md`: one dispatch with `review.md` written (citation names it), one without (citation names the disposition only, and does not name `review.md`). |
| *new* `idle_signal_at_implement_does_not_publish_progress_md` | Mirrors F2.3: stage a `progress.md`, fire the idle signal, assert the phase advanced to Review **and** nothing named `progress.md` reached the canonical work directory. |
| *new* `phase_transitions_logged_are_exactly_the_new_chain` | Drives one ticket Implement → Review → Done through the two detectors and asserts the emitted `PhaseCompleted` phases are exactly `[Implement, Review]`, each once. AC 6's test. |
| `test_idle_signal_review_with_artifact_advances_to_done` | **Unmodified**, and must stay green. AC 3's evidence. |
| `test_idle_signal_implement_advances_to_review` | Unmodified; already asserts no `progress.md` behaviour. |
| *new* `parked_thread_at_implement_cites_its_work_directory` | Builds the dashboard state with a parked thread at Implement and asserts the `ParkedThread.artifact_path` ends in `/T-xxx` with no filename, and that no rendered surface contains `artifact.md`. AC 4's test. |

---

## F3 — `crates/lisa-plugin/src/ui.rs`

**No production code changes.** Research and D4 confirm the module is already four-phase with no
dead arms.

### F3.1 New test `render_threads_draws_a_four_phase_board`

One `PluginState` with four slots, one thread per phase — Ready and Implement and Review as
active/parked rows, Done as an active row (the phase is renderable; the row is what a thread looks
like in the tick before its slot is released). Asserts:

- all four of `RDY`, `IMP`, `REV`, `DON` appear;
- each appears exactly once;
- none of `RES`, `DES`, `STR`, `PLN`, `Research`, `Design`, `Structure`, `Plan` appears anywhere
  in the output.

The negative half is what makes this a four-phase board test rather than four independent
assertions.

### F3.2 Fixture hygiene (3 lines)

`artifact_path` fixtures at ~3117, ~4819, ~4900 name `design.md` and `research.md` — files this
release no longer produces. Retargeted to `review.md` (and, for the parked-at-Implement fixture at
~4819, to the bare work directory, matching F2.4's output). These are inputs, not assertions; no
test's meaning changes.

---

## Ordering

One real dependency: **F1.1 must land before F2.2**, because the detector calls the new method.
Everything else is independent.

Commit sequence (each a `lisa commit-ticket` with exact `--include` paths):

1. `crates/lisa-core/src/types.rs` — F1.1 + F1.2. Compiles and tests green on its own (the method
   is unused by the plugin at this point; it is `pub` on a library type, so no dead-code warning).
2. `crates/lisa-plugin/src/lib.rs` — F2.1 through F2.5. The whole plugin change lands together
   because F2.2's deletion and the test inversion in F2.5 are the same fact.
3. `crates/lisa-plugin/src/ui.rs` — F3.1 + F3.2.

`--include` paths are confined to `crates/lisa-core/src/types.rs`, `crates/lisa-plugin/src/lib.rs`
and `crates/lisa-plugin/src/ui.rs`. T-057-01-03 is in flight in `crates/lisa-cli/`; `git status` is
checked immediately before each commit to confirm no other thread has touched these three files.
`cargo fmt` is run scoped (`-p lisa-core -p lisa-plugin`) so no sibling's file is reformatted — the
mistake T-057-01-01's review recorded as open concern B.
