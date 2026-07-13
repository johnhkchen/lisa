---
title: arcade-nested-root-review-livelock
status: chained
observed_at: 2026-07-12
evidence_repo: /Users/johnchen/swe/repos/arcade
evidence_project: games/midsummer
evidence_tickets: [T-009-01-01, T-009-02-01, T-009-03-01]
materialized_epic: E-042
materialized_tickets: [T-042-01-05, T-042-01-06, T-042-01-07]
agent: codex
---

# Arcade nested-root Review livelock — field note

Lisa was launched from `arcade/games/midsummer`, two levels below the Git root. The plugin captured
that initial cwd as `project_root` and built completion with `--path games/midsummer` plus
project-relative `docs/active/...` ticket/work arguments. `complete-ticket` discovered the enclosing
`arcade` repository for Git operations but retained those include strings, so Git interpreted them
as root-level `arcade/docs/active/...`; those ticket/work paths do not exist. The isolated
transaction safely restored the ticket from its temporary Done preparation to Review.

T-009-02-01 and T-009-03-01 independently finished private Review artifacts at 17:03 and 17:10,
received false missing-review nudges ten minutes later, and remained Review without completion
provenance. T-009-01-01 had previously recovered only when an agent directly invoked
`complete-ticket` with the actual Arcade root and `games/midsummer/docs/...` paths, producing
`f64df75`.

The fix must retain distinct Lisa-project and Git-root identities, normalize effect paths against
the Git root, suppress misleading Review nudges when the artifact exists, and prove the exact
nested topology through the real command builder and isolated transaction.
