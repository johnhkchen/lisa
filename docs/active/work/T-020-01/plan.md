# T-020-01 Plan — implementation sequence for the AskUserQuestion feature

This spike ships **no production code**; its own "implementation" is the four artifacts plus
the captured payload sample. This plan therefore sequences the **downstream implementation
tickets** (T-020-02..04) so they can be executed and verified atomically. Each step names its
verification. Anchors are current-working-tree.

## Phase 0 — the gate (this ticket): DONE

- [x] Confirm `PreToolUse[AskUserQuestion]` fires under `--dangerously-skip-permissions`
      (empirical probe; payload at `pretooluse-payload-sample.json`, `permission_mode:
      bypassPermissions`). **Verification: a captured payload exists. GO.**

## T-020-02 — hook binding + question notification

Step 1. **Interactive validation (closes the residual Q2/Q4 risk).** Hand-wire a
`PreToolUse[AskUserQuestion]` hook in a scratch project, run a *real* `lisa loop` (interactive
pane, not `-p`), force a question, and confirm: (a) the await signal is written, (b)
`on-notify attention …question…` fires, (c) after answering, a `.heartbeat` lands and the
agent resumes. **Verify: signal file + notify observed; heartbeat after answer.** If the
interactive call does *not* block (unlikely given the TUI), record it and reassess scope before
proceeding.

Step 2. Add `NOTIFY_QUESTION_COMMAND` const to `templates.rs` (beside
`NOTIFY_ATTENTION_COMMAND:110`). Writes `pane-$LISA_PANE_ID.awaiting` unconditionally;
`test -x`-gates only the `on-notify` dispatch; `sed`-extracts first question for the detail.
**Verify: `cargo test -p lisa-cli` (new sed-extraction unit test on the sample payload passes).**

Step 3. Add the 6th binding to `settings_local_json()` (`templates.rs:116`) as a *second*
PreToolUse array element with `"matcher":"AskUserQuestion"`. **Verify: `test_settings_local_json`
updated — PreToolUse has 2 entries, JSON parses, matcher present.**

Step 4. Add `ensure_hook(.., "PreToolUse", Some("AskUserQuestion"), NOTIFY_QUESTION_COMMAND)`
to `merge_hooks` (`templates.rs:296`). **Verify: new idempotency test — merge twice, count
stays 1; heartbeat entry untouched.**

Step 5. Extend `lisa validate` (`init.rs:654`, `:680-708`) to expect the new binding; update
init tests (`init.rs:955`). **Verify: `cargo test -p lisa-cli` green; `lisa validate` on a
freshly `lisa init`'d dir reports OK.**

Step 6. Document in `hooks-guide.md`: the PreToolUse row in the lifecycle table, the
`LISA_REASON=question` value (`hooks-guide.md:74`), and the manual-setup JSON
(`hooks-guide.md:180`). **Verify: `test_hooks_guide_embedded` style assertion for `question`.**

Step 7. `just check` (WASM check + full workspace tests) + `cargo clippy`. Commit.
**Verify: green.**

## T-020-03 — plugin awaiting-human flag + injection suppression

Step 8. Add `awaiting_human: HashSet<u32>` to `State` (`lib.rs:241`). **Verify: compiles;
`#[derive(Default)]` needs no init.**

Step 9. Add `check_awaiting_signals()` (model on `check_heartbeat_signals:760`); call it in
`poll_tick` before `check_idle_signals` (`lib.rs:1551-1557`). Add the
`awaiting_human.remove(&pane_id)` clear in `check_heartbeat_signals` (`lib.rs:783`).
**Verify: unit test — write a `pane-7.awaiting` file, run the scan, assert the set contains 7;
then a heartbeat for 7 clears it.**

Step 10. Add `is_pane_awaiting` helper and the `send_line_to_pane` early-return guard
(`lib.rs:268`). **Verify: unit test — mark pane awaiting, call a path that would inject, assert
no write / state unchanged.**

Step 11. Guard the four transition/timeout callers (`lib.rs:1071,1186,1245/1262,1306`) and the
two scheduler sends (`lib.rs:550/559`). **Verify: per-caller unit tests — e.g. an awaiting
Review pane past `review_timeout_secs` gets **no** finish-up prompt; clears after heartbeat then
does.** This is the core correctness test for the story.

Step 12. `just check` + clippy. Commit. **Verify: green.**

## T-020-04 — reclamation exemption + dashboard

Step 13. Add `!awaiting_human.contains(pane_id)` to the reclaim branch of
`check_session_timeouts` (`lib.rs:1425-1438`) and the stale filter of `detect_stale_threads`
(`lib.rs:1484`); keep the over-budget *warning* path active. **Verify: unit test — awaiting
pane silent past `2×stuck_threshold_secs` is NOT failed/released; a non-awaiting one still is.**

Step 14. `ui.rs` "⏸ awaiting human" marker for panes in `awaiting_human`. **Verify: render
test / manual dashboard check; awaiting pane visibly distinct.**

Step 15. `just check` + clippy. Commit.

## Testing strategy

- **Unit (native `cargo test --workspace`)** — the bulk. Pure-function and state tests, no
  zellij/claude deps (per project memory, tests run on native). Reuse `pretooluse-payload-
  sample.json` as a fixture for the sed-extraction test. New tests cluster:
  `templates.rs` (binding/merge/idempotency/extraction), `lib.rs` (awaiting set scan, clear,
  each injection guard, each reclamation exemption).
- **Integration / manual** — T-020-02 step 1 is the one genuinely manual check (interactive
  `lisa loop` blocking + answer + resume); it cannot be unit-tested because it needs a live
  interactive `claude` pane. Everything downstream is pure state logic and is unit-tested.
- **Regression guardrails** — assert the heartbeat liveness invariant is intact: an awaiting
  flag must NOT touch `last_activity_at`, so a *genuinely dead* pane that somehow has the flag
  still eventually trips stale detection only via heartbeat absence — covered by keeping the
  exemption on a *separate* set, not on the activity clock.

## Verification criteria (definition of done for the feature)

1. An agent calling `AskUserQuestion` in any phase (incl. Implement) produces an `on-notify
   attention` with `LISA_REASON=question` and a `.awaiting` signal — **no longer read as
   completion** (`lib.rs:853`).
2. While awaiting, lisa injects nothing into that pane (no `/clear`, prompt, or finish-up) and
   does not reclaim it on the timeout/stale clocks.
3. After the human answers, the next tool call's heartbeat clears the flag and normal
   scheduling resumes — within one poll cycle.
4. `just check` green; clippy clean; `lisa validate` recognizes the new binding.

## Risks / mitigations

- **Interactive blocking unverified in spike** → T-020-02 step 1 gates the rest.
- **Question text extraction fragility** (escaped quotes) → best-effort `sed`, generic
  fallback detail, raw payload still available — never a hard failure (design Q3).
- **Flag never clears (agent answers then idles silently)** → `Stop`/idle path still applies;
  pane is genuinely idle, which is the correct, safe state.
