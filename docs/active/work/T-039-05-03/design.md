# Design: validate and directly regress publication isolation

## Objective

Lock the extracted atomic-publication boundary with direct tests. Make its
same-directory claim enforceable for every typed temporary policy, prove failure
and repeated-success atomicity, prove neighboring ticket paths cannot mix, and
pin provenance attribution without changing valid caller behavior.

## Option 1: tests only against current behavior

Direct tests could cover ordinary exact-temp replacement and rename cleanup while
leaving temporary strings unconstrained.

- This would add useful module coverage.
- It would preserve all current code.
- It could not honestly prove hostile-path rejection.
- A `..` or absolute temporary would still escape the sibling directory.
- Cross-ticket isolation would remain an assumption at callers.

Decision: reject because the acceptance criterion explicitly requires rejection
and no cross-ticket mixing.

## Option 2: sanitize hostile names

The resolver could strip separators, collapse components, or replace hostile
characters.

- Publication would stay within the parent.
- Callers might receive a surprising filename different from their policy.
- Collisions could be introduced by normalization.
- Operator diagnosis would lose the invalid input.
- Quotes and shell metacharacters are legitimate filesystem characters.

Decision: reject. Security boundaries should reject ambiguous policy rather than
silently reinterpret it.

## Option 3: validate one normal path component

Resolve the typed policy string, then require its `Path::components()` sequence
to contain exactly one `Component::Normal` and no second component.

- Empty, `.`, `..`, separator-bearing, rooted, and prefixed paths are rejected.
- Spaces, quotes, dollar signs, semicolons, and Unicode remain valid.
- All current callers already synthesize one safe component.
- Validation occurs before filesystem or shell effects.
- One check applies equally to all three typed policies.
- The same rule makes the sibling-temp documentation mechanically true.

Decision: choose this option.

## Error model

Change path resolution from an infallible value to `Result`.

- Return a stable `invalid publication temporary name ...` diagnostic.
- Include the invalid name for operator diagnosis.
- Never include publication body bytes in the error.
- Rust publication naturally propagates this through its existing `Result`.
- Shell command rendering must also return `Result<String, String>`.
- `State::shell_readiness_probe` already returns `Result`, so it can propagate it.
- Valid caller output and site-specific I/O errors remain unchanged.

## Rejected: validate only `Exact`

`Exact` is the easiest traversal vector, but prefix strings in `Nonce` and
`AttemptNonce` can also contain separators or roots.

Decision: reject. The invariant applies after every policy resolves.

## Rejected: canonicalize paths

Canonicalization requires existing paths, introduces filesystem I/O before the
write, and has race and symlink semantics unrelated to lexical sibling naming.

Decision: reject. A lexical single-component rule is sufficient and deterministic.

## Direct Rust regression design

Add inline tests in `publication.rs`, where private resolution behavior is visible.

### Repeated success

- Use a hostile-but-valid directory and exact temporary name.
- Seed an old destination.
- Publish one complete body, then a different complete body.
- Assert the destination equals only the second body, not an append or duplicate.
- Assert the exact temporary is absent.
- Assert the directory contains exactly one final entry.

### Rename failure

- Occupy the destination with a directory containing a sentinel.
- Publish through an exact temporary.
- Assert the publication returns the site-specific publish error.
- Assert the destination directory and sentinel are unchanged.
- Assert no partial body or temporary remains beside it.

### Cross-ticket traversal

- Create adjacent `T-A` and `T-B` directories.
- Seed each canonical destination with distinct bytes.
- Request ticket A publication with `../T-B/research.md` as its temp.
- Assert rejection before I/O.
- Assert both ticket files remain byte-identical.
- Assert no extra entry or temporary exists.

### Policy coverage

- Test relative traversal for `Exact`.
- Test slash-bearing traversal for `Nonce`.
- Test an absolute value for `AttemptNonce` or `Exact`.
- Test shell command rendering rejects traversal too.
- Test a valid name containing quotes and metacharacters remains accepted.

## Provenance regression design

Extend the core provenance tests with one explicit cross-ticket attribution test.

- Append a record for ticket A and then a separately leased record for ticket B.
- Use a hostile-but-valid ledger path.
- Parse each JSONL line independently.
- Assert each outer `ticket_id` equals its nested lease ticket ID.
- Assert attempt IDs and outcomes remain paired with the correct ticket.
- Assert exactly two complete newline-terminated lines exist.
- Assert no temporary publication files appear because provenance is append-only.

This complements the existing failed-target test. It does not route provenance
through the replacement helper or change the schema.

## Compatibility

- All five current callers pass valid one-component names.
- Their serialization, collision, and error behavior is unchanged.
- Shell command text is unchanged for valid requests.
- No public API changes because the module is crate-private.
- No dependency or manifest change is required.
- Existing characterization remains the compatibility bracket.

## Commit organization

Two meaningful ticket-owned units are expected:

1. plugin boundary validation plus direct boundary regressions;
2. provenance attribution regression.

Each unit will use `lisa commit-ticket` with one exact repository-relative path.
The ordinary index will remain unused.

## Verification

- Run direct publication module tests.
- Run the predecessor publication-site characterization tests.
- Run all core provenance tests.
- Run plugin and core package tests.
- Run workspace tests.
- Run formatting check and Clippy with warnings denied.
- Run `just check`.
- Confirm both ticket-owned paths are clean after isolated commits.

## Decision summary

Validate every resolved temporary as exactly one normal filename component,
return an error before effects, and add direct filesystem regressions for
replacement, cleanup, traversal isolation, and provenance record attribution.
