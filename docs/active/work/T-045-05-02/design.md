# Design — T-045-05-02 field regression assertions

## Decision summary

Extend the predecessor's dedicated live harness in place.

Keep the historical case as the one-ticket false-failure control.

Change the current case to a two-ticket Review dependency chain.

Use real Codex for both current tickets.

Require the same claim-first first tool action for each ticket.

Gate each exact claim on that ticket's observed passive-wait state.

Attempt one producer-side stale claim against the live successor lease before opening its gate.

Record ticket-scoped state transitions and structured process identity.

Assert every epic Done condition before printing the final PASS receipt.

Update the runbook so field evidence and failure interpretation match the stronger contract.

Do not change Rust production code or deterministic tests.

## Goals

The assertion layer must prove observed field behavior, not only file existence.

The same run must connect slow delivery to exact claim ownership.

The same run must cross an actual completion boundary.

The successor must be blocked until the predecessor completes.

The successor must run in a newly launched Codex process.

Durable journals must show one authoritative completion per ticket.

A stale identity must be rejected against a live current lease.

Failures must preserve evidence and remain nonzero.

## Non-goals

Do not alter assignment timeouts.

Do not alter the Codex launcher.

Do not alter claim validation.

Do not alter completion or lease fencing.

Do not add authenticated provider work to `cargo test`.

Do not make Claude use the Codex handshake.

Do not use a stub provider as field proof.

Do not publish attempt evidence to canonical work directly.

## Option 1 — assert only predecessor evidence

The final T-045-05-01 evidence already proves one slow claim and one completion.

This option would add an offline script that reads those retained files.

It would be cheap and would avoid another metered run.

It cannot prove a fresh successor TUI because the fixture had one ticket.

It cannot prove predecessor lease revocation at a successor boundary.

It cannot test a stale claim against a successor's live lease.

It also would not execute assertions within the live harness as the ticket requests.

Decision: reject as incomplete closing evidence.

## Option 2 — add a second independent current case

The harness could run current Lisa twice with one ticket per repository.

Distinct process IDs would be easy to observe.

Each run would still have no scheduler transition between tickets.

Separate repositories cannot prove lease revocation or dependency release.

Two independent sessions are weaker than one actual clean boundary.

They also consume more setup time without adding the required lifecycle relation.

Decision: reject.

## Option 3 — use one current two-ticket chain

The fixture can add a successor Review ticket depending on the first.

The scheduler naturally serializes them on one physical seat.

Completing the first triggers real `/exit`, lease revocation, and shell recovery.

Scheduling the second triggers a new launcher and Codex child.

Both tickets can use the small Review-only metered workload.

Existing generic fixture instructions work for either ticket identity.

This produces direct field evidence for every remaining assertion.

Decision: choose.

## Option 4 — inject a stale claim file directly

The harness could write `pane-<id>.claim` with an old body.

That would exercise plugin consumer rejection.

It would bypass the native claim producer.

It would also look like harness-synthesized claim evidence.

The story emphasizes actual live claim behavior.

Decision: reject as the primary stale-claim proof.

## Option 5 — invoke `lisa claim` against the live successor lease

The sampler knows the real pane ID and fixture root.

At successor passive wait, the lease marker is current and stable.

The harness can invoke current Lisa with that pane ID.

It can name the successor and attempt zero.

The native producer must reject `stale-attempt` before nonce publication.

The exact real-Codex claim then follows through the existing gate.

The receipt is stable, bounded, and separate from provider text.

Decision: choose.

## Why the stale probe is harness-owned

The agent's first tool action remains the exact claim protocol.

The harness probe occurs while that tool action is blocked on its private gate.

The probe cannot accidentally become ownership evidence because it exits nonzero.

No claim signal is published for the rejected identity.

The exact claim still originates from real Codex's first tool call.

This preserves claim-first ownership while testing rejection beforehand.

## Ticket identities

Use `T-FIELD-REVIEW-01` for the predecessor.

Use `T-FIELD-REVIEW-02` for the successor.

Keep both in the same synthetic story.

Both begin `status: open` and `phase: review`.

The successor declares `depends_on: [T-FIELD-REVIEW-01]`.

Each gets its own canonical prior `review.md`.

