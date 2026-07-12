# Review: deterministic real-Zellij delivery boundary regression

## Outcome

T-035-02-01 is implemented and verified.

The repository now contains a deterministic, model-free integration regression for the
fresh-pane two-stage delivery boundary that previously produced phantom ownership after
terminal truncation.

The regression exercises the current Lisa CLI, its embedded production WASM plugin, real
Zellij panes, real zsh parsing, production launch scripts, production signal scanning,
production assignment injection, production acknowledgement matching, production timeout
handling, and production same-pane recovery.

No scheduler or adapter production behavior was changed by this ticket.

## Commit

Ticket-owned source was committed through Lisa's isolated transaction:

```text
ad8d5915a8cc10260ce690e171fd444c044d4cd1
test(cli): cover real Zellij delivery boundary
```

The commit contains exactly:

- `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Both paths are clean and are not present in the ordinary Git index.

## Rust integration wrapper

`real_zellij_delivery_boundary.rs` adds one ignored Cargo integration test.

It locates the retained shell harness relative to `CARGO_MANIFEST_DIR`.

It passes Cargo's exact freshly built `CARGO_BIN_EXE_lisa` path to the harness.

It captures stdout and stderr, requires a successful harness exit, and independently
requires the stable receipt:

```text
real-zellij-delivery-boundary: PASS
```

The test is ignored because it requires Zellij, zsh, `script`, jq, and a current embedded
`wasm32-wasip1` plugin. Once explicitly invoked, all scenarios are automated.

## Shell harness

The retained harness is strict-mode Bash and validates every external dependency before
creating fixtures.

Each scenario receives:

- an independent temporary root;
- a fresh Git repository and committed baseline;
- a minimal Lisa story and one Claude-routed ticket;
- a dedicated unique Zellij session;
- a disposable HOME and ZDOTDIR;
- a fixture-first PATH;
- a local executable named `claude`;
- private evidence and signal directories;
- explicit wait deadlines and cleanup traps.

The disposable shell home prevents developer zsh startup configuration from selecting an
installed provider ahead of the local stub. The passing test therefore requires no model
provider and spends no model tokens.

The Zellij wrapper preserves Lisa's normal `--layout` launch path while translating it to
Zellij 0.44's named-session creation form. This keeps scenarios isolated from the
developer's surrounding Zellij session.

The harness uses a 140x50 PTY so the production 30% dashboard pane has enough height to
render the scheduler-owned status row.

Pane discovery reads Zellij 0.44's actual JSON schema, distinguishes plugin and terminal
IDs explicitly, excludes the compact-bar plugin, and follows the ticket-titled terminal.

Because direct plugin-pane dumps are empty in this Zellij release, the harness focuses the
explicit Lisa plugin ID and dumps its current viewport. Delivery itself remains directed
by the production plugin to the actual assigned terminal pane.

## Stub-provider contract

The stub supports `--version`, so normal `lisa loop` preflight succeeds without weakening
production checks.

At launch it records the physical pane ID and `LISA_ATTEMPT_ID` generation.

Positive scenarios hold process-start publication behind an external gate. Once opened,
the stub atomically copies the authoritative pane lease to the `.started` signal.

The harness requires the start event and subsequent removal of `.started`. Production
removes that file while scanning the exact current lease, and cannot deliver chat without
transitioning through ReadyForAssignment.

The stub reads the live terminal input and records the bounded two-line assignment:

```text
Read and follow the complete assignment at <attempt-private assignment path>.
LISA_ASSIGNMENT {"ticket_id":"T-STUB-01","generation":N}
```

It never opens or executes the assignment document.

Positive acknowledgement is independently gated. When opened, the stub writes normalized
`UserPromptSubmit` JSON containing the exact received prompt, then remains resident.

## Successful-boundary coverage

The success scenario proves:

- one fresh provider launch;
- one physical pane and one attempt generation;
- a bounded attempt-private launch script;
- no assignment prose in the launch script;
- exact process-start signal publication and consumption;
- non-ownership before chat acknowledgement;
- one accepted bounded chat reference;
- a visible Delivering scheduler state;
- continued non-ownership while acknowledgement is gated;
- exactly one matching acknowledgement;
- a visible Owned state only after that acknowledgement;
- no installed provider process.

This is the core regression for the former truncated inline-launch failure.

## Suppressed-start coverage

The `suppress-start` scenario records launch but never publishes `.started`.

It proves:

- the original startup cannot reach chat or ownership;
- Lisa performs exactly one recovery launch;
- both launches use the same physical terminal pane;
- the successor generation is strictly greater;
- the replacement also cannot reach chat or ownership;
- the scenario reaches a durable failure alert within its wall-clock bound;
- no third launch occurs during the post-failure stability window.

## Suppressed-ack coverage

The `suppress-ack` scenario publishes valid process start and accepts terminal chat but
never writes `.ack`.

It proves:

- one provider launch;
- exact process-start consumption;
- non-ownership before acknowledgement;
- visible Delivering state;
- exactly one initial chat plus one bounded retry;
- no acknowledgement event;
- durable bounded failure;
- no provider restart;
- no third chat during the post-failure stability window;
- no Owned publication.

## Real `dquote>` recovery coverage

Generation 1 of the `dquote` scenario exits without start evidence and schedules an
unterminated double quote plus Enter into its own real terminal pane.

The harness requires the real zsh `dquote>` continuation prompt in the terminal viewport.

It then proves:

- no generation-1 process-start signal;
- no generation-1 chat or ownership;
- one successor launch after Lisa's Ctrl-C and shell-readiness recovery;
- the same physical terminal pane across both generations;
- a strictly greater successor generation;
- exactly one replacement process-start signal;
- exact replacement start-signal consumption;
- one replacement bounded chat;
- visible replacement Delivering state while non-owned;
- one matching replacement acknowledgement;
- visible Owned only after that acknowledgement;
- no third launch during the stability window.

## Verification performed

The focused real-Zellij command passed all four scenarios:

```text
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
test result: ok. 1 passed; 0 failed
finished in 125.19s
```

The current WASM was built before this run and re-embedded into the test CLI.

Additional verification passed:

- shell syntax validation;
- Rust formatting check;
- focused integration-test compilation;
- `cargo test --workspace`;
- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- final `git diff --check`;
- exact commit path inspection;
- residual named-session and fixture-process checks.

## Open concerns and limitations

The regression is intentionally ignored in the default workspace suite because it needs
real Zellij, a PTY, zsh, jq, `script`, and the WASM target.

Before invoking it from a checkout, the current release WASM must be built and embedded
according to the repository's existing CLI build contract. `just build-cli` performs the
required WASM-first build; for a debug test binary, the WASM input must likewise be newer
than the CLI build output so Cargo reruns `crates/lisa-cli/build.rs`.

The harness targets the observed Zellij 0.44 JSON and CLI behavior. A future Zellij schema
or named-session semantic change may require harness adaptation.

The process-start consumption helper uses pane 0 because the isolated one-ticket layout
deterministically assigns the first agent slot. Launch-event assertions independently
verify the actual pane and same-pane recovery behavior.

The transient ReadyForAssignment render frame is not treated as authoritative evidence.
Instead the test requires exact `.started` publication and plugin consumption, then the
later production chat delivery that is reachable only through ReadyForAssignment, with
non-ownership checked before acknowledgement.

No critical implementation issue remains. No production TODO was introduced.

## Handoff

The acceptance criterion is satisfied by committed source and a passing real-Zellij run.

Lisa should publish this Review artifact and prepare the completion commit. This attempt
must remain on T-035-02-01 until Lisa confirms that completion commit.
