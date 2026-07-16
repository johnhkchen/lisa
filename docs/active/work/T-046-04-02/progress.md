# Progress — T-046-04-02

## Status

Implementation and verification are complete.

The three ticket-owned source paths are ready for the isolated Lisa commit.

## Completed work

### Released installer remedy

Added one crate-internal constant for the complete supported shell-installer
command.

The value matches README's blessed installation command and points at the
latest cargo-dist `lisa-cli-installer.sh` release asset.

Doctor's empty-embed remedy and loop's empty-embed error use the same constant.

No production failure message in the changed CLI code recommends cloning or
building Lisa from source.

### Git doctor check

Added a required Git dependency check in `doctor.rs`.

It invokes `git --version` through the existing command-version helper.

A successful invocation reports the detected version.

A missing or failing executable reports `git` as `not found`.

The install line is exactly `sudo apt install git`.

Git is included in the external dependency set shared by doctor and loop
preflight.

### Loop Git failure ordering

Moved real-loop Git-root discovery after external dependency preflight.

The loop now checks Git before invoking `git rev-parse`.

An absent executable therefore produces a named dependency report and apt
remedy rather than `Failed to discover Git root` with a raw spawn error.

Dry-run behavior remains unchanged.

### Embedded-WASM doctor check

Imported `PLUGIN_WASM` into doctor.

Added a byte-parameterized classifier for the embedded plugin.

Nonempty bytes report `plugin embedded` and pass.

Empty bytes report `embedded WASM` as unsupported.

The description explicitly says the binary contains an empty embedded WASM
plugin placeholder.

The remedy tells the user to reinstall Lisa with the released shell installer.

The real doctor check delegates to this helper with the compile-time
`PLUGIN_WASM` slice.

The packaging check is required, so an empty-placeholder CLI exits doctor with
failure.

### Loop empty-WASM remedy

Extracted the loop message into `embedded_wasm_error()`.

The existing recognizable prefix, `WASM plugin not embedded`, is preserved.

The message names the empty development placeholder.

It provides the complete shell installer command.

The former clone-and-`just release` recipe was removed.

The defensive guard remains immediately before plugin-byte hashing and output.

### Zellij wording cleanup

The production runtime resolver already directs users to
`[runtime] zellij = "managed"`.

Added a test at that production boundary asserting the managed-runtime remedy.

Replaced every prohibited Zellij Cargo-install fixture in CLI source tests.

Updated the loop preflight formatter fixture to use the managed-runtime wording.

The test-only below-floor classifier continues naming Zellij's prebuilt static
binaries, which is also a no-compile path.

### Unit coverage

Added an empty embedded-WASM report test.

It asserts the named placeholder, unsupported classification, exact shared
installer command, and absence of clone/release source instructions.

Added a nonempty embedded-WASM test.

Added a production Zellij-runtime failure test for managed-mode guidance.

Updated doctor dependency-composition tests to include Git and embedded WASM.

Added a loop dependency-composition test proving Git is shared while embedded
WASM remains owned by the loop's adjacent guard.

Added a loop error-string test that asserts the shell installer and rejects the
old source-build recipe.

### Process-level coverage

Extended `zellij_version_preflight.rs` with an optional isolated PATH mode.

Existing tests continue to prepend stubs to the host PATH.

New missing-Git tests use only the temporary binary directory.

That directory contains supported Zellij and Claude stubs but no Git.

The doctor test asserts a failing exit, named Git row, `not found`, and the apt
remedy.

The loop test asserts the dependency preflight failure and apt remedy.

It also asserts the raw Git-root-discovery error is absent.

## Plan deviation

The initial Design proposed sharing the embedded-WASM check through the loop's
general `check_required_deps` call.

Focused tests passed with that shape, but control-flow review showed it would
make the loop's dedicated empty-WASM error unreachable in an actual placeholder
binary.

The implementation was refined before commit.

External requirements (`git` and the selected agent) now live in
`build_required_deps_checks` and are shared by doctor and loop.

Doctor's broader `build_checks` adds the embedded-WASM packaging diagnosis and
the optional Rust-target diagnostic.

Loop keeps its adjacent WASM guard and its own directly tested installer error.

This preserves both explicit ticket boundaries: doctor catches the build.rs
trap, and loop's existing error string is independently corrected.

No other plan deviation occurred.

## Verification

### Formatting

`cargo fmt --all -- --check` passed.

### Focused CLI suite

`cargo test -p lisa-cli` passed after the main implementation.

The main binary unit target reported 305 passing tests at that point, with all
CLI integration targets passing and the existing real-Zellij boundary ignored
because it requires external tools.

After the control-flow refinement, the workspace run reported 306 passing main
binary unit tests.

### Process preflight suite

Final command:

`cargo test -p lisa-cli --test zellij_version_preflight`

Result: 6 passed, 0 failed, 0 ignored.

This includes both missing-Git tests and all supported/unsupported Zellij
regressions.

### Workspace suite

`cargo test --workspace` passed.

Key reported groups included:

- 19 Lisa CLI library tests passed;
- 306 Lisa CLI binary unit tests passed;
- all Lisa CLI integration tests passed;
- 207 Lisa Core unit tests passed;
- 395 Lisa plugin unit tests passed;
- all shown cross-crate and doc-test targets passed;
- the existing real-Zellij delivery boundary remained ignored by its explicit
  environment gate.

The final added loop missing-Git process test was then run in the focused
six-test integration target and passed.

### Text and diff checks

`git diff --check` passed for all three owned paths.

A scoped search of `crates/lisa-cli/src` and `crates/lisa-cli/tests` found no
prohibited Zellij Cargo-install phrase.

A scoped search confirmed the apt remedy, managed-runtime remedy, and shell
installer appear at their implementation and assertion sites.

The owned diff contains 180 insertions and 30 deletions before the final small
loop integration test addition.

The complete diff was inspected.

## Repository-wide phrase caveat

A literal whole-repository search still finds quotations of the historical
Zellij Cargo path in archived tickets/work, prior research, active planning
records, and the current ticket text itself.

Those are descriptions of old behavior rather than executable failure strings.

Some active planning records are unrelated untracked shared-worktree files.

They were not rewritten or claimed by this ticket.

The prohibited phrase is absent from all CLI runtime and test source where it
could be produced or normalized as an example.

## Ownership guard

Only these source paths were edited:

- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/tests/zellij_version_preflight.rs`.

The shared worktree contains unrelated Lisa metadata and planning files.

They were not staged, cleaned, edited, or included.

No ordinary `git add` or `git commit` command was used.

## Isolated source commit

Ran `lisa commit-ticket` with ticket ID `T-046-04-02`, message
`fix(cli): surface actionable runtime remedies`, and exactly the three owned
source paths.

The command exited successfully and created commit:

`316d5db3ed4bb147f6502a613305ff249f906a49`

The committed diff contains 193 insertions and 30 deletions across only those
three paths.

`git show --name-only` confirmed no unrelated file entered the commit.

Committed-diff whitespace validation passed.

Scoped status after the transaction showed all three owned source paths clean.

No ordinary Git index or commit command was used.

## Remaining implementation work

None.

Review artifacts are the only remaining phase deliverables.
