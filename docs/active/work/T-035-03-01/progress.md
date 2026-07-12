# Progress: T-035-03-01 Codex trust path canonicalization

## Status

Research, Design, Structure, Plan, implementation, and pre-commit verification
are complete.

The ticket-owned source unit is ready for the required Lisa isolated commit.

## Completed phases

- [x] Read `AGENTS.md` and the canonical `CLAUDE.md` project context.
- [x] Read `docs/knowledge/rdspi-workflow.md`.
- [x] Read ticket T-035-03-01 and its parent story.
- [x] Read the T-034-03-02 live evidence and review that exposed the mismatch.
- [x] Map CLI root resolution, loop preflight, doctor preflight, and Codex trust
  configuration ownership.
- [x] Write attempt-private `research.md`.
- [x] Evaluate canonicalization boundaries and write `design.md`.
- [x] Define the one-file source/test boundary in `structure.md`.
- [x] Sequence implementation, verification, commit, and review in `plan.md`.

## Implementation completed

Modified `crates/lisa-cli/src/doctor.rs`.

`pregrant_codex_trust_in` now attempts to canonicalize the supplied work tree
before constructing the Codex `[projects."..."]` table header.

When canonicalization succeeds, the resolved physical path is used for both
existing-header detection and new entry creation.

When canonicalization fails, the helper retains the supplied path spelling.

This preserves the existing best-effort contract for synthetic, missing, or
temporarily inaccessible paths.

The function signature, Boolean result, configuration preservation, directory
creation, and write behavior are unchanged.

## Documentation completed

The helper's Rust documentation now records:

- equality with Codex's resolved cwd as the invariant;
- macOS `/var` to `/private/var` temporary-directory aliases as the motivating
  case;
- fallback to the supplied path when canonicalization fails.

The existing warnings about user-level config, version volatility, and bypass
behavior remain intact.

## Regression completed

Added Unix-gated test:

`test_pregrant_codex_trust_matches_canonicalized_cwd`

The test creates a real temporary project plus a symbolic-link alias.

It verifies the alias and canonical cwd differ, then pregrants trust through the
alias path.

It asserts the generated `config.toml` contains the exact canonical project
header and `trust_level = "trusted"` entry.

It also asserts the alias-form project header is absent.

This supplies the ticket's free acceptance proof without invoking Codex,
Zellij, authentication, the network, or any metered model.

## Focused verification

Passed:

```text
cargo test -p lisa-cli test_pregrant_codex_trust_matches_canonicalized_cwd
```

Result:

- 1 matching test passed;
- 0 failed.

Passed:

```text
cargo test -p lisa-cli pregrant_codex_trust
```

Result:

- 4 trust-pregrant tests passed;
- 0 failed.

The existing write, idempotence, and preservation tests remain green alongside
the new canonical-cwd test.

## Full verification

Passed:

```text
cargo test -p lisa-cli
```

Result:

- 274 CLI unit tests passed;
- 1 `atomic_provider_contract` integration test passed;
- 0 failed.

Passed:

```text
cargo fmt --all -- --check
cargo check -p lisa-cli
```

The CLI and shared core compiled successfully in the development profile.

Passed:

```text
git diff --check -- crates/lisa-cli/src/doctor.rs
```

No whitespace errors were reported.

## Repository integrity before commit

`git diff --cached --name-only` is empty.

No ordinary `git add` or `git commit` command was used.

The only ticket-owned source modification is:

`crates/lisa-cli/src/doctor.rs`

Other modified/untracked paths shown by repository status are unrelated Lisa
coordination, publication, or concurrent ticket work and were not edited or
included by this implementation.

Ticket T-035-03-01 phase/status frontmatter was not manually modified.

All authored phase artifacts were written only under the attempt-private work
directory.

## Deviations from plan

None in the implementation or verification strategy.

The repository's Lisa process admitted phase artifacts to the shared work path
while the turn continued; this was automatic publication, not a direct write by
the agent, and those paths are excluded from the source transaction.

## Source transaction

Completed with exact include:

```text
crates/lisa-cli/src/doctor.rs
```

Message:

```text
fix(cli): canonicalize Codex project trust paths
```

Commit:

```text
398e87415112fce3e244513d2f5ea23145809f1a
```

The transaction was created by `lisa commit-ticket`; no ordinary-index staging
or ordinary Git commit was used.

## Remaining

- [x] Commit the one meaningful ticket-owned source unit with
  `lisa commit-ticket`.
- [x] Verify `doctor.rs` is clean and absent from the ordinary index.
- [ ] Write `review.md` with acceptance mapping, coverage, and open concerns.
- [ ] Stop on this ticket and wait for Lisa's completion transaction.
