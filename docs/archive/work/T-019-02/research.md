# T-019-02 — Research

Scope: the CLI half of S-019 (attention notifications). Add the user-owned
`on-notify` hook template, a catch-all `Notification` Claude Code binding for
permission/attention payloads, and wire both into `lisa init` so new and existing
projects get the full hook set. Touches **only** `crates/lisa-cli/src/templates.rs`
and `crates/lisa-cli/src/init.rs`. No overlap with T-019-01 (plugin crate).

This is descriptive: what exists today, where, and how the pieces connect.

## The existing hook system

Lisa's notification plumbing is shell-hook → signal-file → WASM plugin. Hooks are
shell scripts that Claude Code invokes on lifecycle events; each writes a timestamp
into `.lisa/signals/pane-$LISA_PANE_ID.<ext>`, which the plugin polls. (Confirmed
direction: hooks write, plugin reads — never the reverse.)

Four hooks exist today, all defined as `&str` consts in `templates.rs`:

| Const (templates.rs)      | Line | Script file        | Claude event              | Signal ext   |
|---------------------------|------|--------------------|---------------------------|--------------|
| `ON_IDLE_HOOK`            | 11   | `on-idle.sh`       | `Notification[idle_prompt]` | `.idle`    |
| `ON_STOP_HOOK`            | 25   | `on-stop.sh`       | `Stop`                    | `.stopped`   |
| `ON_CLEAR_HOOK`           | 39   | `on-clear.sh`      | `SessionStart[clear]`     | `.cleared`   |
| `ON_HEARTBEAT_HOOK`       | 55   | `on-heartbeat.sh`  | `PostToolUse`             | `.heartbeat` |

Each script: `#!/bin/sh`, `mkdir -p .lisa/signals`, then `echo <ts> > pane-$LISA_PANE_ID.<ext>`
guarded by `[ -n "$LISA_PANE_ID" ]`. `LISA_GITIGNORE` (line 68) = `"signals/\n"`.

`on-heartbeat.sh` is the most recent addition (v0.2.11). Per project memory, adding
it required edits in a fixed set of spots — that edit is the checklist this ticket mirrors.

## settings.local.json generation (templates.rs)

`settings_local_json()` (74–123) returns a literal JSON string with four hook event
keys: `PostToolUse`, `Stop`, `SessionStart` (matcher `clear`), `Notification` (matcher
`idle_prompt`). Every command is `test -x <path> && <path>` — the `test -x` guard means
the entry is inert until the script is created and made executable. The `Notification`
array currently holds exactly one entry (idle_prompt), lines 108–118.

## Hook merge logic (templates.rs)

`ensure_hook()` (129–199) idempotently inserts one hook entry into a hooks-object:
- Dedup key: for entries **with** a matcher, the matcher value; for entries **without**
  a matcher, the script path extracted via `command.rsplit("&& ").next()` then matched as
  a substring against existing commands.
- If found, it upgrades old bare-path commands to the guarded form; if not, it pushes a
  new entry. This is what makes re-running `init` idempotent and also upgrades pre-v0.2.x
  settings.

`merge_hooks()` (204–243) parses existing JSON, ensures `hooks` is an object, then calls
`ensure_hook` four times (Stop, SessionStart[clear], Notification[idle_prompt],
PostToolUse), last call ending at line 240. Returns pretty-printed JSON.

Dedup implication for this ticket: a new **no-matcher** `Notification` entry is keyed by
its command substring, while the existing entry is keyed by matcher `idle_prompt`. As long
as the new command does not contain the substring `&& ` (which would truncate the extracted
script path) and does not collide with `on-idle.sh`, the two entries are distinct and will
not collapse. The new command references `on-notify`, not `on-idle.sh`, so they are distinct.

## init wiring (init.rs)

`plan_init_actions()` builds a `Vec<InitAction>` (CreateDir / CreateFile / UpdateFile / Skip):
- Hook dirs `.lisa/hooks`, `.lisa/signals` (307–318).
- **Hook scripts array** (321–326): `&[(name, const)]` of the four scripts; loop (327–350)
  emits CreateFile for missing, UpdateFile for stale (content mismatch), Skip for up-to-date.
- `.lisa/.gitignore` (352–364).
- `.claude/settings.local.json` (366–409): if present, `merge_hooks` then compare parsed
  JSON (Skip if equal, else UpdateFile); if absent, CreateFile with `settings_local_json()`.

`run_init()` (415–502) executes the plan, then a **chmod loop** (476–492, unix-only) sets
`0o755` on the four `.sh` scripts.

`validate()` (562–…) collects diagnostics. Hook-relevant checks:
- settings.local.json existence (636–643) + **expected-keys** loop (647–651) over
  `idle_prompt`, `"Stop"`, `"SessionStart"`, `"PostToolUse"` — substring `content.contains`.
- **Hook-script filenames** loop (675): the four `.sh` scripts — checks existence and, on
  unix, executable bit (`mode() & 0o111`).

## Tests that pin counts / hook sets

- `test_plan_init_actions_empty_dir` (946): `creates.len() == 17` (8 dirs + 9 files).
- `test_diagnostics_hook_structure_errors` (2271): expects 4 errors; its filter matches only
  `settings.local.json`, `on-idle.sh`, `on-stop.sh`, `on-clear.sh` (not heartbeat/notify),
  and the no-settings case yields a single settings error → unaffected by new key checks.
- `write_hook_infrastructure()` helper (1146–1169): writes settings + the four scripts and
  chmods them; used by ~15 tests that assert `validate` is clean. **Any new required file in
  validate must also be written here.**
- `run_init` full test (~1044–1090) asserts the four scripts exist + are executable.
- templates.rs tests (320–523): per-hook content tests + `settings_local_json` /
  `merge_hooks` idempotence/upgrade/dedup tests.

## Argument & environment contract (from S-019)

`on-notify <event> [detail]`; `$1` mirrors `LISA_EVENT` (`complete` | `attention`).
Shared env: `LISA_PROJECT` (abs root). complete: `LISA_TICKETS_DONE`, `LISA_DURATION_SECS`.
attention: `LISA_PANE_ID`, `LISA_TICKET`, `LISA_REASON` (`permission` | `idle-without-artifact`).
The permission path (this ticket) is fired by the new `Notification` binding directly on the
host — no plugin involvement. lisa core must never name ntfy.

## Constraints / assumptions

- POSIX `sh` only in the catch-all command and the sample: no `jq`, no bashisms.
- `on-notify` is opt-in: scaffold as `on-notify.sample` (non-executable) so the `test -x`
  guard stays inert until the user copies it to `on-notify` and `chmod +x`.
- The catch-all `Notification` payload matcher semantics for permission prompts are not
  guaranteed, so the binding must be matcher-less and must itself skip `idle_prompt` payloads
  (already covered by `on-idle.sh` + the plugin) to avoid double-handling.
- CI runs on Linux without zellij/claude; all touched logic is pure Rust + serde_json, so it
  remains testable on the native target.
