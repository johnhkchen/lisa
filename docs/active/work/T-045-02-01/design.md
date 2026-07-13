# Design — T-045-02-01 launcher argv construction

## Design goal

Create one native Lisa boundary that starts interactive Codex with the exact durable
assignment path as its initial positional prompt.
The boundary must preserve argument identity independently of shell quoting rules.
It must remain consumable by the next ticket's Zellij injection change without
absorbing scheduler or ownership work now.

## Evaluation criteria

The design is evaluated against these constraints:

1. The assignment path is one unchanged child argv element.
2. No shell parses the path between Lisa's launcher and Codex.
3. No interactive composer paste transports the assignment reference.
4. Existing Codex flags and optional model routing remain expressible.
5. Lifecycle environment can be inherited by the child.
6. Interactive terminal stdio remains attached to the pane.
7. The launcher is directly fixture-testable without model tokens or Zellij.
8. T-045-02-02 can invoke it through a short bounded line.
9. Claude, claim, ownership, and completion behavior remain untouched.
10. The change adds no unnecessary dependency or general process framework.

## Option 1 — extend the existing plugin shell script

The current adapter could append the assignment path to its formatted shell command
and rely on `shell_quote`.

Advantages:

- smallest apparent source edit;
- existing atomic `.lisa-launch-*.sh` transport remains in place;
- existing model and lifecycle formatting can be reused.

Disadvantages:

- `/bin/sh` or the pane shell still parses the composed command;
- safety depends on correct quoting at every dynamic fragment;
- the acceptance criterion asks for a Lisa-owned argv launcher, not only a safely
  quoted shell string;
- a test of the formatted string is weaker than observation at a child-process
  boundary;
- it overlaps the next ticket's injection work.

This option is rejected because it retains the interpretation layer the ticket is
intended to remove.

## Option 2 — pass the assignment through an environment variable

The plugin could export `LISA_ASSIGNMENT_PATH` and a launcher could read the variable
before starting Codex.

Advantages:

- the path need not appear as a shell positional argument to the launcher;
- environment values are inherited directly by a native child;
- the launcher line can remain short.

Disadvantages:

- the shell must still construct the environment assignment at the injection edge;
- the launcher's required input becomes implicit;
- stale inherited values are easier to misuse than an explicit per-invocation
  argument;
- a missing variable produces a late runtime error;
- it adds another assignment identity transport alongside the existing exact path.

This option is rejected because an explicit path argument is smaller and easier to
audit.

## Option 3 — adapt `agent-exec`

The existing host-side wrapper could gain an interactive mode and accept the
assignment path instead of a prompt.

Advantages:

- it already has a pure Codex argv builder;
- it already uses `std::process::Command`;
- it already reports spawn and wait failures.

Disadvantages:

- `agent-exec` is designed around `codex exec --json`;
- it nulls stdin, pipes stdout, parses JSONL, persists thread IDs, and writes synthetic
  lifecycle signals;
- interactive Codex requires inherited terminal stdio;
- sharing the module would create conditionals across unrelated headless and TUI
  contracts;
- the existing argv ordering is version-pinned to the `exec` subcommand.

This option is rejected because it broadens a stable headless wrapper for no shared
behavior beyond `Command::new`.

## Option 4 — add a separate installed launcher binary

The CLI crate could declare a second binary such as `lisa-codex-launcher`.

Advantages:

- the process boundary is explicit;
- it avoids adding another `lisa` subcommand;
- its black-box test can target a very small executable.

Disadvantages:

- installation and embedding currently revolve around one `lisa` binary;
- the plugin config carries one resolved `lisa_bin`, not a sibling binary directory;
- packaging must guarantee the second executable is present;
- T-045-02-02 would need new binary discovery and fallback rules;
- the added operational surface is disproportionate to one command.

This option is rejected because it complicates distribution and discovery.

## Option 5 — hidden native `lisa launch-codex` command

Add a small CLI module and a hidden subcommand to the existing `lisa` binary.
The subcommand accepts:

- the assignment path as a positional `PathBuf`;
- an optional routed model;
- a configurable Codex executable for deterministic tests.

The module constructs native `OsString` arguments and calls
`std::process::Command::status`.

