# Design — T-050-01-02 never-a-dead-end surfaces

## Design goals

- Make every named first-contact failure lead with one exact setup sentence.
- Keep host- and filesystem-specific detail after the plain sentence.
- Render every intentional empty state as a short human sentence.
- Preserve populated and valid-project output byte for byte where applicable.
- Keep project detection early enough to avoid doctor or loop side effects.
- Keep custom ticket-directory configuration visible in validation guidance.
- Make the clean empty-board exit contract explicit and string-pinned.
- Add no flags, commands, configuration, or public library surface.

## Decision 1: where pre-init detection runs

### Option A — let each command fail naturally, then prefix its error

- `loop_cmd::run_loop` already checks `CLAUDE.md` and the ticket directory.
- `status::run_status` already errors for a missing ticket directory.
- `validate` already returns multiple missing-structure diagnostics.
- `doctor` already knows whether its project-version check is skipped.
- Each module could wrap its existing failure with the setup sentence.
- This preserves command-specific technical detail precisely.
- It duplicates the exact sentence and classification across four modules.
- Doctor runs dependency checks and cache cleanup before it knows final status.
- Validate with `--check-tools` can report host failures before project setup.
- Output ordering between stdout and stderr can undermine a shared first line.
- The approach does not establish one testable definition of pre-init.
- This option is rejected because first contact must be deterministic and early.

### Option B — add a shared preflight module used by command modules

- A new module could expose `require_project(root)`.
- Each command implementation could invoke it as its first operation.
- Direct module unit tests would exercise the same boundary as the binary.
- The exact sentence would remain centralized.
- A new module is unnecessary for one private predicate and one renderer.
- Many direct unit fixtures intentionally model only part of initialized state.
- Reclassifying those fixtures would cause broad unrelated test churn.
- The CLI process contract, including exit code and stderr, still lives in `main`.
- This option is viable but creates more surface than this copy-focused ticket needs.

### Option C — preflight the four project-aware command arms in `main.rs`

- Resolve `--path`, then test for Lisa project markers before module invocation.
- On pre-init, print the exact setup lead and one technical-detail line.
- Exit 1 without entering the command module.
- Existing module behavior remains unchanged for project-like roots.
- Doctor cannot perform checks, cleanup, or trust writes first.
- Loop cannot resolve completion, tools, or runtime first.
- Validate cannot lead with optional tool failures first.
- Status cannot lead with its ticket-directory implementation detail first.
- The process-level output contract is easy to black-box test.
- `init` and non-project plumbing remain unaffected.
- This option is selected.

## Decision 2: what counts as an untouched folder

### Option A — missing `.lisa.toml` alone

- `.lisa.toml` is created by every successful modern `lisa init`.
- It is the strongest explicit Lisa configuration marker.
- A missing file is not necessarily an untouched project.
- Config loading intentionally supports defaults when the file is absent.
- Older and focused test fixtures omit it while carrying Lisa ticket structure.
- A deleted config in an otherwise recognizable project should retain diagnostics.
- This option is too strict.

### Option B — missing `CLAUDE.md` alone

- `loop` currently uses `CLAUDE.md` as its first setup check.
- Many non-Lisa projects can have a user-authored `CLAUDE.md`.
- Such a folder would still produce a bare Lisa ticket-directory error.
- This option is not Lisa-specific enough.

### Option C — no config and no default Lisa ticket directory

- Treat a root as project-like when `.lisa.toml` exists.
- Also treat it as project-like when `docs/active/tickets` exists.
- A custom configured project is recognized through `.lisa.toml`.
- A default-layout project with a lost config remains recognizable.
- Existing partial fixtures with a ticket board retain current behavior.
- A generic repository with only source or `CLAUDE.md` gets setup guidance.
- Partial projects continue to receive their specific repair diagnostics.
- This option is selected.

## Decision 3: pre-init error rendering

### Considered formats

- Returning the setup sentence through the existing `Error: {detail}` wrapper.
- Printing `Error: This folder isn't set up yet...` as one line.
- Printing the exact setup sentence first, then a technical-detail paragraph.
- Printing only the setup sentence with no technical detail.

### Selected format

```text
This folder isn't set up yet. Run: lisa init

Technical detail: Lisa couldn't find .lisa.toml or docs/active/tickets/ in <root>.
```

- The first byte of stderr begins the required sentence.
- There is no generic `Error:` prefix before the brand-voice lead.
- The second paragraph explains the predicate and exact inspected root.
- The command exits 1.
- No stdout is produced.
- Tests pin the complete stable prefix and exit code.
- Path rendering after the prefix can vary with the temporary directory.

## Decision 4: empty notes behavior

### Option A — change `note_lines(&[])`

- It could return `vec!["Nothing to read."]`.
- `print_notes` would then render the standalone sentence automatically.
- Status also calls `print_notes`, so it would gain an unlabelled sentence.
- An unlabelled status sentence is not a legible named section.
- Formatter tests currently define empty `note_lines` as empty.
- This option conflates standalone and embedded contexts.

### Option B — change `print_notes`

