# T-009-01 Design: Bug Sweep and Dead Code Cleanup

## Decision 1: Fix non-interactive Claude sessions

### Options considered

**A. Change positional arg to `-p` flag** (chosen)
- Change `lib.rs:build_claude_command()` from `claude ... "{prompt}"` to `claude ... -p "{prompt}"`
- `-p` / `--prompt` starts interactive mode with an initial prompt
- Session stays alive after processing, reaches "Awaiting User Input"
- Idle signal hook can fire

**B. Use `--print` then respawn interactively**
- Keep non-interactive for first pass, detect exit, respawn interactive
- Rejected: complex, doesn't solve the fundamental problem, adds latency

**C. Use `--resume` with session management**
- Rejected: over-engineering for this fix, introduces session ID tracking

### Decision

**Option A.** Minimal change — one format string edit in `lib.rs:40-44`. The
prompt text is unchanged; only the flag changes from positional to `-p`.

---

## Decision 2: Double state transitions

### Options considered

**A. Track advanced tickets in poll cycle, skip sync block for them** (ticket's suggestion)
- Collect ticket IDs from `check_artifact_advances()` and `check_idle_signals()`
- Skip the phase sync block for those IDs

**B. Do nothing** (chosen)
- Research found that the sync block at lib.rs:817-827 already guards with
  `thread.current_phase != ticket.phase`
- After `check_artifact_advances()` updates both the disk and the thread,
  the phases match, so the sync block's condition is false — no duplicate update
- The sync block serves as a defensive reconciliation fallback for cases where
  phases drift (external edits, missed transitions)
- No duplicate log entries are produced

### Decision

**Option B.** The code is already correct. The ticket's concern was based on an
incomplete reading of the guard condition. Adding a `HashSet` of advanced tickets
would add complexity for zero behavioral change. A clarifying comment in the sync
block would be appropriate.

---

## Decision 3: Dead code cleanup strategy

### Options considered

**A. Remove scheduler.rs entirely**
- The entire file is dead code (7 of 13 warnings come from it)
- `mod scheduler;` declaration removed from lib.rs
- Simplest, eliminates 7 warnings + 8 dead tests

**B. Keep scheduler.rs as empty module with `#[allow(dead_code)]`**
- Rejected: the code is superseded, not planned. No reason to keep it.

**C. Selective removal (keep CommitLock for future use)**
- Rejected: commit locking isn't used and the design will change when needed.

### Decision

**Option A.** Remove `scheduler.rs` entirely and its `mod scheduler;` in lib.rs.

For the 6 ui.rs warnings:
- **Remove** `BG_RED`, `status_indicator()`, `ActivityType::ThreadParked` — genuinely unused
- **Remove** `TicketNode::blocks`, `SlotInfo::has_session`, `PluginState::selected_ticket` — never read
- Update all test struct literals that reference removed fields

---

## Summary of changes

| Change | Approach | Risk |
|--------|----------|------|
| Non-interactive sessions | Change positional arg to `-p` flag | Low — flag swap, well-tested by existing tests |
| Double state transition | No code change, add clarifying comment | Zero |
| Dead code in scheduler.rs | Delete entire file | Low — code is dead, tests are for dead code |
| Dead code in ui.rs | Remove 6 items, update test constructors | Low — items are never read/constructed |

## Risks

- The `-p` flag behavior depends on Claude Code CLI semantics. If `-p` doesn't
  accept the prompt as the next argument, the format string may need adjustment
  (e.g., `-p "{prompt}"` vs `--prompt "{prompt}"`). This is verifiable by
  running the generated command string manually.
- Removing `scheduler.rs` is irreversible but the code is in git history.
