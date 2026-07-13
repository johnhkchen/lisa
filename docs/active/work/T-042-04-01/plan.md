# Plan: hostile-order real-adapter regression

## 1. Register the focused module

Add the new test module declaration inside `crates/lisa-plugin/src/lib.rs`.

Keep the declaration adjacent to existing focused native modules.

Verification: Rust discovers the new file without production changes.

## 2. Build the nested Git fixture

Create `hostile_order_regression.rs`.

Initialize a temporary Git repository with test identity.

Create `games/midsummer/docs/active/tickets` and work directories.

Write the Implement primary ticket and Ready dependent ticket.

Commit the baseline fixture.

Verification: baseline HEAD and commit count are stable.

## 3. Build the real adapter state

Scan nested tickets into the real DAG.

Configure `/host/docs/...` plugin paths.

Configure nested project root, Git root, attempt root, journal, and ledger.

Install the Running Implement thread and current attempt lease.

Install a WaitingForStop completing seat and a free dependent seat.

Enable permissions and discovered slots.

Verification: the primary is assigned and the dependent is blocked.

## 4. Add exact transaction helpers

Read option values from adapter-built argv.

Assert `--path` is the Git root.

Assert ticket/work arguments include `games/midsummer/docs/...`.

Construct `CompleteTicketRequest` from the real argv and completion key.

Verification: no hard-coded root-level transaction path is accepted.

## 5. Add private Review evidence helper

Write `review.md` and the selected disposition in the attempt directory.

Write them before calling phase advancement.

Verification: canonical work does not need pre-existing evidence.

## 6. Drive passing artifact order

Start the primary thread at Implement.

Call `check_artifact_advances` after evidence exists.

Assert thread and ticket reach Review.

Assert one launch effect, one pending key, and journal intent/in-flight state.

Verification: no Done bytes exist before the CLI transaction.

## 7. Drive transition Stop and timeout

Call real Stop handling while the seat is WaitingForStop.

Assert the seat moves toward clear and no completion is duplicated.

Age Review clocks and call Review timeout checking.

Assert no FinishUpPromptSent event or marker.

Verification: existing Review evidence suppresses false follow-up.

## 8. Drive attempted operator Done

Submit `d` then Enter through the real key handler.

Assert the modal reports AlreadyPending with stable correlation.

Assert no new effect or generation is created.

Verification: operator recovery cannot duplicate actionable completion.

## 9. Execute and delay the real result

Build the command from the pending completion key.

Assert its nested contract.

Call the real `complete_ticket` transaction.

Withhold the returned successful result from the initial state.

Assert exactly one Git commit and durable Done frontmatter.

Verification: journal remains unresolved and provenance remains absent.

## 10. Reload and replay

Construct a fresh State with the same durable paths.

Restore the completion journal.

Reinstall the exact current attempt, thread, and slot records.

Rebuild the DAG and assert raw Done remains masked as Review.

Submit duplicate Stop before reconciliation.

Reconcile before the stored deadline.

Assert one replay pending with the original key.

Submit further Stop and Reconcile duplicates.

Assert no additional launch or journal transition.

Verification: the same durable generation remains authoritative.

## 11. Converge the idempotent transaction

Call `complete_ticket` again with the exact same request.

Assert it returns the original commit and no committed paths.

Deliver its result to the restarted adapter.

Deliver the same result again.

Verification: exactly one Confirmed transition and Done provenance row exist.

## 12. Assert passing scheduler effects

Assert baseline plus one completion commit.

Assert one authoritative Done ledger record.

Assert the primary thread and lease are gone.

Assert its original seat is released.

Assert the dependent is scheduled onto an eligible seat.

Assert no finish-up prompt was emitted at any point.

Verification: every positive acceptance clause is explicit.

## 13. Drive the blocked sequence

Create a fresh identical fixture with Block disposition.

Advance Implement to Review.

Drive Stop during transition, reconciliation, timeout, and `d` then Enter.

Assert zero launch effects and zero journal completion transitions.

Assert no completion commit and no Done provenance.

Assert the primary stays assigned and the dependent stays unscheduled.

Assert the actionable block reason remains visible.

Assert no finish-up prompt appears because Review exists.

Verification: every negative acceptance clause is explicit.

## 14. Focused verification

Format ticket-owned Rust source.

Run the two hostile-order tests by module filter.

Run the nested transaction and operator-related filters if failures implicate
their shared seams.

Verification: the new module passes deterministically without sleeps.

## 15. Repository verification

Run `cargo test -p lisa-plugin --lib --no-fail-fast`.

Run `cargo test --workspace --no-fail-fast` or `just check`.

Run `cargo fmt --all -- --check`.

Run `git diff --check`.

Verification: no regression or formatting failure remains.

## 16. Track progress

Write `progress.md` in the private attempt directory.

Record completed steps, exact commands, test counts, and deviations.

Record preserved unrelated worktree entries.

Verification: progress matches the actual diff and repository state.

## 17. Commit the source unit

Run `lisa commit-ticket --ticket-id T-042-04-01`.

Use a test-focused message.

Include exactly `crates/lisa-plugin/src/lib.rs` and
`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

Do not use the ordinary Git index.

Verification: both ticket-owned source paths are clean afterward.

## 18. Self-review and disposition

Inspect the committed diff and commit path list.

Confirm no production behavior changed.

Confirm every ticket acceptance phrase maps to an assertion.

Write `review.md` and valid `review-disposition.json` in the attempt directory.

Use Block if a required composed behavior exposes a production defect.

Remain on the current ticket after Review.

## Atomicity and deviation policy

The module registration and focused module are one atomic test source unit.

If existing private helpers are insufficient, add test-private helpers only.

If the composed test exposes a production defect, document it before any
change; the story says new production fixes are out of scope and blocking.

Never absorb unrelated dirty paths into the ticket transaction.

