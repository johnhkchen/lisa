# Plan: append-only ignore and mutation report

## Implementation strategy

Work in small, verifiable units inside the existing plan-then-execute init
pipeline. Establish explicit action semantics first, then introduce the
append-only merge, then make execution/reporting auditable. Preserve all
unrelated working-tree changes and never edit ticket frontmatter.

## Step 1: refine init action categories

### Changes

- Replace `InitAction::Skip` with `NoOp` and `SafetySkip` variants.
- Update `Display` labels to `no-op` and `skip` respectively.
- Classify each planner branch according to the Structure policy table.
- Update the execution match so neither variant performs work.
- Update existing tests to match the correct explicit category.
- Change fresh-plan counting to select create variants directly.

### Verification

- Run the focused init tests.
- Confirm current-template cases are no-ops.
- Confirm unknown, unreadable, and malformed cases are safety skips.
- Confirm fresh initialization still plans eight directories and twelve files.

### Atomic unit

Action-category refactor with mechanically updated tests. It should preserve
filesystem behavior while making output semantics explicit.

## Step 2: add append-only gitignore planning

### Changes

- Add `plan_append_only_gitignore(path, required)` beside the ownership helper.
- Derive required rules from `templates::LISA_GITIGNORE`.
- Compare trimmed existing logical lines for presence.
- Preserve the original existing text as the exact output prefix.
- Insert a separator newline only when a nonempty prefix lacks one.
- Append each missing rule once in current template order.
- Return `NoOp` when all required rules are present.
- Return `SafetySkip` for unreadable/non-UTF-8 content.
- Wire `.lisa/.gitignore` to this helper instead of `plan_owned_template`.

### Verification

- Test absent, legacy, current, customized, no-trailing-newline, surrounding
  spacing, and invalid UTF-8 inputs.
- Assert planning itself never changes the file.
- Assert updated content starts with the exact original bytes.
- Assert missing rules occur once and in template order.
- Assert merged content plans as a no-op on the next pass.

### Atomic unit

Append-only planner and edge-case unit tests.

## Step 3: strengthen the combined field regression

### Changes

- Revise the T-030-01 combined project-customization fixture rather than creating
  a disconnected scenario.
- Retain workflow Story Layer/read-the-story additions.
- Retain customized hook/sample content.
- Keep `.lisa/.gitignore` with `signals/` and `hooks/ntfy-topic`.
- Change the ignore expectation from safety skip to append-only update.
- Run real init and assert workflow/hook/sample bytes are unchanged.
- Assert the gitignore's original bytes are an exact prefix of the result.
- Assert `claude/` and `codex/` are appended and not duplicated.
- Initialize Git in the fixture and invoke `git check-ignore` on the secret path.

### Verification

- Check command exit success and ignored-path output.
- Run the single regression by name.
- Run all init tests.

### Atomic unit

Vend upgrade plus notification-secret regression.

## Step 4: make init output injectable

### Changes

- Import `std::io::Write`.
- Add private `run_init_with_writer` containing the existing implementation.
- Convert init output to `writeln!` calls on the supplied writer.
- Keep public `run_init` as a stdout-locking compatibility wrapper.
- Map output failures to a readable `String` error.

### Verification

- Existing `run_init` filesystem tests continue passing.
- Capture dry-run output and verify the action list and no-changes completion.
- Confirm a dry run creates none of its planned targets.

### Atomic unit

Writer injection without mutation-report behavior change.

## Step 5: record and report successful file mutations

### Changes

- Add private `FileMutationKind` and `FileMutation` types.
- Create an empty mutation vector immediately before action execution.
- After successful `CreateFile`, record `Created(path)`.
- After successful `UpdateFile`, record `Updated(path)`.
- Record neither directories nor no-op/safety-skip actions.
- Emit a deterministic `Files changed:` section after successful execution.
- Emit `none` when no file content was written.
- Preserve created-versus-updated labels in the final report.
- Update next steps to instruct inspection before the next commit.

### Verification

- Build a fixture with a create, update, no-op, and safety skip.
- Snapshot pre-run bytes/existence for every planned file path.
- Run with captured output.
- Derive the actual changed set from before/after snapshots.
- Assert the report equals that set and category.
- Assert skipped and unchanged paths are absent from the report body.
- Run a second time and assert the content mutation report is `none`.

