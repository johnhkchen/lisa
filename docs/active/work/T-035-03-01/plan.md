# Plan: T-035-03-01 Codex trust path canonicalization

## Objective

Make the Codex project-trust entry use the same canonical filesystem identity as
Codex's resolved cwd, and prove that identity with a free symlink-backed unit
fixture.

## Constraints carried into implementation

- Do not edit ticket phase or status frontmatter.
- Do not write workflow artifacts to `docs/active/work/T-035-03-01/`.
- Do not invoke a live Codex provider or perform a metered run.
- Do not change scheduler, ownership, acknowledgement, or startup behavior.
- Preserve best-effort trust seeding when canonicalization fails.
- Preserve user configuration and exact-header idempotence.
- Commit ticket-owned source only with `lisa commit-ticket`.
- Include only exact repository-relative ticket-owned paths.
- Leave no ticket-owned source staged, modified, or untracked before Review.

## Step 1: add canonical project identity at the trust writer

Modify `pregrant_codex_trust_in` in
`crates/lisa-cli/src/doctor.rs`.

Before generating the TOML projects header:

1. call `work_tree.canonicalize()`;
2. use its returned `PathBuf` when successful;
3. fall back to `work_tree.to_path_buf()` on error;
4. build the header from this single selected path.

Do not change the function signature or return contract.

Verification within the diff:

- the selected path is used for lookup and append through the shared `header`;
- no unwrap or panic is introduced;
- nonexistent paths still reach the existing write flow;
- configuration read/create/write behavior is unchanged.

## Step 2: update the helper contract documentation

Extend the Rust documentation for `pregrant_codex_trust_in`.

Document that:

- existing work trees are canonicalized;
- the purpose is equality with Codex's resolved cwd;
- macOS temp aliases such as `/var` and `/private/var` motivate the behavior;
- a canonicalization error falls back under the existing best-effort policy.

Retain existing notes about:

- user-level `$CODEX_HOME/config.toml`;
- repo-local config limitations;
- exact-header idempotence;
- version volatility;
- bypass fallback.

## Step 3: add the symlink-backed regression

In the existing `doctor.rs` test module, add a test gated with `#[cfg(unix)]`.

Fixture setup:

1. create a temporary root;
2. create a real `project` directory;
3. create `project-link` as a symbolic link to `project`;
4. choose a temporary `codex-home` path;
5. canonicalize `project-link` to model Codex's cwd identity.

Assertions before pregrant:

- the alias path and canonical path are different;
- the canonical target exists.

Execute:

`pregrant_codex_trust_in(codex_home, alias)`.

Assertions after pregrant:

- the helper returns true;
- `config.toml` exists;
- the exact canonical projects header is present;
- `trust_level = "trusted"` directly follows that header in the expected entry;
- the alias-form projects header is absent.

This is the ticket's acceptance-level check.

## Step 4: format

Run:

```text
cargo fmt --all
```

Inspect that formatting changes are limited to the ticket-owned source path.

If unrelated files change because of pre-existing work, do not include them in
the ticket transaction and restore nothing destructively; inspect ownership
before proceeding.

## Step 5: run focused verification

Run the new test by filter:

```text
cargo test -p lisa-cli test_pregrant_codex_trust_matches_canonicalized_cwd
```

Expected result:

- one matching test passes;
- no Codex process starts;
- no network or authentication is used.

If the alias path unexpectedly equals the canonical path, fix the fixture so it
uses a real symbolic-link component rather than weakening the assertion.

## Step 6: run existing trust tests

Run:

```text
cargo test -p lisa-cli pregrant_codex_trust
```

Expected coverage:

- canonical existing symlink path;
- literal nonexistent path write;
- repeated invocation idempotence;
- preservation of unrelated config.

All tests must pass.

## Step 7: run the CLI suite and static checks

Run:

```text
cargo test -p lisa-cli
cargo fmt --all -- --check
cargo check -p lisa-cli
```

The full CLI suite checks that doctor and loop callers still compile against the
unchanged helper interfaces.

The check validates the production binary without requiring the WASM target or
provider binaries.

## Step 8: inspect the implementation diff

Review:

```text
git diff -- crates/lisa-cli/src/doctor.rs
git diff --check -- crates/lisa-cli/src/doctor.rs
git status --short
git diff --cached --name-only
```

Verify:

- only intended lines in `doctor.rs` belong to this ticket;
- no ordinary-index entry was created;
- unrelated repository changes remain untouched;
- no ticket frontmatter was manually edited;
- artifacts remain attempt-private.

## Step 9: record progress before the source transaction

Create/update
`.lisa/attempts/T-035-03-01/1/work/progress.md` with:

- phase completion;
- implementation details;
- test commands and outcomes;
- deviations, if any;
- exact source path awaiting commit.

This artifact is not included in the source transaction.

## Step 10: commit the meaningful source unit

Use Lisa's isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-035-03-01 \
  --message "fix(cli): canonicalize Codex project trust paths" \
  --include crates/lisa-cli/src/doctor.rs
```

If the active `lisa` binary does not expose the expected command, use the
repository-built CLI only after verifying its command syntax; do not fall back
to ordinary Git staging or commits.

Record the returned commit ID in `progress.md`.

## Step 11: verify post-commit cleanliness

Run ticket-focused status checks:

```text
git status --short -- crates/lisa-cli/src/doctor.rs
git diff --cached --name-only
git show --stat --oneline HEAD
```

Acceptance for this step:

- `doctor.rs` is neither modified nor untracked;
- `doctor.rs` is not staged in the ordinary index;
- the isolated commit contains exactly the intended ticket-owned source file;
- unrelated worktree paths remain as they were.

## Step 12: complete the Review artifact

Write `.lisa/attempts/T-035-03-01/1/work/review.md`.

Summarize:

- the canonicalization behavior;
- the exact file modified;
- the unit fixture and its equality assertion;
- full verification results;
- source commit ID;
- any test or platform gaps;
- open concerns such as retained stale alias entries.

Stop after Review.

Do not start T-035-03-02, edit ticket frontmatter, publish artifacts manually,
or prepare the Done commit.

## Acceptance mapping

### Symlink-resolved fixture path is pregranted

Steps 1 and 3 ensure an alias path is canonicalized before the project table is
written.

### Pregranted path equals Codex canonicalized cwd

Step 3 constructs the expected table from `canonicalize(alias)` and checks
exact header equality while rejecting the alias header.

### No interactive prompt is needed

The exact project key Codex uses is present with `trust_level = "trusted"`.
The live prompt is not invoked in this ticket; equality is the free contract
proof requested by the acceptance criterion.

### Verified without live metered run

Steps 5–7 use only local Rust tests and compilation.
