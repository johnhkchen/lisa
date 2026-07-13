# Structure — T-045-02-01 launcher argv construction

## Source file map

### Create `crates/lisa-cli/src/codex_launcher.rs`

This native module owns interactive Codex process construction.
It is separate from `agent_exec.rs` because it preserves terminal stdio and does not
parse JSONL, synthesize signals, persist threads, or run the `exec` subcommand.

Define a crate-visible input value:

```text
CodexLauncherArgs
  assignment_path: PathBuf
  codex_bin: PathBuf
  model: Option<String>
```

`assignment_path` is the exact durable reference returned by the assignment boundary.
`codex_bin` defaults at the CLI layer to `codex` and is injectable for tests.
`model` represents the optional route selected by the scheduler.

Define an internal pure argv builder:

```text
build_codex_argv(&CodexLauncherArgs) -> Vec<OsString>
```

The builder returns elements in this order:

1. `--dangerously-bypass-approvals-and-sandbox`;
2. `--dangerously-bypass-hook-trust`;
3. `--model` when `model` is present;
4. the exact model value when present;
5. `--`;
6. the exact assignment path.

The function does not:

- concatenate model or path values into flag strings;
- call `shell_quote`;
- convert the assignment path through UTF-8;
- read assignment contents;
- construct lifecycle environment variables;
- invoke a shell.

Define the public-to-binary execution boundary:

```text
run_codex_launcher(CodexLauncherArgs) -> Result<ExitStatus, String>
```

Responsibilities:

1. require `assignment_path.is_file()`;
2. call the pure builder;
3. create `Command::new(codex_bin)`;
4. attach the vector through `.args(argv)`;
5. call `.status()` with inherited environment and stdio;
6. return the child status;
7. attach the executable path to spawn/wait errors.

The module has no new external dependencies.
`std::ffi::OsString`, `std::path::PathBuf`, and `std::process` provide the complete
surface.

### Modify `crates/lisa-cli/src/main.rs`

Register the module beside the existing native command modules:

```text
mod codex_launcher;
```

Add one hidden Clap subcommand:

```text
LaunchCodex {
    assignment: PathBuf,
    --codex-bin <PATH> = codex,
    --model <MODEL> = optional
}
```

The assignment is positional so the future plugin invocation can remain direct and
bounded.
`codex_bin` is a path-shaped value because fixture executables may contain path
separators and hostile characters.
The command remains hidden from ordinary top-level help.

Add a corresponding `main` match arm.
It constructs `CodexLauncherArgs` without formatting any field.
On `Ok(status)`:

- return normally for success;
- otherwise exit with `status.code().unwrap_or(1)`.

On `Err(error)`:

- print `Error: {error}` to stderr;
- exit with code `1`.

Do not add the internal command to the curated operator/plumbing footer in this
ticket.
It is a scheduler implementation detail and is not yet invoked until T-045-02-02.

Do not change existing `AgentExec`, `CaptureUsage`, claim, commit, completion, or
operator arms.

### Create `crates/lisa-cli/tests/codex_launcher.rs`

Add a black-box argv-capture integration test against `CARGO_BIN_EXE_lisa`.

The test uses Unix fixture behavior and imports:

- `std::fs`;
- `std::os::unix::fs::PermissionsExt`;
- `std::process::Command`;
- `tempfile` through the existing dev dependency.

Fixture construction:

1. create a temporary directory;
2. create a capture stub with a hostile executable filename;
3. write a small `#!/bin/sh` program that truncates `$ARGV_CAPTURE` and writes each
   quoted argument followed by NUL;
4. mark the stub executable;
5. create a durable assignment file whose filename contains shell metacharacters,
   single and double quotes, whitespace, a newline, and command syntax;
6. choose a capture output path.

Invocation:

```text
lisa launch-codex
  --codex-bin <stub>
  --model <hostile-model>
  <hostile-assignment-path>
```

Each caller argument is supplied with a separate native `Command::arg` call.
The capture path is passed through `ARGV_CAPTURE` solely for the stub's output.

Assertions:

1. the launcher exits successfully;
2. the capture file exists;
3. NUL splitting yields exactly seven arguments;
4. the first two are the fixed safety/trust flags;
5. the next two are `--model` and the unchanged hostile model;
6. the fifth is `--`;
7. the final captured value equals the exact assignment path;
8. no extra argument represents an expansion or split fragment.

The exact full-vector comparison is the primary acceptance assertion.
Individual diagnostics can be added only to make a failure legible.

## Public interfaces

No library API is added to `crates/lisa-cli/src/lib.rs`.
The production consumer is the CLI binary's own match arm.
The test intentionally exercises the black-box executable rather than reaching into
the module.

No public API is added to `lisa-core` or `lisa-plugin`.
The launcher accepts the already-defined durable path and does not duplicate
`AssignmentRef` or `AssignmentClaim` types.

## Data flow

The source change establishes this host-side flow:

```text
Clap receives one assignment OS argument
                 |
                 v
CodexLauncherArgs.assignment_path: PathBuf
                 |
                 +--> regular-file preflight
                 |
                 v
build_codex_argv: Vec<OsString>
                 |
                 v
Command::new(codex_bin).args(argv)
                 |
                 v
operating-system process spawn
                 |
                 v
Codex sees assignment path as one [PROMPT] argv element
```

T-045-02-02 will provide the upstream edge:

```text
plugin exact AssignmentRef.path
                 |
                 v
short pane invocation of lisa launch-codex
```

That edge is documented but not implemented here.

## Environment and stdio boundary

The launcher does not call `env_clear`.
Existing `LISA_BIN`, `LISA_AGENT_CLIENT`, `LISA_PANE_ID`, `LISA_TICKET_ID`, and
`LISA_ATTEMPT_ID` values therefore remain available to Codex and its hooks.

The launcher does not set `stdin`, `stdout`, or `stderr`.
Rust's inherited defaults leave the interactive TUI connected to the pane.
The argv-capture stub inherits the same streams but does not use them.

## Failure boundary

Missing or non-regular assignment path:

```text
assignment file is not a regular file: {path}
```

Codex spawn failure:

```text
failed to spawn Codex launcher child {codex_bin}: {error}
```

The exact final wording may follow existing CLI sentence style, but it must include
the relevant path and native error.

`Command::status` combines spawn and wait in one `io::Result`; a single contextual
message is sufficient unless implementation naturally separates them.
No shell error fallback is created in this module.

## Test and formatting boundary

Focused verification:

```text
cargo test -p lisa-cli --test codex_launcher
```

CLI regression verification:

```text
cargo test -p lisa-cli
```

Workspace verification:

```text
cargo test --workspace
cargo fmt --all -- --check
git diff --check
```

The new test consumes no Codex model tokens and does not require Zellij.
It launches only the local capture fixture.

## Commit unit

One meaningful ticket-owned source unit contains:

- `crates/lisa-cli/src/codex_launcher.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/codex_launcher.rs`.

Commit it through:

```text
lisa commit-ticket
  --ticket-id T-045-02-01
  --message "feat(cli): launch Codex with exact assignment argv"
  --include <each exact path above>
```

No phase artifact is part of this source commit.
Lisa publishes attempt-local workflow artifacts after lease verification.

## Explicit non-changes

Do not modify:

- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-core/src/claim.rs`;
- lifecycle hook templates;
- dashboard state or labels;
- ticket frontmatter;
- shared `docs/active/work/T-045-02-01`.
