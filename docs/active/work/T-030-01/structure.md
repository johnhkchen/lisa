# Structure: ownership-aware init planning

## Change overview

The implementation remains inside `lisa-cli`. It adds an embedded legacy
template registry, centralizes static-template planning in `init.rs`, replaces
unsafe expectations with ownership-policy regressions, and records RDSPI work
artifacts. No core, plugin, CLI argument, or persistent metadata schema changes.

## Files created

### `crates/lisa-cli/data/legacy/rdspi-workflow-v0.2.md`

- Exact outgoing workflow bytes from the v0.2.0-v0.2.2 release generation.
- Included at compile time for ownership comparison.
- Preserves the original final newline and all Markdown content.
- Not copied to new projects directly; it is evidence only.

### `docs/active/work/T-030-01/research.md`

- Maps planner actions, template history, tests, and safety boundaries.

### `docs/active/work/T-030-01/design.md`

- Records evaluated ownership approaches and the exact-content decision.

### `docs/active/work/T-030-01/structure.md`

- Defines this file-level blueprint.

### `docs/active/work/T-030-01/plan.md`

- Sequences implementation and verification.

### `docs/active/work/T-030-01/progress.md`

- Tracks completed implementation units and deviations.

### `docs/active/work/T-030-01/review.md`

- Provides the final reviewer handoff after verification.

## Files modified

### `crates/lisa-cli/src/templates.rs`

Add private or crate-visible legacy constants adjacent to the current static
templates:

- `LEGACY_RDSPI_WORKFLOWS: &[&str]` references the legacy data file.
- `LEGACY_ON_IDLE_HOOKS: &[&str]` is empty unless history reveals distinct
  installed bytes.
- `LEGACY_ON_STOP_HOOKS: &[&str]` contains the v0.3 stop script.
- `LEGACY_ON_CLEAR_HOOKS: &[&str]` contains the v0.3 clear script.
- `LEGACY_ON_HEARTBEAT_HOOKS: &[&str]` contains the v0.3 heartbeat script.
- `LEGACY_ON_NOTIFY_HOOKS: &[&str]` is empty unless a distinct installed
  generation is required.
- `LEGACY_LISA_GITIGNORES: &[&str]` contains `signals/\n`.

The constants remain implementation details of init planning. Current template
constants retain their existing names and call sites. Template behavior tests
gain assertions that legacy entries differ from current content and contain the
expected identifying behavior.

Legacy shell templates are short enough to live as raw string constants near
their current forms. The much larger workflow snapshot lives under `data/legacy`
and uses `include_str!`, matching the existing current workflow organization.

### `crates/lisa-cli/src/init.rs`

Add a private helper:

```text
plan_owned_template(path, current, known_prior) -> InitAction
```

The helper owns the complete branch order:

1. If `path` is absent, return `CreateFile` with current content.
2. If reading succeeds and content equals current, return current no-op `Skip`.
3. If reading succeeds and content equals any known-prior entry, return
   `UpdateFile` with current content.
4. If reading succeeds but is unknown, return a preservation `Skip`.
5. If reading fails, return an unreadable preservation `Skip`.

Replace the inline workflow action branch with a helper call.

Represent hook definitions as tuples containing name, current content, and
known-prior slice. Each tuple is sent to the same helper.

Replace the inline Lisa gitignore branch with a helper call. T-030-02 can later
substitute an append-only merge at this one call site.

Leave directory, context, TOML, Claude JSON, and Codex JSON branches structurally
unchanged except for comments that name their policy where useful.

Within the test module:

- add action lookup/assertion helpers to reduce repetitive pattern matching;
- rename stale-template tests so `stale` means known prior, not arbitrary text;
- change arbitrary-content expectations from `UpdateFile` to safety `Skip`;
- add precise skip-reason assertions;
- add plan and execution byte-preservation fixtures;
- add known-prior upgrade fixtures;
- add malformed/non-UTF-8 preservation fixtures;
- retain fresh initialization and structured merge coverage.

## Files not modified

### `crates/lisa-cli/src/config.rs`

- Default configuration generation and version comparison remain unchanged.
- `.lisa.toml` continues to be merged in `init.rs`.

### `crates/lisa-cli/src/main.rs`

- No command-line options or output protocol changes are required.
- Existing action display automatically exposes new skip reasons.

### `crates/lisa-core/**`

- Ownership planning is not a shared ticket/DAG domain concept.

### `crates/lisa-plugin/**`

- The scheduler does not participate in init file writes.

### Ticket frontmatter

