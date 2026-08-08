# T-057-01-05 — Structure

File-level changes, in dependency order.

---

## Created

| Path | Contents |
|---|---|
| `crates/lisa-cli/data/lisa-workflow.md` | the rewritten document (~93 lines raw) |
| `crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.4.md` | byte-exact copy of today's `docs/knowledge/rdspi-workflow.md` (rendered form, 146 lines) |
| `docs/knowledge/lisa-workflow.md` | rendered `LISA_WORKFLOW` — purpose paragraph + the data file |

## Deleted

| Path | Why |
|---|---|
| `crates/lisa-cli/data/rdspi-workflow.md` | replaced by `lisa-workflow.md`; its bytes live on in `legacy/rdspi-workflow-v0.4.4.md` |
| `docs/knowledge/rdspi-workflow.md` | this project migrating itself under its own rule |

Both deletions are named in a `lisa commit-ticket --include`; `git add -A -- <path>` records a
removal.

---

## `crates/lisa-cli/src/templates.rs`

```rust
/// Lisa's workflow document, embedded at compile time.
pub static LISA_WORKFLOW: LazyLock<String> = LazyLock::new(|| {
    format!("{PURPOSE_PARAGRAPH}\n\n{}", include_str!("../data/lisa-workflow.md"))
});

/// Exact outgoing Lisa workflow documents accepted as proof of an unmodified
/// install — under either name. Also the removal warrant for a project's stale
/// `rdspi-workflow.md`: only bytes on this list may be deleted.
pub(crate) const LEGACY_WORKFLOWS: &[&str] = &[
    include_str!("../data/legacy/rdspi-workflow-v0.2.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.4.md"),
];
```

Test changes in the same file:

- `test_rdspi_workflow_embedded` → `test_workflow_document_embedded`: heading assertion becomes
  the new title; the six-phase-word assertions become the four live ones; the byte-equality pin
  reads `../../../docs/knowledge/lisa-workflow.md`.
- `test_review_disposition_contract_is_injected` — identifier swap only; every asserted substring
  is unchanged. This is criterion 6's guarantee, expressed as a diff that touches no string.
- `the_documented_check_contract_matches_the_code_that_enforces_it` — identifier swap only.
- `test_injected_context_is_purpose_first_and_copy_is_single_sourced` — identifier swap; the
  single-source path list entry becomes `../data/lisa-workflow.md`.
- **New** `the_workflow_document_describes_the_board_lisa_actually_runs`: asserts the four phase
  names in order, that `research.md`/`design.md`/`structure.md`/`plan.md` appear nowhere, that
  `review.md` and `review-disposition.json` are the artifact pair, that the rendered document is
  shorter than 146 lines, and that the resume/journal-sealed sentences are present (criteria 4
  and 5).

## `crates/lisa-cli/src/init.rs`

**Enum** (`~230`): add `RemoveFile { path: PathBuf, reason: String }`.
**Display** (`~238`): `"  remove  {path} ({reason})"`.

**New planner**, beside `plan_owned_template`:

```rust
/// Plan the removal of a template Lisa no longer installs at this path.
/// Absent: nothing to say. Exact bytes from a bundled generation: remove it.
/// Anything else is the operator's file and is preserved with a reason that
/// names its replacement.
fn plan_retired_template(path: PathBuf, known: &[&str], replacement: &str) -> Option<InitAction>
```

`SafetySkip` reason: `format!("preserved: content is not a known Lisa template; superseded by {replacement}")` — naming the rename, as criterion 2 requires.

**`plan_init_actions`** (`~362`): install path becomes `docs/knowledge/lisa-workflow.md` with
`templates::LISA_WORKFLOW` / `templates::LEGACY_WORKFLOWS`; immediately after, extend with
`plan_retired_template(root.join("docs/knowledge/rdspi-workflow.md"), LEGACY_WORKFLOWS, "docs/knowledge/lisa-workflow.md")`.

**Executor** (`~985`): a `RemoveFile` arm calling `fs::remove_file` and pushing a
`FileMutation { kind: FileMutationKind::Removed, .. }`. If `FileMutationKind` has no `Removed`
variant, add one and give it a past-tense label wherever mutations are reported.

**Validate** (`~1160`): the existence error moves to `docs/knowledge/lisa-workflow.md`.

**Tests**: ~40 call sites writing `docs/knowledge/rdspi-workflow.md` as a validate fixture become
`lisa-workflow.md`; `"# RDSPI"` placeholder bodies become `"# Workflow"`. Renamed:
`test_validate_missing_rdspi_workflow` → `…_missing_workflow_document`,
`test_plan_init_preserves_unknown_rdspi` → `…_preserves_unknown_workflow`,
`test_plan_init_skips_current_rdspi` → `…_skips_current_workflow`,
`test_plan_init_updates_every_known_rdspi_template` → `…_every_known_workflow_template`.
`test_plan_init_actions_empty_dir` keeps its count of 20 (the retired-template planner emits
nothing when the old file is absent) and its comment loses the word RDSPI.

