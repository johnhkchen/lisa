# 03 · Mechanism mapping — lisa need → Codex equivalent

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 by synthesis over [01](./01-lisa-claude-coupling.md), [02](./02-codex-capabilities.md), and the [verified claims](./04-risks-and-open-questions.md).

## Lisa ↔ OpenAI Codex CLI — Mechanism Mapping (Intel Only)

Scope: This maps every load-bearing coupling between lisa and Claude Code onto its closest OpenAI Codex CLI equivalent, records the fit (1:1 / shim / partial / gap), and folds in the adversarial verification verdicts as parity risk. No implementation plan. Codex facts are pinned to stable `rust-v0.142.5` (2026-07-01); the hooks subsystem is the most version-volatile surface and every hook-dependent mapping inherits that risk.

Confidence tags on Codex facts: **[H]** high, **[M]** medium, **[L]** low, per the research.

---

## 1. Executive mapping table

| # | Lisa need | Claude Code mechanism (file:line) | Closest Codex equivalent (exact names) | Fit | Parity risk |
|---|---|---|---|---|---|
| A | Launch agent in a pane with an initial prompt | Shell line `claude --dangerously-skip-permissions "<prompt>"` typed into pane (`lib.rs:53-60`) | Interactive: `codex "<prompt>"` positional. Headless: `codex exec "<prompt>"` (alias `codex e`) | shim | Med — TUI auto-submit-vs-prefill undocumented [M]; headless is the safe path |
| B | Correlate pane↔agent via env var | `LISA_PANE_ID=<id> LISA_TICKET_ID=<id>` inline env prefix on launch (`lib.rs:53-60`) | No documented env-inheritance for hooks; must use hook stdin JSON `session_id`/`cwd`, or `shell_environment_policy.set` | partial→gap | High — env inheritance into hooks undocumented/refuted; `inherit="core"` would drop `PANE_ID` |
| C | Skip permission prompts (unattended) | `--dangerously-skip-permissions` (`lib.rs:55`) | `--dangerously-bypass-approvals-and-sandbox` / `--yolo`; or `-a never -s workspace-write` | 1:1 (near) | Med — dir-trust prompt survives `--yolo` (open #14345); `exec` doesn't pause |
| D | Type text then Enter into a live pane | `write_chars_to_pane_id` + deferred CR `[13]` after `ENTER_DELAY_SECS=2.0` (`lib.rs:276-306`, `83`) | No supported TUI keystroke-injection API; `disable_paste_burst` config exists as mitigation | partial | High — burst-paste heuristic (8ms) can buffer/strip/auto-submit; refuted for reliable orchestration |
| E | Session reuse / context reset without relaunch | Send `/clear`, await `.cleared`, re-inject prompt (`lib.rs:566-590`, `1246-1279`) | `/clear` slash command → `SessionStart` hook `source="clear"`; OR headless `codex exec resume` | shim | Med — `/clear` blocked while task runs [H]; hook delivery bug #17532 in interactive [M] |
| F | `.stopped` signal (turn/phase complete) | `Stop` hook → `on-stop.sh` → `pane-<id>.stopped` (`templates.rs:28-38`; `lib.rs:1129-1157`) | Codex `Stop` hook event | shim | High — `Stop` unreliable in interactive TUI (#17532), doesn't fire on Esc-interrupt (#22858) |
| G | `.cleared` signal (context wiped) | `SessionStart[clear]` → `on-clear.sh` → `pane-<id>.cleared` (`templates.rs:42-52`; `lib.rs:1246`) | Codex `SessionStart` hook, matcher `source="clear"` | shim | Med — schema confirmed [H]; delivery via repo-local config buggy #17532 [M] |
| H | `.heartbeat` liveness (trusted "still working") | `PostToolUse` (matcher-less) → `on-heartbeat.sh` (`templates.rs:58-68`; `lib.rs:785-813`) | Codex `PostToolUse` hook | partial→gap | High — event-driven, no cadence; dead zones (long tool, reasoning-only, approval wait) false-trip stuck; refuted as heartbeat |
| I | `.idle` signal (awaiting input, drives phase advance) | `Notification[idle_prompt]` → `on-idle.sh` (`templates.rs:14-24`; `lib.rs:870-1076`) | No Codex `Notification[idle_prompt]` equivalent; nearest is `Stop` or `notify` `agent-turn-complete` | gap | High — no per-turn "idle awaiting input" hook; RDSPI advance engine loses its trigger |
| J | `.awaiting` signal (blocked on human question) | `PreToolUse[AskUserQuestion]` inline cmd → `pane-<id>.awaiting` (`templates.rs:126,146-156`; `lib.rs:828-857`) | `PreToolUse` matches tool name; no documented `AskUserQuestion`-equivalent blocking tool | gap | High — under `--yolo` no mid-task approval gate at all; no question-tool analog |
| K | Attention/permission notify (outbound) | matcher-less `Notification` → `on-notify` (`templates.rs:115,188-195`; `lib.rs:315-346`) | Codex `notify=[...]` program, event `agent-turn-complete`; or `PermissionRequest` hook | shim | Med — `notify` only fires turn-complete, not approvals (#11808) [H] |
| L | Config toggles reach agent runtime | `.lisa.toml` → layout `plugin{}` → `PluginConfig::from_config_map` (`config.rs:9`, `loop_cmd.rs:199-247`, `types.rs:559`) | Codex `config.toml` / `-c key=value` / `--profile`; not lisa's concern to parse | 1:1 (parallel) | Low — Codex has richer knobs; profile format drifted (v0.134) |
| M | Doctor dependency check for the binary | `check_claude` runs `claude --version` (`doctor.rs:86-93`) | `check_codex` running `codex --version`; table slots into `build_checks` (`doctor.rs:125-143`) | 1:1 | Low — clean extension point already generic |
| N | Project-context file agents must read | `CLAUDE.md` + `docs/knowledge/rdspi-workflow.md`, agent told to read them (`lib.rs:37`, `templates.rs:340-413`) | `AGENTS.md` (override `AGENTS.override.md`), auto-loaded root→cwd, 32 KiB cap | shim | Low — different filename/auto-load semantics; can also just name the file in the prompt |

---

## 2. Per-mechanism detail

### 2.1 Launch & prompt injection (table rows A, B, C, D)

**Claude Code (lisa today).** Lisa never spawns the agent; it writes a shell line into a pre-created idle terminal pane. `build_claude_command` (`crates/lisa-plugin/src/lib.rs:53-60`) emits:
```
LISA_PANE_ID=<pane_id> LISA_TICKET_ID=<ticket> claude --dangerously-skip-permissions "<prompt>"
```
The prompt (`ticket_prompt`, `lib.rs:34-48`) is the sole positional arg. Text is written via `write_chars_to_pane_id`, and a CR byte `[13]` is sent 2s later (`ENTER_DELAY_SECS=2.0`, `lib.rs:83`, `276-306`). Paths are host-relative via `strip_host_prefix` (`lib.rs:89-92`).

**Codex equivalents.**
- Positional prompt: `codex "<prompt>"` (interactive) or `codex exec "<prompt>"` (headless) — positional & optional [H] (https://developers.openai.com/codex/cli/reference, /noninteractive).
- Skip permissions: `--dangerously-bypass-approvals-and-sandbox` / `--yolo` = "no sandbox; no approvals" [H]; safer split is `-a never` (`--ask-for-approval never`) + `-s workspace-write` (`--sandbox`) [H] (https://developers.openai.com/codex/agent-approvals-security).

**Fit & risk.**
- Row A (positional prompt): **shim.** For headless it's a clean map; for interactive TUI, whether `codex "<prompt>"` auto-submits or merely pre-fills the composer is **not definitively documented — version-dependent [M]** (/cli/reference). Lisa's current model relies on the prompt being submitted; under Codex-TUI that's unproven.
- Row C (skip perms): **near 1:1**, but *verification verdict: confirmed-with-caveat* — a first-run directory-**trust** prompt can still block even under `--yolo` (open issue #14345). `codex exec` "does not pause for human input," so headless sidesteps it (https://github.com/openai/codex/issues/14345, /noninteractive).
- Row D (keystroke injection): **partial, highest-fragility.** *Verification verdict: refuted* for reliable unattended orchestration. Codex's TUI runs an ~8ms burst-paste heuristic (`disable_paste_burst` config exists precisely for this); injected text-then-CR can be classified as a paste and buffered, or an embedded newline can auto-submit early / strip lines (#2006, #10065, #20580; PR #18914). Lisa's exact "type, wait 2s, send CR" pattern (`lib.rs:276-306`) is not documented-supported and is version/platform/emulator-sensitive. The parity-preserving path OpenAI documents for this use case is `codex exec` (single-shot, `--json`, `-o`), not TUI driving (https://developers.openai.com/codex/noninteractive, https://github.com/openai/codex/pull/18914).
- Row B (env correlation): **partial → gap** — see 2.6.

The whole "lisa drives a live TUI pane by typing" architecture (`lib.rs:276-306`, the two-phase type/Enter contract) does **not** cleanly survive the port to Codex; it either needs an empirically-verified TUI-injection shim (with `disable_paste_burst=true`) or a redesign around `codex exec`.

### 2.2 Session reuse / context reset (table row E, G)

**Claude Code (lisa today).** On ticket completion lisa keeps the `claude` process alive (`has_session=true`, `release_slot_for_ticket` `lib.rs:481-503`) and reuses it: sends literal `/clear`, sets `TransitionState::WaitingForClear`, waits for the `pane-<id>.cleared` signal (produced by `SessionStart[clear]` hook), then re-injects the next `ticket_prompt` (`schedule_ready_tickets` `lib.rs:566-590`; `handle_cleared_signal` `lib.rs:1246-1279`). Reuse skips `WaitingForStop` to avoid a consumed-signal deadlock (comment `lib.rs:572-574`). Fallback timeout `CLEAR_SIGNAL_TIMEOUT_SECS=90` (`lib.rs:31`).

**Codex equivalents.**
- `/clear` exists and is a true in-place context reset (starts a fresh chat in the same CLI session) [H] (https://developers.openai.com/codex/cli/slash-commands). It fires `SessionStart` with `source="clear"` [H] (https://developers.openai.com/codex/hooks) — the exact analog of lisa's `.cleared` signal.
- Headless alternative to "reuse the same live process": `codex exec resume --last "<prompt>"` / `codex exec resume <SESSION_ID> "<prompt>"` — a *new* process replays persisted rollout, runs one turn, exits. Persistence is default (disabled by `--ephemeral`, gated by `history.persistence`) [H] (https://developers.openai.com/codex/noninteractive).

**Fit & risk.** **shim.**
- *Verification verdict for row G: confirmed* that `/clear` resets context in-place and `SessionStart source=clear` is the documented detectable signal — this is the correct mapping.
- But two caveats hit lisa's exact flow: (1) `/clear` is **disabled while a task is running** [H]; lisa's reuse path already waits for quiet (`wind_down_secs`, `find_idle_slot` `lib.rs:466-476`) so this mostly aligns, but a `Tab`-queue workaround may be needed [M]. (2) Hook **delivery** for `SessionStart` via repo-local `.codex/config.toml` is an **open bug (#17532)**, reproduced through 0.132.0 in interactive TUI — so the `.cleared` backstop is unreliable exactly where lisa needs it. Mitigation surfaced by research: user-level `~/.codex/hooks.json` (independent of project trust), pinned version, empirical verification.
- Architectural note: Codex's *documented* continuity model is resume-a-new-process, not keep-one-process-alive-and-`/clear`. Lisa's whole `has_session` reuse machine (`lib.rs:481-503`, `TransitionState` `lib.rs:128-137`) presumes a persistent typed-into process — that presumption is Claude-Code-shaped and only weakly supported on Codex TUI.

### 2.3 Signal & hook contract — per signal (table rows F–K)

Lisa's nervous system is flat files in `.lisa/signals/pane-<LISA_PANE_ID>.<kind>`, polled every `POLL_INTERVAL_SECS=5.0` (`lib.rs:20`). The hooks that produce them are the porting crux. **Every** row below inherits the env-correlation gap (2.6) because the hook must know which pane it belongs to.

**`.stopped` (row F).** Claude `Stop` → `on-stop.sh` (`templates.rs:28-38`) → consumed by `handle_stopped_signal` (`lib.rs:1129-1157`); drives `WaitingForStop→/clear→WaitingForClear` and auto-completes Review→Done (`lib.rs:1160-1176`). Codex has a `Stop` hook event [H]. **Fit: shim, high risk.** *Verification verdict: refuted* as a reliable turn-completion signal in a live pane: (a) open #17532 shows the executor silently not delivering `Stop`/`SessionStart` in interactive TUI on some setups through 0.132.0 (collaborator non-repro 2026-04-15 → environment-dependent, not "reliable"); (b) #22858 — `Stop` does **not** fire on Esc-interrupted turns, so lisa's "running→ready" state can stick; (c) #21639 — regression where hooks stopped firing after a desktop update. Codex `Stop` also carries `stop_hook_active` / `last_assistant_message` and a `{"decision":"block","reason":...}` continuation shape [M] that has no lisa analog. Lisa's Review→Done auto-complete (which currently keys off `.stopped`) is therefore built on the shakiest Codex signal.

**`.cleared` (row G).** Covered in 2.2. `SessionStart` matcher `source="clear"` [H]. Shim, med risk (#17532).

**`.heartbeat` (row H).** Claude `PostToolUse` matcher-less → `on-heartbeat.sh` (`templates.rs:58-68`) → `check_heartbeat_signals` (`lib.rs:785-813`), run first each tick; it's the **only trusted liveness proof** (resets stuck/stale/wind-down clocks, clears `awaiting_human` and `notified_attention`). Codex has a `PostToolUse` hook [H]. **Fit: partial → gap, high risk.** *Verification verdict: refuted* as a heartbeat/liveness proxy: `PostToolUse` is purely event-driven with no cadence guarantee; documented dead zones — a single long-running tool call (build/test/`sleep`) emits nothing until it returns, reasoning-only/agent_message-only turns emit nothing, a `PermissionRequest` awaiting approval emits nothing — any of which can exceed lisa's `stuck_threshold_secs=1200` while the agent is genuinely alive, producing false "Stuck" and false pane-reuse. Coverage was also historically Bash-only (apply_patch emission only landed ~v0.119, #16732/PR#18391). Research-recommended substitute: consume the `codex exec --json` stream (`turn.started`/`item.started`/`item.completed`, incl. `reasoning` and `command_execution` items) rather than hook silence. This is the single biggest semantic gap in the port: lisa's entire liveness model (2.4) rests on a signal Codex cannot faithfully provide via hooks (https://developers.openai.com/codex/hooks, https://github.com/openai/codex/issues/16732).

**`.idle` (row I).** Claude `Notification[idle_prompt]` → `on-idle.sh` (`templates.rs:14-24`) → `check_idle_signals` (`lib.rs:870-1076`). This is **the RDSPI phase-advance engine**: Implement→Review on idle alone; Research/Design/Structure/Plan/Review→next on idle+artifact; idle-without-artifact → attention. **Fit: genuine gap.** Codex has no documented `Notification` event with an `idle_prompt` matcher. The nearest signals are `Stop` (turn end, but see row F reliability) or the `notify` program's `agent-turn-complete` event [H]. There is no Codex hook that means "the agent went idle awaiting input mid-work without ending its turn." Lisa's phase advancement would have to be re-derived from `Stop`/turn-completion or from `codex exec` process exit — a semantic remap, not a rename.

**`.awaiting` (row J).** Claude `PreToolUse[AskUserQuestion]` inline command (`templates.rs:126,146-156`, `NOTIFY_QUESTION_COMMAND`) writes `pane-<id>.awaiting` → `check_awaiting_signals` (`lib.rs:828-857`) → suppresses all injection and exempts the pane from reclaim (`is_pane_awaiting` `lib.rs:297-299`, guards throughout). **Fit: genuine gap.** Codex `PreToolUse` matches on **tool name** (e.g. `^Bash$`, `apply_patch`) [M]; there is **no documented `AskUserQuestion`-equivalent blocking question tool** to match. *Verification verdict (row on `--yolo`): confirmed* that under `--yolo`/`exec` there is effectively no mid-task approval-gate "awaiting" state at all — the model can only emit clarifying questions as free text (which headless `exec` won't block on; it just proceeds/ends). The closest real "awaiting" is the `PermissionRequest` hook event [M] and the surviving directory-trust prompt (#14345). Net: lisa's entire question-clobber-guard layer (its most defensively-engineered subsystem) has no natural trigger on Codex; the concept it protects against largely doesn't exist under Codex's unattended modes.

**Outbound attention/permission notify (row K).** Claude matcher-less `Notification` → `on-notify attention` (`templates.rs:115,188-195`); plugin side `build_notify_command`/`fire_notify` (`lib.rs:315-346, 353`). Codex has a dedicated `notify=[cmd]` program receiving one JSON arg (`type`, `turn-id`, `input-messages`, `last-assistant-message`, `thread-id`) [H], plus a `PermissionRequest` hook. **Fit: shim, med risk.** `notify` fires **only** on `agent-turn-complete`, not on approval-request (issue #11808) — so lisa's "permission needed" outbound path has no clean `notify` trigger; it would need the `PermissionRequest` hook (subject to the same hook-fragility). Note Codex's `notify` uses **hyphenated** JSON field names, vs lisa's env-var contract.

**poll/consume ordering (`lib.rs:1659-1693`).** Lisa's deliberate tick order (heartbeats → awaiting → artifact advances → idle → transition → timeouts) is internal and portable as-is *if* the underlying signals exist — but rows H/I/J show three of those inputs are gap/partial, so the ordering logic loses meaning without new signal sources.

### 2.4 Liveness / stuck / timeouts (row H consequences)

**Claude Code (lisa today).** Liveness = silence on `Thread::last_activity` / `AgentSlot::last_activity_at`, bumped only by heartbeat/idle (`bump_pane_activity` `lib.rs:765-777`). Thresholds (`types.rs:466-540`): `stuck_threshold_secs=1200` (warn), 2× = hard reclaim (`check_session_timeouts` `lib.rs:1482-1572`, `detect_stale_threads` `lib.rs:1581-1617`), `review_timeout_secs=600`, `session_timeout_secs=3600`, `wind_down_secs=300`. Hard-coded `STOP_SIGNAL_TIMEOUT_SECS=60`, `CLEAR_SIGNAL_TIMEOUT_SECS=90`. Philosophy: "silence kills, budgets warn" (`lib.rs:1481`).

**Codex mapping.** The *timeout arithmetic* is pure lisa logic and ports unchanged. The **input** does not: liveness is inferred from `.heartbeat`, which (row H) Codex cannot supply faithfully via `PostToolUse`. **Fit: partial → gap.** Consequence: on Codex, a long build or reasoning turn produces false Stuck/reclaim. The only robust liveness source is the `codex exec --json` event stream (`turn.started`, `item.started`/`item.completed`) — a different transport (stdout stream vs signal file) than lisa's current file-poll model. `awaiting_human` reclaim-exemption (`lib.rs:1538,1599`) also loses its trigger (row J).

### 2.5 Config toggle surface (row L)

**Claude Code (lisa today).** Two layers bridged by the CLI: `.lisa.toml` (`config.rs:9`, serde) → layout `plugin{}` block (`loop_cmd.rs:199-247`) → `PluginConfig::from_config_map` (`types.rs:559`). A key must be threaded through all three. Known lisa-internal drift already: `stuck_threshold_secs` and `phase_timeout_*` are parsed by the plugin but not emitted by the layout (dead branches).

**Codex mapping.** This is lisa's own config plumbing and is **1:1 / parallel** — unaffected by the client swap, except that a new client selector (which agent to launch) would be a *new* `.lisa.toml` key needing the same three-place threading. Codex's own config (`~/.codex/config.toml`, `-c key=value`, `--profile`, `features.hooks`, `approval_policy`, `sandbox_mode`, `history.persistence`) is richer than lisa exposes and is orthogonal — lisa would *emit* Codex flags/config rather than parse them. Risk **low**, but note Codex config volatility: profile format changed at v0.134, hooks flag renamed `codex_hooks`→`features.hooks`, approval `reject`→`granular`.

### 2.6 Env-var correlation (row B) — the cross-cutting blocker

**Claude Code (lisa today).** `LISA_PANE_ID` set as an inline env prefix on the launch line (`lib.rs:53-60`) is the correlation key threaded launch → hook → filename → scheduler: each hook reads `$LISA_PANE_ID` and writes `pane-$LISA_PANE_ID.<kind>` (`on-stop.sh:9-11`). This is what makes the entire signal system work.

**Codex mapping.** *Verification verdict: uncertain (do not rely).* No Codex doc confirms hook subprocesses inherit the launch-time process environment. The only documented hook env is plugin-bundled `PLUGIN_ROOT`/`PLUGIN_DATA` (+ `CLAUDE_*` compat aliases) [L] and "commands run with the session `cwd`" [H]. `shell_environment_policy` is documented only for model-generated tool subprocesses; default `inherit="all"` would pass a custom `LISA_PANE_ID` (survives the KEY/SECRET/TOKEN filter), but the **recommended production `inherit="core"` would strip it**, and whether the policy even applies to hooks is undocumented. **Fit: partial → gap, high risk.** The documented-safe correlation channel is the hook **stdin JSON** (`session_id`, `cwd`, `transcript_path`, `turn_id`, `source`) — i.e. lisa would key on `session_id`/`cwd` instead of a pane id it controls. This inverts lisa's model: today lisa *chooses* the correlation id (pane id) before launch; with Codex it would have to *learn* the id (`session_id`) after `SessionStart` fires. This is a structural remap that touches slot discovery (`lib.rs:430-457`), filename parsing (`lib.rs:793-802`), and every consumer.

### 2.7 Dependency doctoring (row M)

**Claude Code (lisa today).** `check_claude` runs `claude --version` (`doctor.rs:86-93`), registered in the table-driven `build_checks` vec (`doctor.rs:125-143`); everything downstream is generic. Research explicitly flagged this as "designed for extension."

**Codex mapping.** **Clean 1:1.** A `check_codex` running `codex --version` with a docs-URL hint, added to `build_checks`, requires no other change. `required=true` gates `lisa loop` preflight. Risk **low**. (Version pinning advice from research: pin `@openai/codex@0.142.x` — but that's operational, not a code-shape concern.)

### 2.8 Project-context file (row N)

**Claude Code (lisa today).** Agents are told to *read* `CLAUDE.md` + `docs/knowledge/rdspi-workflow.md` in the prompt (`lib.rs:37`); `generate_claude_md` writes `CLAUDE.md` (`templates.rs:340-413`), and both files are referenced rather than inlined. (Note existing repo drift: root `CLAUDE.md` says `docs/rdspi-workflow.md`, authoritative path is `docs/knowledge/rdspi-workflow.md`.)

**Codex mapping.** **shim.** Codex's native project-context file is `AGENTS.md` (override `AGENTS.override.md`), auto-discovered root→cwd, concatenated closest-wins, capped at `project_doc_max_bytes=32 KiB`, scaffolded by `/init` [H] (https://developers.openai.com/codex/guides/agents-md). Two options with different semantics: (1) generate `AGENTS.md` so Codex auto-loads it (no prompt instruction needed, but 32 KiB cap and different filename), or (2) keep the "tell the agent to read file X" prompt approach (client-agnostic, unaffected by the swap). Because lisa currently *names the file in the prompt*, option (2) is nearly free; the RDSPI workflow file reference is portable verbatim. Risk **low**.

---

## 3. Genuine gaps & unknowns (require a spike before depending on them)

**Genuine gaps — no clean Codex equivalent:**
1. **Heartbeat/liveness (row H).** `PostToolUse` cannot serve as a cadence heartbeat (refuted). Lisa's entire silence-based stuck/reclaim model has no faithful hook input on Codex. Robust path is the `codex exec --json` event stream — a different transport than file-polling.
2. **`.idle` phase-advance trigger (row I).** No Codex `Notification[idle_prompt]` analog. The RDSPI advancement engine (`lib.rs:870-1076`) must be re-sourced from `Stop`/turn-completion or `codex exec` exit.
3. **`.awaiting` / AskUserQuestion (row J).** No documented Codex blocking-question tool to match in `PreToolUse`. Under `--yolo`/`exec`, the "awaiting human" state largely does not exist (confirmed). Lisa's question-clobber guards have no natural trigger.
4. **Env-var correlation key (row B).** `LISA_PANE_ID` inheritance into hooks is undocumented/unreliable. Correlation must move to stdin-JSON `session_id`/`cwd`, inverting lisa's choose-id-before-launch model.
5. **Persistent-process-and-`/clear` reuse (rows D/E).** Codex's documented continuity is `codex exec resume` (new process replays rollout), not a long-lived typed-into TUI. Lisa's `has_session` reuse machine is Claude-Code-shaped.

**Unknowns to verify empirically on the pinned version (`rust-v0.142.5`):**
- Does `codex "<prompt>"` auto-submit or only pre-fill the composer? [M, undocumented]
- Does any hook subprocess inherit a launch-time env var? Does `shell_environment_policy` apply to hooks? [undocumented, test]
- Do `SessionStart[clear]` and `Stop` hooks actually fire in an interactive pane on this version (open #17532, through 0.132.0)? Via user-level `~/.codex/hooks.json` vs repo-local?
- Does `--yolo` suppress the directory-trust prompt, or does #14345 still reproduce?
- Exact literal JSON field spellings for `SessionStart`/`Stop` and exact `exec --json` item type strings (`file_change` vs `patch`, `todo_list` vs `plan`).
- Whether TUI keystroke injection with `disable_paste_burst=true` + a > burst-interval Enter delay is reliable enough, or whether the whole thing must move to `codex exec`.

---

## 4. Confidence ledger

**Solid mappings (low risk, safe to design on):**
- Dependency doctor (M) — clean table extension, no downstream change.
- Config plumbing (L) — lisa-internal, orthogonal to client swap.
- Project-context file (N) — prompt already names the file; `AGENTS.md` optional.
- Skip-permissions flag (C) — `--yolo` / `-a never -s workspace-write` documented [H]; caveat is the dir-trust prompt.
- `/clear`→`SessionStart[clear]` *schema* (E/G) — the reset signal is confirmed [H].

**Shaky mappings (need a spike; do not commit design):**
- `Stop` hook as turn-completion (F) — refuted for interactive reliability; doesn't fire on Esc-interrupt.
- `PostToolUse` as heartbeat (H) — refuted; the core liveness model breaks.
- `.idle` phase-advance (I) — no equivalent event; gap.
- `.awaiting`/AskUserQuestion (J) — no equivalent tool; concept mostly absent under unattended Codex.
- Env-var pane correlation (B) — inheritance unverified; likely must switch to stdin `session_id`.
- TUI keystroke injection (D) — refuted as reliable; burst-paste heuristic.
- Hook delivery in interactive sessions generally (#17532, open through 0.132.0) undermines every hook-dependent row (E,F,G,H,J,K).

**Overall:** the *client-agnostic* halves of lisa (doctor, config, project-context, prompt authoring) port cleanly. The *coupling surface* — how lisa launches into a live pane, injects keystrokes, and reads back per-turn hook signals — is where 6 of the 7 signal/hook mappings are partial-to-gap and rest on Codex's most volatile, bug-prone subsystem. The research's consistent recommendation is that faithful parity likely requires re-anchoring lisa's agent transport on headless `codex exec` (`--json` events, `resume` for continuity) rather than mirroring the Claude-Code TUI-driving + hook-file model 1:1.


**Sources:** crates/lisa-plugin/src/lib.rs:34-60, crates/lisa-plugin/src/lib.rs:83, crates/lisa-plugin/src/lib.rs:89-92, crates/lisa-plugin/src/lib.rs:94-113, crates/lisa-plugin/src/lib.rs:128-137, crates/lisa-plugin/src/lib.rs:276-306, crates/lisa-plugin/src/lib.rs:297-299, crates/lisa-plugin/src/lib.rs:315-346, crates/lisa-plugin/src/lib.rs:430-457, crates/lisa-plugin/src/lib.rs:466-503, crates/lisa-plugin/src/lib.rs:566-590, crates/lisa-plugin/src/lib.rs:765-777, crates/lisa-plugin/src/lib.rs:785-813, crates/lisa-plugin/src/lib.rs:828-857, crates/lisa-plugin/src/lib.rs:870-1076, crates/lisa-plugin/src/lib.rs:1129-1178, crates/lisa-plugin/src/lib.rs:1246-1279, crates/lisa-plugin/src/lib.rs:1289-1360, crates/lisa-plugin/src/lib.rs:1368-1413, crates/lisa-plugin/src/lib.rs:1482-1617, crates/lisa-plugin/src/lib.rs:1659-1693, crates/lisa-core/src/types.rs:466-540, crates/lisa-core/src/types.rs:559-631, crates/lisa-cli/src/config.rs:9-256, crates/lisa-cli/src/loop_cmd.rs:199-247, crates/lisa-cli/src/templates.rs:14-195, crates/lisa-cli/src/templates.rs:340-413, crates/lisa-cli/src/doctor.rs:86-143, https://developers.openai.com/codex/hooks, https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/cli/slash-commands, https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/agent-approvals-security, https://developers.openai.com/codex/config-reference, https://developers.openai.com/codex/config-advanced, https://developers.openai.com/codex/guides/agents-md, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/22858, https://github.com/openai/codex/issues/21639, https://github.com/openai/codex/issues/16732, https://github.com/openai/codex/pull/18391, https://github.com/openai/codex/pull/18914, https://github.com/openai/codex/issues/14345, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/2006, https://github.com/openai/codex/issues/10065, https://github.com/openai/codex/issues/20580

