# Progress — T-045-02-01 launcher argv construction

## Status

Implementation complete.
The meaningful source unit is committed through Lisa's isolated ticket transaction.
All ticket-owned source paths are clean.
Focused, CLI, workspace, formatting, and WASM checks pass.

## Completed work

### Native launcher module

Created `crates/lisa-cli/src/codex_launcher.rs`.

Added `CodexLauncherArgs` carrying:

- exact `assignment_path: PathBuf`;
- configurable `codex_bin: PathBuf`;
- optional routed `model: Option<String>`.

Added a pure native argv builder using `Vec<OsString>`.
The current vector is:

```text
--dangerously-bypass-approvals-and-sandbox
--dangerously-bypass-hook-trust
[--model]
[exact model]
--
<exact assignment path>
```

Every value occupies its own vector element.
The assignment path is derived directly from `PathBuf::as_os_str`.
There is no UTF-8 conversion, shell quoting, word splitting, command-string join, or
composer transport.

Added a regular-file preflight.
Lisa does not spawn Codex if the supplied assignment path is absent or not a regular
file.
The launcher does not read or copy assignment contents.

Added native process execution with `std::process::Command::status`.
Environment, stdin, stdout, and stderr use inherited defaults.
This preserves lifecycle environment and the interactive pane terminal.
Spawn/wait failure includes the selected Codex executable path.

### CLI wiring

Modified `crates/lisa-cli/src/main.rs`.

Registered the launcher module.
Added a hidden `launch-codex` command accepting:

- positional assignment path;
- optional `--model`;
- `--codex-bin`, defaulting to `codex`.

The command constructs `CodexLauncherArgs` directly from Clap values.
It returns normally when the child succeeds.
It propagates the child's numeric nonzero exit status.
It exits `1` for a status without a numeric code or a launcher error.

The operator-facing top-level help remains unchanged.
The pre-existing claim command from T-045-01-02 was already committed before this
ticket edited `main.rs`; this ticket's diff contains only the new launcher additions.

### Argv-capture acceptance test

Created `crates/lisa-cli/tests/codex_launcher.rs`.

The Unix black-box test runs `CARGO_BIN_EXE_lisa` against a temporary executable
capture stub.
The stub executable's own path contains spaces and a quote.
The assignment filename contains:

- spaces;
- single and double quotes;
- `$()` syntax;
- semicolon;
- glob-like brackets;
- backticks;
- an embedded newline.

The model string also contains shell-significant characters.
The capture stub writes each received argument followed by NUL.
The test asserts the complete six-element vector byte-for-byte.
The exact hostile assignment path is the final and sole positional element.

No real Codex process, Zellij session, network call, or model token is used.

## Design/reference verification

The OpenAI docs skill was used because the change depends on current Codex CLI
interactive behavior.
Its manual helper was attempted first but rejected the response because the required
`x-content-sha256` header was absent.
An official documentation search did not yield a more specific interactive argv
reference.

The installed version-matched `codex-cli 0.144.3` help was then inspected.
It explicitly reports:

```text
codex [OPTIONS] [PROMPT]
```

That concrete local surface informed the decision to place the exact assignment path
in the one initial prompt position after a `--` separator.

## Deviations and corrections

Structure provisionally described seven captured arguments.
The Plan corrected the count before implementation.
The implemented modeled vector has six elements:

1. bypass approvals/sandbox flag;
2. bypass hook trust flag;
3. `--model`;
4. model value;
5. `--` separator;
6. assignment path.

No unplanned source file was added or modified.
No change was needed in `lisa-cli/src/lib.rs` because the integration test exercises
the binary directly.

The CLI command name is `launch-codex`, matching the selected Design.
The command is hidden and intentionally omitted from the curated plumbing footer
until the dependent injection ticket consumes it.

## Verification results

### Focused acceptance

`cargo test -p lisa-cli --test codex_launcher` passed:

- 1 passed;
- 0 failed.

The test proves the metacharacter-heavy assignment path is unchanged and occupies one
child argv element.

### Help regression

`cargo test -p lisa-cli --test help_surface` passed:

- 5 passed;
- 0 failed.

The operator help snapshot remains unchanged and all currently enumerated commands
still resolve.

### CLI suite

`cargo test -p lisa-cli` passed:

- CLI library unit tests passed;
- 269 binary unit tests passed;
- all enabled integration tests passed;
- 1 environment-gated real-Zellij test remained ignored as expected.

The concurrent T-045-01-02 claim command tests also passed.

### Workspace suite

`cargo test --workspace` passed.
Relevant totals included:

- 269 CLI binary unit tests;
- 200 core unit tests;
- 387 plugin unit tests;
- all enabled integration and doc tests;
- the real-Zellij test ignored by its environment gate.

### Repository quick check

`just check` passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- `cargo test --workspace` passed again.

### Formatting and diff checks

`cargo fmt --all -- --check` passed.
`git diff --check` passed before commit.

## Commit

Commit:

`2c895f5cbcb4dce24d5264614427e03015e82e62`

Message:

`feat(cli): launch Codex with exact assignment argv`

The isolated transaction included exactly:

- `crates/lisa-cli/src/codex_launcher.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/codex_launcher.rs`.

The command used the required shape:

```text
lisa commit-ticket \
  --ticket-id T-045-02-01 \
  --message "feat(cli): launch Codex with exact assignment argv" \
  --include crates/lisa-cli/src/codex_launcher.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/codex_launcher.rs
```

No ordinary `git add`, ordinary-index staging, or ordinary `git commit` was used.

## Final ownership check

`git show --name-only` for the commit reports exactly the three ticket-owned paths.
`git status --short` for those paths is empty.
`git diff --cached --name-only` is empty.

Other repository changes and Lisa-managed files remain outside this ticket and were
not altered or included.

## Remaining work outside this ticket

T-045-02-02 must change the plugin/Zellij path to invoke `lisa launch-codex` with the
exact retained `AssignmentRef.path` and lifecycle values.
This ticket deliberately does not modify the current adapter shell line or composer
delivery state.

Later stories remain responsible for:

- first-action claim instructions;
- claim-based ownership evidence;
- delivered-awaiting-claim state;
- bounded fallback evidence;
- clean Codex exit per ticket;
- lease and nonce revocation;
- authoritative completion;
- real Codex/Zellij field validation.
