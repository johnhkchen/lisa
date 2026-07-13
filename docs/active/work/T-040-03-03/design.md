# Design: T-040-03-03

## Objective

Produce an identifiable release CLI and embedded release WASM from the completed
hostile-regression tree, then demonstrate that the complete deterministic native
and WASM gate is green.

The design output is verification evidence, not product behavior.
It must make the two historical regressions explicit rather than relying only on
an opaque aggregate test-suite success.
It must preserve unrelated shared-worktree state.

## Option 1: run only `just build-cli` and `just check`

This is the shortest repository-native approach.
`just build-cli` provides the correct build order and freshness touch.
`just check` performs WASM checking and workspace tests.

Advantages:

- follows root recipes;
- builds the plugin before the CLI;
- triggers the build-script rerun;
- exercises the ordinary contributor gate.

Limitations:

- does not run formatting;
- does not run native or WASM Clippy;
- does not explicitly show either hostile regression passing;
- hides the two named release build commands behind recipe expansion.

This option cannot satisfy the complete acceptance criterion by itself.

## Option 2: run only the literal minimum commands

This option runs the two named release builds, formatting, native Clippy, WASM
Clippy, and `cargo test --workspace`.

Advantages:

- maps directly to every main acceptance clause;
- gives each gate an independent exit status;
- avoids interpretation of recipe aliases.

Limitations:

- does not reproduce the `Justfile` touch unless added;
- aggregate workspace tests make the two historical proofs less visible;
- does not run the canonical `just check` combination.

This is sufficient in principle but weaker as a rebuild handoff.

## Option 3: clean the shared target directory first

This option runs `cargo clean` or removes selected release outputs before builds.

Advantages:

- forces artifact regeneration;
- prevents reuse of compiled intermediates;
- gives a literal from-empty-target interpretation of rebuild.

Limitations:

- all Lisa seats share the workspace and target directory;
- cleanup can disrupt concurrent compilation;
- it invalidates unrelated work and lengthens the critical path;
- the ticket does not require removal of valid Cargo intermediates;
- Cargo already validates source and dependency freshness.

The disruption exceeds the evidence gained.

## Option 4: use a private `CARGO_TARGET_DIR`

This would isolate compilation outputs for the ticket.

Advantages:

- avoids modifying the default target cache;
- enables a clean isolated build;
- makes artifact provenance straightforward.

Limitations:

- `crates/lisa-cli/build.rs` reads the hard-coded workspace `target/` path;
- an alternate CLI target still embeds from the default release WASM path;
- changing the build script would be out of scope;
- it would not test the repository's supported release path.

The current embedding implementation makes this option unsuitable.

## Option 5: ordered repository rebuild plus explicit full gates

This option uses the default target path and performs:

1. preflight revision and worktree capture;
2. exact release plugin build;
3. WASM existence, size, and SHA-256 capture;
4. `Justfile`-equivalent touch of the release WASM;
5. exact release CLI build;
6. fresh `OUT_DIR` copy discovery and identity comparison;
7. CLI size and SHA-256 capture;
8. formatting check;
9. both historical regressions as focused commands;
10. broad native warning-strict Clippy;
11. WASM warning-strict Clippy;
12. full native workspace tests;
13. canonical `just check`;
14. final repository-state inspection.

Advantages:

- tests the supported embedding route;
- names every acceptance gate;
- proves both regressions are selected and pass independently;
- records exact artifacts for the dependent field-report ticket;
- avoids source churn and target cleanup;
- supplies both literal commands and repository-native confirmation.

Limitations:

- Cargo can reuse valid intermediate outputs;
- touching proves invalidation but does not change plugin bytes;
- hashes prove the copy boundary, not live Zellij execution;
- workspace tests repeat once through `just check`.

These limitations are aligned with the ticket's deterministic boundary.

## Decision

Choose Option 5.

The deciding repository fact is that the CLI build script consumes only the
default release WASM path.
The deciding story fact is that the next ticket owns live execution.
The best evidence here is therefore an ordered default-path release build,
byte-identical build-script copy, and complete deterministic gate.

## Rebuild artifact design

The assignment refers to a singular rebuild artifact.
Create `rebuild.md` in the attempt-private work directory during Implement.
It will be a concise, stable evidence ledger containing:

- starting revision and dependency commits;
- exact ordered build commands and exit outcomes;
- release WASM identity;
- `OUT_DIR` copy identity and equality result;
- release CLI identity;
- focused predecessor regression commands and outcomes;
- format, native Clippy, WASM Clippy, workspace test, and canonical gate results;
- observed suite totals;
- retry or anomaly disclosure;
- final source cleanliness and commit disposition;
- a handoff block for `T-040-03-04`.

`progress.md` will track implementation execution and summarize the same results.
`rebuild.md` is the acceptance-focused artifact intended for downstream use.

## Embedding proof

First run:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

Require a non-empty `target/wasm32-wasip1/release/lisa.wasm`.
Capture its logical size and SHA-256.

Touch that exact file, matching `just build-cli`, and run:

`cargo build -p lisa-cli --release`.

Afterward locate build-script outputs matching:

`target/release/build/lisa-cli-*/out/lisa.wasm`.

Select the copy produced by the current release build using modification time and
verify its size and SHA-256 match the release source.
Record the selected exact path.

The successful CLI compile plus identical build-script copy establishes the
compile-time embedding boundary consumed by `include_bytes!`.
It does not claim runtime plugin instantiation.

## Regression proof

Run each newly added regression by its exact stable name:

- `cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done`;
- `cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable`.

Then run the full workspace suite.
Focused passes prove the tests exist and are selected.
The aggregate pass proves they coexist with all other current tests.

## Full gate

The exact explicit gate is:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `cargo test --workspace`;
- `just check`.

The final command intentionally repeats workspace tests while adding the ordinary
WASM check used by the repository default recipe.

## Failure policy

A deterministic source or behavior failure is recorded and blocks disposition.
This ticket does not silently patch predecessor contracts.
An environmental interruption may be retried only after its cause is identified,
and both attempts must be disclosed.

An unexplained mismatch between release WASM and `OUT_DIR` copy is blocking.
Failure of either named regression is blocking.
Failure of any warning-strict, format, test, check, or release build is blocking.

## Source and commit policy

The expected product-source change set is empty.
Generated target artifacts are ignored and not committed.
Attempt-private phase and evidence artifacts are published by Lisa, not through a
ticket-owned source transaction.

If no source changes occur, do not manufacture an empty `lisa commit-ticket` call.
If an unexpected necessary source change arises, document the plan deviation and
commit only exact owned paths through `lisa commit-ticket`.

## Completion boundary

Implementation completes when the ordered artifacts exist, identities match, all
explicit gates pass, both regressions are individually green, and evidence is
written.
Review then assigns pass or block through `review-disposition.json`.
The attempt stops on this ticket without beginning the live field report.
