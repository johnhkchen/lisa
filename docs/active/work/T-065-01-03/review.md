# T-065-01-03 — a live session on this board is a refusal, not a name

## What changed

One commit, `ec32dc1`.

The refusal now turns on the fact that was already on screen. `lisa loop` asks
Zellij what it is holding, and a session **still running** under one of this
board's own names stops the start on its own — no stamp is consulted, and none
has to be fresh.

**`crates/lisa-cli/src/session_name.rs`**

- The listing is read long instead of `--short`: `--short` prints names only,
  and the `(EXITED - attach to resurrect)` marker that separates a dead session
  from a running one is the rest of the line.
- New `Session { name, running }` and `BoardSessions { naming, running_here }`.
  `resolve` returns both facts from the one listing it already fetched: the name
  this run would take, and every session running under this board's names.
- `names_this_board` is exact — `lisa`, `lisa-2` … `lisa-99` and nothing else.
  `lisa-live-codex-7409` is somebody's hand-named session that merely starts the
  same way, and refusing over it would be a lockout with no way out.
- A line is read as running unless it says `EXITED`. That is the safe
  direction: a marker some future Zellij renames costs a refusal an operator can
  clear, not a second scheduler nobody notices.

**`crates/lisa-cli/src/loop_cmd.rs`**

- `second_scheduler_refusal(root, running_here, current_session)` now composes
  two independent kinds of evidence: live roster stamps (unchanged, still
  evidence) **and** running sessions. A stamp and a session belonging to the
  same run are reported once, not twice.
- The refusal names, per holder, `zellij attach <name>` (go look) and
  `zellij kill-session <name>` — the command measured to work on 2026-08-12 when
  `kill <pid>` did not. A record with no session name says so and points at
  `zellij list-sessions` instead of printing nothing.
- The closing paragraph now says the thing that was actually confusing: *a run
  stays on the board until its session ends, and a run that has finished every
  ticket is still sitting there with nothing to do.*
- When the holder is the session this terminal is in, the line says so.
- Zellij runtime resolution and session naming moved above the refusal, since
  the refusal has to ask that exact Zellij what it is holding. Nothing is
  written and nothing is launched before it.

**`crates/lisa-cli/src/schedulers.rs`** — `current_session()` is now
`pub(crate)` so the refusal reads `ZELLIJ_SESSION_NAME` from the one place that
already had it.

## Acceptance criteria, one at a time

**A live session is grounds to refuse on its own.** Yes. With an empty
`.lisa/schedulers/` and no stamp anywhere, a running session refuses —
`a_running_session_refuses_even_with_nothing_in_the_registry`, and the real-world
run below.

**A finished run that is still resident is caught.** Reproduced twice.

- Against real Zellij 0.44.3, on this machine, 2026-08-13: a temp board, a real
  detached session named for it (`zellij attach --create-background
  probe-finished`), then `lisa loop --path <board>` →

  ```
  Error: There is a run already running on this board, so Lisa did not start another one. …

    probe-finished — a Zellij session still open on this board
      look at it with: zellij attach probe-finished
      stop it with:    zellij kill-session probe-finished
  ```

  exit 1, no session started. The board and session were killed and deleted
  after. This is the stricter case than the ticket's: not a stale stamp, *no
  stamp at all*.
- `a_finished_run_still_sitting_in_its_session_refuses_the_next_loop` is the
  ticket's exact shape: a 25-minute-cold stamp (which alone refuses nothing,
  asserted) plus a running session (which refuses), and the cold stamp is still
  read out — `whose scheduler last checked in 25m ago`.
- `loop_refuses_a_board_whose_session_is_still_running` drives the whole CLI over
  a scripted `zellij list-sessions` and asserts Zellij was never launched.

**The refusal says what to do.** Attach and kill-session per holder, both
printed with the session name filled in.

**Attaching.** My answer, in one line: **`lisa loop` should keep refusing, and
the attach affordance belongs to the caller, not to this command.** `lisa loop`
was asked to start a run; silently handing the caller somebody else's running
board is a different thing than it asked for, and it is impossible anyway for a
caller with no terminal — an unattended slot cannot be attached to anything. So
the refusal names `zellij attach <name>` and the next keystroke is a person's.

For the unattended presser `S-065-01` is building toward, the right outcome is
not attach either: it is a **success that means "a run is already here, nothing
to do"** — a separate word, or a flag on this one, whose contract is *ensure a
run exists* rather than *start one*. That is a decision about the button's
contract, it belongs with `T-065-01-01`/`T-065-01-02` where the button is being
built, and it is cheap to add on top of this: `running_here()` is exactly the
predicate such a mode would branch on. Nothing here forecloses it. What this
ticket refuses to do is guess at that contract by making `lisa loop` do
something other than what its name says.

**`scheduler.alive` staying as evidence.** It stays. Live roster records still
produce holders, still print their pid and their age. It is no longer the only
evidence and no longer the deciding one.

**The `-2` suffix path.** Kept, and now genuinely justified rather than merely
present. Zellij refuses a duplicate session name even when the session is dead —
`Session with name "steer" already exists, but is dead.` is exit 1, not a start —
so a crashed run leaves a name that cannot be reused, and a start that dies on
that is worse than a start that takes a number. What changed is that numbering
can no longer paper over a *live* holder: the refusal runs first, so
`SessionNaming::NextRun` is now reachable only past dead sessions.
`an_exited_session_takes_the_name_without_holding_the_board` and
`loop_starts_past_an_exited_session_of_the_same_name` pin that both ways, and the
module doc says it in the source.

## Tests

`just check` (fmt, clippy, workspace tests) — **exit 0**, 31 suites ok.

New: 3 in `loop_cmd` (finished-run refusal, registry-less refusal, one-run-not-
two dedup), 4 in `session_name` (real-listing parse, running-session survey,
exited-session numbering, name-family exactness), 2 end-to-end in
`tests/zellij_version_preflight.rs` (refuses a running session and launches
nothing; starts past an exited one and takes `-2`). Updated: the two existing
refusal tests for the new signature and closing text.

## What still concerns me

- **Two checkouts with the same directory name.** Session names come from the
  project directory, so a live session named `lisa` in a *different* `lisa`
  checkout would refuse a legitimate start here. Refusing is the cheap-to-check
  side of the ticket's own trade, and the message names the session so the
  mistake is visible in one read — but there is no override flag, and the way out
  is to rename a directory or stop the other run. I judged an override worse than
  the false positive it fixes: the accident this ticket exists for was a bare
  `lisa loop`, and a hole in the guard is what the story is about. Worth
  revisiting only if it actually bites.
- **The refusal trusts one listing.** If `zellij list-sessions` cannot be run,
  the listing is empty and the refusal falls back to stamps alone — the old
  behaviour. That is deliberate (an unrunnable Zellij holds no sessions), but it
  means the guard is only as good as that call.
- **The real-world half of the exited case is covered by fixture, not by hand.**
  I killed the probe session rather than letting a real `lisa loop` start a
  second real board in the scratch directory; the `-2` start past an `EXITED`
  session is proven by `loop_starts_past_an_exited_session_of_the_same_name`
  against the same line format captured from Zellij 0.44.3.
- **Detection and refusal now overlap by design.** `note_other_schedulers`
  (T-063-01-01) still reports company after the fact. Nothing here removes it;
  it covers the sessions this refusal cannot see, such as a run started under a
  name that is not this board's.
