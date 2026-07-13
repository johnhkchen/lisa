# Progress: T-040-03-03

## Implementation status

Implementation is complete.
The release WASM plugin and release CLI were rebuilt from the completed
T-040-03-01/T-040-03-02 tree in plugin-first order.
The fresh build-script copy exactly matches the release WASM.
Both historical regressions pass individually and within the full plugin suite.
All required native and WASM gates are green.

No product source modification was needed or made.
No `lisa commit-ticket` source transaction was therefore required.
No ordinary Git index operation was used.

## Starting revision

Commands:

```text
git rev-parse HEAD
git log -5 --oneline
```

Result: PASS.

```text
48b9bf80ca59013e7e46f1010c4ac04623762890
48b9bf8 Complete T-040-03-02
1d8d0ad Pin pre-ownership CLI evidence regression
99562b1 Complete T-040-03-01
b6a574a Pin blocking Review completion regression
c14d41c Complete T-040-02-03
```

The revision contains the completion commits for both declared dependencies and
their two meaningful source commits.

## Starting worktree boundary

Before builds, `git status --short --untracked-files=all` reported:

```text
 M .lisa/provenance.jsonl
 M docs/active/tickets/T-040-03-03.md
?? crates/lisa-plugin/docs/active/work/.attempts/T-001/1/work/review-disposition.json
?? crates/lisa-plugin/docs/active/work/T-001/review-disposition.json
```

The first two paths are Lisa-managed lifecycle state.
The two plugin-relative paths are pre-existing test fixture residue.
They were present before implementation commands and are not owned by this ticket.
They were preserved without staging, deletion, or modification.

As phase artifacts were detected, Lisa published admitted copies under
`docs/active/work/T-040-03-03/`.
The authored artifacts were written only to the assigned attempt-private path.

## Regression source presence

Command:

```text
rg -n "fn (test_t039_06_02_blocking_review_never_prepares_done|rc6_preownership_delivery_miss_is_durable_and_cli_retrievable)" crates/lisa-plugin/src/lib.rs
```

Result: PASS.

```text
7165: fn test_t039_06_02_blocking_review_never_prepares_done()
15213: fn rc6_preownership_delivery_miss_is_durable_and_cli_retrievable()
```

Both predecessor regressions are present in the completed source tree.

## Release WASM build

Exact command:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Result: PASS.

Cargo compiled `lisa-core` and `lisa-plugin` and reported:

```text
Finished `release` profile [optimized] target(s) in 7.32s
```

The expected artifact exists and is non-empty:

```text
target/wasm32-wasip1/release/lisa.wasm
```

## Release WASM identity

Commands:

```text
wc -c target/wasm32-wasip1/release/lisa.wasm
shasum -a 256 target/wasm32-wasip1/release/lisa.wasm
```

Result:

```text
size:   1,425,313 bytes
sha256: 053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
```

This identifies the production plugin input passed to the embedding step.

## Embedding freshness

Command:

```text
touch target/wasm32-wasip1/release/lisa.wasm
```

Result: PASS.

This reproduces the root `Justfile` `build-cli` freshness action.
It updates the `cargo:rerun-if-changed` input without modifying bytes.

## Release CLI build

Exact command:

```text
cargo build -p lisa-cli --release
```

Result: PASS.

Cargo explicitly compiled `lisa-cli` after the touch and reported:

```text
Finished `release` profile [optimized] target(s) in 7.23s
```

The expected executable exists and is non-empty:

```text
target/release/lisa
```

## Release CLI identity

Commands:

```text
wc -c target/release/lisa
shasum -a 256 target/release/lisa
```

Result:

```text
size:   3,078,832 bytes
sha256: 498134e92f43ea5a3d834c5cb22afdf5d6ad180e2543ae543b4ae84588addfe9
```

This is the native RC executable produced from revision `48b9bf8`.

## Build-script copy proof

The newest release build-script output was:

```text
target/release/build/lisa-cli-d7eaaaa8ac31ae12/out/lisa.wasm
```

Commands:

```text
cmp target/wasm32-wasip1/release/lisa.wasm target/release/build/lisa-cli-d7eaaaa8ac31ae12/out/lisa.wasm
wc -c target/release/build/lisa-cli-d7eaaaa8ac31ae12/out/lisa.wasm
shasum -a 256 target/release/build/lisa-cli-d7eaaaa8ac31ae12/out/lisa.wasm
```

Result: PASS.

```text
size:   1,425,313 bytes
sha256: 053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
cmp:    identical
```

The `OUT_DIR` bytes exactly match the fresh release WASM source.
`crates/lisa-cli/src/templates.rs` consumes that output through `include_bytes!`.
This proves the compile-time copy/embedding boundary for the release CLI.

## Blocking-Review regression

Exact command:

```text
cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done
```

Result: PASS.

```text
1 passed; 0 failed; 0 ignored; 340 filtered out
```

The exact historical test exists and executes successfully.

## Pre-ownership regression

Exact command:

```text
cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

Result: PASS.

```text
1 passed; 0 failed; 0 ignored; 340 filtered out
```

The exact rc.6 producer-to-CLI-consumer test executes successfully.

## Formatting gate

Exact command:

```text
cargo fmt --all -- --check
```

Result: PASS.

The command exited zero with no diagnostic and made no source edit.

## Native Clippy gate

Exact command:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Result: PASS.

Cargo compiled the current CLI and finished the native workspace target surface
with warnings denied.

## WASM Clippy gate

Exact command:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: PASS.

The production plugin target finished with warnings denied.

## Native workspace test gate

Exact command:

```text
cargo test --workspace
```

Result: PASS.

Suite summaries:

```text
279 passed; 0 failed; 0 ignored
1 passed; 0 failed; 0 ignored
3 passed; 0 failed; 0 ignored
1 passed; 0 failed; 0 ignored
0 passed; 0 failed; 1 ignored
169 passed; 0 failed; 0 ignored
341 passed; 0 failed; 0 ignored
0 doctests; 0 failed
```

There were 794 executed passing tests and zero failures.
The single ignored target is the existing real-Zellij delivery boundary whose
declaration requires explicit live environment prerequisites.

The 341-test plugin suite output explicitly includes both new regressions by name.

## Canonical repository gate

Exact command:

```text
just check
```

Result: PASS.

The recipe successfully ran:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

The second workspace execution again reported the 341-test plugin suite with both
historical regressions passing.

## Final repository state

Commands:

```text
git status --short --untracked-files=all
git diff --cached --name-only
git diff --check
```

Result: PASS for ticket ownership and whitespace.

No path is staged.
No product source path is modified or untracked by this ticket.
Final status contains only:

- Lisa-managed provenance and active-ticket mutations;
- the preserved pre-existing plugin fixture residue;
- Lisa-published phase artifact copies.

Generated release and test products remain ignored under `target/`.

## Deviations and retries

No plan deviation occurred.
No command failed.
No command required a retry.
No unexplained warning, copy mismatch, test behavior change, or build anomaly was
observed.

One additional quiet workspace test invocation was used to obtain concise suite
summaries after the successful direct invocation; it also passed.

## Source transaction

Meaningful ticket-owned source units: zero.

`lisa commit-ticket` calls: zero, intentionally.
There is no source diff to commit, and phase/evidence publication belongs to Lisa.

## Remaining work

- Write the dedicated `rebuild.md` evidence artifact.
- Write Review artifacts with pass disposition.
- Stop on T-040-03-03 for Lisa completion handling.