- It could print `Nothing to read.` for an empty slice.
- Standalone `run_list` would meet its contract.
- Status would print the sentence without a `Notes for you` heading.
- Populated behavior would remain unchanged.
- Embedded context would remain ambiguous.
- This option is rejected for the same context problem.

### Option C — handle empty in each caller

- Keep `note_lines` and `print_notes` as populated-note primitives.
- `run_list` prints `Nothing to read.` when collection is empty.
- Status prints a named `Notes for you` empty section when empty.
- Both callers reuse the byte-identical populated printer otherwise.
- This option is selected.

## Decision 5: empty status sections

### Output shape

```text
Waiting on you
Nothing waiting.

Notes for you
Nothing to read.

DAG: ...
```

- The existing order remains Waiting, Notes, DAG.
- Each absent section has a direct sentence.
- `Nothing waiting.` distinguishes action-blocking parks from deferred notes.
- `Nothing to read.` reuses the standalone notes empty sentence.
- A trailing blank line preserves separation before the next section.
- Populated Waiting and Notes sections retain their exact current formatting.

### Alternatives

- A combined `Nothing waiting or unread.` line is shorter but erases semantics.
- Inline headings such as `Waiting on you: none` change the established heading form.
- Omitting the Notes empty section leaves one conditional blank unexplained.
- Emitting empty sections after the DAG weakens their operator-first ordering.
- The two-section shape is selected.

## Decision 6: empty validate exit contract

### Option A — retain exit 1 with improved guidance

- Empty remains a readiness error.
- Existing scripts continue seeing a nonzero status.
- The grader still needs a synthetic smoke ticket to obtain a scorable exit.
- A new board is not malformed; it simply has no scheduled work yet.
- The nonzero status makes guidance read like a failure.
- This option does not address the field motivation.

### Option B — exit 0 with guidance

- A fully initialized, clean, zero-ticket board is structurally valid.
- Validation can explain what a ticket is and where to add one.
- Automation can score project setup without creating fake work.
- A ticket parse failure still remains nonzero even if zero tickets parse.
- Missing setup files, hooks, or directories remain nonzero.
- `lisa loop` is not recommended until a ticket exists.
- The test name can document the intentional success contract.
- This option is selected.

## Decision 7: zero-ticket validation representation

### Option A — keep an Error diagnostic and special-case the exit

- The renderer could ignore that one error when it is the only error.
- Error counts and severity would no longer agree with process success.
- Other code reading `ValidationResult::has_errors` would be misleading.
- This option is rejected.

### Option B — downgrade no-tickets to a Warning diagnostic

- `print_diagnostics` would print a structured warning before guidance.
- Output would not be the requested one short paragraph.
- The warning category syntax is more technical than first-contact copy.
- This option is rejected.

### Option C — return a clean zero-count result and branch in `run_validate`

- `validate` returns after a successful empty scan without adding a diagnostic.
- Preexisting diagnostics remain in the result.
- Parse/setup/tool errors still make `has_errors()` true.
- `run_validate` first renders errors when present.
- On clean `ticket_count == 0`, it prints only the guidance paragraph and returns.
- Otherwise it uses the existing success and config-summary path unchanged.
- This option is selected.

## Empty validate copy

```text
No tickets yet. A ticket is a Markdown file that tells Lisa what work to schedule; put one in docs/active/tickets/, then run `lisa validate` again.
```

- The paragraph defines the unit in plain language.
- It names the configured directory, including custom configuration.
- It explains the next validation step.
- It avoids saying all checks passed and then suggesting an empty loop.
- It is one physical output line followed by one newline.
- The default path is string-pinned in the black-box test.

## Test design

- Add one focused integration fixture for first-contact and empty-state output.
- Invoke the compiled binary using `CARGO_BIN_EXE_lisa`.
- Use a truly untouched temporary directory for each pre-init command.
- Test `loop --dry-run` so any missing guard cannot launch Zellij.
- Test `status`, `validate`, and `doctor` with the same empty-root contract.
- Assert status 1, empty stdout, and the exact required stderr lead.
- Use `lisa init --no-history` to create a complete empty project fixture.
- Assert empty `lisa notes` is exactly `Nothing to read.\n` with exit 0.
- Assert empty `lisa validate` is the exact paragraph with exit 0.
- Assert empty-board status includes both named empty sections in order.
- Add module-level exact assertions where pure formatting can be isolated.
- Retain existing non-empty notes and parked-status tests as regression snapshots.
- Run the focused integration test, CLI crate tests, then the workspace suite.

## Rejected expansion

- Do not change `docker/chromebook-test/bin/grade` in this ticket.
- Do not teach `loop --dry-run` a new zero-ticket paragraph.
- Do not add a public `Project` abstraction.
- Do not modify Clap help or examples.
- Do not change dashboard empty states; the ticket names CLI surfaces.
- Do not alter ticket scanning or DAG semantics.
- Do not write phase artifacts into `docs/active/work` directly.

## Final design decision

- Centralize untouched-folder detection and rendering privately in `main.rs`.
- Guard the four named project-aware command arms before module work.
- Preserve populated notes helpers and render emptiness in their callers.
- Render both optional status sections even when each is empty.
- Treat a clean empty board as valid and exit 0 with one guidance paragraph.
- Pin every changed sentence and process status in focused tests.
