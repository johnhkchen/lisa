# T-070-01-05 — a session name says when it started

## What the name is now

`<project>-<MMDD>` for the first run of a project on a day, and
`<project>-<MMDD>-<N>` for the second and after: `steer-0815`, `steer-0815-2`,
`steer-0815-3`.

**Is the day enough? Yes.** The only question a name has to answer is *is this
session from a previous day*. How old a live one is, is already a column in
`zellij list-sessions` (`Created 10h 32m ago`) and a line in `lisa schedulers`
(`started 19h ago`). Four more digits of clock buy nothing in the one place —
a truncating tab — that cannot afford them.

**Two runs of one project on one day** are told apart by a number *behind* the
date. This was the real design question; `renderer` had six sessions on
2026-08-14. Three options were on the table:

| | shape | width for `steer` | what it costs |
|---|---|---|---|
| chosen | `steer-0815-2` | 12 | 2 chars; the number now counts something bounded |
| time in the name | `steer-0815-1432` | 15 | 5 chars, still collides inside a minute, unreadable |
| letter suffix | `steer-0815b` | 11 | 1 char, but `b` reads as part of the project name |

The number also fixes the trap the ticket names at the end: it used to count the
dead of all time, so the same run was `steer-3` on a desk nobody had cleaned and
`steer` on one just swept. Scoped to a day it resets by itself at midnight and no
longer depends on when anybody last ran `zellij delete-all-sessions`.

**What the width costs, and where.** `steer` → `steer-0815` → `steer-0815-2` is
5 → 10 → 12 characters. The widest name Lisa can now produce is
`MAX_BASE_CHARS` (32) + 5 + 3 = 40. `MAX_BASE_CHARS` was deliberately **not**
reduced to make room: the tab that truncated `auspicious-panda` to
`auspicious-p` cuts from the right, so with the project first the date is what a
narrow tab eats — `steer-0815` reads as `steer-08…`, still the project. Losing
the date costs a reader a column `zellij list-sessions` still has; losing the
project name costs the reason the name exists at all.

## The staleness check

Implemented in two places, both reading the **name** rather than a record,
because the name is what a reader has when they have nothing else:

- **`lisa loop`'s second-scheduler refusal.** That refusal holds a list of
  session names and nothing else about them, and telling a run somebody is
  watching from one that died on Thursday is exactly what took twenty minutes on
  2026-08-14 (S-070-01). Each name saying a previous day now says so on its own
  line.
- **`lisa schedulers`.** The same sentence beside the run it belongs to.

Live output, against a seeded registry:

```
  renderer-0815 (renderer-c1133916)
    started 10m ago, stopped stamping 2s ago, zellij server pid 99999
    the Zellij server it recorded, pid 99999, is not a process on this machine any more
    nothing to stop; clear its record with: lisa schedulers --stop renderer-c1133916

  renderer-0814-3 (renderer-49ded6ab)
    started 20h ago, stopped stamping 17h ago, zellij server pid 15340
    its name says it started on 08-14, and today is 08-15 — this session is from a previous day
    the Zellij server it recorded, pid 15340, is not a process on this machine any more
    nothing to stop; clear its record with: lisa schedulers --stop renderer-49ded6ab
```

**Not a sweep, and not in `clean`.** The ticket offered `clean` as the model. Its
stated rule is that a candidate is a *file Lisa wrote for a finished ticket,
inside a directory Lisa created for that ticket* — a Zellij session is neither a
file nor Lisa's, and bending that rule to fit is the wrong trade for the one
module in the CLI whose whole design is a single mechanical rule about what may
ever be destroyed. A sweep of stale sessions would need its own consent shape and
its own verb; it is not in this ticket.

## Who reads a session name for meaning, and who treats it as opaque

**Parses it:**

- `session_name::names_this_board` — decides whether a running session belongs to
  this board, which is what refuses a second scheduler. Rewritten to accept every
  shape Lisa has ever produced: `steer`, `steer-3`, `steer-0815`, `steer-0815-2`.
  The legacy forms had to stay, or an upgrade would stop recognising a running
  `renderer-3` and let a second scheduler onto a live board — the 2026-08-12
  incident, reintroduced. The four-digit day and the two-digit run number can
  never be confused, because the smallest MMDD is `0101` = 101 and the run
  numbers stop at 99.
- `session_name::started_on` / `previous_day_note` — new. Reads the day off the
  tail of any name without needing the project's base, so a caller holding only
  a listing line can use it. Rejects four digits that are not a calendar day, so
  `lisa-field-current-8443` stays what it is: somebody's hand-named session.

**Treats it as opaque (all verified unchanged):**

- `lisa schedulers --stop <target>` — exact equality against `scheduler_id` or
  `session_name`.
- `zellij kill-session` / `zellij attach` — strings passed straight through.
- `presence::Machine::session_running` and `busy::running_session_names` — exact
  equality against the listing.
- `loop_cmd::last_stamp_age` and the layout's `session_name "…"` — equality and
  passthrough.
- `seats::location` → `lisa status --json`'s `run.session` / `run.sessions` —
  sorted, deduped, passed through. **This is what `rail`'s station beacons
  read.** `rail` lives outside this repository, so what Lisa owes it is that the
  field keeps its shape and its meaning, which it does. A beacon holding
  yesterday's cached name simply stops matching when a new day's run starts —
  the same thing that already happened whenever the number changed.
