# T-004-02 Design: Review Gate Alerts

## Decision: Banner Placement and Data Source

### Approach: Filter tickets + join parked threads

The banner filters `PluginState::tickets` for `phase == Phase::Review`, then enriches each entry by looking up the matching `ParkedThread` for artifact path and wait time.

**Why this approach:**
- Catches all review-phase tickets, even if no parked thread exists (e.g., thread was cleaned up by stale detection)
- ParkedThread data is best-effort enrichment, not required
- Aligns with how the dashboard already separates ticket data from thread data

**Rejected: Only use parked_threads as data source**
- Would miss tickets manually set to review phase
- Would miss tickets whose thread was removed by stale detection

### Banner Rendering Design

```
╔══════════════════════════════════════════════════╗
║  ⚠ ATTENTION NEEDED                             ║
║  T-003-02  artifact-phase-adv  design.md  5m 30s║
║  T-004-01  session-status-mod  plan.md    2m 15s║
╚══════════════════════════════════════════════════╝
```

Key decisions:
1. **Box drawing characters** (`╔╗╚╝║═`) for visual weight — distinguishes from the `─` separators used elsewhere
2. **Yellow foreground + bold** rather than BG_YELLOW — background colors in terminal emulators can clash with user themes and make text hard to read. Bold yellow foreground is safer and still visually prominent.
3. **Compact layout**: header line + 1 line per ticket + bottom border
4. **Title truncation**: Ticket titles truncated to ~20 chars to fit in 80-col terminals
5. **Graceful degradation**: If no parked thread exists for a review ticket, show `—` for artifact path and wait time

### Column Layout (within 80-char width)

```
║  {id:<10} {title:<20} {artifact:<16} {wait_time:>8} ║
```

- `id`: 10 chars (e.g., "T-004-02")
- `title`: 20 chars truncated (e.g., "artifact-phase-adv..")
- `artifact`: 16 chars (e.g., "design.md" or "—")
- `wait_time`: 8 chars right-aligned (e.g., "5m 30s" or "—")
- Padding/borders: ~6 chars

Total: ~60 visible chars + ANSI codes. Fits comfortably.

### Integration into render_dashboard_lines()

Insert `render_attention_banner()` call between the title bar/separator and the DAG section:

```rust
// Title bar
...
output.push(render_separator(width));
output.push(String::new());

// NEW: Attention banner (only if tickets need attention)
render_attention_banner(state, width, &mut output);

// DAG section
render_dag(state, &mut output);
```

The function returns early (appends nothing) when no tickets are in review — zero vertical space cost when not needed.

### Color Choice

Add `BG_YELLOW` to colors module: `\x1b[43m`. Use it sparingly — only for the header line `⚠ ATTENTION NEEDED`. Ticket lines use `YELLOW` + `BOLD` foreground only.

Actually, reconsidering: use `BRIGHT_YELLOW` foreground with `BOLD` for the entire banner border and header. This is consistent with how `Phase::Review` already uses `BRIGHT_YELLOW` in the DAG. The box-drawing characters provide enough visual weight.

### Test Plan

1. `test_render_attention_banner_with_review_tickets` — banner renders when tickets are in review
2. `test_render_attention_banner_empty` — no output when no review tickets
3. `test_render_attention_banner_no_parked_thread` — ticket in review but no matching parked thread shows dashes
4. `test_attention_banner_in_dashboard` — full dashboard includes banner when review tickets exist
5. `test_attention_banner_not_in_dashboard` — full dashboard omits banner when no review tickets

### What's NOT in scope

- Showing stuck/failed threads in the banner (future T-004-03 or similar)
- Sound/system notifications
- Keyboard shortcut from banner to jump to review artifact