The historical case stops at the first ticket's expected failure.

Its blocked successor adds no legacy provider cost.

## State evidence design

Replace global first-seen state records with ticket-scoped transitions.

Each TSV row will contain timestamp, ticket ID, and state.

The sampler extracts the scheduler thread row for each ticket.

It appends only when that ticket's displayed state changes.

Expected current transitions per ticket are:

1. `starting`;
2. `delivering`;
3. `delivered-awaiting-claim`;
4. `owned`.

An exact transition sequence rules out a retry back to `delivering`.

It also rules out delivery and claim terminal failures.

The historical assertion accepts the named failed banner for ticket one.

## Prompt evidence design

State transitions are the primary no-reinjection proof.

File cardinality provides the publication proof.

Each ticket must have one assignment file and one launch script.

Each ticket must have one captured valid claim body.

At every screen snapshot, a ticket's tagged `LISA_ASSIGNMENT` line may occur at most once.

Repeated snapshots do not inflate the per-screen count.

The assertion is deliberately per screen, not across the snapshot file.

Together these checks distinguish repeated observation from repeated submission.

## Process evidence design

Add `process-events.tsv` beside raw process snapshots.

Record each newly observed assignment-bearing process once.

Rows contain timestamp, ticket, role, PID, parent PID, and assignment path.

Roles are `launcher` and `codex`.

Each current ticket must have exactly one launcher PID and one Codex PID.

Predecessor and successor launcher PIDs must differ.

Predecessor and successor Codex PIDs must differ.

Each row must carry that ticket's exact immutable assignment path.

The structured ledger avoids parsing repeated raw snapshot occurrences at review time.

## Lease-boundary evidence design

Add `lease-events.tsv` for pane lease transitions.

Record the parsed lease ticket and attempt whenever the marker changes.

Record an explicit `absent` edge when a previously present marker disappears.

The expected current sequence is predecessor lease, absent, successor lease.

At minimum, both exact lease bodies must be captured by the signal sampler.

The stale probe against the successor further demonstrates the predecessor identity is not current.

The new process PID completes the clean-boundary proof.

## Completion assertion design

Wait until both current ticket files are Done.

Copy both final tickets, work trees, and attempt trees.

For each ticket, query the completion journal with `jq -s`.

Require exactly one requested record.

Require exactly one command-in-flight record.

Require exactly one confirmed record.

Require all three to use attempt one and generation one.

Require the confirmation commit ID to be a 40-character hexadecimal string.

For each ticket, query provenance with `jq -s`.

Require exactly one row.

Require Done, authoritative true, fenced false, and Codex actual method.

Require exactly two total completion triples and two total provenance rows.

## Claim assertion design

Each exact captured claim must match its assignment filename nonce.

Do not merely check that nonce has numeric JSON type.

Extract the nonce from the one assignment filename.

Require ticket, attempt one, and exact nonce in the copied signal.

The stale receipt must exit nonzero.

Its stderr must contain `[stale-attempt]`.

Its stdout should be empty.

The signal ledger must contain no attempt-zero claim.

## Failure behavior

Every assertion calls the existing `fail` boundary.

The active case path is printed.

State, signal, terminal, process, and loop diagnostics remain bounded.

Evidence directories remain retained after nonzero exit.

There is no expected-failure exemption in the current case.

Any unexplained Codex prompt, timeout, missing claim, duplicate transition, or journal mismatch fails.

Review disposition will be `block` if such a live failure cannot be explained and corrected as a
harness defect.

## Stable receipts

Keep `legacy-false-delivery-failure: OBSERVED`.

Replace the single-ticket current receipt with granular assertion receipts.

Emit one line for slow claim and no reinjection.

Emit one line for stale claim rejection.

Emit one line for fresh-TUI clean boundary.

Emit one line for exact completion cardinality.

End only with `live-codex-review-boundary: PASS` after all checks.

## Verification strategy

Run shell syntax and ShellCheck before live execution.

Run `PREPARE_ONLY=1` with a private attempt evidence path.

Run focused claim and launcher CLI tests.

Run the two predecessor deterministic plugin regressions.

Commit only after the final authorized live run passes.

Use exact source paths in `lisa commit-ticket`.

Keep field evidence attempt-private and out of the source commit.
