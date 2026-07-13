# Plan — T-045-02-01 launcher argv construction

## Objective

Implement and prove a native Lisa launcher that gives interactive Codex the exact
attempt/nonce assignment path as one operating-system argv element.
Keep scheduler injection, claims, ownership, provider teardown, and live field tests
outside this ticket.

## Step 1 — establish a clean ownership baseline

Inspect:

```text
git status --short
git log -5 --oneline
```

Record which files are already modified or untracked by Lisa and concurrent tickets.
Specifically inspect the three planned CLI paths before editing.
Do not clean, stage, or alter unrelated work.

Verification criteria:

- no planned new path already belongs to another ticket;
- any existing `main.rs` changes are understood before applying the command arm;
- the current dependency commits are present;
- ordinary index state remains untouched.

## Step 2 — add the native launcher module

Create `crates/lisa-cli/src/codex_launcher.rs`.

Add `CodexLauncherArgs` with:

- `assignment_path: PathBuf`;
- `codex_bin: PathBuf`;
- `model: Option<String>`.

Add `build_codex_argv` returning `Vec<OsString>`.
Push fixed flags individually.
Push optional model flag and value individually.
Push `--` individually.
Push `assignment_path.as_os_str().to_os_string()` as the final element.

Add `run_codex_launcher`.
Reject a path that is not a regular file.
Build argv and call `Command::new(&codex_bin).args(argv).status()`.
Return `ExitStatus` and map I/O error into a contextual string.

Implementation constraints:

- no `sh -c`;
- no `Command::new("sh")`;
- no `shell_quote`;
- no joined command string;
- no assignment-content read;
- no stdio redirection;
- no environment clearing;
- no new dependency.

Local verification:

```text
cargo check -p lisa-cli
```

Success criteria:

- module compiles when wired;
- final argv element remains an `OsString` derived directly from the path;
- optional model remains its own element.

## Step 3 — wire the hidden CLI command

Modify `crates/lisa-cli/src/main.rs`.

Register `mod codex_launcher`.
Add hidden `LaunchCodex` to `Commands` with:

- positional `assignment: PathBuf`;
- `--codex-bin` defaulting to `codex`;
- optional `--model`.

Add the `main` match arm.
Construct `CodexLauncherArgs` directly from parsed values.
Call `run_codex_launcher`.
Return normally on child success.
Propagate a numeric child failure status.
Print contextual errors and exit `1` on native launcher failure.

Do not alter the curated top-level help footer.
Do not change existing command variants or their match arms.

Local verification:

```text
cargo run -p lisa-cli -- launch-codex --help
cargo test -p lisa-cli --test help_surface
```

Success criteria:

- hidden command resolves directly;
- operator help snapshot remains unchanged;
- all existing commands still parse;
- no behavior outside the new arm changes.

## Step 4 — add the argv-capture acceptance test

Create `crates/lisa-cli/tests/codex_launcher.rs`.

Use a Unix temporary executable fixture.
Give the fixture path spaces and a quote.
Write a script that:

1. truncates the file named by `ARGV_CAPTURE`;
2. iterates over quoted `"$@"`;
3. emits each argument followed by NUL.

Set executable permissions with `PermissionsExt`.

Create a real assignment file with a hostile leaf containing:

- whitespace;
- single quote;
- double quote;
- dollar and command-substitution syntax;
- semicolon;
- glob-like brackets;
- backticks;
- newline.

Choose a model string with shell-significant characters.
Invoke `CARGO_BIN_EXE_lisa` using native separate arguments:

```text
launch-codex
--codex-bin <stub>
--model <model>
<assignment>
```

Read and NUL-split the capture.
Assert exact equality with:

```text
[
  "--dangerously-bypass-approvals-and-sandbox",
  "--dangerously-bypass-hook-trust",
  "--model",
  hostile_model,
  "--",
  exact_assignment_path,
]
```

The expected count is six, not seven: two fixed flags, model pair, separator, and
assignment.
If Structure's provisional count says seven, this plan corrects it before
implementation; there is no extra argument.

Focused verification:

```text
cargo test -p lisa-cli --test codex_launcher
```

Success criteria:

