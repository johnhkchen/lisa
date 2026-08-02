# Design — T-055-01-02 already-sealed-is-sealed

## The decision that matters: how wide is the convergence match?

Everything else follows from this. An empty include-path diff plus *what evidence* equals
"already sealed"?

### Option A — emptiness alone means done

Rejected outright, and the ticket, story and epic all name it as the failure mode ("would trade
a livelock for a lie", N3). An empty diff also happens when the operator passed the wrong
include paths, when the work directory was never written, when a path was typo'd. Calling any of
those a seal reports a commit that does not contain the work.

### Option B — exact key only: converge when a commit carrying *this* `CompletionGenerationId` is reachable

This is the narrowest honest reading of "this ticket's `Lisa-Completion-Key` for this
correlation".

The problem: **it is already implemented, and it did not save the field board.**
`commit_ticket_with_key` runs `discover_completion_commit` on the exact key before staging
anything (research §"What already converges"). A same-key replay never reaches the empty-diff
branch. So under Option B this ticket's code change is a no-op refactor, and step 4 of the field
trace still loops forever — because the key the loop retries with (`attempt_id: "1"`,
generation 1) is not the key that landed (the operator's generations 2 and 3, and the
`attempt_id: "operator"` track). Option B satisfies acceptance criteria 4 and 6 by accident,
via code that already exists, and fails the epic's stated outcome: "A ticket whose work is
already committed completes cleanly, reporting the commit that already carries its key rather
than refusing."

### Option C (chosen) — ticket-scoped key: converge when a commit carrying *any* completion key for this ticket is reachable, and only when the diff is empty

Convergence requires **both**:

1. `git add -A -- <includes>` into the alternate index produced no diff against HEAD, and
2. some commit reachable from HEAD carries a `Lisa-Completion-Key:` line whose completion
   aggregate — the hex-encoded ticket id, the part of the key before the attempt — is this
   request's ticket.

Condition 1 alone is never sufficient; condition 2 alone is never consulted (the existing
pre-check is a separate, unchanged mechanism). "The completion key is the evidence" is preserved
exactly: the key is what is read, emptiness is only what makes the question worth asking.

Why the wider scope cannot lie. Under condition 1, every byte of every include path is already
in HEAD's tree — this transaction has literally nothing it could commit. Under condition 2, this
ticket has been completion-sealed at a known commit. Reporting that commit is a true statement
about the repository, not an assumption: the seal happened, and nothing is outstanding. The only
way to make it false would be to match a *different* ticket's key, which the aggregate match
excludes, or to drop condition 1, which is Option A.

A second, free property makes it tighter still on the path that matters. `complete_ticket` runs
`update_ticket_done` **before** staging, so on the completion path an empty diff means the
*done* version of the ticket file is what is at HEAD. Convergence there is not merely "nothing
to commit" — it is "the done ticket and its work artifacts are in history".

Cost of Option C over Option B: a completion whose new attempt genuinely produced nothing new
converges instead of erroring. That is the desired behaviour, not a regression — there is
nothing to seal and the ticket is sealed.

Prefer the exact key when it is present: if a commit carrying this precise key exists, report
that one; otherwise report the most recent ticket-scoped completion commit. Deterministic
(`git log` order), and it keeps identical-argument replay reporting the same id it reported
before.

### Option D — require the ticket file at HEAD to parse as `status: done`

Extra evidence, rejected. It re-implements ticket parsing inside the transaction, couples the
provider-neutral transaction to frontmatter semantics, and buys nothing on the completion path
where `update_ticket_done` already guarantees it (above). On the `commit_ticket` path there is
no completion key at all, so the question never arises.

## Scope: which entry points converge

Only transactions that carry a completion key. `lisa commit-ticket` (key `None`) keeps the hard
error unchanged: mid-Implement an empty diff means the agent named the wrong path, and an
agent's unit commit has no idempotency contract to honour. This also means the plain path's
behaviour is untouched by this ticket except for the message improvement below.

## Where the check lives

Inside `run_transaction_body`, in the empty-diff branch — the only place that knows the diff was
empty. `run_transaction_body` therefore takes `completion_key: Option<&CompletionGenerationId>`.

The existing pre-check in `commit_ticket_with_key` stays exactly as it is. It is E-041's
duplicate-suppression promise (same key, *non-empty* diff, do not commit twice) and this ticket
must not change what it does. The two now cover the two directions the epic names: the pre-check
stops a duplicate commit, the body stops a spurious failure.

## Ordinary-index overlap: refuses first, convergent or not

The ticket is explicit that convergence must not weaken the overlap refusal. Today the overlap
check compares `committed_paths` against the ordinary staged snapshot — with an empty
`committed_paths` it can never fire, so a naive "converge on empty" would silently skip it for
exactly the repository state the ticket calls "a different situation again".

So the empty-diff branch checks overlap **first**, against the include paths rather than the
(empty) committed paths: any ordinary staged path that is equal to or beneath an include path
refuses, with the existing overlap message. Only then is convergence considered. Order:

```
empty diff → ordinary index holds an include path?  → refuse (overlap message)
           → completion key for this ticket at HEAD? → converge (report that commit)
           → otherwise                                → refuse (empty-diff message)
```

The non-empty path keeps its existing overlap check untouched.

## The refusal message

Acceptance criterion 5: the message names the paths actually staged. Nothing was staged, so what
it can honestly name is the set of include paths the transaction staged *from* — normalized,
repository-relative, the exact strings `git add` was given:

```
ticket T-002-05 has no changes in the requested include paths: docs/active/tickets/T-002-05.md, docs/active/work/T-002-05
```

The field operator was looking at a modified ticket file and reading "include paths" as prose;
with the list present they can see whether the file they are looking at is in it. The leading
clause is unchanged so existing operator familiarity and the `has no changes` assertion in the
current test still hold.

## Deriving the aggregate scope prefix

The key's encoding belongs to `lisa-core`, and `lisa-core/src/completion.rs` is T-055-01-03's
ground (E-055 names it) — this ticket must not edit it. So the prefix is derived from the
request's *own* key rather than re-implementing the hex encoding: render the key, split on `:`,
and keep `v1:<hex(completion)>:`. Two components are required; anything shorter means the
rendering changed shape, and the code falls back to exact-key matching rather than guessing.
This keeps a single source of truth for the encoding (`Display`) and no cross-crate edit.

## Result shape

`run_transaction_body` returns an outcome rather than a bare result:

```rust
enum TransactionOutcome {
    Sealed(CommitTransactionResult),   // HEAD advanced, paths committed
    Converged(String),                 // nothing to do; this commit already carries the seal
}
```

The caller maps `Converged(id)` to the same public shape the pre-check already returns
(`commit_id == previous_commit_id`, `committed_paths` empty) and — critically — skips the
post-cleanup `rollback_after_ref_advance`, which must never run for a commit this transaction
did not create. Without the distinction, a cleanup failure on a convergent run would issue
`update-ref HEAD <id> <id>` and a pathless `reset` against someone else's commit. The enum makes
that unrepresentable instead of relying on a field comparison.

## Rejected: reporting convergence as a distinct public result variant

Callers (`main.rs` prints `result.commit_id`; the plugin reads the same) treat a convergent
completion exactly as a fresh one — that is the point of criterion 1 ("the same as a fresh seal
would"). Adding a public variant would push a distinction into every caller for no behavioural
gain, and E-055's PRESERVE list keeps the journal record shape as E-042 left it. `committed_paths
== []` already distinguishes the two for anyone who cares.

## Test strategy (detail in plan.md)

- Unit, in `commit_transaction.rs`: both directions in one fixture — a sealed ticket replayed
  under a *different* attempt's key converges to the sealed commit with no new commit; an
  unsealed ticket with an empty diff still errors, and the error names the include paths. This
  is the ticket's negative fixture: Option A passes half of it and fails the other half.
- Unit: empty diff + an ordinary index holding an include path refuses with the overlap message
  even though the ticket is sealed (criterion 3).
- Integration, on T-055-01-01's `SealFixture`: four concurrent completions, then replay each —
  every replay converges to its own ticket's commit, commit count unchanged (criterion 6),
  plus one cross-key replay to prove the field shape and one never-sealed ticket that still
  fails.
