# T-067-01-03 — a loop regenerates a pane it lost

A `lisa loop` now counts the coding panes its layout made, puts back the ones it
lost, and tells anyone who asks whether it did. Proved against real Zellij
0.44.3: a pane killed under a working board is back in the stack, at the same
size, within a second, with the other agent's seat and lease untouched.

## What changed

### The loop notices — `crates/lisa-plugin/src/heal.rs` (new), `lib.rs`

`heal.rs` is the pure half, in the shape `stack.rs` established: it decides, and
`lib.rs` talks to Zellij.

- **What is counted.** Coding panes visible in the loop's own tab against
  `agent_panes`, the number `lisa loop` wrote into the layout *from the same loop
  that emits the `pane` lines* (`loop_cmd.rs`). Not `max_threads * 2` recomputed:
  the ticket's note is right that two derivations can disagree, and pane loss is
  exactly when they do. `the_layout_declares_exactly_as_many_panes_as_it_creates`
  holds them together for every `max_threads`.
- **How often.** On `Event::PaneUpdate` and nowhere else. Zellij sends it when
  the pane set changes, which is exactly and only when this answer can change, so
  noticing costs one comparison on an event the plugin already handles. No poll.
- **Scoped to the loop's tab**, found through the plugin's own pane. A second tab
  an operator opened is theirs — not counted, and never conscripted into a seat.
  A manifest with no plugin pane, or with every coding pane gone at once, decides
  nothing (same discipline as `stack::observe`).

### It regenerates into the same stack

`open_terminal` for the pane, then `stack_panes(<every coding pane, in layout
order>)` to state the arrangement rather than hope for it, then focus back to
whatever held it. Measured live: after healing, four members share one left edge
and one width with one expanded, and the dashboard is still at `y=34, rows=15` —
byte-identical to launch.

Two Zellij boundaries had to be crossed, and **both failed silently**:

1. **`PermissionType::OpenTerminalsOrPlugins`.** The ticket says the plugin
   "already asks zellij for exactly the permissions this needs". It does not:
   renaming and closing panes need only `ChangeApplicationState`, which is why
   this had never been asked for. Without it `open_terminal` is dropped by the
   server with no error — the only evidence was the regeneration budget running
   out. It is now requested, and `doctor.rs`'s pre-grant list carries it too.
2. **Zellij 0.44 replies to `open_terminal`** with the new pane's id, and the
   zellij-tile **0.43** SDK this is built against never reads it. Left in the
   pipe it is decoded as the next event, the plugin panics
   (`DecodeError … stack: [("Event","name")]`), Zellij logs *"Failed to apply
   event"* and carries on — minus that event. When it was the poll `Timer`, the
   scheduler stopped ticking for the rest of the run while the dashboard kept
   rendering a healthy board and heartbeats piled up unconsumed. The reply is now
   drained, version-gated (`heal::open_terminal_replies`), and the version is
   resolved *before* the command because `get_zellij_version` reads its own
   answer off the same one-line channel.

### Nothing in flight is disturbed

Surviving seats keep their pane, ticket, lease and geometry — asserted in the
native tests and live (`T-STUB-01 still on terminal_0, still beating` after the
pane beside it was killed and replaced).

### A regenerated pane is a fresh seat

`retire_vanished_slots` removes the slot whose pane is gone. If it still held the
current lease, `emit_seat_loss` files T-067-01-02's row **before** the slot is
torn down, the thread fails and the lease is revoked. An ending Lisa already
recorded is not recorded twice: a fence revokes the lease before it closes the
pane, so matching the slot's lease against `current_leases` tells the two apart.
The replacement arrives with no ticket, no lease and no session.

This also fixes a bug that predates the feature: until now nothing ever removed a
slot, so an idle slot pointing at a crashed pane was a seat the scheduler would
launch a session into.

### Regeneration is bounded and says so

Three asks in any ten-minute sliding window (`heal::Budget`). Spend it and the
loop **gives up for the rest of the run**: one line in the feed naming the count,
the window and the way back, the same sentence to anyone who asks, and the run
carries on with the panes it has. Sticky on purpose — a budget that re-arms on a
timer is the spin it exists to prevent, only slower. An ask that has been made
and not answered by a pane appearing holds the next one off for 10s, so one lost
pane costs one ask.

### `rail` can ask for it and does not perform it

`lisa heal-panes` (`crates/lisa-cli/src/heal_panes.rs`,
`crates/lisa-core/src/pane_heal.rs`) writes `.lisa/pane-heal.request`; the plugin
takes it on its poll tick and leaves `.lisa/pane-heal.answer`. The command creates
no geometry — `rail`'s `no-zellij-split` holds, and the plugin inside the Zellij
server remains the only thing that makes a pane.

| answer | when | exit |
| --- | --- | --- |
| `healed` | the board was short; a pane was made **and has arrived** | 0 |
| `already-fine` | every declared pane is there; nothing created | 0 |
| `refused` | the budget is spent, or the layout never declared a count | 1 |

*Asked and healed* is deliberately a claim about a pane that exists: an ask that
finds a short board is held and answered when the pane joins the stack. A
scheduler that never replies is reported as **nothing answering** — its own
outcome, naming whether a loop is running here at all, because that means
something different from a refusal.

### Where an operator sees it

