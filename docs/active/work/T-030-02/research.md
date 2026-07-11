# Research: append-only ignore and mutation report

## Ticket state and scope

- Ticket `T-030-02` is in `phase: research` with critical priority.
- It depends on completed ticket `T-030-01`.
- The requested work spans the CLI initializer, its tests, and operator-facing CLI
  documentation.
- The ticket explicitly forbids a second ownership classifier.
- Ticket and story frontmatter are managed by Lisa and are not implementation
  surfaces for this work.
- The required final behavior has two connected concerns:
  append-only `.lisa/.gitignore` upgrades and an auditable init mutation report.

## Repository and crate boundaries

- Lisa is a Rust workspace with `lisa-core`, `lisa-plugin`, and `lisa-cli` crates.
- `lisa-cli` owns `init`, template generation, validation, and CLI output.
- Init implementation and most init tests are colocated in
  `crates/lisa-cli/src/init.rs`.
- Bundled static content and structured JSON merge helpers live in
  `crates/lisa-cli/src/templates.rs`.
- CLI argument parsing and dispatch live in `crates/lisa-cli/src/main.rs`.
- The public CLI reference is in the root `README.md`.
- The hook-oriented manual is embedded from
  `crates/lisa-cli/data/hooks-guide.md`, but it is not the primary general init
  contract.

## Existing init pipeline

- `run_init(root, dry_run)` first rejects a nonexistent root.
- It detects the project type with `detect_project`.
- It computes every action with `plan_init_actions` before any write occurs.
- It prints the complete planned action list.
- A dry run prints `Dry run complete. No changes made.` and returns.
- A real run iterates over the same action vector.
- `CreateDir` calls `fs::create_dir_all`.
- `CreateFile` creates its parent directory if needed, then calls `fs::write`.
- `UpdateFile` calls `fs::write` directly.
- `Skip` performs no filesystem content write.
- After action execution on Unix, four active hook scripts are chmodded to `0755`
  whenever they exist.
- The command currently ends with `Initialization complete.` and generic next
  steps.
- It does not retain or print a post-execution file write set.

## Existing action model

- `InitAction` has four variants: `CreateDir`, `CreateFile`, `UpdateFile`, and
  `Skip`.
- `CreateFile` and `UpdateFile` carry the complete target content.
- `Skip` carries a path and free-form reason string.
- Display labels are `create`, `update`, and `skip`.
- Existing-object no-ops and safety preservation both use `Skip`.
- No-op reasons include `already exists` and `already up to date`.
- Safety reasons include unreadable content, malformed JSON, and content that is
  not a known Lisa template.
- The reason text communicates the distinction to a reader, but the action label
  does not distinguish no-ops from safety skips.
- Tests commonly pattern-match directly on the enum and reason text.

## Ownership policy inherited from T-030-01

- `plan_owned_template(path, current, known_prior)` governs static whole-file
  templates.
- An absent path becomes `CreateFile(current)`.
- Exact current content becomes an already-up-to-date `Skip`.
- Exact known-prior content becomes `UpdateFile(current)`.
- Unknown readable content is preserved with a specific safety reason.
- Unreadable or non-UTF-8 content is preserved with a specific safety reason.
- The workflow and five hook templates must continue to use this policy.
- `.lisa/.gitignore` currently also uses this helper.
- T-030-01 deliberately documented `.lisa/.gitignore` as pending the stronger
  T-030-02 append-only policy.
- Structured TOML and JSON files use separate format-aware preserving merges.

## Current `.lisa/.gitignore` template

- `templates::LISA_GITIGNORE` is `signals/\nclaude/\ncodex/\n`.
- `templates::LEGACY_LISA_GITIGNORES` contains the historical one-line
  `signals/\n` template.
- The current planner creates the full current template when the file is absent.
- It replaces the exact legacy template with the full current template.
- It no-ops on exact current bytes.
- It safety-skips every customized file, including a file with
  `hooks/ntfy-topic`.
- That preservation prevents deletion, but it also prevents adding newly
  required `claude/` and `codex/` rules to customized installations.
- The legacy-template registry is useful for static replacement ownership but is
  not needed to establish permission to append missing ignore rules.

## Ignore-file semantic constraints

- The acceptance criteria require preservation of every existing line.
- Existing lines may not be deleted, reordered, or rewritten.
- Missing Lisa-required rules must be appended only.
- Repeated init must not duplicate required rules.
- Duplicate detection must tolerate harmless surrounding spacing.
- A file without a trailing newline must receive a separator before appended
  rules.
- Preserving bytes before the append boundary is stronger than reconstructing
  the file from parsed lines.
