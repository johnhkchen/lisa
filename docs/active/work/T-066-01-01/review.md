# T-066-01-01 — the envelope says where the run is

`lisa status --json` now carries `run_location`: the session a run is in, and
whether there is one at all.

```json
"run_location": {
  "state": "idle",
  "session": "fascinating-drum",
  "sessions": ["fascinating-drum"],
  "attach_command": "zellij attach fascinating-drum"
}
```

## What changed

Two commits, five files, all under `crates/lisa-cli/`.

**`66ce74c` — the field**

- `src/seats.rs` — `RunReport` gains `stamped: bool` and a `location()` method.
  `stamped` says whether the shared `.lisa/scheduler.alive` stamp is fresh; it is
  computed once in `assess_run_at` and carried, because every early return below
  it needs the answer and only some of them reach the stamp. `location()` turns
  the report into the four-state answer.
- `src/json_output.rs` — `RunLocationView`, beside `ConfigView`.
- `src/status.rs` — one key in `status_payload`, plus `stamped` in the two test
  fixtures that build a `RunReport` by hand.

**`40c669b` — the promise and the tests**

- `data/json-guide.md` — a `run_location` section, and one for `schedulers[]`
  (see "Scope I widened" below).
- `tests/json_output.rs` — six tests.

Nothing else moved. The refusal string in `loop_cmd.rs` is untouched, the human
`lisa status` prose is byte-identical, and `lisa validate --json` is unchanged.

## The design decisions worth reading

**Why `state` has four values and not a boolean.** The ticket asks that absent be
distinguishable from unidentifiable. Three things were silence before and are now
three different answers:

| Situation | `state` | `session` |
| --- | --- | --- |
| Nobody here | `none` | `null` |
| A run, in a session Lisa knows | `working` / `idle` | the name |
| A run Lisa was never told the session of | `working` / `idle` | `null` |
| Lisa could not look | `unknown` | `null` |

Reading a null `session` as "no run" gets the third row exactly backwards, so the
guide says to read `state` first and says why.

**`working` vs `idle`, and the finished-but-resident case.** `idle` means a
scheduler is resident and nothing has moved in `.lisa/signals/`. A run that has
finished every ticket is exactly that: after `T-065-01-01` it goes on stamping,
so it is still resident, still holds the board, and is still the session to
attach to — but it is not work in flight, and the field does not claim it is.

**Where I put it, which the ticket flagged for review.** Beside `run_summary`,
not inside it. `run_summary` is a record of what a run *did* and is `null` when
there is no board; where a run *is* has to be answerable on a board with no
tickets at all, and answerable when the summary is stale. Two questions, two
keys.

**Why `state` is not derived from `RunLiveness` alone.** This is the subtle part
and the reason for the new `stamped` field. `RunLiveness::Working` is decided by
`.lisa/signals/` having been modified inside the window — and `lisa init` creates
that directory, so a board nobody has ever run reads as `Working` for the next
15 minutes. Mapping `Working → "working"` straight through would have made every
freshly initialised board claim a run. `location()` therefore requires evidence
of a *scheduler* — a live registry record, or a fresh shared stamp — before it
will name any state but `none`. I did not change `assess_run`'s own verdicts;
`release-seats` and `reset-ticket` read those and are unaffected.

**What travels.** Only session names, which is what `zellij attach` takes and
what still means something over `gh codespace ssh`. No pid, no socket path, no
`.lisa/` path. The guide says this in as many words and points at
`schedulers[].zellij_pid` as the field that does *not* travel.

**Additive within `schema_version: 1`.** One new top-level key. Nothing renamed,
nothing removed, no meaning changed. `rail/src/lisa_json.rs:401` keeps working.

## Tests

Six, in `crates/lisa-cli/tests/json_output.rs`, all black-box against the real
binary. The three the ticket asks for are the first three:

1. `a_board_with_a_run_says_which_session_holds_it` — a run: session, sessions,
   attach command.
2. `a_board_with_no_run_says_none_rather_than_falling_silent` — no run: `none`,
   and nulls that a consumer can tell apart from a missing field.
