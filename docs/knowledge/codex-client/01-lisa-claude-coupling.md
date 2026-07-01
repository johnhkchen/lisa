# 01 · How lisa couples to Claude Code (today)

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 from a multi-agent read of the repo. All claims carry `file:line` anchors.

This is the surface any second agent client must satisfy. Each section is one mechanism, extracted directly from the source.

---

## How lisa launches a Claude Code agent and injects prompt/keystrokes into a pane

All code is in `crates/lisa-plugin/src/lib.rs`. Lisa is a Zellij WASM plugin; it never spawns `claude` as a subprocess itself. Instead it discovers pre-created terminal panes from the Zellij layout and writes shell text + an Enter keypress into their stdin, driving whatever shell is sitting at the prompt in each pane.

## 1. Pane model: pre-created slots, not opened by lisa
Agent panes are terminals declared in the Zellij layout. Lisa discovers them in `discover_slots()` from a `PaneManifest`, treating every non-plugin pane as an `AgentSlot` (lib.rs:430-457):
```
if !pane.is_plugin { self.agent_slots.push(AgentSlot { pane_id: pane.id, ... }) }
```
Each slot tracks `pane_id`, an optional assigned `ticket_id`, and `has_session` (whether a `claude` process has already been launched in it). So "launching an agent" == writing a shell command line into an idle terminal pane whose shell is at its prompt.

## 2. The launch command string template
`build_claude_command` (lib.rs:53-60) builds the exact shell line that starts Claude Code in a fresh pane:
```rust
format!(
    "LISA_PANE_ID={} LISA_TICKET_ID={} claude --dangerously-skip-permissions \"{}\"",
    pane_id,
    ticket_id,
    ticket_prompt(ticket_dir, ticket_id)
)
```
Breakdown of the template:
- `LISA_PANE_ID=<pane_id>` and `LISA_TICKET_ID=<ticket_id>` are set as inline shell env-var assignments prefixed on the command (per-process env for the `claude` invocation). `LISA_PANE_ID` lets the idle/stop signal hooks identify which pane emitted a signal; `LISA_TICKET_ID` is for debugging/logging context (doc comment lib.rs:50-52).
- Binary: `claude`
- Flag: `--dangerously-skip-permissions` (the only CLI flag passed)
- Final positional arg: the prompt string, wrapped in escaped double quotes `"..."`, so the whole prompt is passed as a single initial-prompt argv to `claude`.

Note the prompt is interpolated raw inside `"..."` with no shell-escaping of characters inside the prompt; the prompt text (below) contains no double quotes or `$`, so it is safe as written, but there is no escaping layer.

