# T-008-02 Structure: Idle-Aware Phase Advancement

## Files Modified

### 1. `crates/lisa-plugin/src/lib.rs`

**State struct** — Add two new fields:

```rust
/// Path to the idle signal directory (`.lisa/signals/` under /host/).
signal_dir: PathBuf,

/// Idle-without-artifact alerts detected during the current poll cycle.
/// Cleared and re-populated each cycle by `check_idle_signals()`.
idle_alerts: Vec<(TicketId, String)>,
```

**`load()`** — After prefixing `work_dir` with `/host/`, compute and store the signal directory:
```rust
self.signal_dir = host.join(".lisa/signals");
```

**New method: `check_idle_signals(&mut self)`** — Core logic:

```
fn check_idle_signals(&mut self):
    clear self.idle_alerts
    read_dir(self.signal_dir):
        for each *.idle file:
            parse ticket_id from filename (strip .idle suffix)
            delete the signal file (cleanup first, prevents re-trigger)
            look up thread for ticket_id
            if no thread or thread not Running: continue
            match thread.current_phase:
                Implement:
                    advance to Review via update_ticket_phase()
                    park thread
                    log PhaseCompleted + TicketPhaseChanged events
                Research | Design | Structure | Plan:
                    check artifact exists (work_dir/ticket_id/artifact_name)
                    if artifact exists:
                        advance to next phase via update_ticket_phase()
                        log events
                        if next phase is Review: park thread
                    else:
                        push (ticket_id, detail) to self.idle_alerts
                        log Warning event
                _ (Ready, Review, Done):
                    ignore (signal already cleaned up)
```

**`poll_tick()`** — Add call to `check_idle_signals()` right after `check_artifact_advances()`:
```rust
self.check_artifact_advances();
self.check_idle_signals();  // <-- new
```

**`to_ui_state()`** — After building health alerts, append idle alerts:
```rust
for (ticket_id, detail) in &self.idle_alerts {
    alerts.push(ui::HealthAlert {
        ticket_id: ticket_id.clone(),
        alert_type: ui::AlertType::IdleWithoutArtifact,
        detail: detail.clone(),
        suggested_actions: vec!["Check agent output".to_string(), "Restart session".to_string()],
    });
}
```

### 2. `crates/lisa-plugin/src/ui.rs`

**`AlertType` enum** — Add new variant:
```rust
pub enum AlertType {
    Failed,
    Stuck,
    IdleWithoutArtifact,  // <-- new
}
```

**`render_attention_banner()`** — The existing rendering code iterates `state.alerts` and displays them. No changes needed to the rendering logic — it already handles any `HealthAlert` with any `AlertType`. Just ensure the display string for `IdleWithoutArtifact` is reasonable.

Actually, looking at the rendering code more carefully — it uses `alert.alert_type` to pick a symbol/color. Need to add a match arm for `IdleWithoutArtifact`:
- Symbol: "⏸" (paused/idle)
- Color: YELLOW (warning, not error)

## Files NOT Modified

- `crates/lisa-core/` — No core type changes. Signal detection is plugin-only.
- `crates/lisa-cli/` — Hook infrastructure already complete from T-008-01.
- `crates/lisa-plugin/src/scheduler.rs` — Scheduler doesn't handle idle signals.

## Module Boundaries

- **lib.rs** owns signal scanning, phase advancement logic, and alert storage
- **ui.rs** owns alert display types and rendering
- **types.rs (core)** provides `Phase::artifact_filename()` and `Phase::next()` used by the signal handler

## Interface Changes

- `ui::AlertType` gets a new variant: `IdleWithoutArtifact`
- `State` gets two new private fields: `signal_dir`, `idle_alerts`
- No public API changes

## Ordering

1. ui.rs first — add the new AlertType variant
2. lib.rs second — add State fields, `check_idle_signals()`, wire into poll_tick and to_ui_state
3. Tests last — test signal handling, alert generation, phase advancement

## Test Surface

Tests for `check_idle_signals()` need a tempdir-based approach since the method uses `std::fs::read_dir` and `std::fs::remove_file`. The test setup:
- Create a temp dir as the signal directory
- Create a temp dir as the work directory
- Write signal files and optionally artifact files
- Invoke the method and verify: signals deleted, phases advanced, alerts generated

Since `check_idle_signals()` is a method on State that accesses `self.signal_dir`, `self.config.work_dir`, `self.threads`, `self.dag`, etc., tests need to construct a State with appropriate fields set. The State struct derives Default, so we can create one and set the fields we need.

However, State uses Dag which requires tickets, and accesses `ticket::update_ticket_phase()` which writes to actual files. For unit testing:
- Create real ticket files in a temp dir
- Build a real Dag from those tickets
- Set up State with the temp dirs
- Write signal files
- Call the method
- Assert: signal files deleted, ticket file frontmatter updated, idle_alerts populated correctly
