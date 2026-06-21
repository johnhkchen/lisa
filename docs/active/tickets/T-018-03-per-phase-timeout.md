---
id: T-018-03
story: S-018
title: per-phase-timeout
type: task
status: open
priority: medium
phase: done
depends_on: [T-018-02]
---

## Context

Different RDSPI phases have different expected durations. Research and design are fast; implement can be very slow (compilation, test suites). Allow per-phase timeout overrides so projects can give implement more time without inflating the timeout for everything.

## Acceptance Criteria

- New optional config in `.lisa.toml`:
  ```toml
  [scheduling]
  session_timeout_secs = 900        # default for all phases

  [scheduling.phase_timeouts]
  research = 300                    # 5 minutes
  design = 300
  structure = 300
  plan = 300
  implement = 1800                  # 30 minutes
  review = 600                      # 10 minutes
  ```
- When `phase_timeouts` is set, per-phase values override `session_timeout_secs` for that phase
- When a session transitions phases, the timeout resets with the new phase's limit
- Missing phase entries fall back to `session_timeout_secs`
- If `phase_timeouts` section is absent, behavior is unchanged (single timeout)
- `lisa validate` shows per-phase timeouts when configured
- Unit tests: mixed config, partial overrides, phase transitions reset timer
