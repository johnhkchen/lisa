# T-020-03 Design — awaiting-human suppression

Decisions grounded in `research.md`. The ticket prescribes the shape heavily (it
came from the T-020-01 spike Q5 table); this doc records the viable alternatives
considered and why the prescribed path wins.

## Decision summary

Mirror the `notified_attention` lifecycle with a new `awaiting_human: HashSet<u32>`,
consume `pane-<id>.awaiting` in a new `check_awaiting_signals()` called before
`check_idle_signals`, clear it on the heartbeat path, and gate injection with a
**belt-and-suspenders** pair: an in-method guard inside `send_line_to_pane` (fails
safe — no clobber even if a caller is missed) plus per-caller early-returns (keeps
slot/transition state coherent).

## D1 — Where does the flag live?

**Options:**
- (a) `HashSet<u32>` field on `State`, keyed by pane id. *(chosen)*
- (b) A bool on `AgentSlot`.
- (c) A new `TransitionState` variant `AwaitingHuman`.

**Decision: (a).** It is the exact pattern `notified_attention` already uses
(`lib.rs:241`), so it inherits a proven, tested lifecycle and `#[derive(Default)]`
needs no `load()` change. (b) spreads the concept across slot bookkeeping and
forces every read site to resolve pane→slot first. (c) is wrong: the agent is *not*
mid-transition — it is blocked inside a live phase; overloading the transition FSM
would entangle awaiting with `WaitingForStop`/`WaitingForClear` and corrupt those
timeouts. The flag is orthogonal to transition state, so it gets its own field.

## D2 — Set path: new reader vs. piggyback on an existing scan

**Options:**
- (a) Dedicated `check_awaiting_signals()` modeled on `check_heartbeat_signals`. *(chosen)*
- (b) Fold `.awaiting` parsing into `check_heartbeat_signals` or `check_idle_signals`.

**Decision: (a).** `check_heartbeat_signals` (`lib.rs:760`) is a tight, single-kind
scanner — read dir, match `pane-N.<suffix>`, delete, mutate state. A parallel
function for `.awaiting` is symmetric, independently testable, and keeps the
ordering explicit in `poll_tick`. Folding into an existing scanner (b) couples two
behaviors with different clear semantics (heartbeat *clears* awaiting; the awaiting
scan *sets* it) and would muddy the heartbeat function's single responsibility.

## D3 — Clear path: what un-sets the flag?

**Options:**
- (a) Clear in `check_heartbeat_signals` beside the `notified_attention.remove`
  (`lib.rs:783`). *(chosen)*
- (b) Clear via a dedicated PostToolUse `.resumed` signal for AskUserQuestion.
- (c) Clear when `.idle` or `.stopped`/`.cleared` is seen.

**Decision: (a).** The heartbeat is the universal "agent made a real tool call"
proof. After a human answers an AskUserQuestion, the agent's very next action is a
tool call → PostToolUse → `.heartbeat`. Clearing there means awaiting auto-resolves
on genuine resumption regardless of whether AskUserQuestion emits its own
PostToolUse (the spike Q4 unknown). (b) invents a new hook and re-introduces the
exact dependency on AskUserQuestion's PostToolUse we are trying to avoid. (c) is
unreliable — a question can be answered without the session going idle or stopping,
and `.idle` while awaiting would mean the *question itself* never cleared.

This single line at `lib.rs:783` is the keystone: it makes the whole feature robust
to Q4 (see `research.md` "notified_attention pattern").

## D4 — Injection guard: in-method, per-caller, or both?

**Options:**
- (a) Only guard inside `send_line_to_pane` (drop the write).
- (b) Only early-return at each of the five callers.
- (c) **Both** — in-method drop + per-caller returns. *(chosen)*

**Decision: (c), belt-and-suspenders.** The two guards cover different failure
modes and neither alone is sufficient:

- The **in-method** guard is the safety net: if a future caller is added or one is
  missed, the write is still dropped → the question is never clobbered. But it
  cannot, by itself, stop a caller from *advancing its state machine* (flipping
  `transition_state`, inserting into `finish_up_sent`, marking a phase change). If
  only (a), a caller would think it sent input and mutate state as if it had —
  leaving the FSM desynchronized from reality.
- The **per-caller** returns keep slot/transition state coherent: the caller skips
  this pane *this tick*, leaving it in a re-tryable state so that once the flag
  clears (human answered → heartbeat), the normal path resumes cleanly.

Per the ticket, callers #1/#2 (`schedule_ready_tickets`) only ever target idle
slots, so their guard is defensive, not load-bearing; #3/#4/#5 are the real ones.

**Guard semantics per caller (from the Q5 table):**

| Caller | Inject | Action when awaiting |
|---|---|---|
| `schedule_ready_tickets:550/559` | `/clear` / launch | skip slot this tick (stays assigned, retried) |
| `handle_stopped_signal:1071` | `/clear` | return early |
| `handle_cleared_signal:1186` | prompt | return early |
| `check_transition_timeouts:1245/1262` | `/clear` / prompt | skip pane in fallback loop |
| `check_review_timeouts:1306` | finish-up prompt | skip candidate |

## D5 — Does the guard touch the liveness clock? **No.**

This is a hard constraint (`research.md` "Liveness model", obs 13214). The guard
**only suppresses writes**; it never calls `bump_pane_activity`, never sets
`last_activity_at`. A question-blocked pane and a dead pane look identical on the
silence clock, and that is correct: lisa must not be tricked into thinking a
blocked-and-then-abandoned pane is alive. Timeout/reclaim exemption is deliberately
T-020-04's job. If we faked activity here we would both (i) destabilize the
v0.2.11 heartbeat model and (ii) prevent stale-reclaim of a truly dead pane.

## D6 — Extracting the pane id inside `send_line_to_pane`

`send_line_to_pane` receives `PaneId` (enum). The guard matches
`PaneId::Terminal(id)` and checks `self.awaiting_human.contains(&id)`. For any other
variant (none occur today — all callers pass `Terminal`) the guard is a no-op and
the write proceeds, which is safe. Logging the drop uses the existing
`log_activity(ActivityEvent::Info{..})` already available on `&mut self`.

## What is explicitly NOT in scope

- Timeout / stale-reclaim exemption for awaiting panes → **T-020-04**.
- Any change to the writer hook or templates (done in T-020-02).
- Surfacing "awaiting human" in the dashboard UI (not required by AC; could be a
  follow-up, noted in review).
