# Structure: T-031-03 provider contract and live regression

## Change inventory

### Production contract and prompt files

- Modify `crates/lisa-cli/data/rdspi-workflow.md`.
- Modify `docs/knowledge/rdspi-workflow.md` to match the shipped contract.
- Create `crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.md`.
- Modify `crates/lisa-cli/src/templates.rs` to register the outgoing template.
- Modify `crates/lisa-cli/src/init.rs` tests for clean upgrade/custom preservation.
- Modify `crates/lisa-plugin/src/lib.rs` common ticket and finish-up prompts.
- Modify prompt tests in `crates/lisa-plugin/src/lib.rs`.

### End-to-end regression files

- Create `docs/active/work/T-031-03/harness/run.sh`.
- Create `docs/active/work/T-031-03/harness/README.md`.
- Create `crates/lisa-cli/tests/atomic_provider_contract.rs`.

### User documentation files

- Modify `README.md`.
- Modify `docs/knowledge/lisa-loop-setup-guide.md`.

### RDSPI artifacts

- Create `docs/active/work/T-031-03/research.md`.
- Create `docs/active/work/T-031-03/design.md`.
- Create `docs/active/work/T-031-03/structure.md`.
- Create `docs/active/work/T-031-03/plan.md`.
- Create `docs/active/work/T-031-03/progress.md`.
- Create `docs/active/work/T-031-03/review.md`.

No source module is deleted. No ticket frontmatter is edited by this work.

## Bundled workflow organization

`crates/lisa-cli/data/rdspi-workflow.md` remains the compile-time source of
truth for newly initialized projects.

The Implement section gains an explicit atomic Git contract after the existing
implementation guidance. Its internal organization will be:

1. execute plan and track `progress.md`;
2. commit meaningful implementation units;
3. use only `lisa commit-ticket`;
4. pass exact repository-relative include paths;
5. prohibit ordinary-index staging and broad ownership;
6. finish Review and wait for scheduler completion.

The Review section remains responsible for the handoff artifact. It gains a
terminal instruction that the agent does not publish Done or select new work.

Phase Rules gains a distinct completion rule explaining Lisa's final transaction.
Concurrency replaces the ambiguous “file locking” sentence with the isolated
alternate-index transaction boundary.

`docs/knowledge/rdspi-workflow.md` will be byte-identical to the new bundled
file. This repository's copy is an unmodified installed template and should
represent the contract Lisa itself dogfoods.

## Legacy workflow ownership

`crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.md` contains the exact bytes of
the current outgoing six-phase workflow before this ticket changes it.

`templates::LEGACY_RDSPI_WORKFLOWS` becomes a two-entry slice:

```rust
pub(crate) const LEGACY_RDSPI_WORKFLOWS: &[&str] = &[
    include_str!("../data/legacy/rdspi-workflow-v0.2.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.md"),
];
```

Ordering is oldest to newest for readability only. Ownership classification is
exact byte equality and does not depend on order.

Init tests will stop assuming entry zero represents all known history. A helper
or loop will write every legacy workflow and require `UpdateFile`. A focused
assertion will ensure the outgoing v0.4 content differs from the new current
template so the migration fixture cannot silently become redundant.

The existing unknown workflow test remains the customization guard. It must
still produce one `SafetySkip` with the established reason and preserve bytes.

## Common provider prompt organization

`ticket_prompt` remains one formatted string parameterized only by:

- real ticket directory;
- ticket ID;
- provider context filename.

The body will have four semantic paragraphs encoded in the existing compact
string style:

1. read ticket, client context, and workflow;
2. produce every remaining RDSPI artifact continuously;
3. obey frontmatter and isolated Git restrictions;
4. write Review, stop on the ticket, and wait for Lisa confirmation.

No Claude-only or Codex-only safety wording is introduced. Both fresh launch
commands and both reuse prompts continue to receive the same body.

`finish_up_prompt` remains provider-neutral and path-specific. Its final clauses
will prohibit phase/status edits and ordinary-index Git operations, then require
waiting for Lisa's completion confirmation.

## Prompt test organization

Extend `test_ticket_prompt_content` to assert the contract phrases:

- `lisa commit-ticket`;
- exact `--include` paths;
- no ordinary-index `git add`/`git commit`;
- no staged ticket-owned files;
- wait for Lisa completion confirmation;
- no self-selection of a new ticket.

Keep `test_ticket_prompt_uses_given_context_file` as the context divergence test.

Add a focused finish-up prompt test if none exists in the main module. Assert it
contains the review path, frontmatter prohibition, ordinary-index prohibition,
and completion wait rule.

Adapter tests already prove both clients use these free functions for reuse and
follow-up. Add explicit safety phrase checks to Codex and Claude reuse tests only
if the common-function tests do not make the provider coverage legible.

## Harness directory boundary

`docs/active/work/T-031-03/harness/` is reusable source, not generated output.

`run.sh` accepts:

```text
run.sh [--keep] [--root <temporary-root>]
```

It reads `LISA_BIN` when supplied and otherwise resolves `lisa` from `PATH`.
The integration test always supplies the Cargo-built executable.

The harness creates:

```text
<temporary-root>/
  repo/                 # real external Git fixture
  evidence/
    activity.jsonl
    provenance.jsonl
    commits.txt
    index.before
    index.after
    status.final
    trees/<ticket>.txt
```

