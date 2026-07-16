# Structure: purpose-first CLI strings

## Change boundary

This ticket changes three user-facing string ownership points and one integration-test file.
No module, command, type, or public interface is added or removed.
No source file is created or deleted.
The phase artifacts in this attempt directory are the only new non-source files.

## Modified source files

### `crates/lisa-cli/src/main.rs`

Role before the change:

- Declares the Lisa CLI with Clap derives.
- Owns top-level help metadata.
- Owns operator, plumbing, and hidden command definitions.
- Dispatches parsed commands to their implementations.

Ticket-owned modification:

- Replace only the top-level `about` string.
- Use the selected purpose-first sentence.
- Leave `name`, `before_help`, `after_help`, and `version` untouched.
- Leave the `Cli` struct unchanged.
- Leave all `Commands` variants unchanged.
- Leave all `display_order` and `hide` attributes unchanged.
- Leave dispatch unchanged.

Resulting boundary:

- Clap still controls the exact help layout.
- The first descriptive line now includes function, work source, and reduced-approval outcome.
- The procedural `before_help` line still appears first but contains no mechanism term.
- The purpose line still precedes all mechanism vocabulary in complete output.

### `crates/lisa-cli/src/setup_guide.rs`

Role before the change:

- Detects project type and name.
- Composes the setup-guide header.
- Builds seven ordered setup sections.
- Renders the full guide to a string.
- Prints the generated guide.
- Contains unit tests for guide content and shape.

Ticket-owned modification:

- Extend the `header` format string in `build_guide`.
- Keep the project-specific Markdown H1 unchanged.
- Insert the purpose statement as the first prose after the H1.
- Preserve the existing instructions after the new purpose statement.
- Do not alter the sections vector.
- Do not alter `render_guide`.
- Do not alter project detection.
- Do not alter guide section bodies.

Resulting boundary:

- Header remains project-specific.
- Purpose precedes setup mechanics and later DAG/scheduling language.
- Step numbering remains one through seven.
- `run_setup_guide` still prints the `build_guide` return value unchanged.

### `crates/lisa-cli/data/hooks-guide.md`

Role before the change:

- Canonical hooks-guide content.
- Included at compile time as `templates::HOOKS_GUIDE`.
- Documents lifecycle hooks, signals, notifications, setup, and repair.

Ticket-owned modification:

- Keep the Markdown H1 unchanged.
- Insert the purpose statement as the first prose below the title.
- Keep the existing setup/repair paragraph after it.
- Do not alter headings, tables, contracts, examples, or hook details.

Resulting boundary:

- The raw Markdown file and emitted CLI output agree.
- Purpose precedes hooks mechanism detail.
- Compile-time inclusion remains unchanged.

### `crates/lisa-cli/tests/help_surface.rs`

Role before the change:

- Black-box regression lock over the built Lisa CLI.
- Pins all thirteen command names.
- Snapshots top-level and operator help.
- Verifies operator/plumbing/internal grouping.
- Rejects configured jargon in operator-facing help.

Ticket-owned modifications:

- Update only the about-line text inside `TOP_LEVEL_HELP_SNAPSHOT`.
- Add an exact purpose sentence constant for semantic assertions.
- Add the four mechanism terms named by this ticket.
- Add a helper asserting purpose appears before the earliest mechanism term.
- Add a test invoking the three required installed surfaces.
- Use `help_stdout(&["--help"])` for top-level help.
- Use `help_stdout(&["setup-guide"])` for the generated guide.
- Use `help_stdout(&["hooks-guide"])` for the embedded guide.
- Leave operator command arrays and snapshots unchanged.
- Leave plumbing and hidden command arrays unchanged.
- Leave existing jargon checks unchanged.

Resulting boundary:

- Exact snapshot coverage catches any unintended top-level byte drift.
- Semantic coverage describes why the wording order matters.
- Real dispatch coverage proves hidden guide commands print purpose-first text.
- Existing structural coverage continues to guard E-044 behavior.

## Test helper shape

The helper is private to the integration-test module.
It accepts a surface label and output string.
It lowercases the output for stable case-insensitive matching.
It lowercases the purpose anchor once per call.
It requires the purpose anchor to exist.
It scans all four mechanism terms.
It retains the earliest `(offset, term)` pair.
It requires at least one named mechanism term in each current output.
It asserts purpose offset is smaller than mechanism offset.
Its failure text identifies the surface and first mechanism term.

The helper is not production code.
It introduces no dependency.
It does not parse Markdown or Clap formatting.
It checks semantic ordering in the exact emitted bytes.

## Purpose string placement

Production forms:

- Top-level Clap: `Runs coding agents through your ticket board, so you don't have to approve every step by hand.`
- Setup guide: `Lisa runs coding agents through your ticket board, so you don't have to approve every step by hand.`
- Hooks guide: `Lisa runs coding agents through your ticket board, so you don't have to approve every step by hand.`

Test anchor:

- Lowercase matching makes the leading `Runs`/`runs` difference irrelevant.
- The anchor omits `Lisa ` so it occurs in all three forms.
- Punctuation remains exact to prevent a weakened partial match.

## Preserved interfaces

- Executable name remains `lisa`.
- CLI invocation syntax remains unchanged.
- All thirteen existing subcommands remain resolvable.
- Five operator commands remain visible in their canonical order.
- Five plumbing commands remain in the curated footer.
- Three internal commands remain hidden.
- Setup-guide still accepts `--path` with the same default.
- Hooks-guide still accepts no arguments.
- `templates::HOOKS_GUIDE` remains `&'static str`.
- `run_setup_guide` and `run_hooks_guide` retain their signatures.
- Error behavior for nonexistent setup-guide paths remains unchanged.

## Ordering of source changes

1. Update production copy at all three ownership points.
2. Update the exact top-level help snapshot.
3. Add semantic ordering coverage for all three outputs.
4. Format Rust sources.
5. Run the focused integration test.
6. Run CLI unit tests and broader workspace verification.
7. Inspect the diff for wording-only production changes.
8. Commit the meaningful source unit with exact ticket-owned paths.

Production and regression tests form one meaningful source unit because the acceptance criterion couples them.
The exact include list contains the four modified repository source/test/data paths.
Attempt artifacts are excluded from this source commit.

## Files explicitly outside scope

- `crates/lisa-core/**`.
- `crates/lisa-plugin/**`.
- CLI scheduler and loop runtime modules.
- README installation and product copy.
- Website or landing-page content.
- Generated Cargo build output.
- Active ticket frontmatter.
- Shared `docs/active/work/T-046-07-01/` publication directory.
- Unrelated active tickets and work artifacts already present in the worktree.

## Review invariants

- A production diff should show strings only.
- The test diff may add constants, a helper, and one test.
- No command enum lines should move.
- No command description should change other than the top-level about string.
- Setup numbered sections should be byte-identical.
- Hooks-guide content after its opening insertion should be byte-identical.
- Focused tests should observe purpose before DAG, WASM, Zellij, or scheduling.
- Git status after `lisa commit-ticket` should show no ticket-owned source changes.
- Unrelated pre-existing worktree changes may remain and must not be included.
