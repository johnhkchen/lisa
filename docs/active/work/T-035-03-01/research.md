# Research: T-035-03-01 Codex trust path canonicalization

## Ticket boundary

The ticket starts in Research and is an enabling change for the later
Codex-first fresh-loop rerun.

Its acceptance criterion is deliberately fixture-verifiable and does not
authorize a live or metered provider execution.

The required invariant is that the project path written to Codex's user-level
trust configuration equals the path Codex obtains after resolving its working
directory.

On macOS, temporary paths commonly expose the same directory through two
spellings:

- fixture creation and shell input can use `/var/...`;
- filesystem canonicalization resolves that path to `/private/var/...`.

Codex compares project trust using the resolved cwd, so the strings must agree.

## Origin of the requirement

T-034-03-02 ran a freshly built Lisa CLI and WASM in an isolated temporary
project.

The loop pregranted Codex trust for a fixture path spelled under `/var/...`.

Codex displayed its project as `/private/var/...` and still presented an
interactive trust prompt.

That run required manual confirmation before its downstream Codex completion
could proceed.

The evidence is recorded in:

- `docs/active/work/T-034-03-02/evidence/live-run.md`;
- `docs/active/work/T-034-03-02/review.md`.

The prerequisite review identifies path canonicalization as follow-up work,
while also separating it from the provider-neutral first-command truncation.

## Current command path resolution

`crates/lisa-cli/src/main.rs` defines `resolve_path`.

For a relative argument it joins the argument to `current_dir`.

For an absolute argument it returns the supplied `PathBuf` unchanged.

It does not call `std::fs::canonicalize`.

The `Loop` command passes this path to config loading and then to
`loop_cmd::run_loop`.

The `Doctor` command also receives a root resolved by the same helper.

Preserving the user's path spelling is otherwise useful for CLI behavior and is
not itself identified as a general bug by this ticket.

## Loop trust-pregrant path

`crates/lisa-cli/src/loop_cmd.rs` computes every provider that the loop can
route to.

If Codex is among them, it first verifies that `.codex/hooks.json` exists.

It then calls `crate::doctor::pregrant_codex_trust(root)`.

The `root` passed here is the noncanonical path supplied by `main.rs` or by a
direct caller.

The pregrant happens before the WASM is extracted, the layout is generated, and
Zellij is executed.

This is the correct lifecycle point for avoiding the prompt; the defect is the
path identity written at that point.

## Shared trust implementation

`crates/lisa-cli/src/doctor.rs` owns Codex trust configuration.

`codex_home` resolves `$CODEX_HOME`, falling back to `$HOME/.codex`.

`pregrant_codex_trust` selects that home and delegates to
`pregrant_codex_trust_in`.

`pregrant_codex_trust_in` currently formats the supplied path directly as:

`[projects."<work_tree display>"]`

It reads the existing `config.toml`, checks for an identical table header, and
appends a trusted entry when none exists.

It creates the Codex home directory as needed and writes the complete file.

The operation is best-effort: it returns a Boolean rather than surfacing an I/O
error to the caller.

`pregrant_codex_trust` returns the config path on success for doctor reporting.

## Existing behavioral guarantees

The implementation preserves existing configuration content.

An exactly matching project table is treated as already seeded.

Repeated calls using the same path spelling do not duplicate the table.

Failure to create or write the Codex configuration returns false/None and does
not prevent Lisa from continuing.

The surrounding comments describe the behavior as version-volatile and retain
provider bypass flags as fallback behavior.

No parser is used for the TOML; exact header matching is the established local
mechanism.

## Existing test coverage

The unit tests in `doctor.rs` exercise the filesystem without invoking Codex.

`test_pregrant_codex_trust_writes_block` checks that `/work/tree` is emitted
with `trust_level = "trusted"`.

`test_pregrant_codex_trust_is_idempotent` checks a repeated literal path.

`test_pregrant_codex_trust_preserves_existing` checks unrelated configuration
content survives.

`test_codex_home_honors_env` covers `$CODEX_HOME` selection.

There is no existing assertion that a symlinked project path is resolved.

There is no existing assertion comparing the emitted project key to a
canonicalized cwd.

The current literal `/work/tree` fixture normally does not exist, so a direct
`canonicalize` call would fail for that existing test input.

## Filesystem behavior and constraints

`std::fs::canonicalize` resolves symbolic links and returns an absolute path.

It requires the target path to exist.

The isolated loop fixture exists before `lisa loop` performs the trust
pregrant, so canonicalization can succeed in the real ticket scenario.

Doctor is also normally run against an existing project root.

The low-level `_in` helper is currently callable in tests with synthetic,
nonexistent paths.

Any change at that boundary must decide how an I/O failure interacts with the
existing best-effort contract.

Rejecting a previously accepted nonexistent test path would be a behavior
change broader than the macOS fixture requirement.

Falling back to the supplied path keeps the existing behavior for missing or
otherwise unresolvable paths while fixing existing symlinked fixtures.

## Appropriate regression fixture

A free unit test can create a real directory and a symlink alias to it.

The test can pass the alias to the trust pregrant, canonicalize that same alias
as the stand-in for Codex's resolved cwd, and inspect the generated TOML.

This reproduces the path-identity class without depending on macOS's global
`/var -> /private/var` mapping.

On macOS, the same canonicalization operation also resolves the actual temp-dir
prefix observed in T-034-03-02.

The regression should compare exact project-header bytes, not merely check that
some trusted entry exists.

No `codex` binary, authentication, network request, token usage, Zellij server,
or built WASM is needed for this proof.

## Scope boundaries

The parent story says this ticket touches trust-pregrant fixture setup and no
scheduler code.

The next ticket, T-035-03-02, owns the committed live startup harness and its
Codex-first/Claude-first run.

This ticket does not repair first-assignment delivery, change seat ownership,
alter acknowledgement semantics, or run the installed providers.

It also does not need to canonicalize every Lisa CLI path globally.

The narrow production boundary is the path used to construct Codex's project
trust key.

## Repository and workflow constraints

All phase artifacts belong under
`.lisa/attempts/T-035-03-01/1/work/` for this attempt.

The shared `docs/active/work/T-035-03-01/` path is owned by Lisa publication.

Ticket phase and status frontmatter are Lisa-owned and must remain untouched by
this implementation.

The parent repository already contains unrelated modified and untracked files.

Ticket-owned source changes must be committed only with `lisa commit-ticket`
and exact repository-relative include paths.

## Research conclusion

The observed trust prompt is explained by a string-identity mismatch at
`pregrant_codex_trust_in`: an existing project path is emitted without resolving
symlinks, while Codex identifies the project by its canonical cwd.

The shared helper is the smallest boundary that covers both loop preflight and
doctor preflight.

The existing best-effort behavior and nonexistent-path tests constrain the
change to canonicalize when possible and retain a safe fallback.

A symlink-backed unit fixture can assert exact equality between the pregranted
project key and the canonicalized fixture cwd without any live metered run.
