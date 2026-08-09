# Plan — T-057-03-01, release-0-5-0-rc-1

Attempt 1's commits are on the branch. This attempt re-proves every acceptance
criterion against live state and then re-issues the handoff under a valid lease.
No new commit is planned unless verification finds something wrong.

## Steps, and what each one proves

1. **Version.** `cargo metadata --no-deps` reports `0.5.0-rc.1` for all three
   `lisa-*` packages; a local `cargo build -p lisa-cli` prints `lisa 0.5.0-rc.1`.
   → criterion 1.
2. **The compare.** Run the three named tests by exit code, individually, so a
   filter that silently matches nothing cannot pass as green.
   → criterion 2.
3. **Checklist.** `VERSION`, `PRIOR_STABLE`, `WORKFLOW_GATE`; all four gates
   present in both `for gate` loops; all four `git merge-base --is-ancestor` of
   HEAD; no prior gate deleted.
   → criterion 3.
4. **Cut record.** Read it end to end against the 0.4.4 record's shape; confirm
   it names all five breaking changes and the S-057-01 regression.
   → criterion 4.
5. **Baseline.** The checklist's four "Channel baseline" commands, live, plus
   local tag, remote tag, and the releases-by-tag API for `v0.5.0-rc.1`.
   → criterion 5.
6. **Gate.** `just check`, judged by exit code alone.
   → criterion 6.
7. **The boundary.** Nothing tagged, pushed, published, or dispatched. Confirmed
   by the absence of `v0.5.0-rc.1` in every place a tag or release could appear,
   and by `git status --short` showing no ticket-owned file staged, modified, or
   untracked.
   → criterion 7.
8. **The check.** Run it as written (expect a verdict of `1` today), run the
   identical command with `0.4.4` substituted (expect `0` — the only way to prove
   the passing path before the fact), time it, then `lisa check-disposition`.
   → criteria 8 and 7's `check` clause.
9. **This machine.** `which -a lisa`, `brew info lisa`, `ls /opt/homebrew/bin/lisa`
   — determine which installation owns PATH, and write step 4 of the handoff for
   *that* one.
   → criterion 9.

## Explicitly not doing

`just release`. Tagging. Pushing. `gh workflow run`. Editing `phase` or `status`
in the ticket frontmatter. Committing Lisa's board files. Filling the cut
record's `PENDING` values with guesses — they are a real handoff and only the
publisher can fill them.

## What would change the plan

Any of steps 1–9 failing turns this from a verification pass into an
implementation pass, and the fix would be committed through `lisa commit-ticket`
with exact `--include` paths before Review. Step 8's `check-disposition` refusal
would mean the check itself needs rewriting — and the ticket is right that this
is the last moment anyone can fix it, because the only later reader is an
operator standing at a refusal they cannot clear.

## Criterion 10 is not this ticket's to close

`brew install johnhkchen/lisa/lisa` yielding `0.5.0-rc.1` is story acceptance
*after the operator acts*. It is what the `check` watches for, and it is why the
disposition is a block rather than a pass.
