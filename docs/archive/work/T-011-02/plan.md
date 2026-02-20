# T-011-02 Plan: Run lisa loop end-to-end on a real project

## Step 1: Run `lisa init` to fix infrastructure

1. Run `lisa init --dry-run` to preview what will be created/skipped
2. Run `lisa init` to execute the plan
3. Verify on-stop.sh, on-clear.sh created and executable
4. Verify settings.local.json updated with all three hook types

**Verification**: `ls -la .lisa/hooks/` shows all three scripts, `cat .claude/settings.local.json` shows Stop + SessionStart + Notification entries.

## Step 2: Fix on-idle.sh manually

The existing on-idle.sh uses `LISA_TICKET_ID` (legacy). Replace with:

```sh
#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification.
SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"
if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
fi
```

**Verification**: `cat .lisa/hooks/on-idle.sh` shows `LISA_PANE_ID`, not `LISA_TICKET_ID`.

## Step 3: Run `lisa validate --check-tools`

Confirm all checks pass: structure, hooks, tools, at least one ready ticket.

**Verification**: Output shows "All checks passed" with ticket/ready counts.

## Step 4: Rebuild from source

Run `just build-cli` to ensure the installed binary has current WASM embedded.

**Verification**: `lisa --version` shows expected version; `just build-cli` completes without errors.

## Step 5: Create test tickets

Create three test tickets with a dependency chain:

**T-TEST-01**: No deps, `phase: ready`. Task: "Create a file `docs/active/work/T-TEST-01/research.md` that lists the top-level files in this repository."

**T-TEST-02**: Depends on T-TEST-01, `phase: ready`. Task: "Read T-TEST-01's research.md and create a design.md summarizing key findings."

**T-TEST-03**: Depends on T-TEST-02, `phase: ready`. Task: "Read T-TEST-02's design.md and create a structure.md with final observations."

**Verification**: `lisa validate` still passes, shows increased ticket count.

## Step 6: Dry-run verification

Run `lisa loop --dry-run --max-threads 1`. Verify:
- T-TEST-01 shows as "ready"
- T-TEST-02, T-TEST-03 show as "blocked"
- Layout has 2 agent panes (2 * max_threads=1)
- Execution order is correct

**Verification**: Output matches expected DAG structure.

## Step 7: Live run

Run `lisa loop --max-threads 1`. During the run:

1. **Layout observation**: Does zellij open with stacked panes + plugin pane?
2. **Dashboard**: Does the plugin pane show the dashboard with ticket status?
3. **Scheduling**: Does T-TEST-01 get scheduled first?
4. **Session spawn**: Does `claude --dangerously-skip-permissions` launch in the agent pane?
5. **Signal monitoring**: In a separate terminal, `watch ls -la .lisa/signals/` to see signals appear
6. **Phase transitions**: Does the plugin detect phase completion and advance?
7. **Session reuse**: After T-TEST-01 completes, does the slot get reused for T-TEST-02?
8. **Keyboard controls**: Test `[p]` pause/resume, `[d]` mark-done modal, `[r]` reset modal
9. **Review handling**: After implement phase, does the review timeout trigger the finish-up prompt?

**Verification**: At least one test ticket completes a full RDSPI cycle. Signal files observed. Dashboard reflects changes.

## Step 8: Record observations in progress.md

Update `docs/active/work/T-011-02/progress.md` with:
- Pass/fail for each acceptance criterion
- Screenshots or terminal output excerpts if useful
- Any bugs, panics, or unexpected behavior discovered
- Timing observations (how long phases take, signal latency)

## Step 9: Cleanup

1. Remove test ticket files: `T-TEST-01.md`, `T-TEST-02.md`, `T-TEST-03.md`
2. Remove test work directories: `docs/active/work/T-TEST-01/`, etc.
3. Keep infrastructure fixes (hooks, settings.local.json)
4. Keep progress.md as the deliverable

## Testing Strategy

This is an observational spike — the "tests" are human observations documented in progress.md. No automated tests are added. The goal is to validate the system works end-to-end and identify issues for future tickets.

If issues are found, they should be recorded as findings in progress.md with enough detail to create follow-up tickets.
