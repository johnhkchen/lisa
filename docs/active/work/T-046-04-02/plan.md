# Plan — T-046-04-02

## Implementation sequence

### 1. Establish shared remediation text

Add the exact released Lisa shell-installer command to `doctor.rs` as an
internal crate-visible constant.

Use the README command without alteration.

Verify the constant contains the latest-release cargo-dist asset URL.

This step is foundational for both doctor and loop output.

### 2. Add the Git dependency check

Implement `check_git` using `git --version` through the existing command-version
helper.

Return a normal found result with the reported version on success.

Return a missing result with `sudo apt install git` on failure.

Insert the check into `build_checks` as required.

Update composition tests for Claude and Codex selections.

Verify only the selected agent remains in each vector.

### 3. Add the embedded-WASM dependency check

Import the compile-time plugin byte slice into doctor.

Implement a byte-parameterized classification helper.

Return found for any nonempty slice.

Return unsupported for an empty slice.

Name the build.rs empty placeholder in the failure description.

Use the released shell installer as its remedy.

Add a wrapper over actual `PLUGIN_WASM` and insert it into `build_checks` as
required.

### 4. Test doctor string production

Add a unit test for nonempty embedded bytes.

Add a unit test for empty embedded bytes.

Render the empty result through `CheckReport` so the user-visible description
and remedy are asserted where produced.

Assert the released shell installer appears.

Assert source-build commands do not appear.

Add or update a Git missing-report test to assert the apt command.

Add a Zellij runtime failure test asserting the current managed-runtime remedy.

### 5. Remove prohibited Zellij Cargo fixtures

Replace every literal occurrence in `doctor.rs` tests.

Use the managed runtime string for Zellij-specific mocks.

Use neutral installer text where a test only cares about generic formatting.

Run a focused search across CLI source and tests.

The search must find no prohibited phrase in runtime or test code.

Historical planning documents are inspected separately and remain outside this
ticket's source ownership.

### 6. Update loop ordering

Move real-loop Git-root discovery after shared dependency preflight.

Do not change dry-run behavior.

Ensure the Git root is still computed before it is used to preseed trust or
generate the layout.

Verify missing Git now fails through the dependency report first.

### 7. Replace the loop empty-WASM remedy

Extract the inline message into a private helper.

Preserve the recognizable `WASM plugin not embedded` phrase.

Name the empty placeholder.

Direct the user to the shared shell-installer command.

Remove cloning and source-release instructions.

Retain the guard before hashing or writing plugin bytes.

### 8. Test loop string production

Add a unit test beside other preflight formatter tests.

Assert the helper names the embedded-WASM problem.

Assert it contains the shell-installer asset path.

Assert it omits `git clone` and `just release`.

Update any neighboring fixed fixture to use managed-runtime wording.

### 9. Add process-level missing-Git coverage

Extend the Unix integration harness to support an isolated PATH.

Keep all existing callers on their current host-PATH behavior.

Create Zellij and Claude stubs as before.

Deliberately omit Git from the isolated path.

Run the compiled Lisa binary's doctor command.

Assert nonzero exit.

Assert stdout names Git as not found.

Assert stdout supplies `sudo apt install git`.

### 10. Reconcile existing doctor integration expectations

Run the focused integration test file.

If the test CLI carries empty placeholder bytes, update the Zellij-success test
to treat the named embedded-WASM failure as the expected post-Zellij result.

Keep its core assertions on detected and supported Zellij versions.

Do not weaken unsupported-Zellij assertions.

Document any necessary adjustment in progress.

### 11. Format and focused verification

Run `cargo fmt --all -- --check` after applying formatting.

Run doctor unit tests or the Lisa CLI library/binary test target as supported by
the crate layout.

Run `cargo test -p lisa-cli`.

Inspect failures for environment dependence versus implementation defects.

Correct implementation defects before proceeding.

### 12. Workspace verification

Run `cargo test --workspace` after focused tests pass.

Run `git diff --check` for the three ticket-owned source paths.

Run a scoped `rg` for old source-build remedies in CLI source and tests.

Run a scoped `rg` for the required shell installer and apt remedy.

Inspect the complete ticket-owned diff.

Confirm unrelated worktree paths remain untouched.

### 13. Record progress

Write `progress.md` in the attempt directory.

List each implementation step completed.

Record test commands and outcomes.

Record deviations from this plan with rationale.

Record the exact intended commit paths.

### 14. Commit the meaningful source unit

Run:

`lisa commit-ticket --ticket-id T-046-04-02 --message "fix(cli): surface actionable runtime remedies" --include crates/lisa-cli/src/doctor.rs --include crates/lisa-cli/src/loop_cmd.rs --include crates/lisa-cli/tests/zellij_version_preflight.rs`

Use the installed Lisa CLI, not ordinary Git index commands.

Do not include attempt artifacts or unrelated shared-worktree files.

Verify all three owned source paths are clean after the transaction.

### 15. Review

Inspect the committed diff and test evidence.

Write `review.md` with changes, coverage, limitations, worktree safety, and
commit identification.

Write `review-disposition.json` with exactly the required pass or block shape.

Choose pass only if all ticket-owned source paths are committed and clean and
the behavior is verified.

Remain on this ticket after Review.

## Verification matrix

### Zellij remedy

Production boundary: `check_zellij_runtime`.

Unit evidence: formatted failure contains `managed`.

Repository evidence: prohibited Cargo phrase absent from CLI source/tests.

### Loop embedded-WASM remedy

Production boundary: `embedded_wasm_error` and the guard in `run_loop`.

Unit evidence: shell installer present; clone/release recipe absent.

Integration compatibility: existing supported-Zellij loop preflight still
recognizes the error prefix when the test binary is a placeholder build.

### Missing Git

Production boundary: `check_git` in required dependency composition.

Unit evidence: formatted missing dependency contains apt remedy.

Integration evidence: doctor under isolated PATH names Git and exits nonzero.

Loop evidence: Git-root discovery occurs after shared dependency preflight.

### Empty embedded WASM

Production boundary: `check_embedded_wasm_bytes(PLUGIN_WASM)`.

Unit evidence: empty bytes are unsupported with named placeholder description.

Unit evidence: nonempty bytes pass.

Process evidence: when the Cargo test binary actually carries placeholder
bytes, doctor exits nonzero for that row.

### Regression protection

Focused crate suite covers internal formatting and process behavior.

Workspace suite covers cross-crate compilation and unchanged behavior.

Formatting and diff checks cover mechanical quality.

Exact-path Lisa transaction covers concurrency-safe source ownership.

## Atomicity rationale

Doctor checks, loop ordering, error copy, and integration tests form one
behavioral unit.

Committing doctor alone would cause existing doctor-success integration tests to
fail for placeholder builds.

Committing loop alone would leave doctor blind to the same packaging defect.

The three-file commit is therefore the smallest meaningful green unit.

If verification exposes an unrelated pre-existing failure, record it clearly
and use focused passing evidence to determine disposition without modifying
unowned code.
