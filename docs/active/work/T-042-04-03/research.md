# Research: live Codex seat field run

## Ticket boundary

`T-042-04-03` is the final ticket in story `S-042-04`.

It is a live field-evidence and release-gate ticket.

It does not request a new completion contract or production fix.

The run must use one isolated disposable Codex seat.

The Lisa project must be nested at least two levels below its Git root.

The freshly rebuilt CLI must contain the freshly rebuilt plugin WASM.

One ticket must complete through normal artifact-driven completion.

A second completion must exercise the dashboard `[d]one` recovery path.

The report must retain path argv, correlations, journal, provenance, and Git tree evidence.

The disposable fixture and its Zellij/Codex runtime state must be removed.

Any unexplained behavior makes the final disposition blocking.

Workspace tests and the release WASM size gate are part of acceptance.

## Predecessor state

`T-042-04-01` added the real-plugin hostile-order regression.

Its fixture places a Lisa project at `games/midsummer` below a temporary Git root.

It crosses `State::build_completion_command` and the real CLI completion transaction.

It proves the command uses the Git root for `--path`.

It proves ticket and work paths are `games/midsummer/docs/...`.

Its passing case proves exactly one commit and authoritative Done row.

Its blocking case proves no effect, commit, Done row, release, or dependent scheduling.

`T-042-04-02` extended that harness for restart and lost-result replay.

It proves the journal reconstructs the original completion generation.

It proves replay retains correlation and absolute deadline.

It proves an idempotent repeat discovers the original commit.

It proves duplicate Stop, Reconcile, and result observations stay exactly once.

Both predecessor source units are committed on the current branch.

## Completion adapter

`crates/lisa-plugin/src/lib.rs` owns the production completion adapter.

All artifact, Stop, idle, observed-Done, reconcile, and operator inputs enter `dispatch_completion`.

The adapter delegates decision state to `lisa-core::completion`.

`execute_completion_effect` is the sole new-command launch boundary.

`build_completion_command` constructs the complete-ticket argv.

The configured `lisa_bin` is the absolute current executable embedded in the layout.

The command's `--path` is `State::git_root`.

The ticket and work paths pass through `completion_repository_relative_path`.

That helper joins host-view paths to the Lisa project root.

It then strips the discovered Git root.

For a `games/midsummer` project, the output begins with `games/midsummer/docs`.

The host command returns through a `lisa_completion` context key.

Only a matching live pending completion consumes the command result.

## Durable completion state

Production stores the adapter journal at `.lisa/completion-journal.jsonl`.

The journal is append-only JSONL.

A new command records `requested` before it records `command-in-flight`.

The correlation string is the completion generation identity.

Normal attempt completion uses the live attempt ID.

Explicit operator completion uses attempt identity `operator`.

Generation currently begins at `1` for either authority.

A successful correlated command appends `confirmed` with a commit ID.

A failed command appends `rejected` with a reason and retryability.

Retryable rejection accepts a later Request.

Action-required rejection suppresses automatic retries.

The journal is the strongest durable evidence for normal versus operator authority.

## Operator recovery path

The dashboard key `d` opens `ModalMode::MarkDone`.

Review-phase tickets remain selectable even with a running thread.

Enter calls `mark_ticket_done` for the selected ticket.

That emits `CompletionInput::OperatorRequested` with `MarkDoneKey` source.

The operator path does not borrow a current attempt lease.

Its correlation is `<ticket>:operator:1`.

The same structured pass disposition gate applies.

The same dependency gate applies.

The same exact-path isolated CLI transaction applies.

Pending and rejection results remain visible in the modal.

Success changes the modal to an accepted outcome before the scheduler releases the seat.

## Recoverable failure seam

`crates/lisa-cli/src/commit_transaction.rs` owns repository serialization.

It opens `.lisa-commit.lock` at the Git root.

It uses a nonblocking exclusive file lock.

A held lock returns an actionable `cannot acquire commit transaction lock` error.

