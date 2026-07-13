# Plan: execute the workspace and WASM barrier

## Step 1: establish starting state

Run read-only Git status checks.

Confirm the ordinary index is empty.

Classify Lisa-owned ticket and provenance modifications as orchestration state.

Record unrelated untracked paths and preserve them.

Pass condition: no pre-existing ticket-owned source delta is mistaken for this
ticket's work.

## Step 2: validate settled inputs

Confirm both integration-test files exist.

Search for their named test entry points.

Inspect `crates/lisa-core/Cargo.toml` around dependency sections.

Confirm `proptest` and `proptest-state-machine` are dev-dependencies only.

Pass condition: the deterministic regression, generated suite, and dev-only
dependency declaration are all present before the gate runs.

## Step 3: check formatting

Run:

```text
cargo fmt --all -- --check
```

Do not run the mutating formatter if the check fails.

Pass condition: exit code zero.

## Step 4: lint native targets

Run:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

This includes the predecessor integration tests and their dev dependencies.

Pass condition: exit code zero with warnings denied.

If it fails, capture exact diagnostics without modifying predecessor code.

## Step 5: lint the WASM target

Run:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Pass condition: exit code zero with warnings denied.

This demonstrates target-specific production compilation independently from
native linting.

## Step 6: run the workspace suite

Run:

```text
cargo test --workspace
```

Capture overall outcome and relevant per-suite counts.

Explicitly verify output includes:

- `completion_state_machine` with one passing property test;
- `recorded_livelock_regression` with one passing regression test.

Record existing ignored tests without treating expected environment gating as a
failure.

Pass condition: Cargo exits zero and every executed test passes.

## Step 7: build release WASM

Run:

```text
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release
```

Use `--locked` to prevent resolution drift.

Pass condition: Cargo exits zero and the stable WASM file exists and is nonempty.

## Step 8: enforce the byte ceiling

Measure:

```text
wc -c target/wasm32-wasip1/release/lisa.wasm
```

Compare the integer to 1,414,183 bytes.

Compute:

```text
headroom = 1,414,183 - current_bytes
```

Pass condition: headroom is zero or positive.

Record the exact current bytes and signed delta.

## Step 9: identify the output

Run:

```text
file target/wasm32-wasip1/release/lisa.wasm
shasum -a 256 target/wasm32-wasip1/release/lisa.wasm
```

Pass condition: `file` reports WebAssembly and the hash command succeeds.

The hash is evidence of what was measured, not a fixed acceptance value.

## Step 10: verify dependency exclusion

Combine the manifest classification with the successful product gate.

Optionally inspect Cargo metadata if clarification is needed.

Do not infer release linkage merely from lockfile membership.

Pass condition: property frameworks remain only under core dev-dependencies and
the plugin release product remains within budget.

## Step 11: record progress

Create `progress.md` after command execution.

Include:

- completed steps;
- exact commands;
- pass/fail results;
- test counts visible in Cargo output;
- current WASM size, budget, and headroom;
- output identity;
- any deviations;
- remaining work.

No source commit is made if the settled tree passes without edits.

## Step 12: inspect final repository state

Run read-only status and index checks.

Confirm no ticket-owned source path is staged, modified, or untracked.

Confirm unrelated files remain untouched.

Pass condition: only Lisa orchestration state, unrelated pre-existing paths, and
private attempt artifacts differ from source history.

## Step 13: review

Write `review.md` summarizing:

- the evidence-only nature of the work;
- acceptance satisfaction;
- test and lint coverage;
- WASM measurement;
- dev-dependency exclusion;
- absence of source changes and commits;
- open concerns.

Write `review-disposition.json` with the strict pass shape only if every gate is
green. Otherwise use the strict block shape with an actionable reason.

## Atomicity and commit plan

There are no planned source commits.

If an unexpected ticket-owned source edit becomes necessary, stop and document
the deviation before editing.

Any authorized meaningful source unit would require one exact-path
`lisa commit-ticket` transaction.

Ordinary `git add`, `git commit`, and broad staging remain prohibited.

## Completion criteria

The plan is complete when:

- formatting passes;
- native all-target Clippy passes;
- WASM plugin Clippy passes;
- the full workspace suite passes with both predecessor suites executed;
- the locked release WASM build passes;
- its byte length is at or below 1,414,183;
- property frameworks remain dev-only;
- repository source state is preserved;
- progress and both Review artifacts are written privately.