**New tests** (criterion 2, both halves):

- `an_unmodified_0_4_4_workflow_is_migrated_to_the_new_name` — write the 0.4.4 bytes to
  `docs/knowledge/rdspi-workflow.md`, plan, assert one `CreateFile` for `lisa-workflow.md` and one
  `RemoveFile` for `rdspi-workflow.md`; then run the executor and assert the old file is gone and
  the new one holds `LISA_WORKFLOW`.
- `a_modified_workflow_is_left_where_the_operator_put_it` — write edited bytes, plan, assert
  `CreateFile` for the new path and `SafetySkip` for the old whose reason names
  `lisa-workflow.md`; execute; assert the edited bytes are still on disk unchanged.

## `crates/lisa-cli/src/setup_guide.rs`

- Scaffold table row → `` `docs/knowledge/lisa-workflow.md` | Lisa's workflow document (injected into agent sessions) ``.
- "You do not need to mention Lisa or the RDSPI workflow" → "…or how its tickets move".
- Validate checklist bullet → "`.lisa.toml` and Lisa's workflow document exist".
- Ticket-format frontmatter comment (`~131`): `phase: ready # ready | implement | review | done`.
- `[scheduling.phase_timeouts]` example (`~84`): `research = 300` → `implement = 1800`.
- `test_guide_unknown_project` drops its `RDSPI` assertion (the guide no longer says it).
- `test_guide_references_rdspi` → `test_guide_references_the_workflow_document`, asserting
  `lisa-workflow.md`, `implement`, `review`.

## Doc comments and prose

| File | Change |
|---|---|
| `crates/lisa-cli/src/check_run.rs:7` | path → `docs/knowledge/lisa-workflow.md` |
| `crates/lisa-core/src/disposition.rs:26` | path → `docs/knowledge/lisa-workflow.md` |
| `crates/lisa-core/src/types.rs:339,441` | "the RDSPI workflow" → "the workflow" / "its phases" |
| `crates/lisa-core/src/ticket.rs:4` | "the RDSPI workflow" → "Lisa's workflow" |
| `crates/lisa-core/src/context.rs:5` | "the RDSPI preamble" → "the workflow document" |
| `crates/lisa-core/src/context.rs:18` | "through its RDSPI phases" → "through implementation and review" |
| `crates/lisa-plugin/src/adapter.rs:354` | "the RDSPI workflow" → "the workflow" |
| `crates/lisa-cli/src/config.rs:524` | comment naming the four retired keys as accepted-for-compat |

## Operator-facing prose

| File | Change |
|---|---|
| `README.md:76` | six phases → "Implement, then Review", one artifact pair; the crash-recovery sentence becomes the commit/journal truth |
| `README.md:258` | "its initial RDSPI prompt" → "its initial ticket prompt" |
| `README.md:284–293` | the numbered six-phase list → two entries, and the ~200-line-artifact paragraph → what the pair is for |
| `README.md:356` | layout tree filename |
| `README.md:363` | `lisa init` no longer creates `CLAUDE.md`/`AGENTS.md` (already true since T-057-01-03); name the workflow document |
| `CONTRIBUTING.md:86–91` | "five phases … ~200-line artifact" → the two-phase reality, link to `docs/knowledge/lisa-workflow.md` |
| `CLAUDE.md:15,54,68` | this repo's own description |
| `AGENTS.md:11` | same line |
| `docs/ROADMAP.md:15,38,40,169` | reword four historical entries so they describe the document without naming the retired path |
| `docs/PROMPT_CODEX.md:18,56` | same two edits |
| `aur/PKGBUILD:7` | `pkgdesc` |
| `.lisa.toml:22–23` | commented `research`/`design` timeout examples → `implement`/`review` |

## Fixtures

| File | Change |
|---|---|
| `crates/lisa-cli/tests/fixtures/live_provider_startup.sh:268` | "concise RDSPI phase artifacts" → "the review artifacts" |
| `docker/chromebook-test/bin/prepare:104` | "the required RDSPI phase artifacts" → "the required review artifacts" |
| `docker/chromebook-test/board/tickets/T-004.md:23` | drop `"RDSPI,"` from the jargon example list |
| `docker/chromebook-test/board/stories/S-001.md:16` | "understand RDSPI, DAGs" → "understand phases, DAGs" |
| `crates/lisa-plugin/tests/fixtures/codex_ack/*.json` | `"complete the RDSPI workflow"` → `"complete the ticket."` — these two files are synthetic ack payloads; the tests read `ticket_id` and `generation` only |

The fixture board's retired `phase:` frontmatter values stay exactly as they are.

## Ordering

1. Legacy capture (`rdspi-workflow-v0.4.4.md`) — must be taken *before* anything else moves.
2. New data file + `templates.rs` + `docs/knowledge/lisa-workflow.md` + deletions of both old
   files. These are one unit: the byte-equality pin fails if they are split.
3. `init.rs` migration (`RemoveFile`, planner, path, validate, tests).
4. Doc comments, prose, fixtures.
