# Review: restart reconstruction and lost-result fixtures

## Review outcome

The ticket is ready to complete.

Two focused real-adapter fixtures now exercise restart reconstruction and
lost-result/duplicate-Stop convergence.

Both run in the predecessor's real temporary Git repository.

Both use the Lisa project at `games/midsummer` below the Git root.

Both call the real plugin completion adapter.

Both call the real CLI isolated completion transaction.

Both rediscover the original completion commit instead of creating another.

Both publish exactly one authoritative Done record.

Both show CommandInFlight retaining its bound and ending Confirmed.

No production behavior changed.

No critical issue remains open.

## Source commit

Commit:

`33d339a878a38d9e6fd4014e02a5cd9dcb042e67`.

Message:

`test(plugin): add restart replay fixtures`.

The commit was created with Lisa's isolated `commit-ticket` transaction.

The command used exact ticket ownership.

Included path:

`crates/lisa-plugin/src/tests/hostile_order_regression.rs`.

`git diff-tree` confirms that is the only path in the commit.

No ordinary Git staging command was used.

No ordinary Git commit command was used.

The ordinary index is empty.

The ticket-owned source path is clean.

## File changed

### `crates/lisa-plugin/src/tests/hostile_order_regression.rs`

Added 259 test-only lines.

Added one private reusable fixture type.

Added two focused native tests.

No existing assertion was removed.

No predecessor test was weakened.

No production symbol was made public for testing.

No dependency was added.

No static fixture file was added.

## Fixture architecture

`LostResultFixture` builds on the predecessor's `Scenario`.

That preserves one nested-repository topology and one command contract.

The fixture captures the original pending completion.

It captures the original typed launch effect.

It captures the first real completion commit ID.

Its constructor advances a passing Review through the real adapter.

The adapter writes Requested and CommandInFlight.

The constructor then executes the real CLI transaction.

It deliberately withholds the command result from the adapter.

The repository is therefore durably Done while the journal remains in-flight.

No authoritative provenance exists at that point.

This is the exact lost-result boundary required by the ticket.

## Real command contract

The fixture uses the existing `transaction_request` helper.

That helper calls `State::build_completion_command`.

It checks `--path` is the temporary Git root.

It checks the ticket path is:

`games/midsummer/docs/active/tickets/T-ARCADE-PRIMARY.md`.

It checks the work path is:

`games/midsummer/docs/active/work/T-ARCADE-PRIMARY`.

It checks completion command context identifies the primary ticket.

The returned argv is decoded into the real `CompleteTicketRequest`.

The tests therefore do not bypass adapter path construction.

## Restart reconstruction coverage

Test:

`plugin_restart_reconstruction_fixture_converges_on_single_prior_commit`.

The test constructs a fresh `State` from durable paths.

It calls production journal restoration.

It calls production DAG rebuilding and masking.

It verifies restoration is healthy.

It verifies no live pending map survives process restart.

It verifies the reconstructed generation equals the original generation.

It verifies state is exact CommandInFlight.

It verifies correlation equals the original correlation.

It verifies deadline equals the original absolute deadline.

It verifies the DAG observes the prior Review phase/status.

The durable ticket bytes are already Done at that point.

This establishes that the journal, not process memory, controls reconstruction.

## Bounded replay coverage

The fixture derives replay time from the stored deadline minus one millisecond.

No sleep or scheduler timing assumption is involved.

The real adapter receives typed Reconcile input at that explicit time.

It launches exactly one replay effect.

The replay effect equals the original effect.

The replay pending entry retains the original generation.

It retains the original correlation.

It retains the original deadline.

It is marked as a reconciliation replay.

No additional Requested record is appended.

No additional CommandInFlight record is appended.

The retained deadline proves CommandInFlight remains bounded across restart.

## Prior-commit convergence

The fixture calls `complete_ticket` again with the original generation.

The CLI returns the first completion commit ID.

It returns no committed paths.

The Git commit count remains baseline plus one.

The completion commit's parent remains the fixture baseline.

There is no second completion commit.

The adapter then receives the correlated successful replay result.

It verifies durable Done bytes.

It appends Confirmed.

The aggregate stores the first commit ID.

The first fixture performs another fresh restart after confirmation.

That restart reconstructs Confirmed with the same commit ID.

