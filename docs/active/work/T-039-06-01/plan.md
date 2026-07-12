# Plan: T-039-06-01

## Step 1: establish the execution boundary

Record the current revision with `git rev-parse HEAD` and `git log -1`.
Inspect `git status --short` before running builds.
Confirm that only expected Lisa lifecycle files are already modified.
Do not alter or include those files.

Pass criteria:

- the revision contains completed dependency `T-039-05-03`;
- no unexpected ticket-owned source change is present;
- the attempt work directory is the only artifact-authoring destination.

## Step 2: build the release WASM plugin

Run the exact acceptance command:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`

Do not clean the shared target directory first.
Cargo may reuse valid dependency artifacts while still validating the current tree.
Capture the command's exit result and relevant Cargo summary in `progress.md`.

Pass criteria:

- command exits zero;
- `target/wasm32-wasip1/release/lisa.wasm` exists;
- the artifact is non-empty.

If it fails, inspect the direct diagnostic.
Do not patch a new structural defect within this closing ticket.
Record an unresolved source failure as blocking.

## Step 3: identify the WASM artifact

Run byte-count and SHA-256 commands against:

`target/wasm32-wasip1/release/lisa.wasm`.

Record:

- repository-relative path;
- byte count;
- SHA-256 digest.

These values identify the exact release plugin passed to the embedding step.
They are supporting evidence and have no threshold-based pass criterion beyond
the artifact being non-empty.

## Step 4: trigger embedding freshness

Run:

`touch target/wasm32-wasip1/release/lisa.wasm`.

This reproduces the `Justfile` `build-cli` recipe's invalidation step.
It ensures the CLI build script's `cargo:rerun-if-changed` input has a fresh mtime.
Do not modify the WASM bytes.

Pass criteria:

- touch exits zero;
- the artifact remains present and non-empty.

## Step 5: build the release CLI

Run the second exact acceptance command:

`cargo build -p lisa-cli --release`

The plugin output must already exist and have been touched.
The CLI build script will copy that file to its `OUT_DIR`.
`templates.rs` will include the copied bytes at compile time.

Pass criteria:

- command exits zero;
- `target/release/lisa` exists;
- the native artifact is non-empty;
- Cargo emits no build error from the copy or inclusion boundary.

## Step 6: identify the CLI artifact

Run byte-count and SHA-256 commands against `target/release/lisa`.
Record its logical size and digest in `progress.md`.

Also restate the build ordering alongside the two identities:

release plugin build → touch WASM → release CLI build.

Do not claim that a hash alone proves live Zellij loading.
The build script and `include_bytes!` supply compile-time embedding evidence.
Live execution belongs to the next ticket.

## Step 7: run the formatting gate

Run:

`cargo fmt --all -- --check`

Pass criteria:

- command exits zero;
- no source file is changed.

If formatting is unexpectedly red, record the affected paths.
Do not run mutating formatting without first establishing that the changes belong
to this ticket and documenting a plan deviation.

## Step 8: run native warning-strict Clippy

Run:

`cargo clippy --workspace --all-targets --all-features -- -D warnings`

This is the broad native lint gate.
It includes workspace library, binary, unit-test, and integration-test targets.

Pass criteria:

- command exits zero;
- no warning is emitted as an allowed residue;
- no source file is changed.

Record the final Cargo result and elapsed time when Cargo prints it.

## Step 9: run production-target WASM Clippy

Run:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

This is separate from native Clippy because the plugin's production compilation
target differs from the host test target.

Pass criteria:

- command exits zero;
- all warnings remain denied;
- no source file is changed.

## Step 10: run the native workspace test suite

Run:

`cargo test --workspace`

Record every test binary summary reported by Cargo.
Record aggregate observations without inventing a single total if Cargo reports
several suites separately.

Pass criteria:

- command exits zero;
- every executed unit test, integration test, and doctest passes;
- there are no ignored failures or panics hidden by a pipeline.

The command will be invoked directly rather than piped, preserving its exit status.

## Step 11: run the canonical repository gate

Run:

`just check`

This executes the ordinary production-target WASM check and workspace tests.
It intentionally repeats the native suite after the explicit acceptance test pass.

Pass criteria:

- the WASM `cargo check` exits zero;
- the repeated workspace tests exit zero;
- the overall Just recipe exits zero.

## Step 12: inspect final repository state

Run `git status --short` and `git diff --check`.
Inspect any reported path rather than assuming it belongs to this ticket.

Expected state:

- Lisa lifecycle mutations may remain in `.lisa/provenance.jsonl` and the ticket;
- private phase artifacts exist in the attempt directory;
- generated Cargo artifacts remain ignored;
- no production source path is modified, staged, or untracked by this ticket;
- the ordinary Git index has not been used.

If the expected no-source-change state holds, do not call `lisa commit-ticket`.
There is no meaningful source unit to commit.

If a ticket-owned source file changed unexpectedly, stop and classify it.
Only a necessary, in-scope unit may be committed, and only via exact-path
`lisa commit-ticket` after its deviation and verification are documented.

## Step 13: write implementation evidence

Create `progress.md` with:

- implementation status;
- revision;
- command-by-command results;
- output artifact identities;
- source-change and commit disposition;
- deviations and retries;
- acceptance mapping;
- remaining work.

The artifact must state failures honestly if any gate is red.
Do not mark acceptance green based on partial command coverage.

## Step 14: self-review

Review all phase artifacts and the final repository status.
Create `review.md` summarizing:

- what was generated;
- what source files were not changed;
- release build and embedded-WASM evidence;
- format, native Clippy, WASM Clippy, test, and canonical gate results;
- coverage boundaries;
- open concerns or anomalies;
- whether human attention is required.

After writing `review.md`, remain on `T-039-06-01`.
Do not edit ticket phase/status.
Do not publish the work directory manually.
Do not start `T-039-06-02`.
Lisa owns completion publication and seat release.
