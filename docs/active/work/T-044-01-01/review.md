# Review: orient and separate help

## Disposition

Ready to complete.

The ticket acceptance criterion is met. `lisa --help` now opens with the
everyday path, keeps the five operator commands in the generated primary list,
and presents the four machinery-facing commands in a separately labeled
plumbing footer. Black-box tests snapshot the full screen and explicitly reject
plumbing leakage into the operator section.

No blocking defect or required follow-up remains in this ticket.

## Acceptance criterion evaluation

### Orientation line

The first rendered line is:

`Everyday path: init → validate → status → loop`

This names all four required everyday steps in the required order.

The established product about line remains directly below the orientation:

`Runs your coding agents through a project's tickets.`

The path is supplied through top-level Clap `before_help` metadata. It does not
alter command parsing or dispatch.

Result: pass.

### Operator and plumbing separation

The generated `Commands:` list now contains:

- `init`
- `validate`
- `status`
- `doctor`
- `loop`
- Clap's built-in `help`

The four plumbing variants are hidden only from that generated list. A distinct
footer titled `Plumbing commands (called by Lisa and agent hooks):` contains:

- `agent-exec`
- `capture-usage`
- `commit-ticket`
- `complete-ticket`

The footer follows the generated `Options:` block, making it visibly secondary
to the everyday operator surface.

All four commands remain direct parser entry points. The existing resolution
test invokes `<command> --help` for each and passes.

Result: pass.

### Snapshot regression

`crates/lisa-cli/tests/help_surface.rs` now contains an inline exact stdout
snapshot for `lisa --help`.

The snapshot includes:

- the new orientation;
- the product about line;
- usage;
- the generated operator command list;
- generated options;
- the plumbing heading;
- all four plumbing rows;
- exact whitespace and final newline.

`top_level_help_matches_snapshot` runs the built binary through
`CARGO_BIN_EXE_lisa` and compares the complete captured stdout.

The snapshot is part of ordinary `cargo test -p lisa-cli`; it is not ignored,
feature-gated, or dependent on a live external service.

Result: pass.

### Plumbing re-entry guard

The older test only required plumbing command offsets to occur after `loop`.
That test passed even when all commands shared one undifferentiated list.

The replacement test splits top-level help at the exact plumbing heading. It
then:

- requires every operator command in the prefix;
- rejects every plumbing command from the prefix;
- requires every plumbing command in the suffix;
- rejects each internal guide/version command from the complete help screen.

If any plumbing variant loses `hide = true`, its generated row appears in the
prefix and the test fails with a command-specific leakage message. If a footer
row disappears, the same test fails with a command-specific missing-row
message.

Result: pass.

## Source changes

### `crates/lisa-cli/src/main.rs`

Modified only Clap help metadata:

- added the everyday-path `before_help` line;
- added the curated multiline plumbing `after_help` block;
- added `hide = true` to `AgentExec`;
- added `hide = true` to `CaptureUsage`;
- added `hide = true` to `CommitTicket`;
- added `hide = true` to `CompleteTicket`.

The existing `display_order` values remain in place.

No variant payload, argument attribute, match arm, module call, or error path
changed.

### `crates/lisa-cli/tests/help_surface.rs`

Modified the existing black-box help regression suite:

- extended its documented scope to S-044-01;
- added the plumbing-heading constant;
- added the inline top-level snapshot;
- added the full-output snapshot test;
- strengthened category assertions around the real heading boundary;
- changed the about lookup to find the `coding agents` masthead after the new
  first line;
- renamed stale hook/lower-band test terminology to plumbing/footer
  terminology;
- retained the twelve-command resolution check;
- retained the operator jargon checks.

No new test dependency or fixture file was added.

## Commit record

Ticket-owned source changes were committed through Lisa's isolated transaction
with exact repository-relative include paths.

### Production help unit

Commit:

`6a0fff1254e65e7a595027de9a7aae67a1d61db7`

Message:

`T-044-01-01: orient and separate top-level help`

Only included:

`crates/lisa-cli/src/main.rs`

### Snapshot and structural test unit

Commit:

`6698c12aa0784836a88501013fbaab0419c3f227`

Message:

`T-044-01-01: snapshot the separated help surface`

Only included:

`crates/lisa-cli/tests/help_surface.rs`

### Review cleanup unit

Commit:

`7ed3c24609df5a038b86f30fb104ca26e36bb271`

