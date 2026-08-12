# T-062-01-03 — the plugin pane is a ticker

The plugin pane's title was
`file:/var/folders/kn/…/lisa-plugin-8fd16992c4a2b9fd.wasm` — a temp path and a
content hash, identical on every project, on the widest constant strip of text a
`lisa loop` screen has. It now says what the loop is doing, and changes as that
changes.

This is generation 10. The formatter and the real-Zellij boundary were committed
by earlier generations of this ticket (`a00ba55`, `f0e6ff0`) and are in `HEAD`;
this generation re-verified them end to end at the current tree, found the
boundary test reporting a false failure, and fixed it (`bf12cc8`).

## The three moments, read out of real Zellij

Not composed and asserted in a unit test — read back from
`zellij action list-panes --json --all` on a live session, driven in order on one
running loop (`crates/lisa-cli/tests/fixtures/real_zellij_ticker.sh`, zellij
0.44.3, run today at this HEAD):

```
ticker[idle]:     steer · idle · 1/3 done
ticker[working]:  steer · 1/4 working · T-TICK-02 <1m
ticker[finished]: steer · idle · done T-TICK-02 · 2/3 done
ticker[settled]:  steer · idle · 2/3 done
```

`steer` is the project directory. With two seats working and one an hour in, the
row reads `steer · 2/4 working · T-062-01-03 1h05m +1`.

## What it carries, and what wins the space

Four facts, joined by ` · `. **The order is the priority**, because the terminal
cuts from the right:

1. **The project.** The row appears on every station and is the only part of the
   screen identical across all of them; this is what tells three loops apart.
2. **The state** — `2/4 working`, `idle`, `all done`, with `paused` in front of
   it when scheduling is held. A paused loop with no threads left otherwise
   looks exactly like an idle one.
3. **What just finished**, for 90 seconds. It sits **ahead of** what is running,
   and that is the contested call: the running attempt is still there a minute
   from now, this is not, so the expiring fact gets the better seat.
4. **The attempt running longest**, with elapsed time and `+N` for the rest —
   the question a glance actually asks is not "what is running" but "has this
   been running too long". When nothing is working, this slot carries `2/3 done`
   instead, so a stopped loop still says how far the board got.

**At 40 columns** the working-plus-completion row cuts to
`steer · 1/4 working · done T-062-01-02 ·` — project, state and the completion
survive; the running ticket is what goes. **At 200** the whole row fits with room
to spare; it is bounded at 80 scalar values (`MAX_TICKER_CHARS`, the same bound
the agent panes use) so a pathological ticket id cannot make it unbounded. Both
are asserted (`forty_columns_still_carries_project_state_and_completion`,
`the_row_is_bounded_and_cut_on_a_character_boundary`).

**Idle is never blank.** `steer · idle · 1/3 done`, and a board with no tickets
at all says `steer · idle · no tickets` rather than sitting there looking broken.

**No wasm path, no temp directory, no content hash.** Every observation in the
harness also asserts the row contains none of `.wasm`, `file:`, `/var/folders/`.
The generated layout names the dashboard pane `lisa · starting`, which is the
floor under the plugin's first render and holds even if the plugin never loads.

## The state it reads (nothing is recomputed)

`ticker::compose` is a pure function of the `ui::PluginState` the dashboard is
about to render, taken in `render()` *after* `to_ui_state()` has already built
it:

| Row segment | State read |
|---|---|
| `2/4 working` | `PluginState.active_threads`, `PluginState.slots` — the same two the status line counts |
| `2/3 done` | `PluginState.tickets[].status` |
| `paused` | `PluginState.paused` |
| `T-062-01-03 3m` | `PluginState.active_threads[].{ticket_id, started_at}`, `PluginState.current_time` |
| `done T-062-01-02` | `State.activity_log` — the newest `TicketPhaseChanged { new_phase: Done }`, the same ring the feed renders from |
| `steer` | `State.project_root`, from `get_plugin_ids().initial_cwd` |

`TicketPhaseChanged` rather than `PhaseCompleted`, deliberately: the latter names
the phase a ticket *left*, which is `Review` on the ordinary path but `Implement`
for an operator mark-done. Only the transition says Done in both cases
(`the_ticker_reads_the_finish_off_the_activity_ring`).

Elapsed time is the one place the row renders a shared fact differently: the
threads table shows `2m 30s`, the title shows `3m`. Same value, coarser — see
below.

## Cost

- **Composed** once per render, inside a render that had already built
  `PluginState`. No new file read, no new DAG walk, no new scan of the ticket
  directory. The work is five small `format!`s, a `join`, and one reverse scan of
  the activity ring, which is bounded at `MAX_ACTIVITY_LOG` = 100 entries and
  normally hits a completion in the first few.
- **Applied** to Zellij only when the string differs from the last one applied
  (`set_pane_ticker`). A loop whose facts have not moved makes **zero** host
  calls.
- **Steady-state rate:** one rename per minute per loop while a seat is working
  (the elapsed minute rolling over), and **none at all** while idle. Real state
  changes — a seat taken, a ticket finished, the 90-second window expiring — add
  a handful more. In today's harness run the row changed 4 times across ~107
  seconds of a loop with a live seat.
- **Three loops on one machine:** three independent Zellij sessions with three
  plugins, so 3× the above — at most about three pane renames a minute in total,
  and nothing when the desk is quiet. Renders themselves are unchanged; the poll
  cadence is still `POLL_INTERVAL_SECS` = 5s.

