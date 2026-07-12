# Structure: gate ownership on acknowledgment

## Change inventory

### Modify

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`
- `crates/lisa-plugin/src/codex_ack.rs`
- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-cli/src/init.rs`

### Create

- `docs/active/work/T-033-01-03/research.md`
- `docs/active/work/T-033-01-03/design.md`
- `docs/active/work/T-033-01-03/structure.md`
- `docs/active/work/T-033-01-03/plan.md`
- `docs/active/work/T-033-01-03/progress.md`
- `docs/active/work/T-033-01-03/review.md`

### Delete

- None.

## Plugin assignment-state shape

Change the pending variant in `lib.rs` from:

```rust
AssignedPendingAck
```

to:

```rust
AssignedPendingAck { generation: u64 }
```

`Owned` remains fieldless. `Recovering` remains fieldless in this ticket because timeout and
recovery metadata are owned by `T-033-01-04`.

All exact matches in tests and production code must destructure or use `matches!`. No helper
may treat all map presence as ownership.

## Scheduler generation allocation

Add to `State`:

```rust
next_assignment_generation: u64,
```

Add a private allocator:

```rust
fn allocate_assignment_generation(&mut self) -> u64
```

The allocator advances the counter and returns a nonzero process-local generation. It is called
only for recycled/reused Codex assignments that enter pending state.

Scheduling computes an optional generation immediately after it knows route and reuse status:

```rust
let assignment_generation =
    (route.agent == AgentClient::Codex && reused_seat)
        .then(|| self.allocate_assignment_generation());
```

The value is used for both prompt construction and state insertion. The same generation must
survive delayed clear and exit delivery paths.

## Spawn context boundary

Extend `adapter::SpawnContext`:

```rust
pub assignment_generation: Option<u64>,
```

Every constructor in scheduler code and tests supplies the field explicitly. Contexts unrelated
to initial assignment delivery, such as timeout reconstruction, recover the generation from the
current seat state when applicable.

Claude adapters ignore the field. Codex adapters use it only to tag ticket prompts.

## Codex adapter organization

In `adapter.rs`, import:

```rust
use crate::codex_ack::{tag_codex_assignment, CodexAssignmentRef};
```

Add a private helper on `CodexAdapter`:

```rust
fn assignment_prompt(&self, ctx: &SpawnContext) -> String
```

The helper:

1. builds the existing `ticket_prompt` using `AGENTS.md`;
2. returns it unchanged when generation is `None`;
3. tags it with ticket ID and generation when generation is `Some`.

Both `interactive_line` and `reuse_prompt` call this helper. This prevents launch and reuse
representations from drifting.

## Detector module cleanup

In `codex_ack.rs`, remove the narrow `allow(dead_code)` attributes from:

- `CodexAssignmentRef`;
- `tag_codex_assignment`;
- `detect_codex_ack`.

They become live scheduler/adapter dependencies. No parsing contract changes are required.

## Scheduler promotion API

Add to `impl State` in `lib.rs`:

```rust
fn acknowledge_codex_assignment(&mut self, pane_id: u32, payload_json: &str) -> bool
```

Its read phase gathers:

- current slot ticket ID;
- current pending generation.

It builds `CodexAssignmentRef` and calls `detect_codex_ack`. Only a true result permits insertion
of `SeatAssignmentState::Owned`. Returning true means the pending-to-owned mutation occurred.

The method must not borrow `seat_assignments` mutably while reading the slot or invoking the
detector. Read/copy first, classify, then insert.

## Ack signal scanner

Add:

```rust
fn check_codex_ack_signals(&mut self)
```

near the heartbeat/transition scanners.

Filename contract:

```text
.lisa/signals/pane-<u32>.ack
```

For each matching entry:

1. parse pane ID from filename;
2. read the entire UTF-8 payload;
3. remove the signal file;
4. skip unreadable payloads;
5. invoke `acknowledge_codex_assignment`;
6. on true, bump pane activity and log one informational event.

Malformed filenames remain untouched by this scanner, consistent with other type-specific
scanners. Validly named files are always consumed so rejected payloads cannot retrigger.

## Poll integration

Add `check_codex_ack_signals()` to `poll_tick` after basic liveness/question signals and before
transition timeout or future recovery evaluation. The scanner is a provider acceptance consumer,
not a phase-completion consumer.

