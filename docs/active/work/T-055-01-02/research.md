# Research — T-055-01-02 already-sealed-is-sealed

What exists today around the empty include-path diff, and what the field trace actually
exercised. Descriptive only.

## The transaction, end to end

`crates/lisa-cli/src/commit_transaction.rs` (1991 lines) owns the whole isolated Git
transaction. Two public entry points:

- `commit_ticket(request)` — the agent's `lisa commit-ticket`. Delegates to
  `commit_ticket_with_key(request, None)`. **No completion key.**
- `complete_ticket(request)` — Lisa's own completion. Rewrites the ticket frontmatter to done
  (`lisa_core::ticket::update_ticket_done`), builds `includes = [ticket_file, work_dir]`, then
  calls `commit_ticket_with_key(request, Some(&completion_key))`. On any error it restores the
  exact original ticket bytes.

`commit_ticket_with_key` (~1080–1170):

1. Validate ticket id / message, normalize includes.
2. `Repository::discover`.
3. `TransactionLock::acquire` — since T-055-01-01 this waits its turn with bounded backoff
   (`COMMIT_GUARD_WAIT_TIMEOUT` 30s, polls 5ms→50ms).
4. **Key pre-check**: when a completion key is present, `discover_completion_commit` greps
   history for the exact key marker. If found, it returns early with
   `CommitTransactionResult { commit_id: found, previous_commit_id: found, committed_paths: [] }`.
5. Otherwise reserve an alternate index and `run_transaction_body`.
6. Clean up index + guard; if cleanup fails after a successful commit, roll the ref back.

`run_transaction_body` (~795–929):

1. Refuse an unborn HEAD.
2. Resolve `old_head`, snapshot the **ordinary** index (`staged_snapshot` = `git diff --cached
   --name-only -z` plus `ls-files --stage` bytes for those paths).
3. `read-tree <old_head>` into the alternate index, then `git add -A -- <includes>` there.
4. `committed_paths = staged_paths(alternate)`. **If empty → hard error**
   (`ticket {id} has no changes in the requested include paths`), lines 828–834. This is the
   ticket's subject.
5. Ordinary-index overlap refusal (836–848): any `committed_paths` entry that is also in the
   ordinary staged snapshot aborts with its own message.
6. `write-tree`, `commit-tree -p old_head`, `update-ref HEAD new old` (compare-and-swap),
   `reset HEAD -- <committed_paths>` to reconcile the ordinary index, then verify the ordinary
   staged snapshot is byte-identical to the one taken in step 2.

## The completion key

`lisa_core::completion::CompletionGenerationId` = `(completion_id, attempt_id, generation)`.
`completion_id` is documented as "the completion aggregate identity, **populated with the ticket
ID**". Its `Display` is stable ASCII:

```
v1:<hex(completion_id)>:<hex(attempt_id)>:<generation>
```

It is written into the commit message as a trailer line by `completion_commit_message`:
`Lisa-Completion-Key: <key>`. `discover_completion_commit` finds it with
`git log --format=%H --fixed-strings --grep <marker>` and then re-verifies by exact line match
on `%B` — grep is the index, the line match is the proof. Unborn HEAD short-circuits to `None`.

Who mints the key:

- `crates/lisa-plugin/src/lib.rs::completion_correlation` (~2327) builds
  `CompletionGenerationId::new(completion_id, attempt_id, 1)` — **generation is hardcoded to 1**;
  `attempt_id` comes from the lease (`source_lease.attempt_id`, the attempt generation number).
  So across a *relaunched attempt* the key changes even though the ticket is the same.
- `crates/lisa-cli/src/main.rs` (~515) builds it from `--attempt-id` / `--completion-generation`,
  which is how the field operator produced generations 2 and 3 by hand.

So one ticket can accumulate several distinct completion keys, all sharing the
`v1:<hex(ticket)>:` scope prefix and differing after it.

## What the field trace actually did (E-055, T-055-01-03)

1. Two completions collide; the loser dies on the guard (fixed by T-055-01-01).
2. Reconciliation deadline passes → completion `rejected` / `action-required`.
3. `lisa unblock` returns the ticket to `review` — the phase that already failed.
4. Every loop start re-attempts `MarkDoneKey` → `complete-ticket` → empty diff → step 4's hard
   error → written back as a review `block` → parked → repeat, forever.
5. The operator's own `lisa complete-ticket` invocations **did** commit, at generations 2 and 3.

Both recorded completion tracks are `attempt_id: "1"` and `attempt_id: "operator"`. That is the
load-bearing detail: **the commit that carries the seal and the key the loop keeps retrying with
are not the same key.** An exact-key-only convergence rule converges the retry only when the
retry reuses the key that landed.

## What already converges, and what does not

Proven by the existing unit test
`repeated_completion_key_discovers_prior_commit_and_different_key_is_independent` (~1907):

- Same key, replayed → the pre-check returns the first commit id, no second commit, even with
  unrelated commits layered on top. **Already works.**
- A *different* key (generation 2) with genuinely changed content → commits independently.
  **Already works, and must keep working.** Note its diff is non-empty; it never reaches the
  empty-diff branch.

Not covered anywhere: a different key whose include-path diff is **empty**. That is the field
livelock, and today it is the hard error.

## Constraints and shared boundaries

- `staged_paths(repo, None)` uses `diff --cached`, so a `git add` of a file identical to HEAD
  leaves *no* ordinary staged path. To construct "empty alternate diff **and** an ordinary index
  holding an include path", the ordinary index must hold a version that differs from HEAD while
  the worktree matches HEAD — reachable by staging a modification and then restoring the file.
- `CommitTransactionResult.previous_commit_id` is private and drives the post-cleanup rollback.
  The pre-check's convergent result sets it equal to `commit_id`; the cleanup-failure branch in
  `commit_ticket_with_key` does not currently distinguish that shape, it simply never reaches it
  because the pre-check returns before an alternate index is ever reserved.
- The concurrency fixture from T-055-01-01 lives in `crates/lisa-cli/tests/support/mod.rs`
  (`SealFixture`, `dispatch_together`, `assert_no_guard_collision`) and is explicitly written to
  be shared. `SealFixture::complete_request(ticket, generation)` already takes a generation, and
  its `completion_key` fixes `attempt_id` at `"1"`.
- Story S-055-01 states the two repairs "share no file": T-055-01-03 owns
  `lisa-plugin/src/lib.rs`, the unblock/recovery route, and disposition provenance. This ticket
  owns `commit_transaction.rs` and its tests. `lisa-core/src/completion.rs` is named in E-055 as
  T-055-01-03's ground (the reducer's `Retryability`), so it is not this ticket's to edit.
- E-055 PRESERVE list: correlation ids, generation keys and the journal format stay as E-042
  left them. So the key's shape and the marker line are fixed; only the transaction's reading of
  them can move.
- N3/P2 honesty constraint, repeated in ticket, story and epic: **emptiness alone is never the
  evidence.** The key is.
- `just check` = `check-wasm` (cargo check for wasm32-wasip1) + `fmt-check` + clippy `-D
  warnings` on all three crates + `cargo test --workspace`.

## Open questions carried into Design

1. Scope of the convergence match: the exact key, or any completion key for this ticket? The
   ticket says "this ticket's `Lisa-Completion-Key` for this correlation"; the epic's "done looks
   like" says "reporting the commit that already carries **its** key". The field trace only
   recovers under the second reading.
2. Where the check belongs: the existing pre-check is key-only and runs before staging; the
   ticket's evidence rule is *empty diff* **and** *key present*, which is only knowable inside
   the body.
3. What the refusal message should name, given nothing was staged.
