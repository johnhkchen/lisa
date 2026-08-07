# T-056-01-03 Research — no-check-that-cannot-pass

Descriptive map of the code the ticket names, as it stands after T-056-01-01 and T-056-01-02 landed.

## 1. Where a check lives, end to end

A `check` string enters the system in exactly one place and is executed in exactly two.

**Authored** — the Review phase writes `review-disposition.json`. The block shape is described in
`docs/knowledge/rdspi-workflow.md:55–59`. Line 59 is the only sentence that describes what a check
is: *"Supply a `check` whenever the remedy is externally observable … The check verifies the remedy
but must never perform it."* Nothing there says where it runs, what it can see, whether it may
write, or how long it may take.

**Validated** — `lisa check-disposition <ticket-id>` (`crates/lisa-cli/src/check_disposition.rs`)
runs during Review, per `rdspi-workflow.md:67`. It resolves the attempt-private path from
`LISA_TICKET_ID` / `LISA_ATTEMPT_ID` (`check_disposition.rs:26–63`), calls
`check_review_disposition` (strict schema parse) and `validate_block_ask` (the ask floor). The
`check` field is validated for *shape only*: `disposition.rs:272–281` accepts any non-empty string.
Nothing executes it.

**Parsed** — `parse_review_disposition` (tolerant) at `disposition.rs:127`, and
`check_review_disposition` (strict authoring) at `159`. The two differ deliberately: the tolerant
one degrades a malformed block into `unstructured_block` (operator-owned, *check dropped*); the
strict one names a fix and refuses. `check` reaches `ReviewDisposition::Block { check, .. }`
(`disposition.rs:99–110`) and then `ParkedRemedy { check, .. }`
(`parking.rs:75–88`, filled at `144–197`).

**Executed** — two callers, both in `crates/lisa-cli/src/unblock.rs`:

- `run_unblock` (`102`) — the operator command, one ticket, `CHECK_TIMEOUT` (`140`).
- `run_world_rechecks` (`211`) — automation, every world-owned parked remedy with a check
  (`232`), same constant.

Both go through `run_check` (`375`). The plugin fires the second every poll: `request_world_recheck`
(`lisa-plugin/src/lib.rs:1761`) is called from the poll handler (`8255`) and at loop start (`9516`),
with `POLL_INTERVAL_SECS = 5.0` (`lib.rs:83`) and a `world_recheck_in_flight` guard. **A parked
world remedy is therefore rechecked roughly every five seconds for as long as it stays parked.**
That cadence is the central constraint on any "record what the check reported" design.

## 2. The three ceilings

### 2.1 The five-second budget

`const CHECK_TIMEOUT: Duration = Duration::from_secs(5);` (`unblock.rs:23`). It is passed
explicitly as `run_check`'s third parameter, so the *function* is already parameterised — only its
two callers hardcode the constant. The expiry sentence is built in `decline_timed_out`
(`unblock.rs:280–285`) and again in `exit_code_line` (`297–306`), both formatting
`CHECK_TIMEOUT.as_secs()`. Both are already derived from the constant rather than spelled out, so
neither contains a literal `"5 seconds"` — but neither can name a per-check budget, because
`CheckRun` (`77–91`) does not carry one.

The unit test `timeout_is_bounded_and_kills_the_shell_group` (`640`) pins the exact sentence
`"That didn't work yet — it took longer than 5 seconds."`, and the black-box test
`automatic_recheck_timeout_is_bounded_and_cannot_reopen` (`tests/parked_ux.rs:604`) asserts a
`sleep 30` world check returns in 4–8 seconds.

### 2.2 Writes

The read-only snapshot and the before/after fingerprint the ticket describes **no longer exist**.
T-056-01-02 removed them: `run_check` now spawns in `root` itself (`unblock.rs:385–391`), and its
doc comment (`355–374`) records the decision and its reason — a before/after fingerprint of a live
tree cannot separate the check's writes from a concurrent agent thread's, because the scheduler
fires `recheck-world` while sessions are editing the same files, so reporting them would be the
same class of false verdict this story exists to remove.

Two residues of the old behaviour remain: `CheckOverrideOutcome::ChangedFiles`
(`provenance.rs:350–351`), deliberately kept on the wire for ledgers written before the change, and
`override_outcome` (`unblock.rs:195–202`) which no longer produces it. What does *not* exist
anywhere is a statement of the decision where a reviewer authors a check.

### 2.3 The silent world recheck

`run_world_rechecks` (`unblock.rs:211–249`): for each world-owned remedy with a check, run it; on
`Passed` flip the ticket to `Open` and push the id onto `reopened`; on
`Failed | Inconclusive | TimedOut` do nothing at all (`244`). The `Vec<String>` return is printed
one id per line by `main.rs:691–704`, and the plugin parses that stdout in
`handle_world_recheck_result` (`lib.rs:1788`). A non-pass produces no output, no ledger row, no
counter — it is indistinguishable from "no world remedies exist". Two black-box tests pin exactly
that silence: `world_owned_failing_check_stays_parked_without_churn` (`parked_ux.rs:563`) and
`automatic_recheck_timeout_is_bounded_and_cannot_reopen` (`604`) both assert empty stdout *and*
empty stderr.

## 3. The execution contract as it actually is

`run_check` (`unblock.rs:375–457`) — the whole contract in one function:

