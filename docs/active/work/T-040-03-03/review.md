# Review: T-040-03-03

## Disposition

PASS.

T-040-03-03 satisfies its acceptance criterion and is ready for Lisa's completion
transaction.

The RC release WASM and CLI were rebuilt from the completed predecessor tree.
The `build.rs` output exactly matches the release WASM input.
Both historical regressions pass individually and in the complete suite.
Every required format, lint, check, test, and release build is green.

No unexplained anomaly or open blocking concern remains.

## Source revision reviewed

```text
48b9bf80ca59013e7e46f1010c4ac04623762890
Complete T-040-03-02
```

The revision includes:

- `b6a574a`, the blocking-Review regression;
- `99562b1`, completion of T-040-03-01;
- `1d8d0ad`, the pre-ownership CLI evidence regression;
- `48b9bf8`, completion of T-040-03-02.

The rebuild therefore consumed the finished dependency tree required by this
barrier ticket.

## What changed

No product source file was created, modified, or deleted.

No change was made to:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-cli/src/preownership_status.rs`;
- `crates/lisa-cli/build.rs`;
- `crates/lisa-cli/src/templates.rs`;
- `Cargo.toml`;
- `Cargo.lock`;
- `Justfile`.

This is intentional.
The ticket verifies the settled source produced by its predecessors rather than
introducing another behavior change.

The authored attempt-private files are:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `rebuild.md`;
- `review.md`;
- `review-disposition.json`.

Lisa publishes admitted copies after checking the active attempt lease.
The agent did not author phase artifacts directly in the shared work directory.

Cargo refreshed ignored generated outputs under `target/`.
Those binaries and intermediates are not repository source changes.

## Release WASM review

Command:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Status: PASS.

Artifact identity:

```text
path:   target/wasm32-wasip1/release/lisa.wasm
size:   1,425,313 bytes
sha256: 053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
```

The file exists and is non-empty.
The release profile finished successfully for the production target.

## Embedding freshness review

The release WASM was touched after the successful plugin build.
This is the same invalidation step used by the root `build-cli` recipe.
It causes Cargo to reconsider the path emitted through `cargo:rerun-if-changed`.
The touch did not alter the WASM bytes.

## Release CLI review

Command:

```text
cargo build -p lisa-cli --release
```

Status: PASS.

Artifact identity:

```text
path:   target/release/lisa
size:   3,078,832 bytes
sha256: 498134e92f43ea5a3d834c5cb22afdf5d6ad180e2543ae543b4ae84588addfe9
```

Cargo explicitly compiled `lisa-cli` after the WASM freshness step.
The resulting native executable is non-empty.

## Embedded-WASM equality review

The fresh build-script output is:

```text
target/release/build/lisa-cli-d7eaaaa8ac31ae12/out/lisa.wasm
```

Its identity is:

```text
size:   1,425,313 bytes
sha256: 053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
```

`cmp` reports exact equality with the release WASM.
The input and output sizes and SHA-256 digests also match.

`crates/lisa-cli/build.rs` owns this copy boundary.
`crates/lisa-cli/src/templates.rs` consumes the copied file through
`include_bytes!`.
The ordered successful compile and exact copy match establish that the release
CLI was built with the fresh plugin bytes.

## Blocking-Review regression review

Exact focused command:

```text
cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done
```

Status: PASS, one selected test.

The test remains a historical discriminator because it asserts that a blocking
Review never enters `pending_completions`, while retaining assignment, lease, and
dependent blocking with no Done provenance.

The pre-S-040-01 unconditional path would have violated that assertion.

## Pre-ownership regression review

Exact focused command:

```text
cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

Status: PASS, one selected test.

The test remains a historical discriminator because it drives the production
delivery timeout and requires a physical durable failure row retrievable through
the CLI status implementation.

The pre-S-040-02 scheduler would have left no row to query.

## Formatting review

Command:

```text
cargo fmt --all -- --check
```

Status: PASS.

No format diagnostic was emitted and no source file changed.

## Native Clippy review

Command:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Status: PASS.

This covers the broad native workspace target surface, including test and
integration targets, with warnings denied.

## WASM Clippy review

Command:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Status: PASS.

This independently covers the deployed target-specific plugin compilation with
warnings denied.

## Workspace test review

Command:

```text
cargo test --workspace
```

Status: PASS.

Observed suite summaries:

- 279 CLI unit tests passed;
- enabled integration groups passed 1, 3, and 1 tests;
- 169 core unit tests passed;
- 341 plugin unit tests passed;
- zero doctests failed;
- one existing real-Zellij integration test remained intentionally ignored.

Total executed: 794 passed, zero failed.

The plugin output explicitly lists both newly added historical regressions as
passing in the same 341-test suite.
Neither regression is ignored or feature-gated out of the native gate.

## Canonical gate review

Command:

```text
just check
```

Status: PASS.

The recipe successfully ran production-target WASM `cargo check` and repeated the
full workspace suite.
This confirms the repository's ordinary combined developer gate in addition to
the stricter explicit Clippy and release-build commands.

## Acceptance criterion mapping

### Release plugin build succeeds

Satisfied by the exact named `cargo build -p lisa-plugin --target wasm32-wasip1
--release` command and non-empty artifact.

### Release CLI build succeeds

Satisfied by the exact named `cargo build -p lisa-cli --release` command after
the repository-defined freshness touch.

### Fresh WASM is embedded through `build.rs`

Satisfied by plugin-first ordering, a successful post-touch CLI compilation, and
exact equality between the release WASM and fresh `OUT_DIR` copy.

### Native and WASM format/Clippy/test gate is green

Satisfied by the explicit formatting, native Clippy, WASM Clippy, workspace test,
and canonical check passes.

### Both new regressions are included

Satisfied by exact focused executions and their named presence in the passing
341-test plugin suite.

### Results are recorded

Satisfied by `progress.md`, the dedicated `rebuild.md`, and this Review handoff.

## Coverage assessment

Coverage is strong for a deterministic rebuild ticket.
The release build covers optimized production WASM compilation.
The native release build covers the CLI build script and final linking.
The copy equality check proves the build-script bytes supplied to compile-time
embedding are exactly the freshly built plugin bytes.

Focused tests prove both incident-specific regressions exist and execute.
The complete native suite proves those tests coexist with the entire current
workspace behavior surface.
Native and WASM Clippy cover distinct compilation targets with warnings denied.
The canonical gate repeats the native suite after an ordinary WASM check.

No new test was appropriate because no interface or behavior changed in this
ticket.
Creating test-only churn would weaken the purpose of a settled-tree barrier.

## Honest coverage boundaries

This ticket does not instantiate the embedded WASM under real Zellij.
The existing real-Zellij test remains ignored under its explicit environmental
contract.

This ticket does not launch a live or metered provider seat.
It therefore does not claim provider ownership, live scheduler progress, or field
behavior from the rebuilt executable.

Those observations belong to dependent ticket `T-040-03-04`, which must use the
artifact identities recorded in `rebuild.md` and block on anomalies.

## Ownership and commit review

No ticket-owned product source path changed.
No ordinary index staging or commit was performed.
No `lisa commit-ticket` invocation was needed because there was no meaningful
source unit.

Lisa-managed provenance, ticket frontmatter, published artifacts, and pre-existing
fixture residue were preserved outside ticket ownership.
The ordinary Git index remains empty.

## Open concerns

None for this ticket.

There was no command failure, retry, warning, hash mismatch, ignored hostile
regression, or unexplained behavior change.

Lisa still owns final artifact admission, Done preparation, completion commit,
and seat release.
This attempt remains on T-040-03-03 and does not begin the field-report ticket.
