# T-057-01-05 — Progress

All five plan steps are complete and committed. Five commits, `just check` green (exit 0).

| Step | Commit | State |
|---|---|---|
| 1+2 — write `lisa-workflow.md`, capture the 0.4.4 generation | `108d280` | done |
| 2b — delete both copies of `rdspi-workflow.md` | `032e34b` | done |
| 3 — the migration (`RemoveFile`, `plan_retired_template`, install path, validate) | `045523c` | done |
| 4 — doc comments, role contract, config validator, setup guide | `e1f09d7` | done |
| 5 — README, CONTRIBUTING, CLAUDE/AGENTS, roadmap, PKGBUILD, fixtures | `d138d08` | done |

## Deviations from the plan

**Steps 1 and 2 became one commit.** The plan expected step 1 to be independently green. It
cannot be: `test_plan_init_updates_every_known_workflow_template` asserts every legacy generation
is byte-*distinct* from the current document, and until the document is rewritten the 0.4.4
capture is byte-identical to it. The capture was still taken and verified first, which is what
step 1 was for.

**Step 2 split into add-then-delete.** `lisa commit-ticket` refused a transaction that both added
`lisa-workflow.md` and deleted `rdspi-workflow.md`, with `ordinary staged entries changed during
verification`. Probes isolated it: adds alone commit, deletes alone commit, add-plus-near-identical-delete
in one transaction does not. Written up as an open concern in `review.md` — it is a real
`commit-ticket` limitation, not a defect in this work.

**Five probe commits were made and removed.** Diagnosing the above put five throwaway commits on
`main`. They were unpushed and no journal or ledger references commit SHAs, so `git reset --mixed`
returned `main` to `0bbd91c` with the worktree intact, and the work was then committed properly.
Recorded in `review.md`; recoverable from the reflog either way.

**Criterion 7's grep needs stated exclusions.** Two code sites must keep naming the retired path
— the legacy `include_str!`s and the `init.rs` migration that removes it — because criteria 2 and
3 require exactly that. Reconciled in `review.md` against the exact command and its output.
