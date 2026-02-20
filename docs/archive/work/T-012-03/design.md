# T-012-03 Design: Fill placeholder URLs and fix dead code warnings

## Decision 1: Placeholder URL replacement

**Approach:** Direct replacement of `<lisa-repo-url>` with `https://github.com/johnhkchen/lisa` on line 18 of `docs/knowledge/lisa-loop-setup-guide.md`.

No alternatives to evaluate — the ticket specifies the exact URL.

The `<your-build-command>` etc. placeholders are intentional template syntax and must not be touched.

## Decision 2: `pane_id` removal strategy

### Option A: Prefix with `_pane_id`
- Suppresses the warning
- Keeps the field for potential future use
- Leaves dead data flowing through the system

### Option B: Remove the field entirely (chosen)
- Eliminates the warning at the source
- Simplifies struct definitions and test construction
- No future use is planned — the rendering code uses `slot_number` for display
- If `pane_id` is ever needed in UI rendering, adding it back is trivial

**Rationale:** These are display-only structs constructed in `build_plugin_state()` and consumed by rendering functions. The rendering functions use `slot_number`, not `pane_id`. Keeping dead fields makes the code misleading — it suggests pane_id matters for rendering when it doesn't. The real `pane_id` tracking lives in `AgentSlot` in lib.rs and is unaffected.

## Risk assessment

Both changes are low-risk:
- URL change is a docs-only edit
- Field removal is fully covered by compilation and existing tests
- If any code actually reads `pane_id` from these UI structs, compilation will fail immediately

## Rejected alternative: Keeping `pane_id` on `SlotInfo` only
Could argue `SlotInfo` might need it since it represents a pane slot. But the rendering code doesn't use it, and `SlotInfo` already has `slot_number` which is what gets displayed. Consistency is better — remove from all three.
