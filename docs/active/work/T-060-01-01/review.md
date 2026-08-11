# T-060-01-01 — reconcile leases left by a run that died

## What this does

A run that dies without shutting down cannot withdraw its pane lease markers,
because the process that would have done it is the one that died. Lisa now
notices, says so where an operator already looks, and gives them one deliberate
command to free the seats.

Three things changed:

1. **The scheduler stamps itself.** Every poll tick (5s) the plugin rewrites
   `.lisa/scheduler.alive` with the moment it happened.
2. **`lisa status` reports abandoned seats** in prose, under **Seats held by a
   run that is gone**, and in `attempts[]` as `abandoned` / `abandoned_reason`.
3. **`lisa release-seats`** frees them, in `lisa clean`'s consent shape: a bare
   run prints the list and the evidence and changes nothing; `--release` carries
   it out.

## The test, and what corroborates it — the substance of the ticket

The ticket proposed `session_timeout_secs` as the discriminator. **I did not use
it as one.** The setup guide states that budget is advisory — *"an over-budget
session is flagged but never interrupted"* — so a lease older than it is not a
session Lisa would have ended, and a long, healthy session would have been
released. A timer alone cannot answer this question honestly.

What can is a statement from the thing whose death actually matters. Nothing but
a running scheduler writes `.lisa/scheduler.alive`, and it stops the instant the
process stops — crash, kill, machine swap, closed terminal — because there is no
shutdown path involved. A seat is called abandoned only when **two independent
facts agree**:

- **No scheduler has stamped** for longer than the window, **and**
- **`.lisa/signals/` has gone quiet** for the same stretch — no file in it and
  not the directory's own timestamp has moved. A live run's panes write
  heartbeats into it, and a live plugin consumes them (which moves the
  directory even when it leaves no file).

Either alone would be wrong, and each covers the other's blind spot: a run
detached into the background and blocked on a question writes no signals for
hours and is kept by the stamp; a scheduler that cannot write its stamp still
has panes signalling and is kept by the quiet test. This is also why the stamp
lives *beside* `.lisa/signals/` and not in it — a stamp among the signals would
make the second fact a restatement of the first.

Every other outcome is *unclear* and keeps the seat: a stamp dated ahead of this
machine's clock, an unreadable signal directory, an unparsable stamp. Doubt
never frees a seat, because handing out a seat somebody is working is the
expensive mistake.

**The window** is one of the project's own numbers, never a new magic constant:

| Situation | Window | Why |
|---|---|---|
| A stamp exists | `max(wind_down_secs, 6 × poll interval, 60s)` — 5m by default | The project's own statement of how long a pane must be silent before it counts as idle |
| No stamp has ever been written here | `max(session_timeout_secs, 900s)` — 1h by default | Without a stamp the whole verdict rests on quiet, which a live run can produce honestly, so the bar is much higher. This is the path a project running a pre-upgrade plugin takes, and it is why upgrading cannot make Lisa call a live old run dead. |

## Why these places

- **The diagnosis belongs in `lisa status`** because the complaint was not that
  recovery was impossible — running `lisa loop` again always cleared it — but
  that it was undiscoverable, and the only cure was the exact thing the stale
  state existed to prevent. `status` is where the misleading `2 in progress`
  was read, so the correction goes directly under it.
- **The deletion is its own command, not a `lisa clean` category.** Clean's one
  rule is *Lisa's litter for a ticket your board records as done, in a directory
  Lisa created*. A lease for a ticket in `implement` is the opposite of that, and
  bending the rule to cover live-state files would weaken the invariant that
  makes clean safe. It reuses clean's consent shape and its symlink gate
  (`clean::reachability`) rather than reimplementing either.
- **Not automatic at `lisa loop` startup.** Loop already overwrites the leases
  as it places seats, so automating it there would add a second mechanism for
  something that already works, in the one situation where the operator does not
  need telling.

## Files

**New**

- `crates/lisa-core/src/liveness.rs` — the stamp's shape and path, shared by the
  plugin that writes it and the CLI that reads it, so they cannot disagree.
- `crates/lisa-cli/src/seats.rs` — the verdict, the plan, and the command. The
  module doc carries the reasoning above.

**Changed**

- `crates/lisa-plugin/src/lib.rs` — `stamp_scheduler_alive()`, called first in
  `poll_tick` and once at load. Best-effort and deliberately not a fence: a
  stamp that cannot be written costs a diagnosis, and failing the scheduler over
  it would turn a reporting aid into an outage. Nothing in scheduling reads it.
