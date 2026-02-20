# T-012-03 Plan: Fill placeholder URLs and fix dead code warnings

## Step 1: Fix placeholder URL in setup guide
- Edit `docs/knowledge/lisa-loop-setup-guide.md` line 18
- Replace `<lisa-repo-url>` with `https://github.com/johnhkchen/lisa`
- Verify: grep confirms no remaining `<lisa-repo-url>` in docs

## Step 2: Remove `pane_id` from UI structs in ui.rs
- Remove `pub pane_id: u32,` from `ActiveThread` (line 142)
- Remove `pub pane_id: u32,` from `ParkedThread` (line 153)
- Remove `pub pane_id: u32,` from `SlotInfo` (line 180)

## Step 3: Remove `pane_id` from build_plugin_state() in lib.rs
- Remove `pane_id: t.pane_id,` from ActiveThread construction
- Remove `pane_id: t.pane_id,` from ParkedThread construction
- Remove `pane_id: s.pane_id,` from SlotInfo construction

## Step 4: Remove `pane_id` from all test struct literals in ui.rs
- Remove `pane_id: N,` from every `ActiveThread`, `ParkedThread`, and `SlotInfo` literal in tests

## Step 5: Verify
- `cargo check -p lisa-plugin --target wasm32-wasip1` — zero warnings
- `cargo test --workspace` — all tests pass

## Testing strategy
- No new tests needed — this is purely removing dead code and fixing a docs placeholder
- Existing tests validate that rendering still works without the field
- Compiler validates that no code reads the removed field