## Not a flicker

Two rules, both tested:

- **Whole minutes.** A per-second figure would rewrite the title on every render
  for nothing a glance can use. Minutes give the row a heartbeat without ever
  moving faster than it can be read.
- **Ties break on ticket id.** `active_threads` comes from a `HashMap`, so two
  attempts started in the same second have no stable order; picking the
  longest-running by `(started_at, ticket_id)` stops the row alternating between
  two names at render speed
  (`threads_started_in_the_same_second_do_not_alternate`).

The row never rotates between two messages. Movement comes from the loop
changing, not from the title cycling.

## What this generation changed

`bf12cc8` — `crates/lisa-cli/tests/fixtures/real_zellij_ticker.sh`.

Running the boundary test at this HEAD, it read all four rows correctly, printed
`real-zellij-ticker: PASS`, and then **failed with exit 101**:

```
rm: /var/folders/…/lisa-ticker.0QF7gK/steer/home: Directory not empty
```

`cleanup`'s `rm -rf` lost a race with processes still writing into the fixture —
the loop runs behind `forkpty`, so killing the runner leaves the CLI, the stub
provider and Zellij's own server alive a moment longer — and under `set -e` that
`rm`'s status became the script's status. A teardown detail was overwriting the
observation the test exists to make, in the direction that matters most: a green
row reported as a red test.

Now `cleanup` kills what this run started (everything of it names the run root on
its command line), removal retries for five seconds while the rest drain, and a
fixture that still will not go warns instead of deciding the verdict. `pkill`
joined the preflight dependency list. Same test, same machine: **exit 101 before,
exit 0 after, and no fixture left behind.**

## Files

**Added** (earlier generations, in `HEAD`)

- `crates/lisa-plugin/src/ticker.rs` — the formatter, `project_label`, 17 unit
  tests. `a00ba55`.
- `crates/lisa-cli/tests/real_zellij_ticker.rs` +
  `crates/lisa-cli/tests/fixtures/real_zellij_ticker.sh` — the real-Zellij
  boundary, `#[ignore]`d like the others in that directory. `f0e6ff0`.

**Modified**

- `crates/lisa-cli/tests/fixtures/real_zellij_ticker.sh` — the teardown fix
  above. `bf12cc8`, this generation.
- `crates/lisa-plugin/src/lib.rs` — `mod ticker`, `plugin_pane_id`,
  `last_ticker`, `set_pane_ticker` / `refresh_pane_ticker` /
  `last_ticket_finished`, the `get_plugin_ids()` capture in `load`, two call
  sites in `render`, 3 tests.
- `crates/lisa-cli/src/loop_cmd.rs` — `name="lisa · starting"` on the dashboard
  pane, and a test for it.

**Attribution note (carried forward, still true):** three S-062-01 tickets were
live in this tree at once and all own `crates/lisa-plugin/src/lib.rs`. The ticker
edits to `lib.rs` and `loop_cmd.rs` landed under T-062-01-04's commits
(`a89043e`, `a0bf2b7`) because that ticket committed first. The code is in `HEAD`
and verified there; only the commit attribution is wrong, and re-committing would
be a no-op. This is the missing dependency edge the workflow notes describe:
none of the three tickets declares a dependency on the others.

## Tests

Verified at this HEAD, by exit code:

- `just check` — **exit 0**. That is `cargo check -p lisa-plugin --target
  wasm32-wasip1`, `cargo fmt --all -- --check`, the three `cargo clippy … -D
  warnings` runs, and `cargo test --workspace` (all suites pass, including 17
  `ticker::tests` and 3 `State`-level ones).
- `cargo test -p lisa-cli --test real_zellij_ticker -- --ignored` — **exit 0**
  against zellij 0.44.3, after the teardown fix; **exit 101** before it, with all
  four rows read correctly.

`just check` was run before the shell-only teardown edit; that file is not
compiled, is not run by `cargo test --workspace` (the test is `#[ignore]`d), and
was re-verified directly by the run above.

## Concerns

1. **The boundary is slow and `#[ignore]`d.** ~107 seconds, 95 of it a deliberate
   sleep proving the completion gives its place back after `DONE_WINDOW_SECS`.
   Nothing in CI runs it; it is invoked by hand, like the three beside it.
2. **The stub seat never gets past "occupied".** The working row is proven with a
   provider that starts, acknowledges and then does nothing, so the harness shows
   `<1m` rather than a real elapsed ladder. `1h05m` is unit-tested, not observed
   live.
3. **"Awaiting human" is not on the row.** An agent blocked on a question reads
   as `working`, which is the one place the title is less than honest. I kept to
   the four facts the ticket named rather than adding a fifth; the dashboard's
   alerts and the agent pane's own title still carry it. Worth a follow-up if the
   operator finds themselves checking the dashboard for it.
4. **The teardown fix is verified by one run, not by a stress loop.** The race it
   closes is timing-dependent: the failing run and the passing run differ by the
   fix, but a single green run is not proof the window is gone for every machine
   load. The failure mode is now bounded either way — the worst case is a warning
   and a leftover directory, not a false red.
5. **Pre-existing, not mine:** `cargo clippy --workspace --all-targets` reports an
   `unused_mut` in `crates/lisa-core/src/completion_journal.rs:1339` (a test
   closure). The three clippy commands `just lint` actually runs do not pass
   `--all-targets`, so this is invisible to the local gate and to CI.
