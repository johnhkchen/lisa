# T-056-01-03 Plan — ordered, committable steps

Seven steps. Each compiles, each is a `lisa commit-ticket` unit, each names its verification.
Acceptance criteria are referenced as C1–C8 in ticket order.

---

## Step 1 — Budget in the schema (C1 record-time half)

**Files:** `crates/lisa-core/src/disposition.rs`, `crates/lisa-core/src/parking.rs`, and the
`Block { .. }` literals in `lisa-core/src/completion.rs`, `lisa-cli/src/proposal.rs`,
`lisa-plugin/src/lib.rs` (test literals), `lisa-core/tests/completion_state_machine.rs`.

Constants, `resolve_check_budget_secs`, the `check_timeout_secs` field, strict-path validation
(malformed / over cap / without check), tolerant-path clamping, `ParkedRemedy` passthrough.

**Tests (unit, `disposition.rs`):**
- strict: `{"check":"…","check_timeout_secs":1500}` accepted; `1801` names "lower
  check_timeout_secs"; `0`, `"20"`, `1.5` name the whole-number fix; a budget without a check names
  "add a check".
- tolerant: `1500` parses to `Some(1500)`; `9999` parses to `Some(1800)` (clamped); `0` and a
  string degrade to the unstructured operator fallback with no check.
- `resolve_check_budget_secs(None) == 5`, `(Some(9999)) == 1800`.

**Verify:** `cargo test -p lisa-core`.

## Step 2 — The runner moves out, and carries its budget (C2 machinery)

**Files:** new `crates/lisa-cli/src/check_run.rs`, `crates/lisa-cli/src/unblock.rs`,
`crates/lisa-cli/src/main.rs` (`mod check_run;`).

Move `run_check` and its helpers verbatim; add `CheckRun.budget`, `budget_for`, `format_budget`.
Move the six runner-level unit tests. `unblock.rs` keeps decline rendering and now formats from
`run.budget`.

**Tests:**
- `check_run.rs`: `format_budget` at 5 s / 90 s / 1500 s / 1800 s; a check that sleeps past a small
  budget times out and one inside it passes (millisecond budgets, no wall-clock cost).
- `unblock.rs`: `decline_header` at two budgets produces two different sentences, the 5-second one
  byte-identical to today's pinned string (**C2**).

**Verify:** `cargo test -p lisa-cli`.

## Step 3 — Both callers use the declared budget (C1 run-time half)

**Files:** `crates/lisa-cli/src/unblock.rs`.

`run_unblock` and `run_world_rechecks` resolve the budget from the remedy instead of the deleted
constant.

**Tests (black-box, `tests/parked_ux.rs`):**
- a blocked ticket whose check is `sleep 6 && test -f release-ready` with
  `"check_timeout_secs":30` unblocks, exit 0 (**C1** "past the default, inside the declared").
  One ~6 s test, in the same cost class as the existing 4–8 s timeout test.
- a check with `"check_timeout_secs":1` that sleeps 5 declines with a sentence naming 1 second and
  leaves the ticket blocked (**C1** "exceeding its declared budget times out", **C2** in the field).

**Verify:** `cargo test -p lisa-cli --test parked_ux`.

## Step 4 — `check-disposition` runs the check (C5)

**Files:** `crates/lisa-cli/src/check_disposition.rs`.

Run the recorded check under the resolved budget after schema validation; refuse `Inconclusive` and
`TimedOut` with a message naming what ran, where, the exit code, and what it printed; accept
`Passed` and `Failed` unchanged.

**Tests (black-box, `tests/check_disposition_cli.rs`):**
- the field case: `check` = `node scripts/check-touch.mjs` in a fixture with no such file → exit 1,
  message contains the command, `ran in`, exit code 127, and the check's own line (**C5**).
