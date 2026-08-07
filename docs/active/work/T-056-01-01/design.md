# T-056-01-01 — Design: say-what-was-run-and-where

Three decisions, each with the options considered and what was rejected: **what a check-caused
decline says**, **where the line between "did not pass" and "could not look" is drawn**, and
**what a forced unblock leaves behind**.

---

## Decision 1 — Carry the facts out of `run_check`, render them in one block

### The problem, restated structurally

`run_check` returns a `CheckResult` (`unblock.rs:33-39`). The check string, the directory passed
to `.current_dir`, the `ExitStatus`, and both captures are all local and are dropped at the
return. No rewording of `decline_message` can fix that, because the facts never reach it.

### Options

**(1a) Widen `CheckResult`'s payloads.** `Failed { exit_code, stdout, stderr }`, etc. Rejected:
the check string and directory belong to *every* variant including `Passed`, so they would be
duplicated across four arms, and `run_world_rechecks`'s match would carry payloads it never reads.

**(1b) Return a report struct wrapping the classification.** `run_check` returns `CheckRun {
result, check, directory, exit_code, stdout, stderr }` where `CheckResult` becomes a plain
classification enum with no payload. Rendering takes `&CheckRun`. **Chosen.** One place holds the
facts, `run_world_rechecks` keeps matching on `run.result` and reads nothing else, and the
reporting site has everything criterion 1 names without recomputing any of it.

**(1c) Log to a file and point at it.** Rejected: the operator is already at a dead end; a second
hop is another named state with an extra step, not a way through.

### The rendering

Four headers — one per non-passing classification — followed by one evidence block. Header
sentences for `TimedOut` and `ChangedFiles` are the **existing strings, byte for byte**, which is
what criterion 2 asks to keep distinct:

| Classification | Header line |
| --- | --- |
| `Failed` | `That didn't work yet — the check ran and did not pass.` |
| `Inconclusive` | `Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on your work.` |
| `TimedOut` | `That didn't work yet — it took longer than 5 seconds.` |
| `ChangedFiles` | `That didn't work yet — it tried to change project files.` |

Then, indented two spaces:

```
  what ran:  node scripts/check-touch.mjs
  ran in:    /private/var/folders/x9/…/T/.tmpAbC123
  exit code: 2

  the check wrote to stderr:
    No build at dist/. Run: npm run build

If you have done this and checked it yourself, run:
  lisa unblock T-010-03 --override-check
```

Rules:

- `what ran:` is the recorded check string, verbatim, sanitized.
- `ran in:` is the directory **`run_check` actually passed to `.current_dir`** — carried in the
  report, never recomputed. This is the ticket's explicit instruction for the T-056-01-02 overlap.
- `exit code:` is the number when there is one. Where there is none it says so plainly rather than
  inventing a zero: `none — Lisa stopped it` (timeout), `none — the check was stopped` (killed by
  a signal).
- Each stream gets its own labelled section, present only when that stream has content:
  `the check wrote to stderr:` / `the check wrote to stdout:`. When both are empty:
  `the check printed nothing.` Nothing the check said ever appears on Lisa's header line — that
  is the attribution failure this ticket exists to fix.
- Every displayed line passes through the existing `sanitize_observation`; lines that sanitize to
  empty are dropped.
