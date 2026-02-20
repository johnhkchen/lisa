# T-012-03 Research: Fill placeholder URLs and fix dead code warnings

## Placeholder URLs

### Confirmed placeholder
- `docs/knowledge/lisa-loop-setup-guide.md` line 18: `git clone <lisa-repo-url>`
- This is the only instance of `<lisa-repo-url>` in the codebase (outside ticket/work docs that reference it)
- Target replacement: `https://github.com/johnhkchen/lisa`

### Not placeholders (intentional template syntax)
- Same file lines 70/73/76: `<your-build-command>`, `<your-test-command>`, `<your-lint-command>`
- These are inside a CLAUDE.md template section telling users what to fill in for their own projects
- They should NOT be replaced

### No other placeholder patterns found
- Searched `<*-url>` and similar patterns across all `.md` files
- No other placeholder URLs exist

## Dead code warnings: `pane_id` in ui.rs

### Structs with the warning
1. `ActiveThread` (line 142) — `pub pane_id: u32`
2. `ParkedThread` (line 153) — `pub pane_id: u32`
3. `SlotInfo` (line 180) — `pub pane_id: u32`

### Usage analysis

**lib.rs** has its own `AgentSlot` struct with `pane_id` that is heavily used (lookups, signal handling, spawning). This is a different struct from the ui.rs display types.

**ui.rs rendering code** reads `slot_number` from all three structs but never reads `pane_id`. The `pane_id` values are populated when constructing `PluginState` in `build_plugin_state()` (lib.rs lines ~1958-2057) but never accessed by any rendering function.

**Tests** set `pane_id` in struct literals but only because the field exists and must be initialized. No test asserts on `pane_id` values from these UI structs.

### Conclusion
The `pane_id` field on these three UI structs is genuinely dead — populated but never read. The rendering code uses `slot_number` instead. The field can be safely removed, which is cleaner than prefixing with `_`. Removing it also simplifies test construction.

## Files to modify
1. `docs/knowledge/lisa-loop-setup-guide.md` — replace placeholder URL
2. `crates/lisa-plugin/src/ui.rs` — remove `pane_id` from 3 structs + all test usages
3. `crates/lisa-plugin/src/lib.rs` — remove `pane_id` from `build_plugin_state()` construction
4. `docs/active/tickets/T-012-03-urls-and-warnings.md` — update phase frontmatter