The transaction does not mutate HEAD before acquiring this lock.

The adapter classifies a failed command result as retryable.

It appends a durable rejected journal row.

It removes process-local pending state and retains the ticket for recovery.

This is an existing tested failure seam, not a production modification.

Holding the lock only in the external fixture cannot affect this repository.

Releasing it immediately before modal Enter permits an operator-owned retry.

## Live startup precedent

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the current live harness precedent.

It builds plugin-first and CLI-second through `just build-cli`.

It uses an external temporary Git repository.

It creates a unique named Zellij session through a wrapper.

It runs `lisa loop` under `script` for a PTY.

It samples plugin and agent panes through Zellij actions.

It uses an ephemeral `CODEX_HOME`.

The ephemeral home symlinks, but does not copy, the installed authentication file.

It installs generated Codex hooks at the user layer and enables hooks.

It verifies canonical project trust written by Lisa doctor logic.

It captures the generated layout and hashes its extracted WASM.

It removes the ephemeral Codex home during cleanup.

## Fixture topology

The external Git root can be a fresh `mktemp` directory.

The Lisa project can be exactly `<root>/games/midsummer`.

That is two path components below the Git root.

Git initialization and the baseline fixture commit occur only in that disposable repository.

The fixture should contain two synthetic tickets in a dependency chain.

`T-LIVE-NORMAL` is initially eligible.

`T-LIVE-RECOVERY` depends on `T-LIVE-NORMAL`.

`max_threads = 1` provides one configured provider seat.

The layout creates two terminal panes for transition capacity, but only one concurrent seat is owned.

Codex uses the production ExitThenFresh reset strategy between assignments.

Both tickets therefore traverse the same configured seat while provider processes remain isolated.

## Normal completion evidence

The first ticket should request concise artifacts and a passing disposition.

The live Codex session must create all six Markdown artifacts.

It must also create `review-disposition.json` with pass/null.

Artifact polling should create the attempt-1 correlation.

The journal should end with requested, command-in-flight, and confirmed.

The fixture should gain one completion commit.

The provenance ledger should gain one authoritative Done execution row.

The dependent should then become scheduled.

## Recovery evidence

The fixture lock should be acquired after normal completion and before the recovery Review.

The second live Codex assignment remains an ordinary artifact-only task.

Its normal artifact completion should reach the exact existing CLI transaction.

The held lock should make that transaction fail without moving HEAD.

The journal should show attempt-1 requested, in-flight, then retryable rejected.

The dashboard modal should be opened while the lock remains held.

The harness should release the lock immediately before Enter.

Enter should create an operator requested/in-flight/confirmed sequence.

The final Git history should contain exactly two completion commits after baseline.

The final provenance should contain exactly two authoritative Done rows.

## Build and size boundary

The release plugin must be built before the release CLI.

The root `just build-cli` recipe performs that ordering and touches the WASM input.

The actual extracted embedded WASM must hash equal to the target release WASM.

The latest settled size measurement before this story was 1,425,425 bytes.

The repository defines material growth, not a hard numeric constant, as the budget criterion.

This ticket adds no production dependency or source.

Any unexplained material growth would therefore be blocking.

## Repository constraints

The current ordinary worktree includes Lisa-managed ticket and provenance changes.

It also includes a pre-existing untracked `crates/lisa-plugin/docs/` directory.

Those paths are outside this ticket's ownership and must be preserved.

All phase artifacts belong in this attempt-private work directory.

No artifact is written directly to shared `docs/active/work`.

No source change is presently expected.

Therefore no `lisa commit-ticket` source transaction is presently expected.

If live behavior exposes a defect, the ticket requires a blocking report rather than a patch.

## Research conclusion

The production surface already contains every mechanism needed for the requested run.

The field ticket should compose existing build, nested-root, completion, journal, modal, and cleanup contracts.

The held disposable transaction lock provides a bounded truthful recovery trigger.

The result can distinguish normal and operator correlations without changing the product.
