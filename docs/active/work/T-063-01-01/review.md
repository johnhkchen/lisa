# T-063-01-01 — a board knows how many schedulers are on it

## What changed

A board could not count the schedulers on it because the only thing a scheduler
wrote about itself was one shared file that every scheduler kept fresh. This
replaces that with a registry of one file per scheduler, written by the
scheduler, and then uses it everywhere the old file was the whole story.

### New: `.lisa/schedulers/` (`crates/lisa-core/src/schedulers.rs`)

- `<scheduler-id>.alive` — one record per scheduler: its id, the Zellij session
  it runs in, the Zellij **server** pid, when it started, when it last stamped,
  and its poll cadence. Session name and server pid are the two facts nobody
  can recover from outside the process, and `zellij kill-session <name>` is the
  only thing measured to stop a detached scheduler.
- `<scheduler-id>.consumed.jsonl` — a receipt per *decisive* signal the
  scheduler took out of `.lisa/signals/`. Heartbeats and `.alive` pings are
  excluded on purpose (they arrive every few seconds and nothing turns on which
  reader got one).
- Readers: `read_roster`, `read_receipts`, `taken_by_another`, plus
  `forget`/`forget_long_dead` for records a stopped scheduler cannot withdraw.
- The single-consumer contract for `.lisa/signals/` is unchanged: a signal is
  still read once and removed. The receipt is written after the removal
  succeeds, so a scheduler never claims a file still sitting in the directory.

### Plugin (`crates/lisa-plugin/`)

- Takes an identity at `load()` (session name + start nanos + plugin id),
  sweeps records older than 7 days, writes its own record on every tick beside
  the existing `.lisa/scheduler.alive` (kept, so older readers still work).
- Reads the roster every poll and logs one warning per newly seen scheduler,
  naming it and its `zellij kill-session` command.
- `signal::ingest` takes an optional `Consumer` and leaves receipts.
- **`begin_startup_recovery` no longer probes a contested pane.** If another
  scheduler's receipt shows it took `pane-N.started` in the last 10 minutes,
  the seat is fenced with that scheduler named, instead of interrupting the
  pane and typing the shell-readiness probe — the probe a live
  `--dangerously-skip-permissions` session runs on the scheduler's behalf, and
  the first and only prompt ten of twelve sessions got on the day.

### CLI (`crates/lisa-cli/`)

- **`lisa schedulers`** (new): lists every scheduler on this board — started,
  last seen, server pid, what it took from `.lisa/signals/` in the last hour —
  with each one's stop command, and a warning when more than one is live.
  `lisa schedulers --stop <id|session>` runs `zellij kill-session` and forgets
  the record. It refuses the session the caller is sitting in, and refuses an
  ambiguous or unknown name.
- **`lisa loop` refuses to start a second scheduler** on a board that already
  has a live one, naming it and how to stop it. The liveness window here is the
  scheduler's own cadence (≈60s), not the project's wind-down, so a restart
  after a crash is never blocked for minutes.
- **`lisa loop` passes `session_name` into the plugin's layout** — the only way
  the plugin can learn which session it lives in.
- **`release-seats` and `reset-ticket` distinguish working from stamping.**
  `RunLiveness::Running` split into `Working` (something moved in
  `.lisa/signals/`) and `Stamping` (a scheduler exists, nothing has moved).
  Both still keep seats, but the stamping refusal now names the schedulers and
  their stop commands instead of quoting an anonymous heartbeat.
- **`lisa status`** prints a section naming every scheduler when more than one
  is running, and `--json` gains a `schedulers[]` array.
- `.lisa/schedulers/` added to the `.lisa/.gitignore` template.

## Acceptance criteria

| Criterion | Where |
| --- | --- |
| A second scheduler is detected and says so | `lisa loop` refuses and names the first (`second_scheduler_refusal`); the plugin warns in the dashboard feed (`note_other_schedulers`) |
| `lisa status` can name the schedulers | `contested_board_lines` + `schedulers[]` in `--json`; `lisa schedulers` is the full listing |
| A supported way to stop a detached scheduler | `lisa schedulers --stop <id>` runs the `zellij kill-session` that was measured to work, with the name already found |
| `release-seats`/`reset-ticket` distinguish working from stamping | `RunLiveness::Working` vs `Stamping`; both refusals name the run and its stop command |
| Signal consumption is attributable | per-scheduler receipt ledger; `taken_by_another` is read by the startup-recovery guard and printed by `lisa schedulers` |
| Reproduce it before fixing it | `tests/two_schedulers_one_board.rs` — two `State`s, one board, one `pane-1.started` the live one never sees |

