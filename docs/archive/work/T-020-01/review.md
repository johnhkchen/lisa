# T-020-01 Review — AskUserQuestion attention spike

**Type:** spike. **Outcome:** GO. **Production code merged:** none (by design).
Handoff for a human reviewer deciding whether to greenlight T-020-02..04.

## What this ticket produced

Files created (all under `docs/active/work/T-020-01/`):

- `research.md` — map of lisa's hook / signal / injection / notification machinery.
- `design.md` — Q1–Q6 answered with evidence; the go/no-go; proposed implementation tickets.
- `structure.md` — file-level blueprint for the implementation.
- `plan.md` — ordered, individually-verifiable implementation steps + test strategy.
- `progress.md` — spike execution log.
- `pretooluse-payload-sample.json` — the **captured live payload** (also a future test fixture).

**No source files were modified.** `git status` confirms the only T-020-01 changes are the new
work-dir files; the `crates/**` modifications in the tree are pre-existing S-019 (T-019-*) work,
unrelated to and untouched by this ticket.

## The headline result (the gate)

S-020's whole premise rested on an **undocumented** question: do
`--dangerously-skip-permissions` agents actually call `AskUserQuestion`? I settled it
empirically rather than by inference. A throwaway project with a `PreToolUse[AskUserQuestion]`
hook, run against `claude --dangerously-skip-permissions` (v2.1.185), produced a real captured
payload stamped `"permission_mode":"bypassPermissions"` — proving the agent invokes the tool in
exactly the mode lisa spawns, and that the `PreToolUse` hook (matcher `"AskUserQuestion"`)
fires. **GO.**

## Findings (evidence quality flagged)

| Q | Answer | Confidence / basis |
|---|--------|--------------------|
| 1 PreToolUse fires; matcher `"AskUserQuestion"` | YES | **High** — observed in probe + docs |
| 2 GATE: bypass-mode agents call it | YES → **GO** | **High** — captured payload, bypassPermissions |
| 3 Question text via POSIX `sh` | YES, best-effort `sed` | Medium — single-line JSON; escaped-quote edge case |
| 4 Flag clears on resume heartbeat | YES | Medium — relies on matcher-less heartbeat, not AUQ's own PostToolUse |
| 5 Suppression: `.awaiting` signal + `awaiting_human` set, guard `send_line_to_pane` + 5 callers | designed | High — mirrors existing `notified_attention` pattern |
| 6 Exempt awaiting panes from reclamation | designed | High |

## Design in one paragraph

A new POSIX-`sh` `PreToolUse[AskUserQuestion]` hook (built like the existing
`NOTIFY_ATTENTION_COMMAND`, `templates.rs:110`) writes `.lisa/signals/pane-<id>.awaiting`
**and** fires the existing `on-notify` with a new `LISA_REASON=question`. The plugin adds an
`awaiting_human: HashSet<u32>` (twin of `notified_attention`, `lib.rs:241`), populated by a new
`check_awaiting_signals()` (modeled on `check_heartbeat_signals`, `lib.rs:760`) and **cleared on
the next heartbeat** at the same site that already clears `notified_attention` (`lib.rs:783`).
While set, it suppresses every `send_line_to_pane` injection (5 callers) and exempts the pane
from timeout/stale reclamation (`check_session_timeouts:1385`, `detect_stale_threads:1477`),
without faking activity — so the v0.2.11 heartbeat-liveness invariant is preserved.

## Acceptance-criteria check

- [x] `design.md` answering Q1–Q6 with evidence (captured payload + yes/no on Q2 with method).
- [x] Clear go/no-go on the Q2 gate: **GO**.
- [x] Implementation-ticket breakdown (T-020-02/03/04) with file:line anchors.
- [x] Confirms reuse of the S-019 `on-notify` contract (new `LISA_REASON=question`, no new user
      hook) and POSIX-`sh`-only hook side.
- [x] No production code merged beyond the design; throwaway prototype kept separate (deleted;
      only its captured payload retained as evidence).

## Test coverage

None applicable to this ticket — a spike merges no code. The downstream test plan is in
`plan.md`: native unit tests dominate (`templates.rs` binding/merge/idempotency + a
`sed`-extraction test using `pretooluse-payload-sample.json`; `lib.rs` awaiting-scan, clear,
each injection guard, each reclamation exemption). One genuinely manual check is unavoidable —
T-020-02 step 1, an interactive `lisa loop` run — because it needs a live interactive `claude`
pane.

## Open concerns / for human attention

1. **Interactive blocking is unverified.** The probe ran headless `-p`, where the question was
   *dismissed without a selection* and the agent **continued** instead of blocking; no
   `PostToolUse` fired. lisa runs claude **interactively**, where the TUI should present the
   question and block until answered. The PreToolUse firing + tool invocation are proven; the
   interactive block/answer/resume cycle is **not**. This is why T-020-02 step 1 is a hard gate
   before the rest of the build-out. **Reviewer: accept this as the first implementation task,
   or ask for the interactive probe now.**
2. **Question-text extraction fragility.** A `question` field containing an escaped `\"` defeats
   the simple `sed`. Mitigated by best-effort extraction + a generic fallback detail + passing
   the raw payload — never a hard failure — but the notification text can be imperfect.
3. **Flag-clear dependency.** Clearing relies on the agent making *any* tool call after the
   answer (matcher-less heartbeat). If an agent answers and then `Stop`s with no tool call, the
   pane is genuinely idle and handled by the existing idle/stop paths — safe, but worth a test.
4. **Scope creep risk.** The story is correctly split into 3 small tickets; resist folding the
   plugin guards (T-020-03) into the hook ticket (T-020-02) — the hook can land and be validated
   independently.

## Recommendation

Greenlight T-020-02 with its step-1 interactive validation as the first gate. The mechanism is
proven at the hook layer; the remaining unknown is purely the interactive blocking behavior,
which is cheap to confirm and low-risk given the PreToolUse evidence. The design reuses existing
patterns (`notified_attention`, the Notification catch-all, the signal-file convention) so the
implementation surface is small and well-bounded.
