# Review — T-046-04-02

## Disposition

Pass.

The runtime failures now direct users and agents to managed or released paths,
doctor covers Git and embedded-WASM packaging, loop surfaces missing Git before
raw Git discovery, tests are green, and every ticket-owned source change is
committed and clean.

## Summary

This ticket changed doctor dependency composition, loop preflight ordering,
two remediation messages, and focused unit/integration coverage.

No CLI flags, configuration schema, public library API, build policy, or plugin
behavior changed.

The implementation is contained in three files and one exact-path commit.

## Files changed

### `crates/lisa-cli/src/doctor.rs`

Added the shared released shell-installer command used by doctor and loop.

Added a required Git checker based on `git --version`.

Missing Git now renders as a named `not found` row with:

`sudo apt install git`

Split dependency construction into two scopes.

`build_required_deps_checks` contains Git and the selected agent and is reused
by real-loop preflight.

Doctor's `build_checks` extends that set with embedded-WASM packaging and the
existing optional Rust-target diagnostic.

Added embedded-WASM classification over a supplied byte slice.

A nonempty slice passes as `plugin embedded`.

An empty slice is required/unsupported and explicitly names the empty embedded
WASM plugin placeholder.

Its remedy is the complete released shell-installer command.

The real doctor path evaluates the compile-time `PLUGIN_WASM` slice, so the
empty file emitted by `build.rs` is no longer reported as healthy.

Updated generic missing-dependency tests so they no longer normalize the old
Zellij source-install wording.

Added direct tests for managed Zellij guidance, both embedded-WASM outcomes,
dependency composition, exact remedies, and source-build omissions.

### `crates/lisa-cli/src/loop_cmd.rs`

Added `embedded_wasm_error()` as the direct string-production boundary for the
loop packaging failure.

The message preserves `WASM plugin not embedded` for recognizability.

It names the empty development placeholder.

It gives the complete latest-release shell-installer command.

It no longer recommends cloning Lisa or running a source release build.

Moved `discover_git_root` after shared external dependency preflight.

The loop therefore checks for Git and emits the apt remedy before attempting
`git rev-parse`.

The adjacent empty-WASM guard remains before hashing or writing plugin bytes.

Added a direct error-message test and refreshed the dependency-preflight fixture
to assert managed-runtime wording.

### `crates/lisa-cli/tests/zellij_version_preflight.rs`

Extended the existing Unix process harness to optionally isolate PATH.

Existing tests retain host-PATH behavior.

New tests construct a PATH containing supported Zellij and Claude stubs but no
Git executable.

The doctor test proves a process-level failing exit with a named Git gap and apt
remedy.

The loop test proves the same structured preflight appears before Git-root
discovery and that the raw discovery error is absent.

## Acceptance assessment

### Zellij remedy stays on the light path

Met for executable and test source.

The production resolver recommends
`[runtime] zellij = "managed"` and a test now asserts that exact boundary.

The prohibited Zellij Cargo-install phrase was removed from all CLI runtime and
test source occurrences.

The test-only version classifier continues recommending Zellij's prebuilt
static binaries, also a no-compile route.

### Loop embedded-WASM error names the shell installer

Met.

The exact error helper uses the same complete installer command documented in
README.

Its unit test asserts the installer asset and rejects clone/`just release`
instructions.

### Missing Git is named with apt remedy

Met.

Git is a required doctor dependency.

The isolated-PATH doctor integration test proves the report names Git, says it
is not found, supplies `sudo apt install git`, and exits unsuccessfully.

The isolated-PATH loop integration test additionally proves preflight wins over
raw Git-root discovery.

### Empty build.rs placeholder fails doctor

Met.

Doctor's required dependency vector includes a check over the actual
compile-time `PLUGIN_WASM` bytes.

The byte-parameterized unit test proves an empty embed is unsupported and
renders the named placeholder plus released-installer remedy.

A companion test proves nonempty bytes pass.

Dependency-composition tests prove the packaging check is part of doctor rather
than loop's generic external dependency set.

### Literal whole-repository phrase search

The executable/test-string intent is met, but a literal whole-tree search is
self-referential and still matches historical descriptions.

Matches remain in the current ticket acceptance text, its parent active
story/epic, archived T-013 records, prior research, and survey documents.

Several active records are unrelated untracked shared-worktree files.

They do not produce runtime output, and rewriting or committing them would
claim orchestration/history owned outside this ticket.

The scoped runtime/test search is clean.

This is documented as a wording caveat rather than a product blocker because
the criterion's literal command necessarily matches the criterion itself.

## Test coverage

### Unit coverage

Doctor tests cover:

- selected-client dependency composition;
- shared external-dependency composition;
- Git formatting with exact apt command;
- managed Zellij remedy at the production resolver boundary;
- nonempty embedded bytes;
- empty placeholder classification and formatted remedy;
- source-build recipe absence.

Loop tests cover:

- exact shell-installer failure production;
- absence of clone/release guidance;
- dependency-preflight formatting with managed Zellij guidance;
- unchanged layout and Git-root helpers through the existing suite.

### Integration coverage

The Unix preflight suite has six passing tests.

It covers supported Zellij versions, below-floor rejection, unparseable output,
doctor Git absence, and loop Git absence/order.

The process harness uses real CLI dispatch and controlled executable stubs.

### Workspace coverage

`cargo test --workspace` passed.

Reported passing groups included 19 CLI library tests, 306 CLI binary unit
tests, 207 core tests, 395 plugin tests, all shown integration targets, and doc
tests.

The real-Zellij delivery boundary remains ignored under its existing explicit
external-environment gate.

The final sixth process test was added after the workspace run and passed in the
focused integration target.

`cargo fmt --all -- --check` passed.

Both working and committed diffs passed whitespace validation.

## Commit and ownership

The exact-path Lisa transaction created:

`316d5db3ed4bb147f6502a613305ff249f906a49`

Commit subject:

`fix(cli): surface actionable runtime remedies`

The commit contains exactly:

- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`;
- `crates/lisa-cli/tests/zellij_version_preflight.rs`.

It contains 193 insertions and 30 deletions.

All three source paths are clean after commit.

No ordinary `git add`, ordinary `git commit`, or broad index operation was
used.

Unrelated modified/untracked Lisa metadata and active planning records remain
outside the commit and untouched.

## Open concerns

No blocking product or code concern remains.

The apt hint is intentionally Debian-specific because acceptance explicitly
requests it and the motivating environment is Crostini/Debian.

Other platforms still receive a clear named Git gap even if they must translate
the package-manager command.

Doctor continues its optional Rust-target diagnostic when rustup exists. That
check remains non-required and does not contradict the new released installer
remedies.

The embedded-WASM classifier treats every nonempty slice as present; build.rs
already validates the WebAssembly header before embedding an existing release
file, so duplicating header validation at runtime is unnecessary for this
placeholder trap.

## Handoff

The implementation is ready for Lisa's completion publication and final ticket
commit.

Do not manually update ticket phase/status or start another ticket.
