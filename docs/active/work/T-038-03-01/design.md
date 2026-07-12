# Design: classify repetition by demonstrated cleanup value

## Decision

Produce a bounded, evidence-linked inventory with exactly the two ticket-required
classifications:

1. `small demonstrated-value cleanup`
2. `too-large → future epic`

The final inventory will identify a repetition family, name its exact sites,
state a one-line justification, and name the verification seam. It will not
modify source. “Too-large” includes repetition that is harmless or intentional
today when removing it safely would require a broader refactor; the rationale
will say whether the future epic is recommended or merely the correct scope if
the project later chooses to remove it.

## Classification test

A candidate is small only when all of the following hold:

- The repeated rule is semantically identical at every selected site.
- A narrow helper or trait default can express it without new runtime coupling.
- The edit stays within one maintained source or fixture file.
- Provider-specific and lifecycle-specific choices remain explicit.
- A focused existing test or one focused added test can prove the contract.
- The change does not reorder scheduler effects or signal consumers.
- The expected review surface is local and mechanical.

A family is too large when any of the following apply:

- Similar-looking code has different authority, timing, payload, or cleanup
  semantics.
- Extraction crosses independently runnable shell fixtures.
- Removal requires a declarative schema or module decomposition.
- The repetition is part of compatibility or no-op proof.
- The most honest cleanup would touch many historical tests at once.
- A safe proof requires multiple integration layers rather than one focused test.

## Option A: count duplicated lines

This approach would run a clone detector or compare normalized text and rank the
largest matches.

Advantages:

- Mechanical and reproducible.
- Finds copy/paste that a semantic read can miss.
- Produces simple size numbers.

Disadvantages:

- Scheduler branches repeat syntax while preserving different state-machine
  rules.
- Adapter symmetry can be an explicit provider contract.
- Test fixtures intentionally repeat complete state to remain readable.
- Shell harnesses use the same function names but close over different worlds.
- Line count does not demonstrate maintenance value.

Decision: reject as the primary classifier. Textual searches remain supporting
evidence, not the decision rule.

## Option B: classify every repeated token or call sequence

This approach would list every recurring `read_dir`, `remove_file`, YAML write,
hook command, and state lookup separately.

Advantages:

- Maximally exhaustive at the statement level.
- Makes no judgment about family boundaries.

Disadvantages:

- Produces dozens of overlapping entries.
- A single cleanup could appear multiple times.
- The successor ticket would not know which unit is independently landable.
- Reviewers would have to reconstruct semantic families themselves.

Decision: reject. The inventory unit is a repeated policy or construction
pattern, with all material sites named under that family.

## Option C: semantic families with a strict smallness gate

This approach groups repetition by the contract it appears to express, then
tests whether that contract can be centralized locally and proved narrowly.

Advantages:

- Matches the story’s demonstrated-value language.
- Separates provider differences from truly common behavior.
- Preserves state-machine effect ordering.
- Gives `T-038-03-02` discrete candidate units.
- Makes deferred architectural work explicit.

Disadvantages:

- Requires judgment rather than a single numeric threshold.
- “Every repetition site” is bounded by the surveyed areas and family model.
- A future code change can make the inventory stale.

Decision: choose Option C. Record the survey boundary and exact locations so
the judgment is auditable.

## Candidate decisions

### C-01: pane signal filename parsing

Classification: `small demonstrated-value cleanup`.

Sites: the pane-id parse chains in heartbeat, process-start, shell-ready,
acknowledgement, awaiting, transition, and error signal consumers in
`crates/lisa-plugin/src/lib.rs`.

Decision rationale: `pane-<u32>.<suffix>` parsing is identical policy and can
be a pure local helper while each consumer keeps its own payload, deletion,
ordering, and state transition.

Proof seam: add table-driven unit coverage for valid, wrong-prefix,
wrong-suffix, non-numeric, and non-UTF-8 names; retain the existing consumer
tests.

### C-02: native adapter clear-reset policy

Classification: `small demonstrated-value cleanup`.

Sites: identical `reset_strategy` implementations for `ClaudeCodeAdapter` and
`CodexAdapter` in `crates/lisa-plugin/src/adapter.rs`.

Decision rationale: both providers intentionally share the exact policy, and a
trait default removes a third place a future adapter could accidentally diverge
without opting out.

Proof seam: existing Claude and Codex reset assertions, plus the mixed resolver
tests, already exercise dynamic dispatch; retain them unchanged or add one
default-specific test if implementation clarity requires it.

### C-03: native adapter review follow-up

Classification: `small demonstrated-value cleanup`.

Sites: identical `follow_up` implementations for the two native adapters.

Decision rationale: construction from `FollowUpContext` is byte-for-byte common
and a trait default preserves the explicit override seam for future providers.

