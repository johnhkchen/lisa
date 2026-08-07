# T-056-01-02 — Review: the-check-can-see-the-project

## What changed

Two files, two commits, both through `lisa commit-ticket`. No files created or deleted.
`267 insertions(+), 337 deletions(-)` — the fix is net negative.

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/unblock.rs` | `run_check` spawns with `.current_dir(root)`; 253 lines of snapshot machinery deleted; `CheckResult::ChangedFiles` and `DECLINE_CHANGED_FILES` removed; two unit tests removed, two added, one edited |
| `crates/lisa-cli/tests/parked_ux.rs` | Three fixture helpers (`git_init`, `write_ignored_marker`, `assert_hidden_from_git`); five new tests; two obsolete `ChangedFiles` tests removed |

| Commit | Message |
| --- | --- |
| `8feeee0` | run the check where the operator stands |
| `b21261f` | pin the gitignored-artifact fixture on both entry points |

## The decision, and what it cost

**Option (a).** The check now runs in the project root — the same directory `proposal.rs`,
`triage_agent.rs`, and `loop_cmd.rs` already use, and the directory the operator was standing in
when they did the ask. The snapshot is deleted, not repaired.

The reasoning is in `design.md`; the short form is that a snapshot is by construction *not* the
durable reality the gate is supposed to be about, so every difference between the copy and the
project is a candidate false gate and `--exclude-standard` was only the first one anyone tripped
over. Options (b) and (c) both fail on the ticket's own criterion 1, whose fixture declares no
execution context and so must work by default; and (b) additionally cannot serve the field case,
because a Node check needs `node_modules/` to resolve its own imports and `node_modules/` is
precisely what makes a copy unusable.

**What was traded, plainly: isolation.** A check that writes a relative path used to write into a
disposable copy. It now writes into the project, and Lisa neither prevents that nor notices it.

**A reviewer should look hardest at this next part**, because it goes one step beyond what the
ticket asked and removes an existing behaviour. The conservative move — keep `.current_dir(root)`
but fingerprint the *live* tree before and after, preserving `ChangedFiles` — was considered and
rejected as unshippable. `run_world_rechecks` is fired asynchronously by the plugin scheduler
(`lisa-plugin/src/lib.rs:1761-1783`) while other threads' agent sessions are editing the same
files, and `lisa unblock` is run by an operator with a loop live. A live-tree fingerprint cannot
tell this check's writes from a concurrent agent's, so it would report another writer's changes as
*the check tried to change project files* — structurally the same error T-056-01-01 just removed
from the message layer, and firing most often unattended under the loop. There is no portable way
to attribute writes to one process tree (overlayfs is Linux-only, `sandbox-exec` macOS-only and
deprecated, and chmod'ing the real project would freeze every concurrent thread). So the honest
choices were "detect dishonestly" or "do not detect", and this takes the second.

The read-only requirement itself is unchanged — it lives in the check contract. Putting teeth on
it at authoring time is T-056-01-03's explicit criterion ("Decide, and state, whether a check may
write… so `build && verify` is rejected by the author rather than by the operator").

## Criteria, and the evidence for each

| # | Criterion | Evidence |
| --- | --- | --- |
| 1 | `out/marker` fixture reopens, exit 0; fails before the fix | `unblock_sees_a_gitignored_build_output_the_operator_can_see`. Run against the pre-change `unblock.rs`: `FAILED`, panicking with `That didn't work yet — the check ran and did not pass. / ran in: /var/folders/…/T/.tmpFJZqEU / exit code: 1`. After: passes. Both quoted in `progress.md` |
| 2 | One run sees a gitignored output and a tracked source | `a_check_reads_a_gitignored_artifact_and_a_tracked_file_in_one_run` — `test -f out/marker && test -f README.md && test -f docs/active/tickets/T-BOTH.md`, with `README.md` genuinely `git add`-ed. Unit half: `a_check_reads_the_project_it_runs_in` |
| 3 | Option and hermeticity tradeoff recorded in the design artifact | `design.md` — the decision, what is traded, what is kept, the three rejected options with their reasons, and the replacement for the "declared rule" clause |
| 4 | Reported cwd == observed cwd | `the_check_runs_where_lisa_says_it_ran` compares the `ran in:` line with the check's own `pwd -P` **and** asserts it equals the project root. Units: `the_reported_directory_is_the_one_the_check_observed` (kept from T-056-01-01), `the_check_runs_in_the_project_root` (new) |
| 5 | `run_world_rechecks` goes through the same path, both entry points covered | `world_recheck_sees_the_same_tree_an_operator_unblock_does` drives the same `out/marker` fixture through `lisa recheck-world`; paired with the criterion-1 test on the operator side. They share `run_check` in code, and this asserts it from outside |
| 6 | Non-git projects still work; the two paths agree | `a_non_git_project_and_a_git_project_agree_about_what_a_check_sees` — same fixture built with and without `git init`, asserting **identical** exit code, stdout, and stderr. See the deviation below |
| 7 | `just check` green | `just check; echo exit=$?` → `exit=0`. Judged by exit code |

## The one criterion met in substance rather than in letter

