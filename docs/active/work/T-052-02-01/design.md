# Design — T-052-02-01 say-it-once

Two independent noise sources, two decisions. Both grounded in the map from
`research.md`.

---

## Part A — the scheduling skip

**Fact:** "T-002 does not need a new thread; it already has a live one."
**Problem:** it is Info-logged on every scheduling pass (§2 of research), and a
pass runs on nearly every event.
**Constraint (P2):** demotion, not erasure. The fact must stay reachable in the
Shift+D dump through a berth that is not the activity ring.

### Options

**A1 — Delete the emit.** One line removed, zero new state. Rejected: violates
the epic's explicit demotion-not-erasure constraint. The dump's only event
record is the ring, so deleting the emit deletes the fact everywhere.

**A2 — Throttle it.** Emit at most once per ticket per N seconds, or only on the
*first* pass that declines a given ticket. Rejected on two counts: it still puts
a recurring no-op in the operator's newspaper (just less often), and per-ticket
time state is the first brick of a severity/rate-limit framework, which N4
forbids. It also interacts badly with T-052-02-02, which is about to add
count-folding to the same append path — two suppression mechanisms fighting
over the same entries.

**A3 — A second capped ring the feed never projects.** `debug_log:
Vec<LoggedActivity>`, appended by a `log_debug` sibling, rendered as its own dump
section. Rejected: it duplicates the ring machinery T-052-02-02 is about to
modify (a second trim, a second envelope, a second folding question), and it is
the wrong shape for the question being asked. "Why didn't X spawn?" is a
question about *now*, not about history — a 100-entry chronological ring answers
it by making the reader scan for the most recent mention of X.

**A4 — A last-scheduling-pass record. ✅ CHOSEN**

One `Option<SchedulingPass>` field on `State`, overwritten wholesale at the end
of every `schedule_ready_tickets()` call, rendered as its own
`=== Last Scheduling Pass ===` section in `format_snapshot`.

```
=== Last Scheduling Pass ===
at:        1784510049 (unix epoch)
ready:     3
spawned:   1
declined:  2
T-002-01-02    thread already running
T-002-01-03    global thread cap reached (2/2)
```

**Why this one.**

- *Last-write-wins is the correct shape.* The question is "why didn't X spawn on
  the most recent pass" — a record that is overwritten each pass answers it
  directly and cannot go stale or scroll away.
- *Bounded by construction.* At most one entry per ready ticket, replaced every
  pass. No trim, no cap, no growth. Nothing for T-052-02-02 to reconcile.
- *It is a net information gain, not just a relocation.* Three of the decline
  arms in `schedule_ready_tickets` today log **nothing** at all — the global cap
  (lib.rs:4998), the per-provider cap (lib.rs:5010), and no-slot-available
  (lib.rs:5022) only bump a local `unscheduled` counter that is never surfaced
  per-ticket. After this change the dump answers "why didn't X spawn?" for
  *every* admission arm, where today it answers for exactly one. Demotion buys
  the operator more than it costs them.
- *It matches existing dump idiom.* `format_snapshot` is already a series of
  last-known-state sections (`=== Agent Slots ===`, `=== Last Known Health ===`).
  A pass record belongs with them, not in the chronological tail.

**Decline reasons captured** (one variant per existing `continue` arm that is a
pure admission decision):

| Arm | Reason |
|---|---|
| lib.rs:4948 durable-Done mask | `DurableDoneMasked` |
| lib.rs:4964 thread exists | `ThreadAlreadyRunning` ← the demoted one |
| lib.rs:4998 global cap | `GlobalCapReached { running, max }` |
| lib.rs:5010 provider cap | `ProviderCapReached { agent }` |
| lib.rs:5022 no compatible slot | `NoSlotAvailable` |
| lib.rs:5047 pane awaiting question | `PaneAwaitingQuestion { pane_id }` |

The two arms below those (lease mint failure, marker publish failure) already
log `Error` events. They stay in the feed — a failure *is* operator news — and
are recorded as `SpawnFailed` in the pass record too, so the section is a
complete account of the pass rather than a partial one.

---

## Part B — the phase-transition stutter

**Fact:** "T-002 finished Research."
**Problem (three compounding):** (1) both `PhaseCompleted` and
`TicketPhaseChanged` project to a "completed" line, so one emit-pair is two
lines and the second names the phase just *entered*; (2) `rebuild_dag`'s
reconciler re-emits `TicketPhaseChanged` for the same transition in the same
tick (research §3); (3) `ThreadExited` mints a latent third "completed Done"
shape from a variant nothing emits.

### The sentence

Keep `"{ticket_id} completed {phase.full_name()}"` with `phase` = the phase
**left**. Not a default-by-inertia: E-052's "Done looks like" section quotes the
target feed verbatim as `"T-016-01-01 completed Design — 3m ago"`. The epic has
already picked the sentence; this ticket's job is to make it appear once.

Rejected: `"{ticket} Research → Design"`, which names the transition more
literally. It would require adding `old_phase` to `ui::ActivityType::
PhaseCompleted` and churning every render fixture (ui.rs:1753, 1847, 2732) and
projection test (lib.rs:11037, 13021) — real cost, against the epic's own copy.

### Options for enforcement

