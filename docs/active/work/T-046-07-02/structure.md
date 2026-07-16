# Structure: purpose-first docs and templates

## Change set

Three repository files will be modified.

No repository file will be created or deleted.

Attempt-local RDSPI artifacts are separate from the source change set.

## `README.md`

### Opening block

Keep the existing H1 and release badge.

Replace the single mechanism-first lede after the badge.

The replacement is one purpose paragraph:

> Lisa runs coding agents like Claude Code and Codex through your ticket board,
> so you don't have to approve every step by hand.

The paragraph remains before the `Install Lisa` heading.

No mechanism sentence is inserted ahead of it.

The detailed DAG and scheduler explanation stays under `What It Does`.

### Review-trail block

Keep the three existing `What It Does` paragraphs in their current order.

Add one paragraph after the RDSPI artifact paragraph.

The added paragraph says:

> Lisa keeps the trail reviewable: an append-only attempt ledger records each
> run, the completion journal ties finished tickets to commits, and each ticket
> keeps its work documents.

This position groups the new claim with artifact review and crash recovery.

It does not change installation, prerequisites, Quick Start, or command docs.

## `crates/lisa-cli/src/templates.rs`

### Public interface

Keep `pub const AGENTS_MD: &str` unchanged as an interface.

Keep `pub fn generate_claude_md(project: &DetectedProject) -> String` unchanged.

No caller changes are needed in `init.rs` or `setup_guide.rs`.

No new public types, functions, or modules are introduced.

### `AGENTS_MD` layout

The generated file will have this paragraph order:

1. `# AGENTS.md`;
2. the purpose paragraph;
3. the one-line agent contract;
4. the existing `CLAUDE.md` source-of-truth pointer;
5. the existing RDSPI workflow reference.

The pointer remains intact and still tells Codex to read `CLAUDE.md` first.

The file still omits Build and Test and Source Layout sections.

The Rust source comment above the constant will be updated to acknowledge the
stable purpose and contract paragraphs in addition to the pointer and workflow
reference.

### `generate_claude_md` layout

The generated file will have this paragraph order:

1. `# CLAUDE.md`;
2. the purpose paragraph;
3. the one-line agent contract;
4. `## Project`;
5. detected project name, type, and description TODO;
6. optional Build and Test section;
7. optional Source Layout section;
8. Directory Conventions;
9. RDSPI workflow reference.

Purpose and contract are static text inside the existing format template.

Project detection and conditional section assembly remain unchanged.

The generated document continues to be a `String`.

### Template test organization

Add one focused test near the current generation tests.

The test creates one representative Rust `DetectedProject`.

It defines the exact purpose paragraph expected on both surfaces.

It defines the exact contract sentence expected on both surfaces.

It calls `generate_claude_md` and reads `AGENTS_MD`.

It asserts both output strings contain both expected sentences.

It asserts the purpose sentence occurs before mechanism-oriented project
sections in generated `CLAUDE.md`.

Existing tests remain responsible for:

- Rust project metadata;
- Node project metadata;
- unknown project behavior;
- RDSPI workflow embedding;
- the `AGENTS.md` pointer;
- absence of duplicated project sections.

This separates copy-contract coverage from project-detection coverage.

## `crates/lisa-cli/src/setup_guide.rs`

### Guide header

Retain the existing `build_guide` function signature and project detection.

Retain the heading format and the sibling ticket's opening purpose sentence.

Insert the provenance sentence after that purpose paragraph.

Keep “Follow these steps...” after the provenance sentence.

The resulting header order is:

1. project-specific setup-guide H1;
2. purpose paragraph;
3. review-trail paragraph;
4. transition into the numbered setup steps.

The seven guide sections and their numbering do not change.

No conditional logic depends on initialized state for the new sentence.

### Setup-guide tests

Add a unit test near the existing guide content tests.

It builds a guide for a temporary unknown project.

It asserts the output contains:

- `append-only attempt ledger`;
- `completion journal`;
- `commits`;
- `work documents`.

This test exercises `build_guide`, the same internal boundary used by all
existing setup-guide tests.

No subprocess or stdout capture is required.

## Copy boundaries

### Purpose paragraph

The exact paragraph appears in three ticket-owned outputs:

- README opening;
- generated `CLAUDE.md`;
- generated `AGENTS.md`.

The setup-guide header has the already-landed sibling form without client names.

This ticket will not alter sibling copy beyond adding its separate provenance
paragraph.

### Agent contract

The exact contract appears only in generated agent-context files.

It does not appear in README because the README already explains the full phase
workflow and atomic completion contract.

It does not appear in setup-guide prose outside the embedded generated template.

### Provenance sentence

The exact sentence appears in:

- README `What It Does`;
- setup-guide header.

Generated context templates do not need this product-level provenance pitch.

Their contract already says the agent leaves a reviewable record.

## Runtime and data boundaries

`lisa init` continues to preserve existing user files.

New context copy is visible only in newly scaffolded template files or in the
setup guide's preview.

No `.lisa` runtime file is read or written by the new code.

The provenance ledger remains append-only under plugin control.

The completion journal remains under plugin control.

Ticket work document publication remains under Lisa's phase/completion flow.

No serialized format or compatibility boundary changes.

## Implementation units

The meaningful source work forms two ticket commits.

Unit 1 contains README copy only.

Its exact include path is `README.md`.

Unit 2 contains compiled templates, setup-guide copy, and their unit tests.

Its exact include paths are:

- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/src/setup_guide.rs`.

Keeping tests with their production copy makes the second unit self-verifying.

No ordinary Git index operation is needed.

## Verification boundaries

Run `cargo fmt --all -- --check` after editing.

Run focused `lisa-cli` tests for template and setup-guide modules.

Run `cargo test -p lisa-cli` to cover initialization and integration consumers.

Run `cargo test --workspace` if the focused suite passes.

Inspect the README's first prose paragraph and keyword order directly.

Inspect `git status --short` before each ticket commit.

After both ticket commits, verify the three owned files are neither modified nor
staged nor untracked.

Unrelated Lisa-managed and other-ticket worktree entries remain untouched.
