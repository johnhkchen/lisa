# T-057-01-05 — Review

Lisa's contract document is now `lisa-workflow.md`, it describes the four-state board the code
actually runs, and a project upgrading from 0.4 is migrated off the old filename under the same
ownership rule that protects every other Lisa template.

Five commits on `main`, `0bbd91c..d138d08`. `just check` exits 0.

---

## What changed

**Created**
- `crates/lisa-cli/data/lisa-workflow.md` (120 lines) — the rewritten document.
- `crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.4.md` (146 lines) — the outgoing text,
  captured in rendered form so an upgrading project is recognised.
- `docs/knowledge/lisa-workflow.md` (122 rendered lines) — this project installing its own output.

**Deleted**
- `crates/lisa-cli/data/rdspi-workflow.md`, `docs/knowledge/rdspi-workflow.md`.

**Modified** — `templates.rs`, `init.rs`, `setup_guide.rs`, `check_run.rs`, `config.rs`,
`disposition.rs`, `types.rs`, `ticket.rs`, `context.rs`, `adapter.rs`, `README.md`,
`CONTRIBUTING.md`, `CLAUDE.md`, `AGENTS.md`, `docs/ROADMAP.md`, `docs/PROMPT_CODEX.md`,
`aur/PKGBUILD`, `.lisa.toml`, and five fixtures.

**The migration.** `InitAction::RemoveFile` is the first action Lisa has that deletes anything,
and it is reachable only through `plan_retired_template`, which requires exact bytes from a
bundled generation. An edited file is preserved and reported with a reason naming its
replacement. Removal is an action rather than a side effect so `--dry-run` shows it first and the
run reports it after.

**The document.** 122 rendered lines against 146. Gone: the four phase chapters and Phase Rules
1, 2, 4, 5. Kept verbatim: the commit discipline, the disposition schema, the `remedy_owner`
vocabulary, the five clauses of the check-execution contract, the `ask` guidance and its
counter-example, `lisa check-disposition`, the seal-gated wait, the ticket format, and the
concurrency rules. Added: a four-state phase list, and an "If your session dies mid-ticket"
section saying that a dead session resumes from its commits and a journal-sealed project has
none, so such a ticket restarts from the beginning.

---

## Acceptance criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | renamed, embedded from the new path, `lisa init` installs it | `templates.rs:5–10`; `init.rs:411–420`; live `lisa init --dry-run` prints `create …/docs/knowledge/lisa-workflow.md` |
| 2 | the migration test | `an_unmodified_prior_workflow_is_migrated_to_the_new_name` and `a_modified_workflow_is_left_where_the_operator_put_it` — both plan **and** execute, and the second asserts the operator's bytes survive and that the skip reason names the rename |
| 3 | 0.4.4 joins the legacy generations | `templates.rs:23`; `test_plan_init_updates_every_known_workflow_template` (which also asserts every entry is byte-distinct from current) |
| 4 | four phases, one artifact pair, no per-phase writing duty, shorter than 146 lines | `the_workflow_document_describes_the_board_lisa_actually_runs` — asserts the phase line, the absence of the four retired artifact names, the pair, and `lines().count() < 146` (actual 122) |
| 5 | states the resume truth | same test, three assertions on the "If your session dies mid-ticket" section |
| 6 | S-056-01 survives verbatim; the `disposition.rs` pin passes | `test_review_disposition_contract_is_injected` and `the_documented_check_contract_matches_the_code_that_enforces_it` both pass; their diffs change one identifier and **no string literal** |
| 7 | no live file names the old document | scoped grep below |
| 8 | `just check` green | exit code 0, 26 `test result: ok` lines |

### Criterion 2, verified end to end with the built binary

```
$ lisa init --dry-run --path <project with byte-exact 0.4.4 rdspi-workflow.md>
  create  …/docs/knowledge/lisa-workflow.md
  remove  …/docs/knowledge/rdspi-workflow.md (superseded by docs/knowledge/lisa-workflow.md)

$ lisa init --dry-run --path <project whose rdspi-workflow.md was edited>
  create  …/docs/knowledge/lisa-workflow.md
  skip    …/docs/knowledge/rdspi-workflow.md (preserved: content is not a known Lisa
          template; superseded by docs/knowledge/lisa-workflow.md)
```

### Criterion 7, and where it needed a reading

