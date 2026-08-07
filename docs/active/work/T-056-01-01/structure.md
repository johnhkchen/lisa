# T-056-01-01 — Structure: file-level blueprint

Six files change, one of them in `lisa-core`. Nothing is created or deleted. Order matters only
between §1 (the ledger row) and §2 (its writer).

---

## 1. `crates/lisa-core/src/provenance.rs` — modified

The new ledger row. Additive; no existing shape changes.

**Constant**

```rust
pub const SCHEMA_VERSION: u32 = 10;   // was 9
```

Doc comment gains: "Version 10 adds the check-override row that records an operator reopening a
parked ticket over its own check (T-056-01-01)."

The three existing tests that assert `"schema_version":9` in serialized rows
(`record_serializes_to_one_compact_line`, `assignment_transition_serializes_to_one_compact_line`,
`parking_transitions_serialize_and_round_trip_as_compact_rows`,
`usage_correction_serializes_to_one_compact_line_and_round_trips`) update to `10`. The
legacy-parse tests (`schema_four_parking_rows_...`, `pre_ladder_assignment_rows_...`,
`SCHEMA_V2_EXECUTION_JSON`) are untouched — that is the point of them.

**New public types**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckOverrideType { CheckOverride }

/// What the overridden check reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckOverrideOutcome { Failed, Inconclusive, TimedOut, ChangedFiles }

/// One parked ticket a person reopened over its own check.
pub struct CheckOverrideRecord {
    pub schema_version: u32,
    #[serde(default)] pub seal: CompletionSeal,
    pub record_type: CheckOverrideType,
    pub ticket_id: String,
    /// Who overrode. `"operator"` for `lisa unblock --override-check`.
    pub actor: String,
    /// The check string exactly as the disposition recorded it.
    pub check: String,
    /// The directory the check actually ran in.
    pub directory: String,
    pub result: CheckOverrideOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub exit_code: Option<i32>,
    /// The sanitized lines the operator saw, capped as displayed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub observed: Vec<String>,
    pub occurred_at: u64,
}
```

Doc comment states the shape reasoning (no `AttemptLease`; the parking attempt is gone; a
synthesized lease would file a fabricated run) — mirroring `OperatorOverrideRecord:302-310`.

**New writer**

```rust
pub fn append_check_override_record(path: &Path, record: &CheckOverrideRecord)
    -> std::io::Result<()> { append_serialized(path, record) }
```

**Enum arm** — inserted in `ProvenanceLedgerRecord` immediately after `OperatorOverride`, before
`UsageCorrection` and `Execution`:

```rust
CheckOverride(CheckOverrideRecord),
```

Ordering rationale: untagged resolution is first-match, and `Execution` (no `record_type`) must
stay last. Placement among the `record_type`-bearing arms is not load-bearing because each one's
`record_type` enum accepts exactly one string, but the ordering convention is preserved.

**Tests added here**

- `check_override_record_round_trips_through_the_mixed_ledger` — append, one line,
  `"record_type":"check-override"`, `"actor":"operator"`, parses back as
  `ProvenanceLedgerRecord::CheckOverride`.
- `check_override_row_does_not_absorb_or_get_absorbed` — extends the existing precedent test's
  shape: the new line parses to the new arm, and each of execution / assignment / parking /
  usage-correction / operator-override still parses to its own arm.
- `usage_fold_ignores_check_override_rows` — `correct_usage` is unaffected.

---

## 2. `crates/lisa-cli/src/unblock.rs` — modified (the bulk of the work)

### 2a. Types

```rust
const MAX_OBSERVED_LINES: usize = 10;            // new: display cap, per stream

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckResult { Passed, Failed, Inconclusive, TimedOut, ChangedFiles }