3. `a_board_whose_run_has_finished_still_says_where_it_is` — every ticket done,
   scheduler still stamping, signal directory aged: asserts the board really is
   drained, that `state` is not `none`, and that it is `idle`.

Plus three more the criteria imply:

4. `a_run_lisa_cannot_place_is_not_a_board_with_no_run` — a scheduler with no
   session name: present, unplaceable, and not confusable with an empty board.
5. `a_board_with_two_runs_names_both_and_picks_neither` — the contested board.
6. `the_guide_names_where_the_run_is` — the shape is documented where a consumer
   is told to look.

The fixtures write `.lisa/schedulers/*.alive` records by hand, because no test
can start a Zellij session. That is the exact shape the plugin publishes, and
`lisa-core::schedulers` round-trips it in its own unit tests; what these fixtures
cover is the decision layered on top.

**Verification run** (in a detached worktree at `HEAD`, carrying only my five
files — see "Concurrency" below):

- `cargo test --workspace` — exit 0, every suite green.
- `cargo clippy -p lisa-cli --all-targets -- -D warnings` — exit 0.
- `cargo fmt --all -- --check` — exit 0.

## Concerns

**1. A pre-existing clippy failure at `HEAD`, in a file this ticket does not
own.** `cargo clippy --workspace --all-targets -- -D warnings` fails on
`crates/lisa-core/src/completion_journal.rs:1339` — `unused_mut` on
`let mut fail_one_generation`. It is at `HEAD`, in a file I did not touch, and it
is one word. I left it alone rather than commit outside this ticket's
`--include` ownership. Someone should file it; `just check` fails until then.
(`cargo test --workspace` is unaffected — `unused_mut` is warn-level outside
clippy's `-D warnings`.)

**2. An acceptance criterion whose two halves disagree, and the reading I
took.** The criterion is:

> **A finished-but-resident run is not reported as running.** That distinction is
> `T-065-01-01`'s subject and the reason a second scheduler started here on
> 2026-08-12; whatever this field says must not reintroduce it.

Taken literally the first sentence and the last conflict: `S-065-01` states the
lesson of that incident as "**A finished run is not an absent one**", so the
failure mode to avoid is reporting a finished-but-resident run as *absent*. I
built to satisfy both halves: such a run is reported (`state` is never `none`,
`session` is named, `attach_command` works), and it is reported as `idle` rather
than as running. Test 3 asserts both directions — `assert_ne!(state, "none")` and
`assert_eq!(state, "idle")`. If the intent was in fact that a drained board
report `none`, that is a one-line change in `location()` and a reversed
assertion — but I believe it would reintroduce the incident, so I did not make
it.

**3. `run_location` is not sufficient to decide whether to start a loop, and the
guide says so.** `lisa status` is a one-shot command that deliberately does not
shell out to Zellij, so it cannot see a running session that stopped stamping.
`lisa loop` does ask, and refuses on it (`T-065-01-03`). A consumer that starts a
run on `state == "none"` alone would be trusting weaker evidence than the loop's
own. The guide's `run_location` section states this directly. `rail up`'s actual
question — "does this loop already have a strip beside it" — is fully answered
here; "may I start one" is not, and should stay `lisa loop`'s to answer.

**4. Scope I widened, deliberately.** `schedulers[]` was already being emitted by
`status_payload` and was in no version of the guide, which under the guide's own
rule 4 means it was not part of the contract. The criterion asks for "whatever
else identifies it on a machine with several", and that array is exactly that, so
I documented it rather than duplicating pids into `run_location`. It is a new
promise about an existing shape; if the product owner would rather it stayed
uncommitted, deleting that one section is the whole reversal.

**5. Concurrency, and what I could and could not verify.** `T-066-01-02` is
working the same branch and has `loop_cmd.rs`, `main.rs`, and a new `headless.rs`
in flight; the shared tree did not compile while I was working. I verified in a
detached worktree at `HEAD` carrying only my five files, so my gates are honest
about my own change and say nothing about the merged result. The two are
disjoint — no file overlaps — but whoever lands second should re-run
`cargo test --workspace`.
