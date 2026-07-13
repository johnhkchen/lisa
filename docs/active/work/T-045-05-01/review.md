# Review — T-045-05-01 real Codex/Zellij field harness

## Disposition

Pass.

The ticket's acceptance criterion is satisfied.

A committed live harness now runs installed Codex inside installed Zellij with hooks disabled.

It starts both comparison subjects on the same existing-Review fixture shape.

The legacy subject reproduces the false delivery failure.

The current subject records native launcher spawn, exact claim, pane signals, ownership, Review
completion, and authoritative provenance without delivery failure.

The final authorized run completed with the stable `PASS` receipt.

No unexplained live behavior remains.

## Ticket source commit

The isolated ticket commit is:

```text
73241910fadb9f3f06193f7e052359cabc7277fc
test(cli): add live Codex Review boundary harness
```

It contains exactly:

- `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`;
- `docs/knowledge/live-codex-review-boundary.md`.

The commit was created with:

```text
lisa commit-ticket \
  --ticket-id T-045-05-01 \
  --message "test(cli): add live Codex Review boundary harness" \
  --include crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh \
  --include docs/knowledge/live-codex-review-boundary.md
```

No ordinary-index `git add` or ordinary repository commit was used for ticket source.

The committed diff is 863 insertions across two new files.

`git show --check 7324191` passes.

Both source paths are clean relative to HEAD.

The ordinary Git index is empty.

## New live harness

`live_codex_review_boundary.sh` is an explicit metered field command.

It is not registered as a default Cargo test.

It keeps the prior hooks-on provider-startup harness unchanged.

It keeps the deterministic stub-provider Zellij harness unchanged.

It requires real `codex`, `zellij`, `script`, shell, Git, build, and JSON tools.

It validates all booleans and time inputs before provider launch.

It requires an authenticated source Codex home.

It records binary versions and hashes.

It fails if legacy and current Lisa subjects have identical SHA-256 identities.

It rebuilds current release WASM and CLI unless the caller explicitly supplies and selects a
previously verified current binary.

## Fixture shape

Every case uses an external disposable Git repository.

The fixture contains one Codex ticket with:

- `status: open`;
- `phase: review`;
- prior canonical `review.md`;
- no product source change;
- no attempt-private output before startup.

This is the reusable T-014-03-01/T-015-02-01-style recovery shape.

The same story, ticket, Review evidence, scheduler config, and Codex version are used for old and
current subjects.

Only the selected Lisa binary/embedded WASM differs.

## Hooks-off and authentication boundary

The harness creates a separate ephemeral `CODEX_HOME` for each case.

It symlinks the operator's existing `auth.json` rather than copying credential content.

It writes `[features] hooks = false`.

It does not install `hooks.json` in that home.

It records a bounded runtime receipt with the false setting and hooks-file absence.

Cleanup removes every ephemeral Codex home and authentication symlink on success or failure.

Neither the source commit nor retained evidence contains authentication bytes.

## Real first-action claim protocol

The fixture `AGENTS.md` requires one exact first shell tool action.

The current branch discovers the exact nonce-bearing assignment in only its own attempt work
directory.

It derives the nonce from that immutable filename.

It waits inside the real Codex tool action for an attempt-private field gate.

The sampler creates that gate only after the real dashboard visibly enters
`delivered-awaiting-claim`.

One second later the same tool action invokes inherited `lisa claim`.

The resulting `.claim` is real CLI output, not a harness-generated signal.

The legacy branch has no nonce assignment and emits no claim.

Its first tool action remains delayed long enough for the old hook-dependent delivery windows to
expire.

The field gate is not a Lisa signal and production code does not know it exists.

No production timeout or scheduler state was changed.

## Evidence capture

The background sampler runs every 100 ms.

It captures dashboard and terminal snapshots.

It records first-seen state timestamps.

It copies distinct pane signal bodies before the plugin consumes them.

It records signal basename, size, digest, and capture location.

It captures case-specific Zellij, Lisa launcher, and Codex child process command lines.

It preserves final launch scripts, assignment files, ticket, work, attempt data, provenance,
completion journal, layout, pane manifest, Git log, status, and final screens.

Failure diagnostics are bounded and the selected evidence directory is always retained.

The default retains disposable fixture repositories for field diagnosis.

## Legacy live result

The final legacy state timeline is:

```text
2026-07-13T16:13:15Z starting
2026-07-13T16:13:25Z delivering
2026-07-13T16:13:44Z FAILED T-FIELD-REVIEW
```

Captured signals contain only `pane-0.lease`.

No `.started`, `.ack`, or `.claim` appeared.

No attempt-private `review.md` existed at failure observation.

The launch script directly invoked:

```text
codex --dangerously-bypass-approvals-and-sandbox --dangerously-bypass-hook-trust
```

It contained no `launch-codex` command and no nonce assignment path.

The live Codex process existed, but the old scheduler labeled the unacknowledged Review delivery
failed.

This is the ticket's requested false-failure reproduction.

The harness stopped the legacy session immediately after capturing it.

## Current live result

The final current state timeline is:

