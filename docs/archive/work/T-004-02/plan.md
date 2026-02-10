# T-004-02 Plan: Review Gate Alerts

## Step 1: Add BG_YELLOW color constant

In `crates/lisa-plugin/src/ui.rs`, add to the `colors` module:
```rust
pub const BG_YELLOW: &str = "\x1b[43m";
```

Verification: compiles, existing tests pass.

## Step 2: Implement `render_attention_banner()`

Add the function between `render_separator()` and `compute_dag_layers()`:

```rust
fn render_attention_banner(state: &PluginState, width: usize, output: &mut Vec<String>)
```

Implementation:
1. Filter `state.tickets` for `ticket.phase == Phase::Review`
2. Early return if empty
3. Build `HashMap<&str, &ParkedThread>` from `state.parked_threads`
4. Compute box width: `min(width, 64)`
5. Render top border with `╔═╗` in BRIGHT_YELLOW+BOLD
6. Render header `⚠ ATTENTION NEEDED` in BRIGHT_YELLOW+BOLD with BG_YELLOW
7. For each review ticket:
   - Lookup parked thread by ticket_id
   - Truncate title to 20 chars
   - Get artifact filename (basename only) or "—"
   - Get wait time string or "—"
   - Render padded line with `║...║`
8. Render bottom border with `╚═╝`

Verification: unit tests (Step 4).

## Step 3: Wire banner into dashboard

In `render_dashboard_lines()`, insert after the title bar separator:

```rust
// Attention banner (review gate alerts)
render_attention_banner(state, width, &mut output);
```

Place it before `render_dag()`. If no review tickets, nothing is added.

Verification: existing `test_full_dashboard_render` still passes. New integration test (Step 4).

## Step 4: Add tests

1. **`test_render_attention_banner_with_review_tickets`**: Create state with 1 ticket in Review + matching parked thread. Assert output contains "ATTENTION NEEDED", ticket ID, artifact path, formatted wait time.

2. **`test_render_attention_banner_empty`**: State with no review tickets. Assert function appends nothing to output.

3. **`test_render_attention_banner_no_parked_thread`**: Ticket in Review but no matching parked thread. Assert banner shows ticket with "—" placeholders.

4. **`test_attention_banner_in_full_dashboard`**: Render full dashboard with review ticket. Assert "ATTENTION NEEDED" appears before "DAG" in the output.

Verification: `cargo test -p lisa-plugin`

## Step 5: Verify build

Run `cargo check -p lisa-plugin --target wasm32-wasip1` and `cargo test --workspace`.

## Commit Strategy

Single commit: all changes are in `ui.rs` and tightly coupled.
