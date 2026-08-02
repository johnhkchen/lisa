# Plan — T-055-01-02 already-sealed-is-sealed

Five steps. Each compiles and leaves `cargo test --workspace` green; only step 3 changes
observable behaviour. Commits go through `lisa commit-ticket` with exact `--include` paths.

## Step 1 — the refusal names its include paths

**Edit** `crates/lisa-cli/src/commit_transaction.rs`: add `render_paths(&[PathBuf]) -> String`;
extend the empty-diff error to
`ticket {id} has no changes in the requested include paths: {rendered}`.

**Verify** `cargo test -p lisa-cli` — the existing `error.contains("has no changes")` assertion
(~1608) still holds because the leading clause is unchanged.

**Commit** `crates/lisa-cli/src/commit_transaction.rs` — "name the paths the refusal staged
from".

## Step 2 — outcome enum and the key reaching the body

**Edit** the same file:

- `enum TransactionOutcome { Sealed(CommitTransactionResult), Converged(String) }`.
- `run_transaction_body(..., completion_key: Option<&CompletionGenerationId>) ->
  Result<TransactionOutcome, _>`; wrap its success in `Sealed`.
- `commit_ticket_with_key`: pass `completion_key` through; match `Ok(Sealed(result))` in the
  cleanup-failure rollback branch; map the outcome to `CommitTransactionResult` at the end
  (`Converged(id)` → `commit_id: id.clone(), previous_commit_id: id, committed_paths: vec![]`).

No behaviour change: `Converged` is not yet constructed.

**Verify** `cargo test -p lisa-cli`, `cargo clippy -p lisa-cli -- -D warnings`.

**Commit** same path — "carry the completion key into the transaction body".

## Step 3 — convergence, with the overlap refusal ahead of it

**Edit** the same file:

- `find_completion_commit(repo, grep, predicate)` — unborn-HEAD short circuit, `git log
  --format=%H --fixed-strings --grep <grep>`, verify each candidate's `%B` with `predicate`,
  return the first (newest) match.
- Reimplement `discover_completion_commit` over it with an exact-line predicate. Behaviour
  identical.
- `completion_scope_prefix(key) -> Option<String>`: render the key, split on `':'`, require at
  least `version` and `completion`, return `format!("{COMPLETION_KEY_PREFIX}{version}:{completion}:")`.
- `discover_sealed_completion_commit(repo, key)`: exact match first; else scope-prefix match
  (`grep` = the prefix, predicate = any line `starts_with(prefix)`).
- `ordinary_entries_within(ordinary, includes)`: ordinary paths equal to or beneath any include
  path, rendered.
- Empty-diff branch, in order: overlap refusal → convergence (`Converged`) → existing empty-diff
  refusal.

**Verify** `cargo test -p lisa-cli` (existing suite, especially
`repeated_completion_key_discovers_prior_commit_and_different_key_is_independent`), clippy.

**Commit** same path — "recognise a ticket already sealed under its completion key".

## Step 4 — unit tests, both directions

**Edit** the `mod tests` block; add the `GitRepo::complete` helper and three tests
(names and constructions in structure.md §Test structure):

1. `already_sealed_ticket_converges_under_a_later_key_and_unsealed_empty_diff_still_fails`
   - Seal `T-055-A` with key(attempt `"1"`, gen 1) → commit `first`.
   - Replay with key(attempt `"2"`, gen 1), nothing changed → `Ok`, `commit_id == first`,
     `committed_paths` empty, `rev-list --count HEAD` unchanged, HEAD unmoved.
   - Second ticket `T-055-B`, base-committed and untouched, never sealed → `Err` containing
     `has no changes` and both of its include paths; `rev-list --count HEAD` unchanged.
   - Assert the ticket file on disk is restored to its pre-`update_ticket_done` bytes on the
     failing leg (existing `complete_ticket` contract, cheap to keep honest).
   - **Negative-fixture property:** an implementation that returns success on emptiness alone
     fails the `T-055-B` leg; an implementation that only ever errors on emptiness fails the
     replay leg. Both halves are asserted in one test so neither can be satisfied alone.
2. `empty_diff_names_the_include_paths_it_staged_from` — assert the exact rendered strings
   `docs/active/tickets/T-055-C.md` and `docs/active/work/T-055-C` appear in the message.
3. `ordinary_index_entry_under_an_include_path_refuses_even_when_sealed` — seal, modify
   `<work_dir>/review.md`, `git add` it, restore the sealed bytes on disk, complete again with a
   later key → `Err` containing `overlaps paths already staged in the ordinary index` and the
   file's path; assert it is *not* the empty-diff message and that HEAD did not move.

**Verify** `cargo test -p lisa-cli`.

**Commit** same path — "prove both directions of the empty diff".

## Step 5 — integration on the shared concurrency fixture

**Create** `crates/lisa-cli/tests/already_sealed_is_sealed.rs` with the three tests from
structure.md. Reuse `mod support;` verbatim; expect no edit to `support/mod.rs`.

**Verify** `cargo test -p lisa-cli --test already_sealed_is_sealed`, then the full gate.

**Commit** `crates/lisa-cli/tests/already_sealed_is_sealed.rs` — "replay any of four concurrent
seals".

## Verification criteria, mapped to acceptance

| Criterion | Evidence |
| --- | --- |
| Empty diff + key at HEAD → success returning that commit id | unit 1 replay leg; integration 1 and 2 |
| Empty diff + no such commit → existing error; test asserts both directions | unit 1, both legs in one test |
| Ordinary-index overlap not weakened | unit 3 |
| Completion twice with identical arguments → one commit, two successes, second reports the first's id | integration 1 (identical requests); existing `repeated_completion_key_…` |
| Refusal names the paths actually staged | unit 2 |
| Built on T-055-01-01's fixture: replay after four concurrent completions converges | integration 1, on `SealFixture`/`dispatch_together` |
| `just check` green | final gate run, exit code recorded in progress.md |

## Risks and how each is handled

- **Over-wide convergence.** Bounded by requiring an empty diff *and* a ticket-scoped key match;
  unit 1's negative leg is the standing guard. Reviewed in design §Option C.
- **Regressing the generation-2-with-new-content case.** The existing
  `repeated_completion_key_…` test covers it and is not modified; its diff is non-empty so it
  never enters the new branch.
- **Key rendering drift.** `completion_scope_prefix` returns `None` on an unexpected shape and
  the code degrades to exact-key matching rather than matching a wrong prefix.
- **Rollback on a commit we did not create.** Prevented structurally by `TransactionOutcome`,
  not by a field comparison.
- **Shared-file collision with T-055-01-03.** Only `commit_transaction.rs` and a new test file
  are touched; `lisa-core/src/completion.rs`, `lisa-plugin/src/lib.rs` and `support/mod.rs` are
  left alone.

## Final gate

`just check` (wasm check, fmt, clippy on three crates, `cargo test --workspace`), judged by exit
code, not by reading output.
