# T-056-01-03 Structure — file-level blueprint

Ordering matters: core schema → core ledger → CLI runner extraction → CLI callers → docs → tests.
Each step below compiles on its own.

---

## 1. `crates/lisa-core/src/disposition.rs` (modified)

**New public constants**, immediately above `RemedyOwner`:

```rust
/// Seconds a recorded check gets when it declares no budget of its own.
pub const DEFAULT_CHECK_BUDGET_SECS: u64 = 5;
/// The most a check may declare. Documented in docs/knowledge/rdspi-workflow.md.
pub const MAX_CHECK_BUDGET_SECS: u64 = 1800;
```

**New public function**:

```rust
/// Resolve a declared budget to the seconds a check actually gets.
/// `None` -> default; `Some(n)` -> n clamped to the cap.
pub fn resolve_check_budget_secs(declared: Option<u64>) -> u64
```

**`ReviewDisposition::Block` gains** `check_timeout_secs: Option<u64>` (documented: seconds the
check declared, already clamped to the cap by the parser; `None` means the default).

**`check_block_document`** (strict authoring path) gains, after `check`:

```rust
let check_timeout_secs = match object.remove("check_timeout_secs") {
    None => None,
    Some(Value::Number(n)) if n.as_u64().is_some_and(|s| s >= 1 && s <= MAX) => Some(..),
    Some(Value::Number(n)) if over cap => Err("lower check_timeout_secs to at most 1800 seconds (30 minutes)"),
    Some(_) => Err("make check_timeout_secs a whole number of seconds from 1 to 1800, or omit it"),
};
if check_timeout_secs.is_some() && check.is_none() {
    return Err("add a check, or remove check_timeout_secs");
}
```
The over-cap arm is separate from the malformed arm because the fixes differ.
The trailing `!object.is_empty()` message gains the field name.

**`validate_block_structure`** (tolerant path) parses the same field inside the existing structure
closure: absent → `None`; a `u64 >= 1` → `Some(min(value, MAX))`; anything else → `None` from the
closure, i.e. the existing `unstructured_block` fallback.

**`unstructured_block`** sets `check_timeout_secs: None`.

## 2. `crates/lisa-core/src/parking.rs` (modified)

- `ParkedRemedy` gains `pub check_timeout_secs: Option<u64>` next to `check`.
- The exhaustive `let ReviewDisposition::Block { .. }` destructure at ~156 gains the field, and the
  `ParkedRemedy` literal passes it through.

## 3. `crates/lisa-core/src/provenance.rs` (modified)

New, placed after the `CheckOverride*` group:

```rust
pub enum WorldRecheckType { WorldRecheck }              // serde kebab-case -> "world-recheck"
pub enum WorldRecheckOutcome { Failed, Inconclusive, TimedOut }  // kebab-case
pub struct WorldRecheckRecord {
    schema_version, seal, record_type, ticket_id,
    check: String, directory: String,
    result: WorldRecheckOutcome,
    exit_code: Option<i32>,          // skip_serializing_if none
    observed: Vec<String>,           // skip_serializing_if empty
    non_pass_count: u64,             // running total at the moment this row was written
    occurred_at: u64,
}
pub fn append_world_recheck_record(path, record) -> io::Result<()>
```

`ProvenanceLedgerRecord` gains `WorldRecheck(WorldRecheckRecord)` between `CheckOverride` and
`UsageCorrection`. Disjointness is by `record_type`, as with every other variant.

**Projection**, next to the record:

```rust
/// The latest recorded non-pass per ticket, folded from the ledger.
pub struct WorldRecheckObservation { pub check: String, pub result: WorldRecheckOutcome,
                                     pub non_pass_count: u64, pub occurred_at: u64 }
pub fn latest_world_rechecks(ledger_path: &Path) -> HashMap<String, WorldRecheckObservation>
```

Last row per `ticket_id` wins (append order is chronological). Unreadable ledger → empty map, the
established "cannot establish a fact" behaviour.

## 4. `crates/lisa-cli/src/check_run.rs` (new)

The execution contract, moved verbatim out of `unblock.rs` except where noted. Module doc states
the contract in the same terms as the workflow document, and points at it.

```
pub(crate) enum CheckResult { Passed, Failed, Inconclusive, TimedOut }
pub(crate) struct CheckRun { result, check, directory, budget: Duration, exit_code,
                             stdout, stderr, stdout_dropped, stderr_dropped }
pub(crate) fn run_check(root: &Path, check: &str, budget: Duration) -> Result<CheckRun, String>
pub(crate) fn budget_for(declared: Option<u64>) -> Duration     // wraps resolve_check_budget_secs
pub(crate) fn format_budget(budget: Duration) -> String         // "5 seconds" / "25 minutes"
fn classify_exit, terminate_check, read_capture, observed_lines, sanitize_observation
const POLL_INTERVAL, MAX_CAPTURE_BYTES, MAX_OBSERVATION_CHARS, MAX_OBSERVED_LINES
```

New in the moved code: `CheckRun.budget`, set from the `budget` argument.

