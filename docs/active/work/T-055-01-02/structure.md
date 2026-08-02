# Structure — T-055-01-02 already-sealed-is-sealed

## Files

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/commit_transaction.rs` | modified — convergence in the empty-diff branch, overlap-first ordering, message, outcome enum, unit tests |
| `crates/lisa-cli/tests/already_sealed_is_sealed.rs` | created — integration tests on the shared `SealFixture` |
| `crates/lisa-cli/tests/support/mod.rs` | modified only if the fixture needs a per-attempt key helper (see below) |

Nothing else. `lisa-core` is untouched (T-055-01-03's ground), `lisa-plugin` is untouched, no
public API of `lisa-cli` changes shape.

## `commit_transaction.rs` — internal shape

### New private items

```rust
/// What a transaction body did. `Converged` reports a commit this transaction
/// did not create, so nothing about HEAD may be rolled back on its behalf.
enum TransactionOutcome {
    Sealed(CommitTransactionResult),
    Converged(String),
}
```

```rust
/// The `Lisa-Completion-Key:` line prefix shared by every key for one ticket.
///
/// Derived from the rendered key so the hex encoding stays owned by
/// `CompletionGenerationId::fmt`. `None` when the rendering does not have the
/// expected `v1:<completion>:<attempt>:<generation>` shape, in which case the
/// caller falls back to exact-key matching.
fn completion_scope_prefix(key: &CompletionGenerationId) -> Option<String>
```

```rust
/// Scan history for a completion commit, newest first: `grep` narrows,
/// `predicate` over the full message body proves.
fn find_completion_commit(
    repo: &Repository,
    grep: &str,
    predicate: impl Fn(&str) -> bool,
) -> Result<Option<String>, CommitTransactionError>
```

```rust
/// The commit that already carries this ticket's seal, if any: the exact key
/// first, then any completion key for the same ticket.
fn discover_sealed_completion_commit(
    repo: &Repository,
    key: &CompletionGenerationId,
) -> Result<Option<String>, CommitTransactionError>
```

```rust
/// Ordinary-index entries at or beneath an include path.
fn ordinary_entries_within(
    ordinary: &[PathBuf],
    includes: &[PathBuf],
) -> Vec<String>
```

```rust
fn render_paths(paths: &[PathBuf]) -> String   // ", "-joined display strings
```

### Refactored existing items

- `discover_completion_commit(repo, key)` keeps its exact-key semantics and its call site in the
  pre-check, reimplemented over `find_completion_commit` (grep = the exact marker, predicate =
  exact line equality). No behaviour change; the unborn-HEAD short-circuit moves into
  `find_completion_commit` so both callers keep it.
- `run_transaction_body(repo, request, includes, alternate_index)` gains a fifth parameter
  `completion_key: Option<&CompletionGenerationId>` and returns
  `Result<TransactionOutcome, CommitTransactionError>`. Its successful commit path returns
  `TransactionOutcome::Sealed(..)` and is otherwise unchanged, including the overlap check, the
  compare-and-swap `update-ref`, the ordinary-index reconcile and the final snapshot equality
  check.
- `commit_ticket_with_key` passes the key through, and in the cleanup-failure branch matches on
  `Ok(TransactionOutcome::Sealed(result))` only — a `Converged` outcome with failed cleanup
  reports the cleanup error without touching HEAD. At the end it maps the outcome to the public
  `CommitTransactionResult`.

### The empty-diff branch, ordered

Replacing lines ~828–848:

```
committed_paths = staged_paths(alternate)

if committed_paths.is_empty() {
    within = ordinary_entries_within(&original_staged.paths, includes)
    if !within.is_empty()        -> Err(overlap message, listing `within`)
    if let Some(key)             -> if let Some(id) = discover_sealed_completion_commit(...)
                                        -> Ok(Converged(id))
    -> Err("ticket {id} has no changes in the requested include paths: {render_paths(includes)}")
}

