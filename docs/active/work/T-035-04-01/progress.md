# T-035-04-01 Progress — split start from chat assignment

## Status

Implementation is complete, verified, and committed through Lisa's isolated transaction.

The fresh native-provider path now separates provider readiness from ticket ownership:

```text
Starting -> ReadyForAssignment -> Delivering -> Owned
```

Full instructions are atomically persisted in the exact attempt work directory before
provider lifecycle input. Claude and Codex launch bare, then receive only a bounded tagged
chat reference after exact process-start evidence.

## Completed plan steps

### Provider launch and assignment split

Extended `AgentAdapter` with:

- `assignment_text`, which produces the complete provider-specific RDSPI instructions;
- `assignment_reference`, which produces a bounded read-file chat message plus exact
  `LISA_ASSIGNMENT` marker.

Changed Claude's launch builder to contain only lifecycle environment, provider flags,
and optional routed model.

Changed Codex's interactive line to contain only lifecycle environment, provider flags,
optional routed model, and its existing process-exit `.error` fallback.

Neither fresh command calls `ticket_prompt` or contains assignment instructions.

Reused prompts still use complete assignment text. Existing Codex reused delivery keeps
its generation marker and acknowledgement contract.

### Atomic attempt assignment

Added `ASSIGNMENT_FILE_NAME = "assignment.md"` and
`State::prepare_assignment`.

The helper:

- creates the exact attempt work directory;
- writes a unique same-directory temporary file;
- renames atomically to `assignment.md`;
- removes the temporary file after rename failure;
- returns the exact destination path.

Scheduling constructs provider-specific assignment text and publishes it after minting
the attempt lease but before any `/exit`, `/clear`, or fresh launch input.

The same preparation is performed for a fresh successor minted by inherited Codex
acknowledgement recovery.

### Two-stage scheduler state

Added:

- `ReadyForAssignment { generation }`;
- `Delivering { generation, ack_deadline, retries }`;
- terminal `DeliveryFailed`.

Exact `.started` evidence now changes only `Starting -> ReadyForAssignment`.

Added `deliver_ready_assignments`, run before new process-start consumption in each poll.
That ordering leaves newly ready state observable for one complete scheduler boundary.

Ready delivery validates:

- the reserved ticket;
- the exact slot attempt lease;
- current lease authority;
- the exact attempt `assignment.md` file;
- a non-blocked pane.

It submits only the bounded reference and moves to Delivering with an Enter-aware absolute
deadline.

The existing raw `.ack` scanner and classifier now admit Delivering for both providers.
Operational activity text is provider-neutral.

### Bounded missing-chat recovery

Added `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1`.

The first expired fresh Delivering deadline revalidates and resubmits the same bounded
reference under the same attempt lease.

The second expired deadline calls `fail_assignment_delivery`, which:

- writes `DeliveryFailed` first;
- fails the logical thread;
- retains pane, ticket, lease, and assignment file;
- retains current lease authority;
- adds one deduplicated error alert;
- logs reset-to-retry guidance;
- sends no launch or exit input;
- mints no lease;
- cannot loop on later polls.

A late exact acknowledgement cannot promote DeliveryFailed.

### Fresh inherited recovery

Implementation review found that the inherited E-033 one-fresh-Codex fallback also uses
`launch_command`. Once launch commands became bare, its old synthetic test flow could
acknowledge a prompt that production never sent.

The implementation was corrected before final verification:

- successor assignment text is atomically prepared;
- the fresh recovery launch enters bounded `Starting`;
- exact successor `.started` reaches ReadyForAssignment;
- bounded successor chat reaches Delivering;
- exact successor acknowledgement reaches Owned;
- missing successor chat acknowledgement gets one retry then DeliveryFailed.

This preserves E-033's one-fresh-fallback and newer-generation fencing while applying the
stronger two-stage contract to the fallback itself.

### Claude acknowledgement hook

Added the existing guarded `on-ack.sh` binding to generated Claude
`UserPromptSubmit` settings and idempotent merge behavior.

Updated the hook comments to describe native-provider assignment acknowledgement rather
than Codex alone.