```text
2026-07-13T16:13:46Z starting
2026-07-13T16:13:56Z delivering
2026-07-13T16:14:06Z delivered-awaiting-claim
2026-07-13T16:14:11Z owned
```

The exact claim was copied at 16:14:07Z:

```json
{"ticket_id":"T-FIELD-REVIEW","attempt_id":1,"nonce":1783959226227998000}
```

The current launch script invoked the freshly built Lisa binary:

```text
launch-codex -- .lisa/attempts/T-FIELD-REVIEW/1/work/
assignment-1-1783959226227998000.md
```

The script contains the path on one line; the review wraps it only for readability.

Process snapshots show the Lisa `launch-codex` parent and real Codex child with the same assignment
argument.

No hook `.ack` or `.started` signal appeared.

No delivery-failed or claim-timed-out snapshot appeared.

The agent wrote passing current-attempt Review artifacts after claim ownership.

The ticket reached durable `status: done` and `phase: done`.

## Completion evidence

The current fixture completion journal contains exactly three records:

1. requested;
2. command-in-flight;
3. confirmed.

The confirmation names fixture commit:

`9b1cbf46553d3368b5e78596d2820bb9096c2a2b`.

The provenance ledger contains exactly one ticket row.

It records:

- ticket `T-FIELD-REVIEW`;
- attempt one;
- outcome Done;
- actual/requested method Codex;
- authoritative true;
- fenced false;
- pane zero;
- 55 seconds wall clock.

This is supporting harness evidence.

The dependent ticket owns the complete exact-once and fresh-successor assertion expansion.

## Binary and tool identity

The final run recorded:

- Codex CLI `0.144.3`;
- Zellij `0.44.3`;
- legacy Lisa `0.4.0-rc.8`;
- current Lisa `0.4.0-rc.8`.

Equal version text did not hide the binary boundary.

Recorded hashes were:

- legacy Lisa:
  `7d03785f7a59c730886d47fa33eec084d82245adc2aa766d1ff8dd58b9c52bf8`;
- current Lisa:
  `92997179e3af21e377abda0e2e155f24c94f969495770e8aa518137c2826c666`;
- current release WASM:
  `3f617aa61de7ee0ade3734aec1b92f9f903679039692006b45b196faf38424c2`.

## Stable live receipts

The final command ended zero with:

```text
legacy-false-delivery-failure: OBSERVED
current-claim-delivery: OBSERVED
live-codex-review-boundary: PASS
```

Final evidence is attempt-private at:

`.lisa/attempts/T-045-05-01/1/work/evidence-live-final/`.

The live fixture roots are listed in that directory's `fixture-roots.txt`.

The named Zellij sessions and ephemeral Codex homes were removed by cleanup.

## Calibration runs and explained failures

Three earlier evidence directories are retained intentionally.

`evidence-live/` demonstrated that a fixed 22-second sleep plus model pre-tool latency could put a
valid claim after the passive deadline.

`evidence-live-2/` demonstrated correct ownership and Done, then found a harness-only absolute vs
pane-relative path comparison error.

`evidence-live-3/` demonstrated that reducing the fixed delay still raced variable pre-tool
latency.

These observations led to the private evidence gate.

The final gate-based run is deterministic with respect to the required observed state rather than
wall-clock guessing.

The prior scheduler refusals were correct: a late valid native claim did not resurrect a terminal
seat.

No product defect was hidden or assertion weakened.

## Non-metered verification

Passed preparation:

```text
live-codex-review-boundary: PREPARED
```

Passed shell gates:

- `bash -n`;
- ShellCheck with no findings;
- `git diff --check`;
- post-commit `git show --check`.

Passed focused CLI tests:

- claim CLI: 3;
- hostile assignment-path launcher: 1.

Passed focused scheduler test:

- slow live Codex claim waits without reinjection and resolves actionably.

Passed focused completion test:

- claimed Codex completion exits, revokes, and launches the dependent ticket fresh.

No Rust production file changed, so a second full workspace suite after the shell-only final edits
would add no proportional coverage beyond the focused predecessor tests and successful live run.

## Open concerns and limitations

Live provider latency is observational and host-specific.

The field gate makes the required ordering reproducible without claiming provider timing is
deterministic.

Terminal and loop transcripts may contain more provider context than a reviewer needs; share the
bounded receipts and timelines preferentially.

The harness currently compares one Review ticket per subject.

T-045-05-02 remains responsible for assertions covering:

- a successor's fresh TUI;
- stale/prior-attempt claim rejection inside the live scaffold;
- zero duplicate injection during passive wait;
- clean exit and attempt revocation;
- exactly one authoritative completion across the full two-ticket lifecycle;
- blocking classification for any future unexplained live behavior.

Those are intentional downstream story boundaries, not defects in this harness ticket.

No scheduler behavior, Claude behavior, or E-034 fencing contract changed.

No critical issue blocks completion.

## Repository integrity

Ticket-owned source is committed and clean.

The ordinary index is empty.

The final source checkout status contains only scheduler/concurrent-ticket paths that predated or
were published outside this ticket's source ownership.

This attempt did not edit the shared ticket phase/status or publish completion itself.

Lisa remains responsible for admitting these Review artifacts and confirming the final ticket
completion commit.

Final assessment: ready to complete.
