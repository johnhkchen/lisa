# Design: exact-artifact local dogfood

## Decision goal

The implementation must make one defensible statement:

The release CLI and embedded WASM freshly rebuilt from the current source were
the exact artifacts used by deterministic local dogfood, with an observed
result recorded for every selected fixture.

This is an evidence task rather than a product behavior task.

The design therefore optimizes for artifact identity, fixture relevance,
repeatability, and honest boundaries.

## Option 1: run only the ordinary workspace test suite

Command shape:

`cargo test --workspace`

Advantages:

- broad native unit and integration regression coverage;
- familiar repository quality gate;
- deterministic and free;
- automatically exercises the atomic provider-contract integration test.

Disadvantages:

- the real-Zellij delivery test is ignored by default;
- ordinary native tests do not load the embedded WASM;
- Cargo normally builds debug-profile test artifacts, not the release CLI from
  `just build-cli`;
- a green workspace result alone cannot satisfy the explicit CLI+WASM dogfood
  requirement.

Decision:

Reject as the primary dogfood method.

The workspace suite is useful integrated regression evidence but does not prove
the embedding boundary was exercised.

## Option 2: run only the ignored real-Zellij integration test

Command shape:

`cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture`

Advantages:

- maintained repository entry point;
- loads the CLI's embedded WASM through `lisa loop`;
- deterministic stub provider avoids model usage;
- covers success and three bounded recovery/failure scenarios;
- Rust wrapper verifies the shell harness's PASS receipt.

Disadvantages:

- without `--release`, Cargo supplies a debug CLI, even though the plugin bytes
  originate from the release WASM path;
- Cargo may rebuild the CLI after the explicit `just build-cli`, making exact
  artifact identity less direct;
- it covers delivery and recovery, not isolated CLI ticket transactions;
- Cargo's successful captured output usually exposes only the Rust test receipt
  rather than the individual scenario lines.

Decision:

Retain the real-Zellij fixture, but invoke its shell entry point directly with
the exact freshly rebuilt release binary.

## Option 3: invoke only the real-Zellij shell fixture directly

Command shape:

`LISA_BIN="$PWD/target/release/lisa" bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Advantages:

- exact executable path is explicit;
- the release binary hash can be taken immediately before execution;
- no intervening Cargo build is needed;
- output names all four scenarios and prints the inner stable PASS receipt;
- the embedded WASM is loaded by real Zellij;
- no live model or metered provider is involved.

Disadvantages:

- it bypasses the small Rust wrapper that asserts the PASS string;
- it does not cover CLI commit/completion transactions;
- it relies on local Zellij, zsh, jq, `script`, and timing envelopes;
- a local fixture is not equivalent to installed-provider field validation.

Decision:

Use this as the primary CLI+WASM dogfood fixture.

The shell script already fails unless every assertion and the final receipt are
reached, so direct invocation retains its behavioral gate.

## Option 4: direct release-binary invocation of both maintained fixtures

Commands:

1. run the atomic provider-contract shell harness with `LISA_BIN` set to the
   freshly rebuilt release CLI;
2. run the real-Zellij shell harness with the same `LISA_BIN` value.

Advantages:

- both fixture runs consume the same exact release CLI path;
- artifact hashes can bind observations to concrete bytes;
- the atomic fixture exercises init, validation, isolated implementation
  commits, completion, dependency gating, provider attribution, and foreign
  index preservation;
- the real-Zellij fixture exercises embedded-WASM loading, process-start
  evidence, assignment delivery, acknowledgement, ownership, bounded retry,
  and same-pane recovery;
- both fixtures are deterministic and provider-free;
- both scripts emit stable PASS receipts;
- the combined boundary matches the epic's state and field-regression goals.

Disadvantages:

- the atomic fixture does not itself load the WASM;
- the real-Zellij fixture is slower and depends on real local process timing;
- direct invocation does not test the Rust wrapper plumbing;
- the two fixtures are not an exhaustive replacement for workspace tests.

Decision:

Choose this option.

It is the strongest evidence set that stays inside the ticket's deterministic,
local, no-live-provider boundary.

## Option 5: run the live-provider startup harness

Entry point:

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh`

Advantages:

- tests installed Codex and Claude client behavior;
- builds the CLI and plugin before running by default;
- includes a deterministic preflight;
- records extensive build and runtime evidence.

Disadvantages:

- explicitly live and metered;
- requires provider executables and authentication;
- retains evidence and fixture state by default;
- directly contradicts the story's honest boundary and out-of-scope list.

