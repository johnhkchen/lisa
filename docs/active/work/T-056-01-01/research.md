# T-056-01-01 — Research: say-what-was-run-and-where

What exists today, where it lives, and which boundaries this ticket touches. Descriptive only.

## 1. The surface the operator sees

`lisa unblock <TICKET_ID>` is declared in `crates/lisa-cli/src/main.rs:121-133`:

```rust
Unblock {
    ticket_id: String,                  // positional, "Ticket to let run again"
    #[arg(long, default_value = ".")]
    path: PathBuf,
}
```

Two arguments. No override, no verbosity, no dry run.

Dispatch is `main.rs:613-626`:

- `UnblockOutcome::Reopened(message)` → `println!`, exit 0.
- `UnblockOutcome::Declined(message)` → `eprintln!`, `exit(1)`.
- `Err(error)` → `eprintln!("Error: {error}")`, `exit(1)`.

So a decline is one line on stderr and exit 1. The `Error:` prefix is reserved for Lisa's own
failures — `parked_ux.rs:161` asserts a decline never carries it. That distinction (Lisa broke vs
Lisa has a finding) already exists; what does not exist is any distinction between *the check's*
finding and *Lisa's* finding.

## 2. `run_unblock` and the check gate

`crates/lisa-cli/src/unblock.rs:42-86`. Ordered gates:

1. `config::load_config` + `config::resolve_config` → `ticket_dir`, `work_dir`. `resolved` also
   carries `completion_mode`, which is what other CLI ledger writers use to stamp a seal.
2. `ticket::scan_tickets`; unknown id → `"I couldn't find {id}."`
3. `ticket.status != Blocked` → `"{id} isn't waiting."`
4. `recording_failure_decline` (`:146-159`) → the S-055-01 `already-done` hand-off.
5. `collect_parked_remedies` (`lisa_core::parking`) → no remedy → `"I couldn't find what {id} is
   waiting for."`
6. **If `remedy.check` is `Some`:** `run_check(root, &check, CHECK_TIMEOUT)`; anything other than
   `CheckResult::Passed` becomes `Declined(decline_message(result))`.
7. Otherwise `ticket::update_ticket_status(.., Open)` and `"{id} can run again."`

Note gate 6's shape: the *only* thing that survives `run_check` into the message is a
`CheckResult`. The check string, the directory, and the exit status are all local to `run_check`
and are dropped at the return. That is the structural reason the message cannot attribute
anything — not a wording choice.

## 3. `CheckResult` and `decline_message`

```rust
enum CheckResult { Passed, Failed(String), TimedOut, ChangedFiles }   // :33-39
```

`decline_message` (`:161-174`) renders three strings, all with the same verdict lead:

| Variant | String |
| --- | --- |
| `Failed(observation)` | `That didn't work yet — {observation}` |
| `TimedOut` | `That didn't work yet — it took longer than 5 seconds.` |
| `ChangedFiles` | `That didn't work yet — it tried to change project files.` |

`Failed`'s payload comes from `observed_line(&stderr, &stdout)` (`:275-286`): the first non-empty
sanitized line, stderr preferred, `"it still isn't ready."` when both streams are empty. The
field failure is exactly this path — `check-touch.mjs` printed `No build at dist/. Run: npm run
build` to stderr and exited 2, and Lisa printed the script's sentence behind Lisa's lead.

`run_check`'s classification (`:245-251`):

```rust
if status.success() { Passed } else { Failed(observed_line(...)) }
```

Every non-zero exit collapses to `Failed`. There is no representation for *the check could not
look*. The field script exits 2 to mean precisely that.

## 4. `run_check` internals — what is available to report

`:176-252`. In order:

- `ReadOnlySnapshot::new(root)` (`:312-333`): `tempfile::tempdir()`, `snapshot_project`,
  `set_tree_read_only`. **`snapshot.path()` is the directory the check actually runs in** —
  `.current_dir(snapshot.path())` at `:192`. This is the cwd the ticket says to report "as
  `run_check` actually used"; T-056-01-02 may change what that path is, but not that it is the
  value reported.
- `fingerprint_tree` before, and again after (`:236-239`) → `ChangedFiles` on any drift.
- `Command::new("/bin/sh").arg("-c").arg(check)` — the recorded check string, verbatim, is
  already in hand at the reporting site.
