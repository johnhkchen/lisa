# Plan — T-045-05-02 field regression assertions

## Step 1 — establish implementation baseline

Record source HEAD and current status.

Confirm predecessor harness and runbook are clean.

Confirm the legacy executable and current build prerequisites remain available.

Do not alter unrelated dirty Lisa runtime or planning paths.

Verification:

- `git status --short` identifies only pre-existing unrelated paths;
- `git diff --` reports no pre-ticket changes for the two owned source files.

## Step 2 — expand fixture identity

Replace the one ticket constant with primary and successor constants.

Create both Review ticket files in every fixture.

Give the successor a dependency on the primary ticket.

Create canonical prior Review work for both.

Keep max threads at one.

Keep the generic claim-first AGENTS protocol unchanged in semantics.

Verification:

- preparation creates two valid tickets;
- `lisa validate` passes for legacy and current fixture roots;
- successor frontmatter names the primary dependency.

## Step 3 — make state evidence ticket-scoped

Extract one dashboard scheduler row per ticket.

Detect the known assignment state from that row.

Record every state change rather than only first observation.

Use a three-column timestamp/ticket/state TSV.

Adapt wait predicates to the new format.

Preserve the legacy named failure observation.

Verification:

- ShellCheck finds no word-splitting or subshell-state issue;
- a prepared fixture does not create live state files;
- live current evidence contains ordered states for both tickets.

## Step 4 — add live stale-claim probe

Trigger the probe on successor `delivered-awaiting-claim`.

Use the actual fixture root, pane ID, and current Lisa binary.

Name successor attempt zero and nonce zero.

Capture stdout, stderr, and numeric exit status.

Require nonzero and stable `[stale-attempt]` stderr.

Create the successor's claim gate only after rejection passes.

Keep the exact claim in real Codex's first tool action.

Verification:

- focused claim CLI tests pass;
- live stale receipt is nonzero with `stale-attempt`;
- no attempt-zero claim appears in captured signals;
- successor exact claim still reaches Owned.

## Step 5 — add process and lease transition ledgers

Reuse each sampler's one `ps` snapshot to avoid extra host scans.

Record each unique assignment-bearing launcher and Codex PID once.

Associate rows with the exact ticket assignment path.

Record pane lease present/absent identity transitions.

Retain existing raw snapshots and signal captures as diagnostic backing.

Verification:

- each current ticket has one launcher row and one Codex row;
- predecessor and successor PIDs differ;
- lease transitions include predecessor, absent, successor;
- captured lease bodies include both ticket identities.

## Step 6 — generalize evidence capture

Copy both final ticket files under `tickets-final/`.

Copy both attempt trees under ticket-keyed snapshot directories.

Copy both canonical work trees under ticket-keyed snapshot directories.

Retain all existing global runtime evidence.

Do not copy Codex authentication or complete Codex home configuration.

Verification:

- successful preparation still deletes temporary Codex homes;
- final evidence has no auth file;
- both ticket and work snapshots exist after live completion.

## Step 7 — implement per-ticket transport assertions

For each current ticket, require one assignment and one launch script.

Require the launch script to invoke current `launch-codex`.

Require the exact pane-relative assignment path.

Derive nonce from the assignment filename.

Require exactly one captured claim matching ticket, attempt, and nonce.

Require exact state transitions:

1. starting;
2. delivering;
3. delivered-awaiting-claim;
4. owned.

Require no current failure vocabulary.

Verification:

- the final live run passes both ticket assertions;
- deleting or duplicating a copied state row makes the offline assertion nonzero;
- a mismatched claim nonce makes the assertion nonzero.

## Step 8 — assert no duplicate injection

Use exact state transition cardinality as the scheduler proof.

Require one immutable assignment and one launch script.

Inspect each timestamped terminal screen independently.

Require no screen to show more than one tagged assignment line for the ticket.

Do not count the same visible line across repeated screen samples as repeated injection.

Verification:

- both live tickets pass;
- `delivering` appears exactly once per ticket;
- no retry state transition appears after passive wait.

## Step 9 — assert clean fresh-TUI boundary

Require distinct launcher PIDs across the two tickets.

Require distinct Codex PIDs across the two tickets.