The loop's own title: `myproject · 2/4 panes · 1/2 working · T-019-01 12m`, and
`2/4 panes (gave up)` once it has stopped trying. On `screen-design` this ran for
hours with every surface reading healthy.

### Also

- `README.md` — a `lisa heal-panes` section.
- `docs/knowledge/flag-audit.md` — the four new flags.
- `.lisa/.gitignore` template — the ask and its answer are session state.

## How it is tested

- **`crates/lisa-plugin/src/tests/a_loop_regenerates_a_pane_it_lost.rs`** — 14
  scheduler tests driving real `PaneManifest`s: the measured 2-of-4 board asks
  for one back; the replacement joins the stack and hands focus back; survivors
  keep tickets and leases; a lost attempt is filed as `seat-lost` and its
  replacement is a fresh seat; a pane Lisa closed itself files nothing twice; the
  bound stops at three; an undeclared layout heals nothing; unrecognised
  manifests retire nothing; and the three answers, including the ask being
  consumed once.
- **`heal.rs`** — 10 unit tests on the census, the sliding window, sticky
  give-up, and the version gate.
- **`heal_panes.rs`** — 7 CLI tests including silence-is-not-a-refusal.
- **`doctor.rs`** — `the_pre_granted_permissions_are_the_ones_the_plugin_asks_for`
  parses the plugin's own `request_permission` call and compares it with the
  pre-grant list. This gap is what made failure (1) above cost an hour: a
  permission the plugin asks for and the pre-grant omits produces a Zellij prompt
  inside a blank pane and a loop that waits forever for a keypress nobody knows
  to make.
- **`crates/lisa-cli/tests/real_zellij_pane_regeneration.rs`** + fixture — the
  reproduction the ticket asks for, against real Zellij, `#[ignore]` like its
  siblings. Two tickets running on four panes; kill an idle pane, then kill a
  pane with an agent in it. Asserts four panes and one stack afterwards, the
  dashboard's geometry unmoved, the surviving seat still holding its pane and
  still beating, a `seat-lost` row for the dead one, a replacement whose id is new
  by set difference, and `already-fine` from `lisa heal-panes` on a whole board.
  **Run twice, PASS both times**, with no plugin panic in Zellij's log.
- `real_zellij_stack_follow` and `real_zellij_ticker` re-run and PASS.
- `cargo test --workspace` green (600+); `cargo fmt`; `cargo check -p lisa-plugin
  --target wasm32-wasip1` green.

Note the harness deliberately does **not** assert the board is ever *observed* at
three panes: healing lands on the same `PaneUpdate` that reports the death, so
the short board can be over before anything outside Zellij can look at it. An
earlier draft asserted it and failed for that reason.

## The question the ticket left for review

> whether healing should also apply after the operator closes a pane by hand

**Yes, it heals those too**, for three reasons:

1. Zellij cannot tell them apart. A shell that exited, a crashed emulator, a pane
   Lisa fenced and an operator's `x` all arrive at the server as the same event.
   Any rule that treated them differently would be guessing.
2. There is no supported gesture for "run this board with fewer panes" — that
   control is `max_threads` in `.lisa.toml`. A closed pane is a capacity loss,
   not a configuration change.
3. The annoyance is bounded and self-limiting: an operator determined to close a
   pane wins on the fourth try, in under ten minutes, and the feed says why.

An operator who wants fewer panes should lower `max_threads` and restart. If this
proves irritating in practice, the cheapest fix is a dashboard key that suspends
healing for the session, not a heuristic about who closed the pane.

## Concerns

1. **`just check` fails on `main` and still does**, on four pre-existing lint
   findings under clippy 0.1.97 — `crates/lisa-core/src/completion_journal.rs:1339`
   (`unused_mut`), `crates/lisa-plugin/src/ui.rs:3011` and `:3544`, and
   `crates/lisa-plugin/src/tests/operator_recovery_matrix.rs:503`. Confirmed
   pre-existing by running clippy on a stashed tree. None are in code this ticket
   touched and none are ticket-owned, so they are left alone and reported here;
   `cargo clippy` without `-D warnings` is clean for everything this ticket
   changed.
2. **The extra permission means one more thing the pre-grant must get right.** The
   new parity test covers drift between the two lists, but not a *new* Zellij
   version renaming a permission — that would still show up as a blank pane. The
   existing version preflight is the natural home for that check if it ever bites.
3. **`stack_panes` restates the whole stack on every heal.** Measured harmless on
   0.44.3 (geometry identical, focus preserved), but it is a bigger hammer than
   "insert one pane". Zellij 0.43 has the command; it is not separately exercised
   here because the live harness runs against the installed 0.44.3.
4. **`heal-panes` has no per-command `--help` snapshot** in `help_surface.rs`. It
   is in the top-level snapshot and the flag audit; the per-command snapshots
   cover only the eleven `OPERATOR_COMMANDS`, which already excludes
   `release-seats`, `reset-ticket` and `schedulers`. Consistent with its
   neighbours, not with an ideal.
5. **The reply-drain is a version gate, not a negotiation.** If a future Zellij
   stops replying to `open_terminal`, or starts replying to `stack_panes`, the
   same silent-event-loss appears again. The panic signature is now documented in
   `heal::open_terminal_replies` so the next person recognises it in minutes
   rather than hours.
