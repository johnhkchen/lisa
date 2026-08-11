# Lisa Hooks Guide

Lisa runs coding agents through your ticket board, so you don't have to approve every step by hand.

You are setting up (or repairing) Claude Code or Codex hooks for a project that uses Lisa.
The fastest path is `lisa init`, which scaffolds everything below automatically and is
safe to re-run. If you cannot run `lisa init`, the **Manual setup** section gives you
the exact files to create by hand. Read **How hooks work** first so you can tell whether
an existing setup is correct.

## How hooks work

Each native agent client fires lifecycle events as a session runs. Those events run small shell
script in `.lisa/hooks/`. Those scripts write timestamped **signal files** into
`.lisa/signals/`, keyed by the pane id Lisa exported as `$LISA_PANE_ID` when it spawned
the session. The Lisa Zellij plugin **reads and deletes** those signal files to track
each session's state and schedule the next ticket.

The flow is one-directional: **shell hooks write signals, the plugin consumes them.**
The plugin never writes signal files. Signal files are ephemeral — `.lisa/signals/` is
gitignored.

The Stop hook also forwards its lifecycle payload to `lisa capture-usage`. Successful
transcript observations append to `.lisa/<client>/captures.jsonl`. If an identified
Stop has a missing, unreadable, or empty transcript, Lisa instead appends a durable row
to `.lisa/<client>/no-captures.jsonl` carrying the pane, provider session, capture time,
and reason. Both ledgers are written under `$LISA_PROJECT` (the hook passes
`capture-usage --cwd "$LISA_PROJECT"`), so a session's usage lands beside its signals in
the project that leased the pane. The hook leaves Lisa's stderr and exit status visible,
so malformed identity or a failure to persist either outcome is not silently discarded.

## Which project a signal belongs to

A hook runs in the agent's **working directory**, and that is not the same fact as the
project its pane was leased from. An agent that steps into a second repository to read
something — ordinary work, and nothing Lisa wants to forbid — is standing somewhere else
when its next hook fires.

So every hook takes its project from **`$LISA_PROJECT`**, exported on the pane's launch
line beside `$LISA_PANE_ID`, and writes to `$LISA_PROJECT/.lisa/signals/`. It never uses
`$PWD`. On a desk that runs one loop at a time the two are the same directory, which is
why a relative `.lisa/signals` looked correct for a long time; on a desk running two
projects at once they are not, and the old behaviour split the difference badly — the
true project lost a heartbeat, and the innocent repository gained a fresh `pane-<id>.*`
file from a pane numbering it does not share. Its own launcher then refused to start a
run, correctly, on evidence that was never its own.

**A hook that cannot name its lease writes nothing.** If `$LISA_PANE_ID` or
`$LISA_PROJECT` is missing, or `$LISA_PROJECT/.lisa/` is not there, the script exits 0
without creating anything — the payload-carrying hooks (`on-stop.sh`, `on-ack.sh`) drain
stdin first so they never break the caller's turn. Silence is the right answer because
the alternative is a plausible file in a directory nobody pointed Lisa at, and nothing
downstream can tell that such a file is a stranger's. An operator's own session has
neither variable and stays silent for exactly this reason.

If you find a `.lisa/signals/` in a repository that has never run Lisa, it is a leftover
from that older behaviour. It is inert — nothing reads it but that project's own
launcher, and only while the timestamps look recent — and **Lisa will not remove it for
you**: `lisa clean` only ever touches directories inside the project you point it at, and
a stranger's tree is not one of them. Delete it by hand when you meet it:

```sh
rm -rf .lisa/signals        # in a repository that does not use Lisa
```

In a repository that *does* use Lisa, leave the directory and remove only the foreign
pane files; `lisa status` shows which panes that project actually holds.

## The five lifecycle hooks

`lisa init` scaffolds these five executable scripts into `.lisa/hooks/`. Claude
and Codex bind their supported subsets through `.claude/settings.local.json` and
`.codex/hooks.json` respectively:

| Script             | Agent lifecycle event        | Signal file written                 | Tells the plugin        |
|--------------------|-----------------------------|-------------------------------------|-------------------------|
| `on-idle.sh`       | `Notification[idle_prompt]` | `.lisa/signals/pane-<id>.idle`      | session finished work   |
| `on-stop.sh`       | `Stop`                      | `.lisa/signals/pane-<id>.stopped`   | session ready for input |
| `on-clear.sh`      | `SessionStart[clear]`       | `.lisa/signals/pane-<id>.cleared`   | context was cleared     |
| `on-heartbeat.sh`  | `PostToolUse`               | `.lisa/signals/pane-<id>.alive`     | a process is in the pane|
| `on-heartbeat.sh`  | `PostToolUse`               | `.lisa/signals/pane-<id>.heartbeat` | this attempt is working |
| `on-ack.sh`        | `UserPromptSubmit`          | `.lisa/signals/pane-<id>.ack`       | assigned prompt accepted|
| *(launch guard)*   | Codex TUI exits non-zero     | `.lisa/signals/pane-<id>.error`     | session failed          |

