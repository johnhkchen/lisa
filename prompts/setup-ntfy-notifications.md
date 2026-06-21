# Set up ntfy.sh notifications for lisa

Give the **## Prompt** below to an agent working in a `lisa init`'d project. It teaches
itself the hook contract via `lisa hooks-guide`, then wires lisa's `on-notify` hook to
push notifications to your phone/desktop through [ntfy.sh](https://ntfy.sh) — so you get
pinged when the loop finishes or an agent needs you (e.g. asks a question).

No account or secret is needed: an ntfy "topic" is just a private channel name. Subscribe
to it in the ntfy app or at `https://ntfy.sh/<topic>`.

---

## Prompt

> Set up lisa loop notifications that reach me via ntfy.sh.
>
> 1. Run `lisa hooks-guide` and read the output. Understand the `on-notify` contract:
>    the events (`complete`, `attention`), the `LISA_*` environment variables, and the
>    full Claude payload available on **stdin** for the `question`/`permission` reasons.
> 2. Enable the hook: if `.lisa/hooks/on-notify.sample` exists, copy it to
>    `.lisa/hooks/on-notify`; otherwise create that file. `chmod +x` it.
> 3. Choose an ntfy topic. Use `lisa-<this project's name>` unless I tell you otherwise,
>    and tell me the topic + the subscribe URL (`https://ntfy.sh/<topic>`) at the end.
> 4. Write the hook (POSIX `sh`, no bashisms) to `curl` `https://ntfy.sh/<topic>` with
>    useful context:
>    - On **`attention`** (I'm needed): `Priority: high`, a `Title` naming the project
>      and ticket, body = the reason and the question/detail. Use `LISA_PROJECT`,
>      `LISA_TICKET_ID`, `LISA_REASON`, `LISA_QUESTION_HEADER`, and `$2`. For the
>      `question` reason you may read stdin (`payload=$(cat)`) to include the options.
>    - On **`complete`** (loop finished): normal priority, body = `LISA_TICKETS_DONE`
>      tickets in `LISA_DURATION_SECS`s, plus `LISA_PROJECT`.
>    - The hook must `exit 0` even if `curl` fails, so it never blocks the loop.
> 5. Show me the final hook, confirm it parses (`sh -n .lisa/hooks/on-notify`), and fire
>    a test so I can confirm a notification arrives:
>    `LISA_EVENT=attention LISA_REASON=question LISA_PROJECT="$PWD" LISA_TICKET_ID=TEST .lisa/hooks/on-notify attention "test question"`
>
> Keep the topic in the hook (it is not a secret). Do not change any other hooks.

---

## Expected result

`.lisa/hooks/on-notify` (executable) roughly like:

```sh
#!/bin/sh
# lisa -> ntfy.sh notifications. Subscribe at https://ntfy.sh/$TOPIC
TOPIC="lisa-myproject"

case "$1" in
  attention)
    title="lisa: ${LISA_TICKET_ID:-?} needs you (${LISA_REASON})"
    curl -s \
      -H "Title: $title" \
      -H "Priority: high" \
      -H "Tags: warning" \
      -d "${2:-needs attention} — $LISA_PROJECT" \
      "https://ntfy.sh/$TOPIC" >/dev/null 2>&1 ;;
  complete)
    curl -s \
      -H "Title: lisa: loop complete" \
      -H "Tags: white_check_mark" \
      -d "Done: ${LISA_TICKETS_DONE:-?} tickets in ${LISA_DURATION_SECS:-?}s — $LISA_PROJECT" \
      "https://ntfy.sh/$TOPIC" >/dev/null 2>&1 ;;
esac
exit 0
```

Then `LISA_REASON=question` pings fire when an agent calls `AskUserQuestion`,
`permission` when it needs a permission, `idle-without-artifact` when it stalls, and
`complete` when the whole loop is done — each tagged with which project and ticket.
