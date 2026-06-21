# T-019-01 Design — Plugin notification emit

## Decision summary

Add a single host-free builder `build_notify_command()` that produces the
`(argv, env)` for invoking `.lisa/hooks/on-notify`, plus a thin `fire_notify()`
wrapper that calls `run_command_with_env_variables_and_cwd`. Wire two call sites
(`complete`, `attention`), add the `RunCommands` permission + `RunCommandResult`
subscription, capture the absolute host root via `get_plugin_ids()`, and add a
per-pane debounce set cleared on heartbeat. All changes confined to `lib.rs`.

## Decision 1 — Which `run_command` API

**Chosen: `run_command_with_env_variables_and_cwd(cmd, env, cwd, context)`.**

Rationale: the ticket suggests `run_command_with_env_variables`, but that helper
does not exist in zellij-tile 0.43 (verified in Research). The only env-capable
function is the `_and_cwd` form. We pass an explicit cwd anyway, which is
*better* than relying on the implicit `.` default — it makes the relative-path
question moot and supports hooks that `cd "$LISA_PROJECT"`.

Rejected: `run_command` (no env). It cannot set the contract's env vars; we would
have to bake `KEY=val` assignments into the `sh -c` string and hand-quote dynamic
values (ticket ids, paths) — fragile and harder to unit-test.

## Decision 2 — Source of the absolute host root

**Chosen: `get_plugin_ids().initial_cwd`, captured once in `load()` into
`self.project_root`.**

Rationale: commands run on the host where `/host` is meaningless. `initial_cwd`
is the real absolute project root. With it we build an absolute hook path
(`project_root.join(".lisa/hooks/on-notify")`), an absolute `LISA_PROJECT`, and an
absolute cwd — no relative-path fragility, no dependence on Zellij's implicit
cwd.

Rejected: `strip_host_prefix(host.join(...))` (ticket's suggestion). It yields a
*relative* path that only works if cwd happens to be the root. Correct in the
happy path but strictly weaker than the absolute form, and it cannot supply an
absolute `LISA_PROJECT`. We keep `strip_host_prefix` untouched (still used
elsewhere) but do not rely on it here.

Rejected: a new config key for the project path. `initial_cwd` already provides
it with zero CLI/layout changes.

## Decision 3 — Guard command shape

**Chosen:** argv =
`["sh", "-c", "if [ -x \"$LISA_HOOK\" ]; then \"$LISA_HOOK\" \"$1\" \"$2\"; fi", "sh", <event>, <detail>]`
with `LISA_HOOK` supplied via the env map.

- `$0=sh`, `$1=<event>`, `$2=<detail>` — passes event + detail positionally,
  matching the `on-notify <event> [detail]` contract.
- The `if [ -x ]` form **exits 0 when the hook is absent or non-executable**, so a
  missing hook is a true silent no-op. This is the key reason to prefer `if` over
  the ticket's `test -x "$LISA_HOOK" && ...` example: the `&&` form exits **1**
  when the test fails, which would surface as a spurious failure in the activity
  log.
- POSIX `sh` only; no bashisms, no `jq`. Consistent with T-019-02's hook posture.

Rejected: `test -x X && X "$1" "$2"` (ticket example) — non-zero exit on absent
hook. Rejected: `test -x X && ... || true` — works but the `if` form reads
clearer and preserves the real hook exit code (so genuine hook failures still
report non-zero).

## Decision 4 — Result handling & noise

**Chosen:** subscribe to `RunCommandResult`; tag every notify call with
`context = {"lisa_notify": <event>}`. In `update()`, match
`Event::RunCommandResult(exit, _, _, context)`; if `context` carries
`lisa_notify`, log:
- `Some(0)` → `ActivityEvent::Info` ("on-notify <event> ok").
- `Some(n)` (n≠0) or `None` → `ActivityEvent::Warning` with the exit code.

Rationale: keeps hook failures visible (the ticket's explicit goal) while a
healthy/absent hook produces at most one low-volume Info per event. Filtering by
the `lisa_notify` context key means we never misattribute some future
`run_command` result.

Rejected: logging nothing on success — would hide the "hook fired" signal that is
useful when debugging a user's notification setup. Rejected: a new dedicated
`ActivityEvent` variant — `Info`/`Warning` already exist and render; adding a
variant touches `lisa-core` and `format_activity_event` for no benefit.

## Decision 5 — Attention debounce

**Chosen:** `notified_attention: HashSet<u32>` (pane ids). In the
`IdleWithoutArtifact` branch: if `pane_id` is known and **not** already in the
set, insert it and fire `attention`; if already present, skip the fire (still push
`idle_alerts` / `Warning` as today — UI behavior unchanged). Clear a pane's entry
in `check_heartbeat_signals()` right after `bump_pane_activity(pane_id)`.

Rationale: idle prompts repeat ~60s; the set prevents re-pinging while a pane
stays stalled. Heartbeats prove the agent resumed real work, so clearing there
lets a resumed-then-re-stalled agent notify again. We must **not** clear in
`bump_pane_activity` (it runs on the idle signal itself, which would defeat the
debounce every cycle).

Legacy `{ticket}.idle` signals carry no pane id; for those we keep today's
alert/warning behavior and skip the `run_command` notify (no pane id to debounce
or export). Acceptable — legacy hooks are deprecated and rare.

Rejected: per-slot bool flag — a `HashSet<u32>` is simpler, survives slot
reordering, and is trivially Default/testable.

## Decision 6 — `LISA_DURATION_SECS`

**Chosen:** add `loop_started_at: Option<SystemTime>`, set in `load()`. On
`complete`, if `Some`, include `LISA_DURATION_SECS = now − start`; if `None`
(e.g. in tests), omit it. `SystemTime::now()` is already used throughout the
plugin (WASM-safe here), so this is low-risk and completes the contract rather
than deferring.

## Testability

`build_notify_command(project_root, event, detail, extra_env)` is a pure
associated fn returning `(Vec<String>, BTreeMap<String,String>)` — no host calls.
Unit tests assert argv shape and env contents for both events. The debounce
`HashSet` add/skip/clear logic is tested directly on `State`. `fire_notify` and
the host call are compiled but not exercised natively (same pattern as existing
`set_timeout`/`write_chars_to_pane_id`).
</content>
