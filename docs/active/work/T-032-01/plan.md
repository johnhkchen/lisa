# Plan: T-032-01 Zellij pane lifecycle names

## Step 1: Add the deterministic formatter

Create `crates/lisa-plugin/src/pane_name.rs` with the assigned/idle `PaneName` enum,
`MAX_PANE_NAME_CHARS`, sanitization helper, and `format_pane_name`.

Implementation details:

- Use `AgentClient::Display` for canonical provider spelling.
- Use the exact middle-dot separator from the ticket.
- Treat `char::is_control()` as unsafe.
- Treat remaining `char::is_whitespace()` values as separators.
- Collapse separators rather than preserving tab/newline widths.
- Substitute `untitled` when nothing visible remains.
- Count and truncate using `.chars()` only.
- Add a single Unicode ellipsis when truncating.
- Preserve the complete assigned prefix.

Verification:

- Module compiles in the plugin crate.
- Focused tests prove exact assigned and idle formats.
- Focused tests prove controls cannot survive.
- Focused tests prove normal output is at most 80 scalar values.
- Focused tests prove full provider and ticket ID survive truncation.

The formatter and integration will be one commit because an unused crate-private module
would otherwise fail warning-denying Clippy.

## Step 2: Add cached rename state

Modify `State` in `crates/lisa-plugin/src/lib.rs`:

- Add `mod pane_name` and imports.
- Add `last_pane_names: HashMap<u32, String>` with explanatory comments.
- Add `rename_slot(pane_id, name) -> bool`.
- Reject unknown pane IDs.
- Compare the cached value before emitting a host command.
- Cache before calling `rename_terminal_pane`.

Verification:

- A native unit test constructs a slot and calls the helper twice.
- First call returns true and stores the title.
- Second identical call returns false and leaves one stable cache value.
- A different title returns true and replaces the cache.
- An unknown pane returns false and does not populate the cache.

## Step 3: Name newly discovered empty shells

Update `discover_slots` so each discovered terminal pane is registered before the rename gate
is invoked. Use `PaneName::Idle { resident_agent: None }`.

Verification:

- Existing slot-discovery tests still pass.
- Add or extend a discovery test to assert `lisa · idle` is cached for terminal slots.
- Plugin panes remain excluded and receive no cache entry.
- Repeated discovery remains no-op after `slots_discovered` is set.

## Step 4: Name assignment before any lifecycle input

Update `schedule_ready_tickets`:

- Resolve the route as today.
- Clone the parsed ticket title before mutable state operations.
- After slot and awaiting-human admission, format the assigned title with `route.agent`.
- Call `rename_slot` before the branch that sends `/exit`, `/clear`, or a fresh launch.
- Do not rename in `.cleared` or timeout prompt paths because they remain the same assignment.

Verification:

- Fresh scheduling test asserts the cache contains the assigned title.
- Same-provider resident reuse test asserts the new ticket title replaces idle/old title
  while the slot enters `WaitingForClear`.
- Cross-provider recycle test asserts the incoming actual provider and ticket appear while
  the slot enters `WaitingForExit`.
- A ticket requesting an invalid provider under a known default asserts the pane uses the
  actual fallback provider, not the raw request.
- Existing pending-enter and transition assertions remain unchanged.

## Step 5: Name slots only when truly released

Update `release_slot_for_ticket`:

- Preserve existing slot clearing and cooldown behavior.
- Derive idle name after the slot mutation.
- Use resident provider only when `has_session` is true.
- Apply the idle name through `rename_slot` after ending the slot borrow.
- Do nothing to naming when the ticket has no matching slot.

Verification:

- Direct release with a Claude resident session produces `claude · idle`.
- Direct release with a Codex resident session produces `codex · idle`.
- Release of an empty/no-session slot produces `lisa · idle`.
- Missing-ticket release leaves all cached names unchanged.

## Step 6: Cover commit-gated completion

Extend existing completion tests rather than creating a parallel completion harness.

Successful result:

- Seed the cache with the assigned Codex name.
- Request completion and publish a valid durable Done result as the fixture already does.
- Assert slot release and `codex · idle`.

Failed result:

- Seed the cache with the assigned Codex name.
- Return a nonzero completion result.
- Assert thread and slot remain assigned.
- Assert cached name remains assigned and does not contain `idle`.
- Retry success and assert idle only after verified publication.

## Step 7: Cover clean-shell recovery

Extend the missing-ticket `WaitingForExit` recovery test:

- Seed the cache with an assigned or resident-provider name.
- Advance beyond the exit grace.
- Assert `has_session = false` and `last_client = None`.
- Assert cache is `lisa · idle`.

Do not rename in stop timeout, clear timeout, awaiting-human, or other non-release tests. Add
assigned-name assertions where useful to prove those branches retain it.

## Step 8: Format and run focused tests

Run formatter, lifecycle, completion, and recycle test filters after `cargo fmt --all`.
Inspect the diff immediately afterward for unrelated formatting.

Document every command and result in `progress.md`.

## Step 9: Run required full verification

Run:

- `cargo test --workspace`
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`

Failures are triaged as ticket regressions, pre-existing unrelated failures, or missing
environment prerequisites. Exact evidence is recorded; no unavailable check is claimed as
successful.

## Step 10: Attempt live mixed-provider validation

Inspect non-mutating prerequisites:

- `zellij --version`
- `zellij list-sessions`
- `command -v claude`
- `command -v codex`
- whether an existing Lisa mixed-provider development loop is safely available.

Do not hijack an unrelated live user session or create billable/authenticated agent work
without a suitable fixture and operator context. If a safe loop is available, observe or
exercise assignment -> completion -> idle -> reassignment and record pane-name evidence.

If the environment lacks a safe live loop, record the blocker precisely in `progress.md` and
`review.md`. Unit/lifecycle coverage is not misrepresented as live validation.

## Step 11: Inspect ownership and commit source

Before committing, inspect the exact diff, worktree status, and ordinary index. Confirm:

- Only `crates/lisa-plugin/src/lib.rs` and `crates/lisa-plugin/src/pane_name.rs` are included.
- No ticket-owned source file is staged in the ordinary index.
- Unrelated dirty files retain their prior status.

Commit through Lisa's isolated transaction with:

- ticket ID `T-032-01`
- message `Name Zellij panes across scheduler lifecycle`
- exact include `crates/lisa-plugin/src/lib.rs`
- exact include `crates/lisa-plugin/src/pane_name.rs`

Do not include ticket or work artifacts; Lisa owns their final completion commit. Afterward,
verify both source paths are clean and no ticket-owned source entry is ordinarily staged.

## Step 12: Complete progress and review artifacts

Write/update `progress.md` throughout implementation with completed steps, test outcomes,
deviations, commit receipt, live-validation evidence or blocker, and remaining work.

Then write `review.md` with source changes, acceptance mapping, test/build/lint coverage,
live evidence status, and open concerns. Confirm ticket phase/status was not manually edited.

After `review.md`, stop without changing the ticket or starting other work.
