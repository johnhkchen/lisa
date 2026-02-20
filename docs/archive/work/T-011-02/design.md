# T-011-02 Design: Run lisa loop end-to-end on a real project

## Decision: Test Strategy

### Option A: Test against lisa repo with real tickets

Run `lisa loop` against the existing S-012 through S-016 tickets. These are real work items.

**Pros**: Tests realistic conditions, accomplishes real work simultaneously.
**Cons**: Risky — `--dangerously-skip-permissions` Claude sessions could make unwanted changes. Hard to isolate observations from side effects. Multiple sessions modifying the same repo simultaneously.

### Option B: Test against lisa repo with isolated test tickets

Create 2-3 small purpose-built test tickets in the lisa repo with a dependency chain. Run `lisa loop` with `max_threads: 1` for controlled observation.

**Pros**: Safe — test tickets have limited, well-defined scope. Can observe the full lifecycle without risk. Easy to clean up afterward.
**Cons**: Doesn't test real-world complexity. Still runs in the same repo as working code.

### Option C: Test against a fresh scratch project

Create a minimal project in a temp directory, run `lisa init`, create test tickets, run `lisa loop`.

**Pros**: Completely isolated. Tests the full `lisa init` → `lisa loop` workflow from scratch.
**Cons**: Misses the "existing project" scenario. Requires setting up a project from nothing.

### Decision: Option B (with elements of C for init testing)

**Rationale**: Option B gives us the safest path to observing the full lifecycle in a realistic project. We test against the lisa repo itself (which has real infrastructure) but use dedicated test tickets that won't cause harm. We can verify `lisa init` behavior first in dry-run mode to confirm it handles the existing setup correctly.

For the init validation, we can also run `lisa init --dry-run` to see what it would do, then selectively apply fixes (add missing hooks, fix on-idle.sh).

## Test Plan Design

### Phase 1: Fix infrastructure gaps

1. Run `lisa init --dry-run` to see planned actions
2. Run `lisa init` to create missing hooks (on-stop.sh, on-clear.sh) and update settings.local.json
3. **Manually fix on-idle.sh**: change `LISA_TICKET_ID` → `LISA_PANE_ID` and filename pattern to `pane-$LISA_PANE_ID.idle`
4. Run `lisa validate --check-tools` to confirm everything passes

### Phase 2: Build from current source

1. Run `just build-cli` to rebuild with latest code (ensures embedded WASM matches)
2. Verify new binary version

### Phase 3: Create test tickets

Create 3 test tickets with a dependency chain:

```
T-TEST-01 (no deps, ready) → T-TEST-02 (depends on T-TEST-01) → T-TEST-03 (depends on T-TEST-02)
```

Tickets should be trivial tasks that a Claude session can complete quickly — e.g., "create a file with specific content" or "write a short analysis of an existing file."

### Phase 4: Dry-run validation

Run `lisa loop --dry-run` to verify:
- Ticket scanning works
- DAG is computed correctly
- Ready tickets are identified
- Generated layout looks correct

### Phase 5: Live run

Run `lisa loop --max-threads 1` to observe:
- Zellij launches with correct layout
- Dashboard renders in plugin pane
- First test ticket is scheduled
- Claude Code session spawns with correct prompt
- Hook signals appear in `.lisa/signals/`
- Phase transitions occur
- Session transitions (stop → clear → new prompt)
- Subsequent tickets get scheduled after first completes

### Phase 6: Observation and recording

Document all observations in `progress.md` as the run proceeds, checking each acceptance criterion.

## Configuration for Test Run

```
max_threads: 1          # Single thread for controlled observation
auto_advance: false     # Manual review preferred for spike
review_timeout_secs: 60 # Short timeout to observe the finish-up flow
```

Using `max_threads: 1` ensures sequential execution, making it easy to follow the lifecycle of each ticket through the DAG.

## Cleanup Plan

After testing:
1. Remove test tickets from `docs/active/tickets/`
2. Remove test work artifacts from `docs/active/work/`
3. Keep infrastructure fixes (hooks, settings.local.json) — they're needed going forward
4. Keep progress.md as the deliverable

## Risk Mitigation

- **Rebuild before testing**: `just build-cli` ensures WASM matches code
- **Single thread**: prevents concurrent modification confusion
- **Test tickets only**: real tickets untouched
- **`--dangerously-skip-permissions`**: inherent to lisa's design, but test ticket scope is limited
- **Monitor signals dir**: watch `.lisa/signals/` in a separate terminal to verify hook execution
