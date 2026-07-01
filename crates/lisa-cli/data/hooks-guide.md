# Lisa Hooks Guide

You are setting up (or repairing) Claude Code hooks for a project that uses Lisa.
The fastest path is `lisa init`, which scaffolds everything below automatically and is
safe to re-run. If you cannot run `lisa init`, the **Manual setup** section gives you
the exact files to create by hand. Read **How hooks work** first so you can tell whether
an existing setup is correct.

## How hooks work

Claude Code fires lifecycle events as a session runs. Each event runs a small shell
script in `.lisa/hooks/`. Those scripts write timestamped **signal files** into
`.lisa/signals/`, keyed by the pane id Lisa exported as `$LISA_PANE_ID` when it spawned
the session. The Lisa Zellij plugin **reads and deletes** those signal files to track
each session's state and schedule the next ticket.

The flow is one-directional: **shell hooks write signals, the plugin consumes them.**
The plugin never writes signal files. Signal files are ephemeral — `.lisa/signals/` is
gitignored.

## The four lifecycle hooks

`lisa init` scaffolds these four executable scripts into `.lisa/hooks/` and binds each
to a Claude Code event in `.claude/settings.local.json`:

| Script             | Claude Code event           | Signal file written                 | Tells the plugin        |
|--------------------|-----------------------------|-------------------------------------|-------------------------|
| `on-idle.sh`       | `Notification[idle_prompt]` | `.lisa/signals/pane-<id>.idle`      | session finished work   |
| `on-stop.sh`       | `Stop`                      | `.lisa/signals/pane-<id>.stopped`   | session ready for input |
| `on-clear.sh`      | `SessionStart[clear]`       | `.lisa/signals/pane-<id>.cleared`   | context was cleared     |
| `on-heartbeat.sh`  | `PostToolUse`               | `.lisa/signals/pane-<id>.heartbeat` | session actively working|
| *(adapter)*        | Codex `turn.failed` / exit≠0| `.lisa/signals/pane-<id>.error`     | session failed          |

Each script is POSIX `sh`, does `mkdir -p .lisa/signals`, and writes a UTC timestamp
only when `$LISA_PANE_ID` is set (so it is inert outside a Lisa session). The
**heartbeat** is the liveness primitive: the plugin reuses a pane only after a stretch
of heartbeat *silence* — not on a stop/idle signal, which can fire before an agent is
truly finished.

The **`.error`** signal is part of the normalized core contract
(`.heartbeat`/`.stopped`/`.error`) but is written by non-Claude *adapters* — the Codex
wrapper emits it on `turn.failed` or a non-zero exit — not by the Claude Code hook
scripts above (Claude sessions have no `.error` emitter today). On consuming it the
plugin **fails the thread and releases its slot immediately** — surfacing a `✗ FAILED`
alert and freeing the ticket for retry — rather than waiting for the silence clock to
reclaim the pane ~40 minutes later. As with every signal, presence is what matters; any
body (the wrapper may write the error message for human debugging) is ignored. An
`.error` for an idle or unknown pane is consumed harmlessly.

## The `on-notify` hook (attention & completion notifications)

`on-notify` is a **user-owned** hook Lisa calls to notify you out-of-band — when the
whole loop finishes, or when a session needs your attention (a permission prompt, or an
agent that went idle without producing its phase artifact). Lisa scaffolds it as a
**non-executable** sample so it stays an inert no-op until you opt in.

### Contract

```
on-notify <event> [detail]      # $1 mirrors $LISA_EVENT; $2 is a human-readable detail
```

### Environment variables

Provided on **every** event:

| Variable       | Meaning                                                        |
|----------------|---------------------------------------------------------------|
| `LISA_EVENT`   | `complete` or `attention`                                     |
| `LISA_PROJECT` | absolute project root (you may `cd "$LISA_PROJECT"`)          |

On `complete` (the loop finished):

| Variable             | Meaning                                  |
|----------------------|------------------------------------------|
| `LISA_TICKETS_DONE`  | number of tickets completed              |
| `LISA_DURATION_SECS` | loop wall-clock duration, when tracked   |

On `attention` (a session needs you):

