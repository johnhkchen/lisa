# Review: Disposition parse model

## Disposition

Pass.

The implementation satisfies T-040-01-02's acceptance criterion and is ready
for Lisa's completion transaction.

## Summary

This ticket adds a fail-closed parser to `lisa-core` for the structured Review
disposition emitted under the T-040-01-01 contract.

The parser transforms the filesystem artifact into exactly one typed domain
outcome:

- `Pass` for explicit valid approval;
- `Block { reason }` for explicit valid refusal;
- `Invalid { reason }` for everything untrustworthy or contradictory.

This makes the safety boundary visible to the dependent scheduler work.
Only an exact `Pass` match can authorize completion; parser and input failures
are ordinary non-passing values rather than errors that callers might default.

## Files changed

Created `crates/lisa-core/src/disposition.rs`.

The new module contains:

- the public `ReviewDisposition` enum;
- the public `parse_review_disposition` filesystem API;
- private document validation and invalid-construction helpers;
- nine focused unit tests.

Modified `crates/lisa-core/src/lib.rs`.

The sole change registers the new public `disposition` module.

No files were deleted.
No dependency or manifest changes were required.

## Contract assessment

The parser consumes the predecessor's canonical shape.

A passing document must be a JSON object with a string `disposition` equal to
`pass` and an explicitly present null `reason`.

A blocking document must be a JSON object with a string `disposition` equal to
`block` and an explicitly present, non-blank string `reason`.
The block reason is retained without rewriting it.

The implementation refuses:

- missing paths;
- all other file read failures;
- invalid UTF-8;
- malformed JSON;
- non-object JSON roots;
- absent `disposition`;
- non-string `disposition`;
- unknown disposition names;
- absent `reason`;
- pass with any non-null reason;
- block with null, non-string, empty, or whitespace-only reason.

Every refusal returns `Invalid`, never `Pass` and never `Block`.

## Acceptance criteria trace

“A new lisa-core module (sibling to provenance.rs)” is met by
`crates/lisa-core/src/disposition.rs` and its public declaration beside
`provenance` in `lib.rs`.

“parses the T-040-01-01 shape” is met by canonical pass and block tests using
the exact two-field JSON contract.

“unit tests cover pass” is met by `parses_pass`.

“block-with-reason” is met by `parses_block_with_reason`, including exact reason
preservation.

“missing file” is met by `missing_file_is_invalid`.

“malformed JSON” is met by `malformed_json_is_invalid`.

“block without reason” is met by `block_without_reason_is_invalid`, which
covers absent, null, empty, and whitespace-only representations.

“pass with block reason” is met by `pass_with_block_reason_is_invalid`.

“last three all resolve to a non-passing Invalid variant, never Pass” is pinned
by the shared `assert_invalid` helper, which specifically matches `Invalid` and
fails for either positive variant.

## Test evidence

`cargo test -p lisa-core disposition` passed all 9 focused tests.

`cargo test -p lisa-core` passed all 169 core tests and doc tests.

`cargo test --workspace` passed the complete native workspace suite:

- 276 `lisa-cli` tests;
- 169 `lisa-core` tests;
- 333 `lisa-plugin` tests;
- doc tests.

`cargo fmt --all --check` passed.
The scoped diff passed `git diff --check` before commit.

## Commit and ownership review

The source unit was committed exclusively through `lisa commit-ticket`.

Commit:

```text
150b2e12e4dd9040bc3782bd0dc524b71109aa25
```

Subject:

```text
Add fail-closed review disposition parser
```

`git show` confirms the commit contains exactly:

- `crates/lisa-core/src/disposition.rs`;
- `crates/lisa-core/src/lib.rs`.

No ordinary `git add` or `git commit` was used.
Neither ticket-owned source path remains staged, modified, or untracked.
Pre-existing and concurrent repository changes were left untouched.

## Design review

Returning the three-way outcome directly is appropriate for the downstream
authorization boundary.
It keeps `Invalid` alongside `Pass` and `Block`, forcing an exhaustive match.

Carrying a reason on `Invalid` adds diagnostic value without weakening the
simple authority rule.
The next ticket can surface malformed/missing evidence to operators without
re-parsing serde or I/O errors.

Using `serde_json::Value` internally is proportionate for this two-field
relationship schema.
It preserves the important distinction between a missing reason and explicit
null and keeps contradiction rules visible in one match.

Unknown extra keys are accepted.
The established contract fixes the required fields and their relationship but
does not declare a closed/no-extension schema.
Required fields, values, types, and contradictions remain strictly validated.

## Gaps and limitations

This ticket does not gate scheduler completion.
That is intentionally deferred to T-040-01-03, which names the two plugin
completion sites and will consume this core API.

The invalid reason strings are diagnostics, not a stable machine-readable error
taxonomy.
No current acceptance criterion requires downstream branching by invalid cause.
If such branching becomes necessary, a typed invalid-reason enum would be a
compatible future refinement of intent but would change the public model.

The tests do not simulate a permissions error or invalid UTF-8 file separately.
Both share the same `read_to_string` failure branch as the covered missing-file
case; the critical fail-closed behavior is exercised.

No WASM-specific build was run because this additive core module is tested
through all native workspace consumers and T-040-01-02 requires the core parser,
not a rebuilt release artifact. The later E-040 rebuild ticket owns full native
and WASM release gates.

## Open concerns

No blocking concern was found.

The downstream integration must continue to use an explicit match on
`ReviewDisposition::Pass`; it must not treat `Block` or `Invalid` as completion
eligible. That is a responsibility of T-040-01-03 rather than unfinished work
in this parser.

