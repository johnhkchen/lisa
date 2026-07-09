# T-020-02 Design — PreToolUse[AskUserQuestion] hook binding + notify

The spike (T-020-01) already chose the mechanism: a `PreToolUse[AskUserQuestion]` Claude Code
hook writes `.lisa/signals/pane-<id>.awaiting` **and** fires `on-notify attention` with
`LISA_REASON=question`. This document fixes the concrete decisions for the `lisa-cli` half and
records what was rejected. It is grounded in the Research map; where the spike's structure.md
disagreed with the live code, the live code wins.

## D1 — Inline command vs. a new `.sh` file? **Inline (no new file).**

The catch-all `Notification` attention hook is an **inline command string** with no backing
file (`NOTIFY_ATTENTION_COMMAND`, `templates.rs:110`). The question hook is the same shape — a
short POSIX `sh` one-liner that writes a signal and conditionally dispatches `on-notify`.

- **Chosen: inline command const** `NOTIFY_QUESTION_COMMAND`, mirroring `NOTIFY_ATTENTION_COMMAND`.
  No file to scaffold, no chmod, no addition to the `hook_scripts` array, no change to the
  plan-count (stays 18) or the validate file-existence loop.
- **Rejected: a new `on-pretool-ask.sh` script.** It would add a file to scaffold + chmod +
  validate + a stale-content path, and a test surface — all for a command that fits on one
  line. The ticket explicitly says "if a separate `.sh` file" (conditional), and the spike's
  change map says "no new scaffolded file (the command is inline, reuses `on-notify`)". More
  files = more drift surface for zero benefit.

Consequence: `init.rs` changes are **validation-only**; the `hook_scripts` array, chmod loop,
and plan-count test are untouched.

## D2 — Where does the binding live, and how does it dedup? **New `PreToolUse` key, matchered entry.**

Lisa has **no `PreToolUse` binding today** (Research §1; the heartbeat is `PostToolUse`). So:

- `settings_local_json()` gains a brand-new top-level `"PreToolUse"` array with a **single**
  entry carrying `"matcher": "AskUserQuestion"`. No matcher-less sibling exists in that array.
- `merge_hooks()` gains one `ensure_hook(hooks_obj, "PreToolUse", Some("AskUserQuestion"), NOTIFY_QUESTION_COMMAND)`.

Because the entry **has a matcher**, `ensure_hook` dedups by matcher value
(`templates.rs:200-203`) — the clean, idempotent path. The ticket's worry about "colliding
with the future matcher-less heartbeat PostToolUse" does **not** apply: (a) the heartbeat is a
different event key (`PostToolUse`), and (b) our entry is matchered, so even a future
matcher-less `PreToolUse` would be a *distinct* array element. No command-substring dedup
needed.

- **Rejected: matcher-less PreToolUse with substring dedup** (as the ticket text hedged). It
  would fire the question command on **every** tool call and force a `case`-filter on
  `tool_name` — strictly worse than letting Claude Code's matcher do the filtering. The
  matcher is the idiomatic and cheaper mechanism.

## D3 — The command body. **Unconditional signal write; best-effort, `test -x`-gated notify.**

Final command (POSIX `sh`, single line; `;`-separated):

```sh
mkdir -p .lisa/signals; [ -n "$LISA_PANE_ID" ] && date -u +%Y-%m-%dT%H:%M:%SZ > ".lisa/signals/pane-$LISA_PANE_ID.awaiting"; in=$(cat); q=$(printf '%s' "$in" | sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p'); [ -z "$q" ] && q="agent is asking a question"; test -x .lisa/hooks/on-notify && LISA_EVENT=attention LISA_REASON=question .lisa/hooks/on-notify attention "$q"
```

Design points:
1. **Signal write is unconditional** (only gated on `$LISA_PANE_ID` being set, like every
   sibling hook). It must work even when `on-notify` is not enabled, because the plugin
   suppression (T-020-03) keys off this file, not off the notify. Only the **notify dispatch**
   carries the `test -x` opt-in gate.
2. **`mkdir -p .lisa/signals` first** — mirrors the four signal scripts (`ON_*_HOOK`), so the
   dir exists even on a fresh checkout where the plugin hasn't created it.
3. **`in=$(cat)` reads stdin exactly once** — same as the catch-all. PreToolUse delivers the
   payload on stdin.
