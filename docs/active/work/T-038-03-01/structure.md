# Structure: inventory-only spike

## Resulting attempt artifacts

This ticket creates only documentation in its private attempt directory:

- `research.md`: descriptive codebase map and repetition families.
- `design.md`: classification method, alternatives, and detailed decisions.
- `structure.md`: artifact and ownership blueprint.
- `plan.md`: ordered inventory production and verification steps.
- `inventory.md`: concise acceptance-facing candidate list.
- `progress.md`: implementation-phase execution record.
- `review.md`: final self-review and handoff.

Lisa will publish admitted artifacts to `docs/active/work/T-038-03-01/` after
lease verification. This attempt does not write that shared directory directly.

## Source tree changes

Created source files: none.

Modified source files: none.

Deleted source files: none.

Ticket frontmatter changes: none.

The ticket explicitly requires “nothing landed yet,” so implementation means
materializing and checking the inventory rather than editing Rust or shell code.

## Acceptance-facing inventory shape

`inventory.md` is the primary deliverable. It has four sections:

1. Scope and classification key.
2. Small demonstrated-value candidates.
3. Too-large candidates deferred to future epic scope.
4. Successor-ticket and verification notes.

Each candidate row contains:

- Stable candidate identifier (`C-01` through `C-14`).
- Repetition family name.
- Exact maintained sites or file/function boundaries.
- One of the two required classification strings.
- A one-line justification.
- A test or verification seam.

Detailed tradeoffs remain in `design.md`; the inventory stays compact enough to
drive `T-038-03-02` and the final release-readiness report.

## Candidate identity and ordering

Identifiers are organized by the first relevant subsystem, not by priority:

- C-01: scheduler signal filename parsing.
- C-02: native adapter clear-reset policy.
- C-03: native adapter review follow-up construction.
- C-04: deterministic harness event counting.
- C-05: whole scheduler signal scanner abstraction.
- C-06: scheduler failure and reclaim unification.
- C-07: scheduler timeout and liveness loop unification.
- C-08: atomic publication writing helper.
- C-09: maintained Zellij harness library.
- C-10: historical harness consolidation.
- C-11: declarative lifecycle-hook schema.
- C-12: scheduler test-fixture writers/builders.
- C-13: provider assignment construction.
- C-14: adapter compatibility assertions.

This ordering keeps the four selected small candidates visible first, followed
by the deferred boundary that the report must name.

## Small candidate boundaries

### C-01 file boundary

Potential successor modification:
`crates/lisa-plugin/src/lib.rs` only.

Potential internal interface:

```text
fn pane_signal_id(path: &Path, suffix: &str) -> Option<u32>
```

The helper owns filename grammar only. It must not read payloads, delete files,
admit leases, mutate state, or choose scan order. Consumers retain all those
responsibilities.

Focused test boundary: a table of `Path` values and expected pane ids, including
malformed and foreign names. Existing scanner tests remain unchanged.

### C-02 file boundary

Potential successor modification:
`crates/lisa-plugin/src/adapter.rs` only.

Potential interface change: supply the existing
`AgentAdapter::reset_strategy` method body as the trait default returning
`ResetStrategy::ClearHandshake`; remove identical native overrides.

The method remains overridable. The enum and scheduler match stay unchanged.

Focused test boundary: existing provider-specific dynamic-dispatch assertions.

### C-03 file boundary

Potential successor modification:
`crates/lisa-plugin/src/adapter.rs` only.

Potential interface change: supply the existing `AgentAdapter::follow_up` body
as a trait default that returns `FollowUp::TypeIntoPane(finish_up_prompt(...))`;
remove identical native overrides.

The method remains overridable. `FollowUpContext`, `FollowUp`, and prompt content
stay unchanged.

Focused test boundary: existing complete-prompt assertions for Claude and Codex.

### C-04 file boundary

Potential successor modification:
`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` only.

Potential internal interface:

```text
event_count <event-kind>
```

`event_count_is` and `event_count_at_least` would call it and retain their
different comparisons. The event file location and tab-delimited schema stay
unchanged.

Focused test boundary: ignored Rust integration wrapper plus the script’s final
PASS receipt.

## Deferred architecture boundaries

### Typed signal ingestion epic

Would own C-05 and may absorb C-01 only after the small helper proves useful.
It would define typed records, payload parsing, consumption guarantees, and poll
ordering. It must preserve legacy idle compatibility and attempt fencing.

### Explicit scheduler transition epic

Would own C-06 and C-07. It would need an invariant matrix for seats, threads,
leases, panes, provenance, retry, activity clocks, and human-awaiting gates.
This is module design, not helper extraction.

### Atomic publication epic

Would own C-08. It would define destination-relative temporary naming,
serialization responsibility, collision behavior, cleanup, host versus pane
execution, and error context before introducing a common utility.

### Harness platform epic

Would own C-09 and explicitly leave C-10 as historical evidence. It would need
to decide between a sourced shell library, a Rust harness driver, or a reusable
CLI fixture command. Standalone invocation and evidence retention are public
contracts of the harnesses.

### Declarative hook specification epic

Would own C-11. A typed specification would feed both initial JSON rendering
and user-preserving merge logic, with provider-specific matcher semantics and
golden upgrade tests.

### Scheduler module/test decomposition epic

Would own C-12 and potentially make C-06/C-07 tractable. Production boundaries
should be established before broad test-builder migration so helpers do not
freeze the current monolith.

### Provider assignment policy epic

Would own C-13 only if another provider demonstrates the need. Until then,
Claude context selection and Codex acknowledgement tagging remain explicit.

C-14 has no recommended epic today. Compatibility assertions should remain
independent until the compatibility layer itself is retired.

## Ownership and commit boundaries

The private phase and inventory artifacts are owned by this ticket but are not
source implementation units. Lisa owns their admission and final publication.

Because there are no ticket-owned source changes, this ticket must not invoke
`lisa commit-ticket` with source paths. The successor ticket will commit each
selected source unit with exact paths if and when it implements them.

No ordinary Git index operation is part of this structure.

## Verification structure

This spike uses documentation verification rather than workspace execution:

- Confirm all seven expected attempt artifacts exist.
- Confirm all candidate identifiers C-01 through C-14 appear in `inventory.md`.
- Confirm every row contains one of the exact required classifications.
- Confirm every row has a one-line justification.
- Confirm the four small candidates have named test seams.
- Confirm deferred candidates remain named for the report.
- Confirm no repository source path changed because of this ticket.
- Confirm pre-existing concurrent worktree changes are untouched.

`cargo test --workspace` and clippy are successor-ticket criteria after code is
changed. Running them here would not test an implementation because this spike
lands none.

## Ordering

1. Finish the descriptive research map.
2. Decide the classification gate and candidate set.
3. Fix the artifact and candidate structure here.
4. Plan materialization and checks.
5. Write `inventory.md` and `progress.md`.
6. Run structural verification and inspect Git status.
7. Write `review.md` and stop on this ticket.
