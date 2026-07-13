# Plan: T-040-03-03

## Step 1: capture the settled-tree boundary

Record `git rev-parse HEAD`, `git log -1`, and concise recent history.
Run `git status --short --untracked-files=all` and inspect the ordinary index.

Pass criteria:

- `HEAD` includes completion of both declared dependencies;
- neither predecessor source path has a pending diff;
- only known Lisa lifecycle and fixture residue is present;
- no ticket-owned source is staged.

Do not clean or alter unrelated paths.

## Step 2: confirm both regression names in source

Use `rg` to locate:

- `test_t039_06_02_blocking_review_never_prepares_done`;
- `rc6_preownership_delivery_miss_is_durable_and_cli_retrievable`.

Pass criteria:

- both exact names exist in `crates/lisa-plugin/src/lib.rs`;
- the source revision is the completed predecessor tree.

This is presence evidence only; execution follows later.

## Step 3: build the release WASM plugin

Run exactly:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

Do not clean the shared target directory.

Pass criteria:

- exit status is zero;
- `target/wasm32-wasip1/release/lisa.wasm` exists;
- the file is non-empty.

An unresolved build diagnostic blocks completion.

## Step 4: capture release WASM identity

Run byte-count and SHA-256 commands against the release WASM.
Record the repository-relative path, logical byte count, and digest.

Pass criteria:

- size is greater than zero;
- both commands succeed.

No size budget is introduced by this ticket.

## Step 5: trigger embedding freshness

Run:

`touch target/wasm32-wasip1/release/lisa.wasm`.

This mirrors `just build-cli` and activates `cargo:rerun-if-changed`.
Verify the file remains non-empty and its digest remains unchanged.

## Step 6: build the release CLI

Run exactly:

`cargo build -p lisa-cli --release`.

Pass criteria:

- exit status is zero;
- Cargo runs the current `lisa-cli` release build after the touch;
- `target/release/lisa` exists and is non-empty;
- no build-script copy or `include_bytes!` error occurs.

## Step 7: prove the build-script copy

Find release files matching:

`target/release/build/lisa-cli-*/out/lisa.wasm`.

Select the most recently modified current release copy.
Capture its exact path, size, and SHA-256.
Compare it with `target/wasm32-wasip1/release/lisa.wasm` using `cmp`.

Pass criteria:

- a current non-empty copy exists;
- `cmp` exits zero;
- size and digest exactly match the release WASM.

A mismatch is blocking because the field report needs the built-in plugin.

## Step 8: capture release CLI identity

Record the byte count and SHA-256 for `target/release/lisa`.
Record workspace version `0.4.0-rc.7` and the starting Git revision beside it.

These values identify the exact native executable for downstream field evidence.

## Step 9: run the blocking-Review regression

Run exactly:

`cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done`.

Pass criteria:

- one matching test executes;
- it passes;
- no other failure occurs.

## Step 10: run the rc.6 pre-ownership regression

Run exactly:

`cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable`.

Pass criteria:

- one matching test executes;
- it passes;
- no other failure occurs.

## Step 11: run formatting

Run:

`cargo fmt --all -- --check`.

Pass criteria:

- exit status is zero;
- no source changes are produced.

Do not run mutating formatting if the check is red without first classifying the
affected files and documenting a plan deviation.

## Step 12: run native warning-strict Clippy

Run:

`cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Pass criteria:

- exit status is zero;
- every warning remains denied;
- native library, binary, unit-test, and integration-test targets compile.

## Step 13: run WASM warning-strict Clippy

Run:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

Pass criteria:

- exit status is zero;
- warnings are denied;
- production-target plugin code compiles.

## Step 14: run the native workspace suite

Run directly and without a status-masking pipeline:

`cargo test --workspace`.

Record all suite summaries, especially CLI, core, and plugin unit totals.
Record ignored tests separately from executed tests.

Pass criteria:

- every executed test and doctest passes;
- both focused regressions are included in the plugin suite;
- no new ignored result conceals either regression.

## Step 15: run the canonical repository gate

Run:

`just check`.

This executes production-target WASM `cargo check` followed by another complete
workspace test run.

Pass criteria:

- both recipe commands exit zero;
- the Just invocation exits zero.

## Step 16: inspect final repository state

Run:

- `git status --short --untracked-files=all`;
- `git diff --cached --name-only`;
- `git diff --check`;
- scoped diffs for any unexpected source path.

Pass criteria:

- no product source changed due to this ticket;
- no ticket-owned path is staged, modified, or untracked;
- known unrelated lifecycle and fixture paths are preserved;
- generated `target/` artifacts remain ignored.

If the expected empty source diff holds, do not call `lisa commit-ticket`.

## Step 17: write implementation artifacts

Create `progress.md` with actual step status and outcomes.
Create `rebuild.md` as the acceptance evidence ledger and field-report handoff.

Both must disclose:

- exact commands;
- pass/fail status;
- artifact identities;
- copy equality;
- named regressions;
- suite totals;
- ignored test behavior;
- retries or anomalies;
- no-source transaction conclusion.

## Step 18: review and disposition

Write `review.md` with acceptance mapping, coverage assessment, limitations, and
open concerns.
Write `review-disposition.json` with the exact valid workflow shape.

Use pass only if every required build and gate is green, both copies match, both
regressions pass, and no unexplained anomaly remains.
Otherwise use block with a non-empty actionable reason.

After both Review artifacts are written, remain on `T-040-03-03`.
Do not edit ticket phase/status, publish manually, or start `T-040-03-04`.
