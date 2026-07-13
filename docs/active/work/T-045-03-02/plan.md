# Plan — T-045-03-02 evidence tiers: hook and artifact

## Goal

Make scheduler ownership evidence explicitly tiered without broadening the
assignment state machine.

The exact claim remains primary.

A matching hook remains a faster supplemental transition while the claim is
pending.

An admitted current-attempt private workflow artifact becomes the bounded
fallback.

Predecessor hook and artifact evidence remain unable to own a replacement.

## Non-goals

Do not add the delivered-awaiting-claim state.

Do not change prompt retry or timeout resolution.

Do not change launcher commands or assignment-file format.

Do not change the CLI claim command or shared claim schema.

Do not force Claude through the Codex claim handshake.

Do not add dashboard labels as a substitute for scheduler state.

## Step 1 — establish the focused baseline

Run the existing primary-evidence test:

```text
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
```

Run existing stale-attempt artifact coverage:

```text
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact
```

Run the matching-ack state-machine test if its name filter is stable.

Record baseline outcomes in `progress.md`.

Verification:

- exact claim owns with no hook;
- predecessor artifact cannot publish;
- the base checkout is green before modification.

## Step 2 — add artifact ownership admission

Edit `crates/lisa-plugin/src/lib.rs` beside claim and hook admission.

Add a private method taking:

- ticket ID;
- exact `AttemptLease` candidate.

Return `Option<u32>`.

Implement fail-closed checks in this order:

1. candidate ticket matches the requested ticket;
2. candidate is current;
3. matching reserved slot exists;
4. slot carries the exact candidate lease;
5. pane has an active delivered generation;
6. generation equals candidate attempt ID.

On success:

- insert `SeatAssignmentState::Owned`;
- return pane ID.

Verification:

- the method cannot own startup, ready, already-owned, or failure states;
- it cannot own from a predecessor candidate;
- it cannot own a different pane's ticket.

## Step 3 — add post-admission effects

Add a small wrapper that receives:

- ticket ID;
- optional source lease;
- artifact name.

For a leased transition returned by Step 2:

- bump pane and thread activity;
- log an `ActivityEvent::Info` naming pane, ticket, attempt, and artifact.

For no lease or rejected/redundant evidence:

- do nothing;
- do not bump activity;
- do not log a success.

Verification:

- unleased legacy artifact fixtures preserve their old phase behavior;
- duplicate recognized artifacts do not create duplicate ownership events.

## Step 4 — wire living Implement progress

Refactor the Implement `progress.md` admission result in
`check_artifact_advances`.

Behavior by result:

- `Ok(true)`: call the post-admission ownership wrapper;
- `Ok(false)`: continue without action;
- `Err`: preserve the existing rejection log.

Do not set `advanced_any`.

Do not change the current phase.

Verification:

- progress remains durable and publishable;
- progress can be bounded fallback ownership evidence for a leased pending
  attempt;
- progress still cannot advance Implement.

## Step 5 — wire phase-edge artifacts

Expand the main `admit_artifact` `Ok(true)` arm.

Call the post-admission ownership wrapper before next-phase processing.

Leave false and error behavior unchanged.

Leave all phase updates and completion dispatch unchanged.

Verification:

- exact current output owns before phase update;
- stale admission errors do not own;
- missing artifacts do not own;
- successful unleased compatibility admission does not invent a pane owner.

## Step 6 — clarify poll hierarchy comments

Inspect the comments around:

- `check_claim_signals`;
- `check_codex_ack_signals`;
- `check_artifact_advances`.

Update only comments necessary to document:

- claim is authoritative and first;
- matching hook is supplemental fast evidence;
- admitted artifact is fallback and last;
- all run before timeout policy.

Do not reorder calls unless the checkout differs from Research.

Verification:

- production order remains claim, hook, artifact;
- timeout processing remains later.

## Step 7 — add matching-hook acceleration regression

Add a scheduler test near existing Codex ownership tests.

Arrange:

1. create scheduled Codex state;
2. create signal directory;
3. schedule ticket;
4. deliver current assignment;
5. verify active `Delivering` and no `.claim` file;
6. write matching `pane-10.ack` payload;
7. run only the hook consumer.

Assert:

- pending seat was not owned before the hook;
- hook file is consumed;
- claim file remains absent;
- seat becomes exactly `Owned`;
- acknowledgment activity is present.

Verification command:

```text
cargo test -p lisa-plugin matching_hook_accelerates_pending_claim_ownership
```

## Step 8 — add stale evidence and current artifact fallback regression

Add a second scheduler test using a predecessor and replacement lease.

Arrange predecessor:

1. schedule first attempt;
2. deliver it so the state path is realistic;
3. release its slot and remove its thread;
4. clear cooldown/activity enough for redispatch;
5. schedule replacement and assert monotonic attempt ID;
6. deliver replacement to pending state.