4. **Best-effort extraction** of the first question via `sed` (verbatim from the ticket / spike
   Q3). Targets the singular `"question":` key. Greedy `.*` resolves to the last match; with a
   single question that is the only one. On miss (empty `q`), degrade to
   `"agent is asking a question"` — never a hard failure.
5. **`LISA_EVENT=attention LISA_REASON=question`** set inline on the `on-notify` call, matching
   the catch-all's inline-env style. `attention` is the event; `question` is the new reason.
6. **`$LISA_TICKET` / `$LISA_PANE_ID`** are already exported by the plugin into the agent env;
   the hook does not need to set them — `on-notify` sees them as the contract promises.

Ordering note: the signal write runs **before** `in=$(cat)`. That is fine — `$LISA_PANE_ID`
comes from the environment, not stdin, so the await signal does not depend on reading the
payload. Reading stdin after is still correct (it has not been consumed yet).

## D4 — Escaped-quote degradation. **Accepted, by design.**

A `question` value containing `\"` truncates the greedy-free `[^"]*` capture at the embedded
quote, yielding a partial or empty `q`. Per the ticket and spike Q3 this is **acceptable**:
empty → generic message; partial → a shorter-but-harmless detail. We do **not** add escape
handling in `sh` (that path leads to a `jq` dependency we forbid). The `on-notify` user hook
still receives a valid call; only the human-readable detail is degraded. A unit test asserts
both: clean extraction on the captured sample, and graceful degradation on an escaped-quote
variant.

## D5 — Keeping the two embeddings in sync. **Const + JSON-literal + a parity test.**

Like `NOTIFY_ATTENTION_COMMAND`, the command is embedded twice: the raw-shell const
`NOTIFY_QUESTION_COMMAND` (used by `merge_hooks` and tests) and the JSON-escaped literal inside
`settings_local_json()`'s raw string. To prevent drift, extend `test_settings_local_json` to
parse the generated JSON and `assert_eq!` the PreToolUse command against the const — exactly
the guard that already exists for the catch-all (`templates.rs:530-531`).

- **Rejected: build the JSON with `serde_json` from the const** (eliminating the duplicate).
  `settings_local_json()` is a hand-written raw string for *all* hooks; rewriting it to a
  programmatic builder is out of scope and would churn every existing assertion. The parity
  test is the established, low-risk pattern.

## D6 — Validation surface. **One new expected-binding row; nothing else.**

`validate` (`init.rs:652-667`) gains one row: a distinctive substring proving the binding is
present. **Chosen substring: `AskUserQuestion`** (label `PreToolUse[AskUserQuestion]`) — more
specific than `"PreToolUse"` (which a future unrelated PreToolUse binding could also satisfy),
and it directly names the matcher that makes this *the question hook*. The hook-file loop and
plan-count are untouched (D1).

## D7 — Step-1 interactive gate (AC, gate-closer). **Manual; documented, not automated.**

The first AC is to validate under a **real `lisa loop`** that an agent's `AskUserQuestion`
(a) fires the PreToolUse hook, (b) blocks the pane until answered, (c) resumes with a
`PostToolUse` heartbeat. This is inherently interactive (a human answers a TUI prompt in a
zellij pane) and cannot be exercised by `cargo test` or a headless agent. The spike already
proved (a) under headless bypassPermissions ([[askuserquestion-fires-pretooluse]]); (b)/(c)
are the residual risk. **Decision:** ship the code + automated tests, and record in
`progress.md` an explicit, runnable manual procedure plus the residual-risk status, flagged
for the human reviewer to execute before T-020-03 leans on the signal. This honors "record the
result in progress.md" while being honest that an autonomous run cannot perform the click.

## Decision summary

| # | Decision | Rejected alternative |
|---|----------|----------------------|
| D1 | Inline command const, no new file | A scaffolded `.sh` script |
| D2 | New `PreToolUse` key, matchered entry, matcher-dedup | Matcher-less + substring dedup + `case` filter |
| D3 | Unconditional signal write; `test -x`-gated best-effort notify | Gating the signal on `on-notify` |
| D4 | Accept escaped-quote degradation | `jq`/escape handling in `sh` |
| D5 | Const + JSON literal + parity test | Programmatic JSON builder |
| D6 | One `AskUserQuestion` validate row | Checking `"PreToolUse"` substring |
| D7 | Manual step-1 gate, documented in progress.md | Pretending tests cover it |
