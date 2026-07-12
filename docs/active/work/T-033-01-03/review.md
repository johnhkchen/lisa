# Review: gate ownership on acknowledgment

## Outcome

Implemented end-to-end acknowledgment-gated ownership for recycled Codex seats.

A recycled or reused physical seat assigned to Codex now retains a unique delivery generation,
receives that identity in the submitted prompt, and remains not-owned until the existing Codex
detector accepts a native `UserPromptSubmit` payload for the exact pending ticket and generation.

Stale ticket payloads, stale generations, malformed/non-submit events, unknown panes, released
assignments, and duplicate acknowledgments cannot promote ownership.

## Source commit

```text
74afa61fe81fd95c68e21919dd7d57c01a7063a4
feat: gate Codex seat ownership on acknowledgment
```

Commit scope:

```text
6 files changed, 360 insertions(+), 54 deletions(-)
```

The commit was created through Lisa's isolated transaction with six exact include paths. The
globally installed Lisa binary lacked `commit-ticket`, so the repository-built CLI was used.

No ticket file or RDSPI work artifact was included in the source commit. Lisa owns their final
completion transaction.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Changed pending assignment truth from a fieldless variant to:

```rust
AssignedPendingAck { generation: u64 }
```

Added a process-local monotonic generation source. Scheduling allocates a generation only when a
recycled/reused physical seat is assigned to Codex. Fresh Codex assignments retain the existing
immediate-owned contract, and every Claude path remains immediately owned.

Added helpers to:

- allocate assignment generations;
- query the generation only while a seat is pending;
- classify a payload against current pane ticket and generation;
- promote pending to owned only on an exact detector result.

The promotion helper returns true only when it performs the state edge. Once a seat is `Owned`,
the pending generation is unavailable, making duplicate acknowledgment inert by construction.

Added a `.ack` signal scanner. It reads raw JSON from `pane-<id>.ack`, removes the file, invokes
the exact promotion helper, and records activity/log output only after a successful transition.
It runs before timeout/recovery processing in the scheduler poll.

Deferred delivery sites—the `.cleared` handler, exit-grace launch, and clear-timeout fallback—now
reconstruct `SpawnContext` with the generation retained in assignment state. Reset transport can
therefore delay prompt submission without changing its identity.

### `crates/lisa-plugin/src/adapter.rs`

Extended `SpawnContext` with `assignment_generation: Option<u64>`.

Added one Codex-only assignment-prompt helper. It builds the existing `AGENTS.md` ticket prompt
and appends the canonical `LISA_ASSIGNMENT` JSON line only when a pending generation exists.
Both bare reuse prompts and interactive shell launch commands use the helper.

The interactive command escapes marker JSON quotes for the surrounding shell string. Codex sees
valid JSON in its prompt, while the shell keeps the prompt as one argument.

Claude ignores the new optional context field. Existing equality tests confirm its launch and
reuse output remain unchanged.

### `crates/lisa-plugin/src/codex_ack.rs`

Removed three prerequisite-only dead-code allowances because `CodexAssignmentRef`,
`tag_codex_assignment`, and `detect_codex_ack` now have production consumers.

The detector contract itself was not weakened or changed. Exact event, marker, ticket, and
generation checks remain as implemented by `T-033-01-02`.

### `crates/lisa-cli/src/templates.rs`

Added the managed POSIX `on-ack.sh` template. It copies the complete lifecycle JSON from stdin to
a pane-scoped temporary file and atomically renames it to `pane-<id>.ack`. The plugin therefore
does not observe a partial JSON document.

Generated `.codex/hooks.json` now binds `UserPromptSubmit` to this script. Merge logic installs
exactly one Lisa ack hook while preserving user-owned entries for the same lifecycle event.

Added tests for atomic payload handling, generated JSON structure, user-hook preservation, and
merge idempotence.

### `crates/lisa-cli/src/init.rs`

Added `on-ack.sh` to the ownership-aware init inventory, executable-mode handling, and validation
requirements. Existing installations are safely upgraded when their managed templates are
recognized; unknown project-owned hook content remains protected by the existing safety skip.

Updated validation test infrastructure and the expected empty-project creation count.

### `crates/lisa-cli/data/hooks-guide.md`

Updated the embedded product guide to describe five executable lifecycle scripts, the raw JSON
ack file, `UserPromptSubmit` binding, scaffold inventory, manual setup, and validation contract.

This documentation file was added during implementation review because leaving it unchanged
would ship instructions that falsely claimed Codex had only Stop, clear, and heartbeat bindings.

## Acceptance criterion evaluation

### Recycled seat with no ack never becomes owned

Met. `test_recycled_codex_ownership_requires_matching_ack_exactly_once` schedules a resident
Codex pane onto a new Codex ticket. The resulting state is
`AssignedPendingAck { generation: 1 }`, and `seat_is_owned` is false before any payload is
injected.

Existing clear-timeout and exit-grace tests also retain generation-bearing pending state and
assert not-owned after transport fallback. Prompt delivery by itself is not acceptance.

### Matching ticket-scoped ack promotes pending to owned

Met. The scheduler test builds a real `UserPromptSubmit` payload with the canonical marker for
the pending ticket and generation. `acknowledge_codex_assignment` returns true, assignment state
becomes `Owned`, and `seat_is_owned` becomes true.

The independent signal integration test writes the payload to `pane-7.ack`, runs the scanner,
asserts file consumption, and observes the same owned result.

