# Research: restart reconstruction and lost-result fixtures

## Ticket boundary

Ticket `T-042-04-02` belongs to story `S-042-04`.

The story is the regression and field-evidence layer for durable completion.

This ticket follows `T-042-04-01` and precedes the live-seat field run.

Its acceptance criterion asks for two focused fixture shapes:

- plugin restart reconstruction;
- lost command result with duplicate Stop observations.

Both must exercise the real plugin adapter.

Both must rediscover a prior completion commit instead of creating another.

Both must publish exactly one authoritative Done result.

The durable CommandInFlight aggregate must terminate in a named bounded state.

The story explicitly excludes a new production completion contract.

Any production defect exposed by these regressions is therefore blocking work.

## Repository topology

The workspace has separate `lisa-core`, `lisa-plugin`, and `lisa-cli` crates.

`lisa-core` owns typed completion state and reconciliation decisions.

`lisa-plugin` adapts scheduler and Zellij observations to those typed inputs.

`lisa-cli` owns the isolated Git completion transaction.

Native plugin tests can depend on `lisa-cli` through a dev dependency.

The plugin library has a `cdylib` target but runs native unit tests as a lib.

Native tests have access to private adapter methods through the parent module.

The native build provides a no-op Zellij host command symbol.

This lets tests observe requested host effects without a live Zellij server.

## Typed completion state

`lisa_core::completion` contains the completion state machine.

The durable states include Eligible, Requested, CommandInFlight, Rejected,
and Confirmed.

CommandInFlight contains a correlation and an absolute deadline.

Confirmed contains the terminal success fact.

Rejected contains a reason and a typed retryability value.

An expired in-flight reconciliation becomes action-required Rejected.

A successful correlated result becomes Confirmed.

These names are visible in adapter assertions and journal reconstruction.

`reconcile_completion` returns a typed reconciliation outcome.

Before the deadline, a reconstructed CommandInFlight returns replay intent.

At or after the deadline, it returns deadline-exceeded intent.

The reducer and reconciliation policy already have core-level tests.

This ticket needs adapter composition evidence rather than reducer-only proof.

## Completion journal

`crates/lisa-plugin/src/completion_journal.rs` is the durable adapter journal.

The journal is newline-delimited JSON.

Requested stores the exact completion generation and prior ticket state.

CommandInFlight stores the same generation, correlation, and deadline.

Confirmed stores the correlation and verified commit ID.

Rejected stores correlation, reason, and retryability when applicable.

`completion_journal::load` reduces journal records into aggregate state.

The latest aggregate is indexed by ticket ID in `State`.

Publication is atomic and leaves no temporary sibling on success.

Appending a transition updates disk before the in-memory aggregate.

An unhealthy or unreadable journal fails closed.

The existing journal unit suite verifies serialization and reconstruction.

It does not by itself execute the CLI Git transaction.

## Plugin restart reconstruction

`State::restore_completion_journal` loads aggregates into a fresh state.

It records whether restoration was healthy.

`State::mask_completion_transaction` hides unconfirmed Done bytes.

Live pending state supplies the pre-command phase and status when present.

After restart, the reconstructed aggregate supplies those same prior facts.

`State::rebuild_dag` scans tickets and applies this mask before DAG creation.

Consequently a completion commit may exist while scheduler authority stays
at Review until its correlated success is confirmed.

`State::reconciliation_state` prefers the reconstructed journal aggregate.

A fresh state has no in-memory pending completion.

Reconciliation recreates pending state only for the exact durable generation.

The replay preserves the original correlation and original absolute deadline.

It marks the pending invocation as a reconciliation replay.

It does not append a second Requested or CommandInFlight record.

## Real adapter gateway

`State::dispatch_completion_at` is the deterministic adapter entry point.

It accepts explicit time, allowing deadline assertions without sleeping.

Reconcile revalidates the current attempt lease and admitted Review evidence.

It asks the core reconciler how to handle current durable state.

Replay is delegated to `State::replay_in_flight_completion`.

That method refuses replay while another pending invocation exists.

It verifies generation, attempt, correlation, deadline, and ticket path.

It rebuilds argv through `State::build_completion_command`.

