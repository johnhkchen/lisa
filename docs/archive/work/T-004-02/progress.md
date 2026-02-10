# T-004-02 Progress: Review Gate Alerts

## Completed

### Step 1: Color constant
- Added `BG_YELLOW` to colors module in ui.rs

### Step 2: `render_attention_banner()` function
- Implemented unified banner that handles both review-phase tickets AND health alerts (stuck/failed)
- Review tickets show: ID, title (truncated 20 chars), artifact filename, wait time
- Health alerts show: icon, ticket ID, detail message
- Uses box-drawing characters (╔═╗║╚╝) for visual weight
- Yellow foreground + bold for prominence
- BG_YELLOW highlight on header text
- Graceful fallback: "—" when no parked thread exists for a review ticket
- Merged with concurrent T-003-03 health alerts banner to avoid duplication

### Step 3: Dashboard integration
- Banner renders between title bar and DAG section in `render_dashboard_lines()`
- No output when no tickets need attention (zero vertical space cost)

### Step 4: Tests (4 new tests)
- `test_render_attention_banner_with_review_tickets` — validates full banner with ticket data
- `test_render_attention_banner_empty` — no output when no review tickets
- `test_render_attention_banner_no_parked_thread` — graceful "—" placeholders
- `test_attention_banner_in_full_dashboard` — banner appears before DAG

### Step 5: Build verification
- `cargo check -p lisa-plugin --target wasm32-wasip1` ✓
- `cargo test --workspace` — 151 tests pass ✓

### Integration with concurrent changes
- T-003-03 added `AlertType`, `HealthAlert`, `alerts` field to `PluginState`, `Warning` ActivityType variant
- Merged both attention banner implementations into one unified function
- Added `alerts` computation to `to_ui_state()` in lib.rs using `Thread::health()` from T-004-01
- Fixed `Warning` match arm in `render_activity_log`

## Deviations from Plan

- Combined review-gate banner with health alerts banner (from concurrent T-003-03 work) into a single unified `render_attention_banner()` function instead of having two separate banners
- Added health alert rendering rows alongside review ticket rows in the same box
