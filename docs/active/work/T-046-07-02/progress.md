# Progress: purpose-first docs and templates

## Status

Implementation is complete.

All ticket-owned source changes are committed through `lisa commit-ticket`.

All planned verification passed.

No ticket-owned source path is staged, modified, or untracked.

## Completed: repository and ownership checks

- Read `CLAUDE.md`, the active ticket, and the full RDSPI workflow.
- Confirmed the active ticket advanced to Implement through Lisa-managed state.
- Inspected the existing README, template generator, setup guide, and tests.
- Confirmed all three ticket-owned source paths began clean.
- Identified unrelated modified and untracked repository entries.
- Left the ordinary Git index untouched.
- Kept phase artifacts in the generation-1 attempt work directory.

## Completed: README purpose-first opening

Changed `README.md`.

The old opening prose was:

> DAG-driven concurrent task scheduling for AI-assisted development.

The new first prose paragraph says Lisa runs coding agents like Claude Code and
Codex through the user's ticket board.

It also states the operator benefit: the user does not have to approve every
step by hand.

The purpose paragraph appears on lines 5-6.

The first mechanism-oriented prose occurs later.

The installation path and detailed `What It Does` section were preserved.

## Completed: README review trail

Added one paragraph after the six-phase artifact explanation.

It names the append-only attempt ledger.

It says the ledger records each run.

It names the completion journal.

It says the journal ties finished tickets to commits.

It says each ticket keeps its work documents.

The paragraph describes current behavior and adds no feature promise.

## README source commit

Committed with the isolated ticket transaction:

```text
lisa commit-ticket \
  --ticket-id T-046-07-02 \
  --message "T-046-07-02: lead README with Lisa's purpose" \
  --include README.md
```

Commit:

```text
e90ae071512d1fd4c268d72b465d8fc2a984db68
```

The commit contains only `README.md`.

Its stat is six insertions and one deletion.

## Completed: generated AGENTS.md

Changed `crates/lisa-cli/src/templates.rs`.

Kept `AGENTS_MD` as a public static string.

Placed the shared purpose paragraph immediately after `# AGENTS.md`.

Placed the shared agent contract after the purpose paragraph.

The contract says the agent takes one ticket through every RDSPI phase, leaves a
reviewable record, and waits for Lisa to confirm completion.

Kept the `CLAUDE.md` source-of-truth pointer after the stable orientation copy.

Kept the RDSPI workflow reference.

Kept project Build and Test and Source Layout material out of `AGENTS.md`.

Updated the source comment to describe the stable purpose/contract paragraphs.

## Completed: generated CLAUDE.md

Placed the exact same purpose paragraph immediately after `# CLAUDE.md`.

Placed the exact same agent contract after it.

Kept `## Project` and the detected name/type line after both paragraphs.

Preserved conditional Build and Test rendering.

Preserved conditional Source Layout rendering.

Preserved directory conventions and the injected-workflow reference.

No generator signature or caller changed.

## Completed: template assertions

Added `test_generated_agent_context_opens_with_purpose_and_contract`.

The test constructs a representative Rust project.

It generates the Claude context and reads the static Codex context.

For both outputs it asserts:

- the correct H1 exists;
- the exact purpose paragraph exists;
- the exact contract exists;
- the H1 precedes purpose;
- purpose precedes contract;
- contract precedes the later project/pointer section.

Existing pointer and no-duplicated-project-body assertions remain in place.

## Completed: setup-guide provenance

Changed `crates/lisa-cli/src/setup_guide.rs`.

Preserved the sibling ticket's purpose-first guide preamble.

Added the same README review-trail sentence immediately after it.

The sentence appears for both initialized and uninitialized projects because it
lives in the unconditional guide header.

The seven guide sections and their numbering are unchanged.

Added `test_guide_names_the_existing_review_trail`.

The test checks the attempt ledger, completion-to-commit relationship, and work
documents as separate required substrings.

## Compiled copy source commit

Committed with the isolated ticket transaction:

```text
lisa commit-ticket \
  --ticket-id T-046-07-02 \
  --message "T-046-07-02: orient generated agent context" \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/setup_guide.rs
```

Commit:

```text
6633cf44fd78e5c83ecc7c43752589135adbe47a
```

The commit contains exactly the two included Rust files.

## Verification completed

### Formatting

```text
cargo fmt --all -- --check
```

Passed before the compiled-copy commit.

Passed again during the final audit.

### Focused template test

```text
cargo test -p lisa-cli templates::tests::test_generated_agent_context_opens_with_purpose_and_contract
```

Passed: one selected test, zero failures.

### Focused setup-guide test

```text
cargo test -p lisa-cli setup_guide::tests::test_guide_names_the_existing_review_trail
```

Passed: one selected test, zero failures.

### CLI package suite

```text
cargo test -p lisa-cli
```

Passed with no failures.

The command covered 14 library tests, 309 binary unit tests, and all enabled CLI
integration tests.

The real-Zellij boundary test remained ignored by its existing environment gate.

### Workspace suite

```text
cargo test --workspace
```

Passed with exit status zero and no failed test.

This included the CLI, core, and plugin suites plus enabled integration and doc
tests.

### Text and diff checks

`git diff --check` passed for each source unit.

Final `cargo fmt --all -- --check` passed.

`rg -n` confirmed the README purpose paragraph is at line 5 and the review-trail
copy is in `What It Does`.

The template test provides executable ordering coverage for both generated
context files.

The existing help-surface order test also passed within the CLI suite.

## Deviations from plan

No implementation scope changed.

The plan described a small dedicated README position script.

Direct line-number inspection with `rg -n` and the README diff provided the same
static verification, so no repository test harness was added for one Markdown
paragraph.

The two planned meaningful source commits were created exactly as designed.

## Final ownership audit

The following paths are clean in the worktree and ordinary index:

- `README.md`;
- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/src/setup_guide.rs`.

`git show --name-only` confirms the first ticket commit owns only README.

It confirms the second ticket commit owns only the two Rust files.

Unrelated Lisa lifecycle files, other tickets, other work documents, release
scripts, and planning files remain outside both ticket commits.

## Remaining

- Write the Review handoff.
- Write the exact Review disposition JSON.
- Remain on `T-046-07-02` while Lisa handles completion publication.