### Atomic unit

Successful-write record, final report, and write-set regression tests.

## Step 6: align hook permission mutations

### Changes

- Replace the unconditional existing-hook chmod loop.
- Determine active hook paths written in the current mutation vector.
- Set `0755` only for those created/updated active hook files.
- Keep `on-notify.sample` excluded.
- Leave no-op and safety-skipped hook modes untouched.

### Verification

- Fresh-init executable-hook assertions remain green.
- Known-prior upgraded hooks remain executable.
- A safety-skipped project hook retains its original mode.
- No skipped hook is described as rewritten.

### Atomic unit

Permission behavior aligned with planner ownership and reporting.

## Step 7: document the operator contract

### Changes

- Expand README's `lisa init` section.
- State the exact-content ownership rule for static templates.
- State preservation and visible safety skip for unknown/unreadable files.
- State that `.lisa/.gitignore` adds only missing Lisa rules.
- State that existing ignore lines are never deleted, reordered, or rewritten.
- State that a real run reports exactly files it created or updated.
- Tell operators to inspect reported files before their next commit.

### Verification

- Read the rendered Markdown section for clarity and command accuracy.
- Search for contradictory claims about existing-file behavior.
- Run `git diff --check`.

### Atomic unit

CLI ownership, ignore, and audit documentation.

## Step 8: focused verification

Run:

```bash
cargo fmt --all -- --check
cargo test -p lisa-cli init::tests
cargo test -p lisa-cli
```

If a focused test fails:

- identify whether the failure reflects an intentional category change;
- update assertions only when they align with the documented policy;
- fix production behavior for any write-set, idempotence, or preservation
  mismatch;
- record deviations in `progress.md` before altering the plan.

## Step 9: full verification

Run:

```bash
cargo test --workspace
just check
git diff --check
```

Optionally run warning-strict clippy for the touched CLI surface if time permits,
while distinguishing known baseline findings from new diagnostics.

## Step 10: implementation progress artifact

Create and maintain `progress.md` with:

- completed steps;
- files changed;
- focused and full test counts/results;
- commits created;
- deviations and rationale;
- remaining work.

Update it after meaningful implementation units, not only at the end.

## Step 11: review artifact

Write `review.md` after implementation and verification. Include:

- outcome and acceptance-criteria mapping;
- exact files created, modified, or deleted;
- append-only algorithm and edge behavior;
- action/reporting semantics;
- vend workflow and secret-ignore regression coverage;
- focused and full test results;
- documentation change;
- known limitations and open concerns;
- suggested human review focus.

## Commit strategy

The shared worktree contains unrelated user and Lisa state. Stage only files
owned by this ticket. Intended commit units:

1. RDSPI research, design, structure, and plan artifacts.
2. Explicit action categories and append-only gitignore planner/tests.
3. Mutation reporting, writer capture, hook permission alignment, and tests.
4. README, final progress, verification updates, and review.

If tightly coupled refactors make two implementation units inseparable, combine
them and document the deviation rather than creating a broken intermediate
commit.

## Acceptance checklist

- [ ] Existing `.lisa/.gitignore` bytes remain an exact prefix.
- [ ] Only missing required rules are appended.
- [ ] No rule duplication with missing newline or surrounding spacing.
- [ ] Repeated real init is idempotent.
- [ ] `hooks/ntfy-topic` remains present and effective under `git check-ignore`.
- [ ] Dry-run shows create, update, no-op, and safety skip distinctly.
- [ ] Real-run plan shows the same distinct categories.
- [ ] Final report lists exactly successful created and updated files.
- [ ] Skipped and unchanged files are absent from the mutation report.
- [ ] Vend workflow customization remains byte-for-byte unchanged.
- [ ] README states ownership, append-only, reporting, and inspection contracts.
- [ ] Focused init tests pass.
- [ ] Full CLI and workspace suites pass.
- [ ] `progress.md` and `review.md` are complete.
- [ ] Ticket frontmatter is unchanged.

## Plan conclusion

The sequence first clarifies planner semantics, then implements the safe merge,
then connects actual successful writes to testable output. Each unit has focused
verification, and the final workspace pass validates that the localized CLI
changes do not affect core scheduling or plugin behavior.
