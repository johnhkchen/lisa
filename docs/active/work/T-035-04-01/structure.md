# T-035-04-01 Structure — file and interface blueprint

## Change inventory

The implementation modifies the native plugin scheduler, provider adapters, lifecycle
templates, and dashboard state. It renames the acknowledgement classifier to reflect its
new provider-neutral role. No core data types, ticket parser, CLI command surface, or
shared documentation paths need source changes.

Ticket-owned source paths:

- modify `crates/lisa-plugin/src/lib.rs`;
- modify `crates/lisa-plugin/src/adapter.rs`;
- rename `crates/lisa-plugin/src/codex_ack.rs` to
  `crates/lisa-plugin/src/assignment_ack.rs`;
- modify acknowledgement fixture references only if required by the rename;
- modify `crates/lisa-plugin/src/ui.rs`;
- modify `crates/lisa-cli/src/templates.rs`.

No production file is deleted without a replacement. The old classifier module path is
removed as part of the rename.

## `crates/lisa-plugin/src/assignment_ack.rs`

This module remains crate-private and pure. It has no scheduler or filesystem access.

Rename public-within-crate types and functions:

```rust
AssignmentRef<'a> {
    ticket_id: &'a str,
    generation: u64,
}

tag_assignment(prompt: &str, assignment: AssignmentRef<'_>) -> String
detect_assignment_ack(payload_json: &str, pending: AssignmentRef<'_>) -> bool
```

The serialized marker schema and `LISA_ASSIGNMENT ` prefix remain byte-compatible.

`LifecycleEvent` remains a minimal deserialization envelope with:

```rust
hook_event_name: String
prompt: Option<String>
```

Detection continues to require `UserPromptSubmit`, a whole marker line, and exact ticket
plus generation equality.

Unit tests remain colocated. Rename test helper terminology from Codex-specific to
provider-neutral. Add a payload with Claude-like extra fields if useful to prove unknown
provider fields are ignored without weakening the exact marker contract.

Existing JSON fixture files can remain under `tests/fixtures/codex_ack/` because their
contents are historical wire examples and moving them adds no behavioral value. If moved,
the rename must be included explicitly in the same source unit.

## `crates/lisa-plugin/src/adapter.rs`

Update module imports to use `assignment_ack`.

Keep `SpawnContext` as the single launch/assignment context. Its existing fields already
provide ticket directory, ticket ID, pane ID, attempt ID, artifact directory, and optional
assignment generation.

Extend `AgentAdapter` with a provider-specific complete assignment method:

```rust
fn assignment_text(&self, ctx: &SpawnContext) -> String;
```

Keep `reuse_prompt`, but implement it from `assignment_text` and optional marker tagging.
This preserves existing reused-session call sites while separating full instructions from
fresh process launch.

Add a shared bounded-reference helper, either as a trait default or module function:

```rust
fn assignment_reference(
    &self,
    ctx: &SpawnContext,
    assignment_path: &Path,
) -> String;
```

It formats only the read-file instruction and exact marker. It requires
`assignment_generation: Some`; absence is a programming error at a fresh delivery call
site and should fail explicitly rather than emit an untagged message.

`ClaudeCodeAdapter::launch_command` calls the simplified bare
`build_claude_command`. It no longer calls `ticket_prompt`.

`ClaudeCodeAdapter::assignment_text` calls `ticket_prompt` with `CLAUDE.md`.

`CodexAdapter::interactive_line` removes `assignment_prompt` and all prompt argv.
It retains lifecycle environment, provider flags, optional model, and `.error` fallback.

`CodexAdapter::assignment_text` calls `ticket_prompt` with `AGENTS.md`.

`reuse_prompt` for both providers continues to return full instructions. Codex retains
attempt tagging for pending reused delivery. Claude remains untagged on its currently
immediate-owned reuse path.

Adapter tests are reorganized around two contracts:

- fresh launch is bare and bounded;
- assignment text/reference contains the expected provider context and exact marker.

## `crates/lisa-plugin/src/lib.rs`: command construction

Change the module declaration from `mod codex_ack` to `mod assignment_ack`.

Simplify `build_claude_command` to inputs that affect a bare launch:

```rust
fn build_claude_command(
    ticket_id: &str,
    pane_id: u32,
    attempt_id: u64,
    model: Option<&str>,
    lisa_bin: Option<&str>,
) -> String
```

It no longer accepts ticket directory or artifact directory and never calls
`ticket_prompt`.

