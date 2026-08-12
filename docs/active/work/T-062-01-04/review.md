# T-062-01-04 — a pane whose agent will not leave wedges the ticket

## What this attempt found on arrival

Attempt 6 opened onto a ticket that was already built. Six commits for this
ticket were on `main`; the ticket file still said `phase: implement`. So the
first job was not to write code but to check that the code there was real, that
it does what the acceptance criteria ask, and that nothing was left half-done.
It was, it does, and one thing was.

| Commit | What it does | Attempt |
| --- | --- | --- |
| `7dcf224`, `36a0999` | the ticket itself, and its corrected diagnosis | 1–2 |
| `a89043e` | `lisa loop` and `lisa doctor` know what an inherited agent session looks like | 5 |
| `516d24f` | every launcher strips those markers before the agent starts | 5 |
| `a0bf2b7` | a held pane keeps the seat, not the ticket: wedge, void record, phase return, dashboard alert | 5 |
| `a82ba86` | `lisa reset-ticket` — the supported way out | 5 |
| `2151de8` | the shell-ready probe says in code what it can and cannot prove | 5 |
| `5db6534` | `lisa doctor`'s environment section is held to what it must say | **6** |

## What this attempt changed

`crates/lisa-cli/src/doctor.rs` — two tests.

`format_inherited_session_markers` was the one piece of this ticket's work with
no test on it, and it is the piece an operator actually reads: the line that
tells them, before a run starts, that this shell would hand a marker down. The
tests hold it to the two things that matter — that it names **every** marker it
found rather than counting them, and that a clean shell gets one plain sentence
and not a warning. The count alone would have been a shrug in the field failure;
the names are what let an operator recognise their own situation.

Nothing else needed changing. The rest of this review is the verification.

## The criteria, checked against the code and the tests

**An agent is not launched into an environment that will break it.** Two layers,
and the second is the one that matters. `lisa loop` reads its own environment
before anything spawns and starts zellij with each marker removed
(`loop_cmd.rs:518`), printing what it cleared. But a session an operator started
themselves is not covered by that, so every launch payload Lisa writes begins by
unsetting the markers in the pane's own shell — generated from the same
constants, so the shell side and the Rust side cannot drift.

The matching is by prefix (`CLAUDE_CODE_`), not a fixed list. That is the right
call: a marker a future release adds is precisely the one that would otherwise
travel down silently. And `CODEX_HOME` and `ANTHROPIC_API_KEY` are explicitly
tested as *not* markers, so nothing strips configuration an operator set on
purpose.

Two of the seven `session_env` tests do not assert on the fragment's text — they
run it under `sh` with a poisoned environment and check what survived. That is
the correct shape for this bug, whose whole lesson is that a shell fragment can
look right and not be right.

**A pane whose agent has not released it does not receive the next attempt.**
The exit ceiling used to mean *launch anyway*. It now means *this pane is
wedged*: the ticket is released, the pane leaves the rotation, nothing is typed
into it. `wedge_seat` (`lib.rs:8288`) does four things in an order that matters —
void record, phase walked back, lease released, pane remembered — and the
comment says why the order matters, which is the kind of thing that rots
silently otherwise.

Reproduced two ways.
`a_held_pane_does_not_receive_the_next_attempt_and_the_ticket_moves_seats`: with
a second pane free, the ticket is seated there.
`with_every_seat_held_the_scheduler_waits_and_names_the_pane`: with no other
seat, the scheduler waits and records `SeatsWedged { panes: [10] }`. Both pass.

**A retry that cannot possibly work does not happen.** The rule the ticket
settled on — *a retry is worth making when the seat it would run in has been
proven vacant; a re-offer into a pane that has proven the opposite is not a
retry* — is the right distinction, and it is drawn from evidence the pane cannot
forge: provider hooks arriving after the `/exit`. The ticket is still retried.
Just never there.

**The dashboard says what is actually wrong.** `⛔ HELD`, reading *"Pane 3 is
still held by its previous agent; T-062-01-03 moved to another seat"*, with two
suggested actions. It is projected from live state on every render
(`lib.rs:11035`), so it appears and disappears with the fact rather than being
pushed once and left to go stale. Nothing in this path says `Session failed`,
because no session did.

**A supported way out exists.** `lisa reset-ticket`. Run against this repository
just now:

```
$ lisa reset-ticket T-062-01-04
Planned actions:
  T-062-01-04  implement → ready   (6 attempts)

A reset changes the ticket's phase and nothing else: committed work, attempt
history, and finished tickets are left exactly as they are.

Dry run complete. No changes made. Add --apply to carry this list out.

$ lisa reset-ticket T-060-01-01
Planned actions:
  T-060-01-01  finished — left alone
...
Nothing to reset.
```

Two details worth naming because they are easy to get wrong and were not. The
bare run is read-only and says so, so an operator standing at a stuck board can
look before they touch — it printed the plan above during a live run without
refusing, because printing is safe. The live-run guard sits *after* the plan and
*before* the first write (`reset_ticket.rs:221`), so an operator who is refused
still sees what they were asking for. And in the common case nobody runs
anything at all: the wedge walks the ticket back itself, and the pane rejoins the
rotation once it has been quiet.

