# Research: pane-time ownership lookup

## Ticket boundary

`T-043-01-02` is the plugin half of story `S-043-01`.

The ticket asks for one lookup over facts the scheduler already records:

- physical terminal pane ID;
- attempt start time;
- attempt end time;
- owning ticket ID.

The observable acceptance case is a recycled physical pane. Ticket A owns the
pane during an earlier interval, ticket B owns it during a later interval, and a
capture outside both intervals has no owner.

The story explicitly excludes writer and consumer behavior changes. The capture
writer continues to emit the legacy artifact until `S-043-02`, and the plugin
continues to consume that artifact until `S-043-03`.

## Repository guidance

`AGENTS.md` delegates all repository guidance to `CLAUDE.md`.

`CLAUDE.md` identifies Lisa as a Rust workspace containing `lisa-core`,
`lisa-plugin`, and `lisa-cli`. Native workspace tests are the normal behavioral
verification surface for plugin logic.

The RDSPI workflow requires Research, Design, Structure, Plan, Implement, and
Review artifacts. Attempt artifacts belong under the private `.lisa/attempts`
work directory. Ticket source changes must be committed through
`lisa commit-ticket` with exact repository-relative include paths.

The working tree already contains Lisa-managed phase changes in both
`T-043-01-01.md` and `T-043-01-02.md`. Those ticket files are not source owned by
this implementation and must not be included in its source commit.

## Story topology

`docs/active/stories/S-043-01.md` defines two independent foundation tickets.

`T-043-01-01` defines the append-only `CaptureRecord` schema in `lisa-core`.
`T-043-01-02` defines the ownership lookup in `lisa-plugin`.

The story says the two tickets have disjoint file ownership and may execute in
parallel. Downstream ticket `T-043-03-01` depends on both contracts and will wire
capture records into plugin provenance attribution.

The downstream text describes the intended consumer operation: load captures,
attribute each by pane and capture time, and sum only the captures belonging to
the provenance record being emitted.

## Current plugin organization

The plugin implementation is rooted at `crates/lisa-plugin/src/lib.rs`.

The repository guide mentions a historical `scheduler.rs`, but the current tree
has no such file. Scheduler state and teardown behavior currently live in
`lib.rs`, with smaller focused modules for adapters, completion journaling,
deadlines, pane naming, publication, signals, and UI.

`State` stores active threads in `HashMap<TicketId, Thread>`.

Each `Thread`, defined in `crates/lisa-core/src/types.rs`, contains:

- `ticket_id`;
- `pane_id`;
- optional attempt lease;
- current phase and status;
- `started_at`;
- last phase and activity timestamps;
- provider and route details;
- concurrency-at-spawn metadata.

The active thread table therefore contains the start-side ownership facts while
an attempt remains resident in scheduler memory.

## Durable terminal intervals

`crates/lisa-core/src/provenance.rs` defines `ProvenanceRecord`.

Its ownership-relevant fields are:

- `ticket_id: String`;
- `attempt_lease: AttemptLease`;
- `started_at: u64`;
- `ended_at: u64`;
- `pane_id: u32`.

The timestamps are UTC epoch seconds. `system_time_to_epoch` converts Rust
`SystemTime` values to that representation and saturates pre-epoch values to
zero.

`ProvenanceRecord` also carries outcome, authority, fencing, route, duration,
usage, cost, and concurrency data. Those fields are orthogonal to the requested
pane-time lookup.

`append_record` serializes one provenance record as one JSONL row. It creates the
parent and file if needed and opens the file in append mode. Existing records are
not rewritten.

The provenance ledger is consequently the durable history that survives removal
of an attempt from the active thread table and later recycling of its pane.

## Mixed provenance ledger

Schema version 3 also supports `AssignmentTransitionRecord` rows.

Those rows describe transitions that ended before provider ownership. They have
pane, ticket, and timestamps, but their module documentation explicitly calls
them pre-ownership provenance.

`ProvenanceLedgerRecord` is an untagged enum with `AssignmentTransition` and
`Execution` variants. Only the `Execution(ProvenanceRecord)` variant represents
the completed attempt interval named by this ticket and story.

This distinction matters because treating assignment failures as ownership
would contradict their documented meaning.

## Teardown writer