Proof seam: existing `native_follow_up_is_type_into_pane` and
`codex_follow_up_is_typed_into_live_tui` compare the complete returned prompt.

### C-04: deterministic harness event counting

Classification: `small demonstrated-value cleanup`.

Sites: `event_count_is` and `event_count_at_least` in
`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.

Decision rationale: file fallback and `awk` counting are identical and can move
to a local `event_count` primitive without coupling harnesses.

Proof seam: the ignored `real_zellij_delivery_boundary` integration test runs
all exact and lower-bound assertions against the script.

### C-05: whole signal scanner abstraction

Classification: `too-large → future epic`.

Sites: the eight `check_*_signals` loops in `lib.rs`.

Decision rationale: payload admission, legacy naming, delete timing, ordering,
and effect semantics differ; an iterator/callback framework would be a scheduler
rewrite beyond the pure parser cleanup.

Future epic: a signal ingestion boundary with typed signal records, explicitly
preserving poll order and attempt admission.

### C-06: scheduler failure/reclaim unification

Classification: `too-large → future epic`.

Sites: assignment delivery failure, assignment recovery failure, startup
failure, startup recovery failure, error signal, session timeout, and stale
thread reclamation paths.

Decision rationale: similar teardown verbs operate under different lease,
seat, thread, pane, provenance, and retry authority.

Future epic: model failure transitions as explicit typed outcomes only after a
state-machine map and invariant test matrix exist.

### C-07: timeout/liveness loop unification

Classification: `too-large → future epic`.

Sites: acknowledgement, transition, review, health, session, and stale-thread
deadline paths.

Decision rationale: the clocks and exemptions are behavior, not boilerplate;
centralizing traversal would obscure active-session and awaiting-human rules.

Future epic: a clock-injected deadline evaluator with per-policy actions and
cross-policy regression tests.

### C-08: atomic publication helper

Classification: `too-large → future epic`.

Sites: fresh launch, assignment, lease marker, admitted artifact, and shell
readiness publication.

Decision rationale: all use rename, but temporary naming, serialization,
filesystem side, collision behavior, and operator-facing errors encode distinct
security and attribution contracts.

Future epic: an audited atomic-publication utility only if those contracts can
be represented as typed options and hostile-path tests remain direct.

### C-09: shared maintained Zellij harness library

Classification: `too-large → future epic`.

Sites: repeated lifecycle and pane-control functions across the deterministic
and live shell harnesses.

Decision rationale: sourcing a common script couples independently executable
fixtures and their globals, packaging, evidence layouts, and failure cleanup.

Future epic: a versioned harness library or Rust driver after defining the
standalone execution and artifact-retention contract.

### C-10: historical harness consolidation

Classification: `too-large → future epic`.

Sites: admitted harnesses under `docs/active/work/T-021-01`, `T-031-03`, and
`T-033-03-02`.

Decision rationale: these are immutable investigation evidence, so editing them
would rewrite history rather than improve a maintained code path.

Future epic: none recommended; if evidence retention policy changes, migrate
copies into a separately versioned fixture area without rewriting the records.

### C-11: declarative hook schema

Classification: `too-large → future epic`.

Sites: literal hook JSON writers plus Claude and Codex merge enumerations in
`crates/lisa-cli/src/templates.rs`.

Decision rationale: centralization spans generation, user-preserving merge,
legacy upgrade matching, provider matcher differences, and many idempotence
tests.

Future epic: introduce typed hook specifications and render/merge from the same
data with golden compatibility fixtures.

### C-12: scheduler test-fixture writers

Classification: `too-large → future epic`.

Sites: repeated ticket frontmatter, signal-file, state, thread, seat, and lease
setup throughout the `lib.rs` test module.

Decision rationale: broad helper migration would create high churn and can hide
the different historical authority assumptions each regression pins.

Future epic: split scheduler tests by subsystem and add builders locally as
each module boundary is established.

### C-13: adapter assignment construction

Classification: `too-large → future epic`.

Sites: both adapters call `ticket_prompt` and implement `reuse_prompt`.

Decision rationale: Claude and Codex intentionally differ in context-file
selection and acknowledgement tagging, so the remaining shared call shape is
small and explicit rather than a demonstrated maintenance defect.

Future epic: only alongside adding a third provider, when a typed assignment
policy can be justified by another real implementation.

### C-14: adapter compatibility assertions

Classification: `too-large → future epic`.

Sites: tests that compare native adapter output with established free-function
output and repeat per-provider contract assertions.

Decision rationale: the repetition is the no-op and provider-parity proof;
deduplicating expected values would weaken regression independence.

Future epic: none recommended unless the compatibility free functions are
retired through a separately reviewed adapter migration.

## Scope decision

`T-038-03-02` should consider only C-01 through C-04. Each is local, has a
named proof seam, and does not alter stable behavior. C-05 through C-14 remain
in place. No implementation occurs in this spike.
