# T-020-02 Research — PreToolUse[AskUserQuestion] hook binding + notify

Descriptive map of the code this ticket touches. The spike (T-020-01) already mapped the
*feature* end-to-end; this narrows to the **hook/notify half** that lands in `lisa-cli`
(`templates.rs`, `init.rs`, `data/hooks-guide.md`). The plugin-side flag + suppression
(T-020-03) and timeout exemption (T-020-04) are out of scope here.

## 1. What exists — the hook system today

Lisa's hooks are **shell scripts that write signal files; the WASM plugin reads them**
(`hooks-guide.md:17`). One-directional. The CLI owns two things:

1. **Script bodies + the inline catch-all command**, all as `&str`/`&[u8]` consts in
   `crates/lisa-cli/src/templates.rs`.
2. **Scaffolding + validation** in `crates/lisa-cli/src/init.rs` (`lisa init` writes the
   files and merges settings; `lisa validate` checks them).

### Hook bindings currently generated (`templates.rs`)

`settings_local_json()` (`templates.rs:116-173`) emits a `.claude/settings.local.json` with
**five** Claude Code bindings across four event keys:

| Event key                    | matcher        | command target                          |
|------------------------------|----------------|-----------------------------------------|
| `PostToolUse`                | (none)         | `on-heartbeat.sh` (liveness primitive)  |
| `Stop`                       | (none)         | `on-stop.sh`                            |
| `SessionStart`               | `clear`        | `on-clear.sh`                           |
| `Notification`               | `idle_prompt`  | `on-idle.sh`                            |
| `Notification`               | (none)         | inline catch-all → `on-notify attention`|

**There is no `PreToolUse` binding today** — confirmed by reading `settings_local_json()`
and `merge_hooks()`; matches the ticket ("lisa binds no PreToolUse today"). The heartbeat
is **`PostToolUse`**, not PreToolUse.

> Note: the spike's `structure.md` (T-020-01 §1) says "PreToolUse already exists for the
> heartbeat (matcher-less)." That is **incorrect** — the heartbeat is `PostToolUse`. The
> consequence is favorable: the new `PreToolUse` key is created fresh with a single
> matchered entry, with no matcher-less sibling to coexist with. The Design corrects this.

### The catch-all pattern this ticket mirrors

`NOTIFY_ATTENTION_COMMAND` (`templates.rs:110`) is the template for an **inline, POSIX-`sh`,
no-file** hook command. Shape:

```sh
test -x .lisa/hooks/on-notify || exit 0          # opt-in gate
in=$(cat)                                          # read stdin payload once
case "$in" in *idle_prompt*) : ;; *) ... ;; esac   # filter + dispatch
LISA_EVENT=attention LISA_REASON=permission .lisa/hooks/on-notify attention "$in"
```

Key properties relevant to this ticket:
- It is embedded **twice**: as the `NOTIFY_ATTENTION_COMMAND` const (raw shell, Rust-escaped
  `\"`) and **literally** inside the `settings_local_json()` raw string (JSON-escaped). A
  test (`test_settings_local_json`, `templates.rs:527-531`) asserts the two stay in sync by
  parsing the JSON and `assert_eq!`-ing the command against the const.
- `merge_hooks()` (`templates.rs:255-299`) re-adds it via `ensure_hook(.., None, NOTIFY_ATTENTION_COMMAND)`.

### `ensure_hook` dedup semantics (`templates.rs:179-249`)

- **With a matcher** (`Some(m)`): dedups by matcher value within the event array
  (`templates.rs:200-203`). Re-runs are idempotent; coexists with other matchers/matcher-less
  entries in the same array.
- **Without a matcher** (`None`): dedups by command substring (the script path / last
  `&&`-segment), `templates.rs:204-215`.
- On a found entry it **upgrades** a bare-path command to the guarded form in place.

This matters: a new `PreToolUse[AskUserQuestion]` binding uses the **matcher path**, so it is
the clean, idempotent case — `ensure_hook(hooks_obj, "PreToolUse", Some("AskUserQuestion"), CMD)`.

## 2. The `on-notify` contract (S-019, reused verbatim)

`on-notify <event> [detail]`; `$1` mirrors `$LISA_EVENT`. On `attention`, the user hook sees
`LISA_PANE_ID`, `LISA_TICKET` (when known), and `LISA_REASON` — today
`permission | idle-without-artifact` (`hooks-guide.md:74`, `ON_NOTIFY_HOOK` comment
`templates.rs:92`). This ticket adds a **third reason value `question`** — no new user hook,
no new script file, just a new `LISA_REASON` the existing `on-notify` already receives. The
hook stays opt-in: a missing/non-executable `on-notify` is a silent no-op (`test -x` guard).

