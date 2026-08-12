# Lisa JSON Guide

Lisa runs coding agents through your ticket board, so you don't have to approve every step by hand.

This guide is for a program, not a person: a status strip, a dashboard, a script. `lisa status`
and `lisa validate` already work out the whole answer and then write it as prose. Add `--json` and
they hand you the answer itself instead.

    lisa status --json
    lisa validate --json

Everything else is unchanged. Without `--json` both commands print exactly the prose they always
did, and the exit status means exactly what it always meant.

## What you get

One JSON document on stdout, one line, and nothing else — no banner, no progress line, no trailing
sentence. You can pipe it without filtering.

Every document has the same envelope:

```json
{
  "schema": "lisa.cli/v1",
  "schema_version": 1,
  "lisa_version": "0.5.0",
  "command": "status",
  "ok": true,
  "error": null,
  "data": { }
}
```

- `schema` and `schema_version` — the contract this document was written to.
- `lisa_version` — which build answered.
- `command` — `status` or `validate`.
- `ok` — whether Lisa could work out an answer at all. **Not** whether the answer was good news.
- `error` — `null` when `ok` is true; otherwise `{"message": "…"}` carrying the same sentence the
  prose would have printed.
- `data` — the answer, or `null` when `ok` is false.

When a command fails you still get a document, never a bare message, so you parse one format
whatever happens.

## Exit status

Exit status keeps its existing meaning and stays authoritative. `--json` adds a body to that
contract; it never replaces it.

| Command | Exit 0 | Exit 1 |
| --- | --- | --- |
| `lisa validate --json` | every check passed | problems were found, **or** Lisa could not answer |
| `lisa status --json` | the board was reported | Lisa could not answer |

`lisa validate --json` finding problems is an *answer*: `ok` is `true`, `error` is `null`,
`data.verdict` is `"failed"`, `data.problems` lists what is wrong, and the exit status is 1. If you
only want to know "could a run start here", read the exit status and ignore the body.

## `lisa status --json`

`data` carries:

| Field | What it is |
| --- | --- |
| `ticket_dir` | Where the tickets were read from. |
| `completion_seal` | `commit` or `journal`. |
| `counts` | `{total, done, in_progress, ready, blocked}` — the numbers in the `Status:` line. |
| `critical_path_length` | The longest chain of tickets, in tickets. |
| `edge_count` | How many dependency links the board has. |
| `tickets[]` | `{id, title, status, phase, depends_on[], blocks[]}` for every ticket. |
| `waves[]` | `{index, depends_on_wave, ticket_ids[]}`. `depends_on_wave` is `null` for the first wave. |
| `ready[]` | Ticket ids that could start now, sorted. |
| `notes[]` | `{ticket_id, attempt_id, generation, summary, criterion_quote, evidence_citation}` for each unread note. |
| `waiting_on_you[]` | Each waiting ticket: `{ticket_id, remedy_owner, ask, reason, steps[], check, check_timeout_secs, origin, proposal}`. |
| `attempts[]` | `{pane_id, ticket_id, attempt_id, ticket_phase, superseded, abandoned, abandoned_reason}` — see below. |
| `token_usage` | `{tickets[], tickets_joined, tokens_in, tokens_out, not_yet_joined[]}`. |
| `run_summary` | The latest run's counts, or `null` when there is no board. |
| `config` | `{max_threads, session_timeout_secs, phase_timeouts, client, model}` — see below. |

`status` and `phase` are spelled exactly as the prose spells them — `open`, `in_progress`,
`blocked`, `review`, `done`, `cancelled` for status; `ready`, `implement`, `review`, `done` for
phase.

Two fields sound alike and are not. `counts.blocked` counts tickets whose dependencies are not
finished yet — nobody is waiting on a person. A ticket that is waiting on *you* is in
`waiting_on_you`, one entry per ticket, with the ask and the reason. If you want "how many things
need me", count `waiting_on_you`.

### What `config` says about what runs the board

`client` and `model` answer "what does this board run on", which is the question
a consumer choosing *where to send work* is really asking. They carry the same
stability promise as the rest of `config`.

