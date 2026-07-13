# Research — T-045-05-02 field regression assertions

## Ticket boundary

The ticket begins in Research.

Its only acceptance criterion is an assertion set over a live field run.

The required provider is installed Codex.

The required terminal multiplexer is installed Zellij.

The required provider configuration has hooks disabled.

The assertions cover six observable lifecycle properties:

- a fresh Codex TUI for each ticket;
- ownership established by a claim before workflow work;
- a slow first response reaching `delivered-awaiting-claim`;
- no duplicate assignment injection during that wait;
- rejection of a stale claim;
- a clean ticket boundary with exactly one authoritative completion per ticket.

Unexplained real-provider behavior must leave the ticket blocked.

The story explicitly excludes stub acknowledgement as closing evidence.

The story also says this ticket changes no scheduler contract.

## Predecessor harness

T-045-05-01 landed in source commit `7324191`.

Its completion commit is `4600b2f`.

The source commit added two files:

- `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`;
- `docs/knowledge/live-codex-review-boundary.md`.

The shell harness is deliberately outside the default Cargo test suite.

It consumes authenticated provider quota only when invoked directly.

It has a non-metered `PREPARE_ONLY=1` mode.

It compares a historical Lisa executable with a release executable built from HEAD.

The historical case reproduces the hook-less false delivery failure.

The current case proves one Review ticket can wait, claim, and complete.

The predecessor review explicitly delegates this ticket's six assertions.

## Existing fixture shape

`create_fixture` creates a disposable Git repository outside the source checkout.

The fixture is initialized with the selected Lisa binary.

Its `.lisa.toml` selects Codex and one scheduler thread.

Assignment acknowledgement timeout is eight seconds.

Review timeout is 300 seconds.

Session timeout is 1800 seconds.

The fixture currently contains one ticket, `T-FIELD-REVIEW`.

That ticket begins open in Review.

Canonical work already contains a prior `review.md`.

The agent needs only current-attempt Review artifacts.

The fixture Git baseline includes all project instructions and canonical work.

## Real-agent protocol

The fixture `AGENTS.md` controls the first tool action.

The first shell action locates the current immutable assignment file.

It derives the assignment nonce from the filename.

It waits for `.field-claim-gate` in the current attempt work directory.

The sampler writes that gate only after observing `delivered-awaiting-claim`.

After a one-second settling delay, the shell action runs `lisa claim`.

Only after the command returns may Codex inspect prior Review work.

The old Lisa case has no nonce assignment and takes a delayed no-claim branch.

The protocol therefore uses real Codex tool execution in both cases.

The claim signal is not synthesized by the harness.

## Claim producer boundary

`crates/lisa-cli/src/claim.rs` implements native claim publication.

The command reads `LISA_PANE_ID` from its inherited environment.

It reads `.lisa/signals/pane-<id>.lease` twice.

It checks ticket identity before attempt identity.

An attempt lower than the pane lease is rejected as `stale-attempt`.

An attempt higher than the pane lease is rejected as `attempt-mismatch`.

An exact attempt with no exact assignment file is rejected as `wrong-nonce`.

Only a valid claim is atomically renamed to `pane-<id>.claim`.

Rejected claims exit nonzero and print a stable bracketed reason to stderr.

The stable stale receipt is `claim rejected [stale-attempt]`.

The CLI producer does not need the plugin to consume a rejected claim.

The live pane lease still makes rejection an end-to-end durable-identity check.

## Claim consumer boundary

`State::admit_assignment_claim` is the plugin's final authority check.

It refuses claims after a seat is already owned.

It requires a claimable delivery state.

It matches pane reservation, current lease, ticket, attempt, and nonce.

It compares the retained `AssignmentRef` with the lease and claim.

Only an exact current claim changes the seat to `Owned`.

`check_claim_signals` deletes consumed claim files.

The harness sampler copies signals before that deletion when possible.

Distinct claim bodies have distinct hashes even when the pane filename is reused.

## Slow-delivery state machine

Current Codex launches start in `Starting`.

After startup grace, Lisa submits the assignment reference once.

That edge enters `Delivering` with retry zero.

At the first acknowledgement deadline, a live Codex seat does not retry.

It enters `DeliveredAwaitingClaim` with a bounded claim deadline.

The UI renders that state as `delivered-awaiting-claim`.

The deterministic test
`live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably`
asserts unchanged delivery logs, pending Enters, launches, and lease.

The live harness currently observes only the first appearance of each label.

It does not yet record repeated state transitions for a particular ticket.

Its dashboard snapshots retain every sampled screen.

Its terminal snapshots retain queued and active prompt text.

## Current live evidence

The predecessor's final authorized evidence remains attempt-private.

Its current state timeline is:

1. `starting`;
2. `delivering`;
3. `delivered-awaiting-claim`;
4. `owned`.

The claim was captured one second after the waiting state.

