# T-020-01 Research — AskUserQuestion attention detection + awaiting-human suppression

Spike. Maps the existing hook, signal, injection, and notification machinery that a
"detect AskUserQuestion → notify + suppress auto-injection" feature would build on.
Descriptive only — options and the go/no-go decision live in `design.md`.

Line anchors are against the **current working tree** (S-019 / T-019-01..03 changes
are present but uncommitted), so they drift slightly from the numbers quoted in the
ticket, which were taken before those edits landed.

## 1. The hook system (shell → signal files → plugin)

Direction is one-way: **Claude Code lifecycle events run POSIX `sh` scripts in
`.lisa/hooks/`, which write timestamped signal files into `.lisa/signals/pane-<id>.<ext>`;
the WASM plugin reads and deletes them.** The plugin never writes signal files
(`data/hooks-guide.md:9-20`).

Five scripts are scaffolded by `lisa init` (`init.rs:321-333`) and bound to Claude Code
events in `.claude/settings.local.json` (`templates.rs:116-173`, `merge_hooks`
`templates.rs:255-299`):

| Script            | Claude event              | Signal file        | Const in templates.rs |
|-------------------|---------------------------|--------------------|-----------------------|
| `on-idle.sh`      | `Notification[idle_prompt]` | `pane-<id>.idle`   | `ON_IDLE_HOOK:14`     |
| `on-stop.sh`      | `Stop`                    | `pane-<id>.stopped`| `ON_STOP_HOOK:28`     |
| `on-clear.sh`     | `SessionStart[clear]`     | `pane-<id>.cleared`| `ON_CLEAR_HOOK:42`    |
| `on-heartbeat.sh` | `PostToolUse`             | `pane-<id>.heartbeat`| `ON_HEARTBEAT_HOOK:58`|
| `on-notify(.sample)` | user-owned, called by lisa | (none — it notifies) | `ON_NOTIFY_HOOK:78` |

Each `.sh` is inert unless `$LISA_PANE_ID` is set, which lisa exports when it spawns the
session (`build_claude_command`, `lib.rs:53-60`: `LISA_PANE_ID=<n> LISA_TICKET_ID=<id>
claude --dangerously-skip-permissions "<prompt>"`).

**Crucial gap for this ticket: lisa binds NO `PreToolUse` hook today.** The only
tool-call-level hook is `PostToolUse` (heartbeat). `merge_hooks` wires exactly Stop,
SessionStart[clear], Notification[idle_prompt], PostToolUse, and a matcher-less
Notification catch-all — nothing on `PreToolUse` (`templates.rs:268-296`).

## 2. The existing `Notification` catch-all (the closest precedent)

S-019 (T-019-02) already added a **matcher-less `Notification`** entry —
`NOTIFY_ATTENTION_COMMAND` (`templates.rs:110`):

```
test -x .lisa/hooks/on-notify || exit 0; in=$(cat); \
case "$in" in *idle_prompt*) : ;; \
*) LISA_EVENT=attention LISA_REASON=permission .lisa/hooks/on-notify attention "$in" ;; esac
```

This is the template for what a new `PreToolUse[AskUserQuestion]` hook command would look
like: POSIX `sh` only (no `jq`, no bashisms), reads stdin once with `in=$(cat)`, guards on
`test -x .lisa/hooks/on-notify`, dispatches via `case`, and passes the raw payload as `$2`.
It currently fires `LISA_REASON=permission` for any non-`idle_prompt` Notification —
which is how permission prompts already reach the human. `ensure_hook` dedups a
matcher-less entry by command substring (`templates.rs:200-216`), so a *second*
matcher-less binding on a different event (PreToolUse) is independent and idempotent.

## 3. The `on-notify` contract (S-019, reused verbatim by this spike)

The user-owned `on-notify` hook is invoked two ways (`hooks-guide.md:79-89`):

