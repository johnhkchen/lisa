# Review — T-045-02-01 launcher argv construction

## Disposition

Pass.

The launcher now crosses the Lisa-to-Codex boundary with native operating-system
arguments.
The acceptance fixture observes the hostile assignment path as one unchanged child
argv element.
All ticket-owned source is committed and clean.

## Commit reviewed

`2c895f5cbcb4dce24d5264614427e03015e82e62`

Message:

`feat(cli): launch Codex with exact assignment argv`

The commit contains exactly:

- `crates/lisa-cli/src/codex_launcher.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/tests/codex_launcher.rs`.

It was created with `lisa commit-ticket` and exact `--include` paths.
No ordinary-index commit or staging command was used.

## What changed

### Lisa-owned native launcher

The new `codex_launcher` module owns only interactive Codex child construction.
It accepts the exact assignment `PathBuf`, Codex executable `PathBuf`, and optional
model.

Its argv builder uses `Vec<OsString>`.
The fixed safety flags, optional model flag/value, separator, and assignment path are
individual vector elements.
The exact child shape with a model is:

```text
--dangerously-bypass-approvals-and-sandbox
--dangerously-bypass-hook-trust
--model
<model>
--
<assignment path>
```

The assignment path is Codex's sole initial `[PROMPT]` positional.
The `--` separator protects a path beginning with `-` from option parsing.
There is no sentence interpolation around the path.

The launcher requires that the path already name a regular file.
This consumes the atomic assignment writer's durable-reference guarantee and fails
before process creation for a missing or non-file reference.
It does not read the assignment body.

The child is spawned with `std::process::Command::status`.
No production shell is started.
No command string is constructed.
No `shell_quote`, lossy string conversion, variable expansion, globbing, command
substitution, or composer paste participates in the Lisa-to-Codex boundary.

Environment and stdio are inherited.
That preserves existing `LISA_*` lifecycle attribution and leaves interactive Codex
attached to the pane terminal.

### Hidden command surface

`main.rs` exposes the module as hidden `lisa launch-codex` plumbing.
The command takes:

- positional assignment path;
- optional `--model`;
- optional `--codex-bin`, defaulting to `codex`.

The binary propagates child success and numeric failure status.
Native preflight/spawn errors print an actionable `Error:` and exit `1`.

The command is not listed in operator help.
Existing operator and curated plumbing help snapshots remain unchanged.
T-045-02-02 can call the already-resolved Lisa binary instead of discovering a second
packaged executable.

### Argv-capture regression

The integration test invokes the actual built `lisa` binary.
It substitutes a local executable capture stub for Codex.
The stub's path itself contains spaces and a quote, which also proves
`Command::new(PathBuf)` does not split the executable name.

The assignment path contains spaces, both quote types, `$()`, semicolon, brackets,
backticks, and an embedded newline.
The optional model contains shell-significant characters too.

The fixture records NUL-delimited child arguments.
The test compares the full vector byte-for-byte with six expected elements.
The final value equals `assignment.as_os_str().as_encoded_bytes()` exactly.
Because the full vector count and values are asserted, a split, expansion,
substitution, or extra fragment fails the test.

The shell script is only the receiving capture fixture.
Production launcher code never invokes a shell.

## Acceptance criterion assessment

Criterion:

> Given an assignment path containing shell metacharacters and quotes, the launcher
> spawns codex with the path as a single argv element (no expansion/interpolation) —
> asserted by an argv-capture test.

Satisfied.

The assignment fixture is a real regular file with the required hostile characters.
The `--codex-bin` fixture captures what the launcher passes at the actual child
process boundary.
The captured final element equals the supplied path byte-for-byte and the complete
vector has no extra path fragments.

## Verification reviewed

### Focused

`cargo test -p lisa-cli --test codex_launcher` passed:

- 1 passed;
- 0 failed.

`cargo test -p lisa-cli --test help_surface` passed:

- 5 passed;
- 0 failed.

### CLI

`cargo test -p lisa-cli` passed all enabled tests.
This included:

