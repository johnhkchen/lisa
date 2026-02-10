# T-004-02 Structure: Review Gate Alerts

## Files Modified

### `crates/lisa-plugin/src/ui.rs`

**Colors module** (line 16-31):
- Add `pub const BG_YELLOW: &str = "\x1b[43m";`

**New function: `render_attention_banner()`** — placed between `render_separator()` and `compute_dag_layers()` (around line 250):

```
fn render_attention_banner(state: &PluginState, width: usize, output: &mut Vec<String>)
```

Logic:
1. Collect tickets where `phase == Phase::Review`
2. If none, return immediately (append nothing)
3. Build a lookup `HashMap<&str, &ParkedThread>` from `state.parked_threads` keyed by `ticket_id`
4. Render top border: `╔═══...═══╗` in BRIGHT_YELLOW+BOLD
5. Render header: `║  ⚠ ATTENTION NEEDED  ║` in BRIGHT_YELLOW+BOLD
6. For each review ticket:
   - Look up matching parked thread
   - Get artifact_path from parked thread or `"—"`
   - Get wait_time via `format_time_since(parked_at, current_time)` or `"—"`
   - Truncate title to 20 chars
   - Render: `║  {id:<10} {title:<20} {artifact:<16} {wait:>8}  ║` in YELLOW
7. Render bottom border: `╚═══...═══╝` in BRIGHT_YELLOW+BOLD

**Modify `render_dashboard_lines()`** (line 655):
- Insert call to `render_attention_banner(state, width, &mut output)` after the title bar separator and blank line, before `render_dag()`.

**New tests** in `mod tests`:
- `test_render_attention_banner_with_review_tickets`
- `test_render_attention_banner_empty`
- `test_render_attention_banner_no_parked_thread`
- `test_attention_banner_in_full_dashboard`

## Files NOT Modified

- `crates/lisa-plugin/src/lib.rs` — no changes needed; `to_ui_state()` already populates parked threads and ticket phases correctly
- `crates/lisa-core/src/types.rs` — no new types needed; existing Phase::Review and TicketStatus cover the filtering
- `crates/lisa-core/src/` — no changes

## Module Boundaries

The banner is entirely within the UI rendering layer (`ui.rs`). It consumes the existing `PluginState` struct with no changes. No new public API surfaces.

## Ordering

Single file change, no ordering constraints. All changes go into `ui.rs`.
