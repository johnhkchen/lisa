# T-070-01-01 — a scheduler whose session is gone can be stopped

**A stamp says a process was alive when it wrote; a pid is a question you can ask
now.** Lisa now asks it. A run whose Zellij server this machine cannot find is
reported as not running however recently it stamped, `lisa schedulers --stop`
succeeds on it, and the messages point at commands that can work.

## What changed

**New: `crates/lisa-cli/src/presence.rs`** — the question and the three answers
it can have.

- `kill(pid, 0)` for whether the recorded pid is held at all.
- `ps -p <pid> -o args=,etime=` for whether what holds it is the same Zellij:
  the first word of the command line has to name a `zellij`, and it has to have
  been up since before the run that wrote the number down (a Zellij younger than
  its own scheduler is a reused pid). A pid held by another user is somebody
  else's process.
- `zellij list-sessions` — machine-wide, like `busy.rs`, asking the project's
  configured Zellij, the one on `PATH`, and the packaged one — when there is no
  pid to ask about, and as a **veto** in every direction: whatever the pid says,
  a session Zellij still lists means the seats stay.
- Three answers, never two. `Gone` frees seats so it is only ever said on
  positive evidence; `Unknown` keeps them, and covers a machine with no Zellij
  to ask, a pid nothing can identify, and two answers that disagree.

**`seats.rs`** — `roll_call` divides the roster into runs that are here and runs
that only look it. A fresh `.lisa/scheduler.alive` is no longer read as "a
scheduler is here" when every scheduler that could have written it is provably
gone; that was the half of the deadlock that survived clearing the registry by
hand. `how_to_stop()` names `lisa schedulers --stop <id>` beside `zellij
kill-session <name>`.

**`schedulers.rs`** — `is_live` consults the machine; the listing prints the
reason a run reads as ended and offers `--stop` rather than a `kill-session`
that cannot succeed; `--stop` treats an already-ended run as the ordinary case
and reports what it found and what it cleaned (the record, the session when one
was still there, and the shared stamp when the run it retired was the last that
could have written it). A kill that fails while this machine still says the run
is here is still a refusal, and now says what it is refusing on.

**`loop_cmd.rs`** — a record whose server is gone no longer refuses the next
loop. A session still listed under this board's name still does.

**`heal_panes.rs`** — the "nothing answered" sentence names a scheduler only
when one is running.

**`README.md`** — both command sections say how a run is judged and what
`--stop` does after a crash.

## How it is tested

`just check` passes: fmt, clippy (no warnings), `cargo check` for the wasm
plugin, and the whole suite — 39 test binaries, 0 failures.

- `presence.rs`: 13 tests over the incident record verbatim — pid gone, pid
  belonging to something else, a younger Zellij on the same number, no pid, no
  session, a machine that cannot be asked, and the disagreement veto in all
  three of its shapes. `ps` line parsing is a pure function with the real macOS
  and Linux shapes as cases.
- `seats.rs`: the same board with the server gone (seats free, evidence names
  the pid), with the server up (every seat held), and on a machine that cannot
  be asked (unchanged from before this ticket).
- `schedulers.rs`: `--stop` on a scheduler whose session is already gone
  succeeds, cleans the record and the shared stamp, and says so; the listing
  shows the reason and the remedy; a live run keeps `kill-session` and its
  refusal; a stamp another scheduler is still rewriting is not taken.
- `loop_cmd.rs`: a gone-server record does not refuse; a listed session still
  does; and a refusal leaves every file under `.lisa/` byte- and mtime-identical.

**Reproduced live on this machine**, in a scratch project, using only documented
commands:

```
$ lisa schedulers                     # a real loop, started headless
1 scheduler is running on this board.
  repro (repro-309f3182)  — running
    started 12s ago, last seen 2s ago, zellij server pid 68521
    stop it with: zellij kill-session repro

$ zellij kill-session repro
$ lisa schedulers
No scheduler is running on this board. The 1 record below is a run that ended.
  repro (repro-309f3182)
    started 28s ago, stopped stamping 3s ago, zellij server pid 68521
    the Zellij server it recorded, pid 68521, is not a process on this machine any more
    nothing to stop; clear its record with: lisa schedulers --stop repro-309f3182

$ lisa release-seats --release
Released 1 marker. The next run places its own seats; nothing else changed.

$ lisa schedulers --stop repro-309f3182
repro (repro-309f3182) was not running any more: the Zellij server it recorded,
pid 68521, is not a process on this machine any more.
Cleaned: its record in .lisa/schedulers/repro-309f3182.alive, and
.lisa/scheduler.alive, which only that run could have written.

$ lisa loop --headless                # the board takes a new run
$ lisa loop --headless                # and still refuses a second one
Error: There is a run already running on this board …
```

