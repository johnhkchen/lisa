# Review: T-018-03 Per-Phase Timeout

## Summary of Changes

### Files Modified

| File | Lines Changed | What |
|------|--------------|------|
| `crates/lisa-core/src/types.rs` | +50 | `phase_timeouts` field, `timeout_for_phase()`, `Phase::from_name()`, 6 tests |
| `crates/lisa-cli/src/config.rs` | +65 | TOML parsing, validation, resolution, default template, 6 tests |
| `crates/lisa-plugin/src/lib.rs` | +165 | Dual timeout check (global + per-phase), 4 tests |
| `crates/lisa-cli/src/init.rs` | +5 | Display per-phase timeouts in validate output |
| `crates/lisa-cli/src/status.rs` | +5 | Display per-phase timeouts in status output |

No files created or deleted.

## Acceptance Criteria Verification

- **New optional config `[scheduling.phase_timeouts]`**: Implemented. TOML sub-table with phase names as keys, u64 values in seconds. Partial entries supported.
- **Per-phase values override `session_timeout_secs`**: `timeout_for_phase()` returns phase-specific value if present, falls back to `session_timeout_secs`.
- **Phase transition resets timeout**: Uses `last_phase_change` timestamp, which is already reset in `check_artifact_advances()` on every phase transition.
- **Missing phase entries fall back to `session_timeout_secs`**: Verified in test `test_per_phase_timeout_fallback_to_global`.
- **Absent `phase_timeouts` section = unchanged behavior**: When `phase_timeouts` is empty, the per-phase check is skipped entirely. Existing global-only behavior unchanged.
- **`lisa validate` shows per-phase timeouts**: Added conditional display line.
- **Unit tests**: 16 new tests covering mixed config, partial overrides, phase-specific triggers, fallback behavior, global cap enforcement.

## Test Coverage

| Area | New | Existing | Total |
|------|-----|----------|-------|
| types.rs (core) | 6 | 22→96 | 102 |
| config.rs (cli) | 6 | 23 | 29+ |
| lib.rs (plugin) | 4 | ~148 | 152 |
| **Workspace** | **16** | **~136** | **152** |

Key test scenarios:
1. Per-phase timeout triggers when time-in-phase exceeds limit
2. Per-phase timeout does NOT trigger within limit
3. Phases without overrides fall back to global session timeout
4. Global session timeout still enforced even with generous per-phase values
5. Config parsing from TOML sub-table
6. Config parsing from Zellij flat key-value (`phase_timeout_{name}`)
7. Validation warns on unknown phase names
8. Empty phase_timeouts = unchanged behavior

## Architecture Notes

- **Dual timeout model**: Global (`started_at` vs `session_timeout_secs`) + per-phase (`last_phase_change` vs `timeout_for_phase(phase)`). Either can trigger timeout.
- **No phase timer reset needed**: `last_phase_change` is already reset in `check_artifact_advances()` — the per-phase timeout gets its reset for free.
- **Config layering**: TOML uses string keys (`HashMap<String, u64>`) because serde/toml doesn't know about `Phase`. The plugin uses `HashMap<Phase, u64>` via `from_config_map()`. CLI stays with string keys since it only displays them.

## Open Concerns

1. **No CLI flag override for phase timeouts**: Unlike `max_threads` which has CLI flag support, phase timeouts can only be set via `.lisa.toml`. This seems fine — per-phase tuning is a project-level concern, not a session override.

2. **Zellij config map convention**: The flat key convention (`phase_timeout_research`) is undocumented outside the code. If the KDL layout generation in `loop_cmd.rs` needs to pass these to the plugin, it would need to convert from `ResolvedConfig.phase_timeouts` (string keys) to `phase_timeout_{name}` format. This isn't wired up yet — it will need a follow-up if the plugin is configured via the KDL layout rather than file-system config.

3. **No validation of timeout values**: We don't enforce minimum values (e.g., phase_timeout_research = 1 is technically valid but probably a mistake). The existing `session_timeout_secs` also has no minimum validation, so this is consistent.
