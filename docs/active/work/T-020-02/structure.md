# T-020-02 Structure — file-level blueprint

Shapes and signatures only. Anchors are current-working-tree line numbers. Corrects the spike
structure.md where it conflicts with live code (heartbeat = PostToolUse, not PreToolUse).

## Change map

| File | Change | What |
|------|--------|------|
| `crates/lisa-cli/src/templates.rs` | modify | new `NOTIFY_QUESTION_COMMAND` const; new `PreToolUse` block in `settings_local_json()`; one `ensure_hook` call in `merge_hooks()`; tests |
| `crates/lisa-cli/src/init.rs` | modify | one new expected-binding row in `validate`; one new `validate` unit test |
| `crates/lisa-cli/data/hooks-guide.md` | modify | document `LISA_REASON=question` + the PreToolUse path; bump "five → six" binding counts |
| (none) | — | **no files created or deleted; no new modules/crates** |

`lisa-core` and `lisa-plugin` are untouched (plugin work is T-020-03/04).

## 1. `templates.rs`

### 1a. New const (next to `NOTIFY_ATTENTION_COMMAND`, ~`:110`)

```rust
/// Command for the `PreToolUse[AskUserQuestion]` hook. POSIX `sh` only (no jq,
/// no bashisms). It (1) unconditionally writes `pane-$LISA_PANE_ID.awaiting` so
/// the plugin can suppress injection while the agent is blocked on a question,
/// and (2) best-effort extracts the first question text and fires the opt-in
/// `on-notify attention` with LISA_REASON=question. The signal write is NOT
/// test-x gated (suppression must work without on-notify); only the notify is.
const NOTIFY_QUESTION_COMMAND: &str = "mkdir -p .lisa/signals; [ -n \"$LISA_PANE_ID\" ] && date -u +%Y-%m-%dT%H:%M:%SZ > \".lisa/signals/pane-$LISA_PANE_ID.awaiting\"; in=$(cat); q=$(printf '%s' \"$in\" | sed -n 's/.*\"question\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); [ -z \"$q\" ] && q=\"agent is asking a question\"; test -x .lisa/hooks/on-notify && LISA_EVENT=attention LISA_REASON=question .lisa/hooks/on-notify attention \"$q\"";
```

(Rust escaping: `\"`→`"`, `\\(`→`\(`, `\\)`→`\)`, `\\1`→`\1`. Resolves to the D3 shell line.)

### 1b. `settings_local_json()` (`:116-173`)

Add a new top-level `"PreToolUse"` key inside the `"hooks"` object (alongside `PostToolUse`,
`Stop`, `SessionStart`, `Notification`). One entry, matchered:

```json
"PreToolUse": [
  {
    "matcher": "AskUserQuestion",
    "hooks": [
      { "type": "command", "command": "<NOTIFY_QUESTION_COMMAND, JSON-escaped>" }
    ]
  }
]
```

