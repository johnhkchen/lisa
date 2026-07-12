# Plan: publication-site characterization

## 1. Confirm baseline and ownership

- Inspect `git status --short`.
- Confirm existing changes are Lisa-managed ticket/provenance state.
- Confirm both ticket-owned source files are clean and unstaged.
- Run the focused existing publication and provenance tests.

Commands:

```text
cargo test -p lisa-plugin prepare_ --no-fail-fast
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically --no-fail-fast
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
```

Verification: all pass on the pre-edit production tree.

## 2. Add test-local publication inspection helpers

In the plugin inline test module, add only the minimal helpers needed to:

- list and sort entry names;
- validate fixed prefixes plus numeric nonce suffixes.

Verification:

- helpers are under `#[cfg(test)]`;
- no production visibility or function changes;
- helpers report useful assertion context.

## 3. Add success/collision characterization

Add:

```text
publication_sites_preserve_serialization_and_collision_contracts
```

Cover all five rename sites:

- fresh launch;
- assignment;
- lease marker;
- admitted artifact;
- shell readiness.

For each site assert:

- exact destination naming;
- distinct serialization;
- existing regular destination replacement;
- hostile path handling;
- success-path temporary cleanup.

Additionally assert:

- bounded fresh-launch command;
- exact assignment return path;
- exact lease round trip;
- staged artifact source remains unchanged;
- shell quoting prevents an injection sentinel.

Focused verification:

```text
cargo test -p lisa-plugin publication_sites_preserve_serialization_and_collision_contracts --no-fail-fast
```

## 4. Add hostile failure characterization

Add:

```text
publication_sites_preserve_temp_names_cleanup_and_operator_errors
```

Use overlong leaf-name fixtures to expose nonce-bearing temporary paths for:

- launch;
- assignment;
- lease marker.

Use destination directories to force rename failures and assert:

- operator-facing prefix;
- final destination rendering;
- Rust-side temp cleanup.

Use a directory at the deterministic admitted-artifact temp path to assert:

- exact temp naming;
- write-error prefix;
- canonical destination integrity.

Use a directory at the shell-ready destination to assert:

- nonzero shell status;
- current residual-temp behavior;
- pane/attempt temp-name identity;
- exact serialized lease in the residual temp.

Focused verification:

```text
cargo test -p lisa-plugin publication_sites_preserve_temp_names_cleanup_and_operator_errors --no-fail-fast
```

If the filename-length fixture reveals filesystem-specific limits different from
the expected Unix target, preserve behavioral assertions with the narrowest
portable hostile shape and document the deviation.

## 5. Add core provenance integrity regression

In `lisa-core/src/provenance.rs`, add:

```text
append_serialization_failure_preserves_existing_ledger
```

Steps:

- seed one valid ledger line under a hostile path;
- produce a non-finite cost value that JSON cannot encode;
- call `append_record`;
- assert `InvalidData`;
- assert preexisting bytes remain exactly unchanged.

Focused verification:

```text
cargo test -p lisa-core append_serialization_failure_preserves_existing_ledger --no-fail-fast
```

## 6. Add plugin provenance operator regression

Near existing plugin provenance tests, add:

```text
provenance_append_failure_is_logged_without_mutating_target
```

Steps:

- configure a hostile ledger destination occupied by a directory;
- construct a current leased thread;
- call `emit_provenance`;
- assert false return;
- assert target directory remains empty;
- assert the stable Error activity prefix and ticket identity;
- assert thread/lease state is untouched.

Focused verification:

```text
cargo test -p lisa-plugin provenance_append_failure_is_logged_without_mutating_target --no-fail-fast
```

## 7. Format and inspect the source diff

Run:

```text
cargo fmt --all
cargo fmt --all -- --check
git diff -- crates/lisa-plugin/src/lib.rs crates/lisa-core/src/provenance.rs
git diff --check
```

Review for:

- test-only changes;
- all five publication sites named in assertions/setup;
- no exact wall-clock nonce assumptions;
- no OS error-tail assumptions;
- no production behavior changes;
- no accidental formatting outside test additions.

## 8. Run focused publication and provenance groups

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
cargo test -p lisa-plugin provenance_ --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
```

Verification:

- new tests are discoverable by stable prefixes;
- existing provenance attribution tests remain green;
- core normal append and failed-serialization integrity both pass.

## 9. Run crate suites

```text
cargo test -p lisa-core --no-fail-fast
cargo test -p lisa-plugin --no-fail-fast
```

Verification: all executed tests pass; environment-gated ignores remain ignores.

## 10. Run workspace and lint gates

```text
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings
just check
```

Verification:

- workspace tests pass;
- test code is warning-free;
- WASM check passes;
- repository-defined check passes.

## 11. Record implementation progress

Write attempt-private `progress.md` with:

- baseline results;
- exact tests added;
- per-site contract coverage;
- provenance coverage;
- all verification commands and counts;
- deviations and limitations;
- source ownership status.

Do not write to the shared active-work directory.

## 12. Commit the meaningful test unit

Use Lisa's isolated transaction only:

```text
lisa commit-ticket --ticket-id T-039-05-01 \
  --message "test: characterize atomic publication sites" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-core/src/provenance.rs
```

Do not use `git add`, `git add -A`, ordinary `git commit`, or a broad include.

Verification:

- command reports a commit hash;
- commit contains exactly the two source paths;
- both ticket-owned files are clean and unstaged;
- Lisa-managed worktree changes remain outside the source commit.

## 13. Post-commit verification

```text
git show --stat --oneline HEAD
git status --short
git diff --cached --name-only
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
```

Verification: the isolated commit is correct and focused regressions remain
green from committed source.

## 14. Complete Review

Write attempt-private `review.md` summarizing:

- source inventory;
- commit identity;
- behavior characterized per site;
- provenance integrity and operator failure coverage;
- test and lint results;
- platform assumptions;
- open concerns or TODOs.

After writing Review, remain on this ticket and stop. Lisa owns phase/status
transition, artifact publication, completion commit, and seat release.