The fixture repository never contains evidence output. This separation allows a
strict no-loop-owned-residue assertion without ignoring diagnostic files.

## Fixture ticket graph

Create six tickets with real frontmatter and descriptive filenames:

```text
T-CDX-01  codex  no dependency
T-CDX-02  codex  no dependency
T-CDX-03  codex  no dependency
T-CDX-04  codex  no dependency
T-CDX-05  codex  depends_on T-CDX-01
T-MIX-01   claude depends_on T-CDX-05
```

All are processed sequentially through logical `seat-1`. Five Codex rows prove
same-provider reuse; the final Claude row proves provider-neutral transaction
behavior after a cross-provider route.

Tickets begin at Review in the deterministic transaction fixture because the
plugin phase engine is already independently tested. Each fixture gets a unique
source file plus six artifact files before its completion request.

## Harness command flow

For each ticket:

1. Assert all dependencies have confirmed commit hashes.
2. Append a `ticket_started` activity row with route and `seat-1`.
3. Create the ticket's source change.
4. Invoke `lisa commit-ticket` with that exact source path.
5. Assert the foreign ordinary-index tuple is unchanged.
6. Create all six work artifacts.
7. Append a `completion_pending` activity row.
8. Invoke `lisa complete-ticket` with the real ticket/work paths.
9. Append confirmed activity and provenance rows using returned hash.
10. Capture the completion tree and run ticket-specific assertions.

The dependent's start helper accepts the prerequisite hash and checks
`git merge-base --is-ancestor <hash> HEAD` immediately before recording start.

## Foreign staged fixture

Create and commit `foreign.txt` in the baseline. Modify it and stage that
modification in the ordinary index before any ticket work.

Capture `git ls-files --stage -- foreign.txt` as `index.before`. After every
implementation and completion transaction, compare the live tuple byte-for-byte
with this file. At the end write `index.after` and compare again.

For every transaction hash, inspect `git diff-tree --name-only` and reject
`foreign.txt`. This proves both preservation and exclusion.

## Completion content assertions

For each completion commit:

- `git show <hash>:<ticket-path>` contains `phase: done` and `status: done`;
- the parent ticket blob does not contain `phase: done`;
- all six artifact paths exist in the commit;
- the unique source file exists in the commit tree;
- scoped porcelain status for ticket, work, and source is empty;
- commit hash is a descendant of the implementation transaction hash.

At harness end:

- exactly six provenance rows exist;
- exactly five Codex starts exist;
- all five Codex starts name `seat-1`;
- one Claude start exists;
- activity ordering puts T-CDX-01 confirmation before T-CDX-05 start;
- T-CDX-05 confirmation precedes T-MIX-01 start;
- only `foreign.txt` appears in ordinary staged status;
- no unstaged or untracked loop-owned fixture path remains.

## Integration test boundary

`crates/lisa-cli/tests/atomic_provider_contract.rs` contains one process test.
It resolves:

- the harness from `CARGO_MANIFEST_DIR` plus the repository-relative work path;
- the Lisa executable from Cargo's `CARGO_BIN_EXE_lisa` integration-test value.

It launches `bash run.sh`, sets `LISA_BIN`, captures stdout/stderr, and fails with
both streams if the harness exits nonzero. This makes `cargo test --workspace`
run the durable composed regression automatically.

The integration test does not implement Git assertions itself; `run.sh` is the
single reusable assertion source for CI and manual diagnostic runs.

## Harness README

Document:

- what is real versus simulated;
- prerequisites (`bash`, `git`, built Lisa binary);
- direct invocation examples;
- how to retain evidence;
- evidence file meanings;
- why no provider credentials or Zellij are required;
- the separate plugin tests that cover pending seat state.

## README organization

Revise “How It Works / Workflow” to six phases and list Review.

Add an “Atomic completion” subsection after the phase list. It describes exact
path commits, no ordinary staging, foreign staged preservation, Lisa-owned Done,
dependent gating, and failure recovery.

Revise Concurrency so the lock is described as part of `lisa commit-ticket` and
`lisa complete-ticket`, not a reason ordinary agent commits are safe.

## Setup guide organization

Revise “Agent Lifecycle” steps 3–7:

- agents produce artifacts; Lisa advances phases;
- implementation commits use the isolated command;
- Review waits for completion confirmation;
- Lisa commits Done then releases dependents.

Add `review.md` to “Phase Artifacts.” Add a compact “Atomic completion and
recovery” subsection describing foreign index preservation and retry behavior.

Do not rewrite unrelated historical/future sections unless they directly state
the obsolete agent-owned transition contract.

## Implementation order

1. Preserve the outgoing workflow fixture.
2. Update bundled and installed workflow copies.
3. Register legacy ownership and tests.
4. Update provider prompts and tests.
5. Build the external-repository harness and integration test.
6. Update user documentation.
7. Run focused verification and correct failures.
8. Run full workspace/WASM/Clippy validation.
9. Complete progress and review artifacts.

## Unchanged boundaries

- No scheduler completion state machine change.
- No transaction algorithm or CLI syntax change.
- No provider-specific Git behavior.
- No broad dirty-tree inference.
- No ticket frontmatter edit by this session.
- No checked-in generated evidence.