Require exact assignment paths in their structured process rows.

Require predecessor lease, absent transition, and successor lease in order.

Require live stale successor rejection.

Require both exact lease bodies in captured signals.

Verification:

- focused deterministic completion boundary test passes;
- live evidence produces one clean-boundary receipt;
- missing absence or equal process identity blocks PASS.

## Step 10 — assert exact completion cardinality

Wait for both ticket files to reach Done.

For each ticket, require one requested journal row.

Require one command-in-flight row.

Require one confirmed row.

Require attempt one and generation one throughout.

Require a valid 40-hex confirmation commit.

Require one authoritative, unfenced Codex Done provenance row.

Require two relevant journal triples and two provenance rows total.

Verification:

- `jq -s` assertions pass over final live files;
- duplicate or missing rows fail;
- receipt states exactly-once completion for both tickets.

## Step 11 — update stable receipts

Retain the historical false-failure receipt.

Emit `current-slow-claim-no-reinjection: ASSERTED`.

Emit `current-stale-claim: REJECTED`.

Emit `current-fresh-tui-boundary: ASSERTED`.

Emit `current-exact-completions: ASSERTED`.

Print the final PASS line only after all assertions succeed.

Verification:

- result file contains the current granular receipts;
- command stdout ends with overall PASS;
- no current receipt exists on early failure.

## Step 12 — update the runbook

Describe the two-ticket current fixture.

Describe ticket-scoped transition evidence.

Describe the stale probe and claim-first ordering.

Describe process/lease ledgers and fresh-TUI proof.

Describe exact journal and provenance cardinalities.

Update evidence layout and expected output.

Remove the prior downstream-delegation language.

Retain metering, authorization, cleanup, and redaction warnings.

Verification:

- examples match actual environment variables and receipts;
- `git diff --check` passes;
- no text suggests stub evidence closes acceptance.

## Step 13 — run non-metered verification

Run:

```text
bash -n crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
shellcheck crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
git diff --check -- <harness> <runbook>
```

Run preparation with a ticket-private evidence directory.

Run focused CLI tests:

```text
cargo test -p lisa-cli --test codex_launcher --test claim_cli
```

Run focused plugin tests:

```text
cargo test -p lisa-plugin live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui
```

Verification:

- every command exits zero;
- preparation ends `PREPARED` without Codex or Zellij provider sessions;
- failures are fixed before the live run.

## Step 14 — run authorized field validation

Use the explicitly named pre-E-045 Lisa executable.

Use the current release Lisa built by preparation.

Use a new attempt-private live evidence directory.

Retain fixture roots for diagnosis.

Allow the harness to launch one legacy Codex turn and two current Review turns.

Do not answer interactive provider prompts.

Verification:

- legacy false failure is observed;
- both current tickets claim and complete;
- all four granular current receipts appear;
- overall PASS appears;
- any unexplained behavior remains a failed evidence set.

## Step 15 — assess field result

Inspect state transition rows for both current tickets.

Inspect stale claim stderr and exit status.

Inspect process and lease ledgers.

Inspect exact claim JSON bodies.

Inspect journal and provenance JSONL.

Inspect final fixture Git status and completion commits.

Document any harness correction before rerunning.

If behavior is unexplained after bounded diagnosis, stop with blocking Review disposition.

## Step 16 — commit the source unit

Run one isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-045-05-02 \
  --message "test(cli): assert live Codex ticket lifecycle" \
  --include crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh \
  --include docs/knowledge/live-codex-review-boundary.md
```

Do not include evidence or attempt artifacts.

Do not use ordinary `git add` or `git commit`.

Verification:

- the command prints a commit ID;
- `git show --check <commit>` passes;
- both ticket-owned source paths are clean;
- ordinary staged diff remains empty.

## Step 17 — complete Implement and Review artifacts

Update `progress.md` throughout implementation.

Record deviations before applying them.

Write `review.md` with source commit, assertions, field evidence, coverage, and concerns.

Write exact passing disposition only if every live assertion is explained and green:

```json
{"disposition":"pass","reason":null}
```

Otherwise write a block disposition with a non-empty actionable reason.

Remain on T-045-05-02 and stop after both Review artifacts.
