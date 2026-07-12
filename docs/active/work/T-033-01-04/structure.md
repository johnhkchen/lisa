# T-033-01-04 Structure — bounded acknowledgment recovery

## Change set overview

The implementation modifies five production source files and the ticket's work
artifacts. No new Rust module is needed because the behavior extends existing
configuration and scheduler boundaries.

Production files:

1. `crates/lisa-core/src/types.rs`
2. `crates/lisa-cli/src/config.rs`
3. `crates/lisa-cli/src/loop_cmd.rs`
4. `crates/lisa-cli/src/init.rs`
5. `crates/lisa-cli/src/setup_guide.rs`
6. `crates/lisa-plugin/src/lib.rs`

Work artifacts:

- `docs/active/work/T-033-01-04/research.md`
- `docs/active/work/T-033-01-04/design.md`
- `docs/active/work/T-033-01-04/structure.md`
- `docs/active/work/T-033-01-04/plan.md`
- `docs/active/work/T-033-01-04/progress.md`
- `docs/active/work/T-033-01-04/review.md`

No files are deleted. `adapter.rs`, `codex_ack.rs`, hook templates, and `ui.rs`
remain unchanged.

## `crates/lisa-core/src/types.rs`

### `PluginConfig` field

Add:

```rust
pub assignment_ack_timeout_secs: u64,
```

Place it with the other scheduling timeouts, after `wind_down_secs` or directly
before it with documentation distinguishing prompt acceptance from pane quiet.

The public meaning is: maximum seconds after a generation-tagged recycled or
recovery Codex prompt is submitted before Lisa abandons that attempt. It is
always finite and positive.

### Default constant

Add:

```rust
pub const DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS: u64 = 30;
```

Initialize the new field in `PluginConfig::new`.

### KDL map parser

In `PluginConfig::from_config_map`, parse
`assignment_ack_timeout_secs`. Accept only positive `u64` values. Missing,
invalid, or zero values retain the default, keeping direct plugin configuration
safe and non-panicking.

### Tests

Add focused tests for:

- default equals 30;
- positive config-map override;
- zero and malformed direct map values retain the finite default.

Existing struct literals use `..PluginConfig::new()` and should not require
broad mechanical updates.

## `crates/lisa-cli/src/config.rs`

### TOML representation

Add `assignment_ack_timeout_secs: Option<u64>` to `SchedulingConfig`.

### Resolved representation

Add `assignment_ack_timeout_secs: u64` to `ResolvedConfig`, initialize it from
the core default, and resolve the optional TOML setting.

### Validation

Add the key to `known_scheduling` so valid configuration does not emit an
unknown-key warning. Add semantic validation rejecting zero with the message:

```text
assignment_ack_timeout_secs must be at least 1
```

This differs intentionally from session/review timeouts, whose zero values can
disable optional behavior.

### Default template

Add a commented line to `default_config_toml`:

```toml
# assignment_ack_timeout_secs = 30
```

### Tests

Extend the full/default resolution assertions and add focused parse, resolve,
known-key, and zero-rejection tests. Existing test helpers should continue to
construct `ResolvedConfig::default` rather than enumerating every field.

## `crates/lisa-cli/src/loop_cmd.rs`

### KDL transport

Add this plugin configuration entry to `generate_layout`:

```kdl
assignment_ack_timeout_secs "{assignment_ack_timeout_secs}"
```

Pass the resolved value through the format arguments. Keep it adjacent to the
other scheduling timeout keys.

### Tests

Extend the generated layout test to assert the default value appears. If a
custom resolved-config test exists, verify its override is emitted exactly.

No CLI flag is introduced; persistent `.lisa.toml` configuration is sufficient
and matches other non-concurrency scheduling settings.

## `crates/lisa-cli/src/init.rs`

### Ownership-aware TOML merge

Add `assignment_ack_timeout_secs` to the `scheduling_keys` table with the
commented 30-second example. Existing projects upgraded by `lisa init` then get
the discoverable setting without overwriting user configuration.

### Tests

Extend init/config merge assertions that enumerate scheduling keys. Verify the
key is present exactly once after repeated merge behavior where practical.

No hook inventory or executable file changes are needed.

## `crates/lisa-cli/src/setup_guide.rs`

Add one configuration bullet explaining:

- the clock starts after Lisa submits a recycled/recovery Codex prompt;
- default is 30 seconds;
- timeout triggers one fresh-session recovery;
- the value must be positive.

The guide already embeds `default_config_toml`, so this prose is the only
additional documentation surface required here.

