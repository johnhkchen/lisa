# T-004-02 Research: Review Gate Alerts

## Objective

Add an "ATTENTION NEEDED" banner to the top of the Lisa dashboard (above the DAG section) that alerts the user when tickets enter the review phase or otherwise need human attention.

## Relevant Code

### Dashboard Rendering (`crates/lisa-plugin/src/ui.rs`)

The dashboard is rendered by `render_dashboard_lines()` (line 655), which composes sections in order:

1. Title bar with status line (line 659-664)
2. DAG section (line 668)
3. Active threads table (line 674)
4. Parked threads table (line 676)
5. Activity log (line 689)
6. Quick jump section (line 694)

Each section appends to a `Vec<String>` output buffer. The banner must be inserted **between the title bar and the DAG section** (between steps 1 and 2).

### UI State Available for the Banner

`PluginState` (line 184) provides:

- `tickets: Vec<TicketNode>` — each has `id`, `title`, `phase`, `status`, `depends_on`, `blocks`
- `parked_threads: Vec<ParkedThread>` — each has `ticket_id`, `phase`, `artifact_path`, `parked_at`, `pane_id`
- `current_time: Duration` — for computing elapsed wait time

Tickets needing attention can be identified by:
1. `ticket.status == TicketStatus::WaitingReview` — tickets in review phase
2. `ticket.phase == Phase::Review` — equivalent check via phase

Parked threads provide additional context:
- `artifact_path` for the artifact that triggered the review gate
- `parked_at` timestamp for computing how long the ticket has been waiting

### Existing Color Infrastructure

`ui.rs` defines ANSI color codes in the `colors` module (line 16-31):
- `BG_BLUE` exists for the title bar background
- `YELLOW`, `BRIGHT_YELLOW` for review/attention colors
- `BOLD`, `RESET` for emphasis
- No `BG_YELLOW` currently defined — will need to add it

### Parked Thread / Review Data Flow

In `lib.rs`, `check_artifact_advances()` (line 265):
- When a phase artifact is detected, the ticket advances to the next phase
- If the next phase is `Phase::Review`, `thread.park()` is called (line 334)
- The parked thread is included in `to_ui_state()` (line 699-720)

In `to_ui_state()`:
- Parked threads are filtered by `ThreadStatus::Parked`
- Each gets an `artifact_path` computed from `work_dir/ticket_id/artifact_filename`
- `parked_at` is set from `thread.started_at` (this is a known approximation)

### TicketNode Fields

`TicketNode` (line 126-134) has `title: String` and `blocks: Vec<String>` fields that are currently read but trigger dead_code warnings because they're only used in test construction. The banner will use `title`, naturally resolving one of these warnings.

### Existing Format Helpers

- `format_time_since(timestamp, current_time)` (line 230) — computes elapsed duration string
- `format_duration(duration)` (line 214) — formats Duration as "2m 30s", "1h 5m"
- `render_separator(width)` (line 247) — renders `─` horizontal rule

## Data Sources for the Banner

The banner needs: ticket ID, title, artifact path, time waiting.

**Ticket ID and title**: Available from `PluginState::tickets` (Vec<TicketNode>). Filter by `status == WaitingReview` or `phase == Phase::Review`.

**Artifact path**: Available from `PluginState::parked_threads` (Vec<ParkedThread>). Join on `ticket_id` to match with ticket.

**Time waiting**: `parked_threads[].parked_at` compared to `current_time` using `format_time_since()`.

Edge case: A ticket could be in Review phase without a corresponding parked thread (e.g., manually set via `mark_ticket_done` modal, or thread was removed by stale detection). The banner should still show these tickets, just without artifact path or wait time.

## Rendering Constraints

- Terminal width: `render_dashboard_lines` receives `width` (capped at 100) and `height`
- The banner takes vertical space from the activity log area (line 684-686 computes remaining)
- Banner should be compact: 2-3 lines for header/border + 1 line per review ticket
- Should auto-hide when empty (no wasted vertical space)

## Dependencies Satisfied

- **T-003-02** (artifact-phase-advance): Implemented. `check_artifact_advances()` detects artifacts, advances phases, parks threads at review. ✓
- **T-004-01** (session-status-model): Implemented. `Thread::health()`, `HealthStatus`, `is_attention_needed()`, stuck detection. ✓

The health model from T-004-01 enables future extension to also show stuck/failed threads in the banner, but the current AC scopes the banner to review-phase tickets only.

## Summary

The implementation is straightforward:
1. Add `BG_YELLOW` ANSI code to the colors module
2. Add a `render_attention_banner()` function that filters tickets in review phase, joins with parked thread data, and renders a colored banner
3. Call it from `render_dashboard_lines()` between title bar and DAG
4. Add tests for: banner with review tickets, banner with no review tickets (hidden), banner formatting
