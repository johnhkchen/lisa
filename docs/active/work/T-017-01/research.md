# T-017-01 Research: Fix formatting and clippy warnings

## Current State

**Formatting:** 219 diffs across 15 files in all 3 crates. Entirely mechanical — `cargo fmt` will fix all of them.

**Files with formatting diffs:**
- lisa-cli: build.rs, config.rs, detect.rs, doctor.rs, init.rs, loop_cmd.rs, main.rs, setup_guide.rs, status.rs, templates.rs
- lisa-core: diagnostics.rs, ticket.rs, types.rs
- lisa-plugin: lib.rs, ui.rs

**Clippy warnings:** 17 total (2 in lisa-cli, 15 in lisa-plugin).

### lisa-cli (2 warnings)
- `map_or` can be simplified → `is_some_and` (2 instances)

### lisa-plugin (15 warnings)
- `format!` in `format!`/`writeln!` args (7 instances) — use direct interpolation or `format_args!`
- `literal with an empty format string` (3 instances) — inline the literal
- `clamp-like pattern without using clamp` (1 instance) — use `.clamp(min, max)`
- `map_or` can be simplified (2 instances) — use `is_some_and`
- `iter().cloned().collect()` on a slice (1 instance) — use `.to_vec()`
- `redundant closure` (1 instance) — pass function directly

## Risk Assessment

- `cargo fmt`: Zero risk. Pure whitespace changes.
- `cargo clippy --fix`: 7 of 17 are auto-fixable. Remaining 10 need manual review but are all straightforward mechanical transformations.
- No behavioral changes. All fixes are purely cosmetic/idiomatic.
