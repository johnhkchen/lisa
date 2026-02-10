# T-006-03 Research: Session Launch Command Audit

## Current Session Launch Mechanism

### Two Entry Points in `lib.rs`

Session launching happens in `schedule_ready_tickets()` (lib.rs:265-353). There are two paths depending on slot state:

1. **Fresh pane** (`has_session == false`): Calls `build_claude_command()` which produces `claude --dangerously-skip-permissions "<prompt>"\r` and sends it via `write_chars_to_pane_id`. Then sets `has_session = true`.

2. **Reused pane** (`has_session == true`): First sends `/clear\r` to reset the conversation, then queues `build_claude_prompt()` output (just the prompt text + `\r`) via `pending_pane_writes` for deferred delivery after FLUSH_DELAY_SECS.

### Command Construction Functions (lib.rs:28-48)

```
ticket_prompt(ticket_dir, ticket_id) -> String
  "Read the ticket at {ticket_dir}/{ticket_id}.md, the project context in CLAUDE.md,
   and the RDSPI workflow in docs/knowledge/rdspi-workflow.md.
   Start from the current phase indicated in the ticket frontmatter."

build_claude_command(ticket_dir, ticket_id) -> String
  "claude --dangerously-skip-permissions \"{prompt}\"\r"

build_claude_prompt(ticket_dir, ticket_id) -> String
  "{prompt}\r"
```

### Duplicate in `scheduler.rs`

`Scheduler::build_claude_command()` (scheduler.rs:409-428) has a parallel implementation that builds args as a `Vec<String>` for `open_command_pane_floating`. This path uses `ClaudeCommand { args }` and is invoked by `spawn_claude_session()` (scheduler.rs:359-397). However, this path is **not used** by the actual plugin — `lib.rs` uses `write_chars_to_pane_id` instead. The scheduler module's `spawn_claude_session` is dead code in the current architecture (pre-created slots replaced the floating pane approach).

### Path Handling: /host/ Prefix

The WASI sandbox mounts the host filesystem at `/host/`. The plugin stores `config.ticket_dir` as `/host/docs/active/tickets`. When building the command sent to the agent, `schedule_ready_tickets()` strips the `/host/` prefix (lib.rs:301-308) so the command seen by the spawned Claude Code session uses relative paths like `docs/active/tickets/T-001.md`.

This stripping is correct because the spawned Claude Code session runs on the host, not inside the WASI sandbox. The agent sees the normal filesystem.

### What Gets Logged Today

When a session is spawned, only `ActivityEvent::ThreadSpawned { ticket_id, pane_id }` is logged (lib.rs:334-337). The actual command string is **not captured** anywhere. There's no way to inspect what was sent to the pane after the fact.

## ActivityEvent Variants (types.rs:534-605)

Current variants relevant to this ticket:
- `ThreadSpawned { ticket_id, pane_id }` — logged on spawn, but no command string
- `Info { message }` — generic info messages
- `Error { message }` — generic error messages

There is no `SessionLaunch` variant. The ticket requests adding one.

## Context Elements the Command Should Include

Per the acceptance criteria, the command must include:
1. `claude` invocation with `--dangerously-skip-permissions`
2. `--print` or appropriate flags (currently `--print` is only in scheduler.rs, not lib.rs)
3. Ticket file path
4. Work directory path (currently **not included** in any command)
5. RDSPI workflow reference (`docs/knowledge/rdspi-workflow.md`)
6. CLAUDE.md reference
7. `/host/` prefix handling for WASI paths

Notable gaps in current commands:
- Work directory is not mentioned in the prompt at all
- The `--print` flag from scheduler.rs was dropped when lib.rs switched to `write_chars_to_pane_id`
- No flag differentiation — all sessions use the same flags

## Test Infrastructure

### scheduler.rs Tests
- `test_build_claude_command` (scheduler.rs:758-792) validates the scheduler's parallel implementation
- Tests verify ticket path, RDSPI reference, and CLAUDE.md reference in the prompt

### lib.rs Tests
- No test for `build_claude_command()` or `build_claude_prompt()` in lib.rs
- `schedule_ready_tickets()` can't be tested directly because it calls `write_chars_to_pane_id` (zellij host function)
- Tests work around this by testing preconditions (slot state, thread creation) but not command content

### What's Testable
- `ticket_prompt()`, `build_claude_command()`, and `build_claude_prompt()` are free functions — easily testable
- `ActivityEvent` enum variants can be pattern-matched in tests
- `/host/` stripping logic can be tested by extracting it from `schedule_ready_tickets()`

## Files Involved

| File | Relevance |
|------|-----------|
| `crates/lisa-plugin/src/lib.rs` | Contains the actual launch code, command construction, and scheduling |
| `crates/lisa-core/src/types.rs` | `ActivityEvent` enum needs new `SessionLaunch` variant |
| `crates/lisa-plugin/src/scheduler.rs` | Parallel (unused) command builder — should be reconciled or removed |
| `crates/lisa-plugin/src/ui.rs` | Will need to render the new `SessionLaunch` event |

## Key Observations

1. **Dual command builders**: lib.rs and scheduler.rs have independent command construction. The lib.rs version is what actually runs. The scheduler.rs version is used only in tests.

2. **No command logging**: The full command string is constructed and sent but never captured. Adding `SessionLaunch` to `ActivityEvent` is the right fix.

3. **Missing work directory**: Neither command builder includes the work directory in the prompt. The ticket requests this be included.

4. **`--print` discrepancy**: scheduler.rs uses `--print` for non-interactive mode; lib.rs doesn't use it because sessions are interactive (user can type in the pane). This is intentional — `--print` would make the session non-interactive.

5. **Path stripping is inline**: The `/host/` stripping in `schedule_ready_tickets()` should be extracted for testability.

6. **Reuse path has different content**: Fresh launch sends `claude ...` while reuse sends just the prompt. Both should be logged but the logged "command" semantics differ.