```sh
git ls-files | grep -v '^docs/archive/' | grep -v '^docs/active/' | grep -v '^docs/knowledge/' \
  | xargs grep -ril rdspi
```

returns six paths:

- the three `crates/lisa-cli/data/legacy/rdspi-workflow-v*.md` files — the legacy template data
  the criterion exempts by name;
- `crates/lisa-cli/src/templates.rs` — the `include_str!`s for those three files and the comment
  explaining what they are;
- `crates/lisa-cli/src/init.rs` — the path the migration removes, and the tests that assert it;
- `crates/lisa-plugin/src/lib.rs` — two assertions that the assignment prompt does **not** contain
  the string.

**The last three are load-bearing and cannot go.** Criterion 2 requires code that removes
`docs/knowledge/rdspi-workflow.md`, and code that removes a file must name it. Criterion 3
requires the legacy generations to be reachable, and `include_str!` takes a path. The plugin's
two hits are a guard against the word, not a use of it. No file in the live tree *describes* the
workflow under the old name, which is the criterion's subject.

The three excluded directories: `docs/archive/**` the ticket excludes; `docs/active/**` is
~300 completed work artifacts and past ticket/story files, which are the record of what was true
when they ran; `docs/knowledge/**` holds field notes and runbooks that S-057-01 puts out of this
ticket's scope in as many words. `ls docs/knowledge | grep rdspi` is empty — the one file in
there this ticket owns is gone, and its replacement is byte-pinned to the template by test.

---

## Test coverage

**New:** three tests in `init.rs` (unmodified-migrates across *every* bundled generation,
modified-is-preserved, never-had-it-says-nothing) and one in `templates.rs` (the document's
shape). The migration tests execute `run_init_with_io` against a real temp directory rather than
asserting on the plan alone, so the deletion is verified on disk and the second run is asserted
idempotent.

**Retargeted:** `test_workflow_document_embedded` (byte equality against
`docs/knowledge/lisa-workflow.md`), ~40 `init.rs` validate fixtures, two `setup_guide` tests.

**Deliberately unchanged:** the two S-056-01 pins. That their diffs contain no string change is
criterion 6's evidence, and it is stronger than any assertion this ticket could have added.

**Gap.** Nothing tests that a project holding *both* a current `lisa-workflow.md` and a stale
`rdspi-workflow.md` converges — the second-pass idempotence assertion inside the migration test
covers the sequence that produces that state, but not the state arrived at another way. Low risk:
the two planners are independent by construction, which is exactly why that state is uninteresting.

---

## Open concerns

1. **`lisa commit-ticket` refuses an add-plus-similar-delete in one transaction.** Committing
   `lisa-workflow.md` (add) and `rdspi-workflow.md` (delete) together failed with
   `ordinary staged entries changed during verification`, repeatably, with a clean ordinary index
   before and after. Probes established that adds alone commit, deletes alone commit, and the two
   together do not — consistent with rename detection in the post-commit
   `git diff --cached` verification (`commit_transaction.rs:954–973`). The work went in as two
   commits. **This is a real limitation an agent will hit again on any rename**, and it is
   outside this ticket's scope to fix. Worth a ticket of its own.

2. **Five probe commits were made and removed from `main`.** Diagnosing (1) left five throwaway
   commits. They were unpushed, and neither the completion journal nor the provenance ledger
   records commit SHAs, so `git reset --mixed 0bbd91c` restored the branch with the worktree
   intact and the work was committed properly afterwards. The reflog still holds them. Flagged
   because rewriting `main` under a running scheduler is not something to do silently.

3. **`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure` is flaky**
   under a loaded parallel test run — it timed out twice mid-work and passes in isolation and in
   the final `just check`. Pre-existing and unrelated to this ticket, but it will bite someone.

4. **`config.rs` still accepts the four retired names as `phase_timeouts` keys.** Kept on purpose,
   for the same reason `Phase` still deserializes them: a 0.4 project's config must not start
   warning about keys that were correct when it wrote them. Now commented as compatibility rather
   than reading as a six-phase workflow.

5. **The document tells an agent to write its artifacts "to the work directory your assignment
   names".** That is deliberately indirect: the artifacts are written to an attempt-private
   directory and published by Lisa to `docs/active/work/{ticket-id}/`, and the document should
   not hard-code a path the prompt owns. If the prompt ever stops naming one, this sentence goes
   vague.