- a satisfiable check (`test -f release`, file absent → exit 1) passes unchanged (**C5**).
- an over-cap `check_timeout_secs` is refused *without running* — the check would `touch` a
  sentinel; the sentinel must not exist afterwards (**C1** "rejected when it is recorded, not when
  it is run").
- a slow check with a 1-second declared budget is refused as timed out.

**Verify:** `cargo test -p lisa-cli --test check_disposition_cli`.

## Step 5 — The world recheck records repeated non-passes (C6)

**Files:** `crates/lisa-core/src/provenance.rs`, `crates/lisa-cli/src/unblock.rs`.

Record type, append fn, ledger variant, `latest_world_rechecks` projection; the doubling schedule
in `run_world_rechecks`.

**Tests:**
- `provenance.rs` unit: a world-recheck row round-trips through the untagged `ProvenanceLedgerRecord`
  and is not mistaken for a check-override row; the projection keeps the latest per ticket and
  ignores unparseable lines.
- `tests/parked_ux.rs`: a world remedy whose check always fails, run three times through
  `lisa recheck-world`, produces rows at counts 1 and 2 (not 3), each naming the ticket, the check,
  the result and the exit code; the ticket is still `blocked`; stdout and stderr stay empty
  (**C6**).

**Verify:** `cargo test -p lisa-core && cargo test -p lisa-cli --test parked_ux`.

## Step 6 — The stuck world remedy is visible in `lisa status` (C6 visibility, N2)

**Files:** `crates/lisa-cli/src/status.rs`.

Threshold constant, projection load, two extra lines.

**Tests (unit, `status.rs`):** below the threshold the rendering is byte-identical to today; at the
threshold the two lines appear in order, name the count, and end in the exact
`lisa unblock <id> --override-check` command. The existing pinned black-box status tests are
untouched (they write no ledger rows).

**Verify:** `cargo test -p lisa-cli`.

## Step 7 — The contract is written down (C3, C4)

**Files:** `docs/knowledge/rdspi-workflow.md`, `crates/lisa-cli/data/rdspi-workflow.md` (identical
bodies), `crates/lisa-cli/src/templates.rs` (assertions).

**Tests (unit, `templates.rs`):**
- the documented default budget equals `DEFAULT_CHECK_BUDGET_SECS` and the documented cap equals
  `MAX_CHECK_BUDGET_SECS`, both asserted through `format!` against the constants, so the document
  cannot drift from the code (**C4**).
- the document states where a check runs, what it sees, that it must only look, and that
  `check-disposition` runs it (**C3**, **C4**).
- `test_rdspi_workflow_embedded` continues to assert the two files are byte-identical.

Plus the writing-check behaviour test in `check_run.rs` (**C3** "a test asserts the documented
behaviour for a writing check"): a check that creates a file and exits 0 is `Passed`, the file
exists afterwards, and no result variant reports the write.

**Verify:** `cargo test --workspace`.

## Step 8 — End to end and the gate (C7, C8)

**Files:** `crates/lisa-cli/tests/parked_ux.rs`.

The story fixture, all three ceilings at once: a git repository ignoring `out/`, a real `out/marker`
on disk, and a blocked ticket whose check is `sleep 6; test -f out/marker` with
`"check_timeout_secs":60`. `lisa unblock` exits 0, the ticket is `open`, the DAG shows it ready,
and no override row was written — it passed on its own merits (**C7**).

Then `just check` (**C8**): fmt, clippy, WASM check, workspace tests, by exit code.

---

## Testing strategy

- **Unit** where the fact is about one function's contract (budget resolution, exit
  classification, budget formatting, decline sentences, status rendering).
- **Black-box CLI** where the fact is about what an operator or reviewer actually experiences
  (unblock exit codes and copy, `check-disposition` refusals, the ledger rows a real
  `recheck-world` invocation writes). The story's whole lesson is that twenty passing unit tests
  missed a field failure that only the outside view could see.
- **Doc-versus-code** for anything a reviewer reads and the runtime enforces, asserted through
  `format!` against the constant rather than a copied literal.
- **Wall-clock discipline:** exactly two tests are allowed to spend real seconds (the 6-second
  declared-budget unblock and the E2E), matching the existing 4–8 s timeout test. Everything else
  uses millisecond budgets or no sleep at all.

## Risks

1. **A new `Block` field touches many literals.** Compile errors are the safety net; every site is
   enumerated in Structure §7.
2. **`serde(untagged)` ambiguity** between the check-override and world-recheck rows. Both carry a
   required single-variant `record_type`, which is what keeps every other variant disjoint; the
   round-trip test pins it.
3. **`check-disposition` now spends the check's time.** Bounded by the cap, and by the fact that
   only a block with a check runs anything.
4. **Doubling-schedule counting reads the ledger each cycle.** One read per `recheck-world`
   invocation, the same file `collect_parked_remedies` already reads.
