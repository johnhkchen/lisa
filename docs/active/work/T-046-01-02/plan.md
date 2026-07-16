# Plan — T-046-01-02 doctor and loop floor enforcement

## Goal

Enforce the shared Zellij 0.43.0 runtime floor in doctor and real loop
preflight, with explicit detected-versus-supported diagnostics and a prebuilt
static-binary remedy.

Every implementation step preserves unrelated worktree and ordinary-index
state.

## Step 1 — add the unsupported dependency state

Modify `crates/lisa-cli/src/doctor.rs`.

Import the shared Zellij classifier, verdict enum, and supported range.

Declare the private static-binary remedy constant.

Extend `CheckResult` with a structured `Unsupported` variant.

Add its `CheckReport` display arm.

Verification:

- the enum remains exhaustive at every match site;
- unsupported output visibly says `unsupported`;
- description and remedy render on indented lines;
- existing result variants retain their output.

## Step 2 — classify complete Zellij version output

Add the pure `check_zellij_version_output` adapter.

Translate `InRange` into a `Found` value containing detected version and shared
range.

Translate `BelowFloor` into `Unsupported` containing the canonical detected
version, shared range, and remedy.

Translate `Unparseable` into a separately named `Unsupported` description that
debug-quotes the trimmed raw output.

Update `check_zellij` to execute `zellij --version` directly.

Feed the complete stdout payload to the pure adapter on successful status.

Use `NotFound` with the static-binary remedy on spawn or status failure.

Leave opaque first-line checks for Claude and Codex unchanged.

Verification:

- no numeric Zellij floor literal is introduced in the CLI;
- all policy decisions flow through `lisa_core::version`;
- multiline or empty successful stdout is not silently accepted;
- command failure remains an absence-class failure.

## Step 3 — make unsupported checks fatal

Update `has_failures` to recognize required `Unsupported` results.

Generalize `format_report`'s failure summary.

Generalize `run_doctor`'s returned error.

Update `check_required_deps_inner` to collect fully rendered required failures
for both `NotFound` and `Unsupported`.

Update the function documentation to describe the new error payload.

Verification:

- doctor returns an error for below-floor Zellij output;
- doctor returns an error for unparseable Zellij output;
- optional skipped checks remain non-fatal;
- the loop-facing vector retains detected version, shared range, and remedy.

## Step 4 — add doctor policy tests

Add a `mock_unsupported` closure helper.

Add pure Zellij adapter tests for:

1. `zellij 0.43.7` passing;
2. `zellij 0.44.2` passing;
3. `zellij 0.40.1` failing below the floor;
4. `zellij not-a-version` failing as unparseable.

Use shared-range display in assertions rather than a copied floor constant.

Assert the pass cases contain detected version, supported range, and `OK` when
rendered as a report.

Assert the below-floor case contains 0.40.1, the shared range, unsupported
state, static-binary wording, and releases URL.

Assert the unparseable case contains the bad output and the phrase
`unparseable Zellij version output`, plus the same range and remedy.

Add generic report and failure-predicate coverage for unsupported results.

Update existing required-dependency tests to inspect rendered errors rather
than bare dependency names.

Add aggregation coverage proving an unsupported report keeps its detail and
remedy.

Verification command:

```text
cargo test -p lisa-cli doctor::tests
```

Expected result: all doctor inline unit tests pass.

## Step 5 — format and inspect the doctor source unit

Run Rust formatting for the workspace.

Inspect the source diff for `doctor.rs` only.

Confirm no unrelated path was reformatted or modified.

If formatting touches an unrelated source file, restore only the formatter's
unrelated edit with a narrow patch; do not use destructive Git commands.

Run the focused doctor tests again after formatting if the file changed.

Verify ordinary index and worktree ownership with:

```text
git status --short
git diff -- crates/lisa-cli/src/doctor.rs
git diff --cached --name-only
```

## Step 6 — commit the doctor source unit

Use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-046-01-02 \
  --message "Enforce Zellij support in doctor checks" \
  --include crates/lisa-cli/src/doctor.rs
```

Do not include attempt artifacts or unrelated paths.

Verification:

- the command returns a commit ID;
- `doctor.rs` is clean after the commit;
- unrelated modified and untracked files remain as they were;
- no ticket-owned source path is staged in the ordinary index.

## Step 7 — preserve details in loop refusal

Modify `crates/lisa-cli/src/loop_cmd.rs`.

Add the pure dependency-preflight error formatter.

Update `run_loop` to pass the rendered failure vector through it.

Use `failures` terminology instead of `missing`.

Keep the dependency-check position and all surrounding launch order unchanged.

Verification:

- the error heading says dependency preflight failed;
- the rendered report string is included verbatim;
- the footer still points to `lisa doctor`;
- no version classifier or floor literal is duplicated in loop code.

## Step 8 — add loop propagation coverage

Add a focused inline unit test for the formatter.

Construct a representative detailed Zellij failure string.

Assert the final refusal includes client, detected version 0.40.1, supported
range >= 0.43.0, static-binary remedy, and doctor follow-up.

Run:

```text
cargo test -p lisa-cli loop_cmd::tests::test_format_dependency_preflight_error
```

Expected result: the new propagation test passes.

Run the combined CLI package tests:

```text
cargo test -p lisa-cli
```

Expected result: all unit and integration tests pass; intentionally ignored live
harnesses remain ignored.

## Step 9 — commit the loop source unit

Inspect the exact loop diff and status.

Use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-046-01-02 \
  --message "Surface Zellij incompatibility in loop preflight" \
  --include crates/lisa-cli/src/loop_cmd.rs
```

Verification:

- the command returns a commit ID;
- `loop_cmd.rs` is clean;
- no ticket-owned source path remains modified, staged, or untracked;
- unrelated worktree state remains untouched.

## Step 10 — run full verification

Run the workspace test suite:

```text
cargo test --workspace
```

Run formatting verification:

```text
cargo fmt --all -- --check
```

Run compilation checks appropriate to the repository if the normal tests do
not cover them:

```text
cargo check -p lisa-cli
```

Inspect recent ticket commits and their exact paths.

Inspect final status and ordinary index.

Verification criteria:

- 0.43.x and 0.44.x are accepted by CLI adapter tests;
- 0.40.1 is unsupported and names detected version, range, and remedy;
- invalid version output is unsupported with a distinct parse diagnostic;
- loop refusal preserves all detailed facts;
- existing workspace behavior remains green;
- no source ownership residue remains.

## Step 11 — write `progress.md`

Record each completed implementation unit, commit ID, tests, and any deviations.

If an expected verification command cannot run, record the exact reason and
alternative evidence.

Do not claim completion until source status is clean and tests pass.

## Step 12 — review the committed result

Read the combined committed diff for the two ticket source units.

Check acceptance criteria one by one.

Look specifically for:

- accidental acceptance of unparseable successful stdout;
- copied floor literals in CLI code;
- missing unsupported match arms;
- misleading `missing` wording;
- loss of remedy text between doctor and loop;
- launch side effects occurring before refusal;
- environment-dependent or globally mutating tests;
- unrelated paths in ticket commits.

Run any targeted test needed to resolve a review concern.

## Step 13 — write Review artifacts

Write `review.md` in the private attempt directory.

Summarize source changes, commit boundaries, coverage, verification results,
known limitations, and open concerns.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

only if all acceptance behavior is ready and source ownership is clean.

If a blocking issue remains, write the required block shape with a concrete
actionable reason instead.

After both Review artifacts exist, remain on T-046-01-02 and stop.