/// One check run, and every fact needed to report it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckRun {
    result: CheckResult,
    /// The recorded check string, verbatim.
    check: String,
    /// The directory passed to `.current_dir` — reported, never recomputed.
    directory: PathBuf,
    /// `None` when the check was stopped rather than exiting.
    exit_code: Option<i32>,
    /// Sanitized, non-empty lines, capped for display.
    stdout: Vec<String>,
    stderr: Vec<String>,
    /// Lines dropped by the display cap, per stream.
    stdout_dropped: usize,
    stderr_dropped: usize,
}
```

`CheckResult` loses its `Failed(String)` payload; the observation now lives in `stderr`/`stdout`.

### 2b. `run_check` — same body, new return

Signature becomes `fn run_check(root: &Path, check: &str, timeout: Duration) -> Result<CheckRun, String>`.

Changes, in place:

1. Capture `let directory = snapshot.path().to_path_buf();` before spawning (the snapshot's
   `TempDir` is dropped at function end, so the value must be cloned out, not borrowed).
2. Move both `read_capture` calls up, to just after the wait loop and before any classification
   branch — so `TimedOut` and `ChangedFiles` also carry evidence.
3. Build the sanitized/capped line vectors through a new helper.
4. Classify:
   - `timed_out` → `TimedOut`, `exit_code: None`
   - fingerprint drift → `ChangedFiles`, `exit_code: status.code()`
   - `status.code()` → `Some(0)` `Passed` / `Some(2|126|127)` `Inconclusive` / `Some(_)` `Failed`
     / `None` `Inconclusive`
5. Return one `CheckRun` from a single construction site.

### 2c. New helpers

```rust
/// Sanitize, drop blanks, cap for display; returns (lines, dropped_count).
fn observed_lines(bytes: &[u8]) -> (Vec<String>, usize)
```

Replaces `observed_line`'s role. `observed_line` itself is **removed** — nothing else calls it,
and its stderr-then-stdout preference is superseded by showing both streams labelled. Its test
(`observation_prefers_stderr_removes_controls_and_caps_length`) is rewritten against
`observed_lines`, keeping the same three properties (control stripping, tab folding, 240-char cap)
plus the new line cap.

`sanitize_observation` is unchanged.

### 2d. Rendering

```rust
fn decline_report(ticket_id: &str, run: &CheckRun) -> String
```

Replaces `decline_message(CheckResult) -> String`. Layout exactly as Design §1:

```
<header>
<blank>
  what ran:  <check>
  ran in:    <directory>
  exit code: <code | "none — Lisa stopped it" | "none — the check was stopped">
<blank>
  the check wrote to stderr:            (omitted when empty)
    <line>…
    … (N more lines)                    (only when dropped > 0)
  the check wrote to stdout:            (omitted when empty)
    <line>…
  the check printed nothing.            (only when both are empty)
<blank>
If you have done this and checked it yourself, run:
  lisa unblock <ticket-id> --override-check
```

Header strings are four `const`s at module top so tests pin one source:

```rust
const DECLINE_FAILED: &str   = "That didn't work yet — the check ran and did not pass.";
const DECLINE_INCONCLUSIVE: &str = "Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on your work.";
const DECLINE_CHANGED_FILES: &str = "That didn't work yet — it tried to change project files.";
// timeout is formatted from CHECK_TIMEOUT so the constant stays the single source:
// format!("That didn't work yet — it took longer than {} seconds.", CHECK_TIMEOUT.as_secs())
```

`Passed` is unreachable here and keeps the existing `unreachable!` guard.

### 2e. `run_unblock` — signature and control flow

```rust
pub fn run_unblock(root: &Path, ticket_id: &str, override_check: bool)
    -> Result<UnblockOutcome, String>
