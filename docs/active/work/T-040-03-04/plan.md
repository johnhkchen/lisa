# Plan: authorized Codex field report

## Goal

Produce an evidence-complete live Codex report for the exact T-040-03-03 rebuild.
Exercise a blocking Review and a terminal pre-ownership failure in isolated fixtures.
Retain structured disposition, transitions, ledger, CLI reconstruction, Git state,
and cleanup proof without changing this repository's active tickets.

## Step 1: freeze the baseline

Record current HEAD and worktree status.
Classify Lisa-managed and pre-existing unrelated paths.
Do not stage, delete, or modify them.

Verify `target/release/lisa` exists and is executable.
Compute its hash and require the T-040-03-03 value.
Verify the release WASM file exists and matches its recorded value.

Do not build or rerun deterministic gates.

Verification:

- exact CLI hash match;
- exact WASM hash match;
- no source change introduced.

## Step 2: write the private harness

Create `live-field-harness.sh` with strict shell options.
Add dependency checks for Bash, Git, jq, Zellij, Codex, script, and shasum.
Require explicit `LISA_BIN`.

Add cleanup traps before any live session starts.
Track every fixture root, Codex home, session, and loop PID.

Verification:

- `bash -n` passes;
- `shellcheck` passes if installed;
- no project source path changes.

## Step 3: implement common fixture setup

Create external temporary roots.
Run pinned `lisa init` and write one-thread auto-advance configuration.
Write fixture-only stories and tickets.
Initialize and validate each disposable Git repository.

Create a Zellij wrapper that forces a unique session name.
Create an ephemeral Codex home with existing authentication symlink and hooks enabled.

Verification:

- fixture validates;
- fixture canonical path is outside this repository;
- baseline commit exists;
- no credentials are copied to retained evidence.

## Step 4: implement observation and build binding

Start `lisa loop` through a PTY.
Discover plugin and ticket panes through Zellij JSON.
Sample screens and named state transitions at short intervals.
Copy transient signal files once.

Copy the generated layout and extract its plugin path.
Require its SHA-256 to equal the T-040-03-03 WASM hash.
Require the layout to name the exact `LISA_BIN`.

Verification:

- named session becomes responsive;
- plugin and live Codex pane are visible;
- layout establishes CLI/WASM identity.

## Step 5: run blocking Review case

Launch `T-LIVE-BLOCK` on a real Codex seat.
Wait for `owned` and matching acknowledgement.
Wait for its valid blocking `review-disposition.json` and `review.md`.
Allow scheduler settling time after agent stop.

Assert:

- exact actionable block disposition;
- Review is retained and ticket is not Done;
- dependent has no attempt and remains non-Done;
- no authoritative Done row;
- no post-baseline completion commit;
- blocking reason is visible or retained in structured evidence.

Copy all case evidence out.
Stop and remove the fixture.
Record teardown assertions.

## Step 6: run pre-ownership failure case

Launch `T-LIVE-PREOWN` on a real Codex seat.
Continuously discover ticket panes.
Before any `owned` observation, close each newly observed live Codex pane.
Let production retry/recovery policy create any replacement.

Wait for one durable terminal assignment transition record.
Invoke pinned `lisa status --ticket T-LIVE-PREOWN` against the fixture ledger.

Assert:

- at least one live Codex pane existed;
- every close happened before ownership;
- `owned` never appeared;
- one terminal pre-ownership row exists;
- row fields identify ticket, attempt, pane, provider, state, reason, timestamps;
- CLI renders the same state and reason;
- no execution outcome or Done commit exists.

Copy all case evidence out.
Stop and remove the fixture.
Record teardown assertions.

## Step 7: audit cleanup

List Zellij sessions and require both names absent.
Require all fixture roots absent.
Require all ephemeral Codex homes absent.
Require no loop or sampler process remains.

Record the final cleanup ledger under each case.
Treat cleanup residue as a blocking anomaly.

## Step 8: evaluate raw evidence

Read every assertion receipt and the physical JSONL rows.
Compare observed chronology with the deterministic contracts.
Check for duplicates, contradictions, or missing transitions.

Inspect fixture Git logs and trees.
Confirm the blocking Review case did not create a completion commit.
Confirm the pre-ownership failure case did not fabricate an execution outcome.

Do not repair unexpected behavior.

## Step 9: write canonical field report

Write `progress.md` as a self-contained report.
Separate sections titled Deterministic Proof and Live Observation.
Include exact artifact hashes and fixture/session identities.

For each live case, include:

- chronology;
- structured evidence;
- state and outcome assertions;
- provenance and commit observations;
- teardown result.

Name every anomaly.
If any anomaly is unexplained, declare it BLOCKING Done.

## Step 10: repository hygiene verification

Run read-only status and diff checks for ticket-owned source.
Confirm the ordinary index contains no ticket-owned entry.
Confirm this ticket changed only attempt-private artifacts.

No `lisa commit-ticket` is expected because no source unit is planned.
If source modification appears necessary, stop and block instead of patching.

## Step 11: Review

Write `review.md` summarizing:

- attempt-private files created;
- exact build used;
- deterministic proof cited;
- both live outcomes;
- evidence coverage;
- cleanup;
- anomalies and limitations.

Write `review-disposition.json`.
Use pass only if both cases and teardown satisfy every assertion.
Otherwise use block with a non-empty actionable reason.

## Step 12: stop

Remain on T-040-03-04.
Do not update ticket phase or status.
Do not publish artifacts directly to shared work.
Do not start another ticket.
Wait for Lisa's completion decision and commit confirmation.
