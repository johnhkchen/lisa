---
id: S-030
title: non-destructive-upgrades
type: story
status: done
priority: critical
tickets: [T-030-01, T-030-02]
---

# S-030: Non-destructive init and upgrade

## Outcome

Running a newer `lisa init` against an existing project is conservative and
auditable. Lisa upgrades content that is still demonstrably lisa-owned,
preserves content that a project has changed, treats ignore rules as
append-only, and tells the operator exactly what happened.

The ownership decision applies to every existing file considered by the init
planner, not only the two paths from the field report. Structured merge targets
such as TOML and JSON may keep their format-aware merge behavior, but must
preserve unrelated user content. Template replacement is allowed only when
lisa can establish that the existing content is an unmodified version it
previously installed. When that cannot be established, the safe behavior is to
leave the file untouched and make the skipped update visible.

## Field regression

The original failure upgraded 0.3.0 to 0.4.0-rc.5 and:

- deleted locally committed Story Layer and read-the-story rules from
  `docs/knowledge/rdspi-workflow.md`;
- replaced `.lisa/.gitignore`, dropping `hooks/ntfy-topic` and exposing a
  notification secret.

Both cases must become permanent regression fixtures.

## Tickets

- **T-030-01 — Ownership-aware init planning:** distinguish safe template
  upgrades from locally modified files and preserve the latter.
- **T-030-02 — Append-only ignores and mutation report:** preserve every ignore
  rule and make the exact init write set visible to the operator.

## Done when

A fixture containing both vend-style customizations can be upgraded without
losing a byte of user-owned content, while an unmodified older lisa scaffold
still receives safe required updates and the command reports its exact writes.
