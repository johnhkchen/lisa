# Progress — T-045-05-02 field regression assertions

## Baseline

- Read `CLAUDE.md`, the ticket, story, epic, predecessor artifacts, and RDSPI workflow.
- Confirmed source HEAD `4600b2fb717c055ab1eb3e081ebddd0dc69b204e`.
- Confirmed the owned harness and runbook had no pre-ticket diff.
- Confirmed the ordinary Git index was empty.
- Confirmed ShellCheck, Lisa, Codex, and Zellij are installed.
- Recorded installed Codex `0.144.3` and Zellij `0.44.3`.
- Recorded installed legacy Lisa `0.4.0-rc.8` at `/Users/johnchen/.local/bin/lisa`.

## Research complete

- Mapped the predecessor live harness and retained evidence.
- Mapped native claim rejection and plugin claim admission boundaries.
- Mapped slow-delivery passive wait and no-retry behavior.
- Mapped clean Codex completion, lease revocation, and successor launch behavior.
- Identified journal, provenance, process, lease, signal, and screen evidence surfaces.
- Confirmed the ticket can remain shell/runbook-only with no scheduler contract change.

## Design complete

- Selected a two-ticket current Review dependency chain in the existing field harness.
- Kept the legacy one-ticket false-failure observation as the historical control.
- Selected ticket-scoped transition records to distinguish observation from reinjection.
- Selected a harness-owned native stale claim probe against the live successor lease.
- Preserved real Codex's exact claim as the first successful ownership action.
- Selected structured launcher/Codex PID and lease-transition receipts.
- Selected exact journal/provenance cardinality assertions per ticket.

## Structure complete

- Limited ticket-owned source to the existing harness and runbook.
- Defined primary/successor fixture identities and snapshot layout.
- Defined state, process, lease, claim, boundary, and completion assertion helpers.
- Defined one isolated source commit spanning executable behavior and documentation.

## Plan complete

- Sequenced fixture expansion, evidence recording, assertion helpers, docs, verification, live run,
  isolated commit, and Review.
- Defined non-metered and live verification criteria.
- Defined blocking disposition for any unexplained real-provider behavior.

## Implement in progress

- Expanded the fixture to `T-FIELD-REVIEW-01` and dependent `T-FIELD-REVIEW-02`.
- Kept both tickets in Review with prior canonical evidence and one physical scheduler thread.
- Added ticket-scoped state transition recording without first-seen suppression.
- Added a live native stale-attempt probe against the successor's actual pane lease.
- Kept each real Codex exact claim in its first tool action behind the passive-wait gate.
- Added structured unique launcher/Codex PID receipts.
- Added pane lease present/absent identity transitions.
- Generalized final evidence capture across both tickets.
- Added exact assignment, nonce, claim, state, reinjection, fresh-process, revocation, journal, and
  provenance assertions.
- Added stable receipts for all four current-path acceptance groups.
- Updated the runbook for three metered turns, the stronger evidence layout, and failure policy.

## Non-metered verification complete

- `bash -n crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`: passed.
- `shellcheck crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`: passed with no findings.
- `git diff --check` for the harness and runbook: passed.
- `PREPARE_ONLY=1 ... live_codex_review_boundary.sh`: passed with `PREPARED`.
- Preparation rebuilt release WASM and CLI and validated both two-ticket fixture repositories.
- `cargo test -p lisa-cli --test codex_launcher --test claim_cli`: 4 passed.
- `cargo test -p lisa-plugin live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably`:
  1 passed, 394 filtered.
- `cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui`:
  1 passed, 394 filtered.

## Remaining

- Run authorized real Codex/Zellij field validation.
- Inspect and explain all retained evidence.
- Correct only demonstrated harness defects, documenting any deviation before doing so.
- Commit exact source paths through `lisa commit-ticket`.
- Complete Review artifacts.

## Plan deviation — first live run exposed harness observation assumptions

The first authorized field attempt retained a failed current case at
`evidence-live-final/current/`.

The legacy control passed.

The current fixture launched real Codex for both tickets, but the harness timed out waiting for
the primary passive-wait state.

The failure is explained by two harness assumptions, not unexplained provider behavior:

- the 50-row PTY gives the 30-percent dashboard pane only 15 rows; two Review attention entries
  consume the available body, so the scheduler thread row and its state are not rendered;
- Lisa's layout has two physical terminal panes even with scheduler concurrency one, so the
  successor launched fresh in pane one while the predecessor used pane zero; the initial design
  incorrectly assumed both serial tickets would reuse one pane.

Because the state recorder never observed `delivered-awaiting-claim`, it did not create the primary
claim gate. The first attempt reached the named `claim-timed-out` terminal state exactly as the
product contract specifies. Its provenance contains that assignment transition. Codex later wrote
Review evidence after its blocked first tool action ended, and Lisa completed the ticket before
launching a fresh successor process. The harness stopped the session at its own 90-second wait.

Before rerunning:

