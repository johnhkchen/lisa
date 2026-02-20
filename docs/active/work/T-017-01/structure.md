# T-017-01 Structure: Fix formatting and clippy warnings

## Files Modified

All modifications are in-place edits. No files created or deleted.

### Formatting (cargo fmt)
15 files across all 3 crates — whitespace-only changes.

### Clippy fixes
- `crates/lisa-cli/src/init.rs` — `map_or` → `is_some_and` (1)
- `crates/lisa-cli/src/loop_cmd.rs` — `map_or` → `is_some_and` (1)
- `crates/lisa-plugin/src/lib.rs` — all 15 plugin warnings
- `crates/lisa-plugin/src/ui.rs` — possible format string warnings

## No new modules, interfaces, or architectural changes.
