# Plan — T-046-01-01 version parse and supported range

## Implementation sequence

### 1. Establish the progress record

Create `progress.md` in the private attempt work directory.

Record the ticket boundary, planned source paths, and initial remaining work.

Verification:

- the artifact exists only under `.lisa/attempts/T-046-01-01/1/work/`;
- it does not alter shared `docs/active/work` state.

### 2. Declare the semantic-version dependency

Add `semver = "1.0"` to `crates/lisa-core/Cargo.toml` normal dependencies.

Do not request unused serialization features.

Let Cargo update the workspace lockfile during the first build or test.

Verification:

- `cargo metadata --no-deps` recognizes the manifest;
- the lockfile's `lisa-core` package record names `semver`;
- no unrelated package versions change.

### 3. Add the public module boundary

Add `pub mod version;` to `crates/lisa-core/src/lib.rs`.

Keep module declarations in the existing flat organization.

Do not add crate-root item re-exports.

Verification:

- the module path is `lisa_core::version`;
- existing module declarations remain unchanged.

### 4. Implement `ZellijVersion`

Create `crates/lisa-core/src/version.rs`.

Add module documentation explaining the process-output and shared-policy
boundary.

Define the private semver-backed newtype.

Derive semantic comparison and value traits.

Add a const `release` constructor for policy constants.

Implement `Display` by delegating to the wrapped version.

Verification:

- release values compare numerically;
- prerelease values use semantic precedence;
- display emits canonical semantic versions.

### 5. Implement command-output parsing

Add `parse_command_output` to `ZellijVersion`.

Trim through whitespace tokenization rather than modifying the source string.

Require exactly `zellij` plus one version token.

Parse the token through `semver::Version`.

Define `ParseZellijVersionError` as the stable domain error.

Implement standard error and display traits.

Verification:

- `zellij 0.43.0` parses;
- surrounding and repeated whitespace parses;
- prerelease and build metadata parse;
- missing, malformed, misidentified, and extra-token output fail.

### 6. Implement the supported range

Define `ZellijVersionRange` with one inclusive public minimum.

Add its `contains` method and display implementation.

Declare `SUPPORTED_ZELLIJ_RANGE` at 0.43.0.

Write the maintenance comment beside the constant.

Name the `zellij-tile = "0.43"` pin and its manifest path.

Name 0.41.0 as the theoretical protocol floor for
`write_chars_to_pane_id`/`write_to_pane_id`.

State why the tested 0.43.0 floor is enforced instead.

Verification:

- exact floor is contained;
- versions below it are excluded;
- newer patch and minor releases are contained;
- range display is `>= 0.43.0`;
- the numeric floor occurs in one production policy definition.

### 7. Implement classification verdicts

Define `ZellijVersionVerdict` with `InRange`, `BelowFloor`, and `Unparseable`.

Retain the parsed version in the two parsed variants.

Add `classify_zellij_version_output` as the one-step downstream API.

Route successful parsing through the range constant.

Route every parsing failure to `Unparseable`.

Verification:

- no parse error reaches `InRange`;
- classification contains no repeated 0.43.0 literal;
- downstream code can match the three required outcomes directly.

### 8. Add focused unit coverage

Add inline tests in `version.rs`.

Stable release cases:

- `zellij 0.43.0` is in range;
- a 0.43 patch release is in range;
- a 0.44 release is in range;
- `zellij 0.40.1` is below floor.

Prerelease cases:

- `zellij 0.43.0-rc.1` parses but is below floor;
- `zellij 0.44.0-rc.1` parses and is in range.

Garbage cases:

- arbitrary prose;
- invalid semantic version;
- missing product name;
- missing version;
- extra tokens;
- empty output.

Ordering cases:

- 0.43.10 sorts after 0.43.9;
- 0.43.0-rc.1 sorts before 0.43.0.

Display cases:

- detected version is canonical;
- declared range names its inclusive floor.

Verification:

- `cargo test -p lisa-core version` passes;
- test names make each acceptance boundary visible.

### 9. Format and inspect

Run `cargo fmt --all -- --check`.

If formatting reports changes, run the repository formatter and recheck only
ticket-owned diffs.

Inspect `git diff --` for the four exact source paths.

Inspect `git status --short` without touching unrelated state.

Verification:

- only planned source paths contain ticket-owned changes;
- unrelated dirty paths are preserved;
- no ordinary index entry was created.

### 10. Run focused verification

Run `cargo test -p lisa-core version`.

Run `cargo test -p lisa-core` to include interactions with all core modules.

If a failure appears, determine whether it is ticket-owned before editing.

Update `progress.md` with results and deviations.

Verification:

- all new classification tests pass;
- all pre-existing core tests pass.

### 11. Run workspace verification

Run `cargo test --workspace`.

Run `just check` as the project-defined combined WASM check and native test
command.

Record exact outcomes in `progress.md`.

Verification:

- all workspace crates compile against the new direct dependency;
- the WASM plugin still checks for `wasm32-wasip1`;
- no downstream regression is observed.

### 12. Commit the meaningful source unit

Use only:

`lisa commit-ticket --ticket-id T-046-01-01 --message <message> --include
crates/lisa-core/src/version.rs --include crates/lisa-core/src/lib.rs --include
crates/lisa-core/Cargo.toml --include Cargo.lock`

Do not run `git add`, ordinary `git commit`, or a broad include.

Verification:

- the command succeeds;
- the commit contains exactly the four planned source paths;
- those paths are clean afterward;
- unrelated worktree and ordinary-index state remains untouched.

### 13. Review the committed result

Inspect the committed diff or exact source files.

Confirm both acceptance criteria against code and test evidence.

Check that no doctor, loop, scheduler, ticket-frontmatter, or shared artifact
change slipped into scope.

Check ticket-owned status after the isolated transaction.

### 14. Write Review artifacts

Write `review.md` with source summary, tests, limitations, and handoff notes for
T-046-01-02.

If all criteria and cleanliness checks pass, write exactly:

`{"disposition":"pass","reason":null}`

Otherwise write the blocking shape with a non-empty actionable reason.

Remain on this ticket after both artifacts exist.

Do not publish, change ticket phase/status, or start the dependent ticket.

## Atomicity rationale

The new module, its declaration, its direct dependency, and the lockfile edge
are one meaningful unit.

Splitting them would temporarily create either unreachable code, an unresolved
crate reference, or an inaccurate lockfile.

Tests live with the module and therefore belong to the same atomic source unit.

The workflow artifacts are deliberately excluded from the source transaction
because Lisa admits and publishes them through the attempt lease.

## Planned fallback behavior

If semantic-version const construction fails under the workspace toolchain,
replace only the range's storage representation while preserving the public
minimum, membership, display, and verdict contract.

If `just check` fails solely because an external target is unavailable, retain
the passing native test evidence and block Review with the exact actionable
environment requirement unless a safe in-scope target install already exists.

If unrelated concurrent changes touch one of the four planned paths, stop and
inspect ownership rather than overwriting or committing them.
