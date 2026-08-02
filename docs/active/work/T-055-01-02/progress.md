# Progress — T-055-01-02 already-sealed-is-sealed

## Done

**Step 1–3 (combined) — `267933c` "recognise a ticket already sealed under its completion key"**
`crates/lisa-cli/src/commit_transaction.rs`:

- `TransactionOutcome { Sealed, Converged }`; `run_transaction_body` takes the completion key and
  returns the outcome; `commit_ticket_with_key` maps it and only rolls back a `Sealed` commit
  when cleanup fails.
- Empty-diff branch reordered: ordinary-index overlap refusal (now asked of the *include* paths
  via `ordinary_entries_within`, since an empty commit set makes the existing check vacuous) →
  convergence → the existing refusal, now naming the include paths through `render_paths`.
- `find_completion_commit(repo, grep, predicate)` generalizes the history scan;
  `discover_completion_commit` keeps exact-key semantics on top of it;
  `discover_sealed_completion_commit` tries the exact key first, then the ticket-scoped prefix
  from `completion_scope_prefix`.

**Deviation from plan:** steps 1, 2 and 3 landed as one commit instead of three. Splitting them
would have meant committing a `TransactionOutcome::Converged` variant that nothing constructs
(dead code, `-D warnings` on clippy) between commits. Each step was still written and verified in
the planned order; only the commit boundaries merged.

**Step 4 — `6ca1152` "prove both directions of the empty diff"**
Unit tests in the same file, plus `GitRepo::seed_ticket` / `seed_done_ticket` / `complete` /
`commit_count` helpers:

- `already_sealed_ticket_converges_under_a_later_key_and_unsealed_empty_diff_still_fails`
- `empty_diff_names_the_include_paths_it_staged_from`
- `ordinary_index_entry_under_an_include_path_refuses_even_when_sealed`

**Deviation:** the negative legs needed tickets seeded *already done* (`seed_done_ticket`).
`complete_ticket` runs `update_ticket_done` before staging, so a ticket seeded at `review` always
produces a non-empty diff and can never reach the empty-diff branch. Not anticipated in the plan;
it is also a useful property, recorded in review.md.

**Step 5 — `0a41711` "replay any of four concurrent seals"**
`crates/lisa-cli/tests/already_sealed_is_sealed.rs`, on T-055-01-01's `SealFixture` /
`dispatch_together` / `assert_no_guard_collision`. `support/mod.rs` needed no change, as
structure.md predicted. Three tests: concurrent seal → identical replay → later-generation
replay; the single-ticket field shape; and an unsealed ticket that still fails.

**Deviation:** the concurrent test gained a third dispatch round (generation 2). The mutation
check below showed the identical-argument round alone passes even with convergence disabled — the
pre-existing exact-key pre-check covers it — so that round proves criterion 4 but not this
ticket's change. The later-generation round is what proves the change under concurrency.

## Verification

- Mutation check: with the convergence branch disabled (`completion_key.filter(|_| false)`),
  `already_sealed_ticket_converges_under_a_later_key_…` and
  `a_later_generations_key_converges_on_the_sealed_commit` both FAIL; everything else passes.
  Restored afterwards. The new tests fail for the intended reason, not by construction.
- `just check` — **exit code 0** (wasm check, fmt, clippy `-D warnings` on all three crates,
  `cargo test --workspace`: 25 `test result: ok` blocks, 0 failures).
- `git status --porcelain` shows no ticket-owned source file staged, modified or untracked. The
  remaining entries (`.lisa/*`, unrelated `docs/active/epic|stories` files) predate this attempt.

## Remaining

Nothing. Review artifacts next.
