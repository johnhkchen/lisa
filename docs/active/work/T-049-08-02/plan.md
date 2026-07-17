# Plan: proposal apply records first

## Step 1: extend proposal evidence vocabulary

- Add Attempted and Failed proposal action variants.
- Add defaulted step evidence fields to ProposalActionRecord.
- Update existing Proposed record fixtures with neutral evidence values.
- Add a record round-trip test that pins field names and action strings.
- Verify with the focused lisa-core provenance test.

## Step 2: add the terminal failed sidecar state

- Add Failed to ProposalState.
- Preserve Pending, Applied, and Dismissed spellings.
- Extend stored proposal write/read coverage to Failed.
- Verify with focused lisa-core triage tests.

## Step 3: make parked proposal projection lease-aware

- Add latest current Park lease extraction in parking.rs.
- Read mixed ledger lines observationally and ignore malformed unrelated rows.
- Treat the latest Park/Unpark transition by line order as current evidence.
- Add provenance ledger path to collect_parked_remedies.
- Require exact Park/source lease equality for optional proposal projection.
- Leave the rest of each ParkedRemedy unchanged.
- Update parking unit tests to write a matching Park.
- Add mismatched-lease and latest-Unpark checks.
- Verify with `cargo test -p lisa-core parking`.

## Step 4: update first-party projection callers

- Pass root `.lisa/provenance.jsonl` from CLI status.
- Pass the same path from single unblock and world recheck.
- Pass State.ledger_path from plugin world checks, triage, and dashboard.
- Update test-only plugin collection calls.
- Run `cargo check --workspace` to catch every signature call site.

## Step 5: refactor apply execution around explicit progress

- Add a writer-taking internal proposal action entrypoint.
- Keep the public entrypoint signature stable for main.rs.
- Add a step-failure structure with applied prefix, failed description, and reason.
- Replace validate-all/apply-all with one ordered executor.
- Announce each description before executing its step.
- Recheck file-edit exact match immediately before replace.
- Preserve atomic file publication.
- Unit-test the executor result independently where useful.

## Step 6: write apply attempt before mutation

- After all ticket/disposition/sidecar/Park authorization checks, construct Attempted.
- Include operator actor, full proposal, and step count.
- Append it to `.lisa/provenance.jsonl`.
- Stop without mutation if the append fails.
- Begin the executor only after append success.

## Step 7: persist clean and failed outcomes

- On executor success, set sidecar Applied.
- Append Applied outcome with all applied descriptions.
- Reopen the ticket only after terminal publications succeed.
- On executor failure, set sidecar Failed.
- Append Failed outcome with applied prefix, failed description, and reason.
- Keep the ticket blocked.
- Attempt both sidecar and outcome publications even if one fails.
- Return a diagnostic error after failure evidence is handled.

## Step 8: strengthen CLI proposal tests

- Extend setup with a matching Park row.
- Make the clean apply fixture contain at least two steps.
- Capture writer output and pin exact announcement strings.
- Parse the mixed ledger and assert Attempted precedes Applied.
- Assert Attempted includes proposal and step count.
- Assert Applied names every step in order.
- Add a mid-list command failure after a successful file edit or command.
- Assert the first mutation landed.
- Assert Attempted and Failed rows remain.
- Assert Failed names the landed prefix and failing step.
- Assert sidecar state equals Failed rather than Pending.
- Assert ticket status remains blocked.
- Add a stale Park lease case and assert no step runs/no action row is added.
- Preserve dismiss coverage with matching lease.

## Step 9: strengthen parking and triage regressions

- In core parking, test mismatched pending proposal returns `proposal: None`.
- Restore matching source lease and assert the original proposal returns unchanged.
- Keep Dismissed suppression assertion.
- In plugin triage, create a stale Pending sidecar for a prior attempt.
- Append/use a newer Park row as the current park.
- Assert request_operator_triage launches fresh triage rather than suppressing it.
- Assert a matching Pending sidecar still suppresses duplicate triage.

## Step 10: format and focused verification

- Run cargo fmt on the workspace.
- Run lisa-core tests.
- Run lisa-cli proposal/status/unblock related tests.
- Run relevant lisa-plugin triage tests.
- Inspect failures for fixture weakening rather than changing assertions opportunistically.
- Update progress.md with results and any plan deviations.

## Step 11: commit meaningful source units

- Inspect exact diffs and ordinary index state before each commit.
- Commit core files with one `lisa commit-ticket` invocation and exact includes.
- Commit CLI files with one exact include set.
- Commit plugin changes separately with its exact path.
- Never run ordinary git add or git commit.
- After commits, confirm no ticket-owned source file remains modified/untracked/staged.

## Step 12: full regression verification

- Run `cargo test --workspace`.
- Run `just check` if it exercises additional WASM checking beyond workspace tests.
- If target/toolchain availability blocks an ancillary check, record the exact limitation.
- Inspect git diff/status to ensure Lisa-managed pre-existing changes remain untouched.
- Inspect committed diff for accidental unrelated changes.

## Step 13: review

- Summarize source files and behavior changes in review.md.
- Map each acceptance criterion to specific tests and commands.
- Describe additive ledger compatibility and failure-path limitations.
- Identify any unresolved publication atomicity concern honestly.
- Write exact pass disposition only when all ticket work is committed and verified.
- Run `lisa check-disposition T-049-08-02`.
- Correct every reported issue.
- Remain on this ticket after Review.

## Verification criteria

- The first apply ledger row exists before any prepared step can execute.
- A clean run has Attempted then Applied records.
- A failed run has Attempted then Failed records.
- Failed evidence names the exact successful prefix and first failed step.
- Every executed/attempted step has one exact pre-execution announcement.
- File edits invalidated by earlier commands fail rather than silently no-op.
- Sidecar is Applied or Failed after execution, never Pending.
- Failed apply does not reopen the ticket.
- Matching proposal/Park leases render and suppress triage as before.
- Mismatched proposal/Park leases do not render and do not suppress triage.
- Existing S-049-07 proposal fixtures retain their behavioral assertions.
- All workspace tests pass.
- All ticket-owned source changes are committed through Lisa's isolated transaction.
