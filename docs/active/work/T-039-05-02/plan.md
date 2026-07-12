# Plan: atomic publication boundary

## 1. Baseline ownership and tests

Record status and confirm the ordinary index is empty. Confirm `lib.rs` is clean
and `publication.rs` absent. Run:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
```

## 2. Create the publication module

Implement the finite temporary-name variants, same-parent path resolver, named
error labels, Rust writer/renamer with cleanup, shell renderer, and canonical
quoting. Document that provenance append and Git refs are outside this boundary.

## 3. Route fresh launch

Retain directory creation, destination, serialization, host stripping, and
bounded command result. Delegate temp naming, write, rename, and two errors.

Verify with focused fresh-launch tests.

## 4. Route assignment

Retain directory creation, raw bytes, destination, and returned path. Delegate
through hidden nonce prefix `.assignment.md.tmp.` and assignment labels.

Verify with focused assignment tests.

## 5. Route lease marker

Retain signal configuration, directory creation, compact JSON, and serialization
error. Delegate through attempt-bearing naming and marker labels.

## 6. Route admitted artifacts

Retain exact lease validation, staged file checks/read, directory creation, and
boolean results. Delegate through the exact deterministic temporary filename.

## 7. Route shell readiness

Retain compact JSON and host stripping. Delegate nonce resolution, quoting, and
the exact `printf && mv` command through the shell-side typed option.

## 8. Format and inspect

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
git diff -- crates/lisa-plugin/src/publication.rs crates/lisa-plugin/src/lib.rs
git diff --check
```

Verify all five sites route through the boundary; predecessor tests, schemas,
authority, directory policy, provenance, and CLI remain unchanged.

## 9. Focused behavior verification

Run:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically --no-fail-fast
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
cargo test -p lisa-cli commit_transaction --no-fail-fast
```

The publication tests must pass unchanged. Provenance must retain append
integrity. CLI fixtures must retain ordinary-index and completion safety.

## 10. Broad gates

Run:

```text
cargo test -p lisa-plugin --no-fail-fast
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings
just check
```

All executed tests and gates must pass.

## 11. Record progress

Write attempt-private `progress.md` with baseline, implementation mapping,
unchanged contracts, all verification results, deviations, and residue status.

## 12. Commit through Lisa

Use only:

```text
lisa commit-ticket --ticket-id T-039-05-02 \
  --message "refactor(plugin): centralize atomic publication" \
  --include crates/lisa-plugin/src/publication.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use ordinary staging or commit commands.

## 13. Post-commit verification

Inspect the HEAD stat, ordinary staged paths, and worktree status. Confirm the
commit contains exactly both ticket-owned paths and those paths are clean. Rerun
the two publication characterization tests from committed source.

## 14. Review

Write attempt-private `review.md` with source inventory, commit, typed boundary,
per-site preservation, seam tests, broad gates, residue assessment, and open
concerns. Then remain on this ticket for Lisa's completion transaction.