Message:

`T-044-01-01: align help test plumbing terminology`

Only included:

`crates/lisa-cli/tests/help_surface.rs`

The installed Homebrew Lisa was version 0.4.0-rc.5 and did not yet expose
`commit-ticket`. The transactions therefore used the repository-built
`target/debug/lisa` at 0.4.0-rc.8. This is the current ticket source's Lisa CLI
implementation and preserved the required isolated-index behavior. No ordinary
`git add`, `git commit`, or broad staging command was used.

## Verification performed

### Baseline

`cargo test -p lisa-cli --test help_surface`

Before implementation:

- 3 passed;
- 0 failed.

This established a green baseline and showed the old suite accepted the
single-list layout.

### Expected-red evidence

After production metadata and before test updates, the focused suite produced:

- 2 passed;
- 1 failed.

The failure was the expected old assumption that the first nonempty top-level
line was the about line. The weak relative-order grouping test still passed,
confirming it did not enforce the new category boundary.

### Focused final test

`cargo test -p lisa-cli --test help_surface`

Final result:

- 4 passed;
- 0 failed.

Covered tests:

- all twelve Lisa-owned commands resolve directly;
- full top-level help matches the snapshot;
- plumbing is separate and internal commands are hidden;
- about/operator help remains free of banned jargon.

### Crate acceptance suite

`cargo test -p lisa-cli`

The final command passed all executed tests:

- 14 `lisa_cli` library unit tests;
- 269 binary unit tests;
- 1 atomic provider contract integration test;
- 2 capture-usage CLI integration tests;
- 4 help-surface integration tests;
- 1 preownership status integration test;
- 0 doc-test failures.

The real-Zellij delivery-boundary test remained ignored by its existing
environment gate. It requires real Zellij, zsh, script, jq, and the WASI target
and does not exercise top-level help metadata.

### Formatting and diff hygiene

`cargo fmt --all -- --check` passed after the final source change.

`git diff --check` across the ticket source range passed.

Read-only status checks confirmed:

- no unstaged diff for `crates/lisa-cli/src/main.rs`;
- no unstaged diff for `crates/lisa-cli/tests/help_surface.rs`;
- no ordinary-index staged diff for either file;
- neither file is untracked.

The shared worktree contains unrelated concurrent Lisa/plugin, epic, story,
ticket, and work-artifact entries. They were not included in any ticket commit
and were left untouched.

## Coverage assessment

Coverage is proportionate and direct for this help-only change.

- The test is black-box, so it validates derived Clap behavior rather than an
  internal helper approximation.
- The exact snapshot covers wording, section order, and whitespace.
- The structural test gives a precise invariant and failure message for command
  category regressions.
- The resolution test covers the primary risk of using `hide = true`: accidental
  command removal.
- Existing jargon checks continue to cover the about line and all operator
  command help.
- The crate-wide suite covers the absence of runtime regressions across command
  implementations.

No additional unit test inside `main.rs` is warranted because the behavior is
entirely produced by Clap and already exercised through the compiled binary.

## Open concerns and limitations

### Static footer maintenance

The plumbing footer repeats the four command names and concise descriptions
instead of deriving a second filtered subcommand section from Clap. Clap's
standard renderer exposes one generated subcommand list, so the static footer
is the narrowest way to provide a true second group without changing command
names or adopting a full custom template.

Maintenance risk is bounded by:

- the full-output snapshot;
- the explicit `PLUMBING_COMMANDS` set;
- direct command-resolution coverage;
- the parent story's stated fixed-command-set boundary.

This is a non-blocking, accepted tradeoff.

### Dependency-rendering sensitivity

The snapshot intentionally pins Clap's non-interactive rendering under the
workspace lockfile. A future Clap upgrade that changes alignment or standard
copy will require deliberate snapshot review. That sensitivity is desirable
for this at-a-glance surface and is not a current defect.

### Dependent ticket boundary

This ticket does not add per-command examples or rewrite every operator
description. Those changes remain assigned to dependent ticket `T-044-01-02`.
The new snapshot harness is ready for that ticket to extend deliberately.

## Critical issues

None.

## TODOs introduced

None.

## Final assessment

The implementation is small, scoped to help metadata and its black-box test,
preserves all runtime contracts, meets every stated acceptance condition, and
has clean ticket-owned source state. The work is ready for Lisa's completion
publication.
