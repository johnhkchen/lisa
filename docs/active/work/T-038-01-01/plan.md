# Plan — T-038-01-01 CLI and WASM size baseline

## Outcome target

Produce a documentation-only, same-environment reproducible baseline for the
release Lisa CLI binary and its embedded release WASM. The final artifacts must
state exact byte counts, the exact producing command, and evidence that an
immediate rerun yields the same counts.

## Step 1 — Confirm execution identity

Record immediately before measurement:

- `git rev-parse HEAD`;
- `git log -1 --oneline`;
- workspace package version from `Cargo.toml`;
- `rustc --version --verbose`;
- `cargo --version`;
- host/kernel identity from `uname -a`.

Verify that HEAD and toolchain are consistent with Research. If tracked source,
manifest, lockfile, or build definitions have changed concurrently, inspect the
change before proceeding and update the evidence identity.

No mutation occurs in this step.

## Step 2 — Run the first release build and size measurement

From the repository root, run exactly:

```bash
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release &&
touch target/wasm32-wasip1/release/lisa.wasm &&
cargo build --locked -p lisa-cli --release &&
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

Verification criteria:

- plugin build exits zero;
- the release WASM exists before the touch;
- CLI build exits zero after the touch;
- `wc -c` exits zero;
- CLI path-specific count is positive;
- WASM path-specific count is positive;
- raw stdout contains both explicit paths.

Capture the path-specific numbers and total exactly as printed.

## Step 3 — Run the identical command a second time

Without editing source, manifests, the lockfile, build scripts, or release
profiles, repeat the exact Step 2 command verbatim.

Verification criteria:

- the full command chain exits zero again;
- the CLI byte count exactly matches Run 1;
- the WASM byte count exactly matches Run 1;
- path spellings and units remain unchanged.

If either count differs, do not claim acceptance. Inspect concurrent source
changes, toolchain changes, Cargo build-script behavior, and artifact identity,
then document the discrepancy.

## Step 4 — Verify artifact identity

Run `file` on both measured paths.

Expected observations in the current environment:

- `target/release/lisa` is a Mach-O 64-bit arm64 executable;
- `target/wasm32-wasip1/release/lisa.wasm` is a WebAssembly module.

This does not add a new acceptance condition; it guards against recording a
directory, debug file, or wrong-target output under a plausible name.

Optionally inspect the build-script copy with byte comparison if its current
hashed `OUT_DIR` path can be identified unambiguously. This is supporting
evidence only; the source code already defines a byte-for-byte copy.

## Step 5 — Write `progress.md`

Create the implementation artifact under the attempt-private work directory.

Include:

- completion checklist;
- source/package/toolchain/host identity;
- exact multiline command in a shell code block;
- unedited Run 1 `wc -c` output;
- unedited Run 2 `wc -c` output;
- a comparison table for the two path-specific values;
- PASS/FAIL assessment of immediate repeatability;
- explanation that the WASM path is the byte-for-byte build-script input;
- statement that no source changes or source commit were required;
- any deviation from this plan;
- remaining work limited to Review.

Do not write this artifact to the shared `docs/active/work` path.

## Step 6 — Check repository integrity

Inspect Git state with read-only commands.

Confirm:

- no production/test/config/build file was changed by the ticket;
- no ticket-owned source file is staged;
- no ticket-owned source file is modified;
- no ticket-owned source file is untracked;
- only attempt-private artifacts and pre-existing Lisa orchestration changes
  account for relevant status.

Do not stage or commit the phase artifacts. Lisa's completion transaction owns
their publication.

Because this ticket has no ticket-owned source unit, do not invoke
`lisa commit-ticket`. The workflow's source-commit requirement is vacuously
satisfied when there is no source change.

## Step 7 — Write `review.md`

Create the final handoff under the attempt-private work directory.

Summarize:

- whether acceptance is met;
- the two baseline byte counts;
- the exact reproduction command;
- the immediate rerun comparison;
- files created and files intentionally untouched;
- release-build verification coverage;
- absence of conventional unit/integration test need for a docs-only measure;
- platform/toolchain caveat for native reproducibility;
- embedded-WASM definition and copy relationship;
- repository integrity and commit status;
- open concerns, TODOs, or limitations.

The review should make it unnecessary for a human to read every phase artifact
to identify the numbers and rerun them.

## Step 8 — Stop at Lisa's completion gate

After `review.md` exists:

- remain on T-038-01-01;
- do not edit ticket phase or status;
- do not publish to the shared work directory;
- do not start another ticket;
- allow Lisa to verify the lease and create the completion commit.

## Verification matrix

| Requirement | Evidence | Pass condition |
|---|---|---|
| Release CLI byte count | `wc -c target/release/lisa` line | Positive exact integer recorded |
| Embedded-WASM byte count | `wc -c target/wasm32-wasip1/release/lisa.wasm` line | Positive exact integer recorded |
| Exact build command | Shell block in Progress and Review | Includes plugin build, touch, CLI build, size command |
| Correct build order | Command chain + `build.rs` research | Plugin precedes CLI |
| Lockfile honored | Both Cargo calls use `--locked` | Builds exit zero |
| No stale measurement on failure | Commands joined by `&&` | `wc` runs only after success |
| Rerun stability | Run 1 vs Run 2 table | Both path-specific values equal |
| Artifact identity | `file` output | Native executable and WASM module |
| Product unchanged | Git status/diff inspection | No ticket-owned source delta |
| Handoff complete | `review.md` | Outcome, command, values, caveats present |

## Test strategy

No unit test is added because no callable code or behavior changes. No
integration test is added because the measurement directly builds the two
release products and observes their file lengths.

The proportionate verification is:

1. two successful locked release builds for both targets;
2. two exact byte measurements;
3. equality comparison;
4. artifact type inspection;
5. repository integrity inspection.

Workspace tests, formatting, and Clippy are deliberately not run as ticket
acceptance gates. Their results cannot validate a file-size observation, and
this ticket creates no source delta for them to assess.

## Atomicity and commits

The only meaningful implementation unit is the private measurement evidence.
The assignment reserves final artifact publication for Lisa. No ticket-owned
source file exists to pass to `lisa commit-ticket`, so there is no source commit
step and no exact `--include` set.

Build products under `target/` are ignored generated outputs and are never
included in a commit.

## Planned deviations policy

Any command spelling change, source revision change, missing target, build
failure, or differing second count will be recorded in `progress.md` before the
plan continues. Acceptance will not be marked complete unless both requested
path-specific numbers reproduce under the final recorded command.
