# Plan: Codex acknowledgment detector

## Objective

Implement and verify a deterministic Codex lifecycle classifier that acknowledges only
a `UserPromptSubmit` event carrying the exact pending Lisa ticket and assignment
generation. Preserve scheduler, Claude, hook transport, and UI behavior.

## Step 1: establish source ownership baseline

Actions:

- inspect `git status --short` before editing source;
- record unrelated modified and untracked paths;
- verify planned source files do not overlap known user changes;
- avoid `.lisa/hooks`, CLI agent-exec, story, and ticket files.

Verification:

- `crates/lisa-plugin/src/lib.rs` is clean before this ticket's edit;
- new detector and fixture paths do not already exist;
- ticket phase/status frontmatter remains unchanged.

Atomic unit: no commit; this is a safety check.

## Step 2: declare the provider-specific module

Actions:

- add `mod codex_ack;` in `crates/lisa-plugin/src/lib.rs`;
- place it with the existing private plugin modules;
- make no scheduler imports or calls.

Verification:

- root module topology remains simple;
- diff contains only the declaration in `lib.rs`;
- no state transition changes are introduced.

Atomic unit: grouped with detector implementation.

## Step 3: define assignment identity

Actions:

- create `crates/lisa-plugin/src/codex_ack.rs`;
- define `CodexAssignmentRef<'a>`;
- include borrowed `ticket_id` and numeric `generation`;
- derive comparison and debugging traits useful to tests and later wiring;
- document that generation allocation belongs to scheduler integration.

Verification:

- the type contains no pane, terminal, or Claude fields;
- it is cheap to copy;
- it cannot itself mutate scheduler state.

Atomic unit: grouped with detector implementation.

## Step 4: implement canonical prompt tagging

Actions:

- define the `LISA_ASSIGNMENT ` marker prefix;
- define a private serializable marker schema;
- implement `tag_codex_assignment`;
- serialize marker JSON through `serde_json`;
- preserve arbitrary ticket IDs through JSON escaping;
- append the marker on its own line;
- avoid duplicate trailing blank lines where practical.

Verification:

- a tagged prompt retains its original body;
- the final line uses the canonical prefix;
- quotes and backslashes in a test ticket ID round-trip safely;
- no shell quoting is performed in this helper.

Atomic unit: grouped with detector implementation.

## Step 5: implement fail-closed classification

Actions:

- define the minimal deserializable lifecycle envelope;
- parse `hook_event_name` and optional `prompt`;
- reject malformed JSON;
- reject every event except exact `UserPromptSubmit`;
- scan prompt lines for the canonical prefix;
- parse marker JSON into the private schema;
- return true only on exact ticket and generation equality;
- return false for all missing fields, malformed markers, and mismatches.

Verification:

- there is no filesystem access;
- there is no transcript parsing;
- there is no terminal parsing;
- there are no Claude event or signal names;
- unknown lifecycle fields are accepted and ignored;
- a malformed input cannot panic.

Atomic unit: grouped with fixture tests because the API is intentionally not wired yet.

## Step 6: add captured-shape lifecycle fixtures

Actions:

- create `crates/lisa-plugin/tests/fixtures/codex_ack/`;
- add a matching `UserPromptSubmit` JSON payload;
- add a still-idle `SessionStart(clear)` payload;
- add a stale previous-ticket `UserPromptSubmit` payload;
- add a stale previous-generation `UserPromptSubmit` payload;
- retain documented session, turn, model, cwd, transcript, and permission fields.

Verification:

- each fixture parses as JSON;
- turn-scoped fixtures include `turn_id`;
- clear fixture has `source: clear` and no prompt;
- positive marker is ticket `T-033-01-02`, generation `42`;
- stale fixtures differ in exactly the attribution dimensions they test.

Atomic unit: grouped with detector implementation.

## Step 7: add fixture truth-table tests

Actions:

- load each fixture using `include_str!`;
- define one pending assignment for ticket `T-033-01-02`, generation `42`;
- assert matching fixture returns true;
- assert clear fixture returns false;
- assert previous-ticket fixture returns false;
- assert previous-generation fixture returns false.

Verification:

- the acceptance criterion maps directly to named assertions;
- tests call the production classifier, not a fixture-only helper;
- fixtures are compile-time included and working-directory independent.

Atomic unit: grouped with detector implementation.

## Step 8: add defensive unit cases

Actions:

- test malformed payload JSON;
- test a non-`UserPromptSubmit` event containing a matching prompt;
- test marker-looking text embedded mid-line;
- test marker text in an unrelated payload field;
- test tagging and detection round-trip with JSON-sensitive ticket text;
- test unknown extra lifecycle fields do not affect detection.