`State::emit_provenance` in `crates/lisa-plugin/src/lib.rs` is called immediately
before a finishing thread is removed.

The method reads the active thread, requires an attempt lease, checks current
lease authority for successful completion, and snapshots the thread's client,
start time, concurrency, and pane.

It obtains the end time from `SystemTime::now()`, converts start and end to epoch
seconds, constructs a `ProvenanceRecord`, fills its usage fields, and appends it
to `.lisa/provenance.jsonl`.

This establishes the terminal interval before scheduler teardown loses the
thread-table copy. Multiple attempts append multiple rows, including retries or
later tickets assigned to a recycled pane.

## Existing provenance tests

Plugin unit tests are embedded in the `lib.rs` test module.

Helpers include `with_ledger`, `read_ledger`, and `read_mixed_ledger`. Existing
tests construct threads, install attempt leases, emit provenance, then deserialize
ledger rows and assert their attempt, pane, time, outcome, and authority fields.

Tests already establish that retries append rather than rewrite and that two
attempt leases for the same ticket remain distinct rows.

Other tests exercise fenced predecessor and replacement timelines. These prove
that multiple terminal attempt intervals can coexist in the ledger, but there is
currently no reusable lookup that selects an owner by pane and timestamp.

## Current usage path

`State::read_usage` selects either `.lisa/codex/<ticket>.usage.json` or
`.lisa/claude/<ticket>.usage.json` from the thread client and ticket ID.

It reads one JSON object, extracts the nested `usage` value, and returns token and
cost fields. The lookup is keyed by a guessed ticket, not by pane and time.

`State::emit_provenance` invokes `read_usage` before appending the terminal
record. This ordering is relevant to the downstream integration ticket but is
not changed by the present contract-only ticket.

## Time and interval observations

Provenance timestamps have one-second resolution.

The ticket describes windows as `t0..t1` and `t2..t3` and asks for timestamps
inside each window plus timestamps outside both. It does not require resolving
an intentionally overlapping pair of different-ticket intervals.

Start and end timestamps are both factual observations on a terminal record.
Captures can occur exactly when either timestamp is recorded because both sides
are reduced to epoch seconds.

The ledger is append ordered in normal operation, but a lookup over pane-time
facts does not need to depend on physical row order when intervals do not
overlap.

Duplicate terminal rows are possible because provenance is append-only and retry
tests explicitly preserve repeated records. Duplicate rows for the same ticket
do not introduce a different owner identity.

Different-ticket overlapping intervals would make ownership ambiguous. The
existing schema does not contain a priority rule that could honestly select one
of them.

## Type and module boundaries

`lisa-plugin` already depends on `lisa-core` and imports `ProvenanceRecord` in
`lib.rs`, so plugin-owned lookup code can operate directly over the durable
record type without changing core schema.

A focused plugin module can depend only on `lisa_core::provenance::ProvenanceRecord`.
It does not need Zellij APIs, filesystem access, mutable scheduler state, capture
schema details, or UI behavior.

The narrow input boundary is an iterator or slice of execution records plus a
pane ID and epoch-second capture time. The narrow result boundary is an optional
borrowed ticket ID.

Keeping record loading outside that boundary lets the later consumer combine
already persisted rows with a current terminal record that has been constructed
but not yet appended.

## Constraints surfaced

- Do not edit either ticket's phase or status frontmatter.
- Do not write workflow artifacts to `docs/active/work`.
- Do not modify the capture writer or legacy usage reader in this ticket.
- Do not include assignment-transition rows as owned intervals.
- Do not create a second durable ownership store; provenance already owns that
  history.
- Do not make lookup correctness depend on ledger append order.
- Do not fabricate an owner when different tickets match the same pane-time.
- Keep the source change inside `lisa-plugin`, preserving the story's declared
  parallel boundary with `T-043-01-01`.
- Verify the recycled-pane acceptance case with a native plugin unit test.
- Commit all ticket-owned source paths with one exact isolated Lisa transaction.

## Research conclusion

The needed facts already exist in terminal `ProvenanceRecord` rows and are
durably append-only. The missing seam is a plugin-local, reusable selection
operation over those records. It must discriminate by both pane and time, ignore
pre-ownership transition evidence by construction, preserve no-owner outcomes,
and remain independent from the writer and consumer changes assigned to later
stories.
