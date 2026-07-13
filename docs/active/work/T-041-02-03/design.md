# Design: settled-tree verification gate

## Decision

Execute a read-only, fail-fast verification barrier against the settled tree and
record the exact outcomes in `progress.md` and `review.md`.

The barrier will validate source presence and dependency placement, run the
required formatting, native lint, WASM lint, test, and release-build commands,
then measure the generated module against the documented 1,414,183-byte ceiling.

No repository source will be changed when the tree passes as-is.

## Option 1: direct one-time verification

This option runs the acceptance commands manually and records their results.

Advantages:

- exactly matches the ticket's barrier role;
- introduces no maintenance surface;
- preserves the story's no-production-change boundary;
- observes the complete settled tree;
- allows precise environment-specific artifact measurement.

Disadvantages:

- the numeric size assertion is evidence rather than a permanent CI step;
- future changes rely on future barrier tickets or reviewers repeating it.

This is the selected option because the ticket asks to prove this landing, not
to redesign repository CI or establish a new permanent threshold mechanism.

## Option 2: add a CI size-budget check

This option would modify `.github/workflows/ci.yml` to build release WASM and
fail above a numeric byte ceiling.

Advantages:

- future regressions would be automatically rejected;
- the budget would become machine-readable policy.

Disadvantages:

- it adds a repository change not requested by the acceptance criterion;
- it changes CI cost and policy;
- the story explicitly frames this ticket as a barrier on a settled base;
- no ticket text assigns ownership of permanent CI budget enforcement;
- platform and toolchain policy would need explicit maintainership decisions.

This option is rejected as scope expansion.

## Option 3: add a checked-in verification script

This option would create a shell script containing the full gate.

Advantages:

- locally reproducible with one command;
- could encode the ceiling and source-presence assertions.

Disadvantages:

- adds source solely to wrap existing standard commands;
- creates a maintenance obligation without an acceptance requirement;
- duplicates the `justfile` and CI command vocabulary;
- still requires a policy decision about where the budget constant belongs.

This option is rejected because the existing commands are already concise and
the RDSPI evidence gives this run a durable audit trail.

## Option 4: inspect dependency metadata only

This option would use Cargo metadata or lockfile inspection without building the
release module.

Advantages:

- fast;
- can demonstrate dev-dependency classification.

Disadvantages:

- does not prove the release target builds;
- does not produce a measurable WASM artifact;
- cannot satisfy the size-budget criterion;
- misses target-specific compiler and linker failures.

This option is insufficient on its own.

## Verification ordering

The selected ordering is:

1. inspect required test paths and manifest placement;
2. run formatting check;
3. run native workspace Clippy with all targets and warnings denied;
4. run plugin Clippy for `wasm32-wasip1` with warnings denied;
5. run all workspace tests;
6. build release plugin WASM with the checked-in lockfile;
7. measure and identify the artifact;
8. verify repository preservation.

Formatting and lint run before the longer tests and optimized release build so
cheap source-quality failures surface early.

## Native Clippy interpretation

The acceptance language says native Clippy is green.

`cargo clippy --workspace --all-targets -- -D warnings` is selected because it:

- covers all three workspace crates on the native host;
- includes integration-test targets;
- compiles the new proptest suite under Clippy;
- is stricter and more complete than linting library targets alone.

The WASM plugin is also linted separately because native target selection cannot
stand in for target-specific compilation.

## WASM Clippy interpretation

The command is:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

This matches the `justfile` and CI target boundary.

It validates the production plugin dependency graph on WASM.

It does not compile the core integration tests for WASM, which is desirable:
the property framework is host-side development tooling rather than plugin
runtime code.

## Test interpretation

`cargo test --workspace` is the authoritative test gate.

The output must show the two named integration suites executing:

- `recorded_livelock_regression`;
- `completion_state_machine`.

Focused reruns are unnecessary if the workspace output identifies both suites
and they pass. A focused rerun remains available for diagnosis if needed.

The existing ignored real-Zellij boundary test may remain ignored because it is
environment-gated and unrelated to this pure-domain story.

## Release build interpretation

The release command will include `--locked` for dependency reproducibility:

```text
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release
```

This is semantically stronger than the acceptance command without changing its
target or product.

Success proves Cargo can resolve and compile the production graph from the
checked-in lockfile.

## Size-budget interpretation

The current module byte length will be obtained with:

```text
wc -c target/wasm32-wasip1/release/lisa.wasm
```

The pass condition is:

```text
current_bytes <= 1,414,183
```

The evidence will also report the delta from the ceiling.

No source modification is allowed merely to massage the measurement if the
module exceeds the threshold; an excess is a blocking discovery.

## Dev-dependency exclusion evidence

Three observations jointly support exclusion:

- manifest placement under `[dev-dependencies]`;
- absence from `[dependencies]`;
- successful release plugin build within the pre-existing size ceiling.

Cargo package metadata may be recorded as supporting evidence, but inspecting
raw WASM strings is not a reliable linkage proof because names can be absent or
optimized away.

The dependency classification is the structural proof; the build and size are
the product proof.

## Failure handling

If formatting fails, no automatic formatter will be run because this ticket
does not own predecessor source.

If Clippy fails, the finding will be attributed to its exact path and the ticket
will block unless the issue is clearly generated/environmental.

If tests fail, the failing suite and seed/counterexample will be preserved.

If the WASM build fails, compiler output will be preserved.

If size exceeds budget, exact bytes and delta will be recorded.

Any such substantive source repair requires new scoped work rather than an
unplanned edit to the completed predecessor units.

## Repository preservation

No ordinary Git index operations will be used.

Unrelated Lisa-managed files and `crates/lisa-plugin/docs/` will remain untouched.

Ignored `target/` outputs may be refreshed by Cargo.

If no ticket-owned source changes are made, `lisa commit-ticket` is neither
necessary nor appropriate.

## Design outcome

The direct barrier provides the required evidence with the smallest truthful
change surface: private phase artifacts plus ignored build outputs only.