```

The check gate becomes:

```rust
if let Some(check) = remedy.check {
    let run = run_check(root, &check, CHECK_TIMEOUT)?;
    if run.result != CheckResult::Passed {
        if !override_check {
            return Ok(UnblockOutcome::Declined(decline_report(ticket_id, &run)));
        }
        record_check_override(root, &resolved, ticket_id, &run)?;   // before the flip
        overrode = true;
    }
}
```

Everything before the check gate is untouched, so the four non-check declines keep their exact
copy and ignore the flag.

Reopen message:

```rust
if overrode { format!("{ticket_id} can run again — you overrode its check.") }
else        { format!("{ticket_id} can run again.") }
```

### 2f. The receipt writer

```rust
fn record_check_override(
    root: &Path,
    resolved: &config::ResolvedConfig,
    ticket_id: &str,
    run: &CheckRun,
) -> Result<(), String>
```

Builds `CheckOverrideRecord` with `actor: "operator"`, `seal:
completion_seal::resolve_for_inspection(root, resolved.completion_mode)`, `occurred_at:
system_time_to_epoch(SystemTime::now())`, `observed:` stderr lines then stdout lines (the display
order), and appends to `root.join(".lisa/provenance.jsonl")` — the same path
`collect_parked_remedies` is already given at `:66`. On error:

```
Err("Could not record the override: {error}")
```

which `main.rs` prints as `Error: …` and exits 1, before any status flip.

### 2g. `run_world_rechecks`

One line changes — the do-nothing arm gains the new variant and drops the payload:

```rust
match run_check(root, &check, CHECK_TIMEOUT)?.result {
    CheckResult::Passed => { /* unchanged reopen */ }
    CheckResult::Failed | CheckResult::Inconclusive
        | CheckResult::TimedOut | CheckResult::ChangedFiles => {}
}
```

Automation gains no override powers; its silence is T-056-01-03's.

### 2h. Unit tests in this file

Rewritten: `passing_and_failing_checks_report_one_plain_observation` (now asserts classification
plus the carried facts), `timeout_is_bounded_and_kills_the_shell_group`,
`relative_write_never_reaches_live_project_and_cannot_pass`,
`mutation_inside_disposable_state_is_detected_even_after_chmod`,
`observation_prefers_stderr_removes_controls_and_caps_length` → `observed_lines_*`.

Added:
- `exit_two_and_shell_failures_are_inconclusive_not_a_verdict` — 2, 126, 127 → `Inconclusive`;
  1 and 3 → `Failed`.
- `every_decline_header_is_distinct_and_names_the_way_through`.
- `the_field_line_is_reported_not_asserted_as_lisas_verdict` — criterion 6, at the unit level.

---

## 3. `crates/lisa-cli/src/main.rs` — modified

Clap definition (`:126-133`) gains:

```rust
/// Let the ticket run again even when its check declines, and record that you overrode it
#[arg(long)]
override_check: bool,
```

Dispatch (`:613-626`) passes `override_check` through. Nothing else changes — the
`Reopened`/`Declined`/`Err` handling and exit codes stay as they are, so a multi-line decline
prints on stderr with exit 1 exactly as a one-line one did.

Doc-comment wording is checked against the banned-jargon list (`dag`, `orchestrat`, `scheduling`,
`leverage`, `solutions`, `deployment`, `case study`, `build log`, `research release`) — it
contains none.

---

## 4. `docs/knowledge/flag-audit.md` — modified

One row, inserted in the everyday-commands table in the existing `lisa unblock` position (rows are
grouped by command, and `--override-check` sorts before `--path`):

```
| `flag:lisa/unblock:--override-check` | Let a waiting ticket run again over its own check | working default | Default is off, so Lisa's check still decides unless you say you checked it yourself. | `override_check_reopens_and_leaves_a_record` | — |
```

Satisfies `validate_row_policy`: bar `working default` (Clap `required=false`), fixture named and
not `—`, category `—`, rule ends in a period, no banned terms in surface or rule.

---

## 5. `crates/lisa-cli/tests/help_surface.rs` — modified

The `unblock` snapshot (`:146-161`) gains the flag line. Clap renders long-flag options in
declaration order, so with `override_check` declared before `path`:

```
Options:
      --override-check  Let the ticket run again even when its check declines, and record that you overrode it
      --path <PATH>     Path to the project root (defaults to current directory) [default: .]
  -h, --help            Print help
```

Exact column padding is taken from the failing assertion's diff rather than guessed.

---

## 6. `crates/lisa-cli/tests/parked_ux.rs` — modified

Helper gains a variant:

```rust
fn unblock_with(root: &Path, ticket_id: &str, extra: &[&str]) -> Output
```

Rewritten:
- `failing_check_declines_plainly_and_leaves_the_ticket_waiting` — asserts the header, the three
  labelled facts, the attributed stderr section, the absence of `Error:`, the ticket still
  blocked, and that the check's sentence is **not** on the header line.
- `attempted_write_is_disposable_reported_plainly_and_does_not_reopen` — the
  `lines().count() == 1` assertion becomes a structural assertion over the same output.

Added (the black-box half of the acceptance criteria):
- `a_check_that_cannot_look_reads_as_inconclusive_not_as_a_verdict` — criteria 2 and 6: a check
  printing `No build at dist/. Run: npm run build` to stderr and exiting 2; asserts the
  inconclusive header, `exit code: 2`, the command, the cwd line, and that the script's sentence
  appears only under the attribution label.
- `a_declined_check_reports_the_command_the_directory_the_code_and_both_streams` — criterion 1,
  fixture writing known bytes to both streams and exiting 3.
- `escape_sequences_and_tabs_are_stripped_from_everything_shown` — criterion 5.
- `override_check_reopens_and_leaves_a_record` — criterion 3 and 4: same fixture, with and
  without the flag; asserts exit 0 + `Reopened` copy + a `check-override` row naming
  `"actor":"operator"` with the flag, and exit 1 + no such row without it.
- `override_check_does_not_bypass_the_non_check_declines` — the flag leaves
  `T-X isn't waiting.` and the `already-done` hand-off exactly as they are.

---

## 7. `README.md` — modified

`### lisa unblock` (`:440-447`) gains two sentences and a second example naming the flag and the
fact that a forced unblock is recorded. This is the "documented" half of criterion 3.

---

## Ordering of changes

1. `provenance.rs` (row + `SCHEMA_VERSION` + its tests) — nothing depends on the CLI.
2. `unblock.rs` (`CheckRun`, classification, rendering, receipt, `run_world_rechecks`).
3. `main.rs` (flag + dispatch) — needs `run_unblock`'s new signature.
4. `flag-audit.md` + `help_surface.rs` — need the live flag to exist.
5. `parked_ux.rs` black-box tests — need the whole path.
6. `README.md`.

Steps 1–2 are independently compilable and testable; 3–4 must land together or
`flag_audit_covers_live_cli_config_and_prompts` fails.
