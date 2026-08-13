# T-067-01-02 — an attempt that happened leaves a record

## What was wrong

`fail_startup_recovery` was the one terminal path in the plugin that ended a
**live** attempt and wrote nothing. It fences the pane, revokes the lease,
clears the pane's signals and withdraws its lease marker — and it did all of
that without touching the ledger. Every other fence path (`check_session_timeouts`
→ `timed-out`, `detect_stale_threads` → `failed`) already emitted a row; this one
did not, and it is the path the `screen-design` failure took.

Nothing else wrote anything either, because every row in `.lisa/provenance.jsonl`
was write-after: a row described an attempt that had ended, and these attempts
had no recorded end. So `grep T-019-01 .lisa/provenance.jsonl` came back empty
for two attempts that had each run twelve minutes.

## What changed

### The ledger (`crates/lisa-core/src/provenance.rs`)

- **`AttemptLaunchRecord`** (`record_type: "attempt-launch"`) — the one
  write-*before* row in the file. It names the ticket, the exact
  `attempt_lease`, the pane, the provider, and the assignment file's name.
- **`RunOutcome::SeatLost`** (`"seat-lost"`) — a fourth outcome, deliberately
  its own word. `failed` means *the work reported an error*; `timed-out` means
  *the attempt outran its budget*; `seat-lost` means *the scheduler ended an
  attempt that had not said anything wrong*.
- **`ProvenanceRecord.reason`** — optional, additive, absent on every row
  written before schema 11, always present on `seat-lost`.
- **`lost_seat_usage_gap`** — tickets whose seat was taken with no tokens
  joined.
- `SCHEMA_VERSION` 10 → 11.

### The scheduler (`crates/lisa-plugin/src/lib.rs`)

- `prepare_assignment` now emits the launch row, immediately after the
  assignment file is published and before the launch line is typed. It is
  emitted **from the publication path itself**, not from each of the three
  dispatch sites (`schedule_ready_tickets`, the same-pane relaunch, and the
  post-exit launch), so a fourth launch path cannot skip it. That is the
  mechanism behind "the ledger should not be able to disagree with the
  existence of that file."
- `fail_startup_recovery` emits `seat-lost` with the scheduler's reason, as its
  **first** action — before the fence, the lease revocation and the signal
  clearing, because those are exactly the inputs the row is built from. Same
  discipline `on-stop.sh` states at the other end of an attempt's life.
- Test helper split: `read_full_ledger` (every row) vs `read_mixed_ledger`
  (everything except launch receipts). Existing tests assert "the ledger holds
  exactly the unpark row" and would otherwise have been counting dispatches.

### What an operator sees (`crates/lisa-cli/`)

- `lisa status` gained a **Tickets nobody is working** section: a ticket in
  `implement` or `review` that no pane lease marker names, with the ledger's
  last word beside it and the command that hands it back. Silent on a healthy
  board.
- `run_summary` counts `seats_lost` and prints
  `Run issues: 0 failed, 0 timed out, 2 lost their seat.` — the `failed: 0`
  complaint in the ticket.
- `token_usage.lost_with_the_seat[]` and a *Lost with the seat* line, kept apart
  from `not_yet_joined` because the prognosis differs.
- `--json` gained `stranded[]` and `token_usage.lost_with_the_seat[]`;
  `lisa json-guide` documents both.

### Docs

`docs/knowledge/provenance-ledger.md` — the new row shape, the `seat-lost`
outcome and why it is its own word, the token-recoverability split, the
write-before exception, and a jq query for attempts launched with no recorded
end.

## Acceptance criteria

| Criterion | Where |
| --- | --- |
| An attempt that was launched leaves a row | `prepare_assignment` → `emit_attempt_launch`; `a_launched_attempt_leaves_a_row_naming_the_assignment_file_it_was_given` |
| A seat that is lost is recorded as lost, with the reason, distinguishable from `timed-out` and `done` | `RunOutcome::SeatLost` + `reason`; `a_lost_seat_is_spelled_apart_from_done_and_timed_out`, `a_seat_lost_mid_attempt_is_recorded_as_lost_with_the_reason` |
| A ticket cannot sit in `implement`/`review` with no seat and no ledger entry | **The state is named**, not walked back — `lisa status` *Tickets nobody is working* + `stranded[]`; `a_ticket_under_way_with_no_seat_is_named_and_carries_the_ledgers_reason`, `a_stranded_ticket_with_no_ledger_row_at_all_still_says_so` |
| The token spend is attributed | Answered below; `lost_seat_usage_is_counted_as_lost_until_a_capture_joins_it`, `tokens_lost_with_a_seat_are_counted_apart_from_a_late_capture` |
| `run_summary` tells the truth | `seats_lost`; `a_run_that_lost_two_seats_does_not_report_itself_as_clean` |
| Reproduce it | `crates/lisa-plugin/src/tests/an_attempt_that_happened_leaves_a_record.rs` — three tests, including the operator's own grep |

