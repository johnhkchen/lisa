# T-008-02 Plan: Idle-Aware Phase Advancement

## Step 1: Add `AlertType::IdleWithoutArtifact` to ui.rs

Add the new variant to `AlertType` enum in `crates/lisa-plugin/src/ui.rs`:
```rust
pub enum AlertType {
    Failed,
    Stuck,
    IdleWithoutArtifact,
}
```

Update the match arm in `render_attention_banner()` (line 452) to handle it:
```rust
AlertType::IdleWithoutArtifact => ("⏸ IDLE  ", YELLOW),
```

Tests:
- `test_render_attention_banner_with_idle_alert` — verify IdleWithoutArtifact renders in banner

Commit: "Add IdleWithoutArtifact alert type to UI"

## Step 2: Add State fields and signal_dir initialization in lib.rs

Add to `State` struct:
```rust
signal_dir: PathBuf,
idle_alerts: Vec<(TicketId, String)>,
```

In `load()`, after the `/host/` path prefixing block, add:
```rust
self.signal_dir = host.join(".lisa/signals");
```

No tests needed for this step — it's just field additions.

Commit: "Add signal_dir and idle_alerts fields to plugin State"

## Step 3: Implement `check_idle_signals()` method

New method on `State` in `crates/lisa-plugin/src/lib.rs`:

```rust
fn check_idle_signals(&mut self) {
    self.idle_alerts.clear();

    let entries = match std::fs::read_dir(&self.signal_dir) {
        Ok(entries) => entries,
        Err(_) => return, // Directory doesn't exist or not readable — normal case
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.ends_with(".idle") => name.to_string(),
            _ => continue,
        };

        let ticket_id: TicketId = filename.trim_end_matches(".idle").to_string();

        // Clean up the signal file immediately (prevents re-trigger)
        let _ = std::fs::remove_file(&path);

        // Look up thread
        let (current_phase, thread_running) = match self.threads.get(&ticket_id) {
            Some(t) if t.status == lisa_core::types::ThreadStatus::Running => {
                (t.current_phase, true)
            }
            _ => {
                // No thread or not running — signal is stale, just clean up
                continue;
            }
        };

        match current_phase {
            Phase::Implement => {
                // Idle signal alone is the completion signal for Implement
                let file_path = self.dag.get_ticket(&ticket_id)
                    .map(|t| t.file_path.clone());
                let file_path = match file_path {
                    Some(p) if !p.as_os_str().is_empty() => p,
                    _ => continue,
                };

                if let Err(e) = ticket::update_ticket_phase(&file_path, Phase::Review) {
                    self.log_activity(ActivityEvent::Error {
                        message: format!("Failed to advance {} via idle signal: {}", ticket_id, e),
                    });
                    continue;
                }

                self.log_activity(ActivityEvent::PhaseCompleted {
                    ticket_id: ticket_id.clone(),
                    phase: Phase::Implement,
                });
                self.log_activity(ActivityEvent::TicketPhaseChanged {
                    ticket_id: ticket_id.clone(),
                    old_phase: Phase::Implement,
                    new_phase: Phase::Review,
                });

                if let Some(thread) = self.threads.get_mut(&ticket_id) {
                    thread.current_phase = Phase::Review;
                    thread.last_phase_change = std::time::SystemTime::now();
                    thread.park();
                }
            }

            Phase::Research | Phase::Design | Phase::Structure | Phase::Plan => {
                // Need artifact + idle signal for these phases
                let artifact_name = match current_phase.artifact_filename() {
                    Some(name) => name,
                    None => continue,
                };
                let artifact_path = self.config.work_dir.join(&ticket_id).join(artifact_name);

                if artifact_path.exists() {
                    let next_phase = match current_phase.next() {
                        Some(p) => p,
                        None => continue,
                    };

                    let file_path = self.dag.get_ticket(&ticket_id)
                        .map(|t| t.file_path.clone());
                    let file_path = match file_path {
                        Some(p) if !p.as_os_str().is_empty() => p,
                        _ => continue,
                    };

                    if let Err(e) = ticket::update_ticket_phase(&file_path, next_phase) {
                        self.log_activity(ActivityEvent::Error {
                            message: format!("Failed to advance {} via idle signal: {}", ticket_id, e),
                        });
                        continue;
                    }

                    self.log_activity(ActivityEvent::PhaseCompleted {
                        ticket_id: ticket_id.clone(),
                        phase: current_phase,
                    });
                    self.log_activity(ActivityEvent::TicketPhaseChanged {
                        ticket_id: ticket_id.clone(),
                        old_phase: current_phase,
                        new_phase: next_phase,
                    });

                    if let Some(thread) = self.threads.get_mut(&ticket_id) {
                        thread.current_phase = next_phase;
                        thread.last_phase_change = std::time::SystemTime::now();
                        if next_phase == Phase::Review {
                            thread.park();
                        }
                    }
                } else {
                    // Idle without artifact — alert
                    let detail = format!(
                        "Agent idle in {} phase but {} not found",
                        current_phase, artifact_name
                    );
                    self.idle_alerts.push((ticket_id.clone(), detail.clone()));
                    self.log_activity(ActivityEvent::Warning {
                        message: format!("{}: {}", ticket_id, detail),
                    });
                }
            }

            _ => {
                // Ready, Review, Done — ignore signal (already cleaned up)
            }
        }
    }
}
```

Tests:
- `test_idle_signal_implement_advances_to_review` — implement phase + idle signal -> review, thread parked
- `test_idle_signal_research_with_artifact_advances` — research phase + artifact + idle signal -> design
- `test_idle_signal_research_without_artifact_alerts` — research phase + no artifact + idle signal -> alert
- `test_idle_signal_cleanup` — signal file deleted after processing
- `test_idle_signal_no_thread_ignored` — signal for nonexistent thread is cleaned up silently
- `test_idle_signal_nonrunning_thread_ignored` — signal for parked/completed thread cleaned up

Commit: "Implement check_idle_signals() for idle-aware phase advancement"

## Step 4: Wire into poll_tick() and to_ui_state()

In `poll_tick()`, add after `self.check_artifact_advances()`:
```rust
self.check_idle_signals();
```

In `to_ui_state()`, after the health alerts section, add idle alerts:
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

Tests:
- `test_to_ui_state_includes_idle_alerts` — verify idle_alerts appear in PluginState.alerts

Commit: "Wire idle signal checking into poll_tick and UI state"

## Step 5: Verify

- `cargo test --workspace` — all tests pass
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles

No separate commit.

## Testing Strategy

All tests for `check_idle_signals()` use tempdir-based setup:
1. Create temp directories for signals, work artifacts, and tickets
2. Write ticket files with appropriate frontmatter
3. Build State with temp paths and a real Dag from the tickets
4. Add Thread entries to `self.threads` for the test tickets
5. Write `.idle` signal files to the temp signal directory
6. Optionally write artifact files to the temp work directory
7. Call `check_idle_signals()`
8. Assert: signal files deleted, ticket frontmatter updated, alerts populated, thread state changed

## Verification Criteria

- Implement -> Review advances when idle signal detected (thread parked)
- Research/Design/Structure/Plan advance when idle signal + artifact present
- Alert surfaces when idle signal arrives but artifact is missing
- Signal files are cleaned up after processing (whether acted on or not)
- Signals for non-existent or non-running threads are silently cleaned up
- Existing artifact-based detection continues to work (no changes to `check_artifact_advances()`)
- All existing tests pass
- WASM target compiles