Keep `shell_quote`, `ticket_prompt`, and `finish_up_prompt` unchanged except for comments
that previously described prompt-bearing fresh launches.

## `crates/lisa-plugin/src/lib.rs`: assignment publication

Add a constant or helper for the deterministic assignment filename:

```rust
const ASSIGNMENT_FILE_NAME: &str = "assignment.md";
```

Add a pure path helper if needed:

```rust
fn assignment_file(artifact_dir: &Path) -> PathBuf
```

Add an atomic publication method parallel to `prepare_fresh_launch`:

```rust
fn prepare_assignment(
    artifact_dir: &Path,
    assignment: &str,
) -> Result<PathBuf, String>
```

It creates the attempt directory, writes a unique same-directory temporary file, renames
to the deterministic destination, cleans failed temporary files, and returns the final
path.

Keep launch-script preparation separate. The launch method continues to return only the
bounded shell indirection.

## `crates/lisa-plugin/src/lib.rs`: seat state

Extend `SeatAssignmentState` with:

```rust
ReadyForAssignment {
    generation: u64,
}

Delivering {
    generation: u64,
    ack_deadline: SystemTime,
    retries: u8,
}

DeliveryFailed
```

Use an always-present deadline in `Delivering` because the state is entered only after
actual pane submission. This prevents an ambiguous armed/unarmed delivery variant.

Keep `AssignedPendingAck` for reused Codex and `Recovering` for its predecessor recovery
path.

Update every exhaustive state match:

- active acknowledgement generation;
- deadline collection;
- timeout dispatch;
- release/reset behavior where matched;
- UI reduction;
- tests.

`seat_is_owned` remains exact equality with `Owned`.

## `crates/lisa-plugin/src/lib.rs`: process readiness

Rename or update `acknowledge_process_start` documentation and transition. Exact current
evidence changes only:

```text
Starting -> ReadyForAssignment
```

It must not write pane input, arm an acknowledgement deadline, or establish ownership.

`check_process_start_signals` remains the only `.started` consumer. It continues to
remove all candidate files before admission and fail closed.

## `crates/lisa-plugin/src/lib.rs`: ready delivery

Add a host-free decision/helper boundary where practical, plus one stateful dispatcher:

```rust
fn deliver_ready_assignments(&mut self)
fn deliver_assignment_to_pane(&mut self, pane_id: u32, retries: u8) -> Result<(), String>
```

The dispatcher:

1. snapshots ready pane IDs;
2. validates state is still `ReadyForAssignment`;
3. resolves exact slot ticket and current lease;
4. derives exact attempt `assignment.md` and verifies it is a file;
5. resolves the ticket adapter and constructs `SpawnContext` with generation;
6. builds the bounded assignment reference;
7. sends it through `send_line_to_pane`;
8. computes the Enter-aware acknowledgement deadline;
9. writes `Delivering` with retry count zero;
10. logs the delivery transition.

The dispatcher must not borrow slot state across `send_line_to_pane` or logging calls.

If preparation preconditions fail, transition directly to `DeliveryFailed` through the
shared failure helper rather than remaining Ready indefinitely.

Run `deliver_ready_assignments` once per poll before `check_process_start_signals`.

## `crates/lisa-plugin/src/lib.rs`: deadline helper

Extract the repeated deadline calculation from `start_assignment_ack_wait`:

```rust
fn assignment_ack_deadline(&self, now: SystemTime) -> SystemTime
```

The existing helper for `Starting`, `AssignedPendingAck`, and `Recovering` calls this.
Ready delivery uses it when constructing `Delivering`.

This keeps the configured timeout and deferred-Enter allowance identical across old and
new acceptance states.

## `crates/lisa-plugin/src/lib.rs`: acknowledgement

Rename `active_assignment_generation` documentation to provider-neutral wording and add
`Delivering` to its accepted variants.

Rename `acknowledge_codex_assignment` to `acknowledge_assignment`. It validates:

- state exposes an active generation;
- pane has a ticket and attempt lease;
- ticket, generation, and current authority match;
- `assignment_ack::detect_assignment_ack` accepts the payload.

It then changes the state to `Owned` for `Delivering`, `AssignedPendingAck`, or
`Recovering`.

Rename `check_codex_ack_signals` to `check_assignment_ack_signals` and update activity
text to avoid claiming every accepted event is Codex-specific.

