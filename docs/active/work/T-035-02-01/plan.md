# Plan: implement and verify the real-Zellij regression

## Execution rules

All phase artifacts remain in `.lisa/attempts/T-035-02-01/1/work/`.

Do not edit the ticket frontmatter.

Do not use the ordinary Git index for parent repository ticket work.

Commit each meaningful test-source unit with `lisa commit-ticket`, ticket ID
`T-035-02-01`, and exact repository-relative include paths.

Preserve every unrelated dirty file already present in the shared worktree.

## Step 1: capture the pre-implementation repository baseline

Record `git status --short` and the current HEAD.

Confirm the two planned source paths do not already exist.

Confirm the attempt-private work directory contains Research, Design, Structure, and Plan.

Verification:

- no ticket-owned source path is staged;
- unrelated dirty paths are catalogued and not touched.

## Step 2: create the minimal Cargo wrapper

Add `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`.

Mark the test ignored with the required runtime dependencies in the reason.

Resolve the shell harness from `CARGO_MANIFEST_DIR`.

Pass the exact `CARGO_BIN_EXE_lisa` as `LISA_BIN`.

Capture stdout/stderr and require both successful exit and the stable PASS receipt.

Verification:

- `cargo test -p lisa-cli --test real_zellij_delivery_boundary --no-run` compiles;
- formatting passes for the new Rust file.

## Step 3: build harness foundations

Add `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` with strict mode.

Implement dependency checks, root/session registries, cleanup traps, explicit timeouts,
and failure diagnostics.

Implement portable macOS/GNU `script` invocation.

Implement explicit-session Zellij helpers and JSON pane discovery.

Verification:

- `bash -n` passes;
- a temporary named Zellij session can start and be killed without affecting other
  sessions;
- list-panes returns one plugin and two terminal panes for a fixture.

## Step 4: implement isolated fixture construction

Use `LISA_BIN init` to scaffold each scenario.

Write the one-ticket story/DAG and short-timeout `.lisa.toml`.

Initialize and commit the disposable Git repository.

Generate a local `bin/claude` and ensure `claude --version` succeeds through scenario PATH.

Canonicalize the fixture root before loop launch.

Verification:

- `LISA_BIN validate --path <fixture>` passes;
- the fixture's PATH resolves `claude` to its stub;
- the fixture baseline is clean before Zellij starts.

## Step 5: implement the normal stub lifecycle

Record launch pane/generation.

Atomically copy the scheduler lease to `.started`.

Read the two-line bounded assignment reference from the terminal.

Record complete chat input without consuming or evaluating the assignment file.

Wait for an external ack gate, then atomically publish valid `UserPromptSubmit` JSON.

Remain resident after acknowledgement.

Verification:

- event log records exactly one start and one complete chat in a direct success run;
- ack JSON is parseable and contains the exact received marker;
- no installed provider process is started.

## Step 6: implement dashboard and evidence gates

Discover the assigned terminal and plugin panes rather than assuming IDs.

Dump and normalize both screens.

Add waits for `ready-for-assignment`, `delivering`, `owned`, `delivery-failed`, and
`startup-failed`.

Add exact negative ownership matching.

Add event-count, pane-identity, generation-order, bounded-launch, and chat-content
assertions.

Verification:

- forced bad state strings fail with useful dumps;
- successful waits retain the matching screen evidence.

## Step 7: complete the success scenario

Start a fresh fixture under a named real Zellij session.

Hold ack closed.

Require ReadyForAssignment and explicitly reject Owned.

Wait for the bounded chat reference, require Delivering, and explicitly reject Owned.

Open ack gate and require Owned.

Assert one pane, one generation, one launch, one logical chat, one matching ack, and a
launcher without full assignment prose.

Verification:

- success scenario passes repeatedly;
- deleting start publication prevents the Ready gate;
- deleting ack publication prevents the Owned gate, demonstrating sensitivity.

## Step 8: implement suppressed-start behavior and scenario

For `suppress-start`, keep the stub foreground and interruptible without publishing
`.started` for either generation.

Wait for Lisa's one same-pane recovery and terminal startup failure.

Require exactly two launch events with identical pane ID and increasing generation.

Require zero chats and zero acks.

Reject Owned at all retained state dumps and at terminal failure.