| Fact | Value | Line |
| --- | --- | --- |
| working directory | the project root, cloned into `CheckRun::directory` | 385–391 |
| what it sees | the live tree: tracked, untracked, and gitignored alike | 355–366 |
| shell | `/bin/sh -c <check>` | 387–389 |
| stdin | `Stdio::null()` | 393 |
| `TMPDIR`/`TMP`/`TEMP` | a disposable `tempfile::tempdir()` | 392–394 |
| writes | not prevented, not detected, not judged | 366–374 |
| time budget | the `timeout` argument; whole process group killed on expiry | 412–429, 476–483 |
| capture | 8 KiB per stream, 10 sanitized display lines, rest counted | 25–30, 490–518 |
| exit mapping | 0 pass; 2/126/127/signal inconclusive; anything else failed | 467–474 |

`CheckRun` carries the facts a report needs — check string, directory, exit code, both streams,
dropped counts — so the report cannot recompute (and drift from) what actually happened. It does
not carry the budget.

## 4. The durable record shapes

`crates/lisa-core/src/provenance.rs` is an append-only JSONL ledger at `.lisa/provenance.jsonl`,
read as `ProvenanceLedgerRecord` — an `#[serde(untagged)]` enum (`439–451`) whose variants are kept
disjoint by single-variant `record_type` enums. Two records are shaped like what this ticket needs:

- `CheckOverrideRecord` (`362–384`): `schema_version`, `seal`, `record_type`, `ticket_id`, `actor`,
  `check`, `directory`, `result: CheckOverrideOutcome`, `exit_code`, `observed`, `occurred_at`.
  Written by `record_check_override` (`unblock.rs:164–187`) *before* the status flip, deliberately.
  It has no `attempt_lease`, and its doc comment says why: the attempt that parked the ticket is
  already gone, so synthesizing a lease would file a run that never happened.
- `ParkingTransitionRecord` — read back by `latest_park_records` (`parking.rs:95–113`), the
  established pattern for "fold the ledger into a current-state projection".

Appends go through `append_serialized` (`provenance.rs:681`), one public `append_*` wrapper per
record type.

## 5. Who reads a parked remedy

- `lisa status` — `waiting_on_you_lines` (`status.rs:87–120`). World-owned remedies render
  `"{id}  {ask} — Lisa checks on its own."` (`93–96`); a pinned black-box test
  (`parked_ux.rs:177`) asserts the whole opening block byte for byte.
- The plugin dashboard — `ui.rs:685`, `"→ [d] mark it done · Lisa checks on its own"`, driven by
  `card.detail.checks_on_own`.
- `lisa unblock` — the decline report (`unblock.rs:313–353`): header, `what ran` / `ran in` /
  `exit code`, both streams labelled, then the `--override-check` way through.

## 6. Where the documented contract is compiled in

`docs/knowledge/rdspi-workflow.md` is not the source of truth — `crates/lisa-cli/data/rdspi-workflow.md`
is. `templates.rs:5–11` builds `RDSPI_WORKFLOW` as `PURPOSE_PARAGRAPH + "\n\n" + include_str!(data
file)`, and `test_rdspi_workflow_embedded` (`templates.rs:747`) asserts it equals
`include_str!("../../../docs/knowledge/rdspi-workflow.md")` byte for byte. **Any doc change must
edit both files identically** (the checked-in one carries the purpose paragraph as its first two
lines). `test_review_disposition_contract_is_injected` (`763`) is the existing precedent for
asserting specific contract sentences are present in the document — the natural home for a
doc-matches-code assertion.

## 7. Construction sites that a new `Block` field touches

`ReviewDisposition::Block` is constructed as a full struct literal in: `disposition.rs` (289, 410,
431, plus ~8 test literals), `lisa-core/src/completion.rs:1356`,
`lisa-core/tests/completion_state_machine.rs:167`, `lisa-cli/src/proposal.rs:69`, and
`lisa-plugin/src/lib.rs` (23580, 23666, 23801, 23894, 27983 — all in tests). It is destructured
exhaustively in `parking.rs:156–164`; everywhere else uses `..`.

## 8. Constraints and assumptions carried into Design

1. **The 5-second poll is the hard constraint on recording.** A row per non-pass is 720 rows/hour
   for one parked ticket. Whatever "surface repeated non-passes" means, it must be sub-linear in
   poll count.
2. **A block's check is *expected* to fail at record time.** The remedy has not happened yet — that
   is why the ticket is blocking. So "a check that cannot pass" cannot mean "exit non-zero"; it can
   only mean the classes that carry no verdict: `Inconclusive` (2/126/127/signal — the field case
   was 127-shaped) and `TimedOut`, plus the record-time-only class of a declared budget above the
   cap.
3. **Two tests currently pin the silence** that criterion 6 asks to break, and one pins the literal
   `"5 seconds"` sentence. Both are correct today and must be revised deliberately, not deleted.
4. **The tolerant parser must stay fail-safe.** It is what reads historical and hand-edited files;
   it may never widen a budget past the documented cap on the strength of a number in a file.
5. **`check-disposition` executing a recorded check is new behaviour for that command.** It already
   runs inside the attempt with the project root passed as `--path`, and the strings it would run
   are ones the same agent just wrote, so the trust boundary does not move — but the command's cost
   becomes the check's cost.
6. Nothing in `lisa validate` (`init.rs`) touches dispositions today; adding the record-time run to
   `check-disposition` alone keeps one execution path for one purpose.
