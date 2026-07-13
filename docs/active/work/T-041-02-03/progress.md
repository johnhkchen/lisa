# Progress: workspace and WASM green gate

## Status

Implementation is complete.

This was a verification-only barrier on the settled predecessor tree.

No production, test, manifest, lockfile, workflow, or recipe source was changed.

All required gates pass.

The deterministic regression and generated property suite both execute and pass.

The release WASM builds successfully.

The property frameworks add exactly zero bytes to the release WASM.

## Starting revision

```text
c4900413fece0ec94b7ff255df124aea74faff0e
c490041 Complete T-041-02-02
```

This is the completion commit for the last required predecessor.

The tree therefore includes both proof tickets before verification begins.

## Starting repository boundary

Initial status contained:

```text
 M .lisa/provenance.jsonl
 M docs/active/tickets/T-041-02-03.md
?? crates/lisa-plugin/docs/
```

The first two paths are Lisa-owned lifecycle state.

The ticket change is the automatic Ready-to-Research phase transition.

The untracked plugin docs path pre-existed this attempt and is unrelated.

The ordinary Git index was empty.

No item above was edited, staged, reverted, or committed by this ticket.

## Settled proof inputs

The deterministic source exists at:

```text
crates/lisa-core/tests/recorded_livelock_regression.rs
```

The generated source exists at:

```text
crates/lisa-core/tests/completion_state_machine.rs
```

`crates/lisa-core/Cargo.toml` declares:

```text
[dev-dependencies]
proptest = "1.10"
proptest-state-machine = "0.8"
tempfile = "3"
```

Neither property crate is a normal production dependency.

## Formatting gate

Command:

```text
cargo fmt --all -- --check
```

Result: PASS.

The command emitted no diagnostic and made no source change.

## Native Clippy gate

Command:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

Result: PASS.

Cargo compiled all native workspace target shapes, including the property-test
integration target and its development dependencies.

Warnings were denied.

## WASM Clippy gate

Command:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: PASS.

Cargo checked `lisa-core` and `lisa-plugin` for the production WASM target.

Warnings were denied.

## Workspace test gate

Command:

```text
cargo test --workspace
```

Result: PASS.

Visible major suite results include:

- `lisa-cli` unit suite: 279 passed;
- `lisa-core` unit suite: 191 passed;
- `lisa-plugin` unit suite: 341 passed;
- all enabled CLI integration suites passed;
- core doctests: zero failures;
- one existing real-Zellij boundary remains intentionally ignored.

No executed test failed.

## Deterministic focused confirmation

Command:

```text
cargo test -p lisa-core --test recorded_livelock_regression --quiet
```

Result:

```text
1 passed; 0 failed
```

This confirms the recorded T-009-01-01 trace is present and executable in the
settled workspace.

## Generated focused confirmation

Command:

```text
cargo test -p lisa-core --test completion_state_machine --quiet
```

Result:

```text
1 passed; 0 failed
```

The property runner completed its configured 256 generated cases.

No proptest failure persistence or counterexample was produced.

## Release WASM build

Command:

```text
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release
```

Result: PASS.

Cargo compiled `thiserror`, `lisa-core`, and `lisa-plugin` and finished the
optimized release profile successfully.

Artifact:

```text
target/wasm32-wasip1/release/lisa.wasm
```

The artifact exists and is nonempty.

## Current WASM identity

Commands:

```text
wc -c target/wasm32-wasip1/release/lisa.wasm
file target/wasm32-wasip1/release/lisa.wasm
shasum -a 256 target/wasm32-wasip1/release/lisa.wasm
```

Results:

```text
size:   1,425,425 bytes
type:   WebAssembly binary module, MVP
sha256: 13c5273bb7d80234f3dd68a9d5b9b704bcdcafbc97f4d9bb7e75d4d85cb1a285
```

## Budget interpretation correction

The original Research pass found the older T-038 baseline of 1,414,183 bytes
and initially treated it as a hard ceiling.

Deeper inspection found that this was a measurement, not a defined budget
constant or ceiling.

The later settled T-040-03-03 measurement is 1,425,313 bytes.

The originating E-041 signal requires preservation of small WASM and rejection
of dependencies that materially expand it without demonstrated value.

The plan and prior phase artifacts were corrected before Review to reflect that
repository reality.

Current movement from the latest settled measurement is:

```text
1,425,425 - 1,425,313 = 112 bytes
112 / 1,425,313 = 0.007858%
```

This is non-material.

It follows the E-041 production landing, which added the permitted `thiserror`
normal dependency and the pure completion module.

## Pre-proptest comparator

To isolate the dev-dependency effect, commit `2c67693` was exported into a
temporary directory and built with:

```text
CARGO_TARGET_DIR=<temporary>/target cargo build --locked \
  --manifest-path <temporary>/Cargo.toml \
  -p lisa-plugin --target wasm32-wasip1 --release
```

`2c67693` is immediately before commit `5c03e6e`, which adds the proptest
manifest entries, lockfile graph, and generated suite.

The only current-tree differences from `2c67693` in the relevant manifest,
lockfile, core source, and plugin source set are:

```text
Cargo.lock
crates/lisa-core/Cargo.toml
```

Production Rust source is identical across this comparison.

The pre-proptest build result was:

```text
size:   1,425,425 bytes
sha256: 13c5273bb7d80234f3dd68a9d5b9b704bcdcafbc97f4d9bb7e75d4d85cb1a285
```

It exactly matches the current product in both logical length and SHA-256.

Therefore the proptest and proptest-state-machine landing adds exactly zero
bytes to the release WASM artifact.

The temporary source and target directory were removed after measurement.

## Production dependency-tree check

Command shape:

```text
cargo tree -p lisa-plugin --target wasm32-wasip1 --edges normal
```

The normal production graph contains neither `proptest` nor
`proptest-state-machine`.

Result: PASS.

This structural evidence agrees with the exact product comparison.

## Final repository hygiene

`git diff --check` passed.

The ordinary index remains empty.

There is no ticket-owned staged, modified, or untracked source path.

Lisa has published admitted work artifacts under
`docs/active/work/T-041-02-03/`; the agent authored only private attempt copies.

The unrelated `crates/lisa-plugin/docs/` path remains untouched.

No ordinary `git add` or `git commit` was used.

No `lisa commit-ticket` source transaction was necessary because no source
change exists.

## Deviations

The only deviation was replacing an initially assumed numeric ceiling with the
actual documented materiality constraint and an exact production-equivalent
before/after comparison.

This strengthened the dev-dependency exclusion proof and avoided presenting an
old measurement as policy.

## Remaining work

Only Review artifacts remain.
