# Structure: External Project Dogfood (T-009-03)

## Files Modified

### 1. `crates/lisa-plugin/src/lib.rs`

**Line 30**: Change path in `ticket_prompt()`:
```
- "...and the RDSPI workflow in docs/knowledge/rdspi-workflow.md..."
+ "...and the RDSPI workflow in docs/rdspi-workflow.md..."
```

**Line 1805**: Update test assertion in `test_build_claude_command`:
```
- cmd.contains("docs/knowledge/rdspi-workflow.md"),
+ cmd.contains("docs/rdspi-workflow.md"),
```

**Line 1892**: Update test assertion in `test_ticket_prompt`:
```
- assert!(prompt.contains("docs/knowledge/rdspi-workflow.md"));
+ assert!(prompt.contains("docs/rdspi-workflow.md"));
```

## Files Created

None.

## Files Deleted

None.

## Module Boundaries

No changes to module structure, public interfaces, or dependencies.
The change is entirely within `ticket_prompt()` — a private function that
builds the prompt string sent to Claude Code sessions.

## Verification

- `cargo test --workspace` must pass with updated assertions
- `cargo check -p lisa-plugin --target wasm32-wasip1` must compile
- The prompt string must reference `docs/rdspi-workflow.md` consistently
  with what `lisa init` creates and `lisa validate` checks
