---
doc: epic-dir
---

# Epics — the top of the board

Epics live here (`E-001.md`, `E-002.md`, …), one file per epic — the bigger-picture plays
intent clears into.

**Vend owns this layer; you don't hand-author these.** `vend chain "<signal>"` proposes an
epic here, then decomposes it into the stories (`../stories/`) and tickets (`../tickets/`)
lisa builds. The id convention is nested and enforced at mint: epic `E-007` → stories
`S-007-01…` → tickets `T-007-01-01…` (a flat id that doesn't resolve to its epic is refused).

This note carries no `id:`, so the board loader skips it.
