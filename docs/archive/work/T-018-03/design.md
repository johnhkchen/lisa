# Design: T-018-03 Per-Phase Timeout

## Problem

A single `session_timeout_secs` doesn't fit all phases. Research/Design are fast (~5 min), but Implement can run 30+ minutes with compilation and test suites. Projects need per-phase control without inflating the global timeout.

## Options Considered

### Option A: HashMap<Phase, u64> in PluginConfig

Add `phase_timeouts: HashMap<Phase, u64>` to `PluginConfig`. In `check_session_timeouts()`, look up the current phase's timeout, falling back to `session_timeout_secs`.

**Pros:** Direct, simple lookup, type-safe keys.
**Cons:** HashMap has overhead for 6 entries. Phase must impl Hash (already does).

### Option B: Flat fields per phase

Add `timeout_research_secs`, `timeout_design_secs`, etc. as separate `Option<u64>` fields.

**Pros:** No HashMap, explicit fields.
**Cons:** 6 new fields, verbose code, annoying to iterate, harder to extend.

### Option C: Array-based lookup

Use `[u64; 8]` indexed by phase discriminant.

**Pros:** Zero allocation, fast lookup.
**Cons:** Fragile (depends on enum ordering), not self-documenting, serde is awkward.

## Decision: Option A — HashMap<Phase, u64>

HashMap is the right abstraction. 6 entries is trivial overhead. The Phase enum already implements Hash+Eq. It maps cleanly to TOML sub-tables and is easy to iterate for display.

## Timeout Semantics

**Per-phase timeout measures time-in-phase, not total session time.**

When `phase_timeouts` is configured:
- Each phase gets its own timeout, measured from `last_phase_change`
- On phase transition, the timer resets automatically (because `last_phase_change` is updated)
- Missing phase entries fall back to `session_timeout_secs`
- `session_timeout_secs` still acts as a global cap (total wall-clock)

This means a session has TWO timeout checks:
1. **Global:** `now - started_at >= session_timeout_secs` (existing behavior, unchanged)
2. **Per-phase:** `now - last_phase_change >= phase_timeout(current_phase)` (new)

Either one triggers timeout. This prevents a session from running forever just because it keeps changing phases.

## TOML Config Format

```toml
[scheduling]
session_timeout_secs = 900        # global cap (still enforced)

[scheduling.phase_timeouts]
research = 300
design = 300
structure = 300
plan = 300
implement = 1800
review = 600
```

Matches the acceptance criteria exactly. Partial entries work — unlisted phases fall back to `session_timeout_secs`.

## Zellij Config Map (from_config_map)

Zellij passes config as flat `BTreeMap<String, String>`. Convention for nested keys:
```
phase_timeout_research = "300"
phase_timeout_design = "300"
phase_timeout_implement = "1800"
```

Parse by iterating keys with `phase_timeout_` prefix, parsing the suffix as a Phase.

## Display Format

When per-phase timeouts are configured, show them in validate/status:

```
Config: max_threads=2, session_timeout=900s
  phase_timeouts: research=300s design=300s implement=1800s
```

Only show phases that have explicit overrides (not fallbacks).

## What Was Rejected

- **Option B (flat fields):** Too verbose. Adding a phase to the enum would require adding another field. The HashMap approach scales without code changes.
- **Option C (array):** Too fragile. Relies on enum discriminant ordering, which is an implementation detail.
- **Removing the global session timeout:** Considered making per-phase timeout replace the global entirely, but the acceptance criteria explicitly keeps `session_timeout_secs` as the default fallback. And a global cap is a useful safety net.
- **Resetting `started_at` on phase change:** Would break the global session timeout. Better to keep both timestamps independent.