| Variable               | Meaning                                                       |
|------------------------|---------------------------------------------------------------|
| `LISA_PANE_ID`         | the originating pane                                          |
| `LISA_TICKET_ID`       | ticket the agent is working on, when known                   |
| `LISA_REASON`          | `idle-without-artifact` (stalled agent), `permission` (prompt), or `question` (agent asked you a question via `AskUserQuestion`) |
| `LISA_QUESTION_HEADER` | short label of the question (`question` reason only)         |

For the `question` and `permission` reasons, the **full Claude Code hook JSON is
piped to `on-notify` on stdin**, so you can extract anything (all questions, their
options, `session_id`, `cwd`, …) with `sed`/`jq`: `payload=$(cat)`. `LISA_PROJECT`
plus `LISA_TICKET_ID` tell you *which loop and ticket* the notification came from —
useful when several loops run at once.

(The plugin also sets `LISA_HOOK` to the resolved hook path for its own `test -x`
guard; you can ignore it.)

### How it fires — three paths

1. **From the plugin**, via Zellij's `run_command`: `complete` when the loop finishes,
   and `attention` with `LISA_REASON=idle-without-artifact` when an agent stalls. These
   are debounced per pane so a repeating idle prompt does not spam you.
2. **From Claude Code's `Notification` event**, via the catch-all hook in
   `.claude/settings.local.json`: this catches permission prompts and fires
   `on-notify attention` with `LISA_REASON=permission`.
3. **From Claude Code's `PreToolUse[AskUserQuestion]` event**: when an agent calls the
   `AskUserQuestion` tool (it needs a decision from you), this binding fires
   `on-notify attention` with `LISA_REASON=question` and a best-effort first-question
   string as the detail. It **also** writes `.lisa/signals/pane-<id>.awaiting` so the
   plugin can avoid clobbering the pane while it waits on your answer. The signal write
   is unconditional; only the `on-notify` dispatch is `test -x`-gated.

A missing or non-executable `on-notify` is always a silent no-op — all three paths guard
the notify dispatch with `test -x`.

### Enable it

The sample is scaffolded but disabled. Turn it on by copying and making it executable:

```sh
cp .lisa/hooks/on-notify.sample .lisa/hooks/on-notify
chmod +x .lisa/hooks/on-notify
```

Then edit `.lisa/hooks/on-notify` to do something. Example using ntfy.sh:

```sh
#!/bin/sh
case "$1" in
  complete)  msg="lisa [$LISA_PROJECT] done: $LISA_TICKETS_DONE tickets in ${LISA_DURATION_SECS}s" ;;
  attention) msg="lisa [$LISA_PROJECT] ${LISA_TICKET_ID:-?} needs you (${LISA_REASON}): $2" ;;
esac
curl -s -d "$msg" ntfy.sh/your-topic-here
```

**Lisa never depends on ntfy or any transport.** The `on-notify` hook is entirely
project-owned: ntfy.sh above is just an example. Replace it with email, a Slack
webhook, a desktop notification, or anything else — Lisa only invokes the script with
the event and environment above.

## Setting up with `lisa init` (recommended)

From the project root:

```sh
lisa init
```

This scaffolds the full hook set and is idempotent (safe to re-run — it skips unchanged
files and only writes what is missing or stale):

| Path                              | Purpose                                              |
|-----------------------------------|------------------------------------------------------|
| `.lisa/hooks/on-idle.sh`          | idle signal hook (executable)                        |
| `.lisa/hooks/on-stop.sh`          | stop signal hook (executable)                        |
| `.lisa/hooks/on-clear.sh`         | clear signal hook (executable)                       |
| `.lisa/hooks/on-heartbeat.sh`     | heartbeat signal hook (executable)                   |
| `.lisa/hooks/on-notify.sample`    | notify hook sample (non-executable; opt in to enable)|
| `.lisa/signals/`                  | ephemeral signal files (gitignored)                  |
| `.lisa/.gitignore`                | ignores `signals/`                                   |
| `.claude/settings.local.json`     | binds all six hooks to Claude Code events            |

After `lisa init`, optionally enable `on-notify` (see **Enable it** above), then run
`lisa validate` to confirm.

## Manual setup (project not `lisa init`'d)

If you cannot run `lisa init`, create these by hand.

