---
id: T-018-01
story: S-018
title: timeout-config-parsing
type: task
status: open
priority: high
phase: done
depends_on: []
---

## Context

Add `session_timeout_secs` to the `.lisa.toml` scheduling config. This is the maximum wall-clock time a single agent session can run before lisa considers it stalled and reclaims the slot.

## Acceptance Criteria

- New optional field in `[scheduling]` section of `.lisa.toml`:
  ```toml
  [scheduling]
  session_timeout_secs = 1800   # 30 minutes, default TBD
  ```
- `PluginConfig` (or equivalent config struct in lisa-core) gains a `session_timeout_secs: Option<u64>` field
- Parsing: reads from TOML, falls back to a sensible default if omitted (e.g., 900s / 15 minutes)
- `lisa validate` reports the configured timeout in its output
- `lisa status` shows the timeout setting in its summary header
- Unit tests: parse with/without the field, verify default, verify override
