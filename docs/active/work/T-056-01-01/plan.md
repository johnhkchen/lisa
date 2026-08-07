# T-056-01-01 — Plan: ordered steps and verification

Seven steps. Steps 1–2 are independently verifiable; 3–4 must land together (the flag-audit test
enumerates the live Clap tree). Each step names its own gate; `just check` is the closing gate.

---

## Step 1 — The ledger row (`crates/lisa-core/src/provenance.rs`)

**Do**

1. `SCHEMA_VERSION` 9 → 10, and extend its doc comment with the version-10 sentence.
2. Add `CheckOverrideType`, `CheckOverrideOutcome`, `CheckOverrideRecord` (shape in
   structure.md §1), with the doc comment explaining why there is no `AttemptLease`.
3. Add `append_check_override_record`.
4. Add the `CheckOverride` arm to `ProvenanceLedgerRecord`, after `OperatorOverride`.
5. Update the four in-file tests that assert `"schema_version":9` to `10`. Leave every legacy
   fixture string (`SCHEMA_V2_EXECUTION_JSON`, the schema-3 and schema-4 raw rows) untouched.

**Tests written in this step**

- `check_override_record_round_trips_through_the_mixed_ledger`
- `check_override_row_does_not_absorb_or_get_absorbed`
- `usage_fold_ignores_check_override_rows`

**Verify**

```bash
cargo test -p lisa-core provenance
cargo test --workspace          # catches any other pinned "schema_version":9 outside this file
```

The workspace run matters here: `lisa-plugin` writes ledger rows and has ~20 tests reading them.
Any assertion elsewhere on the literal 9 surfaces now, not at the end.

**Commit** — `lisa commit-ticket --ticket-id T-056-01-01 --message "record a forced unblock in
the ledger" --include crates/lisa-core/src/provenance.rs`

---

## Step 2 — Report the check (`crates/lisa-cli/src/unblock.rs`)

The largest step, but it is one coherent unit: nothing outside the file compiles differently until
`run_unblock`'s signature changes, which happens at its end.

**Do, in order**

1. Add `MAX_OBSERVED_LINES` and the four header consts.
2. Replace `CheckResult` with the five-variant payload-free enum; add `CheckRun`.
3. Rewrite `run_check`'s tail: clone the directory out of the snapshot before spawning, read both
   captures once after the wait, build sanitized/capped line vectors, classify by
   `timed_out` → fingerprint drift → `status.code()`, return one `CheckRun`.
4. Add `observed_lines`; delete `observed_line`.
5. Replace `decline_message` with `decline_report(ticket_id, &run)`.
6. Update `run_world_rechecks`'s match to `.result` with the five arms.
7. Change `run_unblock` to take `override_check: bool`, add the override branch and
   `record_check_override`, and branch the reopen message.

**Classification rule pinned in code** (design decision 2):

| `status.code()` | result |
| --- | --- |
| `Some(0)` | `Passed` |
| `Some(2 \| 126 \| 127)` | `Inconclusive` |
| `Some(_)` | `Failed` |
| `None` | `Inconclusive` |

with `timed_out` and fingerprint drift taking precedence, in that order (unchanged from today).

**Tests written in this step** (unit, in-file)

| Test | Pins |
| --- | --- |
| `passing_and_failing_checks_carry_the_command_directory_and_code` | criterion 1's facts reach the caller; `directory` equals what the check's own `pwd` printed |
| `exit_two_and_shell_failures_are_inconclusive_not_a_verdict` | 2/126/127 → `Inconclusive`; 1/3 → `Failed` |
| `timeout_is_bounded_and_kills_the_shell_group` | existing timing property + the timeout header string byte-identical |
| `mutation_inside_disposable_state_is_detected_even_after_chmod` | existing + `ChangedFiles` header byte-identical |
| `relative_write_never_reaches_live_project_and_cannot_pass` | existing, unchanged behaviour |
| `observed_lines_strip_controls_cap_length_and_cap_count` | 240-char cap, tab folding, ANSI stripping, 10-line display cap and its dropped count |
| `every_decline_header_is_distinct_and_names_the_way_through` | four distinct headers; each report names `--override-check` |
| `the_field_line_is_reported_not_asserted_as_lisas_verdict` | criterion 6 at unit level: the field sentence appears only under the stderr label, never on the header line |

