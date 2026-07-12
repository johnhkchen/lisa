# Review: repetition inventory and classification

## Outcome

The ticket acceptance criterion is satisfied.

A classified candidate list exists in `design.md`, with the concise working
form in `inventory.md`. It covers fourteen semantic repetition families across
the scheduler, adapters, harnesses, hook writers, and recurring scheduler test
fixture writes.

- Four candidates are `small demonstrated-value cleanup`.
- Ten candidates are `too-large → future epic`.
- Every candidate has a one-line justification.
- Every small candidate names a focused proof seam.
- Every deferred candidate names a future-epic boundary or states why no epic
  is currently recommended.
- No source cleanup landed in this spike.

## Artifacts created

- `research.md` maps the current code and repetition boundaries descriptively.
- `design.md` compares classification approaches and records all fourteen
  detailed candidate decisions.
- `structure.md` defines the inventory shape and exact successor boundaries.
- `plan.md` sequences inventory materialization and structural verification.
- `inventory.md` provides the concise acceptance-facing table.
- `progress.md` records execution, counts, and verification results.
- `review.md` provides this handoff.

All files were written in the private attempt directory. Lisa has already
admitted the required pre-Review phase artifacts to the shared work area while
advancing the ticket automatically. This attempt did not write the shared work
path or ticket frontmatter directly.

## Small candidates handed to T-038-03-02

### C-01: pane signal filename parser

The seven maintained pane-signal consumers repeat the pure
`pane-<u32>.<suffix>` parse chain. A local helper can own only that grammar;
payload parsing, deletion, admission, poll order, and state effects remain in
the consumers.

The successor should add table-driven parser coverage for valid and malformed
paths and retain all scanner regressions.

### C-02: native adapter reset default

Claude and Codex both return `ResetStrategy::ClearHandshake`. Supplying that as
the overridable trait default removes identical policy while preserving the
future-provider escape hatch.

Existing per-provider and resolver tests already exercise the result through
trait objects.

### C-03: native adapter follow-up default

Both native adapters construct the same `FollowUp::TypeIntoPane` value from
`finish_up_prompt`. An overridable trait default is a local removal of identical
behavior and leaves provider-specific launch, assignment, signal, and readiness
methods explicit.

Existing Claude and Codex follow-up tests compare the complete prompt.

### C-04: deterministic harness event counter

`event_count_is` and `event_count_at_least` duplicate event-file fallback and
`awk` counting inside one maintained script. A local `event_count` primitive
removes that policy duplication without creating a common harness library.

The ignored real-Zellij integration test is the required proof; if its external
prerequisites are unavailable, the successor should defer this candidate rather
than land it on shell syntax alone.

## Deferred families

C-05 defers a whole typed signal-ingestion boundary because consumers differ in
payload, legacy naming, deletion, ordering, and effects.

C-06 defers failure/reclaim unification because teardown sites have different
seat, thread, pane, lease, provenance, and retry authority.

C-07 defers deadline-loop unification because clock and exemption rules are
behavioral policy.

C-08 defers atomic-publication centralization because temporary naming,
serialization, collision behavior, execution side, and attribution differ.

C-09 defers a cross-harness library because the maintained scripts are
standalone programs with different globals, evidence, cleanup, and metering.

C-10 leaves historical admitted harnesses unchanged; no epic is recommended
unless evidence-retention policy changes.

C-11 defers a declarative hook schema across generation, merge, legacy upgrade,
provider matchers, and idempotence tests.

C-12 defers broad scheduler test builders until production and test modules are
decomposed by subsystem.

C-13 leaves assignment construction explicit until a third provider
demonstrates a real common policy.

C-14 preserves independent adapter compatibility assertions because their
repetition is regression evidence.

## Verification performed

A structural check over `inventory.md` reported:

```text
rows=14 small=4 too_large=10 unique=14
```

An explicit C-01 through C-14 presence check reported no missing identifiers.

The source diff check was limited to:

```text
crates/lisa-plugin
crates/lisa-cli
crates/lisa-core
```

It returned no changed paths. `git status --short` showed Lisa-managed ticket,
work-artifact, and provenance activity only; existing concurrent work was not
altered, staged, cleaned, or committed by this attempt.

## Test coverage assessment

No Rust or shell implementation changed, so workspace tests and clippy would
not provide a ticket-specific implementation signal and were not run.

The inventory was checked for completeness, uniqueness, exact classification
labels, and source-tree non-mutation. Candidate selection was grounded in named
existing regression seams rather than speculative ease.

`T-038-03-02` still must run its focused tests, `cargo test --workspace`, and
clippy, and must honor its WASM/clean-gate prerequisite. This review does not
claim those future implementation checks have passed.

## Open concerns and limitations

- “Every repetition site” is interpreted at the semantic-family level within
  the story’s named scheduler/adapter/harness/writing scope, not as every
  repeated token in the 34,000-line workspace.
- `lib.rs` continues to combine production scheduling and extensive historical
  regressions; the inventory deliberately does not turn that fact into a local
  cleanup.
- C-01 touches several consumers when landed even though the abstraction is
  pure. The successor should abandon it if the diff begins absorbing scanner
  behavior.
- C-04 has the heaviest environmental proof requirement of the small set.
- Trait defaults in C-02/C-03 are valuable only while native providers really
  share those policies; future providers must override rather than inherit by
  accident.
- The concise `inventory.md` is an auxiliary attempt artifact; the complete
  candidate list is duplicated in required `design.md` so acceptance does not
  depend on auxiliary-artifact publication behavior.

## Critical issues

None found.

There are no uncommitted ticket-owned source files because this ticket changed
no source. No `lisa commit-ticket` call was required or made. No ordinary Git
index or commit command was used.

## Final handoff

Only C-01 through C-04 are eligible inputs to `T-038-03-02`. C-05 through C-14
must remain in place and be named as deferred/larger repetition in the final
release-readiness report.

Review is complete. Remain on `T-038-03-01`; Lisa owns Review admission, Done
publication, completion commit, and seat release.
