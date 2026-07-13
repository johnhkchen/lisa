# Research: Review disposition emission contract

## Ticket boundary

T-040-01-01 is the first ticket in story S-040-01. Its only product boundary is
the agent-facing Review contract: agents must write a machine-readable result in
addition to `review.md`. Parsing belongs to T-040-01-02 and scheduler gating
belongs to T-040-01-03.

The acceptance criterion names two representations of the workflow:

- `docs/knowledge/rdspi-workflow.md`, the checked-in project documentation;
- the workflow embedded by `crates/lisa-cli/src/templates.rs`, used by generated
  projects and injected agent context.

It also requires a template test proving the instruction survives embedding.

## Current workflow documentation

`docs/knowledge/rdspi-workflow.md` defines all six RDSPI phases. The Review
section currently requires only `review.md`. It describes the review's prose
content, tells the agent to remain on the ticket, and identifies the artifact as
`docs/active/work/{ticket-id}/review.md`.

Nothing in the current Review section distinguishes a passing review from a
blocking review. A reviewer can describe critical concerns in prose, but there
is no stable field or companion artifact for automation to inspect.

The Implement section already establishes that attempt artifacts are written in
the active work directory conceptually, while the assignment redirects this
attempt to its private `.lisa/attempts/.../work/` staging directory. Lisa later
publishes admitted artifacts. The disposition therefore belongs beside
`review.md` and should follow the same path substitution behavior.

## Embedded workflow path

`crates/lisa-cli/src/templates.rs` declares:

```rust
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");
```

The embedded text is therefore physically maintained at
`crates/lisa-cli/data/rdspi-workflow.md`. The data copy currently matches the
project documentation's Review section and also mentions only `review.md`.

`generate_claude_md` does not concatenate the workflow body into the generated
`CLAUDE.md`. It generates a pointer stating that the workflow lives at
`docs/knowledge/rdspi-workflow.md` and is injected automatically. During `lisa
init`, `RDSPI_WORKFLOW` is written to that documentation path. Thus the embedded
constant is the outgoing workflow contract even though the generated
`CLAUDE.md` contains only its pointer.

`LEGACY_RDSPI_WORKFLOWS` contains earlier exact templates accepted during safe
upgrade. The current outgoing template is not listed there. Existing init logic
compares known legacy content and can replace a recognized older template with
the new `RDSPI_WORKFLOW`; no legacy file needs rewriting for this ticket.

## Existing template tests

The `#[cfg(test)]` module in `templates.rs` contains
`test_rdspi_workflow_embedded`. It checks that `RDSPI_WORKFLOW` contains the
workflow title and the first five phase names, but it does not currently assert
Review or any Review artifact contract.

`test_generate_claude_md_rust` checks that generated `CLAUDE.md` points to the
workflow path. This establishes the generated-document half of the injection
chain. A disposition assertion against `RDSPI_WORKFLOW` can establish that the
body reached through that pointer includes the new instruction.

Numerous tests in `init.rs` exercise writing and upgrading the embedded workflow.
Those tests compare against `templates::RDSPI_WORKFLOW`, so a content change is
normally absorbed without fixture rewrites. The legacy arrays preserve upgrade
recognition for older exact bodies.

## Downstream parser expectations

T-040-01-02 explicitly consumes the shape settled here. Its acceptance cases
are pass, block with reason, missing file, malformed JSON, block without reason,
and pass with a block reason. This means the contract must make both the field
names and reason semantics explicit; merely showing `{disposition, reason}` is
insufficient.

The downstream model will need to distinguish these valid documents:

- pass, with no blocking reason;
- block, with a human-actionable reason.

It must reject pass plus a reason and block without a reason. JSON `null` is a
natural explicit representation of “no reason” while retaining the required
`reason` key in both variants. A non-empty JSON string is the corresponding
representation for block.

No parser or disposition type exists yet in `lisa-core`; this ticket must not
create one. No plugin completion site reads a companion file yet; this ticket
must not alter completion behavior.

## Naming constraints

The repository has no existing disposition filename. Search results mention the
concept in E-040, S-040-01, and later completion epics, but do not pin a name.
The fixed filename therefore has to be established here for successors.

`review-disposition.json` is scoped to the Review phase and avoids collision
with future dispositions from other lifecycle stages. It is also visibly paired
with `review.md` in directory listings and identifies its serialization format.

## Repository and transaction constraints

The ordinary worktree already contains Lisa-owned phase changes in
`docs/active/tickets/T-040-01-01.md` and another concurrently assigned ticket.
Those paths are not ticket-owned implementation changes and must remain
untouched.

Ticket-owned product changes are expected in:

- `docs/knowledge/rdspi-workflow.md`;
- `crates/lisa-cli/data/rdspi-workflow.md`;
- `crates/lisa-cli/src/templates.rs`.

The first two are two maintained copies of the current contract. The third is
the test site. All must be passed as exact repository-relative includes to
`lisa commit-ticket`; the attempt-private RDSPI artifacts must not be included.

## Verification surface

The narrow unit test target is the template module within `lisa-cli`. Running
the named template test verifies the embedded constant. Running the complete
`lisa-cli` test suite gives broader confidence that init migration and generated
project behavior still accept the changed outgoing template.

A direct comparison of the two workflow files can detect accidental drift. A
text search can verify the fixed filename, both JSON examples, and the semantic
rules occur in documentation, embedded data, and test assertions.

## Constraints and assumptions

- JSON is required by the ticket.
- The document always contains both `disposition` and `reason` keys.
- `disposition` is exactly `"pass"` or `"block"`.
- A pass uses JSON `null` for `reason`.
- A block uses a non-empty string for `reason`.
- Agents write the disposition alongside `review.md` before stopping.
- Publishing, parsing, validation, UI rendering, and scheduler gating are outside
  this ticket.
- Existing workflow phase/status automation remains unchanged.
- Legacy workflow templates remain historical inputs rather than being edited.