- The last two lines name the override, on **every** check-caused decline, not only the
  inconclusive one. Criterion 2 pins it for `Inconclusive`; N2 ("a named state always has an
  action that works") argues for all four, and the header still carries the verdict, so a real
  failure still reads as a failure.

### Display caps (new, and deliberately in the shrinking direction)

`MAX_CAPTURE_BYTES` (8 KiB/stream) and `MAX_OBSERVATION_CHARS` (240/line) are unchanged, as the
ticket requires. A new `MAX_OBSERVED_LINES = 10` per stream bounds *what is shown*, with a
`    … (N more lines)` tail when a stream is longer. Without it an 8 KiB capture is a two-hundred
line wall in the operator's terminal — the opposite of legible. This only ever shows less than
what was captured, so it does not touch the capture contract.

### What this costs

A decline stops being a single line. `parked_ux.rs:387` asserts `stderr.lines().count() == 1`;
that assertion is a test *of the old rendering*, not of a property the ticket preserves — the
ticket requires four facts plus captured output, which cannot honestly be one line. It is
rewritten to assert the new structure.

---

## Decision 2 — "Could not look" is exit 2, 126, 127, or death by signal

### Options for the line

**(2a) Any non-zero except a distinguished set is `Failed`; the distinguished set is
inconclusive.** The ticket names this option and gives the field evidence: `check-touch.mjs`
exits 2 to mean "I could not look."

**(2b) Ask the disposition to declare it** — a `check_inconclusive_exit` field. Rejected here:
that is a schema change to the parking/remedy contract, which the story puts out of scope
("any redesign of the parking/remedy schema beyond the fields these tickets need"), and
T-056-01-02/03 are the tickets that own the `check` field's contract. This ticket must not
pre-empt them.

**(2c) Heuristics on the captured text** ("if it says 'not found', it's inconclusive").
Rejected outright: guessing meaning from someone else's prose is the exact defect being fixed.

**Chosen: (2a)**, with this set:

| `status.code()` | Classification | Why |
| --- | --- | --- |
| `Some(0)` | `Passed` | unchanged |
| `Some(2)` | `Inconclusive` | the field convention, and the widely used "trouble, not a verdict" code (grep, diff) |
| `Some(126)` | `Inconclusive` | POSIX: found but not executable — the check never ran |
| `Some(127)` | `Inconclusive` | POSIX: command not found — the check never ran |
| `Some(_)` | `Failed` | a check that ran and reported no |
| `None` | `Inconclusive` | killed by a signal; nothing was concluded |

126/127 are not decoration: they are the two codes `/bin/sh -c` itself produces when the recorded
check could not be started at all. Classifying those as a verdict on the operator's work would be
the same lie in a different costume.

The pinned invariant either way: an inconclusive result never asserts the remedy was not done, and
it always names the way through.

`run_world_rechecks` gains `Inconclusive` in its existing do-nothing arm. Automation still never
acts on a non-pass; making its silence observable is T-056-01-03's ticket, and is not touched.

### Captures are now read on every path

Today `TimedOut` (`:232`) and `ChangedFiles` (`:236`) return before the captures are read, so
those two declines could not show output even if they wanted to. The rewrite reads both captures
once, after the wait, before classifying — so all four non-passing classifications carry evidence.
The process group is already SIGKILLed before the read on the timeout path, so nothing is still
writing.

---

## Decision 3 — `--override-check`, and a provenance receipt

### The flag

Name candidates weighed against the house style (`--dry-run`, `--with-history`, `--check-tools`,
`--bypass-sandbox`) and the brand voice (name the action, not the subsystem):

| Candidate | Verdict |
| --- | --- |
| `--force` | Rejected: says nothing about *what* is forced, and reads as "ignore all declines" when the flag deliberately only covers the check gate. |
| `--skip-check` | Rejected: describes the wrong mechanism. The check still runs and is still reported; what changes is that its decline no longer holds. |
| `--i-checked-it` | Rejected: reads as a sentence, not a flag; awkward to say aloud and to document. |
| `--override-check` | **Chosen.** Plain English, names exactly the gate it overrides, and scopes itself. |

Semantics:

- The check **still runs and is still reported in full**. The flag changes what happens after the
  report, not whether Lisa looks. This is what makes the receipt worth having: it records what
  the check actually said at the moment it was overridden.
- Applies **only** to the check gate. `I couldn't find T-X.`, `T-X isn't waiting.`,
  `I couldn't find what T-X is waiting for.`, and the `already-done` hand-off all still decline
  with the flag present — those are not gates the operator can satisfy by having done the ask.
- A no-op when nothing declined: a passing check, or no check at all, reopens exactly as today
  and writes **no** receipt. The rule is crisp — *the receipt is written exactly when the override
  turned a decline into a reopen.*
- Success message differs so the screen also tells the truth:
  `T-010-03 can run again — you overrode its check.`

### The durable record

**Option (3a): reuse `OperatorOverrideRecord`.** Rejected. That row is a *completion* receipt: it
carries a `DispositionNote` and a reason from `operator_override.rs`'s catalog, every entry of
which is about accepting finished work ("The work already covers this…"). A forced unblock accepts
nothing and completes nothing — the ticket goes back to running. Worse, `OperatorOverride::recover`
(`operator_override.rs:175-184`) reconstructs a signed completion from any row whose summary
matches a catalog entry, so an unblock receipt would replay as a completion someone signed. That
is a fabricated receipt, which is precisely what that module's doc comment forbids.

**Option (3b): a work-dir sidecar file.** Rejected: work-dir artifacts are agent-owned and are
published/overwritten by the next attempt on the ticket, so the record would not be durable in the
sense criterion 4 needs ("distinguishable after the fact").

**Option (3c): a new provenance row.** **Chosen.** `.lisa/provenance.jsonl` is the project's
append-only, committable evidence ledger, it already carries an operator-actor row written from
the CLI (`ProposalActionRecord`, `proposal.rs:163`), and every consumer reads it
non-exhaustively so a new arm breaks nothing.

```rust
pub enum CheckOverrideType { CheckOverride }          // "check-override"
pub enum CheckOverrideOutcome { Failed, Inconclusive, TimedOut, ChangedFiles }

pub struct CheckOverrideRecord {
    schema_version: u32,
    #[serde(default)] seal: CompletionSeal,
    record_type: CheckOverrideType,
    ticket_id: String,
    actor: String,                 // "operator"
    check: String,                 // as recorded
    directory: String,             // the cwd the check actually ran in
    result: CheckOverrideOutcome,  // what was overridden
    #[serde(skip_serializing_if = "Option::is_none")] exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")] observed: Vec<String>,
    occurred_at: u64,
}
```

Shape reasoning, following `OperatorOverrideRecord`'s own doc comment: no `AttemptLease`. The
attempt that parked this ticket is long gone, and synthesizing a lease to reuse an execution shape
would file a fabricated run. The row carries only observed facts — who overrode, what the check
was, where it ran, what it reported.

`observed` holds the same sanitized lines the operator saw, so the ledger answers "what was
overridden" without a second lookup. `SCHEMA_VERSION` bumps 9 → 10 (an additive row shape, the
same treatment version 9 got for `usage-correction`).

