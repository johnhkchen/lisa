# Structure: T-006-04 Runtime State Snapshot

## Files Modified

### `crates/lisa-plugin/src/lib.rs`

1. **`State::format_snapshot(&self) -> String`** — new method on State impl block (the one starting at line 120). Pure function that reads all state fields and formats a multi-section text dump. Returns a String.

2. **`State::handle_key()`** — add a `BareKey::Char('D')` arm in the normal-mode section (after the existing `BareKey::Char('d')` arm at line 718). This arm:
   - Calls `self.format_snapshot()`
   - Writes the result to `/host/.lisa-state-dump.txt` via `std::fs::write`
   - Logs an `ActivityEvent::Info` message
   - Returns `true` to trigger re-render

3. **`State::format_activity_event(event: &ActivityEvent) -> String`** — helper method that formats a single ActivityEvent to a one-line string representation.

## No New Files

All changes are in `lib.rs`. The snapshot is a single method on the existing State struct — no new modules needed.

## No Changes to Other Crates

- `lisa-core`: No changes needed. Dag, Thread, PluginConfig, ActivityEvent types already expose all needed data via public fields and methods.
- `lisa-cli`: No changes needed.

## Public Interface

No new public API. `format_snapshot()` is a method on the private `State` struct. `format_activity_event()` is a private helper.

## Module Boundaries

The snapshot formatter needs access to:
- `State.dag` — uses `tickets()`, `get_dependencies()`, `stats()`
- `State.threads` — iterates HashMap values
- `State.agent_slots` — iterates Vec (AgentSlot is private to lib.rs)
- `State.config` — reads all fields
- `State.activity_log` — last 50 entries
- `State.last_health` — HashMap iteration
- Various bool flags (initialized, permissions_granted, etc.)

All of these are fields on the private State struct, accessible from within lib.rs.

## Test Location

Tests go in the existing `#[cfg(test)] mod tests` block at the bottom of `lib.rs` (line 1212+). They follow the existing pattern of constructing State with tempdir-based ticket files and DAGs.