1. **From the plugin** via Zellij `run_command`: `fire_notify` (`lib.rs:330-345`) builds
   argv+env with `build_notify_command` (`lib.rs:289-323`) — `sh -c 'if [ -x "$LISA_HOOK" ];
   then "$LISA_HOOK" "$1" "$2"; fi' sh <event> <detail>` with `LISA_EVENT`/`LISA_PROJECT`/
   extras in env. Used for `complete` (`lib.rs:1674`) and `attention`
   `LISA_REASON=idle-without-artifact` (`lib.rs:993`).
2. **From Claude Code's Notification catch-all** (§2) for `LISA_REASON=permission`.

Contract surface (`hooks-guide.md:46-77`): `on-notify <event> [detail]`; env always carries
`LISA_EVENT` ∈ {`complete`,`attention`} and `LISA_PROJECT`; `attention` adds
`LISA_PANE_ID`, `LISA_TICKET`, `LISA_REASON` ∈ {`idle-without-artifact`,`permission`}.
A new reason value (e.g. `question`) slots into this without a new user hook — the spike's
explicit constraint ("reuses the S-019 on-notify contract, no new user hook").

`RunCommandResult` is attributed back via a `lisa_notify` context key
(`lib.rs:337-344`, handler `lib.rs:2532-2539`).

## 4. The injection mechanism and its callers (what suppression must guard)