- `client` is always one of the names Lisa knows — `claude`, `codex`. A board
  that names none in `.lisa.toml` still resolves to one, so this is the client
  that *would* run, not a copy of a line somebody wrote down.
- `model` is the model that board runs within that client, or `null` when it
  leaves the choice to the client's own default. `null` is an answer, not a
  missing field.

Both describe what the board is **configured** to run, so both are there on a
board that has never run anything. An individual ticket may still route itself
to another client or model in its own frontmatter; what actually ran is in the
attempt ledger, not here.

Nothing about credentials crosses this boundary — no keys, no endpoints, no
environment. The question this answers is what runs the board, not how it
authenticates.

### What `attempts` does and does not tell you

`attempts` comes from the lease markers Lisa's plugin publishes into `.lisa/signals/` when it puts
an attempt into a pane. It is Lisa's own record of the placement, not a guess reassembled from the
ledgers. Read it as: **the attempt Lisa most recently put in this pane.**

The marker is deliberately not withdrawn when a seat is released — a slow-starting session needs it
to announce itself — so an entry can outlive the attempt it names. Three fields tell you which
entries have:

- `ticket_phase` is that ticket's phase on the board right now. An entry whose `ticket_phase` is
  `done` names a seat that has finished.
- `superseded` is `true` when a *later* attempt for the same ticket holds another seat. Attempt
  numbers only go up within a ticket, so a superseded entry is an older attempt whatever else is
  true.
- `abandoned` is `true` when the run that placed the seat has stopped without withdrawing it — the
  machine swapped, the terminal was killed, the loop was quit. `abandoned_reason` is the plain
  sentence explaining how Lisa knows, and it is `null` whenever `abandoned` is `false`.

An entry that is none of `superseded`, `abandoned`, or on a `done` ticket is Lisa's best answer to
"this seat is working". Lisa's live seat table lives inside the plugin; `lisa status` is a separate
one-shot command and cannot see it, so this is the closest honest answer it can give.

`abandoned` errs toward `false`. Lisa sets it only when a running scheduler has not said it was
here for longer than the project allows, *and* nothing has stirred in the signal directory for the
same stretch. A seat Lisa is unsure about reads as working, because a seat wrongly reported free
could put a second agent on a ticket somebody is working. If you are deciding whether to start a
run, treat `abandoned` as "this one is not a reason to hold off".

Nothing clears these on its own. `lisa release-seats` prints which seats it believes are free, and
why, and removes them only when the operator adds `--release`.

Do not read `.lisa/signals/` yourself — and do not delete anything in it either. Those files are
single-consumer and the plugin deletes them as it reads them; a second reader loses that race by
design, and a second writer breaks a live pane's ability to announce itself. `attempts` exists so
you do not have to look, and `lisa release-seats` exists so you never have to reach in.

## `lisa validate --json`

`data` carries:

| Field | What it is |
| --- | --- |
| `verdict` | `passed` or `failed`. |
| `ticket_count` | Tickets read. |
| `ready_count` | Tickets that could start now. |
| `error_count` / `warning_count` | How many problems of each kind. |
| `problems[]` | `{path, category, severity, message}` — `severity` is `error` or `warning`. |
| `config` | `{max_threads, session_timeout_secs, phase_timeouts, client, model}` — see below. |

`path` names the file or place the problem is about; `message` is the reason, the same sentence the
prose prints.

## What you can rely on

1. **Every field named in this guide is stable within a `schema_version`.** Its name and its
   meaning will not change while `schema_version` stays `1`.
2. **Ignore fields you do not know.** New fields are added without changing `schema_version`, so a
   field you have never seen is not an error and must not be treated as one.
3. **A field disappearing, or changing what it means, is a breaking change** and comes with a new
   `schema_version`. Check `schema_version` before you trust a field; if it is higher than the one
   you were built for, the safe move is to fall back to the exit status, which does not change.
4. **Anything not named in this guide is not part of the contract**, whatever you can see in the
   output today.

## Two small things

`lisa status --json` reports the whole board. It is not available together with `--ticket`, which
answers a different question with a different shape; asking for both returns an error document
saying so.

If Lisa is not set up in the folder you point at, you get an error document with `ok: false` rather
than the setup message a person would see.