Verification:

- scenario finishes inside its explicit wall-clock bound;
- no third launch appears after an additional stability interval;
- the spare terminal never receives the ticket.

## Step 9: implement suppressed-chat-ack behavior and scenario

Publish start normally and read both assignment deliveries.

Never create `.ack`.

Observe ReadyForAssignment and Delivering while non-Owned.

Wait for DeliveryFailed.

Require exactly one launch and two complete logical chat deliveries.

Verification:

- scenario terminates within the expected polls;
- no ack file/event and no Owned state exists;
- a third chat does not arrive during a short stability interval.

## Step 10: implement the real `dquote>` fault

On generation 1, record launch and schedule delayed pane input.

Exit the provider successfully so the parent zsh regains foreground ownership.

Inject an unmatched double quote plus Enter into that exact terminal.

Require the terminal dump to show `dquote>` before Lisa's deadline.

Do not publish generation-1 start evidence.

On generation 2, run the normal gated lifecycle.

Verification:

- the initial terminal really shows zsh `dquote>`;
- Lisa interrupts it and the generation-2 stub starts;
- both launch rows carry the same pane ID;
- generation increases exactly once;
- ReadyForAssignment and Delivering remain non-Owned;
- only the generation-2 ack gate produces Owned;
- no third launch or spare consumption occurs.

## Step 11: run the complete ignored integration test

Execute:

```text
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
```

The command builds the CLI with its embedded WASM and runs all four scenarios.

Repeat at least once after the first complete pass to catch timing flakiness if runtime is
reasonable.

Verification:

- exit zero;
- stable PASS receipt;
- no residual named sessions;
- no retained temp fixture unless requested;
- no real `claude` or `codex` process associated with the fixtures.

## Step 12: document deviations before changing scope

If Zellij output or platform `script` behavior differs from Research assumptions, update
`progress.md` with the observed behavior and rationale before adjusting the harness.

Do not change production scheduler code merely to simplify the test.

If a production defect is discovered, capture the failing evidence, apply the smallest
ticket-scoped fix only if required by acceptance, and add focused native coverage.

## Step 13: commit the meaningful source unit

Once the harness and wrapper pass together, run:

```text
lisa commit-ticket --ticket-id T-035-02-01 \
  --message "test(cli): cover real Zellij delivery boundary" \
  --include crates/lisa-cli/tests/real_zellij_delivery_boundary.rs \
  --include crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

Use a later exact-path `lisa commit-ticket` only for a genuinely separate corrective unit.

Verification:

- `git show --name-only` contains exactly the intended paths;
- neither path is staged, modified, or untracked afterward.

## Step 14: run regression verification

Run focused checks:

- shell syntax;
- ignored real-Zellij integration test;
- any directly related native two-stage/recovery tests if names remain stable.

Run broad checks:

- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `git diff --check` for ticket-owned paths and attempt artifacts.

If the environment lacks a required WASM or Zellij component, report that as an open
verification concern rather than substituting a weaker claim. The current environment is
known to have Zellij 0.44.3.

## Step 15: inspect the final repository state

Compare final `git status --short` to the baseline.

Confirm unrelated modifications remain present and untouched.

Confirm ticket-owned source paths are clean and absent from the ordinary index.

Confirm only attempt-private phase artifacts remain uncommitted for Lisa publication.

## Step 16: write `progress.md`

Record each completed step, test commands and results, source commit hash, timing or
platform deviations, and any acceptance limitation.

The progress artifact must distinguish test-source commits from Lisa's later completion
commit.

## Step 17: perform self-review

Review the committed diff and harness for:

- false-positive state matching;
- accidental use of a real provider;
- missing timeout bounds;
- session-name collisions;
- cleanup that could kill unrelated sessions;
- generation or pane assertions derived only from stub opinion;
- unescaped shell input and path handling;
- platform-specific `script` behavior;
- pre-fix sensitivity.

Correct defects through exact-path isolated commits and rerun proportional verification.

## Step 18: write `review.md` and stop

Summarize created/modified/deleted files, source commits, successful two-stage evidence,
all three fault results, test coverage, and open concerns.

Write the artifact to the attempt-private directory.

Do not edit ticket phase/status, publish work, mark Done, release the seat, or begin
another ticket.