## `crates/lisa-plugin/src/lib.rs`: bounded chat recovery

Add:

```rust
const MAX_ASSIGNMENT_DELIVERY_RETRIES: u8 = 1;
```

Add a failure helper parallel to `fail_startup`:

```rust
fn fail_assignment_delivery(&mut self, pane_id: u32, reason: &str)
```

It accepts only `ReadyForAssignment` or `Delivering`, writes `DeliveryFailed` first,
fails the thread, retains slot and lease, deduplicates error alerts, and logs operator
reset guidance.

Add a retry helper that reuses the same exact attempt assignment reference and writes a
new `Delivering { retries: 1, ... }` only after sending.

Extend `check_assignment_ack_timeouts_at`:

- expired `Delivering` with `retries < 1` invokes one redelivery;
- expired `Delivering` with `retries == 1` invokes terminal delivery failure;
- all existing Starting, AssignedPendingAck, and Recovering branches remain.

Repeated deadline evaluation after failure is inert.

## `crates/lisa-plugin/src/lib.rs`: scheduling

After minting and registering a lease, build a `SpawnContext` and call
`adapter.assignment_text`. Atomically publish it before any `/exit`, `/clear`, or fresh
launch input for fresh paths. To minimize scope, resident reuse may keep its current full
prompt path; fresh and cross-provider launch paths require the file.

For every fresh process path, set `assignment_generation` to the attempt ID for later
chat reference construction. The launch adapter ignores this field.

Fresh assignment state remains `Starting`; it is never initialized as pending ack or
Owned.

The launch script payload is now bare. Existing launch logging records only the bounded
`sh` path, so no full assignment leaks into activity output.

Cross-provider delayed launch remains unarmed until after exit grace. Its eventual exact
`.started` evidence enters the same Ready state.

## `crates/lisa-cli/src/templates.rs`

Update `ON_ACK_HOOK` comments from Codex-specific to native-provider assignment evidence.
The script bytes and atomic behavior remain unchanged.

Add a `UserPromptSubmit` group to `settings_local_json` using the same guarded
`on-ack.sh` command as Codex.

Add the same hook through `merge_hooks` with `ensure_hook` so repeated initialization is
idempotent and existing user groups remain intact.

Update function documentation to include prompt submission in Claude's generated hook
set.

Template tests assert:

- Claude settings contain one `UserPromptSubmit`/`on-ack.sh` binding;
- Claude merge adds it when absent;
- repeated merge does not duplicate it;
- Codex behavior remains one binding.

`init.rs` requires no functional change because it already installs `on-ack.sh` for all
initialized projects.

## `crates/lisa-plugin/src/ui.rs`

Extend `SeatAssignmentStatus` with:

- `ReadyForAssignment`;
- `Delivering`;
- `DeliveryFailed`.

Labels:

- `ready-for-assignment`;
- `delivering`;
- `delivery-failed`.

Colors:

- ReadyForAssignment and Delivering use yellow;
- DeliveryFailed uses red.

Update UI tests that enumerate or render assignment statuses.

## Native test organization

Replace the predecessor fresh-start success test with a full two-stage test that calls
the actual signal consumers and ready dispatcher in poll order.

The test captures queued/written state through scheduler state and, where host shims do
not expose pane bytes, directly verifies the bounded message constructor.

Add separate Claude and Codex loops or a table-driven helper proving identical fresh
state transitions and stale acknowledgement rejection.

Update missing-start tests to expect no change beyond the existing StartupFailed path.

Add missing-chat tests that extract deadlines, evaluate at exact injected times, count
delivery activity, assert one retry, then assert DeliveryFailed and repeated-poll
inertness.

Update existing recycled-Codex and consecutive-reuse tests only where renamed helpers or
bare recovery launch expectations require it. Preserve their semantics.

## Commit units

Unit 1: provider-neutral assignment transport and hook evidence.

Exact likely paths:

- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/assignment_ack.rs`;
- `crates/lisa-plugin/src/codex_ack.rs` as deletion;
- `crates/lisa-cli/src/templates.rs`.

Unit 2: scheduler state machine, bounded recovery, dashboard, and native regressions.

Exact paths:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ui.rs`.

If compilation requires the module rename and scheduler declaration to land atomically,
combine all ticket-owned source paths into one meaningful unit rather than creating a
broken intermediate commit.

Phase artifacts remain private in `.lisa/attempts/T-035-04-01/1/work/` and are not
included in source commits.