### Stale ack cannot claim the seat

Met. The scheduler test injects both:

- a previous-ticket payload with the current numeric generation;
- a current-ticket payload with a previous generation.

Both calls return false. State remains pending and not-owned until exact current identity arrives.
The prerequisite detector fixture suite additionally covers still-idle clear events, malformed
JSON, incorrect event types, unrelated fields, and malformed marker placement.

### Duplicate ack transitions exactly once

Met. After exact promotion, the same payload returns false because no pending generation remains.
The state stays `Owned`.

The signal integration test writes the duplicate file and verifies it is consumed without a
second transition log. Exactly one successful acknowledgment event is recorded.

## Test coverage

### Focused scheduler and detector coverage

Passed:

```text
cargo test -p lisa-plugin codex_ack
cargo test -p lisa-plugin test_recycled_codex_ownership_requires_matching_ack_exactly_once
cargo test -p lisa-plugin ack_signal
```

Coverage includes:

- no-ack pending state;
- stale ticket rejection;
- stale generation rejection;
- exact promotion;
- duplicate rejection;
- raw signal-file consumption;
- one transition log;
- detector fail-closed cases.

### Adapter coverage

Passed the adapter-focused suite (23 tests during focused verification).

New coverage proves both Codex launch and reuse delivery contain the expected marker. Existing
tests cover no-generation Codex prompt shape and exact Claude adapter behavior.

### CLI hook and init coverage

Passed template, init, and hooks-guide focused suites.

Coverage includes:

- valid generated Codex hook JSON;
- presence of `UserPromptSubmit`;
- atomic hook template operations;
- user hook preservation;
- merge idempotence;
- init creation and executable mode;
- init/validate round-trip;
- embedded guide integrity.

### Package coverage

Passed:

```text
cargo test -p lisa-plugin
  262 passed, 0 failed

cargo test -p lisa-cli
  269 passed, 0 failed
  atomic_provider_contract: 1 passed, 0 failed
```

### Workspace and WASM coverage

Passed:

```text
cargo test --workspace
cargo fmt --all -- --check
just check
```

`just check` passed the `wasm32-wasip1` plugin check and repeated workspace tests.

### Lint coverage

Strict production-target Clippy passed:

```text
cargo clippy -p lisa-plugin --lib -- -D warnings
cargo clippy -p lisa-cli --bin lisa -- -D warnings
```

The broader `cargo clippy --workspace --all-targets -- -D warnings` remains blocked by existing
test-only lint debt outside this ticket's behavioral change:

- twelve unnecessary-owned-string findings in `crates/lisa-core/src/dag.rs` tests;
- one needless-borrow finding in an existing `crates/lisa-cli/src/init.rs` test.

These were not fixed because they are unrelated to the ticket and the relevant production
targets are clean under warnings-as-errors.

## Diff and transaction audit

- `git diff --check` passed for every ticket-owned source path before commit.
- Commit `74afa61` contains exactly the six reviewed paths.
- All six paths are clean after commit.
- No ticket-owned source file is staged, modified, or untracked.
- The ordinary Git index is empty.
- Existing unrelated worktree modifications and untracked files were preserved.
- Ticket frontmatter status was not edited by the agent.
- Ticket phase movement observed during the workflow was Lisa's artifact detection.

## Open concerns and deferred work

### Acknowledgment deadline and recovery

This ticket deliberately does not bound pending duration. If the hook is absent, unreadable, or
its payload is lost, the seat remains pending/not-owned. `T-033-01-04` owns the deadline,
`Recovering` transition, fresh-session fallback, and terminal recovery handling.

### Existing installations require init upgrade

Projects need to run the updated `lisa init` so `.lisa/hooks/on-ack.sh` and the
`UserPromptSubmit` binding are installed. Validation reports stale Codex lifecycle configuration
after the new binary is in use.

### In-memory generation lifetime

The generation counter is process-local, matching the existing in-memory seat state. A plugin
restart loses both the counter and pending assignments. Persisting live scheduler ownership was
not introduced here.

### Ack signal is last-writer-wins per pane

The atomic payload file has one canonical name per pane. Multiple hook events before one poll can
replace an earlier payload. Exact generation matching prevents false ownership; a lost matching
payload can delay ownership and will be handled by the bounded recovery ticket. Native prompt
submission normally produces one relevant event per assigned delivery.

### Fresh Codex remains immediately owned

Only recycled/reused seats use acknowledgment gating, following the prerequisite ticket and this
ticket's acceptance criterion. If future provider evidence shows initial launches also require
positive acceptance, that is a separate contract change.

### Generation exhaustion

Allocation saturates at `u64::MAX`. Reaching that value would require an infeasible number of
pending assignments in one plugin process. No practical collision risk exists, but the behavior
is documented for completeness.

## Critical issues

None identified.

The acceptance criterion is fully covered, production targets pass strict linting, the WASM and
workspace gates pass, and the source transaction is clean. The only remaining story dependency
is the intentionally separate finite acknowledgment timeout and recovery behavior.

## Handoff summary

Ownership now means positive provider acceptance for recycled Codex seats. Scheduler generation,
adapter marker, native payload hook, file scanner, strict detector, and exact-once promotion form
one traceable chain. No clear, heartbeat, stop, idle, terminal rendering, or timeout fallback can
substitute for acknowledgment, and stale or duplicate evidence cannot claim the seat.
