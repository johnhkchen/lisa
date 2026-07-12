# Plan: gate ownership on acknowledgment

## Objective

Implement the complete scheduler acknowledgment gate for recycled Codex seats. Preserve fresh
Codex and all Claude behavior, and prove stale or duplicate acknowledgment cannot claim a seat.

## Step 1: record implementation baseline

- confirm ticket remains in `research` frontmatter;
- record unrelated modified and untracked paths;
- confirm prerequisite commits are present;
- run focused existing pending-state and detector tests;
- document any preexisting failures in `progress.md`.

Verification:

- `git status --short` is captured;
- `cargo test -p lisa-plugin codex_ack` passes;
- `cargo test -p lisa-plugin recycled_codex` passes.

## Step 2: make pending state self-identifying

- add `generation` to `AssignedPendingAck`;
- add the scheduler's next-generation counter;
- add a private allocation helper;
- add a pending-generation query helper;
- update all production and test matches;
- preserve pending generation through clear and exit timeout paths.

Verification:

- compiler finds every old fieldless construction;
- pending tests assert a positive generation;
- ownership query remains false for the data-bearing pending variant.

## Step 3: carry identity through prompt delivery

- extend `SpawnContext` with optional assignment generation;
- allocate generation before the scheduler constructs delivery context;
- populate the context for immediate, clear-deferred, exit-deferred, and timeout delivery;
- add Codex adapter prompt helper;
- tag Codex launch and reuse prompts only when generation is present;
- leave Claude output byte-for-byte unchanged.

Verification:

- Codex reuse prompt includes detector-readable marker;
- Codex launch line includes marker text;
- no-generation Codex path is unchanged;
- Claude adapter equality tests still pass.

## Step 4: expose the raw lifecycle payload

- add `ON_ACK_HOOK` source template;
- atomically write stdin to `pane-<id>.ack`;
- add `UserPromptSubmit` to generated `.codex/hooks.json`;
- add the same hook through merge logic;
- add `on-ack.sh` to init creation/update inventory;
- add it to validation's executable script inventory;
- update template and init tests.

Verification:

- generated JSON parses;
- generated JSON contains exactly one Lisa ack command;
- merging preserves a preexisting user `UserPromptSubmit` command;
- repeated merge remains identical;
- init creates the ack script and Codex hook;
- validate accepts a complete generated installation.

## Step 5: implement exact scheduler promotion

- import the detector and assignment reference into `lib.rs`;
- implement `acknowledge_codex_assignment`;
- require current pane reservation and pending generation;
- invoke the detector with current identity;
- insert `Owned` only on a match;
- return true only for the state transition;
- remove prerequisite dead-code allowances in `codex_ack.rs`.

Verification:

- absent assignment returns false;
- owned assignment returns false;
- stale ticket returns false;
- stale generation returns false;
- exact match returns true;
- duplicate exact match returns false.

## Step 6: connect ack files to the scheduler

- add `check_codex_ack_signals`;
- parse only `pane-<u32>.ack` files;
- read payload before deleting;
- consume validly named files regardless of classification result;
- invoke exact promotion method;
- bump activity and log only on successful promotion;
- call the scanner from `poll_tick` before timeout evaluation.

Verification:

- matching file is removed and promotes ownership;
- stale file is removed without promotion;
- malformed payload is removed without promotion;
- duplicate file after ownership does not log/promote again;
- unrelated signal filenames are unaffected.

## Step 7: satisfy the acceptance criterion with one scheduler scenario

Build a test that schedules a recycled Codex assignment and then injects lifecycle payloads.

Assertions in order:

1. scheduled seat is `AssignedPendingAck { generation }`;
2. scheduled seat reports not-owned before any ack;
3. a previous-ticket ack returns false;
4. state remains pending and not-owned;
5. a same-ticket previous-generation ack returns false;
6. state remains pending and not-owned;
7. exact ticket/generation ack returns true;
8. state becomes `Owned` and reports owned;
9. duplicate exact ack returns false;
10. state stays `Owned`.

This is the primary scheduler test named by the ticket.

## Step 8: format and focused verification

- run `cargo fmt --all`;
- run focused acknowledgment detector and scheduler tests;
- run adapter tests;
- run template hook tests;
- run relevant init/validate tests.