// unchanged from here: overlap over committed_paths, write-tree, commit-tree, update-ref, …
```

Ordering rationale (from design): the refusal must survive convergence, and with an empty
`committed_paths` the existing overlap check cannot fire on its own.

The overlap message keeps its existing wording — `ticket {id} overlaps paths already staged in
the ordinary index: {list}` — so one message covers both branches and nothing new has to be
learned by an operator or a test.

## Module boundaries

- The transaction stays provider-neutral: it reads the key's *rendering*, never its fields'
  semantics, and never parses ticket frontmatter.
- `CompletionGenerationId` is used read-only, by `Display`. No new trait, no new method on it,
  no `lisa-core` edit.
- No new dependencies.

## Test structure

### Unit tests (in-file `mod tests`)

The existing `GitRepo` helper stays as the base. New helper on it:

```rust
fn complete(&self, ticket: &str, work_dir: &str, id: &str, message: &str,
            key: CompletionGenerationId) -> Result<CommitTransactionResult, CommitTransactionError>
```
— a thin `complete_ticket` wrapper so the three new tests read as scenarios rather than struct
literals.

1. `already_sealed_ticket_converges_under_a_later_key_and_unsealed_empty_diff_still_fails`
   — the ticket's both-directions fixture and its negative half.
2. `empty_diff_names_the_include_paths_it_staged_from`
   — criterion 5, asserted on the exact rendered path strings.
3. `ordinary_index_entry_under_an_include_path_refuses_even_when_sealed`
   — criterion 3. Construction: seal, then modify a work file, `git add` it, then restore the
   file's sealed bytes on disk. The alternate index (built from the worktree) sees no diff; the
   ordinary index still holds a differing entry, so `staged_snapshot` lists it.

The existing `repeated_completion_key_discovers_prior_commit_and_different_key_is_independent`
must keep passing unmodified — it is the guard on "a different key with real content still
commits independently". Its generation-2 leg has a non-empty diff and never enters the new
branch.

### Integration test — `crates/lisa-cli/tests/already_sealed_is_sealed.rs`

```
mod support;
use support::{dispatch_together, SealFixture, assert_no_guard_collision};
```

1. `replaying_any_of_four_concurrent_seals_converges_on_its_commit` (criterion 6, built on
   T-055-01-01's fixture): seal four tickets concurrently, record each commit id, then dispatch
   four replays together with the identical requests. Every replay is `Ok`, reports the id its
   first pass reported, contributes no commit (`head_commit_count` unchanged), and no guard
   collision appears (the fixture's own assertion, reused).
2. `a_later_attempts_key_converges_on_the_sealed_commit` — the field shape: seal at
   `generation 1`, replay at `generation 2` with nothing changed, expect the generation-1 commit
   id and no new commit. This is the case Option B of the design would still fail.
3. `an_unsealed_ticket_with_no_changes_still_fails` — a fifth ticket added to the fixture whose
   work directory is committed and untouched: `Err`, message names its include paths, HEAD
   unchanged.

`SealFixture` needs no change for (1) and (2): `complete_request(id, generation)` already varies
the generation and `completion_key` already exposes the key. For (3) the fixture seeds a dirty
`review.md` for every ticket it is constructed with, so the test commits that artifact into the
base with a plain `git add`/`git commit` through the fixture's own `git` helper before calling
`complete_ticket` — no fixture change needed either. `support/mod.rs` is therefore expected to
stay untouched; if a genuine gap appears, the addition is additive only (T-055-01-03 shares this
file).

## Ordering of changes

1. Message + include-path rendering (smallest, independently verifiable).
2. Outcome enum + threading the key into the body (mechanical, no behaviour change yet).
3. Convergence and the overlap-first ordering in the empty-diff branch.
4. Unit tests.
5. Integration tests.

Steps 1–3 each keep the workspace compiling and the existing tests green; step 3 is the only one
that changes observable behaviour.
