---
id: S-018
title: agent-timeout-config
status: open
---

## Agent Timeout Configuration

Add configurable per-agent session timeouts so projects with long-running tasks (big test suites, slow compilation) don't get killed prematurely.

### Motivation

In projects like haul-page (Phoenix/Ash), `mix test` alone takes ~170 seconds. When an agent runs tests as part of its RDSPI cycle, the session can exceed default expectations. With 2 concurrent agents both running tests, the wall-clock time compounds. Projects need a way to say "give agents more time" without modifying lisa internals.

### Scope

- New `.lisa.toml` config: `session_timeout_secs` (default: current behavior)
- Plugin respects the timeout when monitoring active sessions
- Timeout applies to the overall agent session, not individual phases
- When a session times out: log it, mark the thread as timed-out, free the slot
- Stretch: per-phase timeout overrides (e.g., implement phase gets more time than research)