`init.rs` required no change because it already installs `on-ack.sh`.

### Dashboard

Added dashboard statuses and labels:

- `ReadyForAssignment` / `ready-for-assignment` in yellow;
- `Delivering` / `delivering` in yellow;
- `DeliveryFailed` / `delivery-failed` in red.

Owned remains the only green owned state.

## Test coverage added or updated

Adapter tests prove:

- both fresh launch commands omit ticket instructions, context filenames, and assignment
  markers;
- lifecycle identity and routed models remain present;
- Codex retains its `.error` fallback;
- provider assignment text selects `CLAUDE.md` versus `AGENTS.md` correctly;
- the bounded reference contains exact assignment path and generation marker;
- reused Codex prompt tagging remains intact.

Atomic publication tests prove a long, quote-heavy, control-heavy payload is preserved
byte-for-byte and leaves no temporary file.

The main native state regression runs both Claude and Codex through:

- armed Starting after dispatch;
- stale/malformed process-start rejection;
- exact process start to ReadyForAssignment only;
- explicit ready delivery to Delivering;
- stale generation acknowledgement rejection;
- exact acknowledgement to Owned;
- dashboard labels for every positive state;
- duplicate start evidence inert after ownership.

The missing-chat regression proves:

- one original delivery;
- exactly one retry;
- DeliveryFailed after the second deadline;
- no ownership at any missing-ack point;
- retained lease and reservation;
- failed thread and red dashboard status;
- late exact acknowledgement rejected;
- no provider relaunch;
- repeated future timeout evaluation inert.

Predecessor recovery regressions were strengthened to pass the fresh fallback through
start and chat evidence instead of acknowledging it synthetically.

Template tests prove Claude generates and idempotently merges exactly one guarded
`UserPromptSubmit` acknowledgement binding.

## Verification completed

Focused plugin runs:

```text
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin recovery_ack_promotes
cargo test -p lisa-plugin consecutive_reused
```

Result: all focused filters passed.

Plugin suite:

```text
cargo test -p lisa-plugin
```

Result: 280 passed, 0 failed.

CLI suite:

```text
cargo test -p lisa-cli
```

Result: 274 CLI unit tests and the atomic provider integration test passed.

Full workspace:

```text
cargo test --workspace
```

Result: all suites passed, including 274 CLI, 155 core, 280 plugin, integration, and doc
tests.

WASM plus workspace gate:

```text
just check
```

Result: `cargo check -p lisa-plugin --target wasm32-wasip1` passed and the repeated full
workspace suite passed.

Formatting and whitespace:

```text
cargo fmt --all
git diff --check -- <ticket-owned paths>
```

Result: formatting applied only to ticket-owned source; diff check passed.

## Deviations from plan

The Structure artifact proposed renaming `codex_ack.rs` to `assignment_ack.rs`. The
implementation retained the historical module path and wire helper names to avoid a
large compatibility-only rename across predecessor tests and fixtures. Behavior is
provider-neutral: Claude and Codex both emit the same minimal `UserPromptSubmit` payload
to the same exact ticket/generation classifier. Comments and operational log text were
updated where they affect understanding or dashboard operation.

The plan initially described the inherited fresh Codex fallback as remaining in its old
Recovering acknowledgement shape. Implementation review showed that would leave a bare
provider with no chat message. The fallback was strengthened to use the same Starting,
ReadyForAssignment, and Delivering stages under its successor lease. This is required for
correct production behavior and preserves the bounded one-fallback policy.

## Source paths changed

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/ui.rs`;
- `crates/lisa-cli/src/templates.rs`.

No core source, init source, fixture file, ticket frontmatter, or shared work artifact was
manually edited for this implementation.

## Isolated source commit

```text
1bd6c353d0c3d0c11ed86771800f160da7904935
feat(plugin): split provider start from chat assignment
```

Command:

```text
lisa commit-ticket \
  --ticket-id T-035-04-01 \
  --message 'feat(plugin): split provider start from chat assignment' \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/ui.rs \
  --include crates/lisa-cli/src/templates.rs
```

The ordinary index was empty before the transaction. Only the four exact ticket-owned
source paths were included.
