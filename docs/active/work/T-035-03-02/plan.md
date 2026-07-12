# Plan: T-035-03-02 fresh-loop live startup harness

## Implementation strategy

Implement the reusable source first, validate its non-metered structure, commit the
ticket-owned unit through Lisa, then execute the authorized live run from the committed
harness and retain results in the attempt-private work directory.

The live execution is deliberately last because it is metered and should consume provider
turns only after syntax, build, deterministic Zellij behavior, and cleanup are verified.

## Step 1: create progress tracking

Create `.lisa/attempts/T-035-03-02/1/work/progress.md`.

Record the completed Research, Design, Structure, and Plan artifacts.

List the two intended ticket-owned source paths and the live evidence path.

Verification:

- artifact exists only under the attempt-private directory;
- ticket frontmatter is unchanged by the agent.

## Step 2: implement strict harness skeleton

Create `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`.

Add strict mode, repository-root resolution, dependency checks, configurable evidence
root, source/tool identity recording, cleanup trap, and stable failure/PASS receipts.

Ensure the script refuses unsafe combinations such as `SKIP_BUILD=1` without `LISA_BIN`.

Verification:

- `bash -n` passes;
- `shellcheck` runs if installed, with reviewed intentional exceptions;
- a missing dependency/invalid binary fails before fixture creation;
- source paths resolve correctly from any cwd.

## Step 3: add fresh build and deterministic preflight

Implement WASM-first release build and CLI release build.

Record canonical executable, version, source HEAD, CLI hash, and target WASM hash.

Invoke the existing ignored real-Zellij delivery-boundary test before live providers.

Allow documented debugging overrides without weakening the canonical ticket invocation.

Verification:

- release WASM builds for `wasm32-wasip1`;
- release CLI builds after WASM and reports the expected version;
- deterministic ignored test prints its stable PASS receipt;
- command outputs and exit codes are retained.

## Step 4: implement isolated fixture construction

Add a provider-parameterized fixture builder.

For each provider:

1. create a case directory under evidence;
2. run the fresh `lisa init`;
3. write a one-thread live configuration;
4. write one minimal story and provider-routed ticket;
5. install the named-session Zellij wrapper;
6. initialize and commit the disposable Git baseline;
7. record canonical root and baseline commit.

Use one stable ticket ID per provider and identical artifact-only acceptance text.

Verification:

- `lisa validate` passes in each fixture;
- fixture baseline is clean before loop launch;
- configured provider matches ticket frontmatter;
- canonical roots are distinct and outside the parent repository.

## Step 5: implement loop/session lifecycle

Generate a case-local PTY runner that unsets parent Zellij identity, preserves provider
authentication, prepends only the wrapper, and calls the exact fresh Lisa executable.

Launch through BSD or util-linux `script` as appropriate.

Discover the exact named session, Lisa plugin pane, and ticket-titled terminal.

Add bounded teardown for the sampler, session, loop process, and interrupt trap.

Verification:

- wrapper rejects unexpected invocation shapes;
- each case creates a different named Zellij session;
- no pane is added to the parent session;
- teardown leaves no case session running.

## Step 6: implement high-frequency evidence sampling

Sample dashboard and provider terminal at 250 ms intervals.

Append timestamped snapshots and first-seen state rows for:

- starting;
- ready-for-assignment;
- delivering;
- owned.

Continue sampling until durable fixture completion or a named timeout.

Retain final pane JSON and screens before teardown.

Verification:

- sampler files are nonempty once panes are discovered;
- state events are deduplicated by first occurrence;
- background sampler is always joined/stopped;
- timeout diagnostics include the last dashboard and terminal.

## Step 7: implement build, trust, and payload assertions

After each layout appears, copy it and parse the extracted WASM path.

Hash the extracted WASM and require equality with the release target WASM.

For Codex, compute `pwd -P`, inspect only the exact matching project table in active
Codex config, require `trust_level = "trusted"`, and write a narrow receipt.

Locate the attempt-private launch script and assignment document.

Require the launch script to contain the bare selected provider and lifecycle identity,
while rejecting ticket prose, assignment path, and attempt chat marker.

Require the assignment document to contain the ticket and workflow instructions.

Verification:

- mismatched hashes fail;
- alias and canonical Codex trust identities cannot be confused;
- full user Codex config is not copied to evidence;
- launch/payload checks write explicit PASS receipts.

