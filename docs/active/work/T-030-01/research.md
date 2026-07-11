# Research: ownership-aware init planning

## Ticket and scope

- Ticket `T-030-01` is a critical bug in the `lisa init` upgrade path.
- Its current phase is `research`; no work artifacts existed when this pass began.
- The reported failure is loss of project-authored additions in
  `docs/knowledge/rdspi-workflow.md` during an upgrade.
- The ticket explicitly broadens the concern to every path considered by
  `plan_init_actions`, rather than permitting a workflow-specific exception.
- The central constraint is evidentiary: replacement is safe only when the
  existing bytes are an unmodified Lisa-installed template.
- Missing, deleted, corrupt, or otherwise unavailable metadata must not count as
  evidence that Lisa owns a file.

## Init entry points

- `crates/lisa-cli/src/main.rs` dispatches the `init` subcommand.
- `crates/lisa-cli/src/init.rs::run_init` validates the root, detects the project,
  obtains a complete action list from `plan_init_actions`, prints it, and either
  stops for `--dry-run` or executes the actions.
- Planning and execution are deliberately separated. Regression tests can inspect
  the action list without mutating a fixture, then call `run_init` to cover the
  real write path.
- `InitAction` has four variants: `CreateDir`, `CreateFile`, `UpdateFile`, and
  `Skip { reason }`.
- `run_init` writes `CreateFile` and `UpdateFile` content with `fs::write`.
- `Skip` is therefore the only existing representation for a safety-preserving
  decision on an existing plain-text file.
- After action execution on Unix, `run_init` applies executable permissions to
  the four active hook script paths if they exist. This chmod happens even when
  their content action was a skip; it does not change file bytes.

## Complete planner path inventory

### Directories

- Six documentation directories and two `.lisa` runtime directories use
  create-if-absent behavior.
- An existing path produces `Skip("already exists")`; an absent path produces
  `CreateDir`.
- The planner uses `Path::exists` and does not distinguish an existing file from
  a directory at this stage. Execution only creates absent paths.

### Context files

- `CLAUDE.md` is generated from detected project attributes on first init.
- `AGENTS.md` is a static pointer to `CLAUDE.md`.
- Both use preserve-if-present behavior today: any existing path is skipped and
  never read or replaced.
- Their skip reason is `already exists`, which describes state rather than an
  ownership decision, but their write policy is already conservative.

### Workflow document

- `docs/knowledge/rdspi-workflow.md` is backed by
  `templates::RDSPI_WORKFLOW`, compiled from
  `crates/lisa-cli/data/rdspi-workflow.md` with `include_str!`.
- An absent file is created.
- Exact current-template bytes produce `Skip("already up to date")`.
- Every other result, including different readable content and any read error,
  currently produces an unconditional `UpdateFile` with current template bytes.
- This is the direct path that erased the vend project additions.

### Lisa configuration

- `.lisa.toml` is generated from `config::default_config_toml` when absent.
- Existing readable content is parsed to discover its Lisa version.
- Planning updates the version line and appends missing known scheduling keys.
- Existing settings and unrelated text are retained by the transformation.
- Malformed TOML is not replaced wholesale: parsing failure only makes the
  version look stale, after which the textual version/key transforms run.
- An unreadable file is skipped with `exists but unreadable`.
- This path is a format-aware/text-preserving merge rather than template
  replacement, although malformed input deserves explicit regression coverage.

### Hook scripts and sample

- Five static plain-text templates are considered:
  `on-idle.sh`, `on-stop.sh`, `on-clear.sh`, `on-heartbeat.sh`, and
  `on-notify.sample`.
- All are created when absent and skipped on exact current-template equality.
- Any differing bytes or read error currently cause unconditional replacement.
- `on-notify.sample` is intentionally non-executable and is described as a user
  opt-in sample. The active `on-notify` copy is not considered by init.
- The four active scripts are later chmodded to `0755` on Unix.
- The current stop script gained usage capture after the v0.3 generation.
- The current clear and heartbeat scripts changed wording to be client-neutral.
- The idle and notification templates did not change across the v0.3-to-current
  comparison, so current equality already handles those installations.

### `.lisa/.gitignore`

