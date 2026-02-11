---
id: S-011
title: Cross-device verification and QoL feedback
type: story
status: Active
created: 2026-02-11
---

# S-011: Cross-device verification and QoL feedback

## Problem

Lisa has been developed and tested on a single device. The S-010 event-driven transition protocol, `just install` workflow, and end-to-end `lisa loop` experience have never been verified on a fresh machine. Before building more features, we need confidence that the tool works reliably when cloned and built from scratch — and we need structured feedback to drive the next round of quality-of-life improvements.

## Goal

Pull the repo on a second device, build and install the `lisa` CLI, run `lisa loop` against a real project, and produce a feedback document cataloging friction points, bugs, and QoL improvement ideas. This document becomes the input for a follow-up story (S-012).

## Tickets

- **T-011-01:** Build and install on a fresh device (chore)
- **T-011-02:** Run lisa loop end-to-end on a real project (spike)
- **T-011-03:** Write cross-device feedback document (task)

## Dependencies

```
T-011-01 (build + install)
  └── T-011-02 (run lisa loop)
        └── T-011-03 (write feedback doc)
```

## Success Criteria

1. `lisa` CLI builds and installs cleanly on a second device
2. `lisa loop` launches Zellij with the plugin, schedules tickets, and transitions between phases
3. Event-driven hooks (Stop, SessionStart[clear]) fire and produce signal files
4. A `docs/active/work/T-011-03/feedback.md` document exists with categorized findings
5. Findings are actionable enough to generate tickets for a follow-up story