The command literal inside the raw string is the **JSON-escaped** form of the const (every `"`
→ `\"`, every `\` → `\\`), exactly as the catch-all is embedded at `:164`.

### 1c. `merge_hooks()` (`:255-299`)

After the existing five `ensure_hook` calls, add:

```rust
ensure_hook(
    hooks_obj,
    "PreToolUse",
    Some("AskUserQuestion"),
    NOTIFY_QUESTION_COMMAND,
);
```

Matcher-based dedup → idempotent, coexists with everything else.

### 1d. Tests (`mod tests`)

- **Extend `test_settings_local_json`** (`:507-532`): assert `parsed["hooks"]["PreToolUse"]` is
  an array of len 1; its `[0]["matcher"] == "AskUserQuestion"`; and
  `[0]["hooks"][0]["command"] == NOTIFY_QUESTION_COMMAND` (parity guard, like the catch-all).
- **Extend `test_merge_hooks_empty_object`** (`:540`): assert result contains `AskUserQuestion`.
- **New `test_merge_hooks_adds_pretooluse_question`**: merge into a settings that already has
  all five hooks (use `settings_local_json()` minus PreToolUse, or just `settings_local_json()`
  and assert idempotency) → PreToolUse[AskUserQuestion] present exactly once; re-merge keeps it
  at one; other five untouched. Use a JSON-parsing count helper (avoid substring brittleness on
  the escaped command), mirroring `count_attention_commands` (`:584`).
- **New `test_notify_question_command_extracts_question`**: replicate the hook's `sed` in the
  test (run `sed` via `std::process::Command`, gated `#[cfg(unix)]`) against:
  (i) the captured `pretooluse-payload-sample.json` shape → recovers
  `"Which approach should I use to build the feature?"`;
  (ii) an escaped-quote variant → degrades (empty or truncated, never panics). Assert the
  fallback `q` would be the generic string when extraction is empty.
  - *Simplicity option:* if invoking `sed` from a test proves fragile across platforms, assert
    the regex behavior with the Rust `regex` crate against the same inputs **and** assert the
    const string literally contains the documented `sed` expression. Prefer the real `sed`
    when available; the goal is to prove the extraction contract, not reimplement it.

### 1e. New helper in `mod tests`

```rust
/// Count PreToolUse hook commands equal to NOTIFY_QUESTION_COMMAND (JSON-parsed,
/// so escaped quotes in the command don't break a substring match).
fn count_question_commands(json: &str) -> usize { /* parse hooks.PreToolUse */ }
```

## 2. `init.rs`

### 2a. `validate` expected-binding list (`:652-658`)

Add one row to the array:

```rust
("idle_prompt", "Notification[idle_prompt]"),
("on-notify",   "Notification[attention]"),
("\"Stop\"",    "Stop"),
("\"SessionStart\"", "SessionStart[clear]"),
("\"PostToolUse\"",  "PostToolUse[heartbeat]"),
("AskUserQuestion",  "PreToolUse[AskUserQuestion]"),   // NEW
```

`AskUserQuestion` is the distinctive substring (D6). No change to the hook-file existence loop
(`:683-717`) or the plan-count (`:948-961`) — no new file.

### 2b. Tests

- The happy-path validate tests use `setup_hook_infra` → `templates::settings_local_json()`,
  which now contains `AskUserQuestion`, so they pass unchanged.
- **New `test_validate_missing_pretooluse_binding`**: write a settings file that has the five
  pre-existing bindings but **not** `AskUserQuestion` → `validate` reports one error whose
  message contains `PreToolUse[AskUserQuestion]`. Mirrors the existing "missing X binding"
  tests (e.g. around `:1878-1960`).

## 3. `data/hooks-guide.md`

Edits (no structural reshuffle):
- **`LISA_REASON` row** (`:74`): list `question` alongside `idle-without-artifact` / `permission`.
- **"How it fires" section** (`:80-89`): rename to three paths; add bullet 3 — *From Claude
  Code's `PreToolUse[AskUserQuestion]` event*, which writes the `.awaiting` signal and fires
  `on-notify attention` with `LISA_REASON=question`.
- **Binding counts**: "all five hooks" → "all six hooks" (`:136`); "all five bindings"
  → "six" (`:162`); Verify list (`:200-202`) add `PreToolUse[AskUserQuestion]`.
- **Manual-setup JSON** (`:168-186`): add the `PreToolUse` block so the by-hand instructions
  match `settings_local_json()`. Add a short note that the question command writes `.awaiting`
  unconditionally and `test -x`-gates only the notify.
- Keep the "four lifecycle hooks" table (`:21-31`) as-is — the question hook is a notify path,
  not a signal-only lifecycle script.

## 4. Ordering of edits

1. `templates.rs` const + `settings_local_json` + `merge_hooks` (the source of truth).
2. `templates.rs` tests (lock the const/JSON parity + extraction + idempotency).
3. `init.rs` validate row + test.
4. `hooks-guide.md` doc.
5. `just check` (WASM check + full test suite).

No cross-file ordering hazard: `init.rs` reads `templates::settings_local_json()` at runtime,
so step 1 must precede step 3's test, which it does. `lisa-plugin` does not depend on any of
this (it only ever reads the `.awaiting` *file*, in a later ticket).

## Boundaries

All changes are in `lisa-cli`. The only cross-boundary artifact is the **signal-file contract**
(`pane-<id>.awaiting`), produced here and consumed by `lisa-plugin` in T-020-03 — the same
`pane-<id>.<ext>` convention used end-to-end today. No new shared types; `awaiting` state is
local plugin state in the later ticket, not a `lisa-core` concern.