Initial and replay launches share `launch_completion_host_command`.

Native tests record the typed effect in `launched_completion_effects`.

`State::handle_completion_result` is the result boundary.

It verifies the authority is still current.

It verifies successful output is a commit ID.

It rescans durable ticket bytes and requires actual Done frontmatter.

It persists Confirmed before release, provenance, and dependent scheduling.

A duplicate late result has no pending correlation and is ignored.

## CLI idempotency boundary

`lisa_cli::commit_transaction::complete_ticket` is callable from native tests.

It receives repository root, ticket ID, ticket path, work path, message,
and the typed completion generation.

The transaction records completion identity in the Git commit.

Repeating the same request discovers the existing completion commit.

The repeat returns that prior commit ID with no newly committed paths.

This is stronger evidence than counting adapter launch effects alone.

The repository commit count can prove no second commit was created.

The committed tree can prove the ticket is durably Done.

## Provenance authority

`State::emit_provenance` writes the mixed provenance ledger.

Successful completion confirmation emits an Execution record.

The record has outcome Done and `authoritative: true`.

No record is emitted merely because the CLI changed ticket bytes.

The record appears only after the adapter accepts the correlated result.

A repeated result cannot emit a second row without live pending state.

Counting and decoding the ledger establishes authoritative exactly-once.

## Predecessor harness

`crates/lisa-plugin/src/tests/hostile_order_regression.rs` was added by the
immediately preceding ticket.

It is registered from the private test module in `src/lib.rs`.

Its `NestedRepo` helper creates a real temporary Git repository.

The Lisa project lives at `games/midsummer`, two levels below the Git root.

Its `Scenario` helper creates a primary ticket and dependent ticket.

It installs real adapter configuration, thread, slot, and attempt authority.

It writes private Review and disposition artifacts for the current attempt.

Its `restart` method constructs a fresh `State` from durable paths.

That method restores the journal, rebuilds the masked DAG, and restores the
known current thread/lease/slot authority needed by deterministic polling.

Its `transaction_request` helper derives a CLI request from real adapter argv.

The helper asserts the Git root and nested repository-relative path contract.

The passing hostile-order test already includes reload and lost-result steps.

Those steps are embedded inside a broader sequence with phase advancement,
timeout checking, operator `[d]one`, release, and dependent scheduling.

The blocked hostile-order test proves no completion side effects for Block.

## Earlier inline regressions

`src/lib.rs` contains an older restart reconstruction test.

It verifies an exact in-flight aggregate survives a fresh `State`.

It also verifies Confirmed reconstructs after successful result handling.

That fixture uses a synthetic commit ID for the confirmation leg.

It does not prove rediscovery of a real prior Git commit.

`src/lib.rs` also contains a lost-result/reload/duplicate-stop regression.

That test uses a real temporary Git repository and the real CLI transaction.

It predates the story-level nested-monorepo real-adapter harness.

Its project is at the repository root rather than `games/midsummer`.

Its setup and assertions are embedded among the large legacy test module.

There is also a separate deadline regression.

That test proves unresolved in-flight state ends action-required at deadline.

The present ticket asks for focused fixtures consuming the predecessor harness.

## Test fixture conventions

Some crates keep recorded provider inputs under `tests/fixtures`.

Plugin Codex acknowledgment tests use JSON files loaded by `include_str!`.

Other tests use Rust scenario helpers and call those helpers fixtures.

The hostile-order harness is currently a Rust scenario fixture.

The completion journal contains generated correlations and deadlines.

Git commit IDs are also generated per temporary repository.

Static JSON is therefore not the only fixture form present in this project.

## Constraints and ownership

The ticket starts with Lisa-owned changes to its ticket and provenance files.

An unrelated untracked `crates/lisa-plugin/docs/` tree is also present.

Those paths are not ticket-owned source and must remain untouched.

Phase artifacts belong only in the private attempt work directory.

Source must be committed with exact paths via `lisa commit-ticket`.

The ordinary Git index must not be used.

No source file may remain modified or untracked at Review handoff.

The ticket explicitly allows tests and fixtures but no new production contract.

The predecessor harness is the closest boundary for the requested coverage.

The full workspace and WASM check are the repository verification boundaries.