- enlarge the PTY so ticket state rows are field-observable;
- resolve live pane identity from each ticket's durable lease rather than assuming one pane;
- record lease transitions per pane;
- capture terminal snapshots per ticket/pane;
- assert predecessor lease removal and successor lease identity without requiring the successor
  to reuse the predecessor pane.

These corrections strengthen the intended evidence and do not change production timeouts or
relax any acceptance assertion.

## Plan deviation — durable lease markers are historical, not revocation authority

The second authorized field attempt retained a fully completed current case at
`evidence-live-final-2/current/`.

Both tickets produced the exact state sequence:

1. `starting`;
2. `delivering`;
3. `delivered-awaiting-claim`;
4. `owned`.

Both exact claims were captured, the successor stale-attempt probe was rejected, both launcher and
Codex process identities were distinct, both process trees exited, both tickets reached Done, the
completion journal contained two exact triples, and provenance contained two authoritative Done
rows.

The harness then failed its added requirement for deletion of the predecessor `.lease` file.

Source inspection explains the observation. `release_completed_slot_for_ticket` calls
`release_slot_for_ticket`, which revokes `current_leases` in scheduler memory before requesting the
clean Codex exit. It deliberately does not call `clear_pane_lifecycle_signals`; that cleanup helper
is used by reset/fencing paths. The durable marker can therefore remain as historical
producer-validation input, while final scheduler admission still rejects any claim without an
in-memory current lease and retained assignment state.

The deterministic completion-boundary regression already asserts the exact predecessor claim is
rejected after revocation and after successor launch.

Before the final rerun:

- stop treating marker deletion as the revocation contract;
- require exact lease identities for both live panes;
- require both assignment-bearing process trees to exit;
- require their final terminal panes to be idle;
- retain distinct process identities, stale-attempt rejection, exact claims, and exact completion
  cardinality.

This correction aligns the field assertion with the actual E-034 authority boundary and does not
weaken stale-claim or clean-TUI proof.

## Final authorized live run complete

The final passing evidence is retained at
`.lisa/attempts/T-045-05-02/1/work/evidence-live-final-3/`.

Stable receipts:

```text
legacy-false-delivery-failure: OBSERVED
current-slow-claim-no-reinjection: ASSERTED
current-stale-claim: REJECTED
current-fresh-tui-boundary: ASSERTED
current-exact-completions: ASSERTED
live-codex-review-boundary: PASS
```

Recorded tool identities:

- Codex `0.144.3`;
- Zellij `0.44.3`;
- legacy Lisa SHA-256
  `7d03785f7a59c730886d47fa33eec084d82245adc2aa766d1ff8dd58b9c52bf8`;
- current Lisa SHA-256
  `92997179e3af21e377abda0e2e155f24c94f969495770e8aa518137c2826c666`;
- current release WASM SHA-256
  `3f617aa61de7ee0ade3734aec1b92f9f903679039692006b45b196faf38424c2`.

Current ticket one state sequence:

```text
16:45:06 starting
16:45:15 delivering
16:45:26 delivered-awaiting-claim
16:45:30 owned
```

Current ticket two state sequence:

```text
16:46:01 starting
16:46:11 delivering
16:46:20 delivered-awaiting-claim
16:46:25 owned
```

The stale successor probe exited one and reported
`claim rejected [stale-attempt]`.

Exact claims matched the full assignment nonces:

- ticket one nonce `1783961105693011000` on pane zero;
- ticket two nonce `1783961160798852000` on pane one.

Fresh process identities:

- ticket one launcher/Codex PIDs `71240` / `71241`;
- ticket two launcher/Codex PIDs `89060` / `89061`.

Both process trees exited before assertions ran.

Final pane titles were `lisa · idle` and `codex · idle`.

Hooks were false and no `hooks.json` was present.

No `.ack` or `.started` hook signal was captured.

Each ticket had exactly one launch script, assignment file, exact claim, and four-state sequence.

The completion journal contained exactly six rows: one requested, one command-in-flight, and one
confirmed row per ticket.

Confirmed fixture commits were:

- ticket one `19dcdf5695c8a7bd8740b374fbfee558c86e22e2`;
- ticket two `361335f84c1ff0aa986fa2dcd635d5b4d3716bf8`.

Provenance contained exactly two rows, one authoritative unfenced Codex Done outcome per ticket.

Both final ticket files had `status: done` and `phase: done`.

Ephemeral Codex homes were deleted by cleanup; retained fixture roots contain no authentication
bytes.

## Remaining after field pass

- Final `bash -n`, ShellCheck, `git diff --check`, and repository-integrity checks passed.
- Committed the harness and runbook through the isolated ticket transaction.
- Source commit: `53bb7539c1068655f361959f48a36bbf2778caa6`.
- Commit message: `test(cli): assert live Codex ticket lifecycle`.
- `git show --check 53bb7539c1068655f361959f48a36bbf2778caa6`: passed.
- Both ticket-owned source paths are clean relative to HEAD.
- The ordinary Git index remains empty.
- Remaining: write Review and exact disposition, then stop on this ticket.