- `TMPDIR`/`TMP`/`TEMP` redirected to a scratch dir; stdin null; stdout/stderr to `tempfile`s.
- `process_group(0)` on unix so `terminate_check` (`:254-261`) can `kill(-pid)`.
- Poll loop with `POLL_INTERVAL` 10 ms against `CHECK_TIMEOUT` 5 s → `TimedOut`.
- `status: ExitStatus` — `status.code()` is available and currently unused beyond `.success()`.
- `read_capture` (`:268-273`) reads at most `MAX_CAPTURE_BYTES` (8 KiB) per stream. Both streams
  are already read into `Vec<u8>` before the success branch, so full captured output is in scope
  at the classification point.

So every fact criterion 1 asks for — check string, cwd, exit code, both captured streams — is
present inside `run_check` at the moment it decides. None of it crosses the return boundary.

## 5. Sanitizing

`sanitize_observation` (`:288-310`) strips CSI escape sequences (`ESC [` … final byte in
`@..~`), turns tabs into spaces, drops other control characters, trims, and truncates to
`MAX_OBSERVATION_CHARS` (240). It is applied *per line* by `observed_line`, not to a whole
capture. Criterion 5 requires it to keep applying to everything shown — which means any
multi-line rendering must route each line through it rather than printing raw captures.

The 240-char cap is per line and the 8 KiB cap is per stream; the ticket pins both as unchanged.

## 6. The second caller: `run_world_rechecks`

`:95-134`. Walks every parked remedy, keeps `remedy_owner == World` with a `check`, and reopens
only on `Passed`. Its match arm is explicit:

```rust
CheckResult::Failed(_) | CheckResult::TimedOut | CheckResult::ChangedFiles => {}
```

Any new `CheckResult` variant must be handled here. The story's out-of-scope note is clear:
"world-owned remedies still never auto-act on a non-pass; only their silence changes" — and the
silence is T-056-01-03's business, not this ticket's.

## 7. Durable records: what the ledger already carries

`.lisa/provenance.jsonl`, schema `crates/lisa-core/src/provenance.rs`, `SCHEMA_VERSION = 9`.
`.lisa/` gitignores only `signals/`, so the ledger is committable, queryable history.

Existing row shapes, all appended through `append_serialized` (`:618-632`, creates parents, true
append, one JSON line):

| Row | Discriminator | Notes |
| --- | --- | --- |
| `ProvenanceRecord` | none (legacy shape) | terminal execution; needs a live attempt lease |
| `AssignmentTransitionRecord` | `assignment-transition` | pre-ownership failures |
| `ParkingTransitionRecord` | `retry` \| `park` \| `unpark` | written by the plugin |
| `TriageTransitionRecord` | `triage-transition` | first-responder pass |
| `ProposalActionRecord` | `proposal-action` | **written by the CLI** (`proposal.rs:163`) |
| `OperatorOverrideRecord` | `operator-override` | **a person signing over a block** |
| `UsageCorrectionRecord` | `usage-correction` | late token join |
| `NoteAcknowledgmentRecord` | — | notes |

`ProvenanceLedgerRecord` (`:385-396`) is a `#[serde(untagged)]` enum read by every consumer.
Two properties matter for adding a row:

- Every consumer outside `provenance.rs` matches non-exhaustively (`if let` / `matches!` /
  `filter_map`) — `parking.rs:104`, `notes.rs:319`, `lisa-plugin/src/lib.rs` in ~20 places. No
  consumer breaks on an unknown variant.
- Untagged resolution is by shape and order, so a new arm can absorb or be absorbed. The existing
  precedent is `operator_override_row_does_not_absorb_or_get_absorbed` (`:1256-1290`), which
  asserts both directions. A `record_type` enum with a single variant is what keeps rows disjoint.

`OperatorOverrideRecord`'s doc comment (`:302-310`) is the closest precedent for this ticket's
criterion 4, and states the rule directly: the tickets an override serves are the ones whose
agent is already gone, so synthesizing an `AttemptLease` to reuse the execution shape "would file
a fabricated run" — the receipt gets its own shape and carries only facts: who signed, which
reason, what it overrode. It carries `actor: String` (`"operator"`), a frozen `reason` copy, a
stable `reason_id`, and a `DispositionNote`. Its catalog lives in
`crates/lisa-core/src/operator_override.rs`, and every catalog entry is about *accepting completed
work* — none of them is about a check gate.