### What evidence should gate the two recoveries (asked for explicitly)

Documented in `crates/lisa-cli/src/seats.rs`'s module header. In short: not a
shared heartbeat, which cannot tell one writer from three. Gate on (a) movement
in `.lisa/signals/`, which only a live pane or plugin produces and which no
scheduler can fake on another's behalf — that is *work*; and (b) the
per-scheduler registry, which says *who* is here, so every refusal can name the
run it is protecting and the command that ends it. The shared stamp survives
only as the fallback for a board whose scheduler predates the registry.

Both states still refuse. A stamping scheduler can dispatch at any moment, so
letting the reset through would race it; the fix for the lockout is that the
refusal now ends at a command instead of a wall.

## Tests

`cargo test --workspace` and `cargo clippy --workspace --all-targets` are clean
(31 test binaries, all `ok`). New tests:

- `lisa-core/src/schedulers.rs` — 11 unit tests: three schedulers read as
  three, stale vs live, future stamps, id uniqueness, the 7-day sweep, and
  "a signal that vanished into another scheduler is named".
- `lisa-plugin/src/signal.rs` — two consumers split one `.started`; heartbeats
  leave no receipts; an unnamed consumer behaves exactly as before.
- `lisa-plugin/src/lib.rs` — per-scheduler records, the once-per-scheduler
  warning, a stale record not announced as a second scheduler, the probe guard,
  and the guard *not* firing for a single scheduler's own receipt.
- `lisa-plugin/src/tests/two_schedulers_one_board.rs` — the reproduction.
- `lisa-cli/src/schedulers.rs` — 11 tests over the listing and `--stop`,
  including the refusal to stop the caller's own session and the failed-kill
  path that changes nothing.
- `lisa-cli/src/seats.rs`, `status.rs`, `loop_cmd.rs` — the working/stamping
  split, the contested-board section, the layout's `session_name`, and the
  second-loop refusal.

Also exercised by hand against a fabricated registry in a scratch project:
`lisa schedulers`, `lisa loop` (refusal), `lisa status`, `lisa release-seats`,
`lisa reset-ticket --apply`. Output is in the shape shown above.

## Concerns and limitations

1. **This does not rescue today's zombies.** A scheduler already running an
   older plugin never writes a record, so the registry is empty for it and
   `lisa schedulers` will say so. The old recovery (`zellij list-sessions`,
   `lsof`) is still what clears a pre-upgrade zombie. Everything here starts
   working from the first loop started on the new build.
2. **No live two-server test.** Reproducing this against real Zellij means
   starting a second scheduler on a live board — on this repository, that is
   the failure itself, and this attempt is running under one of those
   schedulers. The reproduction is two whole `State`s over one `.lisa/`
   directory, which is the mechanism; the Zellij half (client dies, server
   lives) is unchanged and already field-measured in S-063-01.
3. **Receipt ledgers are not rotated.** A long-lived scheduler appends a line
   per decisive signal for as long as it runs. `forget` removes a stopped
   scheduler's ledger and `forget_long_dead` sweeps after 7 days, but a
   week-long run's ledger just grows. Bounded in practice (decisive families
   only, `.lisa/` is ignored), worth a cap if it ever gets read at scale.
4. **`lisa loop` has no override for the refusal.** Deliberate — the refusal is
   the acceptance criterion — but it means a stamp from a scheduler that died
   in the last ~60 seconds delays a restart by up to a minute. The message says
   so.
5. **The probe guard fences the seat** (`StartupFailed`) rather than retrying.
   It fires only when another scheduler's receipt exists, so a single-scheduler
   board keeps every path it had; under contention, the ticket needs a reset
   after the extra scheduler is stopped.
6. **`.lisa/.gitignore` in this repository was not updated.** It already had an
   unrelated uncommitted change when this attempt started, so committing it
   would have swept work that is not this ticket's. The template is updated;
   the next `lisa init` here appends `schedulers/`.
7. **Cross-ticket sweep, for the record.** My five `.gitignore` expectation
   edits in `crates/lisa-cli/src/init.rs` landed in T-064-01-01's commit
   `e64d918` — that ticket ran concurrently in the same working tree and
   included the file. The content is correct and committed; only the
   attribution is wrong. Nothing needs undoing, and it is one more instance of
   the shared-tree hazard the workflow document names.
