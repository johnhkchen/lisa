# T-011-02 Progress: Run lisa loop end-to-end

## Steps Completed

### Step 1: `lisa init` — PASS

- `lisa init --dry-run` correctly showed planned actions
- `lisa init` created: on-stop.sh, on-clear.sh, .lisa.toml, updated settings.local.json
- Correctly skipped: CLAUDE.md, docs/rdspi-workflow.md, on-idle.sh, directories
- settings.local.json: merged hooks correctly, preserved existing permissions block, upgraded bare-path idle command to guarded form
- Hook scripts created with correct permissions (755)

### Step 2: Fix on-idle.sh — DONE

- Old version used `LISA_TICKET_ID` env var and `$TICKET_ID.idle` filename
- Fixed to use `LISA_PANE_ID` and `pane-$LISA_PANE_ID.idle` format
- Note: `lisa init` skipped this file (never overwrites). This is a manual step that users upgrading from old versions must do.

### Step 3: `lisa validate --check-tools` — PASS

- Output: "All checks passed. 19 tickets, 10 ready, DAG valid."
- zellij 0.43.1, claude 2.1.49, lisa 0.1.6 all found on PATH

### Step 4: Rebuild — PASS

- `just build-cli` completed (6.38s)
- `just install` replaced binary at /Volumes/ext1/cargo/bin/lisa
- 3 dead_code warnings in ui.rs (pane_id fields) — cosmetic, not blocking

### Step 5: Test tickets — PASS (with finding)

- Created T-TEST-01, T-TEST-02, T-TEST-03 with dependency chain

**BUG FOUND: Multi-line YAML `depends_on` silently ignored**

The ticket parser (`crates/lisa-core/src/ticket.rs:parse_string_vec`) only handles inline array syntax (`depends_on: [T-001, T-002]`). Multi-line YAML list syntax (`depends_on:\n  - T-001`) results in an empty depends_on list — the dependency is silently dropped.

**Impact**: 7 tickets across the project have broken dependencies:
- T-011-02 → T-011-01 (broken)
- T-011-03 → T-011-02 (broken)
- T-013-02 → T-013-01 (broken)
- T-014-03 → T-014-01 (broken)
- T-016-01 → T-014-03 (broken)
- T-016-02 → T-014-03 (broken)
- T-016-03 → T-014-03 (broken)

**Workaround**: Use inline syntax only: `depends_on: [T-001, T-002]`
**Fix**: Add multi-line YAML list support to `parse_string_vec` or switch to a YAML parser (serde_yaml).

This is the most significant finding from this spike. Fixed test tickets to use inline syntax.

### Step 6: Dry-run — PASS

- `lisa loop --dry-run --max-threads 1` shows correct DAG with 5 edges
- T-TEST-01 ready, T-TEST-02 blocked by T-TEST-01, T-TEST-03 blocked by T-TEST-02
- Generated layout has 2 panes (2 * max_threads=1), plugin configured correctly

### Step 7: Live run — NOT YET EXECUTED

This step requires launching `lisa loop` which will take over the terminal with zellij. This must be done interactively by the user.

**To execute**:
```bash
lisa loop --max-threads 1
```

**What to observe**:
1. Does zellij launch? (expected: yes)
2. Does the plugin pane show a dashboard? (expected: yes)
3. Is T-TEST-01 scheduled into the first agent pane? (expected: yes)
4. Does a Claude Code session spawn with the correct prompt? (expected: yes)
5. Do signal files appear in `.lisa/signals/`? (monitor with `watch ls -la .lisa/signals/`)
6. After T-TEST-01 completes, does T-TEST-02 get scheduled? (expected: yes, after cooldown)
7. Do keyboard controls work? ([p] pause, [d] mark-done, [r] reset)

## Findings Summary

### Critical: Multi-line YAML `depends_on` silently ignored
- Severity: **High** — silent data loss, breaks DAG correctness
- 7 existing tickets affected
- Should be a new ticket to fix

### Minor: Old on-idle.sh uses LISA_TICKET_ID
- Severity: **Medium** — signals won't be processed correctly
- `lisa init` can't fix this (never overwrites)
- Users upgrading need manual intervention or a separate upgrade command

### Minor: settings.local.json permissions lost on init
- Actually NOT an issue — `merge_hooks` correctly preserves the `permissions` block. This was verified.

### Cosmetic: 3 dead_code warnings in ui.rs
- `pane_id` fields on `ActiveThread`, `ParkedThread`, `SlotInfo`
- Not blocking, but should be cleaned up

## Acceptance Criteria Status

- [x] `lisa init` + `lisa validate` succeed on the target project
- [ ] `lisa loop` launches and schedules at least one ticket — **requires interactive run**
- [ ] At least one ticket completes a full phase cycle — **requires interactive run**
- [ ] Hook signals observed — **requires interactive run**
- [x] All observations documented in this file

## Next Steps

The live run (Step 7) must be performed interactively. The pre-flight checks are all green. The infrastructure is ready.

After the live run, clean up:
1. Remove T-TEST-01.md, T-TEST-02.md, T-TEST-03.md
2. Remove docs/active/work/T-TEST-01/, T-TEST-02/, T-TEST-03/