Each script is POSIX `sh` and writes only when the pane can name both itself
(`$LISA_PANE_ID`) and its project (`$LISA_PROJECT`), so it is inert outside a Lisa
session and never creates a directory in a repository it is only visiting — see
**Which project a signal belongs to** above. The ack hook atomically
preserves its raw JSON payload; the other lifecycle scripts write UTC timestamps. The
**heartbeat** is the liveness primitive: the plugin reuses a pane only after a stretch
of heartbeat *silence* — not on a stop/idle signal, which can fire before an agent is
truly finished.

The heartbeat hook writes **two** files because it has two separable things to say.
`.alive` says only *a process ran a tool call here*; it names nobody, so it costs
nothing to write and there is nothing in it to forge. `.heartbeat` says *this attempt
is making progress*, which moves activity clocks and lifts the question guard, so the
hook publishes it only when the session's own `$LISA_TICKET_ID`/`$LISA_ATTEMPT_ID`
byte-match the pane's lease marker — the same test `on-start.sh` applies. During a
pane recycle a lingering predecessor keeps producing `.alive` (which is how Lisa knows
not to type a launch line into a live TUI) and stops producing `.heartbeat`. Projects
scaffolded by 0.5.0-rc.2 or earlier have the older single-file hook; `lisa doctor`
reports it as behind and `lisa init` replaces it in place.

The **`.error`** signal is part of the normalized core contract
(`.heartbeat`/`.stopped`/`.error`) but is written by non-Claude adapters — native
Codex writes it when its TUI exits non-zero, while the headless JSON fallback also
emits it on `turn.failed` — not by the Claude Code hook
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
   string as the detail. It **also** writes `$LISA_PROJECT/.lisa/signals/pane-<id>.awaiting`
   so the plugin can avoid clobbering the pane while it waits on your answer. That signal
   follows the lease like every other one, and is skipped when the pane cannot name a
   project; only the `on-notify` dispatch is `test -x`-gated. The ledger row and the
   notify dispatch fall back to `$PWD` when there is no `$LISA_PROJECT`, which is what an
   operator's own session has always meant.

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
| `.lisa/hooks/on-ack.sh`           | Codex assignment ack payload hook (executable)       |
| `.lisa/hooks/on-notify.sample`    | notify hook sample (non-executable; opt in to enable)|
| `.lisa/signals/`                  | ephemeral signal files (gitignored)                  |
| `.lisa/.gitignore`                | ignores signals and provider runtime state           |
| `.claude/settings.local.json`     | binds all six hooks to Claude Code events            |
| `.codex/hooks.json`               | binds Stop, clear, heartbeat, and prompt submission  |

After `lisa init`, optionally enable `on-notify` (see **Enable it** above), then run
`lisa validate` to confirm.

## Manual setup (project not `lisa init`'d)

If you cannot run `lisa init`, create these by hand.

