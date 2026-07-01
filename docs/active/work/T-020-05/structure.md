# T-020-05 Structure — interactive-gate-harness

The blueprint. What files exist, their boundaries, and the runtime layout the script
produces. No production source is touched — all artifacts live under the work dir or the
throwaway temp project.

## Files in this repo

| Path | Disposition | Purpose |
|------|-------------|---------|
| `docs/active/work/T-020-05/setup-gate-harness.sh` | **created** (exists) | The one-command harness. Sole deliverable. `chmod +x`. |
| `docs/active/work/T-020-05/{research,design,structure,plan,progress,review}.md` | created | RDSPI phase artifacts (this pass). |
| `crates/**`, `docs/active/tickets/**` | **untouched** | Ticket forbids production changes; must not be modified. |

## `setup-gate-harness.sh` — internal structure

Single bash script, `set -euo pipefail`, one positional arg `DEST_DIR`
(default `/tmp/lisa-gate-dryrun`). Sections, in execution order:

1. **Path resolution.** `SCRIPT_DIR` from `BASH_SOURCE`; `REPO` = four levels up
   (`docs/active/work/T-020-05/` → repo root). No `cd` into the repo beyond the build
   subshell.
2. **Build (subshell).** `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
   → `touch target/wasm32-wasip1/release/lisa.wasm` → `cargo build -p lisa-cli --release`.
   Guard: assert `$REPO/target/release/lisa` is executable, else fail fast.
3. **Scaffold.** `rm -rf "$DEST"; mkdir -p; cd`; `git init` + local user config; minimal
   `CLAUDE.md` (a no-op `Build and Test` block); `mkdir docs/active/{tickets,work}`.
4. **`lisa init`.** Real scaffold → produces `.lisa/`, hooks, `.lisa.toml`, workflow.
5. **Trigger ticket.** Write `docs/active/tickets/T-GATE-01-ask.md` — frontmatter
   (`phase: ready`, `depends_on: []`) + a Context that mandates `AskUserQuestion` as the
   first action with two options.
6. **Logging `on-notify`.** Overwrite `.lisa/hooks/on-notify` (the executable name, not the
   `.sample`) with a `#!/bin/sh` that appends `EVENT/LISA_REASON/DETAIL` to
   `.lisa/on-notify.log`; `chmod +x`.
7. **Trace instrumentation.** For `on-idle on-stop on-clear on-heartbeat`: append (idempotent
   via `grep -q GATE-TRACE`) a timestamped `<hook> pane=$LISA_PANE_ID` line to `.lisa/trace.log`.
8. **Reset logs.** `: > .lisa/trace.log; : > .lisa/on-notify.log`.
9. **Runbook heredoc.** Print DEST, the exact `cd … && lisa loop` command, the 5-step live
   watch list, the two post-run `cat` checks, and the PASS/FAIL definition.

## Boundaries

- **Repo ↔ temp project.** The script reads from `$REPO` (build only) and writes exclusively
  under `$DEST`. No write path targets `$REPO`. This is the invariant that satisfies the
  "never touches this repo's tickets" constraint.
- **Scaffolded hook behavior ↔ instrumentation.** Instrumentation is *append-only*. The
  `on-*.sh` signal-writes scaffolded by `lisa init` run unchanged; the `GATE-TRACE` block is
  additive and idempotent. `on-notify` is the one file fully authored by the harness (the
  scaffold ships only `on-notify.sample`, non-executable, so there is no behavior to preserve).
- **Evidence channels.** `.lisa/on-notify.log` (notification proof) and `.lisa/trace.log`
  (lifecycle timeline) are independent files, each owned by one writer.

## Runtime layout produced at `$DEST`

```
/tmp/lisa-gate-dryrun/
  CLAUDE.md
  .git/
  docs/active/
    tickets/T-GATE-01-ask.md      # forces AskUserQuestion first
    work/
  .lisa/
    .lisa.toml
    hooks/
      on-idle.sh   on-stop.sh   on-clear.sh   on-heartbeat.sh   # scaffolded + GATE-TRACE
      on-notify                                                 # harness-authored, logging
      on-notify.sample                                          # untouched scaffold
    on-notify.log                 # ← attention/question line lands here
    trace.log                     # ← lifecycle timeline; post-answer heartbeat = resume
```

## Interfaces / contracts relied on (read-only, from research)

- **on-notify contract:** invoked as `on-notify <event> <detail>` with env
  `LISA_EVENT`, `LISA_REASON`, `LISA_PANE_ID`, `LISA_PROJECT`, `LISA_HOOK`. Harness reads
  `$1` and `$LISA_REASON`.
- **Lifecycle hook env:** `LISA_PANE_ID` exported to the agent process and available to its
  signal hooks.
- **Awaiting signal:** `pane-<id>.awaiting` in the signal dir, consumed by the plugin — not
  written by the harness; observed indirectly via the `[AWAITING]` dashboard marker and the
  `"Suppressed injection …"` activity line.

## Ordering constraints

- Build (2) strictly before anything that runs `lisa` (4). WASM `touch` strictly between the
  two cargo builds.
- `lisa init` (4) strictly before hook overwrite/instrumentation (6,7) — the files must exist.
- Log reset (8) strictly last among file writes so setup-time noise is cleared.
