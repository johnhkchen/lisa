# Structure: verification-only ticket

## Source tree changes

No production, test, manifest, lockfile, workflow, or recipe change is planned.

The predecessor-owned settled inputs remain:

- `crates/lisa-core/tests/recorded_livelock_regression.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`;
- `crates/lisa-core/Cargo.toml`;
- `Cargo.lock`.

These paths are read and compiled but not modified.

## Private attempt artifacts

The following files are created under the current attempt directory:

- `research.md` maps the settled code and constraints;
- `design.md` selects the direct verification barrier;
- `structure.md` defines the evidence and file boundaries;
- `plan.md` specifies ordered commands and pass criteria;
- `progress.md` records command execution and measurements;
- `review.md` provides the final reviewer handoff;
- `review-disposition.json` communicates pass or block to Lisa.

The private root is:

```text
.lisa/attempts/T-041-02-03/1/work/
```

No artifact is written directly to the shared publication directory.

## Generated build outputs

Cargo may create or refresh:

- `target/debug/` native test and lint outputs;
- `target/wasm32-wasip1/debug/` WASM lint outputs;
- `target/wasm32-wasip1/release/lisa.wasm`;
- incremental and dependency metadata beneath `target/`.

All are ignored generated outputs.

The only measured product is:

```text
target/wasm32-wasip1/release/lisa.wasm
```

## Verification components

### Input-presence check

Confirms both predecessor integration-test files exist.

Confirms their expected test entry points remain present.

Confirms both property crates remain in the core manifest's dev-dependency
section.

### Formatting gate

Command:

```text
cargo fmt --all -- --check
```

Pass boundary: exit code zero and no diff requested by rustfmt.

### Native lint gate

Command:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

Pass boundary: exit code zero with all native workspace and test targets clean.

### WASM lint gate

Command:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Pass boundary: exit code zero for the actual plugin target.

### Workspace test gate

Command:

```text
cargo test --workspace
```

Pass boundary: exit code zero, including both new completion integration suites.

### Release WASM gate

Command:

```text
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release
```

Pass boundary: exit code zero and a nonempty module at the stable path.

### Size gate

Observation:

```text
wc -c target/wasm32-wasip1/release/lisa.wasm
```

Pass boundary: byte count no greater than 1,414,183.

### Type and identity checks

`file` identifies the result as WebAssembly.

`shasum -a 256` records exact artifact identity for the evidence.

These checks support but do not replace compilation and size assertions.

## Dependency boundary

The production edge is:

```text
lisa-plugin -> lisa-core
```

The test-only edge is:

```text
lisa-core integration tests -> proptest
lisa-core integration tests -> proptest-state-machine
```

The property crates are not public interfaces.

They do not alter the `lisa-core` library API.

They do not become `lisa-plugin` runtime dependencies.

No module exports, feature flags, or target-specific dependency tables change.

## Evidence organization

`progress.md` will contain:

- starting repository state;
- settled-input checks;
- each exact verification command;
- exit outcome and relevant suite counts;
- measured byte length;
- budget delta;
- artifact hash and type;
- final repository-state check;
- deviations, if any.

`review.md` will contain:

- disposition;
- acceptance mapping;
- source-change inventory;
- verification table;
- dependency-exclusion assessment;
- test-coverage assessment;
- repository-preservation statement;
- open concerns and human review focus.

`review-disposition.json` will contain exactly one valid workflow shape.

## Commit boundary

There is no planned ticket-owned source unit.

Therefore there is no planned `lisa commit-ticket` invocation.

Attempt artifacts are handed to Lisa for admission and final publication.

Lisa-owned ticket and provenance changes are not agent commits.

Ordinary index state must remain empty throughout.

## Excluded structure

No new CI job is created.

No new `justfile` recipe is created.

No size constant is added to source control.

No new Rust module or test is created.

No dependency version is changed.

No completion reducer or reconciler code is changed.

No plugin adapter wiring is attempted.

No live provider or Zellij harness is run.

## Ordering boundary

Private phase artifacts precede implementation verification.

Cheap static checks precede expensive tests and release optimization.

Measurement occurs only after a successful release build.

Review begins only after all gates and repository preservation checks complete.

## Structure outcome

This ticket has an evidence-only architecture. The settled repository is the
input, standard Cargo commands are the verification components, the release
WASM is the measured output, and private RDSPI files are the only authored
artifacts.