## Step 8: implement state and completion assertions

Require ordered first occurrences of Starting, ReadyForAssignment, Delivering, and Owned.

Require a delivery activity row followed by a matching acknowledgement activity row.

Reject evidence containing `dquote>`, trust-choice UI, startup-failed, delivery-failed, or
recovery-failed.

Wait for Lisa—not the harness—to publish fixture ticket `status: done` and `phase: done`.

Require all six shared artifacts, a fixture completion commit, clean fixture source state,
and a matching provider Done provenance row.

Verification:

- state-order check fails on missing or reordered events;
- completion checks fail if only Owned is observed;
- harness never edits ticket frontmatter or manually publishes artifacts;
- both case receipts and final stable receipt are emitted only after all assertions.

## Step 9: write runbook

Create `docs/knowledge/fresh-loop-live-startup.md`.

Document:

- boundary and limitations;
- real-provider/metered warning;
- authentication and tool prerequisites;
- canonical invocation;
- supported debug overrides;
- state and evidence interpretation;
- failure handling and session cleanup;
- relationship to deterministic preflight.

Verification:

- every public harness variable is documented;
- command examples use repository-relative paths;
- runbook does not promise CI determinism or expose credentials.

## Step 10: non-metered source verification

Run:

```text
bash -n crates/lisa-cli/tests/fixtures/live_provider_startup.sh
cargo fmt --all -- --check
cargo test -p lisa-cli --test real_zellij_delivery_boundary --no-run
git diff --check -- <two exact ticket source paths>
```

Inspect the script for ordinary parent Git staging/commit commands; normal Git operations
must be visibly scoped inside disposable fixture roots.

Run a dry/preflight mode if implemented, stopping before provider launch.

Document all results in `progress.md`.

## Step 11: commit the meaningful source unit

Use exactly:

```text
lisa commit-ticket \
  --ticket-id T-035-03-02 \
  --message "test(cli): add live first-assignment harness" \
  --include crates/lisa-cli/tests/fixtures/live_provider_startup.sh \
  --include docs/knowledge/fresh-loop-live-startup.md
```

Do not use `git add`, `git add -A`, or ordinary `git commit` in the parent repository.

Verification:

- returned commit contains exactly the two include paths;
- both paths are clean after the transaction;
- neither path appears in the ordinary index;
- unrelated worktree changes are preserved.

## Step 12: execute authorized live validation

Invoke the committed harness with:

```text
EVIDENCE_DIR=.lisa/attempts/T-035-03-02/1/work/evidence \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

Do not set the build or deterministic-preflight skips.

Do not type into either Zellij session or accept any trust prompt.

Allow the harness to perform Codex-first and Claude-first cases sequentially.

If the harness reveals a source defect, document it before changing the plan, repair the
smallest ticket-owned harness surface, recommit that exact path through Lisa, and rerun.

Verification:

- final stable PASS receipt;
- both case PASS receipts;
- fresh build and deterministic preflight PASS;
- exact state sequence for both providers;
- matching build hashes and Codex trust identity;
- six artifacts and Done provenance for both fixtures;
- no remaining case Zellij sessions.

## Step 13: record live-run handoff

Create `.lisa/attempts/T-035-03-02/1/work/live-run.md`.

Summarize the immutable source commit, build hashes, versions, session/fixture identities,
state timelines, trust receipt, launch checks, acknowledgement ordering, completion
commits, and any deviations.

Reference evidence files using relative paths under the private work directory.

Avoid copying secrets or unnecessary full provider transcripts.

## Step 14: final regression and hygiene pass

Run proportionate final checks:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo fmt --all -- --check
git diff --check
```

Inspect parent `git status --short`, exact source commit contents, ordinary staged paths,
fixture session cleanup, and ticket-owned source cleanliness.

Update `progress.md` with actual commands/results and any justified deviations.

## Step 15: Review

Write `.lisa/attempts/T-035-03-02/1/work/review.md`.

Summarize source files, live evidence, acceptance mapping, test coverage, gaps, open
concerns, commit identity, and repository hygiene.

After `review.md`, remain on T-035-03-02 and stop. Do not edit ticket phase/status, publish
shared work, mark Done, release the seat, or begin another ticket.
