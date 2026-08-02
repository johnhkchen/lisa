# Review — T-055-01-02 already-sealed-is-sealed

An empty include-path diff is no longer automatically a failure. When a commit already carries
this ticket's completion key, the transaction reports that commit instead of refusing — which is
what closes the livelock the field run ended in.

## Changes

**Modified — `crates/lisa-cli/src/commit_transaction.rs`** (commits `267933c`, `6ca1152`)

| Item | What it does |
| --- | --- |
| `enum TransactionOutcome` | `Sealed(result)` vs `Converged(commit_id)`. Makes "roll back a commit we did not create" unrepresentable rather than guarded by a field comparison. |
| `run_transaction_body(..., completion_key)` | Fifth parameter; returns the outcome. Commit path unchanged. |
| `commit_ticket_with_key` | Passes the key through, maps `Converged` to `commit_id == previous_commit_id` with empty `committed_paths` (the shape the exact-key pre-check already returns), and skips the post-cleanup rollback for it. |
| Empty-diff branch | Overlap refusal → convergence → refusal. |
| `ordinary_entries_within` | Ordinary staged paths at or beneath an include path. |
| `render_paths` | `", "`-joined include paths for the refusal message. |
| `find_completion_commit` | Generalized history scan: `git log --fixed-strings --grep` narrows, a predicate over `%B` proves. Unborn HEAD short-circuits. |
| `discover_completion_commit` | Unchanged semantics (exact key), now built on the above. |
| `discover_sealed_completion_commit` | Exact key first, then any completion key for the same ticket. |
| `completion_scope_prefix` | `v1:<hex(ticket)>:` derived from the key's own `Display`; `None` on an unexpected shape, degrading to exact-key matching. |

**Created — `crates/lisa-cli/tests/already_sealed_is_sealed.rs`** (commit `0a41711`)
Three integration tests on T-055-01-01's shared `SealFixture`.

**Not touched:** `lisa-core` (T-055-01-03's ground), `lisa-plugin`, `tests/support/mod.rs`, the
guard, the marker file, the key format, the journal.

## The judgement call, stated plainly

The acceptance criterion says "a commit carrying this ticket's completion key **for this
correlation**". Read as *the exact `(ticket, attempt, generation)` key*, this ticket is a no-op:
`commit_ticket_with_key` has discovered the exact key before staging since `8482f95`, so an
identical replay already converged and never reached the empty-diff branch. Read that way, the
field board also stays stuck — the loop retried under `attempt_id: "1"` / generation 1 while the
commits that landed carried the operator's generations 2 and 3 and an `attempt_id: "operator"`
track (T-055-01-03 §Context, E-055 §field evidence).

So convergence matches the **completion aggregate** — the ticket — and never a different ticket.
It fires only when *both* hold:

1. staging the include paths into the alternate index produced no diff against HEAD, and
2. a commit reachable from HEAD carries a `Lisa-Completion-Key:` for this ticket.

Under (1) there is nothing this transaction could commit; under (2) the ticket is sealed at a
known commit. Reporting it is a true statement, not an inference. Emptiness alone is never the
evidence (N3) — it is only what makes the question worth asking. Reasoning in full in design.md
§Option C; if a reviewer wants the strict exact-key reading instead, the change is one line
(`discover_sealed_completion_commit` → `discover_completion_commit`) and the tests that then fail
are exactly the ones naming the field shape.

One property makes the completion path tighter than the rule requires: `complete_ticket` runs
`update_ticket_done` *before* staging, so an empty diff there means the **done** version of the
ticket file is already at HEAD. Convergence on that path is not "nothing to commit" but "the done
ticket and its work artifacts are in history".

`lisa commit-ticket` (no completion key) is unchanged except for the richer message: an
agent's mid-Implement empty diff still fails, as it should.

## Test coverage

| Criterion | Test |
| --- | --- |
| Empty diff + key at HEAD → success returning that commit | `already_sealed_ticket_converges_under_a_later_key_…` (unit); `a_later_generations_key_converges_on_the_sealed_commit`, `replaying_any_of_four_concurrent_seals_…` (integration) |
| Empty diff + no such commit → still fails; both directions asserted | same unit test's second leg; `an_unsealed_ticket_with_nothing_to_commit_still_fails` |
| Ordinary-index overlap not weakened | `ordinary_index_entry_under_an_include_path_refuses_even_when_sealed` |
| Twice with identical arguments → one commit, two successes, second reports the first's id | `replaying_any_of_four_concurrent_seals_…` round 2; existing `repeated_completion_key_…` (unmodified) |
| Refusal names the staged paths | `empty_diff_names_the_include_paths_it_staged_from` |
| Built on T-055-01-01's fixture; replay after four concurrent completions converges | `replaying_any_of_four_concurrent_seals_…` |
| `just check` green | exit code 0 |

**Mutation check.** With the convergence branch disabled, the two tests that name this ticket's
behaviour fail and the rest pass — including the identical-argument replay, which the older
pre-check already handled. That is why the concurrent test replays a second time under
generation 2: without it, criterion 6 would have been "proven" by code this ticket did not write.

The negative fixtures need tickets seeded *already done*, because `update_ticket_done` otherwise
guarantees a non-empty diff. That is a real constraint on any future test in this area and is
recorded in the helper's doc comment (`GitRepo::seed_done_ticket`).

**Gap:** no test drives the `completion_scope_prefix` → `None` fallback, because
`CompletionGenerationId::Display` cannot currently produce a short rendering. It is defence
against a future format change, and E-055's PRESERVE list says that format is not changing.

## Open concerns

1. **Scope of convergence is the reviewable decision**, and it is deliberately wider than the
   narrowest reading of the criterion. Stated above and in design.md rather than buried.
2. **A non-empty diff under an already-used exact key still converges** via the pre-check, which
   means new artifact content written after a seal is silently not committed. That is
   pre-existing E-041 behaviour, untouched here, and it is the opposite direction from this
   ticket's subject — but it is the other half of the same question and may be worth its own
   ticket.
3. **The livelock is closed at this boundary only.** The loop still re-attempts `MarkDoneKey` on
   every start; it now succeeds instead of failing, so the cycle terminates. Bounding the
   re-attempt itself, the journal transition for a convergent completion, and the disposition
   that presented a command's error text as a review verdict are T-055-01-03's, as the story
   splits them.
4. **`ordinary_entries_within` uses `Path::starts_with`**, component-wise, so `docs/active/work`
   does not match `docs/active/workspace`. Correct, but it means include paths are compared as
   path prefixes and not string prefixes — worth knowing if the include normalization ever
   changes.