- 269 binary unit tests;
- the new launcher fixture;
- three claim CLI tests from the completed dependency work;
- existing agent-exec, atomic-provider, usage, status, and help integrations;
- one real-Zellij test ignored by its documented environment gate.

### Workspace

`cargo test --workspace` passed all enabled tests.
The main crate totals observed were:

- CLI binary: 269 passed;
- core: 200 passed;
- plugin: 387 passed.

Integration and doc tests also passed.

### WASM and repository checks

`just check` passed:

- plugin `wasm32-wasip1` check passed;
- complete workspace tests passed again.

`cargo fmt --all -- --check` passed.
`git diff --check` passed before commit.

No real Codex process, Zellij process, or model-token call was used.
That matches this ticket's fixture-only honest boundary.

## Compatibility assessment

The installed `codex-cli 0.144.3` reports the interactive invocation as
`codex [OPTIONS] [PROMPT]`.
The implementation uses that current positional surface and preserves the two flags
already used by the existing adapter.
Optional model routing remains `--model` plus one exact value.

The OpenAI docs manual helper could not verify its fetched response because the
required integrity header was missing, and the narrower official-doc search did not
surface a better interactive argv definition.
The version-matched installed CLI help is the concrete contract used here.
This is recorded as a limitation rather than silently assuming a remembered syntax.

The launcher uses only standard library APIs and adds no dependency.
It does not alter serialized types, config files, generated templates, hooks, or
assignment filenames.

The CLI command waits as the parent of interactive Codex rather than replacing itself
with Unix `exec`.
Inherited terminal streams make this appropriate for a TUI, and portable
`Command::status` preserves the cross-platform Rust structure.

## Scope assessment

The source change stays at the requested host argv boundary.
It does not modify:

- plugin adapter command construction;
- Zellij pane injection;
- assignment writer/reference retention;
- claim serialization or publication;
- ownership evidence;
- acknowledgement deadlines or retry states;
- provider exit/reuse behavior;
- lease or nonce revocation;
- completion state;
- Claude launch behavior.

Those exclusions match the story split.
In particular, T-045-02-02 still needs to send only the launcher invocation through
Zellij and select a fresh TUI per ticket.

## Open concerns and limitations

### Upstream shell edge remains deferred

The new launcher guarantees no shell between parsed Lisa arguments and Codex.
The future Zellij command that invokes `lisa launch-codex` must still safely transport
the assignment path into Lisa as one argument.
That is the explicit acceptance boundary of T-045-02-02 and is not hidden here.

### Bare path is the initial prompt

Codex receives the exact assignment path, not the assignment body or a longer
instruction sentence.
This is necessary for the ticket's exact-path argv contract and keeps the launch
bounded.
Whether real Codex reliably treats that path as an instruction to read the file is
reserved for the real field-validation story.

### Unix-only capture fixture

The hostile argv capture test is guarded with `cfg(unix)` because it uses an
executable POSIX shell stub and Unix permission bits.
The production implementation itself uses portable `PathBuf`, `OsString`, and
`Command` APIs.
If Windows becomes a supported field-test platform, it should gain a native argv
capture fixture rather than weakening this Unix assertion.

### Current Codex flag surface can drift

The existing safety/trust flags and `[PROMPT]` positional are version-sensitive Codex
CLI surface.
The repository already documents this concern for `agent-exec`.
The real field story should rerun help/behavior checks after Codex upgrades.

None of these limitations blocks this ticket's requested fixture-proven argv
contract.

## Human review focus

A reviewer should confirm:

1. `build_codex_argv` pushes every dynamic value separately;
2. the final value comes directly from `assignment_path.as_os_str()`;
3. production code has no shell or composer boundary;
4. inherited stdio is intentional for the interactive TUI;
5. the full-vector fixture would fail on splitting or interpolation;
6. the next ticket, not this one, owns safe Zellij invocation and fresh-per-ticket
   integration;
7. commit `2c895f5` contains exactly the three reviewed paths.

## Source ownership

The three source paths are clean after commit.
The ordinary staged index is empty.
Unrelated Lisa-managed and concurrent repository changes remain outside this commit.

The work is ready for Lisa's completion publication and commit gate.