- `crates/lisa-cli/src/status.rs` — `SeatAttempt` gains `abandoned` and
  `abandoned_reason`; the prose section; `read_seat_attempts` takes the verdict.
- `crates/lisa-cli/src/clean.rs` — `reachability`, `display_relative` and
  `plural` widened to `pub(crate)`. No behaviour change.
- `crates/lisa-cli/src/main.rs` — the `release-seats` subcommand.
- `crates/lisa-cli/src/templates.rs` — `scheduler.alive` added to
  `.lisa/.gitignore`; `crates/lisa-cli/src/init.rs` — its fixtures.
- `crates/lisa-cli/data/json-guide.md`, `README.md`,
  `crates/lisa-cli/src/setup_guide.rs`, `docs/knowledge/flag-audit.md`,
  `crates/lisa-cli/tests/help_surface.rs` — the contract and the docs.

## Tests

`just check` passes: fmt, `cargo check -p lisa-plugin --target wasm32-wasip1`,
clippy `-D warnings` on all three crates, and `cargo test --workspace` (no
failures).

19 new tests. The judgement ones take the clock as a parameter rather than
backdating files, so they state "a day later" directly:

- a fresh stamp holds every seat, however old the seats are
- a silent-but-stamping run (the detached, blocked-on-a-question case) keeps its
  seats
- a stale stamp alone is not enough while signals keep moving
- both facts agreeing is what makes a seat abandoned
- with no stamp at all, Lisa waits out a whole session budget (20m of quiet is
  not enough; 2h is)
- a stamp from the future keeps the seats rather than guessing
- a tiny configured `wind_down_secs` cannot shrink the window below the floor
- a dry run prints the evidence and removes nothing
- releasing removes exactly what the plan named, and nothing else in the
  directory — a non-`pane-*` file and the stamp itself both survive
- a live run makes the command refuse and list what it kept
- `abandoned` and `abandoned_reason` serialise as the guide documents
- the plugin stamps beside the signal directory, never in it, and rewriting does
  not accumulate files

**Also driven by hand**, because the reported failure is a whole-system one:

- Against a fixture reproducing the measured incident — four leases, tickets in
  `implement`/`review`/`done`, everything last touched 18h ago — `lisa status`
  names all four seats with the evidence, `attempts[]` carries
  `"abandoned": true`, `lisa release-seats` lists them, `--release` frees them,
  and a second run says `No seats are held here.`
- Against **this repository, during this live run**: `lisa release-seats`
  refuses, listing all four leases as `skip (a run may still be holding it)`,
  because something wrote in `.lisa/signals/` minutes ago. That is the safety
  property exercised against a real live run rather than a fixture.

## What still concerns me

- **Two of my commits were swept by a concurrent ticket, and one of mine swept
  nothing but landed inside theirs.** T-061-01-01 was in `implement` at the same
  time and edits the same two files. Its commit `9bb7387` carried my
  `stamp_scheduler_alive` plugin code, and `daad323` carried my `.lisa/.gitignore`
  line, before I could commit either. That briefly left `HEAD` referencing a
  `lisa-core` module that was not committed yet; my first commit closed that.
  The tree is correct and `just check` passes, but the attribution is wrong in
  history, and this is the missing-dependency-edge case the workflow warns
  about: **T-060-01-01 and T-061-01-01 should have had an edge between them**,
  both touching `crates/lisa-plugin/src/lib.rs` and
  `crates/lisa-cli/src/templates.rs`.
- **Pane markers left in an innocent repository** by the S-061-01 bug will read
  here as seats of a run that never existed in that project, and
  `release-seats` will offer to free them after the no-stamp window. I think
  that is the right outcome and it is a clean answer to T-061-01-01's open
  question about those directories — but the two tickets landed independently,
  so nobody has exercised them together. Note that `release-seats` requires a
  Lisa project, so a bare `.lisa/signals/` created by `mkdir -p` in a
  non-Lisa repository is still out of reach.
- **The stamp is a new file in every project's `.lisa/`.** It is gitignored, and
  `lisa init` appends the rule to existing projects, but a project that never
  re-runs `lisa init` after upgrading will see it untracked once a run starts.
- **`abandoned` is uniform across panes** — the verdict is about the run, not the
  seat, which is true for the case this fixes (everything on disk predates the
  moment the run stopped) but would need revisiting if Lisa ever ran two
  schedulers against one project.
- **Clock changes are not modelled beyond refusing to guess.** A machine whose
  clock jumps backwards produces `Unclear` and keeps every seat, which is safe
  but silent; the operator gets a sentence only from `release-seats`, not from
  `status`.