- `str::lines()` is suitable for comparison but not for reconstructing the
  original prefix because it normalizes line endings and omits terminators.
- Existing invalid UTF-8 cannot be safely parsed by the current text-based
  content model and therefore remains a safety-skip boundary.
- Git ignore matching can be verified using `git check-ignore` in a temporary Git
  repository.

## Field regression fixture

- Story S-030 describes a vend upgrade from 0.3.0 to 0.4.0-rc.5.
- The workflow had committed Story Layer/read-the-story customizations.
- `.lisa/.gitignore` had the project-specific `hooks/ntfy-topic` rule.
- T-030-01 already has a combined fixture with customized workflow, hook/sample,
  and ignore content.
- That test currently expects the customized ignore to be a safety skip.
- T-030-02 must revise this expectation: workflow and other unknown templates
  remain safety skips, while the ignore file receives only missing required
  rules.
- The real-run fixture can assert both byte preservation of the workflow and
  exact append behavior of the ignore file.
- `git check-ignore .lisa/hooks/ntfy-topic` can verify that the secret path stays
  ignored after the upgrade.

## Mutation reporting gap

- The dry-run action list already exposes planned creates, updates, and skips.
- It does not use a distinct no-op label.
- A real run prints the same plan before writing.
- After writing, it does not print the successful file mutations.
- Therefore an operator cannot distinguish the exact completed file write set
  from the larger plan without rereading and inferring action types.
- A write failure returns immediately, so any final report must not claim actions
  that were not successfully completed.
- Directories are filesystem mutations but the acceptance criterion asks for the
  exact set of files created or modified.
- `CreateFile` and `UpdateFile` are the only planned content-write variants.
- Recording a path only after its successful `fs::write` makes the report match
  the completed file write set.
- Skips must never enter that record.
- Directory creation must not be presented as a rewritten file.
- Hook chmod may mutate metadata even for skipped content, but it is not a file
  create/modify content action in the current planner or ticket wording.

## Output testability

- `run_init` writes directly with `println!`, so unit tests cannot inject a
  buffer into the current function.
- Existing tests validate filesystem outcomes and action classifications rather
  than captured command output.
- A small rendering helper can make action/report text testable without spawning
  the compiled binary.
- Alternatively, `run_init` can gain an internal writer-taking implementation
  while retaining its public wrapper.
- The CLI crate currently has no general integration-test harness for `lisa init`
  command output.
- The action enum itself can provide stable labels used by both dry-run and real
  output.

## Existing tests relevant to the change

- Fresh-init tests assert twenty non-skip actions: eight directories and twelve
  files.
- Fresh init asserts `.lisa/.gitignore` exists and contains `signals/`.
- Known-prior template tests currently include the historical gitignore as an
  `UpdateFile`.
- Current-template tests expect every static template, including gitignore, to be
  an already-up-to-date `Skip`.
- The project-modified fixture asserts every customized plain-text target is
  preserved byte-for-byte after planning and real init.
- Non-UTF-8 tests cover workflow and a hook, but not gitignore.
- The full CLI test suite is housed largely in module unit tests and runs with
  `cargo test -p lisa-cli`.
- Workspace verification runs with `cargo test --workspace` or `just check`.

## Documentation state

- README's `lisa init` section currently says only that init scaffolds the
  project and shows basic invocations.
- It does not describe static-template ownership proof.
- It does not state that customized or unclassifiable content is preserved.
- It does not state that `.lisa/.gitignore` is append-only.
- It does not tell operators to inspect the reported mutations before committing.
- The hook guide says init is safe to re-run but does not define the ownership or
  audit contract.

## Constraints and assumptions

- The implementation must build on the T-030-01 action planner rather than
  bypassing it with an independent upgrade command.
- Planning must stay read-only.
- A real run must execute precisely the planned create/update content actions.
- Existing project rules are the source of truth and must survive byte-for-byte.
- Missing required rules can be derived from the current Lisa gitignore template.
- Output should remain readable for both fresh initialization and upgrades.
- Ticket phase and status frontmatter must remain untouched.
- Unrelated dirty worktree entries belong to the user and are out of scope.

## Research conclusion

The change is localized to init planning/execution, tests, and CLI documentation.
The critical implementation seam is replacing only the gitignore call to
`plan_owned_template` with an append-only merge that preserves the existing byte
prefix. The reporting seam is the existing execution loop: successful file
writes can be collected there and rendered after execution. The current `Skip`
variant conflates ordinary no-ops with safety preservation at the label level,
so the design phase must decide whether to refine the enum or add an equally
explicit presentation mechanism without weakening the ownership policy already
established by T-030-01.
