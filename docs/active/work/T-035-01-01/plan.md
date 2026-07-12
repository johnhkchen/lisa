# T-035-01-01 Plan — process-start signal producer

## 1. Add the shared start-hook template

- Define `ON_START_HOOK` in `templates.rs`.
- Require pane, ticket, and attempt environment values.
- Restrict attempt identity to decimal digits.
- Compare the exact expected lease JSON to the scheduler marker.
- Copy through a pane/process-specific temporary file.
- Atomically rename to `.started` only on success.
- Define the legacy-template inventory.

Verification:

- hook begins with POSIX `#!/bin/sh`;
- hook references all three identity values, `.lease`, `.started`, and atomic rename;
- no provider name occurs in its runtime contract.

## 2. Add executable producer fixtures

- Materialize the template in a temporary project root.
- Run with a matching lease and assert exact `.started` bytes.
- Run with a stale attempt marker and assert no signal.
- Run with a wrong-ticket marker and assert no signal.
- Run with missing marker/identity and assert no signal.
- Assert a fixture in which the hook is never invoked produces no signal.

Verification:

- focused template test passes on the native test host;
- rejected cases do not leave temporary start files.

## 3. Bind startup for Claude and Codex

- Add `SessionStart[startup]` to `settings_local_json()`.
- Add the same binding to `codex_hooks_json()`.
- Preserve the separate `SessionStart[clear]` entry.
- Add startup merge calls to both merge functions.
- Extend generated JSON and idempotency assertions.

Verification:

- both JSON documents parse;
- both contain exactly one Lisa startup command;
- both retain exactly one clear command;
- merging twice is stable and preserves user hooks.

## 4. Materialize and validate the managed script

- Add `on-start.sh` to init's owned hook inventory.
- Add it to required hook-file validation arrays.
- Add startup expectations to native configuration validation where applicable.
- Adjust tests with exact file inventories.

Verification:

- fresh init creates executable `on-start.sh`;
- init-then-validate succeeds for Claude and Codex configuration;
- existing divergent-file safety behavior remains green.

Commit unit:

```text
lisa commit-ticket --ticket-id T-035-01-01 \
  --message "feat: scaffold native process-start signal" \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/init.rs
```

## 5. Thread attempt identity into native launches

- Add `attempt_id` to `SpawnContext`.
- Populate it from the exact lease minted during dispatch.
- Add an explicit attempt parameter to `build_claude_command`.
- Export `LISA_ATTEMPT_ID` in Claude's command.
- Export the same value in Codex's command.
- Update every constructor and exact launch-string assertion.

Verification:

- adapter tests prove both native provider commands export pane/ticket/attempt;
- dispatch tests compile with attempt identity from the lease;
- recycled and recovery paths retain their current ack/lease behavior.

Commit unit:

```text
lisa commit-ticket --ticket-id T-035-01-01 \
  --message "feat: bind native starts to attempt identity" \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/lib.rs
```

## 6. Focused verification

Run:

```text
cargo test -p lisa-cli templates
cargo test -p lisa-cli init
cargo test -p lisa-plugin adapter
cargo test -p lisa-plugin --lib
```

Criteria:

- matching fixture publishes the lease;
- stale/mismatched fixtures publish nothing;
- no-start fixture publishes nothing;
- native hook JSON has provider parity;
- existing Codex ack and lease-fencing regressions pass.

## 7. Workspace verification

Run:

```text
cargo fmt --all -- --check
cargo test --workspace
```

If formatting is required, run `cargo fmt --all`, inspect its path set, and commit only
ticket-owned changes through the exact ticket transaction. Do not include unrelated files.

## 8. Implementation tracking and review

- Record completed steps, tests, and deviations in private `progress.md`.
- Confirm no ticket-owned source remains modified, staged, or untracked.
- Write private `review.md` with changed files, coverage, and open concerns.
- Leave ticket phase/status untouched and remain on this ticket.

## Expected non-goals

- No `.started` consumer.
- No ownership-state transition.
- No startup timeout or retry.
- No `agent-exec` changes.
- No first-launch transport rewrite.
- No direct writes to `docs/active/work/T-035-01-01/`.
