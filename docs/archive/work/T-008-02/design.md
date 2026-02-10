# T-008-02 Design: Idle-Aware Phase Advancement

## Decision 1: Where to Add Signal Checking

### Options

**A. Inside `check_artifact_advances()`**
Add idle signal scanning to the existing artifact advancement method. The method already iterates running threads and checks for phase artifacts.

**B. New `check_idle_signals()` method called from `poll_tick()`**
Separate method that scans the signal directory and processes idle signals independently.

**C. Combined: scan signals first, then use results in `check_artifact_advances()`**
Scan signals into a HashSet, pass it to the artifact checker.

### Decision: Option B — Separate method

Rationale:
- Idle signals have different semantics than artifact detection (Implement phase needs no artifact check)
- The method can be tested independently with a temp directory
- Clean separation: `check_artifact_advances()` handles artifact-driven advancement, `check_idle_signals()` handles idle-driven advancement
- Called right after `check_artifact_advances()` in `poll_tick()`, so both run on every cycle
- The two mechanisms coexist — if `check_artifact_advances()` already advanced a ticket, the idle signal for that ticket is simply cleaned up

## Decision 2: How to Handle Alerts for Idle-Without-Artifact

### Options

**A. Add `AlertType::IdleWithoutArtifact` to ui.rs**
New alert type in the attention banner.

**B. Store alerts in State as `Vec<(TicketId, String)>` and convert in `to_ui_state()`**
Use existing `AlertType::Stuck` or reuse warning infrastructure.

**C. Store a `HashSet<TicketId>` of idle-without-artifact tickets in State**
Check in `to_ui_state()` and generate alerts from it.

### Decision: Option A + store in State

Rationale:
- Add `AlertType::IdleWithoutArtifact` for clear display in the attention banner
- Store `Vec<(TicketId, String)>` in State (ticket_id + detail message)
- In `to_ui_state()`, convert stored idle alerts to `ui::HealthAlert` with the new AlertType
- Clear the stored alerts each poll cycle before scanning (they're re-detected if still present)
- This follows the existing pattern where alerts are derived from state, but idle alerts need explicit storage since they aren't derivable from thread health alone

## Decision 3: Signal Scanning Approach

### Options

**A. `std::fs::read_dir()` on the signal directory**
Simple, standard library call. Parse filenames for ticket IDs.

**B. `scan_host_folder` zellij API**
Faster but returns a more complex structure.

### Decision: Option A — `std::fs::read_dir()`

Rationale:
- Signal directory has at most `max_threads` files (typically 2-4)
- No performance concern for scanning a tiny directory
- `std::fs::read_dir()` already works in WASI (used by ticket scanning)
- Simpler code, no zellij API dependency in the signal logic

## Decision 4: Implement -> Review Behavior

### Options

**A. Park thread, keep slot**
Same as current Review parking behavior.

**B. Park thread, release slot**
Free the slot since the agent is done coding.

**C. Mark done, release slot**
Skip Review entirely and mark ticket as done.

### Decision: Option A — Park thread, keep slot

Rationale:
- Matches existing behavior when `check_artifact_advances()` parks at Review
- The ticket spec says "park thread" which is the existing pattern
- Slots are released when the human marks the ticket done via 'd' key
- Changing slot behavior for idle signals vs artifact detection would be inconsistent
- If slot contention becomes an issue, that's a separate optimization

## Decision 5: Signal Path Computation

### Options

**A. Hardcode `/host/.lisa/signals/` in the scanning code**

**B. Compute from a base path, similar to work_dir**

### Decision: Option B — Compute from `/host/` prefix

Rationale:
- In `load()`, the plugin already prefixes relative paths with `/host/`
- Store the signal directory path alongside other config paths
- For testing (non-WASI), the path is just `.lisa/signals/` relative to a temp dir
- Add a `signal_dir` field to State (not PluginConfig — it's not user-configurable)

Actually, simplifying: just compute it in the method from a known base. The plugin knows `/host/` is the mount point. But for testability, pass the signal dir as a parameter or derive from the existing `config.work_dir` parent.

**Revised:** Compute the signal directory as a sibling of the existing config directories. The config paths are prefixed with `/host/` in `load()`. The `.lisa/signals/` path should be computed the same way. Store it in State directly during `load()`.

## Decision 6: Phase Advancement Mechanics

For advancing a ticket via idle signal:
1. Look up the thread for the ticket_id from the signal filename
2. Get the current phase from the thread
3. Apply rules:
   - **Implement**: advance to Review, park thread
   - **Research/Design/Structure/Plan**: check artifact exists, if yes advance to next phase, if no generate alert
   - **Review/Done/Ready**: ignore signal (clean up file)
4. Update ticket frontmatter on disk via `ticket::update_ticket_phase()`
5. Update thread phase and `last_phase_change`
6. Log appropriate activity events
7. Delete the signal file

This mirrors the existing `check_artifact_advances()` pattern exactly, adding the Implement case and the idle-without-artifact alert.

## Summary of Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Separate `check_idle_signals()` method | Clean separation, independently testable |
| 2 | New `AlertType::IdleWithoutArtifact` + stored alerts in State | Clear display, follows existing pattern |
| 3 | `std::fs::read_dir()` for scanning | Simple, sufficient for tiny directory |
| 4 | Park thread, keep slot at Review | Consistent with existing parking behavior |
| 5 | Compute signal_dir in State during load() | Testable, consistent with other paths |
| 6 | Mirror check_artifact_advances() pattern | Reuse proven advancement mechanics |
