# Review — T-045-01-02 claim command surface

## Disposition

Pass.

The ticket's command boundary is implemented, committed, and covered by black-box
tests through the built `lisa` executable.
A claim for the current attempt and exact nonce under the pane's held lease marker
succeeds and atomically publishes typed evidence.
A prior-attempt claim is rejected with `stale-attempt` even when its old assignment
file still exists.
A mismatched nonce under the current attempt is rejected with `wrong-nonce`.

No critical issue blocks completion.

## What changed

The work is split across two isolated commits:

1. `69b3f9b8392ecd42ac6e9a3e9156d9da5b5017c8`
   `feat(core): define assignment claim identity`
2. `d02d93b2a25c82a5af4bef08db80bc2c63c32594`
   `feat(cli): add lease-bound claim command`

Both commits were created with `lisa commit-ticket` and exact repository-relative
include paths.
No ordinary Git staging or commit command was used.

## Shared claim contract

Created `crates/lisa-core/src/claim.rs`.

`AssignmentClaim` is the provider-neutral payload containing:

- ticket ID;
- attempt ID;
- assignment nonce.

It serializes as compact JSON and retains a full `u128` nonce.
The later plugin consumer can deserialize the same type without copying a wire schema
into the WASM crate.

`ClaimRejection` gives semantic failures stable names independent from explanatory
prose.
The command surface currently names:

- `pane-unavailable`;
- `lease-unavailable`;
- `invalid-lease`;
- `wrong-ticket`;
- `stale-attempt`;
- `attempt-mismatch`;
- `wrong-nonce`;
- `lease-changed`.

This gives tests, agents, and future scheduler diagnostics a value they can match
without parsing a human sentence.

The same module owns `assignment_file_name(attempt_id, nonce)`.
The shared result is exactly:

`assignment-{attempt_id}-{nonce}.md`

Modified `crates/lisa-core/src/lib.rs` to export the claim module.

Modified `crates/lisa-plugin/src/assignment.rs` to use the core filename helper.
Its private duplicate was removed.
The atomic writer and `AssignmentRef` behavior otherwise remain unchanged.
This alignment ensures the assignment producer and claim validator cannot silently
disagree on punctuation or numeric formatting.

## Command surface

Created `crates/lisa-cli/src/claim.rs`.

Registered the hidden plumbing command in `crates/lisa-cli/src/main.rs`:

```text
lisa claim \
  --path <project-root> \
  --ticket-id <ticket-id> \
  --attempt-id <u64> \
  --nonce <u128>
```

`--path` defaults to the current directory.
The command obtains pane routing from `LISA_PANE_ID`, matching Lisa's native launch
contract.
It deliberately has no `--pane-id` override.

The command loads the E-034 lease transport marker from:

`.lisa/signals/pane-{pane-id}.lease`

It requires a regular file containing valid `AttemptLease` JSON.
It compares the complete ticket and attempt identity before looking at the nonce.
This ordering is material: a predecessor can leave an immutable assignment behind,
but its claim must still be named stale rather than accepted from file residue.

The command then derives and requires the exact regular assignment file:

`.lisa/attempts/{ticket}/{attempt}/work/assignment-{attempt}-{nonce}.md`

There is no directory scan, glob, newest-file heuristic, or reconstruction from an
untrusted directory order.
An arbitrary nonce whose exact file is absent returns `wrong-nonce`.

The pane marker is read again after assignment validation.
If it is missing, malformed, or differs from the first complete lease, the command
returns `lease-changed` and publishes no claim.

## Claim publication

The accepted payload is serialized from `AssignmentClaim`.
The command writes the complete JSON to a hidden sibling temporary containing:

- pane ID;
- process ID;
- wall-clock nanosecond nonce.

It renames that temporary to:

`.lisa/signals/pane-{pane-id}.claim`

The durable claim therefore becomes visible only after the complete JSON write.
A rename failure removes the temporary and returns an operational error.
The scheduler cannot observe a torn claim through the final filename.

Success prints one concise line with the accepted ticket, attempt, and nonce.
Semantic rejection uses the stable form:

`Error: claim rejected [reason-name]: description`

and exits nonzero without creating a claim signal.

## Command help

Modified `crates/lisa-cli/tests/help_surface.rs` to pin the new surface.

The suite now recognizes 13 Lisa commands and 5 plumbing commands.
`claim` resolves directly and appears in the curated plumbing footer.
It remains absent from the operator command listing.
The existing operator help snapshots and jargon checks remain intact.

## Acceptance coverage

Created `crates/lisa-cli/tests/claim_cli.rs` with three black-box tests.
Every case launches `env!("CARGO_BIN_EXE_lisa")`; none bypasses Clap or calls the
validation module directly.

### Current attempt and nonce

The fixture writes:

- `pane-7.lease` for ticket `T-CLAIM-01`, attempt 2;
- the exact attempt-2 assignment for nonce 100.

It invokes the same identity and asserts:

- exit status 0;
- exact success stdout;
- empty stderr;
- a durable `pane-7.claim` exists;
- the signal deserializes to the exact expected `AssignmentClaim`;
- no claim temporary residue remains.