### The token question, answered plainly

**Those two sessions' usage is not recoverable.** `lisa capture-usage` runs from
the provider's Stop hook, so a capture exists only if the session reached a
stop. Both panes were fenced with live agents inside them, so no capture was
ever written and there is no other reader of the provider's accounting. Nothing
downstream can invent it.

What is now true is that the loss is *recorded*: the `seat-lost` row exists with
`tokens_in: null` — never a fabricated zero — and `lost_seat_usage_gap` counts
it, which `lisa status` prints. The other half is a genuine recovery: because a
`seat-lost` row is a durable execution row, `sweep_usage_captures` treats it as
a pane reign, so a seat lost *after* its session stopped gets its capture joined
exactly as a completed ticket does. That case used to lose its tokens too.

## Testing

`just check` — exit 0. `cargo test --workspace`: 665 plugin, 341 core, 576 CLI
unit + integration suites, all green.

Verified end to end against a real board, not only in unit tests. A ticket at
`phase: review` with a `seat-lost` row in the ledger and no lease marker:

```
Tickets nobody is working
  T-019-01     review     attempt 1 lost its seat: startup was never observed and the pane proved nothing
  The board says these are under way and no pane holds them.
  To hand one back: lisa reset-ticket <ticket-id> --apply

Token usage
  Lost with the seat: 1 ticket — the pane went before its session could report what it spent, so those tokens are not recoverable.

Run issues: 0 failed, 0 timed out, 1 lost their seat.
```

`--json` carries the same facts in `stranded[]`, `run_summary.seats_lost` and
`token_usage.lost_with_the_seat[]`.

## What still concerns me

1. **The phase is named, not walked back.** The criterion allowed either. I
   named it because walking a ticket's phase backwards from the scheduler
   would write agent-owned frontmatter on a ticket whose agent may still be
   alive in a pane Lisa has merely stopped watching — the failure that started
   this story was a scheduler acting on evidence it could not confirm, and
   walking the phase back is that same move. The named state plus
   `lisa reset-ticket` keeps the decision with a person. If the board is meant
   to self-heal, that is a follow-up with its own argument to make.

2. **The launch row is best-effort.** A ledger write failure logs loudly and the
   dispatch proceeds, exactly like every other ledger write in the plugin. So
   the invariant is "an assignment file implies a row *unless the ledger itself
   is unwritable*". Fail-closed was deliberately not chosen: a ledger that can
   refuse a dispatch is a ledger that can stop the board, which this project has
   already been bitten by (0.4.4's fail-closed journal replay).

3. **Legacy rows read as silence.** Existing ledgers carry no launch rows and no
   `reason`, so a ticket stranded before this change reports *"the ledger has no
   record of an attempt on this ticket"*. That is accurate rather than helpful,
   and it is the honest floor — the evidence genuinely is not there. Documented
   in `lisa json-guide`.

4. **`stranded` is a one-shot command's best answer.** `lisa status` cannot see
   the plugin's live seat table; it reads lease markers. A ticket dispatched in
   the same instant the command runs — after the ticket's phase moved but before
   its marker was published — would appear stranded for one poll. The window is
   small and the entry disappears on the next run, but it exists.

5. **Concurrency with `T-067-01-01`.** Both tickets ran at once on
   `crates/lisa-plugin/src/lib.rs` with no dependency edge between them — the
   missing edge the workflow document warns about. In practice their commit
   `23b11b2` swept my `lib.rs` hunks and my new test file into itself, so this
   ticket's plugin changes are durable but carry that commit's message rather
   than mine. Nothing is lost or uncommitted; the attribution in `git log` is
   just wrong for those hunks. Worth an edge on the next pair of tickets that
   share a file.
