---
id: S-019
title: attention-notifications
status: open
---

## Attention Notifications & Hook Enablement

Give the operator a push notification at the two moments a lisa loop actually
wants a human — when the loop **finishes all its work**, and when an agent
**needs human input** (a permission prompt, or stalling/asking instead of making
progress). Today the loop runs unattended on a variable 10–45 minute timer with
no way to know it has completed or gotten stuck, so the human has to babysit it.

The notification mechanism must stay project-owned and dependency-free: lisa
fires a single user-owned hook script and never references ntfy.sh (or any
service) directly. If the script is absent, nothing happens.

### Motivation

`lisa loop` is designed to run without supervision, but the operator currently
has no signal for "come back and look." The result is either constant babysitting
or long idle gaps after the work is already done. A notification on completion and
on "needs input" closes that gap while keeping lisa's zero-dependency posture.

### Design summary (decided)

- **One user hook**: `.lisa/hooks/on-notify`, invoked as `on-notify <event> [detail]`.
  `$LISA_EVENT` is the discriminator (`complete` | `attention`). `test -x` guarded;
  absent = no-op. Scaffolded by `lisa init` as `on-notify.sample` with a commented
  ntfy.sh example. lisa never names ntfy.

- **Two fire paths into the same hook:**

  | Event | Source | Mechanism |
  |---|---|---|
  | `complete` | Plugin — DAG drained (`lib.rs:1548`, the `terminated` transition) | Zellij `run_command` → `on-notify complete` |
  | `attention` (permission — Tier 1) | New Claude Code `Notification` hook (runs on host) | Hook calls `on-notify attention "<msg>"` directly; no plugin involvement |
  | `attention` (idle-without-artifact — Tier 2/3) | Plugin — `IdleWithoutArtifact` (`lib.rs:879`) | `run_command` → `on-notify attention "<ticket> stalled in <phase>"` |

- **Plain clarifying questions (Tier 3)** surface in practice as *idle without the
  expected artifact*, which is the same condition as Tier 2 — so they are caught by
  the Tier 2 path everywhere except the **Implement** phase, where idle alone is
  treated as completion (`lib.rs:765`). The Implement-phase question case is a known
  gap; catching it requires changing the auto-advance logic, which is **explicitly
  out of scope** here (notify-only — no scheduler hot-path changes).

### Environment / argument contract (shared by all tickets)

`on-notify <event> [detail]`, plus environment variables:

- `LISA_EVENT` — `complete` | `attention` (mirrors `$1`)
- `LISA_PROJECT` — absolute project root (scripts may `cd "$LISA_PROJECT"`)
- complete: `LISA_TICKETS_DONE`, `LISA_DURATION_SECS`
- attention: `LISA_PANE_ID`, `LISA_TICKET` (when known), `LISA_REASON`
  (`permission` | `idle-without-artifact`)

### Scope

- Plugin fires `complete` and `attention` notifications via `run_command`, with
  per-pane debounce so a 60s-repeating `idle_prompt` doesn't re-ping (T-019-01).
- `on-notify` hook template, a catch-all `Notification` Claude hook binding for
  permission/attention payloads, and `lisa init` scaffolding + merge for both the
  new hook and the existing four (T-019-02).
- `lisa hooks-guide` command: an agent-facing guide for setting up a project's
  hooks, including `on-notify` customization (T-019-03).

### Out of scope

- Any change to idle auto-advance / `send_line_to_pane` suppression (the
  "awaiting human" correctness fix). Deferred.
- Catching Implement-phase clarifying questions.
- Bundling ntfy.sh or any notification transport into lisa.

### Tickets

- **T-019-01** — Plugin: fire `complete` + `attention` notifications via `run_command`
- **T-019-02** — `on-notify` hook template, attention Notification binding, `init` scaffolding
- **T-019-03** — `lisa hooks-guide` command + embedded hooks guide
