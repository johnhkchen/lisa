# Progress — T-057-01-02

All four plan steps complete. Three commits, `just check` exit 0.

| Step | State | Commit |
|---|---|---|
| 0 — baseline | done | `just check` 0 on an unmodified tree |
| 1 — `Phase::completion_artifact()` | done | `2a55e7e` Name the artifact that completes a phase |
| 2 — plugin subtraction | done | `c4772bd` Stop publishing progress.md and read the phase edge from one place |
| 3 — four-phase board test | done | `201fe77` Pin the four-phase board |
| 4 — gate and review | done | `just check` exit 0; tree clean of ticket-owned files |

## Deviations from the plan

**None in substance.** Two details worth recording:

1. **Step 2.5 predicted "exactly two failures"; there was one.**
   `operator_override_cites_review_and_progress_only_when_they_exist` kept passing, because its
   `progress.md` assertion was already a *negative* one. It was still rewritten — the property it
   proved ("a citation names only what is there") was being carried by a file that can no longer
   exist, so it now proves the same thing through a `review.md` that is genuinely absent in one
   half of the test and present in the other.

2. **Step 3 needed no production change, as designed.** `render_threads_draws_a_four_phase_board`
   passed on first run. That is the evidence for AC 5 rather than a shortcut around it, and it was
   confirmed to be load-bearing by mutation (below).

## Mutation checks run (not committed)

Each was applied, observed, and reverted.

| Mutation | Expected | Observed |
|---|---|---|
| `completion_artifact()` → `self.artifact_filename()` | core test fails | `phase_completion_artifact_diverges_from_what_the_phase_produces` FAILED |
| detector reads `artifact_filename()` instead | Implement strands; the named regression test fails | 14 plugin tests FAILED, including `test_check_artifact_advances_implement_to_review_via_review_md` and `phase_transitions_logged_are_exactly_the_new_chain` |
| `Phase::Implement.short_name()` → `"PLN"` | board test fails on the retired-name half | `render_threads_draws_a_four_phase_board` FAILED at the `PLN` assertion |

## Test counts

`lisa-core` 313 (+1), `lisa-plugin` 580 (+4), `lisa-cli` 396 (unchanged). Zero failures anywhere.