Verification commands:

```bash
cargo test -p lisa-plugin codex_ack
cargo test -p lisa-plugin acknowledgment
cargo test -p lisa-plugin adapter
cargo test -p lisa-cli templates
cargo test -p lisa-cli init
```

If Rust test name filtering differs, run the closest package test set and record it.

## Step 9: package verification

- run the entire plugin package suite;
- run the entire CLI package suite;
- confirm no test relies on ordinary index state;
- investigate every new failure before expanding scope.

Commands:

```bash
cargo test -p lisa-plugin
cargo test -p lisa-cli
```

Acceptance:

- all plugin scheduler and adapter tests pass;
- all CLI template/init tests pass;
- no unrelated worktree file is changed by test execution.

## Step 10: workspace and target verification

- run workspace tests;
- run formatting check;
- run Clippy with warnings denied;
- run the project's quick check or explicit WASM target check.

Commands:

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
just check
```

If `just check` duplicates completed work, it is still the repository-prescribed WASM and test
gate. Record any environment-only issue precisely rather than weakening the result.

## Step 11: inspect exact diff

- run `git diff --check` on the five source paths;
- inspect each source diff;
- confirm ticket frontmatter was not edited;
- confirm generated project-instance hooks were not claimed;
- confirm no ticket-owned source path is staged.

Owned paths:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/adapter.rs
crates/lisa-plugin/src/codex_ack.rs
crates/lisa-cli/src/templates.rs
crates/lisa-cli/src/init.rs
```

Review questions:

- Can any non-detector event promote ownership?
- Does a delayed generation remain rejected?
- Does a duplicate matching payload remain inert?
- Does deferred prompt delivery carry the stored generation?
- Are fresh Codex and Claude paths unchanged?
- Does init safely upgrade existing Codex hook configuration?

## Step 12: update progress before commit

Write `progress.md` with:

- completed implementation units;
- exact behavioral decisions;
- tests run and results;
- deviations from this plan;
- remaining work;
- unrelated worktree preservation notes.

The progress artifact is not included in the source commit. Lisa owns final artifact publication.

## Step 13: isolated source commit

Use the repository CLI if the globally installed `lisa` lacks the subcommand. Commit exactly:

```bash
lisa commit-ticket \
  --ticket-id T-033-01-03 \
  --message "feat: gate Codex seat ownership on acknowledgment" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/codex_ack.rs \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/init.rs
```

Never use ordinary `git add` or `git commit`.

Acceptance:

- isolated command succeeds;
- HEAD contains only the intended source paths for this ticket;
- ordinary staged entries remain untouched;
- ticket-owned source files are clean afterward.

## Step 14: post-commit audit

- inspect `git show --stat --oneline HEAD`;
- inspect `git status --short` for owned paths;
- verify no owned source file is modified, staged, or untracked;
- rerun a focused scheduler acknowledgment test if commit tooling changed the worktree;
- record commit ID in `progress.md` and `review.md`.

## Step 15: Review artifact

Write `review.md` summarizing:

- source files modified;
- end-to-end acknowledgment flow;
- exact acceptance-criterion evidence;
- test and lint coverage;
- commit boundary and worktree cleanliness;
- open concerns and intentionally deferred recovery behavior;
- any critical human-review issue.

Do not update ticket phase or status. Stop after `review.md` is complete.

## Expected deviations

- Compilation may reveal additional `SpawnContext` constructors; update them mechanically and
  record the count.
- Existing broad tests may assert an exact generated hook inventory; update only expectations
  that intentionally change with `on-ack.sh`.
- If shell-hook source is stored in a different inventory than currently observed, follow the
  established init ownership pattern rather than introducing a parallel writer.

## Completion criteria

- recycled Codex is not owned without ack;
- only matching current ticket and generation promote it;
- promotion occurs exactly once;
- stale and duplicate ack cannot claim it;
- tagged prompt and native payload transport are live;
- Claude and fresh Codex behavior remain unchanged;
- focused, package, workspace, lint, and WASM checks are recorded;
- all ticket-owned source changes are isolated-committed;
- `review.md` exists;
- ticket frontmatter remains untouched.
