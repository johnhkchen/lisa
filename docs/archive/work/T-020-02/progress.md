# T-020-02 Progress

## Status: implementation complete; step-1 interactive gate is MANUAL (pending human run)

## Completed (plan steps)

- **Step 1** — `NOTIFY_QUESTION_COMMAND` const added to `templates.rs` (raw-shell, Rust-escaped),
  with a doc comment explaining the unconditional signal write vs. the `test -x`-gated notify.
- **Step 2** — new `"PreToolUse"` block in `settings_local_json()` with a single
  `"matcher": "AskUserQuestion"` entry; command is the JSON-escaped form of the const.
- **Step 3** — `ensure_hook(hooks_obj, "PreToolUse", Some("AskUserQuestion"), NOTIFY_QUESTION_COMMAND)`
  added to `merge_hooks()`; both relevant doc comments updated (5 → 6 bindings).
- **Step 4** — `count_question_commands` JSON-parsing test helper added.
- **Step 5** — `templates.rs` tests: extended `test_settings_local_json` (PreToolUse len/matcher
  + const-parity guard) and `test_merge_hooks_empty_object`; new
  `test_merge_hooks_adds_pretooluse_question` (idempotency) and
  `test_notify_question_command_extracts_question` (real-`sed` extraction: happy path, escaped-
  quote degradation, no-question → empty → generic fallback).
- **Step 6** — `init.rs` validate expected-binding row `("AskUserQuestion", "PreToolUse[AskUserQuestion]")`.
- **Step 7** — `init.rs` new `test_validate_missing_pretooluse_binding`.
- **Step 8** — `data/hooks-guide.md`: `LISA_REASON=question` row; third firing path; six-binding
  counts; Manual-setup `PreToolUse` JSON block + explanation.

## Test results

- `cargo test -p lisa-cli --bin lisa` → **172 passed, 0 failed** (was 169 before this ticket;
  +3 new tests: `test_merge_hooks_adds_pretooluse_question`,
  `test_notify_question_command_extracts_question`, `test_validate_missing_pretooluse_binding`).
- `just check` → see review.md (WASM check + full workspace suite).
- Plan-count test (`test_plan_init_actions_empty_dir`, == 18) unchanged — no new file scaffolded,
  as designed (the command is inline).

## Deviations from plan

- **None material.** One clarification recorded in research/design: the spike's `structure.md`
  claimed the heartbeat was a matcher-less `PreToolUse`. It is actually `PostToolUse`; lisa had
  **no** `PreToolUse` binding before this ticket. So the new `PreToolUse` key is created fresh
  with a single matchered entry (cleaner than the "coexist with a matcher-less sibling" path the
  ticket hedged about). The matcher-based `ensure_hook` dedup is used (not command-substring).

## Step-1 interactive gate — MANUAL, NOT YET RUN (gate-closer per AC)

The first acceptance criterion requires validating, **interactively under a real `lisa loop`**,
that an agent's `AskUserQuestion` call (a) fires the `PreToolUse` hook, (b) blocks the pane until
answered, and (c) resumes with a `PostToolUse` heartbeat after answering. This **cannot be
exercised by an autonomous/headless run** — it requires a human to answer a TUI prompt inside a
zellij pane. The spike (T-020-01) already proved (a) under headless `--dangerously-skip-permissions`
and captured a real payload ([[askuserquestion-fires-pretooluse]]); (b)/(c) are the residual risk
flagged in the spike's design (Q2/Q4).

**Procedure for the human reviewer (run before T-020-03 builds on the signal):**

1. From a project that has been `lisa init`'d with this build, start `lisa loop`.
2. In a spawned pane, prompt the agent to call `AskUserQuestion` (e.g. ask it to choose between
   two options and require it to use the tool).
3. Confirm a `.lisa/signals/pane-<id>.awaiting` file appears (the hook wrote it):
   `ls .lisa/signals/`.
4. Confirm the pane **blocks** on the question (it does not auto-advance / clobber).
5. Optionally enable `on-notify` (`cp on-notify.sample on-notify && chmod +x on-notify`, point it
   at any transport) and confirm an `attention` notification fires with `LISA_REASON=question`.
6. Answer the question in the TUI. Confirm the agent **resumes** and its next tool call writes a
   `.heartbeat` signal.

**If blocking/resume does not behave as designed, STOP and reassess before T-020-03** (which adds
the plugin-side `awaiting_human` flag + injection suppression that depends on this signal). This
ticket only *writes* the `.awaiting` signal; nothing consumes it yet, so shipping the code ahead
of the manual gate is harmless (an unread file the plugin ignores).

## Gate run — 2026-06-20 (automated portion CLOSED; interactive remainder narrowed)

Ran by the orchestrator. The gate decomposes into the signal/notify path (automatable)
and claude's interactive block/resume (needs a human TUI). Results:

- **(a) hook fires + writes `.awaiting` + fires `on-notify` — VALIDATED THREE WAYS:**
  1. *Deterministic:* the captured real payload (`T-020-01/pretooluse-payload-sample.json`)
     piped through the **exact** shipped `NOTIFY_QUESTION_COMMAND` wrote
     `pane-<id>.awaiting` and fired `on-notify` with `EVENT=attention`,
     `DETAIL="Which approach should I use to build the feature?"`, `LISA_REASON=question`.
  2. *Wiring:* `lisa init` generates a valid `settings.local.json` with the
     `PreToolUse[AskUserQuestion]` matcher + the command (`.awaiting` + `LISA_REASON=question`);
     `on-notify.sample` scaffolded alongside the four existing hooks.
  3. *Live:* a real `claude --dangerously-skip-permissions -p` run (claude 2.1.185) in the
     init'd project fired the hook from the generated settings, wrote `pane-live9.awaiting`,
     and fired `on-notify` with the live-extracted question + `LISA_REASON=question`.
- **plugin consume → flag → suppress → exempt → surface — COVERED:** 11 dedicated
  `lib.rs` tests pass (`check_awaiting_signals` insert/delete, heartbeat-clear, all five
  injection-caller guards, both reclaimer exemptions, UI marker), keyed on the exact
  signal filename the hook is now proven to write.
- **(b) interactive block + (c) resume-with-heartbeat — STILL NEEDS A HUMAN:** headless
  mode dismisses the prompt (claude ran ~7s and exited), so claude's interactive *halt*
  and post-answer heartbeat can't be observed without a TUI. This is the only unverified
  link; the lisa-side response to both (suppression on the proven signal; heartbeat-clear)
  is unit-tested. Run steps 4 + 6 of the procedure above in a live `lisa loop` to close it.
