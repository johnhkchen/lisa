# Structure: real-Zellij delivery regression

## Change summary

The implementation adds test-only source in the Lisa CLI crate and leaves production
Rust modules unchanged.

Two source units are created:

```text
crates/lisa-cli/tests/real_zellij_delivery_boundary.rs
crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

The first integrates the harness with Cargo. The second owns fixture construction, stub
provider behavior, Zellij control, assertions, diagnostics, and cleanup.

No source file is deleted or modified outside those paths unless implementation discovers
a concrete harness-enabling defect.

## Cargo integration wrapper

File: `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`

Define one ignored integration test:

```rust
#[test]
#[ignore = "requires zellij, zsh, script, and wasm32-wasip1"]
fn real_zellij_delivery_boundary()
```

The wrapper resolves the harness relative to `CARGO_MANIFEST_DIR`.

It obtains the freshly built CLI through `env!("CARGO_BIN_EXE_lisa")`.

It launches `bash <harness>` with `LISA_BIN` set to that exact executable.

It captures output so failures include stdout and stderr in the Rust assertion.

It asserts successful exit and a stable terminal receipt such as:

```text
real-zellij-delivery-boundary: PASS
```

The wrapper contains no scenario semantics; all behavioral assertions live in the shell
harness so the same test can be run directly during diagnosis.

## Shell harness top-level organization

File: `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Use bash with strict mode.

Organize the file into these sections:

1. constants and environment validation;
2. cleanup and diagnostic traps;
3. portable timing/poll helpers;
4. Zellij action helpers;
5. fixture and stub construction;
6. state/evidence assertion helpers;
7. scenario runners;
8. top-level sequential execution and receipt.

The script is executable source and must pass `bash -n`.

## Environment inputs

Required:

- `LISA_BIN`: exact freshly built Lisa executable.

Optional:

- `KEEP_LISA_ZELLIJ_FIXTURES=1`: retain fixture roots after a failure for debugging;
- `LISA_ZELLIJ_TEST_ROOT`: parent directory override for controlled environments.

The harness must not inherit a parent `ZELLIJ`, `ZELLIJ_SESSION_NAME`, or plugin attempt
identity into fixture bootstrap.

It prepends each fixture's `bin/` directory to PATH only for that scenario.

## Temporary-root registry

Maintain arrays of created roots and named sessions.

The EXIT trap iterates all sessions with `zellij kill-session` and removes roots unless
retention was requested.

Session names combine the process ID, scenario, and a deterministic-safe suffix.

No cleanup action may target an unnamed or pre-existing session.

## Polling primitives

Provide a portable epoch helper using `date +%s`.

Provide `wait_for_file`, `wait_for_pattern`, and `wait_for_count` functions with explicit
timeout arguments.

Every loop sleeps no more than one second and reports its awaited condition on failure.

Provide negative assertions that inspect retained evidence at known stable boundaries;
do not attempt to prove absence by waiting forever.

## Zellij helpers

Define one function that applies an action to an explicit session:

```text
zellij --session <name> action ...
```

Discover panes with `action list-panes --json --all`.

Extract one plugin ID and the assigned terminal ID. Prefer `jq` when present, but avoid a
new dependency by using stable text tools or a small inline Ruby/Perl fallback if needed.

Dump plugin and terminal screens into scenario-owned evidence files.

Normalize carriage returns and ANSI escapes before state matching.

The helper that injects the unmatched quote targets an explicit terminal pane ID and
sends Enter as byte 13.

## Fixture tree

Each scenario creates:

```text
<root>/
  bin/claude
  evidence/
    events.log
    dashboard.txt
    terminal.txt
    loop.log
  docs/active/tickets/T-STUB-01.md
  docs/active/stories/S-STUB.md
  .lisa.toml
  ... lisa init output ...
```

The attempt and signal directories remain under `.lisa/` and are inspected in place.

The minimal ticket remains at phase Research for the duration of the test.

## Stub-provider executable

The generated `bin/claude` is a standalone bash program.

It reads scenario control from:

- `LISA_STUB_SCENARIO`;
- `LISA_STUB_EVIDENCE_DIR`;
- `LISA_STUB_ACK_GATE`;
- the production `LISA_PANE_ID`, `LISA_TICKET_ID`, and `LISA_ATTEMPT_ID`.

It supports `--version` before requiring attempt identity.

It appends tab-separated event records under an advisory file lock when available; the
scenarios are sequential, so atomic single-line append is sufficient.

