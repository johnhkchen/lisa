# Research: authorized Codex field report

## Ticket boundary

T-040-03-04 is the final field-evidence ticket in S-040-03.
It starts at Research and requires all six RDSPI phases.
Its output is evidence and a report, not a scheduler patch.
The acceptance criterion authorizes live, metered Codex seats.
It requires two hostile observations:

- a real Review carrying a blocking disposition;
- a real failure before assignment ownership.

Both cases must run in isolated disposable fixtures.
They must not alter this repository's active tickets to manufacture state.
Any unexplained behavior or behavior change blocks Done.

## Settled input build

The dependency T-040-03-03 completed at commit `3f99539`.
Its rebuild consumed source revision `48b9bf80ca59013e7e46f1010c4ac04623762890`.
The release executable is `target/release/lisa`.
Its recorded SHA-256 is:

`498134e92f43ea5a3d834c5cb22afdf5d6ad180e2543ae543b4ae84588addfe9`.

The release plugin is `target/wasm32-wasip1/release/lisa.wasm`.
Its recorded SHA-256 is:

`053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f`.

T-040-03-03 compared the build-script copy with the release WASM byte for byte.
It also ran the full deterministic gate and both hostile regressions.
This ticket must bind to those artifacts rather than rebuild or rerun those gates.

## Deterministic evidence already available

`docs/active/work/T-040-03-03/review.md` records 794 passing tests.
It records the native and WASM Clippy gates, formatting, `just check`, and release builds.
The blocking Review discriminator is:

`test_t039_06_02_blocking_review_never_prepares_done`.

That test proves a blocking disposition retains assignment and lease,
does not enter pending completion, emits no Done provenance,
and leaves a dependent blocked.

The pre-ownership discriminator is:

`rc6_preownership_delivery_miss_is_durable_and_cli_retrievable`.

That test drives the production delivery timeout,
requires a physical assignment-transition ledger row,
and reads that same row through the CLI status implementation.

Those tests are deterministic proof, not live-provider observations.
The present ticket must preserve that distinction in its report.

## Review disposition contract

The RDSPI workflow requires `review.md` and `review-disposition.json` together.
The only valid blocking form is:

`{"disposition":"block","reason":"<non-empty actionable reason>"}`.

`lisa-core` parses this document fail-closed.
The plugin admits it through the attempt-aware artifact path.
The completion gate reads the admitted disposition before preparing Done.
A live blocking case must therefore reach Review with both artifacts present.
It must remain assigned and non-Done after the model stops.

## Pre-ownership evidence contract

`crates/lisa-core/src/provenance.rs` defines assignment transition records.
They are distinct from authoritative terminal execution records.
The row includes ticket, attempt, pane, provider, state, reason, and timestamps.
Named terminal states include delivery, recovery, and startup failures.

The writer is invoked from scheduler failure transitions in `lisa-plugin`.
The durable ledger is `.lisa/provenance.jsonl` in the fixture repository.
The CLI reconstruction surface is:

`lisa status --path <fixture> --ticket <ticket-id>`.

The report must retain both the physical JSONL row and rendered CLI output.
It must not misdescribe a pre-ownership failure as an execution outcome.

## Existing live harness patterns

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the closest host harness.
It creates external temporary Git repositories,
runs `lisa init`, writes a one-thread config,
starts a named Zellij session through a wrapper,
discovers plugin and agent panes,
samples dashboards and signals,
and tears down live sessions.

It also establishes the current Codex bootstrap contract.
Codex begins in `starting`, then startup grace moves directly to `delivering`.
It must not claim `ready-for-assignment` merely because time elapsed.
A matching assignment acknowledgement moves `delivering` to `owned`.

The live harness uses an ephemeral `CODEX_HOME`.
It symlinks the installed authentication file,
copies Lisa's generated hooks,
enables hooks,
and lets `lisa loop` add canonical project trust.
The ephemeral home is deleted during cleanup.

## Host control surfaces

`zellij action list-panes --json --all` exposes pane identity and titles.
Plugin panes are reported as plugin pane IDs.
Agent panes have ticket-bearing titles.
`zellij action dump-screen` retains dashboard and terminal observations.
`zellij action close-pane --pane-id` can terminate an agent pane
without killing the plugin session.

The scheduler owns retry and terminal failure policy.
The harness must observe it rather than changing retry counts.
Repeatedly interrupting only live pre-ownership Codex panes
can exercise the bounded startup/recovery path.
The case is complete only when a durable terminal assignment row exists.

## Isolation constraints

Fixture roots must be outside this repository.
Each fixture must be a new Git repository with its own baseline commit.
The fixture tickets use synthetic IDs that cannot collide with active tickets.
The ticket contents manufacture cases only inside the disposable repositories.
This repository's ticket frontmatter remains Lisa-managed and untouched by the harness.

Evidence must live outside the fixture while the fixture is torn down.
The attempt-private work directory is the durable destination.
Fixture snapshots are unnecessary if targeted evidence is copied first.
Named Zellij sessions and fixture roots must be removed on success and failure.
Ephemeral Codex homes must also be removed.

## Evidence surfaces

Build binding needs the absolute `LISA_BIN`, size, SHA-256, and version.
The generated layout names the CLI and extracted embedded WASM.
The extracted WASM hash must match the T-040-03-03 release WASM hash.

For each case, useful transition evidence includes:

- timestamped dashboard state observations;
- final dashboard and agent screen;
- pane inventory;
- lease, started, and acknowledgement signals when present;
- ticket frontmatter after the case;
- private and published artifact listings;
- `review-disposition.json` content;
- `.lisa/provenance.jsonl`;
- rendered `lisa status` output;
- Git log, commit tree, and worktree status.

## Blocking Review expected observation

The Codex assignment should explicitly request concise artifacts only.
It should require the final disposition to block for a stated harness reason.
After the agent stops, the ticket should be in Review rather than Done.
No completion commit should exist.
No authoritative Done provenance should exist.
A dependent ticket should remain open and unscheduled.
The blocking reason should remain readable in the structured disposition.

## Pre-ownership expected observation

The Codex process must actually launch in a ticket pane.
The harness must interrupt it before `owned` is ever observed.
If Lisa performs its bounded recovery launch, that replacement must also be interrupted.
The scheduler should settle in a named terminal pre-ownership failure.
Exactly one durable terminal assignment record should be present for the attempt.
The CLI should render provider, state, reason, and timing from that ledger alone.
No authoritative execution outcome or Done commit should exist.

## Repository ownership

This ticket does not presently require product source changes.
Attempt-private scripts and evidence are phase/report artifacts,
not shared production source units.
Therefore `lisa commit-ticket` is only required if implementation discovers
a justified ticket-owned repository source change.
The ticket explicitly forbids patching anomalies during the report,
so such a discovery would instead produce a blocking disposition.

## Known constraints

The run is live and timing-sensitive.
Dashboard sampling can miss brief states,
so durable signals and ledger rows are stronger evidence than screen text alone.
Codex may acknowledge quickly after prompt submission.
The pre-ownership case therefore needs continuous pane discovery and prompt interruption.

Provider behavior, Zellij timing, and network availability are observations,
not deterministic guarantees.
An inability to produce either required live case is itself acceptance failure.
An unexpected completion, missing row, duplicate outcome, or teardown failure is blocking.

## Research conclusion

The repository already contains the production behavior and deterministic regressions.
The remaining work is a bounded two-case live observation against the exact rebuild.
The evidence boundary is strong enough to identify build identity,
state transitions, structured disposition, durable pre-ownership failure,
commit outcome, and cleanup without modifying active project tickets.