Verification:

- all invalid evidence returns false;
- exact marker parsing has no substring shortcut;
- marker serialization never relies on manual escaping.

Atomic unit: grouped with detector implementation.

## Step 9: focused verification

Commands:

```text
cargo fmt --all
cargo test -p lisa-plugin codex_ack
```

Verification:

- formatter completes without unrelated source rewrites;
- every detector unit test passes;
- compiler emits no warnings attributable to the new module.

If formatting touches unrelated dirty files, inspect and avoid claiming those changes.

## Step 10: package verification

Commands:

```text
cargo test -p lisa-plugin --lib
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Verification:

- all plugin unit tests pass;
- strict Clippy passes;
- existing scheduler transition tests remain unchanged and green;
- no dead-code or visibility workaround is broader than necessary.

## Step 11: workspace verification

Command:

```text
cargo test --workspace
```

Verification:

- CLI, core, plugin, integration, and doc tests pass;
- fixture inclusion works from normal Cargo execution;
- no cross-crate behavior regresses.

If a failure is unrelated and pre-existing, reproduce it and document evidence rather
than changing unrelated files.

## Step 12: inspect implementation diff

Actions:

- run `git diff --check` on ticket-owned source paths;
- inspect `git diff` for `lib.rs` and `codex_ack.rs`;
- inspect fixture JSON with a parser;
- search detector source for forbidden assumptions;
- confirm no `Claude`, `cleared`, `terminal`, or transcript-reading logic exists.

Verification:

- source diff matches Structure;
- fixture truth table matches acceptance;
- no scheduler promotion is accidentally implemented;
- no hook generation is accidentally modified.

## Step 13: write implementation progress

Actions:

- create `progress.md` before the source commit;
- record completed steps and tests;
- record any deviations and rationale;
- identify dependent-ticket integration work explicitly;
- list exact source paths planned for commit.

Verification:

- progress does not claim scheduler wiring;
- known boundaries and open work are explicit;
- frontmatter remains untouched.

## Step 14: commit the meaningful source unit

Preferred command shape:

```text
cargo run -p lisa-cli -- commit-ticket \
  --ticket-id T-033-01-02 \
  --message "feat: detect ticket-scoped Codex acknowledgments" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/codex_ack.rs \
  --include crates/lisa-plugin/tests/fixtures/codex_ack/matching-prompt-submit.json \
  --include crates/lisa-plugin/tests/fixtures/codex_ack/still-idle-clear.json \
  --include crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-ticket.json \
  --include crates/lisa-plugin/tests/fixtures/codex_ack/stale-previous-generation.json
```

Use the installed `lisa` binary only if it supports the required command. Never use
ordinary `git add`, `git commit`, or the ordinary index.

Verification:

- transaction succeeds;
- resulting commit contains only exact source and fixture paths;
- commit message identifies the detector unit;
- unrelated working-tree changes remain present and uncommitted.

## Step 15: post-commit reconciliation

Actions:

- inspect `git show --stat --oneline HEAD`;
- inspect `git status --short` for ticket-owned paths;
- inspect ordinary staged paths with `git diff --cached --name-only`;
- rerun the focused acceptance test after commit if reconciliation changes HEAD.

Verification:

- every ticket-owned source path is clean;
- no ticket-owned source path is staged;
- no fixture remains untracked;
- work artifacts remain available for Lisa's final transaction;
- unrelated dirty paths are unchanged.

## Step 16: Review artifact

Create `review.md` summarizing:

- the provider-native evidence chosen;
- files created and modified;
- fixture truth table and defensive coverage;
- focused, package, Clippy, and workspace results;
- isolated commit identity;
- lack of scheduler and hook transport wiring by design;
- open operational concerns around hook delivery and generation allocation;
- critical issues, if any.

After `review.md` is written, stop. Do not change ticket status or phase, publish Done,
release the seat, or begin another ticket.

## Acceptance mapping

### `ack=true` only for pending ticket and generation

Covered by the matching fixture plus exact ticket and generation comparison.

### still-idle returns false

Covered by the `SessionStart(clear)` fixture.

### stale previous-ticket returns false

Covered by the previous-ticket fixture in the same representative session.

### stale assignment generation returns false

Covered by the same-ticket generation-41 fixture against pending generation 42.

### no Claude-handshake assumptions

Covered structurally by the isolated `codex_ack` module and source inspection.

### no terminal-render assumptions

Covered structurally by accepting only lifecycle JSON and never accessing pane contents.