CommandInFlight therefore ends in the named successful terminal state.

## Duplicate-Stop coverage

Test:

`lost_result_duplicate_stop_fixture_converges_on_single_prior_commit`.

The fresh adapter receives two Stop observations before replay.

Neither creates pending state.

Neither launches an effect.

Neither changes journal bytes.

After one replay is pending, it receives two more Stop observations.

It also receives a repeated Reconcile observation.

Those duplicates do not launch another effect.

Those duplicates do not append journal evidence.

The replay then converges on the first commit.

The test delivers the same successful result again after confirmation.

That duplicate result changes neither journal nor ledger bytes.

The Git commit count remains baseline plus one throughout.

## Exactly-one authority evidence

Each fixture ends with three journal records.

There is exactly one Requested record.

There is exactly one CommandInFlight record.

There is exactly one Confirmed record.

The final aggregate is Confirmed.

Its confirmed commit ID is the real prior commit.

The provenance ledger contains exactly one record.

It is an Execution record for the primary ticket.

Its outcome is Done.

It is authoritative.

No provenance exists before the correlated result is accepted.

No duplicate result can add a second authoritative row.

## Acceptance criterion assessment

Restart reconstruction replays through the real adapter.

Satisfied by the first focused fixture.

Lost-result/duplicate-Stop replays through the real adapter.

Satisfied by the second focused fixture.

Each converges on the single prior completion commit.

Satisfied by commit ID equality, empty committed paths, and stable count.

Each yields one authoritative Done.

Satisfied by typed provenance decoding and exact ledger count.

No duplicate commit exists.

Satisfied by baseline-plus-one count after every replay and duplicate.

CommandInFlight ends in its named bounded state.

Satisfied by exact retained deadline followed by durable Confirmed state.

The unsuccessful deadline boundary remains covered by the existing
`reconciliation_deadline_ends_action_required_without_infinite_replay` test.

## Verification results

Focused fixture filter:

`cargo test -p lisa-plugin --lib fixture --no-fail-fast`.

Passed: 6; failed: 0.

The filter includes both new fixtures and four existing Codex fixture tests.

Hostile-order module:

`cargo test -p lisa-plugin --lib hostile_order_regression --no-fail-fast`.

Passed: 4; failed: 0.

Plugin library:

`cargo test -p lisa-plugin --lib --no-fail-fast`.

Passed: 375; failed: 0.

Workspace:

`cargo test --workspace --no-fail-fast`.

Passed across all workspace targets and doctests.

Project check:

`just check`.

The WASM target check passed.

The workspace suite run by the recipe passed.

Formatting and hygiene:

- `cargo fmt --all -- --check` passed;
- `git diff --check` passed;
- ordinary Git index is empty;
- ticket-owned source is clean.

## Coverage assessment

The tests are deterministic and native.

They use real filesystem and real Git repositories.

They use real completion transaction identity and idempotent discovery.

They use private adapter methods through the established native test seam.

They do not start a live Zellij server.

They do not invoke a live Codex provider seat.

Those boundaries are appropriate for this deterministic fixture ticket.

The dependent `T-042-04-03` owns the freshly built live-seat field run.

## Open concerns and limitations

No blocking concern exists.

The focused cases intentionally overlap a subset of the broad hostile-order
passing test and an older root-level inline lost-result regression.

That overlap provides independently selectable acceptance evidence.

Removing older regression duplication is outside this ticket.

The tests reconstruct current attempt authority deterministically because a
fresh generic plugin load has no provider pane history of its own.

The durable journal, mask, reconciliation, command, result, provenance, and Git
boundaries are production implementations.

The Zellij host call itself is a native no-op, consistent with the established
plugin adapter test seam.

## Repository preservation

Lisa-managed ticket and provenance changes were not included in source.

Lisa-managed shared phase artifact publication was not included in source.

The unrelated untracked `crates/lisa-plugin/docs/` tree was preserved.

All authored RDSPI artifacts were written to the private attempt directory.

Only the exact ticket-owned test path entered the isolated source commit.

## Final disposition

Pass.

The acceptance criterion is satisfied, verification is green, source is
committed and clean, and no critical issue requires human attention.

This attempt remains on `T-042-04-02` for Lisa to admit Review, publish the
completion transaction, and release the seat before the live field ticket.