Inject stale evidence:

1. write predecessor-generation tagged hook to the replacement pane;
2. write predecessor `research.md` under its private attempt directory;
3. run hook consumer;
4. run artifact checker.

Assert:

- stale hook was consumed;
- stale hook did not own or bump replacement activity;
- stale artifact did not publish or advance;
- direct predecessor artifact admission returns an error;
- replacement remains `Delivering`.

Inject current fallback:

1. write different `research.md` bytes under replacement private directory;
2. run artifact checker.

Assert:

- replacement becomes `Owned`;
- Research advances to Design;
- canonical bytes are replacement bytes;
- predecessor bytes remain private;
- activity was bumped;
- fallback log names the current artifact and attempt.

Verification command:

```text
cargo test -p lisa-plugin current_artifact_is_bounded_fallback_and_stale_evidence_is_ignored
```

## Step 9 — format and inspect

Run:

```text
cargo fmt --all -- --check
```

If formatting is required, run the formatter and inspect the exact diff.

Run:

```text
git diff -- crates/lisa-plugin/src/lib.rs
```

Verification:

- no unrelated reformatting;
- no signal or shared schema changes;
- only the planned source unit is modified.

## Step 10 — focused regression suite

Run the two new tests.

Run the existing claim-only test.

Run focused filters for:

- `claim`;
- `ack`;
- `artifact_advances`;
- `stale_attempt`.

Verification:

- primary claim remains sufficient without hook;
- supplemental hook remains sufficient before claim arrival;
- fallback current artifact owns;
- stale evidence remains inert;
- phase semantics stay intact.

## Step 11 — complete plugin suite

Run:

```text
cargo test -p lisa-plugin
```

This covers the full scheduler state machine, UI snapshots, signal ingestion,
timeouts, completion publication, and split-brain regressions.

If a failure is caused by the ticket change:

- diagnose it;
- update the design deviation in `progress.md` before changing course;
- fix within ticket scope;
- rerun the focused and full plugin suites.

## Step 12 — workspace verification

Run:

```text
cargo test --workspace
```

This protects core claim behavior and CLI claim production in addition to the
plugin.

Run `cargo fmt --all -- --check` again after the final edit.

Optionally run `just check` if time and environment permit, because it includes
the WASM check expected by repository guidance.

Record all commands and results in `progress.md`.

## Step 13 — write implementation progress

Create `progress.md` in the private attempt directory.

Record:

- baseline results;
- source methods added;
- artifact call sites changed;
- tests added;
- test command outcomes;
- deviations from this plan;
- remaining work.

Before committing, mark implementation complete only if tests pass.

## Step 14 — isolated ticket commit

Inspect ordinary-index state without changing it:

```text
git status --short
git diff -- crates/lisa-plugin/src/lib.rs
```

Commit the single meaningful source unit through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-045-03-02 \
  --message "feat(plugin): tier hook and artifact ownership evidence" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add` or ordinary `git commit`.

Do not include ticket, story, provenance, journal, or attempt-artifact paths.

Verification:

- command succeeds;
- source diff is durable in Git history;
- `crates/lisa-plugin/src/lib.rs` is neither staged nor modified afterwards;
- unrelated worktree entries are untouched.

## Step 15 — post-commit verification

Run a focused test after commit if the transaction changed HEAD while the
worktree remained shared.

Inspect:

```text
git show --stat --oneline HEAD
git status --short
```

Confirm the commit contains exactly the source unit intended by this ticket.

If `lisa commit-ticket` fails:

- do not fall back to ordinary Git commands;
- inspect its error;
- correct exact path ownership or source state;
- retry the Lisa transaction.

## Step 16 — Review

Write `review.md` in the private attempt directory.

Summarize:

- evidence hierarchy behavior;
- production method and call-site changes;
- test additions and results;
- compatibility with claim, Claude, stale fencing, phase advancement, and
  timeout paths;
- open concerns or limitations.

Write `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

only if source is committed, tests pass, and no ticket-owned source remains
dirty.

Otherwise write the exact blocking shape with a non-empty actionable reason.

## Completion criteria

- Exact claims remain primary and claim-only coverage passes.
- Matching hooks own a delivered pending seat before a claim arrives.
- Only successfully admitted recognized current-attempt artifacts provide
  fallback ownership.
- Predecessor hooks are consumed without owning or bumping current activity.
- Predecessor artifacts remain private and cannot advance or own.
- Current fallback output advances its phase and changes the real seat state to
  `Owned`.
- Claude behavior and timeout state names are unchanged.
- The source unit is committed through `lisa commit-ticket` with an exact path.
- Review artifacts exist in the private attempt directory.
