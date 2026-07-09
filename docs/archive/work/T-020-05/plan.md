# T-020-05 Plan — interactive-gate-harness

Ordered, independently-verifiable steps to produce and validate the harness. Because the
sole deliverable is a setup script whose *purpose* is a human-run dry run, the testing
strategy splits into (a) static/dry checks the author can run headlessly and (b) the live
acceptance run the human performs.

## Implementation steps

1. **Script skeleton + arg/path handling.**
   `set -euo pipefail`, `DEST` default, `SCRIPT_DIR`/`REPO` resolution.
   *Verify:* `bash -n setup-gate-harness.sh` parses; `REPO` resolves to repo root when run
   from the work dir.

2. **Build block with WASM re-embed.**
   plugin → `touch` → CLI; assert `target/release/lisa` executable.
   *Verify:* running the block produces a fresh `lisa` binary; deliberately skipping the
   `touch` and confirming the embed is stale is the rationale (not a repeated test).

3. **Scaffold throwaway project.**
   `rm -rf`/`mkdir`/`cd`, `git init`, local git identity, minimal `CLAUDE.md`,
   `docs/active/{tickets,work}`.
   *Verify:* `$DEST/.git` exists; `git status` clean-ish; dir is fully removable/rebuildable.

4. **`lisa init`.**
   *Verify:* `$DEST/.lisa/hooks/{on-idle,on-stop,on-clear,on-heartbeat}.sh` exist and are
   executable; `on-notify.sample` exists and is *not* executable; `.lisa.toml` present.

5. **Trigger ticket `T-GATE-01`.**
   Frontmatter valid (`phase: ready`, `depends_on: []`); Context mandates `AskUserQuestion`
   first with two options.
   *Verify:* `lisa validate` (in `$DEST`) accepts the ticket; DAG shows it schedulable.

6. **Logging `on-notify`.**
   Author executable hook appending `EVENT/LISA_REASON/DETAIL` to `.lisa/on-notify.log`.
   *Verify:* `.lisa/hooks/on-notify -x`; a manual
   `LISA_REASON=question .lisa/hooks/on-notify attention foo` appends the expected line.

7. **Trace instrumentation (idempotent).**
   Append `GATE-TRACE` block to the four `on-*.sh`, guarded by `grep -q`.
   *Verify:* each hook contains exactly one `GATE-TRACE` block after two script runs;
   invoking a hook with `LISA_PANE_ID=9` appends `<hook> pane=9` to `trace.log`; the
   scaffolded signal-write still runs (behavior preserved).

8. **Log reset + runbook heredoc.**
   Truncate both logs; print DEST, run command, watch list, checks, PASS/FAIL.
   *Verify:* both logs are empty after setup; printed `lisa loop` command is copy-pasteable
   and paths are absolute.

## Testing strategy

- **Static (headless, author-run):**
  - `bash -n` on the script (syntax).
  - `shellcheck` if available (lint; advisory).
  - Dry execution to a scratch DEST, then assert the runtime layout from `structure.md`
    (hooks present/executable, logs empty, ticket valid, `on-notify` fires on manual invoke).
  - Idempotency: run the script twice; confirm no double-instrumentation.
- **Live acceptance (human-run, the actual point):**
  Run `cd $DEST && lisa loop`, then check against acceptance criteria:
  - (a) Trigger: `on-notify.log` has `EVENT=attention … LISA_REASON=question`.
  - (b) Block/no-clobber: pane shows the question; no `/clear`/prompt typed over it;
    `[AWAITING]` marker present; if a timeout path fires, activity log shows
    `"Suppressed injection into pane N (awaiting human)"`.
  - (c) Resume: after answering, a `heartbeat pane=N` line appears in `trace.log` *after*
    the answer and the `[AWAITING]` marker clears.
  - FAIL signs documented and watched: clobbered pane, missing marker, marker never clears.

## Verification criteria (definition of done)

- Script runs clean end-to-end (`set -e` survives) and prints the runbook.
- Produced layout matches `structure.md` exactly.
- Re-running is safe (idempotent instrumentation, `rm -rf` scaffold).
- Runbook maps every acceptance criterion to a concrete observable (live or persistent).
- Zero writes outside `$DEST` and this work dir; no production source changed.

## Rollout / handoff

No release artifact — this is an internal QA harness. Handoff is the printed runbook plus
`review.md`. If the live run FAILs, the regression is in the live block/resume assumption
(not the unit-tested machinery); `review.md` flags this as the escalation path before S-020
is relied on in production.

## Risks to watch during implementation

- Build-order/`touch` omission → stale embed, false result. (Step 2 encodes the order.)
- Overwriting a real `on-notify` — mitigated because the scaffold ships only the
  `.sample`; the harness owns the executable name.
- Non-idempotent trace append → duplicated lines on re-run. (Step 7 `grep -q` guard.)
