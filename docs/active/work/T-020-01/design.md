# T-020-01 Design — AskUserQuestion attention detection + awaiting-human suppression

Spike deliverable. Answers Q1–Q6 with evidence, gives the **go/no-go on the gate (Q2)**,
and proposes the implementation tickets (T-020-02+). No production code is merged from this
ticket; the empirical probe below was a throwaway, kept only as the captured payload sample
at `pretooluse-payload-sample.json`.

## Evidence: the empirical probe

Documentation alone left Q2 (the gate) **undocumented**, so it was settled by experiment.
Setup: a throwaway project with a `PreToolUse` hook `{"matcher":"AskUserQuestion"}` that
appended its stdin to a log, plus a `PostToolUse[AskUserQuestion]` logger. Run:

```
claude --dangerously-skip-permissions -p "<prompt forcing a single AskUserQuestion call>"
```

(claude 2.1.185, macOS). Result: the hook **fired**, capturing this real payload (pretty-printed):

```json
{ "session_id":"…", "transcript_path":"…", "cwd":"/private/tmp/auq-spike",
  "permission_mode":"bypassPermissions", "effort":{"level":"high"},
  "hook_event_name":"PreToolUse", "tool_name":"AskUserQuestion",
  "tool_input":{"questions":[{"question":"Which approach should I use to build the feature?",
    "header":"Approach",
    "options":[{"label":"A: New signal file","description":"…"},
               {"label":"B: Event-driven","description":"…"}],
    "multiSelect":false}]},
  "tool_use_id":"toolu_01Gbpr…" }
```

The agent stdout confirmed it *chose* to call the tool: "I've asked the question via the
AskUserQuestion tool, but it looks like the prompt was dismissed without a selection."

## Q1 — Does `AskUserQuestion` fire `PreToolUse`, and the matcher string? **YES**

Confirmed by the probe and docs. The hook event is `PreToolUse`; the matcher is the literal
tool name `"AskUserQuestion"`; the payload's `tool_name` is exactly `"AskUserQuestion"`. This
is the same `PreToolUse` machinery lisa does **not** yet bind (`templates.rs:268-296` wires no
PreToolUse). High confidence.

## Q2 — GATE: do `--dangerously-skip-permissions` agents invoke it? **YES → GO**

**Determined empirically** (the docs do not state this). The captured payload carries
`"permission_mode":"bypassPermissions"` — i.e. the agent invoked `AskUserQuestion` *while in
the exact mode lisa spawns* (`build_claude_command`, `lib.rs:55`). PreToolUse hooks fire in
bypass mode, and `AskUserQuestion` has no permission requirement, so it is not suppressed.
**The signal is real; the story is a GO.**

Residual risk (honest): the probe ran headless `-p`, where the question was *dismissed
without a selection* and the agent **continued** rather than blocking. lisa runs claude
**interactively in a zellij pane**, where the question is presented in the TUI and blocks
until answered. The probe therefore proves *the model calls the tool and the hook fires under
bypassPermissions*, but does not exercise the interactive blocking/answer cycle. That cycle is
validated as step 1 of the first implementation ticket (below).

## Q3 — Question text extractable in POSIX `sh` (no jq)? **YES, with care**

The payload is **single-line JSON**. The detail string for `on-notify` can be lifted with a
POSIX `sed` against the first question, mirroring the existing `NOTIFY_ATTENTION_COMMAND`
style (`templates.rs:110`):

```sh
in=$(cat)
q=$(printf '%s' "$in" | sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p')
[ -z "$q" ] && q="agent is asking a question"
```

Gotchas: a `question` containing an escaped `\"` truncates the greedy-free `[^"]*`; and the
catch-all already passes the **raw payload** as `$2` and lets the user hook decide. Decision:
**do not over-parse in the hook.** Extract a best-effort first-question string for the
`detail`, but also pass enough context that a missed extraction degrades to a generic
"needs you" message — never a hard failure. The header (`"header":"Approach"`) is similarly
extractable and is a good short label.

## Q4 — Does the pane resume with a `PostToolUse` heartbeat, clearing the flag? **YES (by design, not single-hook)**

The probe did **not** see a `PostToolUse[AskUserQuestion]` (the headless question was
dismissed, not answered, so the call did not "succeed"). That is acceptable because the clear
does **not** depend on AskUserQuestion's *own* PostToolUse:

- lisa's heartbeat hook is a **matcher-less `PostToolUse`** (fires for *every* tool —
  `templates.rs:288-291`, `ON_HEARTBEAT_HOOK`). After the human answers and the agent resumes,
  its **next tool call** writes `.heartbeat`, which `check_heartbeat_signals` consumes
  (`lib.rs:760-785`) and which already clears `notified_attention` (`lib.rs:783`).
- If `AskUserQuestion` *does* emit its own PostToolUse on a real answer (docs say PostToolUse
  fires after a tool succeeds), that *also* writes a heartbeat — same clear path, sooner.
- If the agent answers and immediately stops with no further tool call, `Stop`→`.stopped`
  fires and the pane is genuinely idle — the correct state anyway.

So the existing heartbeat path is the clear mechanism; the awaiting flag rides on the same
`HashSet<u32>` + heartbeat-clear pattern as `notified_attention` (`lib.rs:241,783`). Robust to
the Q4 uncertainty.

## Q5 — Smallest suppression change

**Mechanism — new signal file (chosen).** A `PreToolUse[AskUserQuestion]` hook writes
`.lisa/signals/pane-<id>.awaiting`; a new `check_awaiting_signals()` (modeled on
`check_heartbeat_signals`, `lib.rs:760`) reads it and inserts the pane id into a new
`awaiting_human: HashSet<u32>` field on `State` (alongside `notified_attention`, `lib.rs:241`;
`#[derive(Default)]` needs no init — obs 23137). Rejected alternative: inferring from a
PreToolUse *event* in the plugin — the plugin has no event channel from Claude Code; it only
reads signal files (`hooks-guide.md:17`). The signal-file route is the only one consistent
with the architecture and reuses the existing `read_dir(signal_dir)` scan.