**B1 — Stop emitting one of the two variants at all four sites.** Rejected on
three counts: it breaks ~6 tests that assert on the internal event stream
(research §4), it destroys the dump's audit record of transitions (the
`old → new` pair is strictly more information than `completed old`), and —
decisively — it does **not** fix the reconciler duplicate, because that is a
duplicate of `TicketPhaseChanged` by an emitter this ticket has no business
rewriting.

**B2 — Projection-only: `TicketPhaseChanged => None`.** Necessary but not
sufficient. It makes the pair one line and silences the reconciler for free,
but if two *detectors* both ran the full advance for one transition, each would
emit its own `PhaseCompleted` and mint its own line. The ticket names that
fixture explicitly.

**B3 — Dedup at render time** (collapse adjacent identical lines in
`render_activity_log`). Rejected: the story's honest boundary states the UI
projection stays pure — no render-time rescan. That boundary was written for
T-052-02-02 and applies here for the same reason.

**B4 — Projection narrowing + an emit-layer transition choke point. ✅ CHOSEN**

Two changes that address different layers of the same defect:

**B4a — one variant may mint a feed line.** In
`activity_event_to_ui_entry`:

- `PhaseCompleted` → `ui::ActivityType::PhaseCompleted` *(unchanged — the one
  line)*
- `TicketPhaseChanged` → `None` *(was: a second "completed {new}" line)*
- `ThreadExited` → `None` *(was: a latent "completed Done"; nothing emits this
  variant anywhere in the workspace)*

Both events stay in the ring and stay in the dump through
`format_activity_event`. This is the established demotion mechanism —
`PluginStarted`, `TicketStatusChanged`, and `DagRecomputed` already return
`None` here.

After B4a the projection is *structurally* incapable of minting two lines from
one emit-pair: only one of the pair's two variants has a feed shape at all.

**B4b — one choke point for transition emits.** A new method:

```rust
fn log_phase_transition(&mut self, ticket_id: &str, from: Phase, to: Phase)
```

It emits the `PhaseCompleted{from}` + `TicketPhaseChanged{from → to}` pair
exactly as the four detector sites do today — *unless* `logged_transitions`
already records `(from, to)` as the last transition logged for this ticket, in
which case it emits nothing and returns.

`logged_transitions: HashMap<TicketId, (Phase, Phase)>` on `State`. Last
transition only, not a set: a ticket that is reset and re-run legitimately
repeats `Research → Design`, and a set would swallow the second, real
occurrence. Storing only the previous transition suppresses the double-observe
(same transition, same tick, two detectors) while letting a genuine repeat
through after any intervening transition.

`reset_ticket` (lib.rs:8390) clears the ticket's entry, so a retry's first
transition is never mistaken for an echo of the pre-reset run.

**Why both halves.** B4a fixes "one emit becomes two lines." B4b fixes "one
transition becomes two emits." Neither alone satisfies the acceptance criteria;
the AC's three-path-plus-double-observe fixture set is exactly the test matrix
for the two halves together.

### The reconciler stays as it is

`rebuild_dag` keeps emitting a bare `TicketPhaseChanged` for phases it observes
changing on disk (lib.rs:3394). It is not routed through `log_phase_transition`,
deliberately:

- After B4a it costs **zero** feed lines, so its same-tick duplicate of a
  detector transition is already solved.
- It is a reconciler, not a detector. Routing it through the choke point would
  make it emit `PhaseCompleted` — a "completed" claim — for file states it
  merely *observed*, including two error paths (lib.rs:3064, 3080) that rebuild
  the DAG precisely because a completion could **not** be verified. Announcing
  "T-001 completed Review" from those branches would be the P2 lie this epic
  exists to remove.

So its role narrows honestly: an audit record in the ring and the dump, not a
headline.

### One collateral fix

`reset_ticket` is the only site whose sole feed-visible signal was
`TicketPhaseChanged`, and it rendered as `"T-001 completed Ready"` — a ticket
that was reset for retry has completed nothing. B4a would silence it. It gets a
plain `Info` instead:

> `T-001 reset to Ready for retry`

Net feed lines for that path: one before, one after. The sentence stops lying.

---

## What one transition costs after this ticket

| Path | Ring entries | Feed lines |
|---|---|---|
| Artifact detector advances R→D | 3 (pair + reconciler echo) | **1** |
| Idle detector advances R→D | 3 | **1** |
| Both detectors observe R→D | 3 (second pair suppressed at emit) | **1** |
| Completion advances Review→Done | 3 + success `Info` | **1** + the Info |
| Scheduling pass, all candidates live | **0** | **0** |
| Two tickets complete the same phase | 6 | **2** — distinct, never merged |

The last row is the negative fixture: the dedup key is
`(ticket_id, from, to)`, so nothing can merge across tickets. It is the reason
the map is keyed by ticket rather than by transition shape.

## Out of scope

- Folding recurring events with an `(x3)` tag — that is T-052-02-02, and it
  edits `log_activity_at`, a function this ticket does not touch.
- `AllTicketsDone`'s `"all completed Done"` projection (lib.rs:9140). Odd copy,
  but it is one line for one real fact and no acceptance criterion reaches it.
- Any change to which sections the Operations view displays (story: out of
  slice).
- Severity levels, filtering UI, config surface (N4).
