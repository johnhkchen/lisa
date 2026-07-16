# Review: purpose-first docs and templates

## Disposition

Ready to complete.

Both acceptance criteria are satisfied.

All ticket-owned source changes are committed through Lisa's isolated ticket
transaction.

Focused, package, and workspace tests pass.

## Outcome

The README now tells a new reader what Lisa is for before explaining how it
works.

The first prose paragraph names coding agents, Claude Code, and Codex.

It says Lisa runs those agents through the reader's ticket board.

It also states the no-babysitting benefit in ordinary language: the reader does
not have to approve every step by hand.

Newly scaffolded Claude Code and Codex context files now open with the same
purpose paragraph.

Both files follow it with the same one-line operating contract.

That contract tells the agent to take one ticket through every RDSPI phase,
leave a reviewable record, and wait for Lisa to confirm completion.

The README and setup guide now expose Lisa's existing review trail.

They name the append-only attempt ledger, the completion journal that ties
finished tickets to commits, and each ticket's work documents.

No new provenance feature or behavior was introduced.

## Files changed

### `README.md`

Replaced the mechanism-first lede.

The previous first paragraph led with DAG scheduling.

The replacement leads with the operator purpose and supported coding agents.

Added one review-trail paragraph under `What It Does`, next to the existing RDSPI
artifact and recovery explanation.

Installation commands, prerequisites, Quick Start, command reference, and
development instructions are unchanged.

### `crates/lisa-cli/src/templates.rs`

Extended the static `AGENTS_MD` scaffold with purpose and contract paragraphs.

Kept its `CLAUDE.md` source-of-truth pointer.

Kept the RDSPI workflow reference.

Kept project-specific Build and Test and Source Layout content out of the pointer
file.

Extended `generate_claude_md` with the same purpose and contract paragraphs.

Placed both before `## Project` and before generated build/source details.

Kept project type detection, optional sections, directory conventions, and the
workflow reference unchanged.

Added a focused test for exact shared text and ordering in both generated files.

### `crates/lisa-cli/src/setup_guide.rs`

Preserved the purpose-first preamble landed by the sibling copy ticket.

Added the review-trail sentence immediately after that preamble.

Placed it in the unconditional header so initialized and uninitialized projects
both see it.

Kept all seven setup steps and their numbering unchanged.

Added a focused test for the three provenance concepts.

## Acceptance review

### README purpose comes before mechanism

Pass.

The purpose paragraph begins on README line 5.

It names `coding agents`, `Claude Code`, and `Codex`.

It states the operator does not need to approve every step by hand.

The former DAG-first lede is gone.

Mechanism details remain later, where the README explains installation and
operation.

### Provenance sentence in README and setup guide

Pass.

Both surfaces use the same plain-language sentence.

The sentence accurately describes existing storage and lifecycle behavior.

The attempt ledger corresponds to the append-only provenance ledger.

The completion journal's confirmed transitions bind ticket IDs to commit IDs.

The RDSPI workflow already keeps per-ticket work documents.

The copy does not claim a new command, UI, or persistence feature.

### Generated templates carry purpose and contract

Pass.

Generated `CLAUDE.md` and `AGENTS.md` contain the exact same purpose paragraph.

They contain the exact same one-line agent contract.

Tests verify the order H1, purpose, contract, then project/pointer material.

Existing tests still verify the Codex pointer and absence of duplicated project
sections.

### Brand voice

Pass.

The new copy starts with active verbs: Lisa runs, Lisa keeps, you take.

It uses short spoken phrases rather than implementation terms.

The first-impression copy avoids DAG, scheduling, WASM, and Zellij.

The provenance sentence uses the required nouns only where they identify real
review records.

## Test coverage

### New unit coverage

`test_generated_agent_context_opens_with_purpose_and_contract` covers both
generated context surfaces.

It checks the exact purpose sentence.

It checks the exact contract.

It checks relative ordering rather than mere presence.

`test_guide_names_the_existing_review_trail` checks the setup guide for:

- the append-only attempt ledger;
- the completion journal's relationship to commits;
- each ticket's work documents.

### Existing regression coverage

The existing Rust, Node, and unknown-project template tests passed.

The existing `AGENTS.md` pointer test passed.

Initialization tests passed, including scaffold creation and preservation of
user-authored context files.

Setup-guide project-type, initialized-state, RDSPI, ticket-format, and numbering
tests passed.

The sibling help-surface purpose-order and snapshot tests passed.

### Commands run

```text
cargo fmt --all -- --check
```

Passed twice, including after final source commits.

```text
cargo test -p lisa-cli templates::tests::test_generated_agent_context_opens_with_purpose_and_contract
```

Passed.

```text
cargo test -p lisa-cli setup_guide::tests::test_guide_names_the_existing_review_trail
```

Passed.

```text
cargo test -p lisa-cli
```

Passed with no failures; the existing environment-gated real-Zellij test stayed
ignored.

```text
cargo test --workspace
```

Passed with no failures across CLI, core, plugin, integration, and doc tests.

`git diff --check` also passed.

## Commits

`e90ae071512d1fd4c268d72b465d8fc2a984db68`

Message: `T-046-07-02: lead README with Lisa's purpose`

Exact owned path: `README.md`.

`6633cf44fd78e5c83ecc7c43752589135adbe47a`

Message: `T-046-07-02: orient generated agent context`

Exact owned paths:

- `crates/lisa-cli/src/templates.rs`;
- `crates/lisa-cli/src/setup_guide.rs`.

## Ownership and repository state

The ordinary Git index was not used.

No ordinary `git add`, broad add, or ordinary commit was run.

All three ticket-owned paths are clean and unstaged.

The repository still contains unrelated Lisa lifecycle, ticket, work-document,
release-script, and planning changes.

Those entries were present or arrived independently and were not included in
either ticket commit.

## Open concerns and limitations

No blocking concern remains.

Because `lisa init` preserves user-authored context files, the new template copy
appears in newly created files and setup-guide previews; it does not overwrite
existing project instructions. That is established behavior and intentional.

README copy has static diff/line-order verification rather than a dedicated
Markdown unit-test harness. The generated outputs, where regressions are easier
to introduce through code, have explicit string and order tests.

The story's field-level tour-probe rematch is deliberately outside this ticket
and belongs to the closing-run ticket. This ticket supplies the required inputs
for that later human-facing validation.

No WASM build was required because the change affects README and native CLI copy
generation only. The full workspace native suite exercised all relevant code.

## Handoff

The work is ready for Lisa's completion transaction.

Lisa should publish the attempt artifacts, write Done state, bind completion to
the final commit, and release the seat only after the completion receipt is
verified.
