# T-012-03 Structure: Fill placeholder URLs and fix dead code warnings

## Files modified

### 1. `docs/knowledge/lisa-loop-setup-guide.md`
- Line 18: Replace `git clone <lisa-repo-url>` with `git clone https://github.com/johnhkchen/lisa`
- No other changes

### 2. `crates/lisa-plugin/src/ui.rs`

**Struct changes (remove `pane_id` field):**
- `ActiveThread` (line 142): remove `pub pane_id: u32,`
- `ParkedThread` (line 153): remove `pub pane_id: u32,`
- `SlotInfo` (line 180): remove `pub pane_id: u32,`

**Test changes (remove `pane_id` from struct literals):**
- All `ActiveThread { ... }` literals in tests: remove `pane_id: N,`
- All `ParkedThread { ... }` literals in tests: remove `pane_id: N,`
- All `SlotInfo { ... }` literals in tests: remove `pane_id: N,`

### 3. `crates/lisa-plugin/src/lib.rs`

**`build_plugin_state()` method changes:**
- `ActiveThread` construction (~line 1970): remove `pane_id: t.pane_id,`
- `ParkedThread` construction (~line 2000): remove `pane_id: t.pane_id,`
- `SlotInfo` construction (~line 2057): remove `pane_id: s.pane_id,`

## Files NOT modified
- `crates/lisa-plugin/src/scheduler.rs` — no UI structs used here
- `crates/lisa-core/` — no dependency on UI structs
- `crates/lisa-cli/` — no dependency on UI structs

## No new files created
## No files deleted
## No interface changes (these are internal display structs)