- `lisa_core::schedulers::scheduler_id` — takes the session name as opaque text
  and sanitizes it into a filename base. The id inherits the date for free:
  `renderer-0815-2-c1133916`. Five to eight characters wider, machine-facing,
  and `--stop` still takes it verbatim.

**On the ticket's open question — name or scheduler id?** Both, and they did not
want different answers in the end. The scheduler id already separated two runs of
one session name with its hash; it now carries the date too, at no design cost,
because it is filename-safe and nobody reads it off a tab. But putting the date
*only* there would have left the tab, `zellij list-sessions`, and
`zellij kill-session` exactly as uninformative as before, which is the whole
complaint. The name needed it directly.

## Files changed

- `crates/lisa-cli/src/session_name.rs` — the date in the name, `StartDay`, the
  local-day reading, `started_on`, `previous_day_note`, `names_this_board`
  widened to every legacy shape. Module header rewritten to say why.
- `crates/lisa-cli/src/loop_cmd.rs` — the refusal names which holders are from a
  previous day.
- `crates/lisa-cli/src/schedulers.rs` — the listing says the same beside each run.
- `crates/lisa-cli/src/channel.rs` — `civil_from_days` made `pub(crate)` for the
  non-Unix day reading. No behaviour change.
- `crates/lisa-cli/tests/zellij_version_preflight.rs` — the reproduction, plus
  two existing fixtures updated to the dated name.
- `README.md` — what the name carries, where the width goes, and that older
  Lisa's names still name their board.

## Tests

- **`crates/lisa-cli/src/session_name.rs`** — 25 unit tests. First run of a day;
  a second and third; yesterday's sessions not numbering today's; a gap reused;
  every number taken; a clock that cannot be read still starting; the day read
  back off a name and only a real one; the previous-day sentence firing on
  yesterday and staying quiet on today; every legacy and current shape of a
  board's own name against four hand-named lookalikes; a dated name still legal
  (ASCII, no slash, no leading/trailing dash) at its widest for three hostile
  project directories; and this machine's own clock reading as a calendar day.
- **`schedulers.rs`** — `a_session_named_for_a_previous_day_is_called_one`: two
  records, one named yesterday and one named today, exactly one sentence printed.
  Both dates are derived from the test's own `now`, so it is deterministic in any
  timezone and on any day.
- **`loop_cmd.rs`** —
  `a_refusal_says_which_of_the_sessions_holding_the_board_are_from_a_previous_day`,
  same shape, against the refusal text.
- **The ticket's reproduction, end to end through the real binary** —
  `two_runs_of_one_project_on_one_day_both_say_the_day_and_can_be_told_apart`.
  A stub Zellij that lists back every session it has been asked to start, and two
  `lisa loop` runs against it. Asserts the startup report reads
  `Session: project-<today>` then `Session: project-<today>-2`, and that Zellij
  was actually asked for those two distinct dated names.
- `just check` (fmt + clippy + workspace tests) exits 0. 744 CLI unit tests, 12
  in that integration file, whole workspace green.
- Manually run against a seeded registry with the built binary; output pasted
  above.

## What still concerns me

- **A run that starts at 23:59 is called stale at 00:01,** while still live. Its
  name says the day it *started*, which is correct and is the point, but the
  note reads as a judgement. `lisa schedulers` prints `— running` on the same
  record, so the two lines together are honest; the refusal in `lisa loop` does
  not have that marker, so a nine-minute-old run crossing midnight would read as
  a previous day's. Cheap and real, and I left it: the alternative is a note that
  also consults the stamp, which is the thing the ticket asked the *name* to
  answer.
- **MMDD carries no year.** A session named `steer-0815` seen on 2026-08-15 a
  year later reads as today's. Nothing on this desk survives twelve months, and
  a stamp and a `Created 3 months ago` catch it long before the date wraps, but
  it is a real hole in the encoding.
- **Sessions named by older Lisas get no staleness note at all** — `lisa-8` and
  `steer-3` are on this desk right now — because their names genuinely carry no
  day. They are still recognised as their board's, so nothing breaks; they simply
  age out as runs restart.
- **The day is read with `localtime_r`.** A machine whose timezone changes
  between naming a session and reading it (travel, a DST boundary at the wrong
  hour) can disagree with itself by a day. Non-Unix falls back to UTC, which is
  a few hours out at the day's edges; Lisa's runs are Unix runs and that path
  exists so the crate compiles, not because anybody reads it.
- **No sweep of the 320 EXITED sessions.** The name now makes one possible and
  the check reports staleness, but nothing deletes anything. That is deliberate
  (see `clean` above) and is the obvious follow-up.
- **I did not start two real Zellij sessions on this desk.** The reproduction
  runs the real `lisa` binary twice against a stub Zellij that records and lists
  back what it was asked to start; a live `lisa loop` here would have started a
  second scheduler on the board this attempt is running on. The real
  `zellij list-sessions` on this machine was read (`lisa-8`, `steer-3`,
  `overseer`, `screen-design-9`) and every one of those legacy shapes is covered
  by test.
