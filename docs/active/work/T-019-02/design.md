# T-019-02 — Design

Decisions for the three deliverables, each grounded in the research: (1) the
`ON_NOTIFY_HOOK` sample template, (2) the catch-all `Notification` binding, and
(3) the `lisa init` / `validate` wiring. The guiding principle is **mirror the
`on-heartbeat.sh` addition exactly** — same set of edit sites, same idempotence
guarantees — plus the one new wrinkle that `on-notify` is a non-executable sample.

## 1. The `ON_NOTIFY_HOOK` sample

### Decision
A `#!/bin/sh` script stored in const `ON_NOTIFY_HOOK`, scaffolded to
`.lisa/hooks/on-notify.sample`. Body is a documented no-op (`exit 0` at the end)
with the dispatch logic shown as **commented** examples, including a commented
ntfy.sh block. lisa never names ntfy outside a comment.

### Rationale / rejected alternatives
- **Scaffold `on-notify` directly (live), not `.sample`** — rejected. The `test -x`
  guard pattern relies on the live path being absent until the user opts in. If we
  scaffolded a live `on-notify`, the catch-all binding would start firing a no-op on
  every permission prompt the moment `init` runs. A `.sample` keeps the system inert
  by default, matching S-019's "absent = no-op" guarantee.
- **Active dispatch logic uncommented in the sample** — rejected. A `.sample` the user
  copies should be safe to run as-is (do nothing) and obvious to customize. Real logic
  uncommented risks a half-configured curl firing, and forces a transport choice lisa
  refuses to make. Commented `case` + commented `curl` is the contract documentation.
- **Make the sample executable** — rejected. Then a stray `cp on-notify.sample on-notify`
  without the user reading it could activate it; more importantly validate would have to
  treat it like a live hook. Non-executable `.sample` is the safe default; the guide (and
  comments) tell the user `cp on-notify.sample on-notify && chmod +x on-notify`.

The sample documents the full env contract from S-019 in comments and shows the
`case "$1" in complete) … attention) … esac` dispatch plus the commented ntfy example
from the ticket verbatim.

## 2. The catch-all `Notification` binding

### Decision
Add a **second, matcher-less** entry to the `Notification` array in both
`settings_local_json()` and `merge_hooks()`. Its command, POSIX `sh` only:

```
test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case "$in" in *idle_prompt*) : ;; *) LISA_EVENT=attention LISA_REASON=permission .lisa/hooks/on-notify attention "$in" ;; esac
```

Behaviour:
- `test -x … || exit 0` — if the user has not opted in (no executable `on-notify`),
  exit immediately. Inert by default.
- `in=$(cat)` — read the JSON payload from stdin **once**.
- `case … *idle_prompt*) : ;;` — skip idle payloads; those are already handled by
  `on-idle.sh` + the plugin. No double-handling.
- otherwise — fire the user hook as `on-notify attention "<payload>"` with
  `LISA_EVENT=attention LISA_REASON=permission` set inline (the pane already exports
  `LISA_PANE_ID`).

### Rationale / rejected alternatives
- **Use the ticket's exact example with `&&` (`test -x … && in=$(cat); case …`)** —
  functionally close, but rejected for two reasons. (a) Correctness: with `&&` then `;`,
  the `case` runs unconditionally; when `on-notify` is absent, `$in` is empty and the
  `*)` arm still invokes the (missing) `on-notify`, producing a stderr error on every
  permission prompt. `|| exit 0` avoids invoking anything when not opted in and avoids
  consuming stdin needlessly. (b) Dedup safety: `ensure_hook` extracts the dedup script
  path via `command.rsplit("&& ")`. A command containing `&& ` truncates that path to the
  trailing fragment. Avoiding `&& ` makes the dedup key the whole command string, which is
  unambiguous and cannot collide with the idle entry. The ticket states the command is an
  example ("e.g."), so a corrected, equivalent form is in scope.
- **Add a `matcher` to the new entry** — rejected. The research notes permission-payload
  matcher semantics are not guaranteed across Claude Code versions; a matcher-less entry
  that filters in-script is robust. The in-script `*idle_prompt*` skip restores the
  separation a matcher would have given.
- **Set env via the script instead of inline** — the sample already defaults `LISA_EVENT`
  from `$1`, but setting `LISA_EVENT`/`LISA_REASON` inline in the binding guarantees they
  are present regardless of how the user edits the copied hook. Inline is cheap and explicit.

### Dedup correctness
`merge_hooks` will call `ensure_hook("Notification", None, <catch-all cmd>)` as a fifth
call. With matcher `None`, `ensure_hook` searches existing `Notification` entries for one
whose command contains the extracted path (the whole catch-all command, since no `&& `).
The existing `idle_prompt` entry's command (`…on-idle.sh && …on-idle.sh`) does not contain
it → no false match → new entry pushed. On re-run, the catch-all entry already present →
found → no duplicate. Both entries coexist; idempotent. A unit test will pin this.

## 3. `lisa init` / `validate` wiring

### Decisions
- **hook-scripts array** (init.rs ~321): add `("on-notify.sample", templates::ON_NOTIFY_HOOK)`.
  Flows through the same Create/Update/Skip plan logic → idempotent re-run.
- **chmod loop** (init.rs ~479): **do not** add `on-notify.sample` — it stays non-executable
  by design.
- **settings merge**: already handled by extending `settings_local_json()` + `merge_hooks()`.
- **validate expected-keys** (init.rs ~647): add a check for the attention binding. Marker
  substring `on-notify` (label `Notification[attention]`) — distinctive and present only when
  the catch-all command is wired.
- **validate filenames** (init.rs ~675): require `on-notify.sample` to **exist**, but skip the
  executable check for it (it is a non-executable sample). Implement by gating the unix exec
  check on `!script.ends_with(".sample")` within the existing loop, adding `on-notify.sample`
  to the list.

### Rationale / rejected alternatives
- **Require `on-notify` (live) in validate** — rejected. The live hook is opt-in; requiring it
  would make every fresh project fail validation. We require only the scaffolded `.sample`.
- **Separate existence-only loop for the sample** — viable but adds a second loop; gating the
  exec check on the `.sample` suffix inside the existing loop is smaller and keeps one source of
  truth for hook filenames.

### Test impact (decided up front)
- `test_plan_init_actions_empty_dir`: `creates.len()` 17 → **18** (one new file).
- `write_hook_infrastructure` helper: also write `on-notify.sample` (non-executable) so the
  ~15 clean-validate tests keep passing now that validate requires it.
- `test_diagnostics_hook_structure_errors` (expects 4): unaffected — its filter does not match
  `on-notify.sample`, and the no-settings branch yields a single settings error regardless of
  the new key check.
- New tests: `ON_NOTIFY_HOOK` content; `merge_hooks` catch-all + idle coexistence & idempotence;
  `settings_local_json` contains the attention binding; init plans the sample as a new file.

## Out of scope (per S-019)
No scheduler/auto-advance changes, no plugin-crate edits, no ntfy bundling. Plugin-side
`complete`/`attention` firing is T-019-01; the hooks-guide is T-019-03.
