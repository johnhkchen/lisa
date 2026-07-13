# Review — T-045-05-02 field regression assertions

## Disposition

Pass.

The ticket's acceptance criterion is satisfied on installed Codex and installed Zellij.

The live harness now asserts the complete E-045 Done contract rather than retaining raw evidence
for a downstream ticket.

The final authorized run ended with the stable overall PASS receipt.

No unexplained real-Codex behavior remains.

## Ticket source commit

The isolated source commit is:

```text
53bb7539c1068655f361959f48a36bbf2778caa6
test(cli): assert live Codex ticket lifecycle
```

It contains exactly:

- `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`;
- `docs/knowledge/live-codex-review-boundary.md`.

The commit was created with:

```text
target/release/lisa commit-ticket \
  --ticket-id T-045-05-02 \
  --message "test(cli): assert live Codex ticket lifecycle" \
  --include crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh \
  --include docs/knowledge/live-codex-review-boundary.md
```

No ordinary `git add`, broad staging command, or ordinary `git commit` was used.

The committed diff is 558 insertions and 113 deletions across the two files.

`git show --check 53bb7539c1068655f361959f48a36bbf2778caa6` passes.

Both ticket-owned source paths are clean relative to HEAD.

The ordinary Git index is empty.

## Harness change

The historical comparison remains one real Codex turn under the explicit pre-E-045 Lisa binary.

The current comparison is now a two-ticket Review dependency chain.

The tickets are:

- `T-FIELD-REVIEW-01`;
- dependent `T-FIELD-REVIEW-02`.

Both begin open in Review with existing canonical Review evidence.

Both run under hooks-disabled ephemeral Codex homes.

Scheduler concurrency stays one.

The generated layout still exposes two physical terminal panes.

The harness accepts either physical-pane reuse or another idle pane, but always requires a fresh
launcher and fresh Codex child process for the successor.

## Claim-first ownership

The fixture `AGENTS.md` still requires one exact first shell tool action.

That action discovers only the current ticket's immutable assignment file.

It derives the full nonce from the filename.

It waits for the ticket's private field gate.

The sampler opens that gate only after the dashboard exposes
`delivered-awaiting-claim` for that exact ticket.

After the configured one-second settling interval, the same tool action calls native
`lisa claim`.

Only after claim acceptance may Codex inspect Review evidence or write artifacts.

The exact claim therefore remains the first successful ownership action.

The sampler copies each ephemeral claim before the plugin consumes it.

The final assertions compare the claim ticket, attempt, and full numeric nonce with the one
assignment filename.

## Slow response and no reinjection

The harness records ticket-scoped state transitions rather than first-seen global labels.

For each current ticket it requires exactly:

1. `starting`;
2. `delivering`;
3. `delivered-awaiting-claim`;
4. `owned`.

Because repeated states are recorded when they return, a retry back to `delivering` fails.

Each ticket must also have exactly one launch script.

Each must have exactly one immutable nonce assignment.

Each must have exactly one exact captured claim.

Each timestamped terminal screen is checked independently.

No one screen may contain more than one tagged assignment message for that ticket.

Repeated sampling of the same visible message does not inflate the injection count.

No delivery-failed, claim-timed-out, or failed current dashboard state is accepted.

## Stale claim rejection

While the successor's real Codex first action waits on its field gate, the harness resolves the
actual pane from the successor's durable attempt-one lease.

It invokes the current native Lisa claim command against that live pane.

The probe names the correct successor ticket but attempt zero.

The command must exit nonzero.

Its stderr must contain the stable `[stale-attempt]` reason.

Its stdout must contain no acceptance receipt.

No attempt-zero claim signal may be published.

Only after that rejection does the harness open the real Codex claim gate.

The exact attempt-one successor claim then reaches Owned.

## Fresh TUI and clean boundary

The sampler creates a structured `process-events.tsv` ledger.

It records each unique assignment-bearing Lisa launcher and Codex child once.

It associates the process row with the exact ticket assignment path.

Each ticket must have one launcher row and one Codex child row.

The predecessor and successor launcher PIDs must differ.

The predecessor and successor Codex PIDs must differ.

Both complete process trees must exit within the bounded post-completion wait.

The sampler also records durable lease identities per physical pane.

Each live ticket must have an exact attempt-one lease body.

The final Zellij pane manifest must show each used terminal at an idle shell boundary.

The scheduler's in-memory current lease remains the final authority.

Successful completion revokes that authority before requesting `/exit`.

Durable `.lease` files can remain as producer-validation history because successful completion is
not a reset/fence cleanup path.

The deterministic completion regression confirms an exact predecessor claim is rejected after
revocation and after successor launch.

## Exact completion assertions

The harness waits until both final ticket files have `status: done` and `phase: done`.

For each ticket, it requires exactly one completion-journal `requested` row.

It requires exactly one `command-in-flight` row.

It requires exactly one `confirmed` row.

All rows must name attempt one and generation one.

The confirmation must carry a 40-character hexadecimal commit ID.

The combined relevant journal cardinality must be six rows.

For each ticket, provenance must contain exactly one row.

That row must be Done, authoritative, unfenced, and actual Codex.

The combined relevant provenance cardinality must be two rows.