- The file is a static plain-text template and currently follows the same unsafe
  absent/current/different replacement pattern as workflow and hooks.
- v0.3 used `signals/`; current content additionally has `claude/` and `codex/`.
- Story S-030 records a field failure where replacement removed a project rule
  for `hooks/ntfy-topic`.
- Ticket T-030-02 will make ignore handling append-only and add mutation reports.
- Until that follow-up, this ticket still requires an explicit safe policy for
  the path because it is part of the complete action set.

### Claude and Codex hook configuration

- `.claude/settings.local.json` is generated when absent.
- Existing JSON is passed through `templates::merge_hooks`.
- `.codex/hooks.json` is similarly passed through `merge_codex_hooks`.
- Both merge functions parse the document, ensure Lisa hook entries, preserve
  unrelated keys and hook groups, and pretty-print the result.
- The planner compares parsed old and new JSON values to avoid formatting-only
  writes.
- Malformed or unreadable JSON is skipped with a specific manual-action reason.
- These paths already implement format-aware merge behavior and never fall back
  to wholesale replacement on an error.

## Template history available in the repository

- Release tags from v0.2.0 through v0.4.0-rc.5 are present locally.
- The workflow data has two distinct tagged blobs: the v0.2.0-v0.2.2 generation
  and the v0.2.3-and-later generation.
- The v0.3.0 workflow bytes equal the current embedded workflow bytes. The field
  regression was therefore not a legitimate template upgrade; local additions
  made it differ and triggered replacement.
- The v0.3 template source records the exact earlier hook scripts and the earlier
  one-line Lisa gitignore.
- Git history is development evidence, but an installed CLI cannot query the
  source repository. Any ownership proof used at runtime must be compiled into
  the binary or derived from project content that remains independently safe.
- Exact content comparison is already the planner's equality convention and is
  byte-sensitive, including final newlines.

## Existing tests and observable gaps

- Unit tests are colocated in `crates/lisa-cli/src/init.rs` and use temporary
  directories as integration-style fixtures.
- Fresh init tests assert the expected number of created paths and inspect the
  real files written by `run_init`.
- Existing tests assert that `CLAUDE.md` and `AGENTS.md` are never overwritten.
- Existing JSON tests cover merging, preservation of custom hooks, malformed
  documents, and idempotence.
- Existing TOML tests cover stale versions, missing key upserts, custom active
  values, comments, and no-op behavior.
- Existing plain-template tests encode the unsafe expectation: arbitrary strings
  called `old` or `stale` must become `UpdateFile` actions for workflow and hooks.
- `test_run_init_updates_stale_hooks` likewise expects arbitrary hook content to
  be overwritten.
- There is no fixture that distinguishes a known historical Lisa template from
  locally modified or unclassified text.
- There is no byte-for-byte plan-plus-execution regression for a customized
  workflow.
- Read-error behavior for workflow, hooks, and Lisa gitignore is not protected.

## Boundaries and constraints

- The change belongs in `lisa-cli`; no core DAG, plugin scheduler, or public
  ticket type is involved.
- `InitAction` can express the required result without a new executor action.
- The planner is synchronous and uses standard filesystem reads throughout.
- Adding a metadata ledger would introduce lifecycle, corruption, migration, and
  trust questions contrary to the ticket note; content recognition avoids those
  dependencies.
- An exact historical template match proves that the bytes are consistent with
  an unmodified Lisa installation. It cannot prove how the file was originally
  created, but replacement is content-preserving relative to that known state.
- A partial match, recognizable header, version in `.lisa.toml`, or path name is
  weaker evidence and cannot distinguish project edits.
- Safety skip reasons are visible in dry-run and real-run plans through
  `InitAction`'s `Display` implementation.
- T-030-02 owns append-only ignore semantics and detailed mutation reporting, so
  this ticket should not consume that ticket's reporting scope.

## Research conclusion

- The unsafe behavior is a repeated policy branch, not a workflow-only defect.
- Create-only and structured-merge paths are already conservative in their core
  behavior.
- Static plain-text targets need a shared ownership decision with three outcomes:
  current/no-op, known-prior/update, and unknown-or-unreadable/preserve.
- Historical bytes available in release history are sufficient to establish a
  concrete initial registry without trusting mutable project metadata.