Suggested order:

```text
heartbeat
awaiting
acknowledgment
artifact and idle phase processing
transition and error processing
transition timeouts
future acknowledgment deadline processing
```

No existing consumer is removed.

## Deferred prompt reconstruction

`handle_cleared_signal` reconstructs `SpawnContext` after scheduling. It must look up the pending
generation for that pane and pass it to the adapter so the actual post-clear prompt carries the
same identity stored in the assignment map.

`check_transition_timeouts` has two delivery sites:

- exit-grace fresh launch;
- clear-timeout prompt fallback.

Both must obtain the pending generation from `seat_assignments` and populate `SpawnContext`.
Fresh/owned assignments naturally yield `None`.

Introduce a small query helper if repeated extraction harms clarity:

```rust
fn pending_assignment_generation(&self, pane_id: u32) -> Option<u64>
```

## CLI hook template

Add `ON_ACK_HOOK` in `templates.rs` beside other lifecycle scripts.

The script contract:

- input: raw Codex `UserPromptSubmit` JSON on standard input;
- environment: `LISA_PANE_ID` supplied by the pane launch;
- output: atomically replaced `pane-<id>.ack` payload file;
- absent pane ID: consume no state and exit successfully;
- write failure: remove temporary file best-effort and do not create a final ack file.

No lifecycle classification occurs in shell.

## Codex hook JSON

Extend `codex_hooks_json()` with:

```json
"UserPromptSubmit": [
  {
    "hooks": [
      {
        "type": "command",
        "command": "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
      }
    ]
  }
]
```

Extend `merge_codex_hooks()` with `ensure_hook` for the same event and command. Use no matcher;
deduplication is by Lisa script path and preserves user entries.

## Init and validation inventory

Where `init.rs` enumerates managed hook scripts, add:

```text
on-ack.sh -> templates::ON_ACK_HOOK
```

Add `on-ack.sh` to validation's required executable scripts. Existing ownership-aware planning,
legacy matching, mode handling, and mutation reporting remain unchanged.

No generated project-instance `.lisa/hooks/on-ack.sh` is committed by this ticket; the source
template is the durable product change.

## Unit-test placement

### `lib.rs`

- extend recycled scheduling assertions to capture pending generation;
- add a test for no ack, stale ticket, stale generation, matching ack, duplicate ack;
- add filesystem scanner coverage for matching and rejected payloads;
- update manually constructed pending variants with explicit generations;
- verify timeout paths preserve the pending generation.

### `adapter.rs`

- update `spawn_ctx` helper with optional generation;
- verify Codex launch command contains a detectable assignment marker;
- verify Codex reuse prompt contains the marker;
- verify no-generation Codex output remains untagged;
- retain Claude equality tests.

### `templates.rs`

- require generated `UserPromptSubmit` array and `on-ack.sh` command;
- require merge idempotence for the Lisa ack hook;
- continue proving the preexisting user `UserPromptSubmit` hook survives.

### `init.rs`

- existing init/validate tests should exercise inventory additions;
- update explicit assertions or expected hook name arrays where needed.

## Ownership and commit boundary

The five Rust source files are ticket-owned. Work artifacts remain for Lisa's final completion
transaction. The isolated source commit must include only exact repository-relative paths for
the five Rust files. Existing unrelated modified and untracked files remain untouched.

## Verification boundary

Run in increasing scope:

1. focused plugin acknowledgment tests;
2. focused adapter and template tests;
3. `cargo test -p lisa-plugin`;
4. `cargo test -p lisa-cli`;
5. `cargo test --workspace`;
6. WASM target check or project `just check` if available;
7. `cargo fmt --all -- --check`;
8. `cargo clippy --workspace --all-targets -- -D warnings` when baseline permits;
9. `git diff --check` on exact ticket-owned source paths.

## Structural outcome

The finished path is linear and auditable:

```text
scheduler allocates generation
  -> Codex adapter tags delivered prompt
  -> UserPromptSubmit hook atomically stores raw JSON
  -> plugin scanner reads pane payload
  -> detector compares current ticket + generation
  -> pending state becomes owned once
```

Every other event remains liveness, transport, phase, or failure evidence rather than ownership.
