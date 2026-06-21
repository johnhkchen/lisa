# T-020-02 Review — PreToolUse[AskUserQuestion] hook binding + notify

Handoff for the human reviewer. What changed, how it's tested, what's still open. This is the
hook/notify half of S-020; the plugin-side flag + suppression is T-020-03, timeout exemption +
dashboard is T-020-04 — both out of scope here and untouched.

## What changed

### `crates/lisa-cli/src/templates.rs` (source of truth)
- **New const `NOTIFY_QUESTION_COMMAND`** (beside `NOTIFY_ATTENTION_COMMAND`). POSIX `sh`,
  no `jq`/bashisms. It:
  1. `mkdir -p .lisa/signals` and, when `$LISA_PANE_ID` is set, **unconditionally** writes
     `.lisa/signals/pane-$LISA_PANE_ID.awaiting` (a UTC timestamp line).
  2. Reads the payload once (`in=$(cat)`), best-effort extracts the first question via `sed`
     (`s/.*"question":[ ]*"\([^"]*\)".*/\1/p`), falling back to
     `"agent is asking a question"`.
  3. `test -x`-gated: fires `LISA_EVENT=attention LISA_REASON=question .lisa/hooks/on-notify
     attention "$q"`.
- **`settings_local_json()`**: new top-level `"PreToolUse"` array with one
  `"matcher": "AskUserQuestion"` entry (JSON-escaped command literal). lisa had no PreToolUse
  binding before this ticket.
- **`merge_hooks()`**: one new `ensure_hook(.., "PreToolUse", Some("AskUserQuestion"),
  NOTIFY_QUESTION_COMMAND)`. Matcher-based dedup → idempotent; coexists with the matcher-less
  `PostToolUse` heartbeat (different event key).
- Doc comments on `settings_local_json`/`merge_hooks` updated (five → six bindings).

### `crates/lisa-cli/src/init.rs`
- **`validate`**: one new expected-binding row `("AskUserQuestion", "PreToolUse[AskUserQuestion]")`.
  No change to the hook-file existence loop or the chmod loop — **no new file** is scaffolded
  (the command is inline, like the Notification catch-all). Plan-count stays 18.

### `crates/lisa-cli/data/hooks-guide.md`
- `LISA_REASON` row now lists `question`.
- "How it fires" is now **three paths** (added the PreToolUse path).
- Binding counts bumped five → six in the scaffold table, Manual-setup, and Verify sections.
- Manual-setup JSON gains the `PreToolUse[AskUserQuestion]` block + an explanation of the
  unconditional-signal / gated-notify split.

## Test coverage

New/changed tests (all green; `just check` exits 0):

| Test | Asserts |
|------|---------|
| `test_settings_local_json` (extended) | PreToolUse array len 1, matcher `AskUserQuestion`, command **==** `NOTIFY_QUESTION_COMMAND` (const↔JSON parity, no drift) |
| `test_merge_hooks_empty_object` (extended) | merge from `{}` adds the question binding once |
| `test_merge_hooks_adds_pretooluse_question` (new) | merge into five-binding settings adds it once; re-merge stays at one; five legacy bindings survive |
| `test_notify_question_command_extracts_question` (new, `#[cfg(unix)]`) | runs the const's real `sed`: recovers the question from the captured-payload shape; escaped-quote variant truncates (never panics); no-question → empty → generic fallback. Also asserts the const embeds the documented `sed`, writes `.awaiting`, and sets `LISA_REASON=question` |
| `test_validate_missing_pretooluse_binding` (new) | settings without `AskUserQuestion` → exactly one `PreToolUse[AskUserQuestion]` validate error |

Regression coverage relied on: `test_validate_valid_setup` (full init still passes),
`test_plan_init_actions_empty_dir` (== 18, unchanged), `test_merge_hooks_already_complete`.

Counts: **lisa-cli 172 passed** (+3 net new), lisa-core 106, lisa-plugin 164. WASM check clean,
no clippy/compiler warnings.

### Gaps in automated coverage
- The `sed` extraction test is **unix-only** (`#[cfg(unix)]`); CI is Ubuntu so it runs there,
  but a pure-Windows run would skip it. The non-`sed` assertions (const contents) run everywhere.
- The extraction test replicates the `sed` invocation rather than executing the *whole* hook
  command (the signal write + dispatch). The full command is exercised end-to-end only by the
  manual gate below — unit-testing a `mkdir`/`date`/`test -x` shell pipeline adds little over
  asserting the const's content and the JSON parity.

## Open concerns / for the reviewer

1. **Step-1 interactive gate is NOT yet run (the AC's gate-closer).** It requires a human to
   answer an `AskUserQuestion` inside a real `lisa loop` zellij pane — an autonomous run cannot
   click it. The spike proved the hook fires under headless bypassPermissions and captured a
   real payload; the **interactive block → answer → resume** cycle (spike Q2/Q4 residual risk)
   is unverified. A runnable procedure is in `progress.md`. **Run it before T-020-03 builds on
   the `.awaiting` signal.** If blocking/resume misbehaves, stop and reassess.
2. **Greedy `sed` + escaped quotes** — a `question` with an embedded `\"` yields a partial or
   empty detail; by design it degrades to the generic message (never a hard failure). With
   multiple questions, only one (the last greedy match) is surfaced as the best-effort detail.
   Acceptable per the ticket; the user's `on-notify` can inspect more if it wants.
3. **Two embeddings of the command** (raw const + JSON literal in `settings_local_json`) — same
   pattern as the existing catch-all. Drift is caught by the parity `assert_eq!`; keep both in
   sync if either is edited.
4. **Spike doc correction** — the spike's `structure.md` mislabeled the heartbeat as a
   matcher-less `PreToolUse`; it is `PostToolUse`. This ticket's research/design/structure
   correct it. No code impact (the correction made the binding *simpler*).

## Scope boundary (intentionally deferred)
- T-020-03: plugin `awaiting_human: HashSet<u32>`, `check_awaiting_signals`, injection guards,
  heartbeat-clear. Consumes the `.awaiting` file this ticket writes.
- T-020-04: timeout-reclamation exemption for awaiting panes + `ui.rs` "awaiting human" marker.

Until T-020-03 lands, the `.awaiting` file is written but unread — harmless.
