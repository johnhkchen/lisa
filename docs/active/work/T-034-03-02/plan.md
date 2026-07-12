# Plan: T-034-03-02 live proof and Claude parity

## Step 1: establish owned paths and baseline

1. Record parent `HEAD` and short Git status.
2. Confirm the prerequisite regression commit is an ancestor of `HEAD`.
3. Confirm `crates/lisa-plugin/src/lib.rs` contains the exact named test.
4. Create the ticket evidence directory.
5. Record installed tool paths and versions.

Verification:

- source revision is `0ffe40f...` or a descendant;
- parent ticket phase/status remain unchanged;
- unrelated dirty paths are identified and excluded.

## Step 2: build the fresh runtime

Run:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo build -p lisa-cli --release
```

The plugin build runs first so the CLI build script embeds current WASM bytes.

Copy `target/release/lisa` to a new temporary `bin/lisa` path.

Record build output, binary version, file metadata, and SHA-256 hashes.

Verification:

- both commands exit zero;
- fresh binary is executable;
- fresh binary exposes `commit-ticket` and `complete-ticket`;
- fresh binary is not `/opt/homebrew/bin/lisa`.

## Step 3: execute the committed split-brain harness

Run:

```text
cargo test -p lisa-plugin \
  split_brain_timeline_fences_old_attempt_and_admits_one_winner \
  -- --exact --nocapture
```

If the Rust test path prevents `--exact` matching, rerun using the accepted
filter while keeping the exact named test visible in output.

Capture output in `evidence/split-brain-test.txt`.

Verification:

- exactly one target regression runs and passes;
- no test is ignored;
- output is tied to the recorded source revision.

The committed assertions cover fence-before-reschedule, stale signal and
artifact rejection, no duplicate ownership, and one authoritative Done.

## Step 4: scaffold the isolated project

Create a unique temporary root with `bin/` and `repo/`.

Invoke the copied fresh binary:

```text
<tmp>/bin/lisa init --path <tmp>/repo
```

If project detection requires a package marker, create the minimal marker before
init.

Inspect generated Claude and Codex hook configuration.

Customize only fixture instructions and configuration needed for the proof.

Verification:

- both provider hook configs exist;
- all Lisa hook scripts are executable;
- `lisa validate --path <repo>` succeeds after tickets are added.

## Step 5: create matched provider tickets

Write two minimal ticket files with identical context and acceptance criteria.

T-LIVE-CODEX:

- `agent: codex`;
- `phase: research`;
- `depends_on: []`.

T-LIVE-CLAUDE:

- `agent: claude`;
- `phase: research`;
- `depends_on: [T-LIVE-CODEX]`.

The task is to write the six required evidence-only RDSPI artifacts, make no
source change, and stop after Review.

Set `max_threads = 1`, short but safe review timeout, and normal session timeout.

Verification:

- fresh Lisa validates the DAG;
- dry-run shows Codex ready and Claude blocked;
- both tickets resolve to the intended provider.

## Step 6: commit the fixture baseline

Initialize and configure the temporary Git repository if `lisa init` did not.

Commit all generated scaffold and ticket files as a baseline.

Record commit ID and tree listing.

Verification:

- fixture worktree is clean;
- neither ticket is Done;
- no work artifacts exist before launch.

Normal Git commands are allowed only inside this disposable fixture.

## Step 7: launch the fresh loop

Choose a unique Zellij session name.

Start from the fixture directory with a real PTY using the absolute fresh CLI:

```text
<tmp>/bin/lisa loop --path <repo> --max-threads 1 --client codex
```

Detach the client after the session is established, leaving the Zellij server
running.

Capture:

- `.lisa-layout.kdl`;
- Zellij session and pane listings;
- initial dashboard and provider pane output;
- generated content-hashed WASM path.

Verification:

- layout's `lisa_bin` equals the copied fresh binary;
- layout's WASM path exists;
- target and extracted WASM SHA-256 hashes match;
- at least one agent pane and one plugin pane are live;
- the parent session is not targeted.

## Step 8: observe Codex assignment and completion

Poll the fixture ticket, work directory, Git history, provenance, and panes at
short intervals.

Capture the Codex pane when its ticket assignment is visible.

Allow the provider to run without injecting manual completion state.

Success requires:

- Research through Review artifacts exist canonically;
- ticket frontmatter becomes `phase: done` and `status: done` only after Review;
- a completion commit contains the ticket and work directory;
- provenance identifies method Codex/provider OpenAI with outcome Done;
- the dependent Claude ticket becomes schedulable afterward.

If the ticket stalls, inspect hook and pane evidence before any action.

## Step 9: observe Claude parity

Capture the Claude pane when the dependent ticket is assigned.

Allow normal Claude `/clear`/stop hook behavior and artifact progression.

Success requires the same durable completion evidence as Codex:

- six canonical artifacts;
- commit-gated Done frontmatter;
- a completion commit;
- provenance identifies method Claude/provider Anthropic;
- no pending Codex generation acknowledgement is required for Claude.

Compare timing and state observations only at the contract level.

Do not claim byte-identical provider transport behavior.

## Step 10: capture final evidence

Before cleanup, write:

- final pane listing and selected screen dumps;
- final ticket frontmatter;
- artifact inventory and SHA-256 checksums;
- complete fixture Git graph with changed paths;
- final Git status;
- provenance ledger;
- remaining signal and attempt directory listing;
- final generated layout and all runtime hashes.

Assert:

- each fixture ticket has exactly one authoritative Done row;
- Codex completion precedes Claude assignment by DAG receipt;
- no canonical artifact is shared between ticket attempt paths;
- fixture worktree has no ticket-owned residue after completion.

## Step 11: terminate the isolated runtime

Kill only the uniquely named temporary Zellij session.

Confirm it disappears from `zellij list-sessions`.

Preserve the evidence copied into the parent ticket directory.

Remove the temporary project only after every evidence file is readable.

Do not alter or terminate the parent loop.

## Step 12: run regression verification

Run:

```text
cargo test -p lisa-plugin
cargo fmt --all -- --check
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- docs/active/work/T-034-03-02
```

No source diff is expected.

If broad tests expose unrelated dirty-path failures, document them precisely and
do not mutate unrelated files.

## Step 13: verify repository integrity

Inspect parent status for ticket-owned and unrelated paths separately.

Verify:

- no parent source path was modified by this ticket;
- the ordinary index has no ticket-owned entries;
- only T-034-03-02 work artifacts/evidence are new;
- ticket frontmatter phase/status are untouched.

Because there is no source change, do not invoke `commit-ticket` merely to commit
documentation that Lisa owns at final completion.

## Step 14: complete progress and review

Update `progress.md` with:

- build and test commands;
- hashes and revision;
- temporary session lifecycle;
- Codex and Claude observations;
- deviations, failures, or retries;
- final parent integrity state.

Write `review.md` with:

- outcome;
- evidence file map;
- acceptance mapping;
- test coverage;
- provider parity interpretation;
- open concerns and critical issues.

Stop after `review.md` is written.

Do not edit the ticket phase/status and do not start another ticket.

## Completion criteria

Implementation is complete when:

- the fresh release build is recorded;
- extracted loop WASM matches the newly built WASM;
- the committed split-brain regression passes;
- the isolated loop provides durable Codex assignment/completion evidence;
- the same loop provides durable unchanged Claude assignment/completion evidence;
- all evidence is retained under this ticket's work directory;
- broad verification passes or honest limitations are documented;
- no parent source or ticket frontmatter was changed;
- `progress.md` and `review.md` are complete.
