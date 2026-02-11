---
id: T-011-02
title: Run lisa loop end-to-end on a real project
type: spike
phase: ready
status: Todo
priority: high
story: S-011
created: 2026-02-11
depends_on:
  - T-011-01
---

# T-011-02: Run lisa loop end-to-end on a real project

## Objective

Exercise `lisa init` and `lisa loop` against a real project (can be the lisa repo itself or another project). Observe the full lifecycle: ticket scheduling, Claude Code session spawning, hook signals, phase transitions, and dashboard behavior.

## Steps

1. **Initialize a project**
   ```bash
   cd <target-project>
   lisa init
   lisa validate
   ```
   Note any validation errors and whether `lisa init` scaffolds correctly.

2. **Create at least 2-3 test tickets** in `docs/active/tickets/` with dependencies between them. At least one should be a simple task that can complete end-to-end.

3. **Run the loop**
   ```bash
   lisa loop
   ```

4. **Observe and record:**
   - Does Zellij launch with the correct layout?
   - Does the dashboard render correctly?
   - Are tickets scheduled in dependency order?
   - Do Claude Code sessions spawn and receive the correct prompt?
   - Do hook signal files appear in `.lisa/signals/`? (`.stopped`, `.cleared`, `.idle`)
   - Do transitions between tickets work (WaitingForStop → WaitingForClear → Idle)?
   - Does the review auto-complete trigger?
   - Are there any panics, hangs, or UI glitches?
   - Does `[p]` pause/resume work? `[d]` mark-done? `[r]` reset?

5. **Test edge cases** (if time permits)
   - Kill a Claude session mid-run — does the plugin recover?
   - Create a ticket with a missing dependency — does validate catch it?
   - Run with `max_threads: 1` vs `max_threads: 3`

## Acceptance Criteria

- [ ] `lisa init` + `lisa validate` succeed on the target project
- [ ] `lisa loop` launches and schedules at least one ticket
- [ ] At least one ticket completes a full phase cycle
- [ ] Hook signals observed (`.stopped` and/or `.cleared` files created)
- [ ] All observations documented in `docs/active/work/T-011-02/progress.md`