1. **Hook scripts.** Create `.lisa/hooks/` with the four `.sh` scripts above. Each is a
   POSIX `sh` script that opens with the same lease guard and then writes one file —
   `.idle`, `.stopped`, `.cleared`, `.heartbeat` respectively:

   ```sh
   #!/bin/sh
   [ -n "${LISA_PANE_ID:-}" ] || exit 0
   [ -n "${LISA_PROJECT:-}" ] || exit 0
   [ -d "$LISA_PROJECT/.lisa" ] || exit 0
   SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
   mkdir -p "$SIGNAL_DIR" || exit 0

   echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
   ```

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
   exists, and addresses the script through `${LISA_PROJECT:-.}` — a client runs
   hook commands in the agent's working directory, so a bare `.lisa/hooks/…` there
   means whichever directory the agent walked into, or nothing at all. The
   matcher-less `Notification` entry is the catch-all that fires
   `on-notify` for permission/attention prompts while skipping `idle_prompt` (already
   handled by `on-idle.sh`). The `PreToolUse[AskUserQuestion]` entry fires when an agent
   asks you a question:

   ```json
   {
     "hooks": {
       "PostToolUse": [
         { "hooks": [ { "type": "command", "command": "h=\"${LISA_PROJECT:-.}/.lisa/hooks/on-heartbeat.sh\"; test -x \"$h\" && \"$h\"" } ] }
       ],
       "PreToolUse": [
         { "matcher": "AskUserQuestion", "hooks": [ { "type": "command", "command": "proj=\"${LISA_PROJECT:-$PWD}\"; if [ -n \"$LISA_PANE_ID\" ] && [ -n \"$LISA_PROJECT\" ] && [ -d \"$LISA_PROJECT/.lisa\" ]; then mkdir -p \"$LISA_PROJECT/.lisa/signals\"; date -u +%Y-%m-%dT%H:%M:%SZ > \"$LISA_PROJECT/.lisa/signals/pane-$LISA_PANE_ID.awaiting\"; fi; if [ -d \"$proj/.lisa\" ]; then printf '%s\\n' '{\"event\":\"manual-intervention\",\"kind\":\"question\"}' >> \"$proj/.lisa/run-events.jsonl\"; fi; in=$(cat); q=$(printf '%s' \"$in\" | sed -n 's/.*\"question\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); [ -z \"$q\" ] && q=\"agent is asking a question\"; hdr=$(printf '%s' \"$in\" | sed -n 's/.*\"header\":[ ]*\"\\([^\"]*\\)\".*/\\1/p'); if test -x \"$proj/.lisa/hooks/on-notify\"; then printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=question LISA_PROJECT=\"$proj\" LISA_QUESTION_HEADER=\"$hdr\" \"$proj/.lisa/hooks/on-notify\" attention \"$q\"; fi" } ] }
       ],
       "Stop": [
         { "hooks": [ { "type": "command", "command": "h=\"${LISA_PROJECT:-.}/.lisa/hooks/on-stop.sh\"; test -x \"$h\" && \"$h\"" } ] }
       ],
       "SessionStart": [
         { "matcher": "clear", "hooks": [ { "type": "command", "command": "h=\"${LISA_PROJECT:-.}/.lisa/hooks/on-clear.sh\"; test -x \"$h\" && \"$h\"" } ] }
       ],
       "Notification": [
         { "matcher": "idle_prompt", "hooks": [ { "type": "command", "command": "h=\"${LISA_PROJECT:-.}/.lisa/hooks/on-idle.sh\"; test -x \"$h\" && \"$h\"" } ] },
         { "hooks": [ { "type": "command", "command": "in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) proj=\"${LISA_PROJECT:-$PWD}\"; if [ -d \"$proj/.lisa\" ]; then printf '%s\\n' '{\"event\":\"manual-intervention\",\"kind\":\"permission\"}' >> \"$proj/.lisa/run-events.jsonl\"; fi; if test -x \"$proj/.lisa/hooks/on-notify\"; then printf '%s' \"$in\" | LISA_EVENT=attention LISA_REASON=permission LISA_PROJECT=\"$proj\" \"$proj/.lisa/hooks/on-notify\" attention \"$in\"; fi ;; esac" } ] }
       ]
     }
   }
   ```

   The catch-all command is POSIX `sh` only (no `jq`, no bashisms): it exits early when
   `on-notify` is not executable, reads the payload from stdin once, skips
   `idle_prompt`, and otherwise invokes `on-notify attention "<payload>"` with
   `LISA_EVENT`/`LISA_REASON` set inline. The `PreToolUse[AskUserQuestion]` command is
   likewise POSIX `sh`: it writes the `pane-<id>.awaiting` signal whenever the pane can
   name its project (so the plugin can hold the pane while you answer), best-effort
   extracts the first question text with `sed`, and only then `test -x`-gates the
   `on-notify attention` call with `LISA_REASON=question`. A question containing an
   escaped quote degrades to a generic "agent is asking a question" detail rather than
   failing.

   Both commands address `$LISA_PROJECT` — the project the pane was leased from —
   falling back to `$PWD` only for the ledger row and the notify dispatch, which is what
   `$PWD` has always meant in an operator's own session. Neither creates anything outside
   an existing `.lisa/`, so a session reading an unrelated repository leaves nothing
   behind there and never runs *that* project's `on-notify`.

4. **Codex bindings.** Create `.codex/hooks.json` with `Stop`, `SessionStart`
   matched on `clear`, `PostToolUse`, and `UserPromptSubmit` command hooks pointing at
   `on-stop.sh`, `on-clear.sh`, `on-heartbeat.sh`, and `on-ack.sh` respectively, in
   the same `h="${LISA_PROJECT:-.}/.lisa/hooks/<script>"; test -x "$h" && "$h"` form
   as the Claude bindings. Lisa's Codex launch uses
   `--dangerously-bypass-hook-trust` for these generated definitions; project
   trust is still pre-seeded by `lisa doctor` and `lisa loop`.

## Verify

```sh
lisa validate
```

`lisa validate` confirms the hook set is wired correctly. It checks that
`.claude/settings.local.json` contains all six bindings (`idle_prompt`, the `on-notify`
catch-all, `Stop`, `SessionStart`, `PostToolUse`, and `PreToolUse[AskUserQuestion]`) and
that the six hook files exist — with the five `.sh` scripts executable and
`on-notify.sample` exempt from the executable check (it is opt-in by design). The
`PreToolUse[AskUserQuestion]` binding is an inline command with no backing script file, so
there is nothing extra to scaffold. When Codex is selected, validation also requires
`.codex/hooks.json` with Stop, clear, heartbeat, and prompt-submission bindings. Fix any
reported errors, then run `lisa loop`.