## `crates/lisa-plugin/src/lib.rs`

### Assignment state

Replace the current state shape with:

```rust
enum SeatAssignmentState {
    AssignedPendingAck {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    Owned,
    Recovering {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    RecoveryFailed,
}
```

The enum remains private, copyable, and scheduler-owned. Remove the old
dead-code allowance from `Recovering`; all variants gain production consumers.

### Identity helpers

Replace `pending_assignment_generation` with an unowned-assignment helper that
returns the generation for both pending and recovering states. Keep a narrow
pending-only query only if tests or transition classification require it.

Update `acknowledge_codex_assignment` to accept an exact acknowledgment for
either active generation-bearing state and replace it with `Owned`.

### Deadline helper

Add:

```rust
fn start_assignment_ack_wait(&mut self, pane_id: u32, now: SystemTime) -> bool
```

It arms only an active generation-bearing state whose deadline is `None` and
returns whether it changed state. A small deadline-construction helper may keep
duration arithmetic centralized.

### Recovery failure helper

Add:

```rust
fn fail_assignment_recovery(&mut self, pane_id: u32, reason: &str)
```

It sets `RecoveryFailed`, marks the same ticket's retained thread failed,
deduplicates or appends an error alert, and logs the reset instruction. It does
not release the slot or remove the thread.

### Timeout evaluator

Add:

```rust
fn check_assignment_ack_timeouts_at(&mut self, now: SystemTime)
fn check_assignment_ack_timeouts(&mut self)
```

The injected-time method collects expired actions before mutation. Original
pending expiry calls a recovery-begin helper; recovery expiry calls the failure
helper. Already-owned, failed, unarmed, and future deadlines are ignored.

### Recovery-begin helper

Add a focused method that validates the slot/ticket, allocates the recovery
generation, changes assignment state before pane input, sends `/exit`, and
places the slot in `WaitingForExit`. It clears stale flags and logs the
pending-to-recovering transition.

### Delivery sites

Update every `SeatAssignmentState::AssignedPendingAck` construction and pattern
to include `ack_deadline`.

Call `start_assignment_ack_wait` after tagged delivery in:

- scheduling paths that submit immediately;
- `handle_cleared_signal`;
- clear-timeout prompt fallback;
- exit-grace launch handling.

Use the generalized generation helper in every `SpawnContext` reconstruction.

### Exit-grace handling

When the seat is `Recovering`, require the ticket route to resolve to Codex,
submit the recovery-generation launch once, clear transport state, arm the
recovery deadline, retain `Recovering`, and log a `SessionLaunch` event.

The existing non-recovery branch continues to launch cross-provider tickets and
then arms a pending deadline when applicable.

### Error signals

Before generic `.error` reclaim, inspect assignment state. A recovering seat
uses `fail_assignment_recovery`; all other seats retain the current fail,
provenance, release, remove, and retry behavior.

### Poll order

Call `check_assignment_ack_timeouts` after acknowledgment consumption, error
signal consumption, and transition-timeout delivery. This gives current-poll
acknowledgments priority and ensures newly delivered prompts are armed before
deadline evaluation.

### Scheduler tests

Update existing expected enum values with `ack_deadline: None` or a wildcard.
Extend delivery tests to assert deadlines become armed only after prompt launch.

Add:

- one end-to-end withheld-ack acceptance test covering pending, recovering,
  exactly one launch, and terminal recovery failure;
- one recovery acknowledgment success test;
- one late original generation rejection assertion;
- one recovery `.error` terminal-state test if not already covered by the main
  acceptance path.

## Public and private boundaries

The only new public API is `PluginConfig::assignment_ack_timeout_secs` and its
default constant. Scheduler state and helpers remain private to the plugin.
Adapter and detector contracts stay unchanged; they consume the new recovery
generation through the existing `SpawnContext` field.

## Ordering constraints

1. Add core config so downstream CLI structs can reference its default.
2. Complete CLI parse/resolve/layout/init/guide transport.
3. Change assignment state and repair all compiler-reported pattern matches.
4. Add deadline arming at actual delivery sites.
5. Add timeout/recovery/failure behavior.
6. Add acceptance and regression tests.
7. Format and run focused suites before broader verification.

## Transaction boundary

The six production paths form one coherent source unit because neither the
scheduler nor configuration is usable without the other. Commit them through
one exact Lisa isolated transaction after all verification passes. Work
artifacts remain uncommitted for Lisa's completion transaction.