## 3. How the prompt text is built
`ticket_prompt` (lib.rs:34-48) builds the initial prompt. It computes the ticket path as `ticket_dir.join("<ticket_id>.md")` and formats a multi-line instruction (joined with `\` line continuations into one logical string) telling the agent to: read the ticket + `CLAUDE.md` + `docs/knowledge/rdspi-workflow.md`, start from the current phase and run through ALL remaining RDSPI phases (Research, Design, Structure, Plan, Implement, Review) without stopping, write each phase artifact to `docs/active/work/<ticket_id>/`, NOT touch the ticket's phase/status frontmatter (lisa detects artifacts and drives transitions), and simply stop after `review.md` is written.

`finish_up_prompt` (lib.rs:63-72) is a separate prompt injected into a Review session that has stalled past the review timeout. It points at `work_dir/<ticket_id>/review.md` and asks the agent to finish that artifact (changes made, files touched, test coverage, open concerns, critical issues), again reminding it not to edit frontmatter. Sent from `check_review_timeouts` at lib.rs:1402-1403.

## 4. Path handling: strip the /host/ prefix
Inside Zellij's WASI sandbox the host filesystem is mounted at `/host/`, but commands typed into panes execute on the host, so paths must be host-relative. `strip_host_prefix` (lib.rs:89-92) strips the `/host/` prefix:
```rust
PathBuf::from(s.strip_prefix("/host/").unwrap_or(&s).to_string())
```
Callers do `let host_ticket_dir = strip_host_prefix(&self.config.ticket_dir);` before building any prompt/command (e.g. lib.rs:555, 1266, 1351).

## 5. How text is typed vs. how Enter is pressed (the two-step injection)
Injection goes through `send_line_to_pane` (lib.rs:276-293):
```rust
write_chars_to_pane_id(text, pane_id);      // types the characters immediately
self.pending_enters.push_back(pane_id);     // queue a deferred Enter for this pane
set_timeout(ENTER_DELAY_SECS);              // arm a timer
self.pending_timer_count += 1;
```
So the command/prompt characters are written into the pane's stdin at once via `write_chars_to_pane_id`. The Enter keypress is NOT sent in the same call. Instead the pane is pushed onto `pending_enters` and a timer is armed.

The deferred Enter fires from the `Event::Timer` handler in `update()` (lib.rs:2632-2642), which calls `flush_pending_enters()` before polling. `flush_pending_enters` (lib.rs:302-306) drains the queue and sends a raw carriage return to each pane:
```rust
while let Some(pane_id) = self.pending_enters.pop_front() {
    write_to_pane_id(vec![13], pane_id); // Enter key (0x0D)
}
```
So: characters via `write_chars_to_pane_id`, then after a delay a literal byte `13` (CR / 0x0D) via `write_to_pane_id` to submit.

## 6. Timing and why (TUI-specific assumption)
`ENTER_DELAY_SECS = 2.0` (lib.rs:83). The rationale (doc comment lib.rs:74-82): Claude Code's TUI needs a full event-loop tick to process typed characters and commit them into its input field before Enter can trigger "submit". If the text write and the CR write are issued back-to-back, the two `write_to_pane_id` calls can coalesce in the PTY buffer so the TUI reads text+CR as one chunk — Enter then fires before the input state is committed, inserting a newline instead of submitting. A 2-second gap is imperceptible to a human but gives the TUI ample time. This is why every injection is a two-phase (type, wait, Enter) operation gated on the timer.

## 7. Two launch paths: fresh pane vs. session reuse
In `schedule_ready_tickets` (lib.rs:566-586):
- Fresh pane (`has_session == false`): builds the full shell command with `build_claude_command(&host_ticket_dir, &ticket_id, pane_id)` and `send_line_to_pane`s it, then sets `has_session = true`. This actually starts the `claude` process (lib.rs:581-585).
- Session reuse (`has_session == true`, slot idle): the pane already has a running Claude Code at its prompt. Lisa sends `/clear` via `send_line_to_pane("/clear", ...)`, sets `transition_state = WaitingForClear`, and stashes the next `ticket_prompt` to send after the `.cleared` signal (lib.rs:569-579). The prompt is then injected in `handle_cleared_signal` (lib.rs:1246-1278) with `send_line_to_pane(&prompt, ...)`, or via the timeout fallback in `check_transition_timeouts` (lib.rs:1339-1354) if the `.cleared` signal never arrives.

So reuse injects `/clear` (a slash command to the existing TUI) rather than a shell command — same two-step type+Enter mechanism.

## 8. Env-var wiring recap
`LISA_PANE_ID` and `LISA_TICKET_ID` are set ONLY on the fresh-pane launch, as inline env assignments prefixed on the `claude` command line in `build_claude_command` (lib.rs:55). They are per-invocation shell env vars, not exported by lisa's process. Session-reuse and `/clear`/prompt re-injection paths do not re-set them (the original `claude` process keeps its environment). Note `LISA_PANE_ID` is the Zellij pane id (`AgentSlot.pane_id`, a `u32`), the same id lisa writes to.

## 9. Permissions / plumbing
`load()` (lib.rs:2512-2554) subscribes to `PaneUpdate`, `PermissionRequestResult`, `Timer`, `Key`, `RunCommandResult`, and requests `WriteToStdin` (needed for `write_chars_to_pane_id`/`write_to_pane_id`), `ChangeApplicationState`, `ReadApplicationState`, and `RunCommands`. Scheduling only proceeds once `permissions_granted` and `slots_discovered` are true (lib.rs:507). Separately, the `on-notify` hook is the only thing lisa runs as a real host subprocess, via `run_command_with_env_variables_and_cwd` (lib.rs:361) — that is a hook invocation, not the agent launch.

## Safety guards on injection
`send_line_to_pane` refuses to write into a pane that is blocked on an `AskUserQuestion` (its `pane-<id>.awaiting` signal was seen): if `is_pane_awaiting(id)` it logs and returns before writing chars or queuing Enter, so no stray Enter is left (lib.rs:281-288). Per-caller guards repeat this check (e.g. lib.rs:562-565, 1263-1265, 1323, 1341).

## Verifying tests
`test_build_claude_command` (lib.rs:3144), `test_build_claude_command_includes_env_vars` (lib.rs:3160), and `test_build_claude_command_includes_rdspi_reference` (lib.rs:3172) assert the command template shape, the `LISA_PANE_ID`/`LISA_TICKET_ID` presence, and the rdspi-workflow reference.

**Anchors:** /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:34-48, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:50-60, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:63-72, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:74-92, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:276-306, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:430-457, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:566-586, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1246-1278, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1320-1360, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1402-1403, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:2512-2554, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:2632-2642, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:3144-3174

---

## Lisa session-reuse / context-reset handshake (the /clear flow)

## Overview

Lisa keeps a fixed set of pre-created terminal panes (`AgentSlot`s) in a stacked Zellij layout. When a ticket finishes, lisa does **not** kill the Claude Code process in the pane. Instead it keeps `has_session = true`, waits for the pane to genuinely go quiet, then reuses the running process by sending the literal slash command `/clear` (to wipe conversation context) and re-typing a fresh ticket prompt. Relaunching (`claude --dangerously-skip-permissions "<prompt>"` from the shell) happens only for a pane that has never hosted a session (`has_session == false`).

The reuse is gated by a small per-slot state machine driven by **hook-generated signal files**, not blind timers. Timers exist only as fallbacks.

## The state machine: `TransitionState`

Defined at `crates/lisa-plugin/src/lib.rs:128-137`:

```rust
enum TransitionState {
    #[default]
    Idle,             // No transition pending — slot idle or running normally.
    WaitingForStop,   // Phase complete, waiting for `.stopped` before sending `/clear`.
    WaitingForClear,  // `/clear` sent, waiting for `.cleared` before sending the prompt.
}
```

Doc comment (`lib.rs:125-127`): "Gates `/clear` and prompt sends on hook-generated signal files (`.stopped` and `.cleared`) instead of blind timers."

Per-slot fields on `AgentSlot` (`lib.rs:94-113`):
- `has_session: bool` — whether a Claude Code session was ever started in this pane. Drives the relaunch-vs-reuse branch.
- `transition_state: TransitionState`
- `transition_started_at: Option<SystemTime>` — start of the current transition, for timeout fallbacks.
- `cooldown_until: Option<SystemTime>` — earliest time the slot can take new work after completion.
- `last_activity_at: Option<SystemTime>` — last sign of life (heartbeat/stop/idle/cleared signal, or lisa-sent input). Used for the "quiet"/wind-down guard.

## The signal files (produced by Claude Code hooks)

From `crates/lisa-cli/src/templates.rs` and `data/hooks-guide.md:29-30`. Each hook writes a UTC timestamp into `.lisa/signals/pane-<LISA_PANE_ID>.<ext>`:
- `on-stop.sh` — Claude Code `Stop` event → `pane-<id>.stopped` ("session ready for input") — `templates.rs:28-38`.
- `on-clear.sh` — `SessionStart[clear]` event → `pane-<id>.cleared` ("context was cleared") — `templates.rs:42-52`.
- `on-idle.sh` → `.idle`; `on-heartbeat.sh` (PostToolUse, fires after every tool call) → `.heartbeat`. Heartbeats are the real "still working" proof; stop/idle are explicitly distrusted because "agents often report stopped and then keep working for another minute or two" (`lib.rs:107-112`, `templates.rs:55-57`).

The literal slash command sent into the pane is `"/clear"` (e.g. `lib.rs:575`, `1148`, `1332`).

## Full flow (states, triggers, actions)

The two entry points differ. This is the key subtlety that fixed a past deadlock.

### Entry A — scheduling a released, still-running pane (`schedule_ready_tickets`, `lib.rs:566-590`)

When a ready ticket is placed into an idle slot whose `has_session == true`, lisa **skips `WaitingForStop` entirely** and sends `/clear` immediately, jumping straight to `WaitingForClear`:

```rust
if self.agent_slots[slot_idx].has_session {
    // Session reuse: the slot is idle (ticket_id was None), so
    // Claude Code is already at its prompt. Send /clear directly
    // and wait for the .cleared signal before sending the prompt.
    // (The old WaitingForStop approach deadlocked because the
    // previous session's .stopped signal was already consumed by
    // check_transition_signals() earlier in the same poll_tick.)
    self.send_line_to_pane("/clear", PaneId::Terminal(pane_id));
    self.agent_slots[slot_idx].transition_state = TransitionState::WaitingForClear;
    self.agent_slots[slot_idx].transition_started_at = Some(SystemTime::now());
    launch_cmd = ticket_prompt(&host_ticket_dir, &ticket_id);
} else {
    // Fresh pane — launch Claude Code from the shell.
    ... send_line_to_pane(&cmd, ...); self.agent_slots[slot_idx].has_session = true;
}
```
(`lib.rs:568-586`). A slot only becomes eligible for this via `find_idle_slot` (`lib.rs:466-476`), which requires `ticket_id.is_none()`, cooldown elapsed, and — if `has_session` — that `last_activity_at` is at least `wind_down_secs` old (the busy-pane guard).

### Entry B — the `WaitingForStop` path (`handle_stopped_signal`, `lib.rs:1129-1157`)

`WaitingForStop` is still reachable by other callers (it is the "phase complete, wait for stop, then clear" path). When a `.stopped` signal arrives for a slot in `WaitingForStop`, `handle_stopped_signal` sends `/clear` and advances to `WaitingForClear` (`lib.rs:1142-1156`). If the slot is `Idle` and its ticket is in Review phase, the same `.stopped` instead triggers `auto_complete_review` (Review → Done, `lib.rs:1159-1178`).

### Signal-driven advance every poll (`check_transition_signals`, `lib.rs:1085-1122`)

Scans `.lisa/signals`, and for each file: reads/deletes it, calls `bump_pane_activity(pane_id)` (restarting the wind-down clock — `lib.rs:1106-1108`), then:
- `.stopped` → `handle_stopped_signal` (WaitingForStop → send `/clear` → WaitingForClear).
- `.cleared` → `handle_cleared_signal`.

Doc (`lib.rs:1081-1082`): "`.stopped` → if slot is `WaitingForStop`, send `/clear` and move to `WaitingForClear`; `.cleared` → if slot is `WaitingForClear`, send the prompt and move to `Idle`."

### Completing reuse (`handle_cleared_signal`, `lib.rs:1246-1279`)

On `.cleared` for a slot in `WaitingForClear`: type the new ticket prompt into the pane and reset to `Idle` (`transition_started_at = None`). If the pane is awaiting a human question, it does nothing and stays `WaitingForClear` for a later tick.

## Timeout fallbacks (`check_transition_timeouts`, `lib.rs:1289-1360`)

Constants (`lib.rs:22-31`):
- `STOP_SIGNAL_TIMEOUT_SECS = 60`
- `CLEAR_SIGNAL_TIMEOUT_SECS = 90`

A fallback fires only when **both** `elapsed > timeout` AND the pane has been "quiet" — `last_activity_at` older than `wind_down_secs` (default `DEFAULT_WIND_DOWN_SECS = 300`, `lisa-core/src/types.rs:540`). This "quiet" AND-condition means "the prompt is never injected into a session that is still working" (`lib.rs:27-31`, `1285-1288`, `1300-1302`):
- `WaitingForStop` timed out + quiet → force-send `/clear`, advance to `WaitingForClear` (`lib.rs:1320-1337`; warning "Stop signal timeout for pane {}, sending /clear anyway").
- `WaitingForClear` timed out + quiet → force-send the prompt, reset to `Idle` (`lib.rs:1339-1359`; warning "Clear signal timeout for pane {}, sending prompt anyway").

## Ordering constraints & past deadlocks (from the comments)

1. **The `WaitingForStop` deadlock (the main one).** `poll_tick` runs `check_transition_signals()` (`lib.rs:1676`) before `schedule_ready_tickets` gets to reuse the pane. The previous session's `.stopped` file is consumed and deleted early in the tick. If reuse then set the slot to `WaitingForStop`, it would wait for a `.stopped` signal that was already consumed and will never re-fire — a permanent stall. Fix: on reuse the code sends `/clear` immediately and goes straight to `WaitingForClear` (comment at `lib.rs:572-574`).

2. **Signal ordering within a tick** (`poll_tick`, `lib.rs:1659-1693`): the fixed sequence is `check_heartbeat_signals` → `check_awaiting_signals` → `check_artifact_advances` → `check_idle_signals` → `check_transition_signals` → `check_transition_timeouts` → ... Heartbeats are consumed first "so activity clocks are current before any health or timeout decisions" (`lib.rs:1660-1662`). Awaiting/question flags are set before any consumer "can inject into them this tick (must precede check_idle_signals and the timeout fallbacks)" (`lib.rs:1664-1667`).

3. **Question-clobber guards (fail-safe layering).** Every injection point checks `is_pane_awaiting(pane_id)` and skips rather than typing over an `AskUserQuestion` UI: reuse scheduling (`lib.rs:562-565`), `handle_stopped_signal` — "Never /clear a pane blocked on a question — would discard the agent's session mid-question. Stay in WaitingForStop; retry once unblocked" (`lib.rs:1143-1147`), `handle_cleared_signal` (`lib.rs:1261-1265`), both timeout branches (`lib.rs:1321-1325`, `1340-1343`), and a final belt-and-suspenders drop inside `send_line_to_pane` itself (`lib.rs:277-288`).

4. **Enter must be deferred.** `send_line_to_pane` writes characters immediately but queues Enter (0x0D) for `ENTER_DELAY_SECS = 2.0` later (`lib.rs:276-293`, `301-306`). Without the gap the TUI can read text+CR in one PTY chunk and insert a newline instead of submitting (comment `lib.rs:74-83`).

5. **Busy-pane / wind-down guard everywhere.** Both `find_idle_slot` reuse eligibility and the timeout fallbacks require the pane to have been signal-silent for `wind_down_secs`, because stop/idle signals fire before agents truly finish (`lib.rs:107-112`, `461-465`, `1285-1302`).

## Cooldown

On ticket completion, `release_slot_for_ticket` (`lib.rs:481-503`) sets `ticket_id = None`, **keeps `has_session = true`** ("has_session stays true — Claude Code is still running", `lib.rs:487`), and sets `cooldown_until = now + wind_down_secs`. `find_idle_slot` then refuses the slot until `now >= cooldown_until` (`lib.rs:471`), giving the just-finished agent time to actually settle before `/clear` reuse.


**Anchors:** /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:22-31, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:74-83, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:94-137, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:276-306, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:459-503, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:562-590, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:765-777, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1085-1179, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1244-1360, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1659-1693, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:6611-6618, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:26-68, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/data/hooks-guide.md:29-30, /Users/johnchen/swe/repos/lisa/crates/lisa-core/src/types.rs:540

---

## Lisa's signal/hook nervous system: the hook↔scheduler contract

# Lisa signal/hook nervous system

Lisa's agents (Claude Code sessions) communicate with the Zellij WASM plugin (the scheduler) purely through **flat files dropped in `.lisa/signals/`**. Claude Code hook events fire tiny POSIX-`sh` scripts that write `pane-<id>.<kind>` files; the plugin polls that directory every `POLL_INTERVAL_SECS = 5.0` seconds (`lib.rs:20`) and consumes/deletes them. There is no socket, no IPC — just create-file / read-file / delete-file.

## IMPORTANT: on-disk state is stale vs. the code

The checked-in working tree is an **older, simpler generation** than what the current CLI generates. Document the *code* (templates.rs) as the canonical contract, but be aware of the drift:

- On disk `.lisa/hooks/` (`.lisa/hooks/on-stop.sh`, `on-clear.sh`, `on-idle.sh`) has **only 3 scripts — no `on-heartbeat.sh`, no `on-notify.sample`**.
- On disk `.claude/settings.local.json` wires **only** `Stop`, `SessionStart[clear]`, `Notification[idle_prompt]`, with **bare-path commands** (`.lisa/hooks/on-stop.sh`), no `test -x` guard.
- The current code (`crates/lisa-cli/src/templates.rs:133` `settings_local_json()`) additionally wires `PostToolUse` (heartbeat), `PreToolUse[AskUserQuestion]` (awaiting + question-notify), and a matcher-less `Notification` catch-all (permission-notify), all with `test -x … &&` guards.

`lisa init` regenerates hooks (`init.rs:322-325` writes `on-idle.sh/on-stop.sh/on-clear.sh/on-heartbeat.sh`; note `on-notify.sample` referenced at `init.rs:681-687`) and `merge_hooks()` (`templates.rs:284`) upgrades bare-path commands to guarded form in place.

## LISA_PANE_ID: launch → hook → filename → scheduler

This is the correlation key threaded end-to-end:

1. **Slot discovery.** On `PaneManifest` update the plugin records every non-plugin pane as an `AgentSlot { pane_id: pane.id, … }` — `pane_id` is the **Zellij terminal pane id** (`lib.rs:430-449`, `discover_slots`).
2. **Launch.** When scheduling a fresh pane, the plugin writes a shell command into that pane setting the env var to that same id: `build_claude_command` produces `LISA_PANE_ID={pane_id} LISA_TICKET_ID={ticket} claude --dangerously-skip-permissions "…"` (`lib.rs:53-60`, called at `lib.rs:582`). So `$LISA_PANE_ID` inside the agent's shell == the Zellij pane id the plugin tracks.
3. **Hook write.** Each hook script guards `if [ -n "$LISA_PANE_ID" ]` and writes `"$SIGNAL_DIR/pane-$LISA_PANE_ID.<kind>"` where `SIGNAL_DIR=".lisa/signals"` (e.g. `on-stop.sh:9-11`). Content is a single UTC ISO-8601 timestamp: `date -u +%Y-%m-%dT%H:%M:%SZ` (unused by the plugin — mere presence + mtime matter, plugin never reads the body).
4. **Scheduler read.** The plugin sets `self.signal_dir = /host/.lisa/signals` (`lib.rs:2530`; `/host` is the WASI sandbox mount of the project root), `read_dir`s it each tick, parses filenames with `strip_prefix("pane-")` + `strip_suffix(".<kind>")` + `parse::<u32>()`, and matches the id back to the owning `AgentSlot`/`Thread` (e.g. `lib.rs:793-802`).

## poll_tick consumption order (lib.rs:1659-1679)

Order is deliberate and load-bearing:
1. `check_heartbeat_signals()` — refresh activity clocks first
2. `check_awaiting_signals()` — flag blocked panes before any injector runs
3. `check_artifact_advances()` — advance phases on artifact files
4. `check_idle_signals()`
5. `check_transition_signals()` — `.stopped`/`.cleared`
6. `check_transition_timeouts()` — fallback if signals never arrive
7. `check_review_timeouts()`, `evaluate_health()`, `check_session_timeouts()`, `detect_stale_threads()`

## The six signals — per-signal contract

### 1. `pane-<id>.stopped`
- **CC hook event:** `Stop` (fires when Claude finishes responding / is ready for input).
- **Script:** `.lisa/hooks/on-stop.sh` (template `ON_STOP_HOOK` `templates.rs:28-38`).
- **Wiring:** `settings.local.json` → `"Stop"` → `test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh` (`templates.rs:157-166`).
- **Consumer:** `check_transition_signals()` (`lib.rs:1085-1122`) → `handle_stopped_signal(pane_id)` (`lib.rs:1129`). First calls `bump_pane_activity` (`lib.rs:1108`) since agents "often keep working past their stop signal."
- **Scheduler decision — two cases:**
  - If slot is `TransitionState::WaitingForStop`: send `/clear` to the pane, advance to `WaitingForClear` (`lib.rs:1142-1157`).
  - If slot is `Idle` **and** its ticket is in `Phase::Review`: **auto-complete the ticket to Done** via `auto_complete_review` (`lib.rs:1160-1176`) — marks phase+status Done on disk, completes thread, releases slot (guarded by `all_dependencies_done`).
  - Suppressed if pane is awaiting-human (`lib.rs:1145`).

### 2. `pane-<id>.cleared`
- **CC hook event:** `SessionStart` with `matcher: "clear"` (fires after `/clear` is processed).
- **Script:** `.lisa/hooks/on-clear.sh` (`ON_CLEAR_HOOK` `templates.rs:42-52`).
- **Wiring:** `settings.local.json` → `"SessionStart"` matcher `"clear"` (`templates.rs:167-177`).
- **Consumer:** `check_transition_signals()` → `handle_cleared_signal(pane_id)` (`lib.rs:1246`).
- **Scheduler decision:** if slot is `WaitingForClear`, inject the next ticket's prompt (`ticket_prompt`) into the reused session and return slot to `Idle` (`lib.rs:1260-1278`). Suppressed if awaiting-human (`lib.rs:1263`). This is the second half of the **session-reuse handshake**: `/clear` sent → wait `.cleared` → send new prompt (instead of relaunching `claude`).

### 3. `pane-<id>.idle`
- **CC hook event:** `Notification` with `matcher: "idle_prompt"` (session went idle awaiting input).
- **Script:** `.lisa/hooks/on-idle.sh` (`ON_IDLE_HOOK` `templates.rs:14-24`).
- **Wiring:** `settings.local.json` → `"Notification"` matcher `"idle_prompt"` (`templates.rs:178-187`).
- **Consumer:** `check_idle_signals()` (`lib.rs:870-1076`). Resolves ticket via the slot owning `pane_id`; calls `bump_pane_activity` (`lib.rs:903`). Also still supports a **legacy `{ticket_id}.idle` form** (`lib.rs:913-916`).
- **Scheduler decision — phase-dependent (the core RDSPI advancement engine):**
  - `Implement`: idle **alone** advances Implement→Review; if `review.md` already exists, jumps straight to Done in the same tick (`lib.rs:924-988`).
  - `Research|Design|Structure|Plan|Review`: idle **plus the phase's artifact file** (`current_phase.artifact_filename()`, in `docs/active/work/<ticket>/`) advances to `.next()` phase (`lib.rs:990-1038`).
  - **Idle without artifact:** pushes an `idle_alerts` entry (attention banner) and fires the `attention` notification once per stall (debounced via `notified_attention`, `LISA_REASON=idle-without-artifact`) (`lib.rs:1039-1068`).
  - Only acts on `Running` threads; signal file always deleted first (`lib.rs:886`).

### 4. `pane-<id>.heartbeat`
- **CC hook event:** `PostToolUse` (matcher-less — fires after **every** tool call).
- **Script:** `.lisa/hooks/on-heartbeat.sh` (`ON_HEARTBEAT_HOOK` `templates.rs:58-68`). *(Not present on disk — code-only.)*
- **Wiring:** `settings.local.json` → `"PostToolUse"` → `test -x .lisa/hooks/on-heartbeat.sh && …` (`templates.rs:136-145`).
- **Consumer:** `check_heartbeat_signals()` (`lib.rs:785-813`), run **first** each tick.
- **Scheduler decision (liveness ground-truth):** each heartbeat is treated as the *only trusted* proof of genuine progress. It (a) `bump_pane_activity` → resets the slot's wind-down clock and the thread's stuck/stale clocks; (b) removes the pane from `notified_attention` (re-arm attention debounce); (c) removes it from `awaiting_human` (a real tool call means an `AskUserQuestion` was answered). Rationale in `templates.rs:54-57` and `lib.rs:107-112`: stop/idle fire *before* agents truly finish, so the scheduler reuses/reclaims a pane only after it has been heartbeat-silent for `wind_down_secs` — never based on stop/idle alone.

### 5. `pane-<id>.awaiting`
- **CC hook event:** `PreToolUse` with `matcher: "AskUserQuestion"` (agent is about to block on a human question).
- **Emitter:** inline command in `settings.local.json` (`templates.rs:146-156`, const `NOTIFY_QUESTION_COMMAND` `templates.rs:126`), **not** a standalone script. It **unconditionally** `mkdir -p .lisa/signals` and writes `pane-$LISA_PANE_ID.awaiting`, then best-effort `sed`-extracts the question text and (only if `test -x .lisa/hooks/on-notify`) fires the opt-in `on-notify attention` with `LISA_REASON=question`.
- **Consumer:** `check_awaiting_signals()` (`lib.rs:828-857`), run **second** each tick (before any injector). Inserts pane into the `awaiting_human` set.
- **Scheduler decision (injection suppression):** while a pane is in `awaiting_human`, `is_pane_awaiting()` (`lib.rs:297`) gates every writer so lisa never types over the question UI — checked in `send_line_to_pane` itself as a belt-and-suspenders drop (`lib.rs:281-288`), plus per-caller guards in scheduling (`lib.rs:562`), `handle_stopped_signal` (`lib.rs:1145`), `handle_cleared_signal` (`lib.rs:1263`), both transition timeouts (`lib.rs:1323,1341`), review timeout, session-timeout kill, and stale detection. Cleared on the next `.heartbeat` (`lib.rs:811`). Deliberately does **not** bump activity clocks, so an abandoned blocked pane still trips stale detection. Surfaced in the UI via `awaiting` flag (`lib.rs:2727`).

### 6. (no signal file) — permission/attention notification catch-all
- **CC hook event:** matcher-less `Notification` (second entry) (`templates.rs:188-195`, const `NOTIFY_ATTENTION_COMMAND` `templates.rs:115`).
- **Behavior:** reads stdin payload; skips `*idle_prompt*` (already handled by `.idle`); otherwise, if `test -x .lisa/hooks/on-notify`, fires `on-notify attention` with `LISA_REASON=permission`. Writes **no** signal file — it's purely an outbound user-notification path.

## The on-notify user hook (outbound, opt-in)
- Scaffolded as **non-executable** `.lisa/hooks/on-notify.sample` (`ON_NOTIFY_HOOK` `templates.rs:78-107`); user runs `cp on-notify.sample on-notify && chmod +x on-notify` to enable. All `test -x` guards keep it inert until then.
- Contract: `on-notify <event> [detail]`, `$1==$LISA_EVENT` (`complete`|`attention`). Env: `LISA_PROJECT`, and for attention `LISA_REASON` (`question|permission|idle-without-artifact`), `LISA_PANE_ID`, `LISA_TICKET_ID`, `LISA_QUESTION_HEADER`; for complete `LISA_TICKETS_DONE`, `LISA_DURATION_SECS`.
- The **plugin side** invokes it via `build_notify_command` (`lib.rs:315`) / `fire_notify` for the `idle-without-artifact` attention case (`lib.rs:1054-1065`); the hook-side inline commands invoke it for `question`/`permission`.

## Transition state machine (session reuse) — TransitionState
`Idle → WaitingForStop → WaitingForClear → Idle` (`lib.rs:128-137`). Signal-gated, not timer-gated. Timeout fallbacks in `check_transition_timeouts` (`lib.rs:1289`) fire only after both a time budget AND wind-down silence: `STOP_SIGNAL_TIMEOUT_SECS = 60` (`lib.rs:25`), `CLEAR_SIGNAL_TIMEOUT_SECS = 90` (`lib.rs:31`) — so a still-working (heartbeating) pane is never force-clobbered. Note: for **reuse of an already-idle slot**, scheduling sends `/clear` directly and goes straight to `WaitingForClear` (`lib.rs:568-579`), skipping `WaitingForStop` (comment there explains the earlier deadlock).

## Housekeeping
- `.lisa/.gitignore` = `signals/` (`LISA_GITIGNORE` `templates.rs:71`) — signal files are ephemeral, gitignored.
- Every consumer deletes the signal file immediately after reading to prevent re-trigger (`lib.rs:804, 847, 886, 1101, 1111`).
- Wind-down reuse gate: `find_idle_slot` only returns a slot with a live session once it's been silent ≥ `wind_down_secs` (`lib.rs:466-476`).


**Anchors:** /Users/johnchen/swe/repos/lisa/.lisa/hooks/on-stop.sh:1-11, /Users/johnchen/swe/repos/lisa/.lisa/hooks/on-clear.sh:1-11, /Users/johnchen/swe/repos/lisa/.lisa/hooks/on-idle.sh:1-11, /Users/johnchen/swe/repos/lisa/.claude/settings.local.json, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:14-201, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:284-338, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/setup_guide.rs:52-54, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/init.rs:307-325, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:20-60, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:107-137, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:276-299, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:430-476, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:562-590, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:765-857, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:870-1076, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1085-1279, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1289-1357, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:1659-1679, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:2530

---

## Lisa liveness/stuck detection, timeouts, cooldowns, and awaiting/attention state

## Overview

Lisa infers session liveness from **signal files** that Claude Code hooks drop into `self.signal_dir` (`.lisa/signals/`), not from PTY output. Each poll runs every `POLL_INTERVAL_SECS = 5.0` (`crates/lisa-plugin/src/lib.rs:20`) via `poll_tick` (lib.rs:1659). The four signal types are `pane-<id>.heartbeat` (PostToolUse — proof of an active tool call), `pane-<id>.idle` (on-idle), `pane-<id>.awaiting` (PreToolUse[AskUserQuestion]), and `.stopped`/`.cleared` (transition handshake). The single "signs of life" clock is `Thread::last_activity` (types.rs:346-351) plus the per-slot `AgentSlot::last_activity_at` (lib.rs:107-112); both are bumped by `bump_pane_activity` (lib.rs:765-777), which heartbeat/idle signals call.

`poll_tick` ordering matters (lib.rs:1660-1691): heartbeats consumed first (refresh clocks + clear debounce/awaiting), then `check_awaiting_signals` (flag before any injector runs), then idle/transition/review/health/session-timeout/stale in that order.

## Timing thresholds (config: `PluginConfig`, types.rs:466-514; defaults 516-540)

All are parsed from the Zellij config map in `from_config_map` (types.rs:559-619).

| Constant | Default | What it gates |
|---|---|---|
| `stuck_threshold_secs` | 1200 (20 min) | types.rs:490. "Seconds of total inactivity (no heartbeats, signals, or phase changes) before a thread is flagged **Stuck**." Used as the **warning** threshold in `evaluate_health` (lib.rs:1424) and the UI banner (lib.rs:2771). The **hard reclaim bar is 2× this value** (40 min) — used by both `check_session_timeouts` (`hard_silence`, lib.rs:1486) and `detect_stale_threads` (`hard_timeout`, lib.rs:1586). Doc: "tolerates a single silent 30-minute test or integration run with room to spare" (types.rs:487-489, 529-531). |
| `review_timeout_secs` | 600 (10 min) | types.rs:492-494. Seconds a running **Review** thread waits (measured from `last_phase_change`) before receiving a finish-up prompt. `0` disables. Enforced in `check_review_timeouts` (lib.rs:1368-1413). |
| `session_timeout_secs` | 3600 (1 hr) | types.rs:496-499. **Advisory** wall-clock budget per session (measured from `started_at`). "An over-budget session is flagged but only reclaimed after prolonged total silence (2× stuck_threshold_secs)." `0` disables. Used in `check_session_timeouts` (lib.rs:1503-1508). |
| `phase_timeouts` | empty map | types.rs:501-504. Per-phase override of `session_timeout_secs` for **time-in-phase** checks (measured from `last_phase_change`). Missing phases fall back to `session_timeout_secs` via `timeout_for_phase` (types.rs:626-631). Config keys are `phase_timeout_<phase>` (lib.rs parse at types.rs:608-617). |
| `wind_down_secs` | 300 (5 min) | types.rs:506-513. Seconds a pane must be **signal-silent** before the scheduler may reuse it, AND how long a released slot stays in cooldown. Doc rationale: "agents often report 'stopped' and then keep working for another minute or two." |

Hard-coded (non-config) transition-handshake timeouts: `STOP_SIGNAL_TIMEOUT_SECS = 60` (lib.rs:22-25) and `CLEAR_SIGNAL_TIMEOUT_SECS = 90` (lib.rs:27-31). `ENTER_DELAY_SECS = 2.0` (lib.rs:83). Default `max_threads` in code is actually **2** (types.rs:527, `DEFAULT_MAX_THREADS = 2`) despite the CLAUDE.md doc saying 4.

## How liveness is inferred

`Thread::health(now, stuck_threshold)` (types.rs:436-449): only `Running` threads are evaluated; `Parked`/`Completed` return `Healthy` (types.rs:440-442). It computes `elapsed = now - last_activity` and returns `Stuck` iff `elapsed >= stuck_threshold`, else `Healthy` (types.rs:443-448). Note `health()` never returns `Failed` itself — `Failed` comes from `ThreadStatus::Failed` via `is_attention_needed` (types.rs:454-463) and the UI. Crucially, liveness is driven by `last_activity` (silence), not time-in-phase — "a session is only considered stuck when it has gone *silent*, not merely because a phase is taking a long time" (types.rs:346-350).

A **heartbeat** (`check_heartbeat_signals`, lib.rs:785-813) is the master liveness signal: it deletes the file, calls `bump_pane_activity` (resets thread stuck/stale clock AND slot wind-down clock), removes the pane from `notified_attention` (re-arm attention notify), and removes it from `awaiting_human` (a real tool call means the question was answered). Doc lib.rs:781-784: an active session "is never flagged stuck, never reclaimed by a timeout, and never has its pane reused." Idle signals also call `bump_pane_activity` (lib.rs:903).

**Two reclaim paths, both requiring 2× stuck_threshold of silence:**
- `check_session_timeouts` (lib.rs:1482-1572): a thread over its budget (`session_timeout_secs` from `started_at`, or a per-phase limit from `last_phase_change`) is only killed if `silent_for = now - last_activity >= hard_silence` (2× stuck_threshold) AND not awaiting-human (lib.rs:1530-1538). Otherwise it goes to `over_budget_active`, which logs a one-time warning (`over_budget_warned`, lib.rs:1546-1555) but does not interrupt. On reclaim: `thread.fail()`, `release_slot_for_ticket`, remove thread, push `timeout_alerts` (lib.rs:1558-1571). Process is NOT killed (lib.rs:1472-1473).
- `detect_stale_threads` (lib.rs:1581-1617): marks `Running` threads whose `health(now, hard_timeout=2×stuck_threshold) == Stuck` as failed, excluding awaiting-human panes (lib.rs:1597-1601). "silence kills, budgets warn" (lib.rs:1481).

## Cooldown / wind-down gating of pane reuse

`release_slot_for_ticket` (lib.rs:481-503) clears `ticket_id` but keeps `has_session = true` and sets `cooldown_until = now + wind_down_secs` (lib.rs:488-491). `find_idle_slot` (lib.rs:466-476) requires: `ticket_id.is_none()` AND cooldown elapsed (`now >= cooldown_until`) AND (`!has_session` OR pane signal-silent for `wind_down` since `last_activity_at`). So a slot needs both cooldown expiry and sustained quiet before reuse (lib.rs:459-465). The same `wind_down` quiet gate protects `check_transition_timeouts` fallbacks (lib.rs:1300-1302), and `check_review_timeouts` requires `now - last_activity >= wind_down` before prodding (lib.rs:1389). Transition state machine (lib.rs:128-137): `Idle → WaitingForStop → WaitingForClear`, gated on `.stopped`/`.cleared` signals with the 60s/90s fallback timeouts.

## The awaiting/attention (question) state

There are TWO distinct concepts:

**1. `awaiting_human` — blocked on an `AskUserQuestion` (question state).** A `HashSet<u32>` of pane IDs (lib.rs:243-249). `check_awaiting_signals` (lib.rs:828-857) consumes `pane-<id>.awaiting` files (written unconditionally by the PreToolUse[AskUserQuestion] hook), deletes them, and inserts the pane. It deliberately does NOT bump activity clocks (lib.rs:824-827) — a blocked-then-abandoned pane still trips stale detection on the normal silence clock. `is_pane_awaiting` (lib.rs:297-299) reports membership. Effects while set:
- **All injection suppressed.** `send_line_to_pane` has a belt-and-suspenders drop (lib.rs:281-288); every injector also guards explicitly: scheduling (lib.rs:562), stop-signal `/clear` (lib.rs:1143-1145), clear→prompt (lib.rs:1261-1263), transition-timeout stop (lib.rs:1323) and clear (lib.rs:1341), review finish-up (lib.rs:1397). None of these mark state consumed, so they retry once unblocked.
- **Reclaim exemption (T-020-04).** Excluded from `detect_stale_threads` (lib.rs:1599) and from the kill branch of `check_session_timeouts` (lib.rs:1538) — a human may take far longer than hard-silence to answer, so it must never be killed mid-question. Over-budget awaiting panes fall into the warn branch instead.
- **Cleared** on the pane's next heartbeat (`check_heartbeat_signals`, lib.rs:811) — a real tool call means the question was answered.
- **Surfaced in UI** via `ActiveThread.awaiting = is_pane_awaiting(t.pane_id)` (lib.rs:2727; field defined ui.rs:141-144). The thread table renders `<ticket> [AWAITING]` with CYAN color and status text "Awaiting" instead of "Running" (ui.rs:729-733). Comment stresses an exempt-but-invisible pane is the bad state to avoid (ui.rs:724-726).

**2. Attention / the "ATTENTION NEEDED" banner (needs-human-action).** Distinct from the question state. Sources:
- **Idle-without-artifact** (`check_idle_signals`, lib.rs:1039-1068): agent went idle in a Research/Design/Structure/Plan/Review phase but the expected artifact is missing → pushes to `idle_alerts` (lib.rs:1045), logs a Warning, and fires the `on-notify` hook with event `"attention"` (lib.rs:1065), debounced once per stall via `notified_attention` (lib.rs:1054-1055; set defined lib.rs:237-241, cleared by heartbeat at lib.rs:808 so a resumed-then-re-stalled agent re-notifies). Env passed: `LISA_PANE_ID`, `LISA_TICKET_ID`, `LISA_REASON=idle-without-artifact` (lib.rs:1056-1060).
- **Health alerts** built in `to_ui_state` (lib.rs:2772-2830): `Stuck` threads → `AlertType::Stuck` ("No phase change for N+ min", lib.rs:2782-2785); `Failed` → `AlertType::Failed`; `idle_alerts` → `AlertType::IdleWithoutArtifact` (lib.rs:2803-2812); `timeout_alerts` → `AlertType::TimedOut` (lib.rs:2816-2829).
- `render_attention_banner` (ui.rs:337-386) draws the bordered "⚠ ATTENTION NEEDED" box, shown whenever there are Review-phase tickets OR any alerts (ui.rs:345-350); returns/appends nothing otherwise (ui.rs:336). `Thread::is_attention_needed` (types.rs:451-463) is the logical definition: attention needed if health is `Stuck` or `Failed`, OR status is `Parked`.

**Notify hook plumbing:** `build_notify_command` (lib.rs:315-346) builds an `sh -c` invocation of `.lisa/hooks/on-notify` (missing hook = silent no-op via `if [ -x ]`), passing `event`/`detail` as `$1`/`$2` and `LISA_HOOK`/`LISA_EVENT`/`LISA_PROJECT` + extras via env. `fire_notify` (lib.rs:353+) runs it on the host via `run_command`, tagging context with `lisa_notify` (no-op when `project_root` empty, i.e. native tests). Events observed: `"attention"` (idle-without-artifact, lib.rs:1065) and `"complete"` (loop finished, lib.rs:1790). `RunCommandResult` is attributed back and logged (lib.rs:2649-2662).

**Anchors:** crates/lisa-core/src/types.rs:436-463, crates/lisa-core/src/types.rs:466-540, crates/lisa-core/src/types.rs:559-631, crates/lisa-core/src/types.rs:294-356, crates/lisa-plugin/src/lib.rs:20-31, crates/lisa-plugin/src/lib.rs:83, crates/lisa-plugin/src/lib.rs:94-113, crates/lisa-plugin/src/lib.rs:128-137, crates/lisa-plugin/src/lib.rs:237-253, crates/lisa-plugin/src/lib.rs:276-346, crates/lisa-plugin/src/lib.rs:353-360, crates/lisa-plugin/src/lib.rs:459-503, crates/lisa-plugin/src/lib.rs:562, crates/lisa-plugin/src/lib.rs:765-777, crates/lisa-plugin/src/lib.rs:785-857, crates/lisa-plugin/src/lib.rs:1039-1076, crates/lisa-plugin/src/lib.rs:1143-1145, crates/lisa-plugin/src/lib.rs:1261-1263, crates/lisa-plugin/src/lib.rs:1289-1413, crates/lisa-plugin/src/lib.rs:1420-1466, crates/lisa-plugin/src/lib.rs:1482-1617, crates/lisa-plugin/src/lib.rs:1659-1691, crates/lisa-plugin/src/lib.rs:2727, crates/lisa-plugin/src/lib.rs:2771-2830, crates/lisa-plugin/src/ui.rs:141-144, crates/lisa-plugin/src/ui.rs:327-386, crates/lisa-plugin/src/ui.rs:720-733

---

## Lisa config surface, doctor dependency checks, and CLAUDE.md/RDSPI generation-injection path

## Overview: two distinct config layers

Lisa has **two separate config representations**, which is important context for the whole task:

1. **`LisaConfig`** (serde/TOML) — the on-disk `.lisa.toml` read by the **CLI** (`lisa init`, `lisa validate`, `lisa loop`, `lisa doctor`). Defined in `crates/lisa-cli/src/config.rs:9`.
2. **`PluginConfig`** (hand-rolled string-map parser) — read by the **WASM plugin** at load time from the Zellij layout's plugin block. Defined in `crates/lisa-core/src/types.rs`, parsed by `from_config_map` at `crates/lisa-core/src/types.rs:559`.

The CLI bridges them: `lisa loop` parses `.lisa.toml` → `ResolvedConfig`, then writes those values as bare string key/values into a generated Zellij layout's `plugin { ... }` block, which Zellij hands to the plugin as a `BTreeMap<String,String>` that `from_config_map` re-parses. So a config key must be threaded through **all three** places to take effect: `.lisa.toml` schema, layout emission (`loop_cmd.rs`), and `from_config_map`.

---

## (a) `.lisa.toml` parsing and the config surface

### On-disk schema — serde-based (`crates/lisa-cli/src/config.rs`)
`.lisa.toml` is parsed with `toml::from_str` into `LisaConfig` (NOT hand-rolled on the CLI side). Structure (`config.rs:9-34`):

- top-level: `version: Option<String>` (`config.rs:10`), `[dirs]` (`config.rs:12`), `[scheduling]` (`config.rs:14`)
- `[dirs]` (`DirsConfig`, `config.rs:19-23`): `tickets`, `stories`, `work` (all `Option<String>`)
- `[scheduling]` (`SchedulingConfig`, `config.rs:27-34`): `max_threads: Option<usize>`, `auto_advance: Option<bool>`, `review_timeout_secs: Option<u64>`, `session_timeout_secs: Option<u64>`, `wind_down_secs: Option<u64>`, `phase_timeouts: Option<HashMap<String,u64>>`

Loading: `load_config` (`config.rs:70-83`) returns defaults if the file is absent; otherwise reads and calls `validate_config`.

Validation (`validate_config`, `config.rs:129-202`) does a two-pass check: first parses into a generic `toml::Value` to warn on **unknown keys** against hardcoded allowlists — `known_top = ["version","dirs","scheduling"]` (`config.rs:130`), `known_dirs = ["tickets","stories","work"]` (`config.rs:131`), `known_scheduling` (`config.rs:132-139`), and `known_phases = ["research","design","structure","plan","implement","review"]` (`config.rs:171-178`). Unknown keys are **warnings, not errors**. Then it deserializes into `LisaConfig` and enforces one semantic rule: `max_threads == Some(0)` is a hard error (`config.rs:197-199`).

Precedence: `resolve_config` (`config.rs:88-116`) merges `defaults < .lisa.toml < CLI flags`. Only `max_threads` has a CLI override (`cli_max_threads`, from `main.rs:75`). Defaults come from `ResolvedConfig::default` (`config.rs:50-64`), which pulls the `PluginConfig::DEFAULT_*` consts.

Version tracking: `LISA_VERSION = env!("CARGO_PKG_VERSION")` (`config.rs:205`); `version_is_stale` (`config.rs:210-229`) parses `(major,minor,patch)` — stripping `-`/`+` suffixes — and returns `proj < curr`, or `true` if either fails to parse.

`default_config_toml` (`config.rs:232-256`) is the template `lisa init` writes: sets `version`, `[dirs]` (tickets/stories/work), `[scheduling] max_threads = 2`, with the other scheduling keys and `[scheduling.phase_timeouts]` commented out.

### Plugin-side hand-rolled parser (`crates/lisa-core/src/types.rs:559`)
`PluginConfig::from_config_map(config: &BTreeMap<String,String>)` (`types.rs:559-620`) reads bare string keys via `config.get(...)`:

- `ticket_dir`, `story_dir`, `work_dir` → `PathBuf` (`types.rs:562-572`)
- `max_threads` → `.parse()`, silently kept-as-default on parse failure (`types.rs:574-578`)
- `auto_advance` → true only if the string is `"true"` or `"1"` (`types.rs:580-582`)
- `stuck_threshold_secs` (`types.rs:584-588`) — note this key exists in the plugin parser but is **not** in the `.lisa.toml` schema nor emitted by the layout, so it is effectively unreachable from `.lisa.toml`
- `review_timeout_secs`, `session_timeout_secs`, `wind_down_secs` → parse `u64` (`types.rs:590-606`)
- `phase_timeout_{phase}` prefix keys → `Phase::from_name` + `u64` (`types.rs:608-617`). Note the plugin expects **flattened `phase_timeout_<name>` keys**, whereas `.lisa.toml` uses a nested `[scheduling.phase_timeouts]` table — and the layout generator does not emit these, so phase timeouts configured in `.lisa.toml` are currently not propagated to the plugin.

Defaults (`PluginConfig` consts, `types.rs:518-540`): ticket/story/work dirs = `docs/active/{tickets,stories,work}`, `DEFAULT_MAX_THREADS = 2`, `DEFAULT_STUCK_THRESHOLD_SECS = 1200`, `DEFAULT_REVIEW_TIMEOUT_SECS = 600`, `DEFAULT_SESSION_TIMEOUT_SECS = 3600`, `DEFAULT_WIND_DOWN_SECS = 300`; `auto_advance = false`. The plugin invokes this at `crates/lisa-plugin/src/lib.rs:2514`.

### How `.lisa.toml` reaches the plugin (`crates/lisa-cli/src/loop_cmd.rs`)
`generate_layout` (`loop_cmd.rs:199-247`) writes the plugin block with these keys (from `ResolvedConfig`): `ticket_dir`, `story_dir`, `work_dir`, `max_threads`, `auto_advance`, `review_timeout_secs`, `session_timeout_secs`, `wind_down_secs` (`loop_cmd.rs:223-231`). **Not emitted:** `stuck_threshold_secs` and any `phase_timeout_*` — so those two `from_config_map` branches never fire via the normal CLI path.

---

## (b) `lisa doctor` dependency checks (`crates/lisa-cli/src/doctor.rs`)

Structure is **table-driven and designed for extension** — a new client check slots into `build_checks`:

- `CheckResult` enum (`doctor.rs:8-12`): `Found { version }`, `NotFound { install_hint }`, `Skipped { reason }`.
- `DependencyCheck` struct (`doctor.rs:15-19`): `name: &'static str`, `required: bool`, `check: Box<dyn Fn() -> CheckResult>`.
- `get_command_version(cmd, args)` (`doctor.rs:49-62`): runs the command, on success returns the trimmed **first line** of stdout; `None` otherwise.
- `which(name)` (`doctor.rs:65-73`): runs `which <name>`, returns bool.

**The checks list — `build_checks` (`doctor.rs:125-143`):**
1. `zellij`, required=true → `check_zellij` (`doctor.rs:75-84`): runs `zellij --version`; hint = `"cargo install zellij\n    Or visit: https://zellij.dev/documentation/installation"`.
2. `claude`, required=true → `check_claude` (`doctor.rs:86-93`): runs `claude --version`; hint = `"https://docs.anthropic.com/en/docs/claude-code"`.
3. `wasm target`, required=false → `check_wasm_target` (`doctor.rs:95-122`): if `rustup` absent → `Skipped`; else runs `rustup target list --installed` and looks for a line `== "wasm32-wasip1"`; hint = `"rustup target add wasm32-wasip1"`.

**To add a second client's check** (the "slot in" mechanism): write a `fn check_<x>() -> CheckResult` mirroring `check_claude` (it is the closest analog — a single `get_command_version` call with a docs-URL hint), then add a `DependencyCheck { name, required, check: Box::new(check_<x>) }` entry to the `build_checks` vec (`doctor.rs:126-142`). Everything downstream (`run_checks` `doctor.rs:146-155`, `format_report` `doctor.rs:158-175`, `has_failures` `doctor.rs:178-182`, `check_required_deps` `doctor.rs:186-202`) is generic over the vec, so no other code changes are required. `required=true` means a missing binary fails `lisa loop`'s preflight (`check_required_deps`) and makes `run_doctor` return `Err`.

`run_doctor` (`doctor.rs:372-412`) additionally: appends a **project version check** (`check_project_version`, `doctor.rs:205-262` — reads `.lisa.toml`, parses via `toml::from_str::<config::LisaConfig>`, compares `version` against `LISA_VERSION` with `version_is_stale`), then **cleans the Zellij plugin cache** (`clean_zellij_plugin_cache_in`, `doctor.rs:281-306`). Note doctor also owns `pregrant_plugin_permissions_in` (`doctor.rs:335-362`) and the `PLUGIN_PERMISSIONS` list (`doctor.rs:318-323`: WriteToStdin, ChangeApplicationState, ReadApplicationState, RunCommands) — used by `lisa loop`, not by the doctor command itself.

---

## (c) CLAUDE.md / RDSPI generation and injection path

### Embedding at compile time (`crates/lisa-cli/src/templates.rs` + `build.rs`)
- `RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md")` (`templates.rs:4`)
- `HOOKS_GUIDE` = `include_str!("../data/hooks-guide.md")` (`templates.rs:7`)
- `PLUGIN_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"))` (`templates.rs:10`)
- `ON_IDLE_HOOK` and sibling hook scripts are inline string consts (`templates.rs:14+`).

`build.rs` (`crates/lisa-cli/build.rs`) copies the compiled plugin `target/wasm32-wasip1/release/lisa.wasm` (`build.rs:13`) into `OUT_DIR/lisa.wasm` (`build.rs:14,20`) so `include_bytes!` picks it up; if the wasm is missing it writes an **empty placeholder** (`build.rs:22-23`). `lisa loop` guards against that placeholder before launching (`loop_cmd.rs:34-41`).

### CLAUDE.md generation (`templates.rs:340-413`)
`generate_claude_md(project: &DetectedProject)` builds a per-project CLAUDE.md string: a project-type label (`templates.rs:344-350`), an optional Build/Test section (`templates.rs:352-371`), an optional Source Layout (`templates.rs:373-385`), a fixed Directory Conventions block, and a trailer line: `"The RDSPI workflow definition is in docs/knowledge/rdspi-workflow.md and is injected into agent context by lisa automatically."` (`templates.rs:406`).

### Where it is written to disk (`crates/lisa-cli/src/init.rs`)
`lisa init` writes: `CLAUDE.md` from `generate_claude_md` (`init.rs:225-236`), and `docs/knowledge/rdspi-workflow.md` from `RDSPI_WORKFLOW` (`init.rs:239-259`, skipped/updated based on content match), plus `.lisa.toml` from `default_config_toml` (`init.rs:302`) and hook scripts (`init.rs:322`). `validate` requires `CLAUDE.md` (`init.rs:593`) and `docs/knowledge/rdspi-workflow.md` (error, `init.rs:602-606`).

### Where CLAUDE.md is referenced in the agent prompt (`crates/lisa-plugin/src/lib.rs`)
The plugin injects both files by **instructing the agent to read them** (not by inlining content). `ticket_prompt` (`lib.rs:34-48`):
> `"Read the ticket at {path}, CLAUDE.md, and docs/knowledge/rdspi-workflow.md. ..."` (`lib.rs:37`)

`build_claude_command` (`lib.rs:53-60`) wraps that prompt into the launched shell command: `LISA_PANE_ID=... LISA_TICKET_ID=... claude --dangerously-skip-permissions "<prompt>"` (`lib.rs:55`). So injection is via the CLI positional prompt argument, relying on the agent to open `CLAUDE.md` and `docs/knowledge/rdspi-workflow.md` from the working directory.

### Path discrepancy worth flagging
The embedded template and plugin prompt both reference **`docs/knowledge/rdspi-workflow.md`** (`templates.rs:406`, `lib.rs:37`, `init.rs:240`), but this repo's own root `CLAUDE.md` states the workflow lives at **`docs/rdspi-workflow.md`**. For a second client / new project, the authoritative path is `docs/knowledge/rdspi-workflow.md` (that is what `lisa init` writes and what agents are told to read).


**Anchors:** /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:9, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:19, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:27, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:70, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:88, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:129, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:197, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:205, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:210, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/config.rs:232, /Users/johnchen/swe/repos/lisa/crates/lisa-core/src/types.rs:559, /Users/johnchen/swe/repos/lisa/crates/lisa-core/src/types.rs:518, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:2514, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/loop_cmd.rs:199, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/loop_cmd.rs:223, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:8, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:15, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:49, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:75, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:86, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:95, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:125, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:205, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:318, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/doctor.rs:372, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:4, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:10, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:340, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/templates.rs:406, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/build.rs:13, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/init.rs:225, /Users/johnchen/swe/repos/lisa/crates/lisa-cli/src/init.rs:239, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:34, /Users/johnchen/swe/repos/lisa/crates/lisa-plugin/src/lib.rs:53

---