**Clear:** in `check_heartbeat_signals`, where `notified_attention` is already cleared
(`lib.rs:783`), also `self.awaiting_human.remove(&pane_id)`. One line, same place, same proof
("a real tool call happened → not blocked anymore").

**Guard — wrap injection in `send_line_to_pane`'s callers.** Add a helper
`fn is_pane_awaiting(&self, pane_id: u32) -> bool` and guard the **five injection points**
(Q5 list, current anchors):

| Caller (`lib.rs`)               | Inject                | Guard action when awaiting |
|---------------------------------|-----------------------|----------------------------|
| `schedule_ready_tickets:550/559`| `/clear` / launch     | skip slot this tick (it stays assigned; retried next poll) |
| `handle_stopped_signal:1071`    | `/clear`              | return early (no transition while blocked) |
| `handle_cleared_signal:1186`    | prompt                | return early |
| `check_transition_timeouts:1245/1262` | `/clear` / prompt | skip that pane in the fallback loop |
| `check_review_timeouts:1306`    | finish-up prompt      | skip candidate (most acute clobber risk) |

Cleanest single chokepoint: an awaiting check **inside `send_line_to_pane`** that drops the
write and logs, *plus* early-returns at the transition callers so they don't advance their
state machine mid-question. Belt-and-suspenders: guarding the one method makes a missed
caller fail safe (no clobber), while the per-caller returns keep the slot/transition state
coherent. Note callers #1/#2 (fresh scheduling) only ever target an *idle* slot, which by
definition has no awaiting agent — guarding them is defensive, not load-bearing.

**Heartbeat-model safety:** the flag never fakes activity (it does not touch
`last_activity_at`), so genuinely-dead panes still trip stale detection
(`detect_stale_threads`, `lib.rs:1477`) on the normal silence clock. It only *gates writes*.
This preserves the v0.2.11 liveness invariant (`[[liveness-heartbeat-design]]`).

## Q6 — Timeout interaction

Two reclamation paths can kill an awaiting pane on the wall-clock (research §7):
`check_session_timeouts` (`lib.rs:1385`, reclaims at `2×stuck_threshold_secs` hard silence)
and `detect_stale_threads` (`lib.rs:1477`, same bar). **Decision: exempt awaiting panes from
*reclamation* but keep them visible.** A human may take many minutes to answer — longer than
hard-silence — and reclaiming mid-question is exactly the failure S-020 exists to prevent. Add
`!self.awaiting_human.contains(&pane_id)` to the filters in both reclaimers. Do **not** exempt
the *injection* timeouts (`check_transition_timeouts`/`check_review_timeouts`) from running —
they are already guarded by Q5 (they just skip the write), and we want them to resume normally
once the flag clears. Surface awaiting panes in the dashboard (`ui.rs`) so an exempt pane is
never invisible. (Per-phase/global budget *warnings* may still log; only the kill is exempt.)

## Reuse & ordering

Reuses S-019 verbatim: the `on-notify` contract + a **new `LISA_REASON=question`** value (no
new user hook — `hooks-guide.md:46-77`), and the same POSIX-`sh`/`test -x` hook style as
`NOTIFY_ATTENTION_COMMAND`. The notify fires **from Claude Code's PreToolUse hook** (like the
permission catch-all path), not from the plugin — so it needs no `run_command` plumbing.
Hard ordering: this builds on T-019-02 (on-notify scaffolding + `merge_hooks`) and the
heartbeat clear path (already shipped). The doc update belongs with T-019-03's `hooks-guide.md`.

## Go / No-Go: **GO** (conditional on step-1 interactive validation)

## Proposed implementation tickets

- **T-020-02 — PreToolUse[AskUserQuestion] hook binding + notify** (depends on T-019-02).
  New `ON_PRETOOL_ASK_HOOK`-style command + `templates.rs` const; add a 6th binding in
  `settings_local_json` (`templates.rs:116`) and `merge_hooks` (`templates.rs:296`,
  matcher-less PreToolUse, dedup by command substring like the Notification catch-all);
  write `pane-<id>.awaiting` **and** call `on-notify attention "<q>"` with
  `LISA_REASON=question`. Update `lisa validate` (`init.rs:654,680-708`) + `hooks-guide.md`.
  **Step 1: validate interactively under real `lisa loop`** (closes the Q2/Q4 residual risk)
  before further work. Tests: `templates.rs` merge/idempotency + payload-extraction `sed`.
- **T-020-03 — Plugin awaiting-human flag + injection suppression** (depends on T-020-02).
  Add `awaiting_human: HashSet<u32>` (`lib.rs:241`); `check_awaiting_signals()` consuming
  `.awaiting` (model on `lib.rs:760`); clear in `check_heartbeat_signals` (`lib.rs:783`);
  guard `send_line_to_pane` + the five callers (Q5 table); call `check_awaiting_signals` in
  `poll_tick` before `check_idle_signals` (`lib.rs:1551-1557`). Unit tests per guarded caller.
- **T-020-04 — Timeout-reclamation exemption + dashboard surfacing** (depends on T-020-03).
  Add `!awaiting_human.contains` to `check_session_timeouts` (`lib.rs:1399`) and
  `detect_stale_threads` (`lib.rs:1484`); render an "awaiting human" marker in `ui.rs`. Tests
  asserting an awaiting pane is not reclaimed at hard silence.