Tests that move with it: the six `run_check`-level tests currently in `unblock.rs`
(`passing_and_failing_checks_carry_…`, `the_reported_directory_…`, `exit_two_and_shell_failures_…`,
`a_check_reads_the_project_it_runs_in`, `the_check_runs_in_the_project_root`,
`observed_lines_strip_controls_…`) minus the parts that assert decline copy, which stay behind.
New tests here: `format_budget`, budget resolution and clamping, a writing check (D3).

`main.rs` gains `mod check_run;`.

## 5. `crates/lisa-cli/src/unblock.rs` (modified)

Removed: everything listed under §4, plus the `CHECK_TIMEOUT` constant.
Kept and adjusted:

- `run_unblock` — `run_check(root, &check, check_run::budget_for(remedy.check_timeout_secs))`.
- `decline_timed_out(budget: Duration)` / `exit_code_line` — format from `run.budget`.
- `decline_header(run: &CheckRun)` — takes the run rather than the result, so the timeout arm can
  reach the budget. Its callers are `decline_report` and tests.
- `record_check_override`, `override_outcome` — unchanged.
- `run_world_rechecks` — the non-pass arm becomes `record_world_non_pass(...)`:

```rust
let observations = latest_world_rechecks(&ledger);      // once, before the loop
…
non_pass => {
    let previous = observations.get(&id).filter(|o| o.check == check).map_or(0, |o| o.non_pass_count);
    let count = previous + 1;
    if count.is_power_of_two() { append_world_recheck_record(...)?; }
}
```
plus a private `world_outcome(CheckResult) -> WorldRecheckOutcome` mirroring `override_outcome`.
Errors from the append are reported as command errors (`Could not record …`), consistent with
`record_check_override`.

## 6. `crates/lisa-cli/src/check_disposition.rs` (modified)

After the existing schema + ask validation:

```rust
if let ReviewDisposition::Block { check: Some(check), check_timeout_secs, .. } = &disposition {
    let run = check_run::run_check(project_root, check, budget_for(*check_timeout_secs))?;
    match run.result {
        Passed | Failed => {}                       // a not-yet-done remedy is the ordinary case
        Inconclusive | TimedOut => return Err(fix(unrunnable_check_message(&run))),
    }
}
```

`unrunnable_check_message` is local to this module (reviewer-facing copy, distinct from the
operator decline): one lead sentence naming the class, then `what ran` / `ran in` / `exit code` /
first observed lines, then what to do about it. Success copy gains a clause naming that the check
ran and what it reported, so a reviewer sees the check was exercised.

## 7. `crates/lisa-cli/src/status.rs` (modified)

`waiting_on_you_lines(remedies, rechecks: &HashMap<String, WorldRecheckObservation>)` — for a
world-owned remedy whose observation matches the remedy's check and whose `non_pass_count >=
STUCK_NON_PASS_COUNT` (8), push two lines after the lead:

```
       Lisa has checked at least {n} times and it still isn't passing.
       If you have checked this yourself, run: lisa unblock {id} --override-check
```

`run_status` loads the projection from the same ledger path it already passes to
`collect_parked_remedies`.

## 8. Documentation (both copies, identical)

`docs/knowledge/rdspi-workflow.md` and `crates/lisa-cli/data/rdspi-workflow.md` — after the
`remedy_owner` / `check` paragraph (~line 59), a new block:

- what a check is for (unchanged sentence, kept),
- **where it runs** — the project root, the operator's own tree,
- **what it sees** — every file really there, gitignored build output included,
- **writes** — must only look; Lisa runs it in the live project and cannot stop it; therefore
  `npm run build && npm run verify` is not a check,
- **how long** — 5 seconds, `check_timeout_secs` up to 1800 (30 minutes), expiry says how long it
  waited,
- **`lisa check-disposition` runs it** — a check that cannot run is refused here, while the
  reviewer can still fix it.

The `{"disposition":"block",…}` example line gains `"check_timeout_secs":<optional seconds>`; the
pinned assertion in `templates.rs:763` is updated to the new literal.

## 9. Tests

| File | Adds |
| --- | --- |
| `lisa-core/src/disposition.rs` | budget accepted / clamped / refused over cap / refused without check / malformed → fallback |
| `lisa-core/src/provenance.rs` | world-recheck row round-trips through the untagged enum; projection keeps the latest per ticket |
| `lisa-cli/src/check_run.rs` | `format_budget` units; declared budget honoured; a writing check runs and is judged by exit code alone |
| `lisa-cli/src/unblock.rs` | timeout sentence at two budgets (criterion 2) |
| `lisa-cli/src/templates.rs` | documented default == `DEFAULT_CHECK_BUDGET_SECS`, documented cap == `MAX_CHECK_BUDGET_SECS`, the write sentence is present (criterion 4) |
| `lisa-cli/src/status.rs` | the stuck-world lines appear at the threshold and not below it |
| `tests/check_disposition_cli.rs` | field case refused at record time; satisfiable check passes unchanged; over-cap budget refused without running |
| `tests/parked_ux.rs` | declared budget lets a slow check pass (criterion 1 + 7); world remedy that always fails writes the record and stays parked (criterion 6) |

Two existing tests are revised, deliberately:
`world_owned_failing_check_stays_parked_without_churn` (stderr/stdout stay empty — still true; it
gains an assertion that the ledger row exists) and
`automatic_recheck_timeout_is_bounded_and_cannot_reopen` (unchanged behaviour, gains the row).