All text injection into agent panes goes through one method:
`send_line_to_pane(text, pane_id)` (`lib.rs:268-273`) — writes chars immediately, queues a
deferred Enter (`ENTER_DELAY_SECS = 2.0`). Six call sites, five logical injection points
(the ticket's Q5 list):

| # | Caller (`lib.rs`)              | What it injects                        |
|---|-------------------------------|----------------------------------------|
| 1 | `schedule_ready_tickets:550`  | `/clear` (session reuse)               |
| 2 | `schedule_ready_tickets:559`  | full `claude …` launch command         |
| 3 | `handle_stopped_signal:1071`  | `/clear` after `.stopped`              |
| 4 | `handle_cleared_signal:1186`  | ticket prompt after `.cleared`         |
| 5 | `check_transition_timeouts:1245` | `/clear` (stop-timeout fallback)    |
| 5 | `check_transition_timeouts:1262` | ticket prompt (clear-timeout fallback)|
| 6 | `check_review_timeouts:1306`  | finish-up prompt to a stuck Review     |

Callers #2/#4 inject into a pane that should be at a fresh prompt; #1/#3/#5 send `/clear`;
#6 prods a parked Review session. **The danger this ticket addresses:** if an agent is
blocked on an `AskUserQuestion`, the pane is quiet, and any of these (most acutely #6's
finish-up prompt, or a `/clear` from reuse) types over the pending question.

## 5. Why a quiet pane is currently unprotected — the liveness model

Liveness is **heartbeat-based** (project memory `liveness-heartbeat-design`, v0.2.11).
`check_heartbeat_signals` (`lib.rs:760-785`) consumes `.heartbeat` files (one per
`PostToolUse`), calls `bump_pane_activity` (`lib.rs:740-752`) to refresh
`slot.last_activity_at` + `thread.last_activity`, and clears the `notified_attention`
debounce for that pane. Every gate that injects or reclaims keys off *heartbeat silence*
(`wind_down_secs`, `stuck_threshold_secs`).

**The hole:** an `AskUserQuestion` call emits no further tool calls while it waits, so no
`PostToolUse` / heartbeat fires. To every silence-based gate the pane looks like a winding-
down or stalled session — exactly the state that *invites* injection/reclamation. Heartbeat
silence cannot distinguish "blocked on a human" from "finished / dead." (Story
`S-020:22-33`.)

## 6. The Implement-phase blind spot

`check_idle_signals` (`lib.rs:798-1004`) treats an `.idle` signal differently per phase:

- **Implement** (`lib.rs:852-916`): idle **alone** = completion → advances Implement→Review
  (and Review→Done if `review.md` already exists). No artifact required.
- **Research/Design/Structure/Plan/Review** (`lib.rs:918-997`): idle **+ artifact**
  advances; idle **without** artifact pushes an `idle_alerts` entry and fires
  `attention`/`idle-without-artifact`, debounced via `notified_attention`
  (`lib.rs:982-995`).

So a clarifying question that surfaces as idle is caught heuristically in most phases
(idle-without-artifact) but in **Implement** is read as "done" (`lib.rs:853`) — the agent's
question is missed and the ticket wrongly advances. This is the precise case the
`AskUserQuestion` signal would close (story `S-020:9-20`).

## 7. The timeout / reclamation paths (Q6 surface)

Four periodic checks in `poll_tick` (`lib.rs:1548-1577`) can act on a quiet pane:

- `check_transition_timeouts` (`lib.rs:1207-1269`): force-advances a stalled
  stop/clear handshake **only once `quiet` for `wind_down_secs`** — then injects `/clear`
  or a prompt (#5 above).
- `check_review_timeouts` (`lib.rs:1277-1316`): after `review_timeout_secs` in Review with
  no `review.md`, and `wind_down` silence, injects the finish-up prompt (#6). Tracked by
  `finish_up_sent`.
- `check_session_timeouts` (`lib.rs:1385-1468`): global/per-phase budget. Reclaims
  (fails thread, releases slot) **only at hard silence = `2×stuck_threshold_secs`**;
  otherwise just warns (`over_budget_warned`). Does not inject.
- `detect_stale_threads` (`lib.rs:1477-1506`): fails + releases at `2×stuck_threshold_secs`
  of total silence. Does not inject.

All four are silence-gated, so an awaiting-human pane (silent by nature) is eligible for
both injection (transition/review) and reclamation (session/stale) once enough wall-clock
passes. Q6 is whether awaiting panes should be exempted from reclamation.

## 8. State + per-pane bookkeeping available to a flag

`State` (`lib.rs:155-245`) already carries per-pane sets keyed by `u32` pane id:
`notified_attention: HashSet<u32>` (`lib.rs:241`) is the closest analog to a new
`awaiting_human: HashSet<u32>` flag — set on a signal, cleared on heartbeat. `agent_slots`
(`AgentSlot`, `lib.rs:95-113`) hold `pane_id`, `ticket_id`, `transition_state`,
`last_activity_at`. `#[derive(Default)]` on `State` means a new `HashSet` field needs no
init code (observation 23137). The heartbeat consumer (`lib.rs:783`) is the natural place
to clear such a flag — it already clears `notified_attention` there.

## 9. Constraints & assumptions

- **WASI sandbox:** plugin reads host files under `/host`; `signal_dir = /host/.lisa/signals`
  (`lib.rs:2414`). `run_command` runs on the host, so `project_root` is the real cwd
  (`lib.rs:2419`). A new signal file (e.g. `pane-<id>.awaiting`) fits the existing
  `read_dir(signal_dir)` scan pattern used by every `check_*_signals`.
- **POSIX `sh` only** on the hook side (no `jq`, no bashisms) — established by
  `NOTIFY_ATTENTION_COMMAND` and all `.sh` scripts.
- **Don't destabilize heartbeat liveness** (memory `liveness-heartbeat-design`): a flag
  must not fake activity or it would defeat stall detection for genuinely-dead panes.
- **Open behavioral unknowns (answered in `design.md`, evidence-gathered via
  claude-code-guide):** (Q1) does `AskUserQuestion` fire `PreToolUse` and the exact
  matcher string; (Q2 GATE) do `--dangerously-skip-permissions` agents ever invoke
  `AskUserQuestion`; (Q3) is the question text extractable from the `PreToolUse` stdin
  payload in POSIX `sh`; (Q4) does a `PostToolUse` heartbeat fire after the answer so the
  flag self-clears on the existing path.