1. **Hook scripts.** Create `.lisa/hooks/` with the four `.sh` scripts above (each a
   POSIX `sh` script that `mkdir -p .lisa/signals` and writes
   `"$(date -u +%Y-%m-%dT%H:%M:%SZ)" > ".lisa/signals/pane-$LISA_PANE_ID.<ext>"` when
   `$LISA_PANE_ID` is set — `.idle`, `.stopped`, `.cleared`, `.heartbeat` respectively).
   Make them executable:
   ```sh
   chmod +x .lisa/hooks/on-idle.sh .lisa/hooks/on-stop.sh \
            .lisa/hooks/on-clear.sh .lisa/hooks/on-heartbeat.sh
   ```
   Optionally add `.lisa/hooks/on-notify.sample` (leave it non-executable).

2. **Signals dir + gitignore.**
   ```sh
   mkdir -p .lisa/signals
   printf 'signals/\n' > .lisa/.gitignore
   ```

3. **Claude Code bindings.** Create `.claude/settings.local.json` with all six
   bindings. Each command is `test -x`-guarded so it stays silent until the script
   exists. The matcher-less `Notification` entry is the catch-all that fires
   `on-notify` for permission/attention prompts while skipping `idle_prompt` (already
   handled by `on-idle.sh`). The `PreToolUse[AskUserQuestion]` entry fires when an agent
   asks you a question:

   ```json
   {
     "hooks": {
       "PostToolUse": [
         { "hooks": [ { "type": "command", "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh" } ] }
       ],
       "PreToolUse": [
         { "matcher": "AskUserQuestion", "hooks": [ { "type": "command", "command": "mkdir -p .lisa/signals; [ -n \"$LISA_PANE_ID\" ] && date -u +%Y-%m-%dT%H:%M:%SZ > \".lisa/signals/pane-$LISA_PANE_ID.awaiting\"; in=$(cat); q=$(printf '%s' \"$in\" | sed -n 's/.*\"question\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); [ -z \"$q\" ] && q=\"agent is asking a question\"; hdr=$(printf '%s' \"$in\" | sed -n 's/.*\"header\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); test -x .lisa/hooks/on-notify && printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=question LISA_PROJECT=\"$PWD\" LISA_QUESTION_HEADER=\"$hdr\" .lisa/hooks/on-notify attention \"$q\"" } ] }
       ],
       "Stop": [
         { "hooks": [ { "type": "command", "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh" } ] }
       ],
       "SessionStart": [
         { "matcher": "clear", "hooks": [ { "type": "command", "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh" } ] }
       ],
       "Notification": [
         { "matcher": "idle_prompt", "hooks": [ { "type": "command", "command": "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh" } ] },
         { "hooks": [ { "type": "command", "command": "test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=permission LISA_PROJECT=\"$PWD\" .lisa/hooks/on-notify attention \"$in\" ;; esac" } ] }
       ]
     }
   }
   ```

   The catch-all command is POSIX `sh` only (no `jq`, no bashisms): it exits early when
   `on-notify` is not executable, reads the payload from stdin once, skips
   `idle_prompt`, and otherwise invokes `on-notify attention "<payload>"` with
   `LISA_EVENT`/`LISA_REASON` set inline. The `PreToolUse[AskUserQuestion]` command is
   likewise POSIX `sh`: it **always** writes the `pane-<id>.awaiting` signal (so the
   plugin can hold the pane while you answer), best-effort extracts the first question
   text with `sed`, and only then `test -x`-gates the `on-notify attention` call with
   `LISA_REASON=question`. A question containing an escaped quote degrades to a generic
   "agent is asking a question" detail rather than failing.

## Verify

```sh
lisa validate
```

`lisa validate` confirms the hook set is wired correctly. It checks that
`.claude/settings.local.json` contains all six bindings (`idle_prompt`, the `on-notify`
catch-all, `Stop`, `SessionStart`, `PostToolUse`, and `PreToolUse[AskUserQuestion]`) and
that the five hook files exist — with the four `.sh` scripts executable and
`on-notify.sample` exempt from the executable check (it is opt-in by design). The
`PreToolUse[AskUserQuestion]` binding is an inline command with no backing script file, so
there is nothing extra to scaffold. Fix any reported errors, then run `lisa loop`.
