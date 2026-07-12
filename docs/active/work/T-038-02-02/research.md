# T-038-02-02 Research: Clippy Zero Warnings

## Ticket scope

- Ticket `T-038-02-02` is a high-priority task in story `S-038-02`.
- Its stated goal is to drive Clippy to zero warnings on native Rust targets and
  on the plugin's WebAssembly target.
- Its acceptance criterion requires both the commands and their output to be
  recorded.
- It limits code changes to those required by a lint and therefore establishes
  behavior preservation as a constraint.
- The ticket depends on `T-038-02-01`.
- The current repository head is `763f2a4`, whose subject is
  `Complete T-038-02-01`.
- The dependency is therefore present in the checked-out history.
- The ticket begins in the Research phase.
- Attempt artifacts belong under
  `.lisa/attempts/T-038-02-02/1/work/`.
- Lisa, rather than this task, owns phase/status frontmatter changes and final
  artifact publication.

## Workspace organization

- The root `Cargo.toml` defines one Cargo workspace with resolver version 2.
- Workspace members use the glob `crates/*`.
- The workspace uses Rust edition 2021.
- All three crates currently share version `0.4.0-rc.6`.
- `lisa-core` contains shared domain types, parsing, diagnostics, routing,
  provenance, and DAG logic.
- `lisa-cli` contains the native command-line program and its integration tests.
- `lisa-plugin` contains the Zellij plugin, its scheduler/state implementation,
  UI, provider adapters, and acknowledgement handling.
- The plugin crate is the only workspace member that is intentionally compiled
  for `wasm32-wasip1` in routine project workflows.
- Native workspace Clippy nevertheless compiles all workspace members for the
  host target.

## Existing lint entry points

- The root `justfile` provides a `lint` recipe.
- The first command in that recipe is
  `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.
- The second command is `cargo clippy -p lisa-core -- -D warnings`.
- The third command is `cargo clippy -p lisa-cli -- -D warnings`.
- Each recipe command denies warnings, so any diagnostic has a nonzero result.
- The repository CI workflow installs the Clippy component.
- CI installs the `wasm32-wasip1` Rust target as well.
- CI runs warning-strict Clippy separately for `lisa-core` and `lisa-cli` on the
  native target.
- CI runs warning-strict Clippy for `lisa-plugin` on `wasm32-wasip1`.
- CI also runs formatting, all workspace tests, and a WASM `cargo check`.
- These entry points already encode warning-free linting as a project invariant.

## Commands relevant to the acceptance criterion

- The ticket names `cargo clippy --workspace` as the native command.
- Cargo applies that command to every workspace package for the host target.
- Adding `-- -D warnings` makes the zero-warning property machine-enforced.
- The existing WASM lint command is
  `cargo clippy -p lisa-plugin --target wasm32-wasip1`.
- Adding `-- -D warnings` likewise makes any target-specific warning fail.
- A full workspace command with the WASM target is not the repository convention
  because the CLI is a native executable and the plugin is the WASM deliverable.
- The CI and `justfile` agree on the package boundary for the WASM invocation.

## Baseline execution

The native warning-strict command was run from the repository root:

```text
$ cargo clippy --workspace -- -D warnings
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Checking lisa-core v0.4.0-rc.6 (/Users/johnchen/swe/repos/lisa/crates/lisa-core)
   Compiling lisa-cli v0.4.0-rc.6 (/Users/johnchen/swe/repos/lisa/crates/lisa-cli)
    Checking lisa-plugin v0.4.0-rc.6 (/Users/johnchen/swe/repos/lisa/crates/lisa-plugin)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.92s
```

- The command exited with status 0.
- No compiler or Clippy diagnostic appeared.
- The `Blocking waiting` lines are Cargo coordination notices caused by
  concurrent baseline commands, not warnings.
- All three workspace packages were covered.

The warning-strict WASM command was also run from the repository root:

```text
$ cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s
```

- The command exited with status 0.
- No compiler or Clippy diagnostic appeared.
- Cargo reused existing build products for this invocation.
- The installed target and Clippy component are functional in this environment.

## Current warning surface

- The observed native warning count is zero.
- The observed `wasm32-wasip1` warning count is zero.
- Because warnings were denied, both successful exits prove there were no lint
  warnings hidden behind a successful status.
- There are no diagnostic locations to map to source files.
- There are no machine-applicable lint suggestions in the baseline output.
- There are no allow attributes implicated by the baseline output.
- There are no target-specific warnings requiring conditional handling.

## Repository state and ownership boundaries

- Before task work, `git status --short` showed modifications to
  `.lisa/provenance.jsonl` and `docs/active/tickets/T-038-02-02.md`.
- Those files are controlled by the active Lisa workflow.
- No Rust source file was reported modified or untracked at baseline.
- The assignment forbids ordinary `git add` and `git commit` for ticket work.
- Any ticket-owned source changes must use `lisa commit-ticket` with exact paths.
- Attempt phase artifacts are not source changes and remain in the private
  attempt directory for Lisa to admit and publish.
- Lisa owns the final completion commit containing admitted work artifacts.

## Constraints and assumptions

- The stable toolchain available to this checkout determines the active Clippy
  lint set.
- A zero-warning result can change when the toolchain changes, even if source
  code does not.
- This ticket validates the current checkout and current installed toolchain.
- The ticket does not request dependency upgrades, edition migration, formatting
  policy changes, or lint configuration changes.
- No behavior change is justified when both required surfaces are already clean.
- Tests remain useful as a regression gate even when linting requires no edits.
- Exact command output should be captured again during implementation/review so
  the final handoff records the post-work state rather than only the baseline.

## Research conclusion

- The repository already has warning-strict Clippy commands in both local and CI
  workflows.
- The acceptance surfaces are the host workspace and the WASM plugin package.
- Both surfaces pass warning-strict Clippy at the start of this attempt.
- No source-level lint remediation is currently indicated.
- The remaining work is to choose and document a validation-only approach,
  execute the final gates, and record their exact results in the attempt
  artifacts without manufacturing a behavior or configuration change.
