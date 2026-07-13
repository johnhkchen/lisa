---
id: S-032-01
title: informative-pane-names
type: story
status: done
priority: medium
tickets: [T-032-01]
---

# S-032-01: Informative pane lifecycle names

## Outcome

An operator can scan the Zellij tab bar and immediately answer:

- Which ticket is running in this pane?
- What is the ticket about?
- Is Claude or Codex actually working it?
- Is the pane assigned or idle?

Lisa owns the display name as scheduler state changes instead of relying on
Claude's prompt-derived title or Codex's process title. The displayed provider
is the resolved/actual agent, including per-ticket overrides and provider
fallbacks, rather than merely the requested frontmatter value.

## Naming contract

A concrete punctuation and truncation scheme is left to the RDSPI design, but
the information hierarchy is fixed:

- assigned: `<actual-agent> · <ticket-id> · <ticket-title>`
- idle with a reusable session: `<resident-agent> · idle`
- idle without a resident session: `lisa · idle`

The ticket title comes from parsed frontmatter, not from the initial prompt or
shell command. Transient lifecycle labels may be added if they materially help,
but they must not obscure agent, ticket, or idle state.

## Ticket

- **T-032-01 — Zellij pane lifecycle names:** implement and verify uniform,
  scheduler-owned pane titles across assignment, reuse, switching, and release.

## Done when

A live mixed Claude/Codex loop shows the same informative naming format in all
coding panes, updates each name when the pane is reused for another ticket, and
changes a completed/released pane to an accurate idle name only after completion
is durable.