### Start publication helper

Read `.lisa/signals/pane-<P>.lease`.

Copy it to a uniquely named temp file and rename to `.started`.

Record the marker bytes in the event log.

Do not synthesize a lease independently.

### Composer loop

Read terminal input one submitted line at a time.

Because the assignment marker is on a second line, accumulate lines until the
`LISA_ASSIGNMENT ` line arrives.

Record both the human-readable assignment-file reference and marker as one logical chat
event.

For normal acknowledgement, wait for the gate and write JSON with the complete accumulated
prompt to `.ack` through temp-and-rename publication.

After ack, remain alive using an interruptible read/sleep loop.

For missing ack, continue accepting the retry and record each complete assignment without
publishing `.ack`.

### Suppressed-start behavior

Record launch but do not publish `.started`.

Wait in a foreground loop that terminates on Ctrl-C, allowing the parent shell to execute
Lisa's reset probe.

Apply the behavior to both generations.

### `dquote>` behavior

For attempt 1 only, record the fault and start a short background helper.

The helper waits until the provider and launch shell have returned, then uses the named
Zellij session and explicit pane ID to write an unmatched quote and Enter.

The attempt-1 provider exits zero without start evidence.

For attempts greater than 1, use normal start/composer behavior.

## Fixture bootstrap function

`create_fixture <scenario>` returns a canonical absolute root.

It calls `LISA_BIN init`, writes minimal story/ticket/config content, and initializes Git.

It writes the stub executable and exports scenario variables only into the loop client
process.

It commits the disposable fixture baseline.

It does not invoke normal Git commands in the parent repository.

## Loop startup function

`start_loop <scenario> <root> <session>` runs approximately:

```text
script -q <loop-log> env ... <LISA_BIN> loop --path <root> --client claude
```

Platform-specific `script` flags are selected once for macOS versus GNU util-linux.

The process runs in the background. The harness waits until the named session responds to
`list-panes` and the plugin pane is present.

The loop PID is retained for cleanup but session liveness is the operational boundary.

## State assertion helpers

`wait_dashboard_state <state> <timeout>` repeatedly dumps the plugin pane and matches the
normalized status label.

`assert_dashboard_not_owned` dumps immediately at a stable gate and rejects `owned` while
allowing `not-owned` wording only if rendering ever introduces it; exact token matching
should prevent substring ambiguity.

`wait_terminal_dquote` requires `dquote>` in the assigned terminal dump.

`assert_same_pane_generations` parses launch event rows and requires all pane values equal,
two strictly increasing generation values, and no third row.

`assert_bounded_launcher` inspects every `.lisa-launch-*.sh` and the visible terminal
command. It requires the provider command be bare and rejects ticket prompt prose as a
positional argument.

## Scenario functions

### `run_success`

Observe ReadyForAssignment, Delivering, and Owned in order with the ack gate between the
last two. Validate one launch, one logical chat, exact marker, and bounded launcher.

### `run_suppressed_start`

Wait for terminal startup failure. Validate two same-pane launches, increasing generation,
no chat, no ack, no Owned, and wall-clock completion.

### `run_suppressed_ack`

Observe ReadyForAssignment and Delivering, then terminal DeliveryFailed. Validate one
launch, two logical chats, no ack, and no Owned.

### `run_dquote_recovery`

Observe real `dquote>`, then generation-2 ReadyForAssignment and Delivering in the same
pane. Open the ack gate and require Owned. Validate exactly two launches and one fault.

## Diagnostic function

On any failed assertion, print all retained scenario artifacts plus:

```text
find <root>/.lisa/signals -maxdepth 1 -type f -print -exec ...
find <root>/.lisa/attempts -name '.lisa-launch-*.sh' -print -exec ...
```

Avoid exposing parent environment variables or unrelated repository content.

## Commit boundaries

Commit the shell harness and Cargo wrapper together if both are required for one runnable
test unit, using exact repository-relative include paths.

If a second source change is necessary after initial execution, commit it as a separate
meaningful harness-fix unit with only its exact touched paths.

## Verification boundaries

Direct checks:

- `bash -n` for shell syntax;
- ignored Cargo integration test for the real boundary;
- wrapper compilation through `cargo test --no-run` or workspace tests;
- `cargo fmt --all -- --check`;
- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`;
- `git diff --check` on ticket-owned paths.

Repository checks:

- ticket-owned source paths clean after isolated commits;
- ordinary index contains no ticket-owned entries;
- unrelated dirty paths unchanged.