**Attempt counters are accounted for.** The counter still climbs — attempt ids
are minted monotonically, and walking one back would break the fencing that
keeps two attempts off one ticket. So the refused attempt writes
`.lisa/attempts/<ticket>/<n>/void.json` instead, and `lisa reset-ticket` reads
them: *"4 attempts, 3 of which never started a session"*. That is the sentence
the field record was missing. No `void.json` exists in this repository yet,
which is expected — every attempt here did start a session.

**Is the shell-ready handshake the right shape?** No, and the answer the earlier
attempt reached is right, so I will not restate it at length: the mechanism is
sound and the channel is wrong. What the probe attests is *someone here can run
a shell command*. What the scheduler needs is *the pane's shell is what reads my
keystrokes*. Those differ exactly in the case the probe exists to detect, and no
typed command can separate them, because a shell and a TUI both accept typed
lines. Evidence for vacancy cannot travel down the channel the occupant
controls; it has to come from the host. This ticket does not build that. It takes
the probe out of the path where it was doing damage and records the limit in
`shell_readiness_probe`'s own doc comment.

## How it is tested

`just check` — `cargo check` for `wasm32-wasip1`, `cargo fmt --check`, `clippy -D
warnings` across all three crates, `cargo test --workspace`. Run twice this
attempt, before and after my commit. **Exit 0 both times**, 31 test binaries, no
failures. (Checked by exit code, not by reading the output.)

This ticket's tests, all present and passing in that run:

- `session_env` (7) — prefix and exact matching, configuration variables
  explicitly not matched, and two that execute the fragment under `sh`.
- `loop_cmd` (2) — the zellij child has exactly the found markers removed; a
  clean environment is untouched.
- `doctor` (2, new this attempt) — the environment section names every marker
  and says Lisa drops them; a clean shell gets one plain sentence.
- `lib.rs` `test_launcher_starts_the_agent_without_an_inherited_session_marker` —
  runs the real generated launcher under a poisoned environment.
- `deadline.rs` `the_exit_ceiling_wedges_a_held_pane_and_still_releases_a_quiet_one`
  — the three-way boundary: hold below the ceiling, wedge above it while noisy,
  ordinary release for a pane that went quiet however late.
- `lib.rs` three scenario tests over a field fixture — seat refused and ticket
  moves; every seat held and the scheduler waits; wedged pane rejoins when quiet.
- `wedge.rs` (5), `reset_ticket.rs` (6).

Manual: the `lisa reset-ticket` runs quoted above, against this repository, with
a live run going. I did **not** run `lisa doctor` live — it cleans the Zellij
plugin cache and seeds Codex trust, which is a write into a tree three other
threads are working in. The two new unit tests cover the line I would have been
looking at, and are better evidence anyway.

## Concerns

1. **A session's work landed under a "delivery failed" verdict, and this attempt
   is the cost.** `.lisa/provenance.jsonl` records attempt 5 as
   `delivery-failed — "provider did not acknowledge the bounded chat
   assignment"`, ended `12:00:16`. That same attempt then committed five units
   between `12:06:31` and `12:24:29` and wrote its review at `12:26`. Attempt 6
   was launched at `13:44` to do work that was already done. This is a different
   defect from the wedge — nobody was stuck, the acknowledgment probe was simply
   wrong — but it is the same family the ticket is about: two sides holding
   contradictory beliefs about whether a prompt arrived, and a full session spent
   on the disagreement. It is not in this ticket's acceptance criteria and I have
   not fixed it. It is the clearest follow-up on the board, and the evidence for
   it is sitting in this repository.
2. **`crates/lisa-plugin/src/lib.rs` is shared with in-flight work.**
   `T-062-01-01` was editing the same file while this ticket ran, and
   `lisa commit-ticket` commits whole paths — so commit `516d24f` carries that
   ticket's then-uncommitted stack work as well as this ticket's. I verified
   this rather than taking it on trust: `git show 516d24f -- .../lib.rs` contains
   `+mod stack;`, `stack_view`, `pane_heartbeats`. Nothing was lost — the other
   ticket's own commits landed afterwards — but one commit is mixed. That is the
   missing dependency edge the workflow warns about, not something the isolated
   transaction can fix.
3. **Startup recovery still trusts the probe.** `begin_startup_recovery` types
   the readiness probe into a pane whose occupant is unknown, and a live TUI can
   still answer it, costing one relaunch into itself. Bounded
   (`MAX_SAME_PANE_STARTUP_RELAUNCHES = 1`) and now documented at the probe, but
   it is the same forgery this ticket names. Host-side vacancy evidence would
   close it and concern 1 has a claim on the same work.
4. **The wedge release rests on silence.** A wedged pane rejoins after 90 seconds
   with no provider hook. That is the same evidence every other release in the
   file uses, held to a longer standard, but it is still an inference: a live TUI
   that simply stops emitting will be believed departed. If that proves wrong in
   the field, the honest next step is host-side evidence, not a longer timer.
5. **Not reproduced against real zellij.** The wedge path is covered by native
   tests over a hand-built fixture. The repository has `real_zellij_*` tests for
   other paths; this one has none, because reproducing "an agent that will not
   exit" needs a live provider session the test would have to hold open. Worth a
   follow-up if the behaviour is ever doubted.
6. **The marker set is Claude-shaped.** Codex has no equivalent nested-session
   marker in this build, so nothing is stripped for it. If one appears it goes in
   `NESTED_SESSION_PREFIXES` and both layers pick it up.