- `docs/active/tickets/T-030-01-ownership-aware-init-planning.md` is not edited.
- Lisa detects artifacts and manages phase/status transitions.

## Internal interfaces

### Current template interface

Existing constants remain the source for fresh installations:

- `RDSPI_WORKFLOW`
- `ON_IDLE_HOOK`
- `ON_STOP_HOOK`
- `ON_CLEAR_HOOK`
- `ON_HEARTBEAT_HOOK`
- `ON_NOTIFY_HOOK`
- `LISA_GITIGNORE`

### Historical template interface

Each current static target gets a companion slice, even if empty. This makes the
policy explicit at every helper call and prevents accidental fallback to “any
difference means stale.” The slices contain only distinct prior bytes.

Visibility should be `pub(crate)` if `init.rs` consumes constants defined in the
sibling `templates` module. They are not exported outside the binary crate.

### Planner helper interface

The helper accepts a `PathBuf` by value because every result owns the path. It
accepts `&str` content and creates an owned `String` only for create/update
actions. It accepts a slice rather than a version map because only membership is
needed for the safety decision.

No new `InitAction` variant is required. `Skip` plus a stable, specific reason
supports both display and tests.

## Policy matrix enforced by structure

| Path family | Existing behavior after change | Code boundary |
|---|---|---|
| docs/archive and docs/active dirs | create if absent | directory loop |
| `.lisa/hooks`, `.lisa/signals` | create if absent | directory loop |
| `CLAUDE.md`, `AGENTS.md` | preserve if present | dedicated branches |
| workflow Markdown | replace if proven pristine | shared helper |
| five hook templates | replace if proven pristine | shared helper |
| `.lisa/.gitignore` | replace if proven pristine | shared helper |
| `.lisa.toml` | format-aware preserving merge | config branch |
| Claude settings JSON | format-aware preserving merge | `merge_hooks` |
| Codex hooks JSON | format-aware preserving merge | `merge_codex_hooks` |

## Test component boundaries

### Static-template classifier tests

- Work through `plan_init_actions`, not a public helper.
- Assert variant, path, content when applicable, and skip reason.
- Cover current, known-prior, unknown readable, and non-UTF-8 input.

### Field regression fixture

- Begin from a fresh initialized temporary project or construct the relevant
  scaffold deterministically.
- Write a workflow with the stock template plus committed-style Story Layer and
  read-the-story additions.
- Clone the bytes before planning.
- Assert planning produces a preservation skip and performs no mutation.
- Run real non-dry init and assert exact byte equality afterward.

### Hook regression fixture

- Put project additions in an active hook and notification sample.
- Assert both plan preservation and real-run byte equality.
- Account for Unix chmod separately; content comparison is the acceptance
  boundary and executable hooks may receive required permissions.

### Known-prior upgrade tests

- Write the exact v0.3 stop, clear, heartbeat, and gitignore content.
- Assert `UpdateFile` carries current bytes.
- Run init for at least one representative and assert disk bytes become current.
- Write current content and assert the action remains a no-op.

### Error tests

- Non-UTF-8 bytes make `read_to_string` fail deterministically.
- Assert no `UpdateFile` exists and the reason identifies preservation/unreadable
  state.
- Existing malformed JSON tests remain the structured-error reference.

### Policy inventory test

- A focused test/comment table names every planner path and its category.
- Behavioral tests support each category rather than exposing a production enum
  used only for testing.

## Ordering constraints

1. Add historical evidence before switching planner branches, so the code always
   has a legitimate upgrade path.
2. Add the shared helper and migrate all static plain-text targets together.
3. Update unsafe old tests in the same code unit; they otherwise fail by design.
4. Add field and error regressions after the classifier compiles.
5. Run formatting and focused tests before full workspace verification.
6. Write `review.md` only from final verified state.

## Commit boundaries

- Commit RDSPI research/design/structure/plan artifacts as the completed
  pre-implementation unit.
- Commit historical templates, classifier, and focused regressions as the main
  implementation unit.
- Commit progress/review artifacts after verification if repository state allows
  an isolated commit without capturing unrelated user changes.

## Deletion plan

- No production files are deleted.
- Unsafe test assumptions are replaced, not merely disabled.
- No existing release templates are removed from current fresh-init sources.

## Resulting architecture

`plan_init_actions` remains the single planner, while a small helper becomes the
single safety gate for whole-file template replacement. `templates.rs` owns both
the bytes installed today and the distinct older bytes considered proof of an
unmodified Lisa installation. The executor remains intentionally simple because
all ownership decisions are complete before any write begins.
