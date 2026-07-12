# Review: T-035-03-01 Codex trust path canonicalization

## Outcome

The ticket is implemented and its acceptance criterion is satisfied by a local
filesystem regression with no live provider execution.

Lisa now canonicalizes an existing project path before writing Codex's
user-level project-trust entry.

This makes the pregranted table key equal to the cwd identity Codex obtains
after resolving filesystem aliases.

On macOS, a fixture supplied under `/var/...` is therefore trusted under its
resolved `/private/var/...` spelling.

The later Codex-first live harness can consume this behavior without an
interactive trust confirmation caused by that spelling mismatch.

## Source change

Modified:

`crates/lisa-cli/src/doctor.rs`

No other production or test source file was created, modified, or deleted by
this ticket.

The change was committed through Lisa's isolated transaction as:

`398e87415112fce3e244513d2f5ea23145809f1a`

Commit message:

`fix(cli): canonicalize Codex project trust paths`

The transaction included exactly:

`crates/lisa-cli/src/doctor.rs`

## Production behavior

`pregrant_codex_trust_in` now calls filesystem canonicalization on the supplied
work tree before constructing its `[projects."..."]` header.

The selected path is shared by both exact-header lookup and new-entry
generation.

That preserves idempotence after the corrected canonical entry exists.

If the project path cannot be canonicalized, the function falls back to the
original supplied path.

This fallback preserves the established best-effort behavior for missing,
synthetic, permission-limited, or otherwise unresolvable paths.

The function remains non-panicking and retains its Boolean success contract.

The surrounding configuration behavior is unchanged:

- `$CODEX_HOME/config.toml` remains the target;
- existing content is preserved;
- the Codex home is created as needed;
- an exact existing project table prevents duplication;
- write/create failure returns false;
- callers may continue because the pregrant is best-effort.

## Caller coverage

The change sits in the shared low-level trust writer.

Both existing production paths inherit it:

- `lisa doctor` when Codex is the selected client;
- `lisa loop` whenever any configured ticket route may use Codex.

No caller-specific canonicalization is required.

CLI-wide root resolution remains unchanged, so init, validate, status, setup,
usage capture, and commit-transaction behavior are not widened by this ticket.

## Regression test

Added:

`test_pregrant_codex_trust_matches_canonicalized_cwd`

The Unix-gated test creates an existing project directory and a symbolic-link
alias to that directory.

The alias is deliberately distinct from its canonicalized target.

The test passes the alias path to the same low-level trust writer used by
production.

It models Codex's cwd identity by canonicalizing the alias.

It then asserts the generated TOML contains the exact entry:

```toml
[projects."<canonical cwd>"]
trust_level = "trusted"
```

It additionally asserts the alias-form table header is absent.

This negative assertion prevents a false pass where both spellings are seeded.

The test uses no Codex binary, model invocation, authentication, network,
Zellij server, WASM build, or token spend.

## Acceptance mapping

### Pregrant the symlink-resolved fixture path

Met.

The trust writer resolves symbolic-link components before table construction.

The regression exercises a real link alias rather than relying only on string
manipulation.

On the target macOS environment, filesystem canonicalization also resolves the
observed `/var` temp prefix to `/private/var`.

### Pregranted path equals Codex's canonicalized cwd

Met.

The expected table header is built from the fixture alias's canonicalized cwd
and compared against the actual generated config bytes.

The uncanonicalized alias header is explicitly rejected.

### No interactive trust prompt needed

Met at the ticket's specified contract boundary.

The exact project identity Codex uses is present with
`trust_level = "trusted"`; the previous prompt was caused by the two paths not
matching.

The next story ticket owns the authorized live harness confirmation.

### No live metered run

Met.

All evidence came from local filesystem unit tests and Rust compilation.

## Verification results

Focused acceptance test:

```text
cargo test -p lisa-cli test_pregrant_codex_trust_matches_canonicalized_cwd
```

Result: 1 passed, 0 failed.

Trust-pregrant test group:

```text
cargo test -p lisa-cli pregrant_codex_trust
```

Result: 4 passed, 0 failed.

Full CLI tests:

```text
cargo test -p lisa-cli
```

Result:

- 274 unit tests passed;
- 1 provider-contract integration test passed;
- 0 tests failed.

Static verification:

```text
cargo fmt --all -- --check
cargo check -p lisa-cli
git diff --check -- crates/lisa-cli/src/doctor.rs
```

All passed.

## Coverage assessment

The test coverage is proportionate to the change.

The new regression covers canonicalization success and exact emitted identity.

Existing tests continue covering canonicalization fallback through the
nonexistent `/work/tree` fixture, configuration preservation, and idempotence.

The complete CLI suite verifies the unchanged wrapper and caller interfaces.

No live provider test belongs in this ticket because the acceptance criterion
explicitly requires a free check and T-035-03-02 owns the fresh-loop execution.

## Open concerns and limitations

### Existing alias entries are not removed

If a user's config already contains `[projects."/var/..."]`, the first corrected
run appends the canonical `[projects."/private/var/..."]` entry and leaves the
old table intact.

This is intentional: rewriting arbitrary user TOML is riskier than preserving
the stale, harmless alias entry.

Subsequent runs are idempotent against the canonical entry.

### Canonicalization fallback cannot guarantee identity

If an existing project cannot be resolved because of a filesystem or permission
error, Lisa retains the old best-effort behavior and may not match Codex's
identity.

The isolated harness fixture exists and is accessible before loop startup, so
this limitation does not affect the acceptance scenario.

### Symbolic-link test is Unix-gated

The regression uses `std::os::unix::fs::symlink` because the reported defect and
supported live environment are macOS/Unix.

The production `Path::canonicalize` call is portable.

No Windows-specific link fixture was added.

## Critical issues

None found in this change.

The provider-neutral first-assignment delivery defect remains separate work in
the rest of E-035 and was not modified here.

## Repository integrity

`crates/lisa-cli/src/doctor.rs` is clean after the isolated source commit.

The ordinary Git index contains no ticket-owned entry.

No ordinary `git add`, broad staging command, or ordinary `git commit` was used.

Ticket phase and status frontmatter were not manually edited.

All authored RDSPI artifacts were written to the attempt-private directory;
Lisa controls shared publication and final completion.

Unrelated modified and untracked repository paths remain untouched.

## Final assessment

The macOS fixture trust mismatch is closed at the correct shared boundary.

The emitted Codex project key is now derived from the same canonical filesystem
identity as Codex's cwd, with exact free regression evidence and preserved
best-effort behavior.

The ticket is ready for Lisa's completion transaction and no further source work
is required.