## 3. The captured payload (gate evidence)

`docs/active/work/T-020-01/pretooluse-payload-sample.json` is a **real** single-line payload
captured under `--dangerously-skip-permissions` (the spike's GO evidence,
[[askuserquestion-fires-pretooluse]]). Salient fields for extraction:

- `"hook_event_name":"PreToolUse"`, `"tool_name":"AskUserQuestion"`, `"permission_mode":"bypassPermissions"`.
- `"tool_input":{"questions":[{"question":"Which approach should I use to build the feature?","header":"Approach",...}]}`.

The first question text is reachable by a POSIX `sed` on the single-line JSON. Note the outer
key is **`"questions"`** (plural) and the per-item key is **`"question":`** (singular) — the
extraction must target the singular form. An escaped `\"` inside a question would truncate a
greedy-free `[^"]*` capture; the ticket explicitly accepts that, degrading to a generic detail.

## 4. `init.rs` scaffolding & validation surface

- **Scaffold loop** (`init.rs:321-330`): `hook_scripts` array of `(name, content)`. The
  `.sample` is excluded from the chmod loop. This ticket adds **no new file** (the question
  command is inline, like the catch-all) → this array is **unchanged**.
- **chmod loop** (`init.rs:332-...`, and explicit `init.rs:484` exec list): unchanged.
- **settings write** (`init.rs:371-412`): writes `templates::settings_local_json()` verbatim,
  so the new PreToolUse binding ships automatically once `settings_local_json()` includes it.
- **`validate` settings check** (`init.rs:652-667`): iterates a `(substring, label)` list of
  expected bindings and flags any missing. Currently five rows. This ticket adds a row for
  the new binding (a distinctive substring such as `AskUserQuestion`).
- **`validate` hook-file existence loop** (`init.rs:683-717`): checks the five script files
  exist (and `.sh` are executable). **Unchanged** — no new file.
- **plan-count test** (`init.rs:948-961`): asserts `creates.len() == 18` (8 dirs + 10 files).
  **Unchanged** — no new file added.
- **validate test helper** `setup_hook_infra` (`init.rs:1190-...`) writes
  `templates::settings_local_json()`, so the happy-path validate tests inherit the new binding
  and keep passing. No hand-rolled settings fixture omits it in a *pass*-expecting test.

## 5. `hooks-guide.md` (the doc, from T-019-03)

`crates/lisa-cli/data/hooks-guide.md` is the embedded guide printed by `lisa hooks-guide`
(`HOOKS_GUIDE`, `templates.rs:7`). Spots that count bindings and will need updating:
- §"four lifecycle hooks" table (`:21-31`) — the question hook is a *notify* path, not a
  signal-only lifecycle script; it belongs in the on-notify section, not this table.
- on-notify §"Environment variables" `LISA_REASON` row (`:74`) — add `question`.
- on-notify §"How it fires — two paths" (`:80-89`) — becomes three paths (add PreToolUse).
- `lisa init` scaffolds-table caption "binds all five hooks" (`:136`) and Manual-setup "all
  five bindings" (`:162`) → six.
- Verify §"all five bindings (...)" (`:200-202`) → add `PreToolUse[AskUserQuestion]`.

## 6. Constraints & assumptions

- **POSIX `sh` only** — no `jq`, no bashisms. `sed`, `cat`, `printf`, `test`, `mkdir`, `date`,
  `case` are fair game (all already used by sibling hooks).
- **`$LISA_PANE_ID` is exported** into the agent env by the plugin (`lib.rs:55`), so the
  signal filename `pane-$LISA_PANE_ID.awaiting` matches the plugin scan convention.
- **The `.awaiting` write must be unconditional** (not `test -x`-gated) so the future plugin
  suppression (T-020-03) works even when the user never enabled `on-notify`. Only the *notify
  dispatch* is `test -x`-gated.
- **Step-1 interactive gate** (AC) requires a real `lisa loop` with a human answering an
  `AskUserQuestion` in a zellij pane — cannot be exercised by unit tests or a headless agent.
  It is a manual verification, recorded in `progress.md`.
- Writing `.awaiting` now is harmless: nothing reads it until T-020-03 (an unread file the
  plugin ignores).