Duplicate completion publication therefore fails before the PASS receipt.

## Final authorized field run

Final evidence is retained at:

`.lisa/attempts/T-045-05-02/1/work/evidence-live-final-3/`.

The invocation selected:

- `/Users/johnchen/.local/bin/lisa` as the historical subject;
- the immediately prepared `target/release/lisa` as the current subject;
- fixture retention for diagnosis;
- a fresh evidence directory.

Stable receipts were:

```text
legacy-false-delivery-failure: OBSERVED
current-slow-claim-no-reinjection: ASSERTED
current-stale-claim: REJECTED
current-fresh-tui-boundary: ASSERTED
current-exact-completions: ASSERTED
live-codex-review-boundary: PASS
```

## Recorded tool and binary identity

The run recorded:

- Codex CLI `0.144.3`;
- Zellij `0.44.3`;
- Cargo nightly `1.99.0-nightly`;
- both Lisa subjects reporting `0.4.0-rc.8`.

Equal version text did not collapse subject identity.

The legacy Lisa SHA-256 was:

`7d03785f7a59c730886d47fa33eec084d82245adc2aa766d1ff8dd58b9c52bf8`.

The current Lisa SHA-256 was:

`92997179e3af21e377abda0e2e155f24c94f969495770e8aa518137c2826c666`.

The current release WASM SHA-256 was:

`3f617aa61de7ee0ade3734aec1b92f9f903679039692006b45b196faf38424c2`.

The harness required legacy/current hashes to differ.

## Current state timelines

Ticket one:

```text
2026-07-13T16:45:06Z starting
2026-07-13T16:45:15Z delivering
2026-07-13T16:45:26Z delivered-awaiting-claim
2026-07-13T16:45:30Z owned
```

Ticket two:

```text
2026-07-13T16:46:01Z starting
2026-07-13T16:46:11Z delivering
2026-07-13T16:46:20Z delivered-awaiting-claim
2026-07-13T16:46:25Z owned
```

Neither sequence contains reinjection or terminal failure.

## Claim and process evidence

Ticket one used pane zero.

Its exact claim nonce was `1783961105693011000`.

Its Lisa launcher PID was `71240`.

Its Codex child PID was `71241`.

Ticket two used pane one.

Its exact claim nonce was `1783961160798852000`.

Its Lisa launcher PID was `89060`.

Its Codex child PID was `89061`.

The successor stale probe exited one with the named stale-attempt rejection.

No hook `.ack` or `.started` signal was captured.

The Codex runtime receipt records hooks false and `hooks.json` absent.

Final terminal titles were `lisa · idle` and `codex · idle`.

## Completion evidence

Ticket one confirmed fixture commit:

`19dcdf5695c8a7bd8740b374fbfee558c86e22e2`.

Ticket two confirmed fixture commit:

`361335f84c1ff0aa986fa2dcd635d5b4d3716bf8`.

The final fixture Git log contains the baseline followed by exactly those two completion commits.

Provenance has two rows total.

Both are authoritative, unfenced, Codex Done rows under attempt one.

Both final synthetic ticket files are Done.

## Calibration failures and explanation

Two earlier current evidence sets failed and remain retained.

The first failed run used a 50-row PTY.

With two Review attention entries, the dashboard's 30-percent pane had too few rows to render its
thread table.

The harness therefore did not observe the waiting state and did not open the claim gate.

Lisa reached its correct named `claim-timed-out` state.

This was fixed by enlarging the PTY and was not relabeled as acceptance evidence.

The second run completed both tickets and passed all transport, claim, process, and completion
facts.

It failed only an added harness assumption that completion deletes `.lease` files.

Source inspection showed successful completion revokes in-memory authority but reserves marker
deletion for reset/fence cleanup.

The final assertion now checks the actual authority and clean-process boundary.

Both failures are fully explained harness calibration findings.

Neither is unexplained real-Codex behavior.

## Non-metered verification

Passed:

- `bash -n crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`;
- ShellCheck with no findings;
- `git diff --check` for harness and runbook;
- preparation mode with the stable `PREPARED` receipt;
- post-commit `git show --check`.

Focused CLI tests passed:

- claim CLI: 3;
- Codex launcher: 1.

Focused plugin regressions passed:

- slow Codex claim waits without reinjection and ends actionably;
- Codex completion exits, revokes, and launches a fresh successor TUI.

## Open concerns and limitations

Live provider timing remains observational and host-specific.

The field gate makes the required state ordering observable without pretending model latency is
deterministic.

Terminal and loop transcripts contain more context than the compact receipts; share compact
evidence preferentially.

Durable lease markers should not be interpreted as current scheduler authority in isolation.

No scheduler production code changed in this ticket.

No Claude behavior changed.

No default automated test launches an authenticated provider.

No critical issue blocks completion.

## Repository integrity

Ticket-owned source is committed and clean.

The ordinary index is empty.

The remaining checkout status consists only of Lisa-managed runtime, ticket, story, epic, and
published work paths outside this ticket's source commit.

This attempt did not edit ticket phase/status or publish completion itself.

Lisa remains responsible for admitting these Review artifacts and confirming the final ticket
completion commit.

Final assessment: ready to complete.