Decision:

Reject.

No part of this ticket will invoke a live model provider.

## Freshness and artifact identity

Use the canonical repository recipe:

`just build-cli`

This establishes the required order and forces the CLI build script to observe
the release WASM.

After build success, record:

- current source `HEAD`;
- UTC observation time;
- `target/release/lisa` byte size and SHA-256;
- `target/wasm32-wasip1/release/lisa.wasm` byte size and SHA-256;
- CLI-reported version;
- toolchain versions relevant to reproduction.

The CLI hash identifies the container that embeds the plugin.

The standalone WASM hash identifies the plugin build copied during embedding.

The build script does not expose an embedded-section hash independently.

The real-Zellij fixture's successful behavior is the runtime proof that the
embedded plugin is nonempty and loadable.

Use the same canonical absolute CLI path for both fixture runs.

Recheck both artifact hashes after the fixtures.

Unchanged hashes show that fixture execution did not replace the tested bytes.

## Fixture execution order

Run the faster atomic provider-contract fixture first.

Reasons:

- confirms basic release CLI execution, init, validation, and transaction
  behavior before starting Zellij sessions;
- fails quickly on a broken native executable;
- leaves the expensive process-boundary fixture for the second step;
- does not mutate the Lisa checkout because it builds an external temporary
  repository.

Run the real-Zellij fixture second.

Reasons:

- it is the decisive embedding-boundary proof;
- it has the larger dependency and timing surface;
- its cleanup trap handles all successful scenario roots;
- any failure should be diagnosed with retained fixture evidence before Review.

## Observation model

For each stage record:

- exact command;
- wall-clock duration measured by the execution environment;
- exit status;
- stable receipt;
- concise behavior observations;
- pass or fail classification.

Classify build and fixtures independently:

- Build: PASS only if `just build-cli` exits zero and both output files exist.
- Atomic fixture: PASS only if it exits zero and emits its six-ticket receipt.
- Real-Zellij fixture: PASS only if it exits zero after all four named scenarios
  and emits `real-zellij-delivery-boundary: PASS`.

Do not convert a partial scenario success into an overall fixture pass.

Do not call a build pass a dogfood pass.

## Failure policy

If the build fails:

- record the failure and diagnostic stage;
- inspect whether the failure is environmental or source-owned;
- do not run fixtures against stale outputs.

If the atomic fixture fails:

- retain and inspect its printed external evidence path;
- record the failed assertion boundary;
- continue only if the failure is clearly fixture-environmental and the exact
  artifact remains usable; otherwise diagnose before proceeding.

If the real-Zellij fixture fails:

- rerun with `KEEP_LISA_ZELLIJ_FIXTURES=1` only when retained state is needed;
- inspect events, panes, signals, launch scripts, and loop output;
- record the exact failed scenario;
- do not invoke live providers as a fallback.

A source fix is not anticipated.

If one becomes necessary, document the deviation before editing, verify it,
and commit each exact source path through `lisa commit-ticket`.

## Artifact structure

Required phase artifacts remain:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

`progress.md` will be the durable observed-results record for implementation.

It will include an explicit fixture result table and reproduction commands.

`review.md` will summarize acceptance, coverage, and open concerns.

No extra raw-output artifact is required because both scripts expose concise,
stable receipts and the execution transcript is available during the attempt.

No file will be written under the shared publication work path.

## Commit strategy

The expected implementation changes no repository source.

Therefore, the expected number of ticket source commits is zero.

Phase artifacts are Lisa-managed completion inputs, not source units for a
manual ticket commit.

If no source changes occur, calling `lisa commit-ticket` would create no useful
meaningful unit and is unnecessary.

The final ownership check will explicitly confirm:

- no ticket-owned source path is staged;
- no ticket-owned source path is modified;
- no ticket-owned source path is untracked;
- pre-existing Lisa ticket and provenance changes remain untouched.

## Chosen design summary

1. Rebuild through `just build-cli`.
2. Fingerprint the release CLI and release WASM.
3. Run the atomic provider-contract script directly with that CLI.
4. Run the four-scenario real-Zellij script directly with that CLI.
5. Confirm artifact fingerprints remain stable.
6. Record exact observed results and boundaries in `progress.md`.
7. Run proportionate integrated checks only if needed to distinguish a fixture
   failure; do not broaden a passing evidence ticket into unrelated changes.
8. Review scope, cleanliness, coverage, and limitations.