- stub runs successfully;
- captured vector has exactly six entries;
- hostile model is unchanged;
- hostile assignment path is unchanged and is the sole final element;
- no shell expansion artifact or split fragment exists.

## Step 5 — inspect the focused diff

Run:

```text
git diff -- crates/lisa-cli/src/codex_launcher.rs
git diff -- crates/lisa-cli/src/main.rs
git diff -- crates/lisa-cli/tests/codex_launcher.rs
git diff --check -- <the same exact paths>
```

Inspect for:

- accidental concurrent-ticket changes in `main.rs`;
- shell process invocation in production;
- lossy path conversion;
- command string concatenation;
- assignment contents in argv;
- unrelated help or command edits;
- debug output or temporary fixture paths.

If a concurrent ticket changed `main.rs`, retain both independently scoped arms and
ensure the ticket commit includes only the combined current file when ownership is
safe.
If exact ownership cannot be established, stop before commit and document an
actionable block rather than consuming another ticket's source.

## Step 6 — run CLI regression tests

Run:

```text
cargo test -p lisa-cli
```

This covers:

- the new argv-capture integration test;
- top-level and per-command help snapshots;
- agent-exec argv behavior;
- claim/commit/completion commands present at the current HEAD;
- init, validate, status, doctor, and loop regressions.

Success criteria:

- all enabled CLI tests pass;
- the real-Zellij environment-gated case may remain ignored;
- no test starts real Codex;
- no model tokens are consumed.

## Step 7 — run workspace and formatting verification

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
git diff --check
```

Formatting is a mechanical rewrite and may touch only files that actually require
formatting.
Inspect status immediately afterward to ensure formatter output did not create
ticket ownership ambiguity.

When practical, run the repository quick check:

```text
just check
```

This adds the WASM compile check to native workspace tests.
The launcher itself is native CLI code and is not compiled into the WASM plugin, but
the full check guards cross-crate regressions.

Success criteria:

- format check passes;
- workspace tests pass;
- WASM check passes if run;
- no ticket-owned file remains with whitespace errors.

## Step 8 — commit the meaningful source unit

Before commit, confirm exact status:

```text
git status --short -- \
  crates/lisa-cli/src/codex_launcher.rs \
  crates/lisa-cli/src/main.rs \
  crates/lisa-cli/tests/codex_launcher.rs
```

Commit through Lisa's isolated transaction only:

```text
lisa commit-ticket \
  --ticket-id T-045-02-01 \
  --message "feat(cli): launch Codex with exact assignment argv" \
  --include crates/lisa-cli/src/codex_launcher.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/codex_launcher.rs
```

Do not run `git add`, `git add -A`, ordinary `git commit`, or any ordinary-index
mutation.

Record the returned commit ID in `progress.md`.

Post-commit criteria:

- exact source paths are clean;
- no source path owned by this ticket is staged, modified, or untracked;
- unrelated repository state is unchanged;
- commit diff contains only the intended launcher unit.

## Step 9 — write implementation progress

Create attempt-local `progress.md`.
Record:

- each completed implementation step;
- the exact source files;
- any deviation from Design or Structure;
- the Structure count correction from seven to six expected child arguments;
- focused and broad test outcomes;
- commit ID and exact `lisa commit-ticket` command shape;
- final ownership status.

Do not publish to `docs/active/work`.
Do not edit ticket phase or status.

## Step 10 — review and disposition

Inspect the committed diff:

```text
git show --stat --oneline <commit>
git show --format=fuller --no-ext-diff <commit> -- <exact paths>
git status --short
```

Evaluate:

- whether the acceptance path is truly one exact argv element;
- whether any shell exists between Lisa launcher and child;
- whether assignment content is absent from launcher argv;
- whether stdio and environment inheritance fit an interactive TUI;
- whether test coverage is deterministic and token-free;
- whether deferred Zellij work is clearly outside scope.

Write attempt-local `review.md` with changes, verification, limitations, and human
review focus.
Write exactly:

```json
{"disposition":"pass","reason":null}
```

when all requirements pass, or a block shape with a non-empty actionable reason if
source ownership, implementation, or verification remains unresolved.

After both artifacts exist, remain on T-045-02-01 and stop.