Seal stamping from a CLI command: `completion_seal::resolve_for_inspection(root, completion_mode)`
(`crates/lisa-cli/src/completion_seal.rs:54-64`, `pub(crate)`), fed from
`resolve_config(..).completion_mode`. `proposal.rs:188` is the working example.

## 8. What pins the current strings and the current flag set

Changing the decline rendering or adding a flag trips these:

- `crates/lisa-cli/tests/parked_ux.rs:144-164` — `failing_check_declines_plainly_and_leaves_the
  _ticket_waiting` asserts stderr **equals** `"That didn't work yet — the key link still returns
  404\n"`.
- `parked_ux.rs:373-392` — the write-probe decline asserts `stderr.lines().count() == 1` and a
  `"That didn't work yet — "` prefix.
- `unblock.rs:569-656` — four unit tests over `run_check` / `decline_message` /
  `observed_line`, including the exact `TimedOut` and `ChangedFiles` strings.
- `crates/lisa-cli/tests/help_surface.rs:146-161` — a **byte-exact snapshot of `lisa unblock
  --help`**. Adding a flag requires editing this snapshot.
- `help_surface.rs:428-455` — operator help must contain no banned jargon (`dag`, `orchestrat`,
  `scheduling`, `leverage`, `solutions`, `deployment`, `case study`, `build log`,
  `research release`). The new flag's `///` doc comment is operator-facing copy under this gate.
- `crates/lisa-cli/src/main.rs:805-1085` — `flag_audit_covers_live_cli_config_and_prompts` walks
  the live Clap tree and requires a row in `docs/knowledge/flag-audit.md` for **every** long flag.
  A non-required flag must be barred `working default`, must name a pinning fixture (not `—`),
  must have category `—`, must end its rule cell with a sentence terminator, and its surface+rule
  copy is under the same banned-jargon gate.
- `README.md:440-447` — the `lisa unblock` section; the ticket's "documented override flag" lands
  here.

## 9. Copy conventions in force

- Declines are lowercase-plain, one sentence, no stack traces, no `Error:` prefix
  (T-048-02-01's rule).
- `status.rs` renders the parked ask verbatim and never leaks `remedy_owner` or field names
  (`parked_ux.rs:87-97`).
- Brand voice: kitchen-table English, name the action not the subsystem
  (`operator_override.rs:260-275` even tests this for the override catalog).
- The RDSPI workflow's `ask` guidance (`docs/knowledge/rdspi-workflow.md:61`) is the model for
  "always names the way through".

## 10. Constraints and assumptions carried into Design

1. **Report what `run_check` used, not what it should have used.** The ticket is explicit: if this
   and T-056-01-02 disagree about cwd, this one reports whatever `run_check` actually used. So the
   reported directory must be *derived from the same value passed to `.current_dir`*, not
   recomputed.
2. **Caps stay.** `MAX_CAPTURE_BYTES`, `MAX_OBSERVATION_CHARS`, and `sanitize_observation` are
   unchanged in behaviour; only what is *shown* changes.
3. **Exit-0 override.** Criterion 3 pins `UnblockOutcome::Reopened` and exit 0 for a forced
   unblock, and byte-identical non-forced behaviour apart from the new reporting.
4. **A forced unblock must be distinguishable after the fact**, and an ordinary unblock must write
   no such record — so whatever is written must be conditional on the flag, and only on the path
   where a check actually declined.
5. **Both `run_check` callers must compile and keep their semantics.** `run_world_rechecks` is
   automation; it must not gain override powers.
6. **`Failed` vs inconclusive needs a line.** Where it is drawn is Design's call. Facts available:
   `status.code()` is `Option<i32>`; the field case exits 2; POSIX shells use 126 for
   "found but not executable" and 127 for "not found"; the timeout path already handles signals
   separately.
7. **Multi-line stderr is new.** Today a decline is exactly one line, and one test asserts that.
   Criterion 1 requires four facts plus captured output — which cannot be one line honestly.
   The single-line assertion in `parked_ux.rs:387` is a test of the old rendering, not of a
   property the ticket preserves.