No `.ack` or `.started` hook signal was captured.

The terminal showed one tagged assignment message queued during the first tool action.

Repeated terminal snapshots repeat the same visible screen.

Snapshot occurrence counts therefore cannot be treated as injection event counts.

A transition-aware state ledger can show whether delivery restarted.

The immutable assignment and launch-script cardinalities provide additional evidence.

## Completion boundary

T-045-04-01 makes Codex completion request `/exit`.

It revokes the completed ticket's attempt lease.

It releases the slot into `WaitingForExit`.

The next ticket cannot launch until the old TUI has exited.

The slot then returns to an empty shell before scheduling a successor.

T-045-04-02's deterministic test
`codex_completion_exits_revokes_and_launches_next_fresh_tui`
exercises the entire sequence.

That test compares predecessor and successor assignment paths and nonces.

It rejects the predecessor claim after completion and after successor launch.

It asserts one completion effect, one confirmation, and one provenance row.

The existing live harness has only one current ticket.

It therefore cannot yet observe the successor launch boundary.

## Completion durability evidence

The live harness copies `.lisa/completion-journal.jsonl`.

A normal ticket completion has exactly three records:

- `requested`;
- `command-in-flight`;
- `confirmed`.

Each record carries completion ID, attempt ID, and generation.

The confirmed record carries a 40-character commit ID.

The harness also copies `.lisa/provenance.jsonl`.

A successful current Review fixture has one schema-version-three record.

That record has `outcome: done`.

It has `authoritative: true` and `fenced: false`.

It names the actual and requested Codex provider.

Per-ticket JSON filtering can assert cardinality without relying on line order.

## Process and lease observations

The sampler records host process snapshots every 100 milliseconds.

It retains full rows for the fixture root, Lisa launcher, and assignment-bearing Codex.

The current case shows a `lisa launch-codex` parent PID.

It also shows a Codex child PID with the same assignment path.

The current evidence has only one ticket, so PID inequality is not asserted.

The signal sampler records each unique basename-and-digest pair.

The pane lease basename can be reused across tickets.

Its body changes because ticket identity changes.

The sampler can therefore retain both predecessor and successor lease generations.

A brief absence between lease bodies is also observable at the polling boundary.

## Fresh-TUI assertion inputs

A two-ticket dependency chain is necessary for a real boundary.

Both tickets can use the same Review recovery shape.

The successor should depend on the predecessor.

With `max_threads = 1`, both use the same physical pane serially.

Each ticket receives a distinct attempt directory.

Each receives a distinct nonce-bearing assignment.

Each receives a distinct launch script in its own work directory.

The launcher and Codex child PIDs must differ between assignments.

The predecessor process must disappear before the successor process is admitted.

The successor's lease body must name only the successor.

## Stale-claim assertion inputs

The native CLI can exercise rejection against the live successor lease.

Setting `LISA_PANE_ID` to the actual agent pane selects the durable lease.

Naming the successor ticket avoids a `wrong-ticket` short circuit.

Using attempt zero against live attempt one produces `stale-attempt`.

Nonce validation occurs later, so a placeholder nonce is sufficient.

The rejection must exit nonzero.

It must leave no published claim signal.

The exact real-Codex claim can then establish ownership normally.

The rejection receipt can be retained separately from provider transcripts.

## No-reinjection assertion inputs

One ticket-specific state ledger can record every state transition.

The expected prefix is `starting`, `delivering`, `delivered-awaiting-claim`, `owned`.

`delivering` must occur exactly once.

`delivered-awaiting-claim` must occur exactly once.

No delivery or claim failure may appear.

There must be one assignment file and one launch script for attempt one.

There must be one captured exact claim for the ticket.

The prompt snapshot at the waiting boundary must not show duplicate tagged messages.

These observations cover transport, state, and durable publication separately.

## Harness safety and constraints

All waits must remain bounded.

Unexpected provider UI must fail and retain evidence.

No assertion may rewrite unexplained evidence into a pass.

Authentication content must remain outside evidence.

Ephemeral Codex homes must always be deleted.

Named Zellij sessions must always be killed.

Failed fixture repositories should remain available for diagnosis.

The source checkout has unrelated Lisa-owned dirty paths.

Ticket source ownership is limited to the existing harness and runbook.

Attempt artifacts belong only under this attempt's private work directory.

Source changes must be committed with `lisa commit-ticket` and exact includes.

## Verification surfaces

`bash -n` checks shell syntax.

ShellCheck is available from the predecessor run and checks quoting and portability.

`PREPARE_ONLY=1` validates fixture creation without launching providers.

Focused CLI tests cover claim and launcher producer behavior.

Focused plugin tests cover slow wait and completion boundary behavior.

Only the authorized live run can close the ticket's field acceptance.

The live result must include stable assertion receipts.

The final review disposition must be `block` for any unexplained live result.
