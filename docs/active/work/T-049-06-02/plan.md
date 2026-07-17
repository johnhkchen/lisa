# Plan — T-049-06-02 Notes for you queue

## 1. Provenance acknowledgment

Add the disjoint note acknowledgment row, mixed-ledger variant, and append helper.
Test legacy mixed rows, ack round-trip, and newline-terminated append.

## 2. Shared queue reducer

Create `lisa_core::notes` with exact generation key and queued entry. Parse strict
completion JSONL, select confirmed notes, subtract exact provenance acknowledgments,
and sort deterministically. Implement ack by resolving the current ticket entry before
append.

Tests cover requested/confirmed, pass/note, exact ack, later generation resurfacing,
duplicate and unknown ack, missing files, malformed JSON, and torn history.

Run:

```bash
cargo test -p lisa-core notes::
cargo test -p lisa-core provenance::
```

## 3. Commit core unit

Inspect formatting and exact diff, then use `lisa commit-ticket` with only core lib,
notes, and provenance paths. Record the commit ID in progress.md.

## 4. Add `lisa notes`

Create CLI list and ack functions. Format heading/count, summary lead, criterion, and
evidence. Empty list emits nothing. Add explicit `notes ack <ticket-id>` wiring and
plain help copy in main.

## 5. Integrate status

Load the shared queue in status. Print Waiting on you, then Notes for you, then DAG.
Reuse the formatter and suppress the heading for empty queues.

## 6. CLI lifecycle fixtures

Create a real-binary fixture in a project path containing spaces. Verify list output,
status section distinction/order, acknowledgment row, durable clearing across new
processes, duplicate rejection, and unchanged ticket bytes/DAG readiness.

Update help snapshots, operator command arrays, all-command arrays, and counts.

Run:

```bash
cargo test -p lisa-cli --test notes_ux
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli status::
```

## 7. Commit CLI unit

Format, inspect exact paths, commit only CLI source and test paths through Lisa, and
record the returned commit ID.

## 8. Dashboard renderer

Add NoteItem and PluginState.note_items. Render a distinct Notes for you section after
Waiting on you and before attention/threads. Add pure tests for empty, summary-first,
details, and section ordering.

## 9. Plugin durable projection

Call the core reducer from `State::to_ui_state`, map entries, and avoid all scheduler
state changes. Add a fixture that completes a note, observes it, reconstructs State,
observes it after restart, acknowledges it, and sees it clear.

In the same fixture assert dependency flow, no parks, unchanged ticket bytes, and no
seat/thread/completion change from acknowledgment.

Run focused UI and lifecycle tests.

## 10. Commit plugin unit

Format and commit exactly plugin lib and UI through `lisa commit-ticket`. Record the
commit ID in progress.md.

## 11. Full verification

Run:

```bash
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-cli
cargo test -p lisa-plugin --lib
cargo test --workspace
just check
git diff --check
```

Document any plan deviation before corrective work and commit corrective ticket paths
through another exact Lisa transaction.

## 12. Ownership audit

Inspect read-only Git status and commit file lists. Preserve pre-existing Lisa-managed
and other-ticket changes. Ensure every ticket source path is clean and no ticket-owned
file is staged, modified, or untracked. Never use ordinary `git add` or `git commit`.

## 13. Review

Write progress outcome, then review.md with files, commits, durability model, rendering,
lifecycle, restart and zero-effect coverage, checks, and concerns. Write the exact pass
JSON only if all checks pass and ticket source is clean; otherwise write an actionable
block disposition. Remain on this ticket after both artifacts exist.
