# T-020-05 Research — interactive-gate-harness

Descriptive map of the machinery this ticket must make *observable*. No solutions here.

## What the ticket closes

S-020 gates the loop so that when an agent calls `AskUserQuestion`, lisa (a) fires
attention notification, (b) blocks its own injection into that pane (no clobber), and
(c) resumes cleanly once the human answers. The automated half is unit-tested
(`T-020-02/progress.md`: 11 plugin tests over consume/suppress/exempt/surface). The
residual is the *live* TUI block+resume cycle, which cannot be exercised headlessly.
This ticket is **harness + runbook only** — no production code changes — producing a
one-command setup that makes the question→block→resume sequence reviewable after a
real `lisa loop`.

## The awaiting state machine (crates/lisa-plugin/src/lib.rs)

- **Field:** `awaiting_human: HashSet<u32>` (`lib.rs:249`) — the set of terminal pane
  ids currently blocked on an `AskUserQuestion`. Every gate behavior is a projection
  of this set.
- **Signal ingest:** `check_awaiting_signals()` (`lib.rs:828`) scans the signal dir for
  `pane-<id>.awaiting` files, parses the id from the filename
  (`strip_suffix(".awaiting")`, `lib.rs:840`), inserts into `awaiting_human`
  (`lib.rs:848`), and consumes the file. It runs each tick from the timer path
  (`lib.rs:1667`). The `pane-<id>.awaiting` file is written by the
  PreToolUse[AskUserQuestion] hook (documented at `lib.rs:818`).
- **Clear:** on a heartbeat/next-tool signal for the pane, `awaiting_human.remove(&pane_id)`
  (`lib.rs:811`) clears the block. Resume is therefore self-healing: the first tool call
  after the human answers drops the pane out of the set.
- **Injection guard:** `send_line_to_pane()` (`lib.rs:275`) is the single choke point for
  typing into a pane. It early-returns if `is_pane_awaiting(id)` (`lib.rs:283`), logging
  `"Suppressed injection into pane {id} (awaiting human)"` via `ActivityEvent::Info`, and
  returns *before* queuing the deferred Enter (so a dropped line leaves no stray Enter).
  This is the "belt-and-suspenders" net; per-caller guards exist too, e.g. the hard-silence
  timeout path checks `!awaiting_human.contains(&t.pane_id)` (`lib.rs:1538`).
- **UI projection:** `to_ui_state` sets `awaiting: self.is_pane_awaiting(t.pane_id)` per
  active thread (`lib.rs:2736`); the dashboard renders the `[AWAITING]` marker purely from
  this. A test asserts the marker is a pure projection of the set (`lib.rs:5705`).

## The notification path (on-notify)

- `build_notify_command()` (`lib.rs:315`) is pure/host-free (unit-tested). It builds an
  `sh -c` argv whose guard is `if [ -x "$LISA_HOOK" ]; then "$LISA_HOOK" "$1" "$2"; fi`
  so an absent/non-executable hook is a silent no-op. `$1`=event, `$2`=detail.
- Env passed: `LISA_HOOK` (= `<root>/.lisa/hooks/on-notify`), `LISA_EVENT`, `LISA_PROJECT`,
  plus `extra_env` — notably `LISA_REASON` and `LISA_PANE_ID` (see the idle path,
  `lib.rs:1057-1059`, and tests `lib.rs:5404-5418`).
- `fire_notify()` (`lib.rs:348`) runs the argv via Zellij `run_command`, tagging the
  context with `lisa_notify` so `RunCommandResult` attributes success/failure back and logs
  `"on-notify {event} ok"` / `"... failed (exit …)"` (`lib.rs:2649-2658`).
- For the gate, the attention event carries `LISA_EVENT=attention` and `LISA_REASON=question`
  — this is the line the harness captures as durable proof the notification fired.

## What `lisa init` scaffolds (crates/lisa-cli/src/init.rs)

- Hook files written under `.lisa/hooks/` (`init.rs:322-330`):
  `on-idle.sh`, `on-stop.sh`, `on-clear.sh`, `on-heartbeat.sh` (each `chmod +x`), plus
  `on-notify.sample` (deliberately *not* executable — the user copies it to `on-notify`).
- These lifecycle hooks are the signal writers the loop reads. Their env includes
  `LISA_PANE_ID` (`lib.rs:51-55`: the agent is launched with `LISA_PANE_ID=<n>` so its
  hooks can identify their pane).
- `lisa validate` / init checks that the four `.sh` hooks are present and executable
  (`init.rs:484-489`, `init.rs:681-712`).

## The existing deliverable

`setup-gate-harness.sh` already exists in this work dir (the ticket is marked `review`).
It: builds plugin→CLI (touch forces WASM re-embed), scaffolds a throwaway git project at
`/tmp/lisa-gate-dryrun`, runs `lisa init`, drops `T-GATE-01` (forces `AskUserQuestion` as
first action), installs a logging `on-notify` (→ `.lisa/on-notify.log`), appends a
`GATE-TRACE` timestamped line to each `on-*.sh` (→ `.lisa/trace.log`), truncates both logs,
and prints the run command + PASS/FAIL checklist.

## Constraints & assumptions

- **No production code changes.** The harness may only touch its own work dir and the
  throwaway temp project — never this repo's tickets or `crates/`.
- **Build order matters:** plugin WASM must build before the CLI, and the WASM must be
  touched so `build.rs` re-copies it into the embedded CLI (`CLAUDE.md`, harness lines
  15-21). A stale embed silently tests old plugin behavior.
- **`review` not `ready`:** so a `lisa loop` in *this* repo won't schedule the ticket as
  agent work; the dry run is human-executed.
- **Headless gap:** the block/resume is interactive by nature — the human must answer in
  the TUI. The harness cannot assert PASS itself; it produces durable evidence
  (`on-notify.log`, `trace.log`) that a human checks against the checklist.
- **Trace env dependency:** the trace line uses `$LISA_PANE_ID`; if a lifecycle hook is
  invoked without that env exported, the pane id renders empty (cosmetic, not a failure).
