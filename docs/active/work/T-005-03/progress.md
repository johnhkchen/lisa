# Progress: T-005-03 fix-phase-change-detection

## Step 1: Fix rebuild_dag() detection — DONE
Changed `if let Some(...)` to `match` with `None` arm that treats non-Ready first-seen tickets as changes.

## Step 2: Move done-ticket detection out of `if changed` — DONE
Removed `if changed` gating. Done-ticket detection, thread phase sync, and sweep all run unconditionally every tick.

## Step 3: Add sweep_stale_slots() — DONE
New method iterates agent_slots, finds slots pointing at Done tickets in DAG, releases them with warning log.

## Step 4: Add tests — DONE
- `test_done_ticket_detected_on_first_poll`: empty last_phases + done ticket → detected, slot released
- `test_done_ticket_detected_between_polls`: Research → Done transition → detected, slot released
- `test_sweep_stale_slots_releases_done_ticket`: orphaned slot + done ticket → swept, warning logged

## Verification
- `cargo test --workspace`: 175 tests pass (49 CLI + 59 core + 67 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: compiles clean (only pre-existing warnings)
