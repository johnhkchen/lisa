# T-033-01-04 Plan — bounded acknowledgment recovery

## Goal

Implement a finite, configurable recycled-Codex acknowledgment wait. On the
first timeout, abandon the old generation and launch at most one fresh tagged
Codex session for the same ticket. On exact fallback acknowledgment, establish
ownership; on fallback timeout or process error, retain the ticket in an
explicit actionable failure state without automatic retry.

Each step below has an independent verification point. Source changes will be
committed only after the complete behavior and configuration transport pass.

## Step 1 — establish the configuration contract in core

Modify `crates/lisa-core/src/types.rs`.

1. Add `DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS = 30`.
2. Add `assignment_ack_timeout_secs` to `PluginConfig`.
3. Initialize it in `PluginConfig::new`.
4. Parse a positive `assignment_ack_timeout_secs` from the Zellij map.
5. Retain the finite default for missing, malformed, or zero direct values.
6. Add focused tests for default, override, and fail-safe parsing.

Verification:

```bash
cargo test -p lisa-core assignment_ack_timeout
```

Atomic criterion: the WASM-facing configuration type always has a positive
deadline even if constructed from an invalid raw map.

## Step 2 — carry the setting through CLI configuration

Modify `crates/lisa-cli/src/config.rs`.

1. Add the optional TOML field to `SchedulingConfig`.
2. Add the resolved field to `ResolvedConfig`.
3. Apply the core default and TOML override in `resolve_config`.
4. Register the key in the known scheduling keys.
5. Reject zero with an actionable validation error.
6. Add the commented default line to generated `.lisa.toml`.
7. Extend parse, resolve, template, known-key, and invalid-value tests.

Verification:

```bash
cargo test -p lisa-cli config::tests:: --lib
```

Atomic criterion: positive TOML values resolve exactly, zero cannot create an
infinite contract, and valid input has no unknown-key warning.

## Step 3 — transport and document the setting

Modify:

- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/src/init.rs`;
- `crates/lisa-cli/src/setup_guide.rs`.

1. Emit the resolved value in generated KDL.
2. Extend layout assertions.
3. Add the commented setting to ownership-aware init merges.
4. Extend init merge tests that enumerate scheduling keys.
5. Document its start boundary, default, and one-fallback behavior in the setup
   guide.

Verification:

```bash
cargo test -p lisa-cli loop_cmd::tests:: --lib
cargo test -p lisa-cli init::tests:: --lib
cargo test -p lisa-cli setup_guide::tests:: --lib
```

Atomic criterion: `.lisa.toml -> ResolvedConfig -> generated KDL -> PluginConfig`
is complete and discoverable to existing projects after init upgrade.

## Step 4 — enrich assignment state without changing ownership semantics

Modify `crates/lisa-plugin/src/lib.rs`.

1. Add optional absolute deadlines to pending and recovering variants.
2. Add terminal `RecoveryFailed`.
3. Update every construction and match site.
4. Generalize active assignment generation lookup across pending/recovering.
5. Keep `seat_is_owned` true only for `Owned`.
6. Generalize exact acknowledgment promotion across the two active states.

Verification:

```bash
cargo test -p lisa-plugin recycled_codex_ownership
cargo test -p lisa-plugin codex_ack_signal
```

Atomic criterion: existing pending/stale/exact/duplicate behavior is preserved,
and recovery identities can use the same detector without weakening matching.

## Step 5 — arm deadlines only at delivery

Add `start_assignment_ack_wait(pane, now)` and wire it into all prompt delivery
sites.

1. Leave new pending assignments unarmed during `/clear` and `/exit` transport.
2. Arm after `.cleared` prompt submission.
3. Arm after clear-timeout prompt submission.
4. Arm after exit-grace launch submission.
5. Cover immediate tagged delivery if a future `FreshExec` adapter uses it.
6. Never restart an already-armed deadline.

Update current transition tests to assert:

- scheduling into `WaitingForClear` has no deadline;
- clear completion/timeout arms one;
- `WaitingForExit` has no deadline before launch;
- exit-grace launch arms one.

Verification:

```bash
cargo test -p lisa-plugin assignment_ack_wait
cargo test -p lisa-plugin transition_timeouts
cargo test -p lisa-plugin recycle_exit_grace
```

Atomic criterion: no assignment can time out before its tagged prompt is sent,
and every actual tagged delivery becomes finite.

## Step 6 — implement the first timeout and recovery launch

Add injected-time timeout evaluation and the recovery-begin transition.

1. Collect expired pending seats without holding mutable borrows.
2. Validate pane/ticket reservation consistency.
3. Allocate a new generation.
4. replace pending state with recovering/`None` before input.
5. submit `/exit` once and enter `WaitingForExit`.
6. clear stale abandoned-TUI attention flags.
7. use the existing exit grace to submit one fresh Codex launch carrying the
   recovery generation.
8. reset transport to `Idle`, arm the recovery deadline, and retain not-owned
   recovering state.
9. record one launch event at actual fallback delivery.

Verification:

```bash
cargo test -p lisa-plugin withheld_ack
cargo test -p lisa-plugin recovery_launch
```

Atomic criterion: the original generation cannot acknowledge after the state
edge, the ticket and pane remain paired, and repeated polls launch no second
fallback.

## Step 7 — implement success and terminal failure

1. Allow the exact recovery generation acknowledgment to promote to `Owned`.
2. Reject the abandoned original generation during recovery.
3. On recovery deadline expiry, enter `RecoveryFailed`.
4. On `.error` during recovery, use the same terminal path.
5. Mark the retained thread failed and record an error alert.
6. Keep the slot/ticket/thread association to block automatic retry.
7. Log an explicit instruction to reset the ticket.
8. Ensure repeated timeout/error checks do not add launches or duplicate state
   transitions.

Verification:

```bash
cargo test -p lisa-plugin recovery_ack
cargo test -p lisa-plugin recovery_failure
cargo test -p lisa-plugin error_signal
```

Atomic criterion: recovery either becomes exactly one owner or terminates in a
named state that cannot silently retry.

## Step 8 — acceptance test

Add one scheduler test that exercises the criterion as a continuous scenario:

1. schedule a ticket into a resident Codex pane;
2. submit the tagged prompt through the clear handler;
3. verify pending, armed, and not-owned;
4. withhold acknowledgment and advance past the configured deadline;
5. verify recovering, different generation, same ticket, and `/exit` transport;
6. advance past exit grace and verify one fresh launch for the same ticket;
7. invoke transition evaluation again and verify launch count remains one;
8. withhold fallback acknowledgment and advance past its deadline;
9. verify `RecoveryFailed`, retained failed thread/reservation, actionable error,
   no ownership, and no additional launch.

Add a companion success scenario that injects the matching recovery payload and
verifies exactly-once `Owned` promotion.

Verification:

```bash
cargo test -p lisa-plugin bounded_ack_wait
```

## Step 9 — focused and package verification

Run:

```bash
cargo fmt --all -- --check
cargo test -p lisa-core
cargo test -p lisa-plugin
cargo test -p lisa-cli
```

If formatting fails, run `cargo fmt --all`, inspect only ticket-owned path
changes, and repeat the check.

## Step 10 — workspace, WASM, and lint verification

Run:

```bash
cargo test --workspace
just check
cargo clippy -p lisa-core --lib -- -D warnings
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo clippy -p lisa-cli --bin lisa -- -D warnings
```

Document unrelated pre-existing all-target lint failures rather than modifying
out-of-scope tests.

## Step 11 — transaction and ownership audit

1. Inspect `git diff --` for the six production paths.
2. Run `git diff --check --` for those paths.
3. Confirm no unrelated path was modified by formatting.
4. Write `progress.md` with implementation, deviations, test results, and exact
   ownership paths.
5. Commit the production unit only:

```bash
lisa commit-ticket \
  --ticket-id T-033-01-04 \
  --message "feat: bound Codex assignment recovery" \
  --include crates/lisa-core/src/types.rs \
  --include crates/lisa-cli/src/config.rs \
  --include crates/lisa-cli/src/loop_cmd.rs \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-cli/src/setup_guide.rs \
  --include crates/lisa-plugin/src/lib.rs
```

If the installed binary lacks the command, build the repository CLI and invoke
that exact binary, matching the preceding ticket's established fallback.

6. Verify the resulting commit contains only those paths.
7. Confirm all ticket-owned source paths are clean and the ordinary index is
   untouched.

## Step 12 — review handoff

Write `review.md` with:

- source commit and exact file summary;
- state-machine and configuration behavior;
- acceptance-criterion evidence;
- focused/package/workspace/WASM/lint results;
- ownership and transaction audit;
- open concerns and known limits;
- critical issues, if any.

Do not edit ticket frontmatter. Stop after `review.md`; Lisa owns phase movement,
the final artifact transaction, Done publication, and seat release.

## Expected non-changes

- Claude scheduling and prompts remain byte-for-byte unchanged.
- Ordinary fresh Codex assignment remains immediately owned.
- No UI rendering changes.
- No hook or acknowledgment detector changes.
- No live Codex execution or token spend.
- No ordinary-index Git operations.