The stamp was **three seconds old** at the moment the board was recovered. On
the day, that was the exact state that could not be recovered at all.

**That reproduction caught a real defect in this fix.** The first live run read
its own scheduler as dead: macOS truncates `ps -o comm=` at sixteen characters,
so `/opt/homebrew/Cellar/zellij/0.44.3/bin/zellij` arrived as `/opt/homebrew/Ce`
and the pid looked reused. Fixed by reading `args`, and the session veto was
widened to cover every gone-verdict so a second guard would have held anyway.
Both are now cases in the test suite. Nothing but the reproduction would have
found it — every unit test stated its own `ps` output.

## Decisions the ticket asked for

**Should a record for a provably-gone scheduler be retired automatically?
No.** The registry is a ledger — four records from four runs is the fact that
made the incident diagnosable — and now that the listing says plainly which
records are history and why, keeping them costs nothing to read. Retirement
stays an explicit act: `lisa schedulers --stop <id>`, plus the existing
seven-day sweep at loop start.

**No new staleness window.** Every reader keeps the window it had — this adds a
question that has no window at all. The one new constant, `REUSE_SLACK_SECS`,
is not a staleness bound: it is how much later than its own scheduler a Zellij
may have started before Lisa calls the pid reused.

## What still concerns me

1. **The ticket says a refusing `lisa loop` renewed the stamp. I could not find
   a code path where it does.** Nothing in the CLI writes `.lisa/scheduler.alive`
   or a `.lisa/schedulers/*.alive` record — the only writer in the codebase is
   the plugin, at load and every poll. The renewal that fits the evidence is a
   `lisa loop` that got *past* the refusal, started a session whose plugin
   stamped at load, and then collapsed — which also explains the four records on
   that board. I have locked the property with a test (a refusal writes nothing,
   contents and mtimes) and, more to the point, made a renewed stamp harmless: a
   stamp from a process that is gone no longer holds anything. If the field
   later shows a renewal this does not cover, the writer will be in the plugin's
   load path, not in a refusal.

2. **The pid probe costs a `ps` per live-stamped record** on `lisa status`,
   `release-seats`, `schedulers`, `reset-ticket` and `heal-panes`. Boards have a
   handful of records, and `zellij list-sessions` is spawned lazily — only when a
   pid cannot be read or reads as gone — so an ordinary status pays one `kill(2)`
   and one `ps` per record. Worth watching if a board ever accumulates dozens.

3. **`presence` is Unix-only in substance.** On a non-Unix host every record
   reads `Unknown` and behaviour is exactly what it was before this ticket. That
   matches Lisa's Zellij dependency, but it is a silent degradation rather than a
   stated one.

4. **The reused-pid check needs `etime`.** A `ps` that answers with an
   unparsable elapsed time leaves a Zellij on a reused number reading as
   running — the safe direction (seats stay), but it is a hole in the
   reused-pid guard rather than a closed door.

5. **Test fixtures that invented a pid had to stop.** `json_output.rs`'s
   scheduler records now carry no `zellij_pid` and state which sessions their
   machine is holding, and `headless_loop.rs`'s stub Zellij answers
   `list-sessions`. That is more honest, but it does mean those fixtures now
   describe a machine as well as a board.

## Files

- added `crates/lisa-cli/src/presence.rs`
- modified `crates/lisa-cli/src/{main.rs,seats.rs,schedulers.rs,loop_cmd.rs,heal_panes.rs}`
- modified `crates/lisa-cli/tests/{json_output.rs,headless_loop.rs}`
- modified `README.md`

Committed in four units through `lisa commit-ticket`: 833ead8, 36304db,
e1ad7f4, 5c2bad1.