Criterion 6 reads: "the `copy_small_tree` fallback path keeps a passing test, and the fixture above
is repeated in a non-git project to prove the two paths agree."

`copy_small_tree` no longer exists. It was one arm of a tree copy that no longer happens, and
keeping a tree-copying function with no caller purely so a test could name it would be dead code.
The second half is done exactly as written, and the first half's substance — non-git projects
still work — is asserted more strictly than a test of the fallback could: there is now one code
path, and the test pins the two project kinds to byte-identical output.

Worth noting what this collapsed: the two arms already **disagreed**. The git arm dropped ignored
paths; the non-git arm (`should_skip` knew nothing of `.gitignore`) copied them. The same fixture
would unblock in a plain directory and decline in a repository. That is also why the field bug had
no regression test — `project()` in `parked_ux.rs` never ran `git init`, so all 20 existing
black-box checks took the arm the field failure did not.

The criterion is phrased in the vocabulary of options (b)/(c). The ticket offered option (a), and
its own criterion 3 is conditioned on "*if* the snapshot is kept", so choosing (a) is not a way of
failing it. Recorded here rather than smoothed over.

## Test coverage

`crates/lisa-cli/src/unblock.rs`: 9 unit tests (was 9 — two removed, two added).
`crates/lisa-cli/tests/parked_ux.rs`: 23 black-box tests (was 20 — two removed, five added).
Workspace: `just check` exit 0.

**Removed, and why:**

- `relative_write_never_reaches_live_project_and_cannot_pass` (unit) — asserted the isolation
  property being traded. Its removal is the visible face of that trade.
- `mutation_inside_disposable_state_is_detected_even_after_chmod` (unit),
  `automatic_recheck_write_attempt_is_disposable_and_cannot_reopen` and
  `attempted_write_is_disposable_reported_plainly_and_does_not_reopen` (black box) — all assert
  the `ChangedFiles` decline, which no longer exists.

The automation side keeps its non-pass coverage through
`automatic_recheck_ignores_operator_owned_passing_checks` and
`automatic_recheck_timeout_is_bounded_and_cannot_reopen`, so "automation acts only on a pass"
is still pinned.

**Gaps a reviewer should know about:**

1. **No test asserts that a writing check now writes.** Deliberate: a test that lets a check mutate
   its own fixture would pin a behaviour T-056-01-03 is chartered to change. The behaviour is
   stated here and in the `run_check` doc comment instead.
2. **No test covers a check under real concurrency.** The concurrency argument above is the reason
   `ChangedFiles` was removed rather than moved, and it rests on reading the scheduler
   (`lib.rs:1761-1783`), not on a reproduction. Writing that race as a test would be valuable and
   is not in this ticket.
3. **The new fixtures require `git` on PATH.** `git_init` asserts each step, so an absent `git`
   fails the test loudly rather than silently falling back to the non-git shape — which is the
   failure mode that hid this bug for a year.

## Open concerns

1. **A check can now write to the project, and nothing stops or reports it.** The largest
   consequence of this ticket. Mitigations: the contract forbids it; a check is authored by the
   same reviewer agent that already has unrestricted write access to the repository, so no new
   capability is created; and record-time validation lands in T-056-01-03. Until then a careless
   check is a real hazard, and it is worth reading T-056-01-03's first criterion with that in
   mind.
2. **`CheckOverrideOutcome::ChangedFiles` is now unconstructed** but retained in
   `lisa-core/src/provenance.rs`, because ledgers written before this change contain it and must
   keep parsing. `SCHEMA_VERSION` stays at 10. A comment on `override_outcome` says so.
3. **The contract is documented in code, not yet where reviewers read.**
   `docs/knowledge/rdspi-workflow.md:59` still says only "a read-only verification command". The
   paragraph stating the full execution contract — directory, which files are present, whether
   writes are allowed, the time budget — is an explicit acceptance criterion of T-056-01-03, and
   writing a partial version now would collide with it on the same lines. Until that lands, a
   reviewer still has no operator-facing statement of where a check runs. This is the one loose
   end I would most want a human to confirm is acceptable to defer.
4. **The field case is still not unblockable.** With this ticket, `node scripts/check-touch.mjs`
   finds `dist/` — but the check is a ~20-minute sweep against a hardcoded 5-second budget, so it
   would now decline with `it took longer than 5 seconds`. That is exactly what the story
   predicted and what T-056-01-03 exists to fix; it is not a defect in this work, but nobody
   should read "the root cause is fixed" as "T-010-03 would unblock today".
5. **Checks now see `.lisa/` and `.git/`.** They already ran as the operator's user with the whole
   filesystem reachable by absolute path, so this is not a new capability — but it is a change in
   what a careless relative glob can reach.

## Not in this ticket

The 5-second budget, the writability decision, record-time validation of checks
(`lisa check-disposition` trying the check), `run_world_rechecks`'s silence on repeated
non-passes, and the `rdspi-workflow.md` execution-contract paragraph. All are T-056-01-03
acceptance criteria.

## Environment

`wasm32-wasip1` was already installed, so `just check`'s WASM leg and the `client_autodetect`
tests that failed for T-056-01-01 on a missing target passed here without intervention. No
repository file changed for the environment.
