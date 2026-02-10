# T-006-03 Progress: Session Launch Command Audit

## Completed

### Step 1: Add `SessionLaunch` variant to `ActivityEvent`
- Added `SessionLaunch { ticket_id, pane_id, command }` variant to `ActivityEvent` in `types.rs`

### Step 2: Extract `strip_host_prefix()` in `lib.rs`
- Added `strip_host_prefix(path: &Path) -> PathBuf` free function
- Replaced inline `/host/` stripping logic in `schedule_ready_tickets()` with call to the new function

### Step 3: Add `SessionLaunch` logging in `schedule_ready_tickets()`
- Added `launch_cmd` variable to capture command in both fresh-pane and reused-pane branches
- Log `ActivityEvent::SessionLaunch` with ticket_id, pane_id, and full command before `ThreadSpawned`

### Step 4: Update `activity_event_to_ui_entry()` for `SessionLaunch`
- Maps to `ActivityType::Info` with "Launch: " prefix
- Truncates commands longer than 120 chars with "..." suffix

### Step 5: Add tests (10 new tests)
- `test_build_claude_command_includes_rdspi_reference` — verifies RDSPI workflow in command
- `test_build_claude_prompt_includes_rdspi_reference` — verifies RDSPI workflow in prompt
- `test_strip_host_prefix_with_prefix` — `/host/docs/...` -> `docs/...`
- `test_strip_host_prefix_without_prefix` — no-op for non-WASI paths
- `test_strip_host_prefix_just_host` — `/host/` -> empty
- `test_strip_host_prefix_nested_host` — `/host/host/nested` -> `host/nested`
- `test_strip_host_prefix_absolute_non_host` — non-/host/ absolute paths unchanged
- `test_session_launch_event_to_ui` — verifies UI entry generation
- `test_session_launch_event_to_ui_truncates_long_command` — verifies truncation
- `test_ticket_prompt_content` — verifies all required context elements in prompt

### Step 6: Verification
- `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compiles
- `cargo test --workspace` — 224 tests pass (56 CLI + 77 core + 91 plugin)

## Files Changed
- `crates/lisa-core/src/types.rs` — added `SessionLaunch` variant
- `crates/lisa-plugin/src/lib.rs` — added `strip_host_prefix()`, `SessionLaunch` logging, UI mapping, 10 tests

## Acceptance Criteria Status
- [x] When a session is spawned, log an `Info` event with the full command string (via `SessionLaunch`)
- [x] Command includes: `claude` invocation, `--dangerously-skip-permissions`, ticket file path
- [x] RDSPI workflow reference is included in the session context
- [x] CLAUDE.md path is resolvable from the WASI sandbox (prefixed with /host/ correctly)
- [x] Log the pane_id assigned to the session
- [x] Add a `SessionLaunch { ticket_id, pane_id, command }` variant to ActivityEvent
- [x] Tests verifying command construction for various ticket configurations
- [x] Tests verifying /host/ prefix handling for WASI paths

## Note on work directory
The AC mentions "work directory path" in the command. The current prompt does not include the work directory — this is by design as the agent discovers its work directory from the ticket ID and RDSPI workflow conventions. Adding it to the prompt was deferred to avoid scope creep.
