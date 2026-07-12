# Review: T-032-01 Zellij pane lifecycle names

## Outcome

Implemented deterministic scheduler-owned Zellij terminal-pane names across discovery,
assignment, provider reuse, provider switching, release, and empty-shell recovery.

The implementation is committed as:

- `cd9257d5a9499068d6ba61c16f24e502bc7e70a7`
- `Name Zellij panes across scheduler lifecycle`

The commit contains exactly the two ticket-owned source paths. No ordinary-index staging or
ordinary Git commit was used.

## Files created

### `crates/lisa-plugin/src/pane_name.rs`

Adds the single formatter used for every provider and lifecycle state.

Assigned form:

`<actual-agent> · <ticket-id> · <sanitized-title>`

Idle forms:

- `<resident-agent> · idle` for a reusable resident session.
- `lisa · idle` for a slot with no resident session.

The module defines the normal maximum as 80 Unicode scalar values. It sanitizes the human
title by replacing control/whitespace runs with a single ASCII space, trimming the result,
and substituting `untitled` when no visible content remains. Overlong titles are shortened
with a Unicode ellipsis. The full actual agent and ticket ID prefix is preserved.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Registers the formatter module and adds `State.last_pane_names`, keyed by physical terminal
pane ID. `State::rename_slot` is now the only `rename_terminal_pane` call site. It compares
the desired name with the cached last-applied name and skips identical operations, preventing
poll/event spam.

Newly discovered empty panes receive `lisa · idle` once `ChangeApplicationState` permission
is available. Both event orderings are covered: pane discovery before permission, and
permission before discovery.

Scheduling formats the assigned name from `ResolvedRoute.agent`, the ticket ID, and the
parsed frontmatter `Ticket.title`. It applies the name after all scheduling safety gates but
before the first lifecycle input:

- Before the fresh provider launch command.
- Before `/clear` for same-provider reuse.
- Before `/exit` for a cross-provider switch.

Consequently, a previous ticket or idle name is replaced before the next command/prompt is
submitted. `.cleared` handling and timeout recovery keep the assigned name because neither
path releases ownership.

The common `release_slot_for_ticket` path derives the idle title from post-release session
truth. A retained client produces its provider idle form; an empty slot produces Lisa idle.
The cross-provider exit recovery path also restores `lisa · idle` if its pending ticket
disappears and the pane becomes a clean shell.

## Acceptance-criteria assessment

- One deterministic formatter: met. `format_pane_name` handles all assigned and idle forms.
- Assigned actual agent, ID, and title: met. Scheduler passes `route.agent`, never raw
  `requested_agent`.
- Fresh launch, same-provider reuse, cross-provider switch: met in code and lifecycle tests.
- No stale name after prompt submission: met by rename placement before `/clear`, `/exit`, or
  launch input.
- Resident and empty-shell idle forms: met.
- Commit-gated idle naming: met. Failed completion does not call release and retains the
  assigned cache value; verified success releases and names idle.
- Awaiting-human and timeout non-release behavior: met structurally. Those paths do not call
  release or rename idle; the assignment remains.
- Sanitization and documented bound: met for canonical Lisa ticket IDs and all title input.
- Redundant rename suppression: met through the physical-pane cache.
- Unit/lifecycle coverage: met for every requested named branch.
- Focused tests, workspace tests, WASM build, and Clippy: met.
- Live mixed Claude/Codex validation: not completed; see Open Concerns.

## Test coverage

### Formatter unit tests

Six tests cover:

- Exact Claude and Codex assigned strings.
- Claude, Codex, and empty-shell idle strings.
- Newline, tab, carriage return, escape, and Unicode control sanitization.
- Whitespace collapse.
- Control-only title fallback.
- Exact-bound behavior.
- Unicode-scalar-safe truncation and ellipsis.
- Full agent/ticket scan-key preservation.

### Scheduler lifecycle tests

Six focused `pane_title` tests cover:

- Deduplicating repeated rename requests.
- Fresh launch.
- Invalid requested agent falling back to the actual Codex default.
- Same-provider reuse replacing a stale prior-ticket name.
- Cross-provider switch displaying the incoming actual provider.
- Release to resident-provider idle.
- Release of a no-session slot to Lisa idle.
- Missing pending ticket after provider exit returning to Lisa idle.

Existing completion tests were extended to assert:

- Durable verified completion releases a resident Codex session as `codex · idle`.
- Failed completion retains `codex · T-CDX-01 · codex-a` and keeps the slot assigned.
- A successful retry changes the title to idle only after verification and release.

## Verification results

- `cargo test -p lisa-plugin pane_name`: 6 passed, 0 failed.
- `cargo test -p lisa-plugin pane_title`: 6 passed, 0 failed.
- Focused successful-completion test: passed.
- Focused failed-completion/retry test: passed.
- `cargo test --workspace`: passed. Plugin reports 250 passed, 0 failed; CLI, core,
  integration, and doc-test suites also passed.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: passed.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- `git diff --check` on both source paths: passed.

## Commit and ownership verification

The installed Homebrew Lisa predates `commit-ticket`, so the repository-built CLI was used:

`cargo run -q -p lisa-cli -- commit-ticket ...`

The resulting commit includes only:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/pane_name.rs`

After the commit, both paths are clean and `git diff --cached --name-only` is empty. Existing
unrelated worktree changes were preserved and excluded. Ticket/work artifacts remain for
Lisa's own completion transaction.

## Open concerns and limitations

### Live mixed-provider evidence is outstanding

This is the one unmet acceptance item and needs human attention before treating the ticket as
fully live-validated.

The host has Zellij 0.44.3 plus both Claude and Codex binaries. However, the current
`nautical-piano` session is a shared active Lisa loop containing this ticket and T-031-03,
and `dump-layout` confirms it loaded a cached temporary WASM that predates this commit.
Replacing or reloading that plugin would disrupt active ticket execution. Starting a second
authenticated multi-agent loop from inside the current Zellij loop was not treated as a safe
isolated validation.

A reviewer should launch a fresh loop from a Lisa CLI embedding commit `cd9257d` or later and
observe this sequence in Zellij:

1. Fresh Claude and Codex assignments show actual provider, ticket ID, and title.
2. Successful completion changes each retained session to `<provider> · idle`.
3. Same-provider reassignment replaces idle with the new ticket before prompt submission.
4. Cross-provider recycling shows the incoming provider during `/exit` and fresh launch.
5. A forced completion-commit failure retains the assigned name.

### Pathological ticket IDs

The 80-scalar bound applies to canonical Lisa IDs. The formatter deliberately preserves a
complete agent and ticket ID even if malformed external input makes that immutable prefix
itself exceed 80 characters, omitting the title in that case. This resolves the ticket's
priority that stable scan keys remain complete, but means pathological noncanonical IDs can
exceed the cosmetic bound. The repository currently has no ticket-ID length validator.

## Critical issues

No code, test, build, lint, ownership, or commit defects were found. The missing live
mixed-provider observation is the only critical review item.

## Workflow integrity

- All six RDSPI phases were completed continuously.
- `research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`, and `review.md` exist.
- The ticket's `phase` and `status` frontmatter fields were not manually edited.
- No next ticket was started.