### Prior attempt

The fixture writes a current attempt-2 marker and real assignment files for both
attempts 1 and 2.
It invokes attempt 1 with that predecessor's actual nonce.

The test asserts:

- nonzero exit;
- empty stdout;
- stderr contains `[stale-attempt]`;
- no claim signal exists.

The predecessor file's presence proves rejection is lease-driven rather than an
accidental missing-file result.

### Wrong nonce

The fixture writes a matching attempt-2 marker and assignment nonce 100.
It invokes attempt 2 with nonce 101.

The test asserts:

- nonzero exit;
- empty stdout;
- stderr contains `[wrong-nonce]`;
- no claim signal exists.

Together these cases directly cover the ticket acceptance criterion.

## Core and predecessor coverage

The shared core module has three unit tests:

- exact filename formatting, including maximum numeric values;
- exhaustive stable rejection-name mapping;
- JSON round-trip with hostile ticket characters and a nonce above `u64::MAX`.

The two T-045-01-01 assignment tests remain green after switching to the shared
filename helper:

- complete ticket/attempt/nonce write and read-back;
- interrupted partial temporary never becoming the durable assignment.

This verifies that the command contract did not regress the atomic writer it depends
on.

## Verification performed

Focused verification passed:

```text
cargo fmt --all -- --check
cargo test -p lisa-core claim::tests
cargo test -p lisa-plugin assignment::tests
cargo test -p lisa-cli --test claim_cli
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli
```

Observed focused results:

- 3 claim core tests passed;
- 2 assignment writer tests passed;
- 3 black-box claim tests passed;
- 5 help-surface tests passed;
- the complete CLI package passed;
- no ticket-attributable compiler warning remained.

Workspace verification passed on a consistent shared-worktree rerun:

```text
cargo test --workspace
```

The passing run included:

- 19 CLI library tests;
- 269 CLI binary tests;
- all CLI integration suites;
- 200 core tests;
- 2 core integration tests;
- 387 plugin tests;
- doc tests.

The existing real-Zellij integration remained intentionally ignored by its declared
environment gate.

Production-target verification passed:

```text
just check
```

This completed `cargo check -p lisa-plugin --target wasm32-wasip1` and reran the
workspace tests successfully.

## Shared-worktree race observed

The first workspace invocation raced with concurrent T-045-02-01.
That ticket's untracked launcher integration test appeared after Cargo compiled the
old binary, so the newly discovered test temporarily saw no `launch-codex` command.

Inspection confirmed the failure belonged to an in-flight foreign unit:

- this ticket's HEAD and commits were unchanged;
- the neighboring module/test and additive `main.rs` diff were not in this ticket's
  transactions;
- no claim file changed;
- a stable rerun passed both the neighboring test and the full workspace.

This is documented as concurrency evidence, not a product defect in the claim work.

## Source ownership audit

All ticket-owned additions are committed.
Both commits pass `git show --check`.
The ordinary index is empty.

The following paths are clean relative to HEAD:

- `crates/lisa-core/src/claim.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-cli/src/claim.rs`;
- `crates/lisa-cli/tests/claim_cli.rs`;
- `crates/lisa-cli/tests/help_surface.rs`.

`crates/lisa-cli/src/main.rs` has a current foreign additive worktree diff from
T-045-02-01 for `launch-codex`.
This ticket's claim command changes in that shared file are already committed in
`d02d93b`.
The foreign diff was preserved and excluded from both isolated transactions.

Unrelated runtime ledgers, epic/story/ticket materialization, and neighboring ticket
artifacts were not included.

## Open concerns and deliberate limits

The pane `.lease` marker is durable E-034 identity transport, not the scheduler's
final authority registry.
Revocation removes `State::current_leases` but does not necessarily delete the marker
in this ticket.
Therefore T-045-03-01 must revalidate every claim signal against the current in-memory
lease before treating it as ownership proof.

Repeated assignment preparation within one attempt can leave an older same-attempt
nonce-bearing file.
The CLI can prove that a requested file was published, but it cannot inspect the
plugin's retained live `assignment_refs` entry.
The scheduler consumer must compare the claim nonce with that retained reference.
This is the explicit producer/consumer boundary chosen by S-045-01, not an omitted
check that this standalone process can authoritatively perform.

The command is not yet injected into assignment text and does not promote a seat to
Owned.
Launcher integration belongs to S-045-02; claim consumption and ownership transition
belong to T-045-03-01.

Nonce revocation, late-claim rejection after completion, clean TUI exit, timeout
policy, dashboard state, and live Codex/Zellij proof remain assigned to later E-045
tickets.

No live provider or Zellij test was required or run for this fixture-proven story.

## Final assessment

The implementation satisfies the command-surface acceptance criterion while
preserving E-034 lease semantics and the assignment writer's atomicity.
It creates a typed, atomic evidence boundary with stable rejection reasons and a
clear handoff to the dependent scheduler-admission ticket.
The work is ready for Lisa's completion publication.