Advantages:

- the plugin already knows an absolute or PATH-resolved `lisa` binary;
- no second install artifact is required;
- `Command::arg` establishes the exact no-shell child boundary;
- inherited stdio preserves the interactive TUI;
- inherited environment preserves existing `LISA_*` lifecycle identity;
- the configurable child executable enables an argv-capture fixture;
- hiding the command avoids expanding operator-facing help.

Disadvantages:

- it adds a command arm to the already central `main.rs`;
- the next ticket must shell-quote the invocation used to reach this native boundary;
- a resident Lisa parent process remains while Codex runs.

The remaining shell edge is limited to starting Lisa itself.
Once Clap has parsed the assignment argument, the path crosses to Codex without a
shell or composer.
The parent process is acceptable because it waits and preserves terminal streams.

This option is selected.

## Child argv decision

The launcher will build this logical vector:

```text
codex
  --dangerously-bypass-approvals-and-sandbox
  --dangerously-bypass-hook-trust
  [--model, <model>]
  --
  <assignment-path>
```

The assignment path is the sole interactive `[PROMPT]` positional.
It is not prefixed or interpolated into a sentence.
This makes the acceptance assertion exact: the captured vector contains the original
path as one element.

The `--` separator is a distinct fixed element.
It prevents a valid path beginning with `-` from being interpreted as another Codex
option.
It does not alter the prompt value received by Clap in Codex.

The optional model remains two elements, `--model` and the exact model string.
No string concatenation creates a flag fragment.

## Native type decision

The input assignment and child executable use `PathBuf`.
The argv builder returns `Vec<OsString>`.
Fixed flags and the routed model convert into `OsString` without shell formatting.
The final path is cloned from `assignment_path.as_os_str()`.

This avoids `to_string_lossy` and preserves non-UTF-8 Unix paths even though the
acceptance fixture uses readable hostile UTF-8 characters.
No path canonicalization occurs because canonicalization would change the supplied
argv identity and can fail for otherwise meaningful relative paths.

## File validation decision

The launcher will require the assignment path to name a regular file before spawning
Codex.
The dependency contract promises an atomically published durable file.
A missing, directory, or stale reference should fail before opening an unassigned TUI.
The check does not read or copy assignment contents.
It also keeps the process input bounded to a path.

The argv-capture test will create the hostile-named assignment file, so it exercises
the production validation path.

## Process and exit decision

`Command::status` is used with no stdin/stdout/stderr override.
The child inherits the launcher's terminal streams.
The function returns the child's `ExitStatus` on a successful spawn and wait.
It returns a contextual string only for preflight, spawn, or wait failure.

The CLI arm exits successfully when Codex succeeds.
It propagates Codex's numeric nonzero exit code when available.
If the child ended without a numeric code, it exits with `1`.
This prevents the wrapper from silently converting provider failure into success.

Signal-file fallback and recovery interpretation remain for T-045-02-02 and later
state-machine tickets.

## Test design

Add a black-box CLI integration test.
It creates:

- a temporary executable capture stub whose own path contains spaces and a quote;
- a real assignment file whose name contains spaces, quotes, `$()`, semicolon,
  brackets, backticks, and a newline;
- a capture output path supplied only through a fixture environment variable.

The stub writes every received argument as NUL-delimited bytes using quoted `"$@"`.
The test invokes the built `lisa` binary with separate `Command::arg` calls.
It supplies a hostile model string as well, demonstrating that model routing is also
one argument rather than a composed fragment.

The assertion compares the complete captured vector with the expected fixed flags,
model pair, separator, and original assignment path.
An exact vector assertion proves no expansion, splitting, quote removal, substitution,
or globbing occurred inside the Lisa-to-Codex launch boundary.

## Scope exclusions

This ticket will not:

- modify `CodexAdapter`;
- modify `State::prepare_fresh_launch`;
- inject the launcher into a Zellij pane;
- change assignment generation or storage;
- add a claim to assignment text;
- change acknowledgement or ownership states;
- exit an existing TUI at a ticket boundary;
- revoke leases or nonces;
- run real Codex or consume model tokens;
- change Claude behavior;
- publish artifacts directly to `docs/active/work`.
