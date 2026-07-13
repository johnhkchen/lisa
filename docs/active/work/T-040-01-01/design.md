# Design: Review disposition emission contract

## Decision to make

The workflow needs one stable machine-readable Review output that represents
either permission to complete or a blocking result with an actionable reason.
The contract must be precise enough that T-040-01-02 can reject contradictory
documents without guessing, while remaining simple for an agent to emit.

## Option 1: add fields to `review.md` frontmatter

The Review document could gain YAML frontmatter such as `disposition: pass` and
`reason: null`.

Advantages:

- keeps all review information in one artifact;
- remains readable without opening a companion file;
- follows the ticket/story markdown convention.

Costs:

- contradicts the acceptance criterion's request for a disposition file
  alongside `review.md`;
- forces a parser to separate prose markdown from metadata;
- makes malformed frontmatter and prose-only reviews harder to distinguish;
- couples machine admission to the structure of the human handoff.

Rejected because the story intentionally asks for a separate structured signal.

## Option 2: generic `disposition.json`

The companion file could be named `disposition.json` and contain a tagged JSON
object.

Advantages:

- short filename;
- direct JSON parsing;
- easy to emit and inspect.

Costs:

- the name does not identify which lifecycle decision it represents;
- future attempt, assignment, or completion dispositions could collide;
- directory listings do not visibly pair it with Review.

Viable, but weaker as a durable cross-ticket contract.

## Option 3: `review-disposition.json`

The companion file is named `review-disposition.json`. Every valid document has
exactly the conceptual shape `{disposition, reason}`:

```json
{"disposition":"pass","reason":null}
```

or:

```json
{"disposition":"block","reason":"Explain the blocking issue and required action."}
```

Advantages:

- explicitly scoped to Review;
- visibly paired with `review.md`;
- advertises the serialization format;
- provides a fixed, unsurprising path for the parser and plugin;
- retains the same keys in both variants.

Costs:

- slightly longer filename;
- requires the documentation to state cross-field validation rules.

Chosen because its specificity reduces ambiguity at the scheduler boundary.

## Reason representation alternatives

### Omit `reason` on pass

An optional key makes the passing document compact, but the ticket describes a
shape containing `reason`. Omission also creates two structural schemas and
leaves ambiguity between deliberately absent and accidentally forgotten.

Rejected.

### Use an empty string on pass

This keeps the value type uniform, but empty strings can hide whitespace and
normalization questions. They are less explicit than JSON's native absence
value and make “block with empty reason” superficially similar.

Rejected.

### Use `null` on pass and a non-empty string on block

This retains both keys and makes the variants structurally explicit. It maps
naturally to Rust's `Option<String>` for the successor parser. The parser can
then validate the relationship between the tag and optional value.

Chosen.

## Validation semantics established for successors

A valid pass has `disposition` exactly `"pass"` and `reason` exactly JSON
`null`. A valid block has `disposition` exactly `"block"` and `reason` as a
non-empty string explaining why completion must not proceed and what needs
attention.

The workflow will explicitly say that pass with a reason and block with null,
an omitted reason, or an empty reason are invalid. This directly settles the
contradictory cases named by T-040-01-02.

The contract does not decide behavior for unknown extra JSON keys. That is a
parser strictness choice for T-040-01-02; the emitted canonical examples contain
only the two named fields.

## Placement in the Review instructions

The new instruction belongs after the existing description of `review.md` and
before the instruction to remain on the ticket. That ordering reflects agent
execution:

1. self-assess and write the human handoff;
2. write the machine disposition alongside it;
3. stop and let Lisa process both artifacts.

The Review section will name both artifact paths. The disposition instruction
will use exact one-line JSON examples to minimize formatting mistakes in agent
output.

The “After writing `review.md`” sentence will become “After writing both Review
artifacts” so the stop condition cannot be read as permitting the disposition
to be skipped.

## Embedded contract synchronization

The repository maintains the current workflow twice:

- project truth at `docs/knowledge/rdspi-workflow.md`;
- outgoing embedded data at `crates/lisa-cli/data/rdspi-workflow.md`.

Both Review sections will receive identical text. `templates.rs` continues to
embed the data file with `include_str!`; duplicating the prose inside Rust would
create a third source and is not warranted.

The existing template test will be strengthened to assert:

- the Review phase is embedded;
- `review-disposition.json` is embedded;
- both canonical JSON documents are embedded;
- the reason validity instruction is embedded;
- generated `CLAUDE.md` still contains the workflow pointer.

The generated file itself should not duplicate the full disposition prose. Its
existing design intentionally points to the injected workflow, and the test can
exercise the pointer plus embedded body as the complete contract.

## Test design

A single focused test named for the Review disposition contract will inspect
`RDSPI_WORKFLOW`. Exact-string assertions protect filename and JSON shape from
unintentional drift. A generated Rust-project `CLAUDE.md` assertion protects the
link from the scaffold to that embedded contract.

The existing broad phase-name test remains useful and gains `Review`. It should
not be the only disposition test because phase presence does not prove schema
presence.

After edits:

- compare the two workflow files byte-for-byte;
- run the focused template tests;
- run all `lisa-cli` tests if time permits;
- run formatting checks for the Rust assertion changes;
- inspect the exact diff for ticket ownership.

## Compatibility

No Rust public API changes. No runtime parser or scheduler behavior changes.
Fresh `lisa init` installations receive the new embedded contract. Existing
projects with recognized Lisa workflow templates can be upgraded by existing
safe-init logic. User-modified workflow files continue to be preserved by that
logic.

Legacy embedded workflow constants remain unchanged so they can still identify
older known templates. The new current body remains distinct from every legacy
body, which existing tests already enforce.

## Scope controls

This ticket will not:

- add `review-disposition.json` to completion transaction paths;
- add a parser or model to `lisa-core`;
- gate either plugin completion call;
- render blocked reasons in the UI;
- change ticket phase/status fields;
- modify historical workflow templates;
- publish attempt artifacts directly.

## Final decision

Adopt `review-disposition.json` with two canonical two-field JSON variants.
Require `reason: null` for pass and a non-empty actionable reason string for
block. Put identical instructions in the documented and embedded Review
sections, and enforce the emitted contract with exact assertions in
`templates.rs` tests.
