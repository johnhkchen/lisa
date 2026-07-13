# Review: workspace and WASM green gate

## Disposition

Pass.

T-041-02-03 satisfies its acceptance criterion on the completed predecessor
tree.

Formatting, native all-target Clippy, WASM-target Clippy, workspace tests, and
the locked release WASM build are green.

Both requested completion proof suites are present and pass.

The property frameworks remain dev-only and contribute exactly zero bytes to
the release plugin.

No blocking concern remains.

## What changed

No repository source file was created, modified, or deleted by this ticket.

No change was made to:

- `Cargo.toml`;
- `Cargo.lock`;
- `crates/lisa-core/Cargo.toml`;
- either predecessor integration test;
- `crates/lisa-core/src/`;
- `crates/lisa-plugin/`;
- `crates/lisa-cli/`;
- `.github/workflows/`;
- `justfile`.

This is intentional because the ticket is the closing verification barrier for
already completed proof work.

The authored files are private RDSPI artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

Cargo refreshed ignored outputs under `target/`.

## Acceptance mapping

### Workspace tests pass

Satisfied by:

```text
cargo test --workspace
```

The command exited zero with no executed failure.

Major unit suites reported 279 CLI, 191 core, and 341 plugin tests passing.

All enabled integration suites passed.

The existing real-Zellij integration boundary remained intentionally ignored by
its declared environment contract.

### Deterministic regression is present

Satisfied by the workspace run and focused command:

```text
cargo test -p lisa-core --test recorded_livelock_regression --quiet
```

Result: one passed, zero failed.

The test continues to replay the recorded artifact-before-phase, stop, timeout,
reload, and confirming-result order.

### Generated suite is present

Satisfied by the workspace run and focused command:

```text
cargo test -p lisa-core --test completion_state_machine --quiet
```

Result: one property test passed, zero failed.

The runner retains its configured 256 cases over one-to-63-event sequences.

### Formatting is green

Satisfied by:

```text
cargo fmt --all -- --check
```

Result: PASS.

### Native Clippy is green

Satisfied by:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

Result: PASS.

This includes native integration-test targets and therefore lints the new proof
suites and their development-only tooling.

### WASM Clippy is green

Satisfied by:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: PASS.

### Release WASM builds

Satisfied by:

```text
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release
```

Result: PASS.

The artifact is a valid WebAssembly MVP binary module.

## WASM size review

Current release artifact:

```text
path:   target/wasm32-wasip1/release/lisa.wasm
size:   1,425,425 bytes
sha256: 13c5273bb7d80234f3dd68a9d5b9b704bcdcafbc97f4d9bb7e75d4d85cb1a285
```

The repository does not define a hard numeric byte ceiling.

Its E-041 source signal requires small WASM and rejects a dependency that
materially expands it without demonstrated value.

The latest settled pre-E-041 measurement, from T-040-03-03, was 1,425,313 bytes.

The current delta is 112 bytes, or 0.007858%.

That movement is non-material and follows the production E-041 landing, which
added the explicitly permitted `thiserror` dependency and pure completion
domain.

## Dev-dependency exclusion proof

Manifest inspection places both property crates under `[dev-dependencies]` only.

The plugin's normal `wasm32-wasip1` Cargo dependency tree contains neither
`proptest` nor `proptest-state-machine`.

Most decisively, commit `2c67693` was built in a temporary isolated directory.

That revision is immediately before the property dependency commit and contains
the exact same production source and normal dependency declarations.

Its release result was:

```text
size:   1,425,425 bytes
sha256: 13c5273bb7d80234f3dd68a9d5b9b704bcdcafbc97f4d9bb7e75d4d85cb1a285
```

The current and pre-proptest artifacts match exactly.

The development tooling therefore contributes zero release bytes.

This is stronger evidence than comparing against a historical build produced
before E-041's intentional production dependency change.

## Test coverage assessment

Coverage is complete for this barrier's claims.

The deterministic suite proves the fixed field-derived ordering.

The property suite exercises generated orderings and its four invariants.

The full workspace run proves the additions coexist with all core, CLI, and
plugin behavior.

Native all-target Clippy compiles the new test targets with warnings denied.

WASM Clippy and the optimized release build separately validate the deployed
target graph.

The exact pre/post product identity isolates the dev-dependency effect.

No new test was appropriate because this ticket changes no behavior.

## Repository and commit review

No ticket-owned source change exists.

No `lisa commit-ticket` transaction was required.

No ordinary Git staging or commit command was used.

The ordinary index is empty.

Lisa-owned provenance, ticket phase, and admitted artifact publication state
remain outside any agent source transaction.

The unrelated pre-existing `crates/lisa-plugin/docs/` path was preserved.

## Open concerns and limitations

No blocking concern was found.

WASM byte identity depends on source, lockfile, compiler, target, and release
profile. The exact comparison was deliberately performed in the same environment
and toolchain to isolate only the proptest landing.

The historical 1,414,183-byte and 1,425,313-byte values are measurements, not
checked-in policy constants. If the project wants a hard permanent ceiling,
that belongs in separately scoped CI policy work.

This ticket does not exercise a live provider or real Zellij runtime. The story
is explicitly a pure-domain proof slice; adapter and live Arcade-shaped behavior
belong to E-042.

The one ignored real-Zellij integration test is therefore not a coverage gap for
this acceptance criterion.

## Critical issues requiring human attention

None.

## Human review focus

A reviewer should confirm the exact pre-proptest artifact match is the correct
proof that development-only property tooling stays out of release WASM, and that
the 112-byte, 0.007858% movement from the latest prior settled measurement is a
non-material consequence of E-041's permitted production landing.

Review is complete. This attempt remains on T-041-02-03 for Lisa to admit the
Review, prepare the completion commit, publish Done, and release the seat.
