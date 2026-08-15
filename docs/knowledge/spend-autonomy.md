# What acts on its own when spend runs low (T-072-01-02)

`T-072-01-01` (`lisa spend`) made the number readable. This is the decision
about what, if anything, reads that number and does something about it —
written down here because the wrong answer is worse than no answer, and
because the desk already has two rules this has to answer to: nothing runs
when nobody asked, and a tool refuses rather than guesses.

## The position: it stops, but never starts or changes

Three positions were on the table (`T-072-01-02`'s own Context):

- **It only says.** Safest, and it fails exactly when the overnight loops are
  running and nobody is watching the desk line.
- **It stops, but never starts or changes.** *Chosen.*
- **It downshifts** — swaps model, drops effort, keeps going. Gets more work
  out of a thin allowance, at the cost of a ticket finishing on a model the
  operator did not choose for it, discovered after the fact in the ledger.

Stopping is the reversible direction. A stopped loop restarts with
`lisa loop`; nothing about what it would have done next is guessed at or
changed. A silent model swap is not reversible in the same way — the ticket
finished on whatever it finished on — and it is exactly the kind of surprising
result the desk's own rule against guessing exists to prevent. "Only says"
was rejected because the story's own opening line is an operator who was
asleep when four panes crashed at once; a mechanism that only speaks is a
mechanism that is silent at the one moment it matters.

**What this does *not* build:** an automatic downshift gear. The *manual* one
— `[agent].model` / `[agent].effort` (`T-071-01-01`) — already exists, is
already "the same lever pulled by hand" the story names, and is the right
tool for an operator who wants to spend a thin allowance deliberately rather
than have Lisa guess how.

## Where it acts, and what it acts on

`lisa spend --guard` (`crates/lisa-cli/src/spend.rs`). The bare `lisa spend`
command is unchanged: read-only, decides nothing. `--guard` reuses the exact
same desk-wide reading and, only when every one of the following is true,
stops *this board's own* loop:

1. `[scheduling].weekly_token_allowance` is configured in this project's
   `.lisa.toml`. No key, no action — see "nothing acts on a number it could
   not read" below.
2. Every host in `lisa spend`'s own desk-wide reading was reachable. A total
   with a hole in it because a machine did not answer is not a low reading —
   it is a reading with unknown parts folded in as if they were zero, which
   `T-072-01-01` already refuses to do for the plain report. The guard
   inherits that refusal rather than relaxing it.
3. This week's spend is at or over 90% of the configured allowance
   (`LOW_SPEND_STOP_PCT` in `spend.rs`). Frontloading 80% on Monday is legal
   (`S-072-01` is explicit this is not a rate limiter) — the threshold exists
   so that "keep going and hope" stops being the default once the number
   really is close, not so spend gets paced against it.
4. This board's own `[scheduling].priority` is `low`.

Only then does it stop. Every other combination reports what it read and does
nothing — including the case an operator will hit most often: spend is high
but this board never opted in, which prints the reading and says plainly
which condition was not met, rather than staying silent about a check that
almost fired.

## Priority is a thing a board can have

`[scheduling].priority` in `.lisa.toml` — `low` | `medium` | `high` |
`critical`, the same four words a ticket's own `priority:` frontmatter already
uses (`lisa_core::types::Priority`). **Default is `medium`.** A board that
never configured this is never the one the guard stops: only a board
explicitly marked `low` is ever a stop target. This is the acceptance
criterion's own wording made literal — *"a board with no priority must not
become the one that gets stopped"* — by making the unconfigured case land on
a value that is not the trigger value, rather than on an absence the code has
to special-case.

This is a *board's* priority, not a ticket's. `T-071-01-01`-era `priority:`
on a ticket already says which piece of work matters more within one board's
queue; nothing on the desk said which whole *loop* — which project, which
machine — was more expendable than another when the shared subscription
allowance runs thin. `[scheduling].priority` is that missing word, read from
this project's own `.lisa.toml`, the same file every other per-board setting
already lives in.

## Nothing acts on a number it could not read

Two places this shows up, both refusals rather than guesses:

- **An unreachable machine.** `lisa spend`'s own aggregation already keeps an
  unreachable host's tokens out of every total rather than counting it as
  zero (`T-072-01-01`). The guard goes one step further: it will not act on a
  desk-wide total *at all* while any host is unreachable, because "the
  reachable part crossed 90%" and "the desk crossed 90%" are different claims
  and only the guard's own board is allowed to act on the second.
- **No allowance configured.** There is no published API for "how much of
  your week is left" (`T-072-01-01`'s own reasoning, unchanged here) — the
  allowance is a number the operator sets by calibrating against what they
  see on screen, exactly as the plain `lisa spend` report already tells them
  to do. An unset allowance is not zero and not infinite; it is unknown, and
  the guard treats it exactly like an unreachable machine — nothing to act
  on.

## Work in flight is ended cleanly, not killed

`stop_for_guard` reuses the identical path `lisa schedulers --stop` already
uses (`crate::schedulers::run_schedulers`) for every scheduler this board has
recorded — the same `zellij kill-session`, the same refusal to stop the
session the caller is sitting in, the same graceful handling of a record
whose scheduler already died. A loop the guard stopped is described by
`lisa schedulers` exactly the way a loop an operator stopped by hand is, and
restarts the same way: `lisa loop`.

## Whatever acts, says it did

`stop_for_guard` calls `rail tell --kind loop-degraded --project <this
board> --what … --do …` after acting. `loop-degraded` is not a perfect fit —
it was written for "running on fewer panes than it was given," and this is a
whole loop stopping — but it is the closest of the four facts `rail tell`
defines, it reaches the overseer's own next-turn hook, and it is what a
stopped loop is closest to: work that is running on fewer resources than it
was configured for, all the way down to zero. Adding a fifth fact belongs to
`rail`'s own repository, not this one.

`rail` is best-effort. If it is absent or refuses, the same sentence is
printed to stdout instead — a stop that already happened must never read as a
stop that went unreported, on a desk where `rail` may not even be installed
everywhere `lisa` is.

## What this does not solve

The second failure mode the story names — *"using too much within a day,
making the rest of the week unavailable"* — is explicitly out of scope here,
per the ticket's own Notes: it cannot be caught at a threshold, because by the
time a threshold trips the week is already spent. The only lever is the daily
reading from `lisa spend` arriving in a morning brief early enough to change
what the day does — a reporting problem, not an orchestration one, and not
this ticket's job.