`run_check` is called against `tempfile::tempdir()` roots, matching the existing tests' style. The
`pwd` assertion is the one that guards the T-056-01-02 hand-off: it compares the reported
`directory` with the directory the check itself observed, so if that ticket moves the cwd, this
test moves with it rather than pinning a stale path. (Canonicalize both sides — macOS `/var` is a
symlink to `/private/var`, and `pwd` in `sh` resolves it while `TempDir::path` does not.)

**Verify**

```bash
cargo test -p lisa-cli --lib unblock
```

Expected to be red at first for the `main.rs` call site until step 3; run
`cargo check -p lisa-cli` and treat only `run_unblock`'s arity error as acceptable, or fold step 3
in before running. In practice: make the edit in step 2, then step 3 immediately, then test.

**Commit** — after step 3, since the two are one compiling unit:
`--include crates/lisa-cli/src/unblock.rs crates/lisa-cli/src/main.rs`

---

## Step 3 — The flag (`crates/lisa-cli/src/main.rs`)

**Do**

1. Add `override_check: bool` with `#[arg(long)]` to the `Unblock` variant, declared **before**
   `path` (help ordering, and the flag-audit row's position).
2. Doc comment: `Let the ticket run again even when its check declines, and record that you
   overrode it`. Checked against the banned-jargon list — contains none of the nine terms.
3. Pass it to `unblock::run_unblock(&path, &ticket_id, override_check)`.

**Verify**

```bash
cargo run -p lisa-cli -- unblock --help
cargo test -p lisa-cli --lib               # flag_audit_* now fails: expected, fixed in step 4
```

---

## Step 4 — Documentation gates (`flag-audit.md`, `help_surface.rs`, `README.md`)

These three are one unit: the flag-audit test and the help snapshot both fail until they are
updated, and the README is the criterion-3 "documented" surface.

**Do**

1. `docs/knowledge/flag-audit.md`: insert the `flag:lisa/unblock:--override-check` row above the
   existing `--path` row, exactly as drafted in structure.md §4.
2. `crates/lisa-cli/tests/help_surface.rs`: update the `unblock` snapshot. Take the exact padding
   from the assertion failure output rather than hand-computing Clap's column alignment.
3. `README.md`: extend `### lisa unblock` with the override and the fact that using it is
   recorded.

**Verify**

```bash
cargo test -p lisa-cli --lib flag_audit
cargo test -p lisa-cli --test help_surface
```

Both must be green before moving on; `flag_audit_missing_row_fixture_names_every_gap` must stay
green too (it uses a separate fixture and should be unaffected).

**Commit** — `--include docs/knowledge/flag-audit.md crates/lisa-cli/tests/help_surface.rs
README.md`

---

## Step 5 — Black-box acceptance tests (`crates/lisa-cli/tests/parked_ux.rs`)

The criteria are written in operator-visible terms, so this is where most of them are pinned.

**Do**

1. Add `unblock_with(root, ticket_id, extra: &[&str])`; keep `unblock` as a thin wrapper so the
   untouched tests stay as they are.
2. Add a `provenance_rows(root) -> Vec<serde_json::Value>` helper reading
   `.lisa/provenance.jsonl` (absent file → empty vec).
3. Rewrite the two tests whose assertions describe the old one-line rendering.
4. Add the five new tests listed in structure.md §6.

**Criterion → test map** (the checklist this ticket is graded on)

| Criterion | Test |
| --- | --- |
| 1 — command, cwd, exit code, attributed both streams | `a_declined_check_reports_the_command_the_directory_the_code_and_both_streams` |
| 2 — distinct inconclusive variant and wording; `TimedOut`/`ChangedFiles` still distinct | unit `every_decline_header_is_distinct_and_names_the_way_through` + `a_check_that_cannot_look_reads_as_inconclusive_not_as_a_verdict` |
| 3 — documented override flag; with it `Reopened`/exit 0, without it unchanged | `override_check_reopens_and_leaves_a_record` (both halves) + README + help snapshot |
| 4 — durable record naming the override and the actor; none for an ordinary unblock | `override_check_reopens_and_leaves_a_record` (asserts the row with the flag, and no row without) |
| 5 — sanitization still applies to everything shown | `escape_sequences_and_tabs_are_stripped_from_everything_shown` |
| 6 — the field line reproduced then improved | `a_check_that_cannot_look_reads_as_inconclusive_not_as_a_verdict` |
| 7 — `just check` green | step 7 |

The criterion-6 fixture is the field case verbatim:

```json
"check": "printf 'No build at dist/. Run: npm run build\n' >&2; exit 2"
```

and asserts: the output names the command, names a `ran in:` directory, says `exit code: 2`, does
**not** start with `That didn't work yet — No build at dist/`, and carries the sentence only under
the stderr attribution label.

**Verify**

```bash
cargo test -p lisa-cli --test parked_ux
```

---

## Step 6 — Sweep for other callers and stale strings

**Do**

```bash
rg -n "That didn't work yet" --type rust --type md
rg -n "run_unblock|decline_message|observed_line|CheckResult" --type rust
rg -n '"schema_version":9|SCHEMA_VERSION' --type rust
```

Confirm: no plugin-side copy of the decline strings, no other `run_unblock` caller, no remaining
reference to the deleted helpers. `crates/lisa-plugin/src/lib.rs:9068` performs unblock's *flip*
in-plugin but does not call this code and does not run checks — it is out of scope and must stay
untouched.

---

## Step 7 — Full gate

```bash
just check      # cargo check --target wasm32-wasip1, fmt --check, clippy -D warnings, cargo test --workspace
```

Judged by **exit code**, not by reading output. If clippy objects to `CheckRun`'s field count or
`record_check_override`'s argument count, prefer restructuring (pass `&CheckRun`, group args) over
an `#[allow]`; the file has no existing allows.

**Commit** — `--include crates/lisa-cli/tests/parked_ux.rs` (and any file the gate forced a
formatting change into).

---

## Testing strategy, stated plainly

- **Unit tests in `unblock.rs`** own the classification table, the sanitizer, and the four header
  strings — the things that are cheap to get wrong and cheap to pin.
- **Black-box tests in `parked_ux.rs`** own everything a criterion phrases as "the output
  reports…" or "leaves a record". They drive the real binary, so they also prove the exit codes.
- **`provenance.rs` tests** own the wire format and the untagged-enum disjointness, matching how
  every previous row shape was introduced.
- No test asserts on a hard-coded temp path; the cwd assertion compares Lisa's reported directory
  against what the check itself observed.

## Risks and how each is handled

| Risk | Handling |
| --- | --- |
| `SCHEMA_VERSION` bump breaks a pinned assertion in `lisa-plugin` | Step 1 ends with a full `cargo test --workspace`, before any CLI work is layered on |
| Help snapshot padding guessed wrong | Take it from the assertion diff, not by hand |
| macOS `/var` symlink makes the `pwd` comparison flaky | Canonicalize both sides in the assertion |
| Multi-line stderr breaks an assumption elsewhere | Step 6's `rg` sweep for the decline strings |
| Ledger append fails and leaves a reopened ticket unrecorded | Ordering: receipt first, flip second (design decision 3) |
| Scope creep into T-056-01-02/03 | `CHECK_TIMEOUT`, the snapshot, and `rdspi-workflow.md` are read but never modified |