Untagged-enum safety: `CheckOverrideType` deserializes only `"check-override"`, so no existing row
can land in the new arm and the new arm cannot absorb one. The precedent test
(`operator_override_row_does_not_absorb_or_get_absorbed`, `provenance.rs:1256`) is extended to
cover both directions for this row.

### Ordering, and what happens if the ledger cannot be written

The receipt is appended **before** the status flip, and a failed append declines the unblock with
a plain, actionable error naming the ledger.

Reasoning: an override that leaves no trace is exactly the state criterion 4 forbids —
indistinguishable from a check that passed. Writing first means Lisa can never reach a reopened
ticket with no receipt; the opposite failure (receipt written, flip fails) leaves the operator
with a visible, retryable state and an error that says so. This is not a gate with no way through:
its cause is an unwritable `.lisa/`, which blocks every other Lisa command equally and has a
concrete local remedy that the message names.

### Seal stamping

`completion_seal::resolve_for_inspection(root, resolved.completion_mode)` — the same call
`proposal.rs:188` makes. `run_unblock` already has `resolved` in hand.

---

## Documentation surfaces the flag must reach

Adding a long flag is gated by CI in three places, all identified in Research §8:

1. `crates/lisa-cli/tests/help_surface.rs` — the byte-exact `lisa unblock --help` snapshot, plus
   the banned-jargon gate over the flag's doc comment.
2. `docs/knowledge/flag-audit.md` — a row is mandatory. Clap `required=false` ⇒ bar
   `working default`, category `—`, and a named pinning fixture.
3. `README.md:440-447` — the `lisa unblock` section, which is where "documented override flag"
   lands for a reader.

`docs/knowledge/rdspi-workflow.md` is deliberately **not** touched: the execution contract a
reviewer writes a check against is T-056-01-03's criterion, and editing it here would collide.

---

## What is explicitly not in this ticket

- Where the check runs, and whether it can see the project — T-056-01-02. This ticket reports
  whatever `run_check` used, so it stays correct across that change with no edit.
- The 5-second timeout, the write ban, record-time validation, and world-recheck silence —
  T-056-01-03. `CHECK_TIMEOUT` is read for the timeout header rather than hardcoded into the
  string, so that ticket's "name the budget you waited for" criterion is one substitution away,
  but the string itself stays byte-identical here.
- Any change to `MAX_CAPTURE_BYTES`, `MAX_OBSERVATION_CHARS`, or `sanitize_observation`'s rules.
