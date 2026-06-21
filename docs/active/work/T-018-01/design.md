# Design: T-018-01 timeout-config-parsing

## Decision Summary

Follow the established pattern used by `review_timeout_secs`: add the field at all three layers (core types, CLI config, KDL layout passthrough) with a sensible default. Default: 900 seconds (15 minutes). Zero means disabled (no timeout).

## Options Considered

### Option A: Follow existing review_timeout_secs pattern exactly

Add `session_timeout_secs` as a `u64` field everywhere, with a constant default in `PluginConfig`. The CLI parses it from TOML, resolves defaults, and passes it through the KDL layout to the WASM plugin.

- Pros: Consistent with existing code. Minimal diff. Easy to review.
- Cons: None significant.

### Option B: Use Option<u64> in PluginConfig to distinguish "not set"

Keep the field as `Option<u64>` in `PluginConfig` rather than defaulting to a concrete value.

- Pros: Allows downstream code to distinguish "user explicitly set 900" from "default 900".
- Cons: Inconsistent with how `stuck_threshold_secs` and `review_timeout_secs` work (both are `u64` with defaults). Forces all consumers to handle `None`.

### Option C: Separate "enabled" flag from the timeout value

Add both `session_timeout_enabled: bool` and `session_timeout_secs: u64`.

- Pros: Explicit enable/disable.
- Cons: Over-engineered for the use case. `0 = disabled` is a well-understood convention (used by `review_timeout_secs` already: "Set to 0 to disable").

## Decision: Option A

Follow the existing pattern. `session_timeout_secs` is a `u64` defaulting to 900. A value of 0 means "no timeout" (consistent with `review_timeout_secs` docstring which says "Set to 0 to disable").

## Default Value: 900 seconds (15 minutes)

Rationale:
- The story mentions 170s test suites as the motivating case. 15 minutes gives ample headroom.
- Most RDSPI phases (research, design, structure, plan, review) should complete well within 15 minutes.
- Implement phase is the longest but even complex implementations rarely exceed 15 minutes of wall-clock.
- Users with slow test suites can increase to 1800s (30min) as shown in the ticket example.
- This is the _session_ timeout, not per-phase. A full RDSPI cycle with 6 phases at 15 min each would be 90 minutes max.

Wait — re-reading the ticket: "maximum wall-clock time a **single agent session** can run." This means the entire RDSPI cycle for one ticket. So 900s might be too short for a full cycle. Let me reconsider.

A typical session runs all 6 phases. If each artifact phase takes ~3-5 minutes and implement takes ~10-15 minutes, total wall time is 30-45 minutes. A 15-minute timeout would kill sessions prematurely.

**Revised default: 1800 seconds (30 minutes).** This matches the ticket's example value and gives enough room for a full RDSPI cycle while still catching genuinely stalled sessions within a reasonable timeframe.

## Semantic Validation

- `session_timeout_secs = 0`: Valid, means disabled.
- No minimum enforced (unlike `max_threads` which rejects 0).
- No relationship validation with `stuck_threshold_secs` — they serve different purposes (per-phase vs total session).

## Display in CLI Output

### `lisa validate`
Add a line to the success message: `"session_timeout: 1800s (30m)"` or similar. This goes in `print_diagnostics()`.

### `lisa status`
Add to the summary header block, after the DAG stats line. Something like:
```
Config: max_threads=2, session_timeout=1800s
```

This keeps it compact and informational without cluttering the wave output.

## What Was Rejected

- **Option B** (Option<u64> in PluginConfig): Inconsistent, unnecessary complexity.
- **Option C** (separate enabled flag): Over-engineered.
- **15-minute default**: Too short for full RDSPI cycles.
- **Validation that session_timeout > stuck_threshold**: Not needed; they're orthogonal concepts.
