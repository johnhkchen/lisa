# T-056-01-02 — Design: the-check-can-see-the-project

## The decision, first

**Option (a): the check runs in the project root, and the snapshot is removed.**

`run_check` spawns `/bin/sh -c <check>` with `.current_dir(root)` — the same directory
`proposal.rs`, `triage_agent.rs`, and `loop_cmd.rs` already use, and the same directory the
operator was standing in when they ran the ask by hand. `ReadOnlySnapshot`, `snapshot_project`,
`git_visible_paths`, `copy_visible_path`, `copy_entry`, `copy_small_tree`, `should_skip`,
`set_tree_read_only`, `fingerprint_tree`, `collect_entries`, `path_bytes`, and
`os_string_from_bytes` are deleted with it, along with the `ChangedFiles` classification they
produced.

**What is traded, named plainly:** the check loses *isolation*. Today a check that writes a
relative path writes into a disposable copy and cannot touch the project; after this change, a
check that writes a relative path writes into the project. Lisa no longer prevents, and no longer
detects, a check that mutates the tree.

**What is kept:** the timeout and its process-group kill; `TMPDIR`/`TMP`/`TEMP` still point at a
disposable scratch directory, so a well-behaved check's temp files still land outside the project;
stdin is still `/dev/null`; output is still captured, capped, and sanitized; and the read-only
contract itself is unchanged — `docs/knowledge/rdspi-workflow.md` still says a check "must never
perform" the remedy. What changes is that the contract is now enforced by the reviewer who writes
the check rather than by a copy, which is where T-056-01-03 is chartered to put teeth
("Decide, and state, whether a check may write… so `build && verify` is rejected by the author
rather than by the operator").

## Why the snapshot cannot be repaired in place

The ticket's own framing — P2, "the gate's verdict corresponds to durable reality in the project,
which is the only reality the operator can act on" — is the argument. A snapshot is by
construction *not* durable reality. It is a copy taken to be different from the project (smaller,
frozen, read-only). Every difference between the copy and the project is a way for the gate's
verdict to be about something the operator cannot act on, and `--exclude-standard` is only the
first one anybody tripped over. Repairing the exclusion list keeps the category error and buys the
next false gate.

Concretely, the three candidate repairs each fail on the research:

**Adding `--ignored`.** The ticket already rules it out and `should_skip` independently confirms
why: `node_modules/` (and `target/`, and `.venv/`) make the copy unusably slow. A field project's
`dist/` is 709 files; its `node_modules/` is six figures. This is not a tuning problem.

**Option (b), materialise a declared list of gitignored paths.** Two failures. First, criterion 1's
fixture declares nothing — its disposition is a plain `check: test -f out/marker` — so the
*default* must already see `out/marker`, which a declared list by definition does not provide.
Second, the field check is `node scripts/check-touch.mjs`: a Node script needs `node_modules/` to
resolve its own imports, so the declaration that would make it work is the declaration that makes
the copy unusable. The mechanism cannot serve the case it was designed for.

**Option (c), let the disposition declare `check_cwd: project | snapshot`.** The same criterion-1
objection applies to the default, so `project` would have to be the default — at which point
`snapshot` is a second, less-tested execution mode kept alive for no demonstrated user, and the
schema grows a field that every future check has to reason about. That is the opposite of N4
("the smallest contract that removes the failure mode, not a rewrite of the parking system"). If a
real hermetic-check requirement ever appears, adding the field then is strictly easier than
removing it now.

## Why mutation detection goes too, rather than moving to the live tree

The obvious conservative move — keep `.current_dir(root)` but fingerprint the live tree before and
after, so `ChangedFiles` survives — is wrong here, and this is the least obvious part of the
design.

`run_world_rechecks` is fired asynchronously by the plugin scheduler
(`lisa-plugin/src/lib.rs:1761-1783`) at its ordinary cadence, whenever a world-owned park with a
check exists. That is *while other threads' agent sessions are editing the same tree*. `lisa
unblock` has the same exposure: an operator runs it with a loop session live. Between the two
fingerprints, unrelated writers will touch tracked files, `.lisa/provenance.jsonl` will gain rows,
and ticket frontmatter will flip.

A live-tree fingerprint cannot tell those writes from the check's. It would report another
writer's changes as *the check tried to change project files* — structurally the identical error
T-056-01-01 just removed from the message layer, where a project script's sentence was relayed as
Lisa's verdict. Shipping it would manufacture exactly the class of false gate this story exists to
close, and it would fire most often under the loop, unattended.

There is no portable way to attribute writes to one process tree: overlayfs is Linux-only,
`sandbox-exec` is macOS-only and deprecated, and chmod'ing the *real* project read-only for the
duration of a check would freeze every concurrent thread. So the honest options are "detect
dishonestly" or "do not detect". This design does not detect, and says so.

The cost is bounded and observable: a check that writes now writes, and Lisa neither stops nor
notices it. The mitigations are that the contract already forbids it, that a check is authored by
the same reviewer agent that already has unrestricted write access to the repository (so the
marginal capability is nil), and that T-056-01-03's record-time validation is the designed place
to catch it while the author is still on the ticket.

## What replaces the "declared rule" requirement

Criterion 3 asks that, if the snapshot is kept, what is and is not materialised is a declared rule
rather than an emergent property of `--exclude-standard`. The snapshot is not kept, so the rule
becomes a statement about the execution contract instead, recorded here and as a doc comment on
`run_check`:

> A check runs in the project root, as the operator's own shell would. It sees every file that is
> there — tracked, untracked, and gitignored alike — because that is the tree whose state the
> operator changed. It gets a null stdin, a disposable `TMPDIR`, a time budget, and no protection
> from its own writes. It must not write.

Nothing is filtered, so there is no list to drift. That is the point: the previous rule was a
filter nobody had written down, and the replacement is the absence of a filter.

The reviewer-facing wording of this contract in `docs/knowledge/rdspi-workflow.md` is an explicit
acceptance criterion of T-056-01-03 ("states the execution contract a reviewer is writing against:
the directory the check runs in, which files are present, whether writes are allowed, and the time
budget"), and that ticket also owns the write decision this design defers. Writing a partial
version of that paragraph now would collide with it on the same lines for no gain, so this ticket
puts the contract in the code and in this artifact and leaves the operator-facing doc to -03.

## What this does to the git / non-git split

It removes it. There is one path now: the check runs in `root`. `git_visible_paths` and
`copy_small_tree` were the two arms of a copy that no longer happens, so both are deleted.

That has a direct consequence for criterion 6, and it is a deviation worth stating rather than
hiding: **`copy_small_tree` does not survive to "keep a passing test"**, because keeping a
tree-copying function with no caller purely to have something to test would be dead code. The
criterion's substance — non-git projects still work, and a git project and a non-git project agree
about what a check sees — is met in a stronger form than a test of the fallback could give: the
two cases run identical code, and a test asserts the `out/marker` fixture reopens in both. The
criterion is written in the vocabulary of options (b)/(c); the ticket offered option (a), and its
own criterion 3 is conditioned on "if the snapshot is kept", so choosing (a) is not a way of
failing it.

Note the pre-existing asymmetry this collapses, from research §2: today `out/marker` is invisible
in a git project and visible in the same tree without `.git`. The two arms already disagreed. One
path is the fix for that too.

## Consequences for existing tests

Three existing tests assert properties of the snapshot rather than of the check contract, and they
change rather than break silently:

- `relative_write_never_reaches_live_project_and_cannot_pass` — its premise is isolation. Replaced
  by a test that pins the new truth: a check's relative *read* resolves against the project.
- `mutation_inside_disposable_state_is_detected_even_after_chmod` and
  `automatic_recheck_write_attempt_is_disposable_and_cannot_reopen` and
  `attempted_write_is_disposable_reported_plainly_and_does_not_reopen` — all assert
  `ChangedFiles`. The classification is gone; these are removed, and their removal is called out
  in the review as the visible face of the traded guarantee.
- `every_decline_header_is_distinct_and_names_the_way_through` — drops its `ChangedFiles` arm; the
  remaining three headers must still be distinct and still name `--override-check`.

`CheckResult::ChangedFiles` and `DECLINE_CHANGED_FILES` are deleted. `CheckOverrideOutcome::
ChangedFiles` in `lisa-core/src/provenance.rs` **stays** on the wire: it is a serialized enum, old
ledgers may contain it, and removing a variant would break reading them. `override_outcome` simply
stops producing it. That is an additive-compatible no-op for the ledger and keeps `SCHEMA_VERSION`
at 10.

## Rejected outright

- **chmod the real project read-only during the check.** Freezes every concurrent Lisa thread and
  can leave the repository unwritable if the process dies mid-check. Not viable.
- **`git worktree` or `git stash` based isolation.** A worktree contains exactly the tracked files
  — the same blindness to `dist/` that caused the bug, at higher cost.
- **A platform sandbox (`bwrap`, `sandbox-exec`, seccomp).** Portability, and a rewrite of the
  parking system's execution model, against N4. If isolation is ever genuinely required this is
  the direction, and it needs its own story.
- **Running the check in `root` but hiding gitignored files with a mount/overlay.** Linux-only,
  and it re-creates the bug.

## Risk register

| Risk | Severity | Handling |
| --- | --- | --- |
| A reviewer-authored check writes to the project | Medium | Contract forbids it; T-056-01-03 adds record-time validation; the authoring agent already has full write access, so no new capability |
| A check now sees `.lisa/`, `.git/`, secrets in ignored files | Low | It already ran as the operator's user with the whole filesystem reachable by absolute path; visibility of the cwd is not a new capability |
| A slow check now runs against a large real tree instead of a copy | None/positive | Removing the copy makes `run_check` strictly cheaper — the old path copied and hashed the whole visible tree twice before the check even started |
| `ChangedFiles` disappearing surprises someone reading the ledger | Low | Wire variant retained; the change is stated in review.md |
