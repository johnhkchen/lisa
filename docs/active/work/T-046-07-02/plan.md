# Plan: purpose-first docs and templates

## Preconditions

1. Reconfirm the three owned files have no pre-existing worktree or index diff.
2. Leave active ticket, provenance, journal, and unrelated ticket files alone.
3. Keep every phase artifact under the generation-1 attempt work directory.
4. Do not update ticket phase or status frontmatter manually.

## Step 1: rewrite the README opening

Modify `README.md` only.

Replace the current DAG-first lede with the purpose paragraph.

Use the exact text chosen in Design:

> Lisa runs coding agents like Claude Code and Codex through your ticket board,
> so you don't have to approve every step by hand.

Keep it immediately after the release badge.

Confirm the paragraph appears before the first use of `DAG`, `scheduling`,
`WASM`, or `Zellij` in prose.

Do not rearrange installation material.

## Step 2: add the README review-trail sentence

In the `What It Does` section, find the paragraph describing six RDSPI phase
artifacts.

Add the exact provenance sentence after it:

> Lisa keeps the trail reviewable: an append-only attempt ledger records each
> run, the completion journal ties finished tickets to commits, and each ticket
> keeps its work documents.

Confirm all three requested concepts are present.

Confirm the sentence describes existing behavior rather than a future feature.

Review the local README diff for copy-only scope.

## Step 3: verify README independently

Read the first 100 lines of `README.md`.

Use a short read-only script to locate the purpose paragraph and mechanism terms.

Require the purpose paragraph position to precede all mechanism terms in the
opening surface.

Search for the exact review-trail sentence.

Run Markdown-oriented checks if the repository exposes any; none are currently
required by the ticket.

## Step 4: commit the README unit

Run:

```text
lisa commit-ticket \
  --ticket-id T-046-07-02 \
  --message "T-046-07-02: lead README with Lisa's purpose" \
  --include README.md
```

Do not use `git add` or ordinary `git commit`.

Record the returned commit ID in `progress.md`.

Confirm `README.md` is clean afterward.

## Step 5: orient generated AGENTS.md

Modify `crates/lisa-cli/src/templates.rs`.

Update the comment above `AGENTS_MD` to match its stable content.

Place the purpose paragraph after the H1.

Place the chosen one-line contract after the purpose paragraph.

Keep the source-of-truth pointer next.

Keep the RDSPI workflow reference last.

Preserve the `pub const AGENTS_MD: &str` interface.

## Step 6: orient generated CLAUDE.md

In the same source file, update the format template returned by
`generate_claude_md`.

Place the exact same purpose paragraph after `# CLAUDE.md`.

Place the exact same contract after it.

Keep `## Project` and the detected project content below both sentences.

Do not change type labels, optional build rendering, optional source layout
rendering, directory conventions, or RDSPI reference behavior.

## Step 7: assert the template contract

Add a focused unit test in `templates.rs`.

Construct a representative detected Rust project.

Generate `CLAUDE.md` content.

Use `AGENTS_MD` for the Codex content.

Assert both contain the exact purpose paragraph.

Assert both contain the exact agent contract.

Assert the purpose paragraph follows each H1 and precedes later project sections.

Retain `test_agents_md_points_to_claude` assertions so project context is not
duplicated.

## Step 8: add setup-guide provenance copy

Modify `crates/lisa-cli/src/setup_guide.rs`.

Keep the sibling purpose sentence as the first header paragraph.

Insert the exact README review-trail sentence after it.

Keep the setup-step transition after the new sentence.

Do not change guide sections, numbering, initialized-state branches, or output
control flow.

## Step 9: assert setup-guide provenance

Add a setup-guide unit test.

Create a temporary project using the existing test pattern.

Call `build_guide`.

Assert the output names the append-only attempt ledger.

Assert it names the completion journal and its commit relationship.

Assert it names per-ticket work documents.

This test protects the acceptance criterion without snapshotting the full guide.

## Step 10: format and run focused tests

Run `cargo fmt --all -- --check`.

If formatting fails, run the formatter and inspect its exact changes.

Only ticket-owned files may be changed by formatting.

Run the template module tests.

Run the setup-guide module tests.

Use the package's actual test-name filters if module-path filters do not select
tests as expected.

Resolve any failure before committing.

## Step 11: run package and workspace tests

Run `cargo test -p lisa-cli`.

This covers template consumers in initialization plus setup-guide tests.

If it passes, run `cargo test --workspace`.

Record exact pass/fail counts or command outcomes in `progress.md`.

If a failure is unrelated and pre-existing, capture its evidence and determine
whether focused ticket coverage still establishes readiness.

If a failure is caused by this change, fix it before Review.

## Step 12: commit compiled copy and tests

Review the exact diff of the two Rust files.

Run:

```text
lisa commit-ticket \
  --ticket-id T-046-07-02 \
  --message "T-046-07-02: orient generated agent context" \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/setup_guide.rs
```

Record the returned commit ID in `progress.md`.

Do not include any artifact, ticket, provenance, or unrelated source path.

## Step 13: final implementation audit

Run `git status --short`.

Verify all three ticket-owned source paths are clean.

Run `git diff --cached --` on the three paths and require no output.

Inspect the two ticket commits with `git show --stat` and exact-path diffs.

Confirm other dirty entries were preserved and excluded.

Update `progress.md` with completed steps, test results, commits, and deviations.

## Step 14: review

Write `review.md` in the attempt work directory.

Summarize the three changed files and user-visible outcomes.

Describe focused, package, workspace, formatting, and string-order verification.

State whether there are open concerns or test gaps.

Recheck both acceptance criteria against the final committed content.

Write `review-disposition.json` with the exact pass shape only if all owned source
files are committed and verification passes.

Remain on this ticket after both Review artifacts exist.
