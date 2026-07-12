# Driving with vend (and lisa)

Two engines share one board under `docs/active/`.

## The two engines
- **vend clears intent into work.** A signal — a one-line idea or a pulled demand — becomes a
  typed board: an **epic** (`docs/active/epic/`), its **stories** (`docs/active/stories/`),
  and **tickets** (`docs/active/tickets/`). Work is admitted only if it clears the gates
  (valuable · allocatable · in-bounds · well-formed). Nothing unworthy settles.
- **lisa builds the work into commits.** It picks up `phase: ready` tickets and runs each
  through the RDSPI loop (`rdspi-workflow.md`), committing as it goes.

vend writes the board; lisa consumes it. The board is the contract between them.

## The drive (the loop)
1. **Author once** — edit `SEED.md` (your one-line intent) and tune
   `docs/knowledge/charter.md` (your value function: what "valuable" means here).
2. **Clear → build → sweep, repeated one pull at a time:**
   - `vend steer` — read the project; stage a ranked board + the genuine forks.
   - `vend chain "<signal>"` — pull ONE signal: mint an epic and decompose it (graph-valid by
     construction). Add `--after <ticket>` to queue an epic BEHIND a running loop race-free —
     its entry tickets are born blocked on that ticket, so a greedy loop can't grab them early.
   - **lisa builds** the `phase: ready` tickets into commits (the RDSPI loop). vend clears
     intent; lisa builds it. The board under `docs/active/` is the contract between them.
   - **sweep** — when a loop finishes, verify done⇒committed and mark the cleared epics done,
     then pull the next signal and `vend chain` again. (The deliberate one-signal pull is the
     point — vend recommends, you pull; it never auto-drains the board.)
   - `vend doctor` (green = sound) · `vend svg` (see the board).

## Complementarity — run `lisa init`, then `vend init`
`lisa init` lays the build side (`.lisa.toml`, the hooks, `stories/tickets/work/`, the RDSPI
doc). `vend init` adds the clearing side on top, never clobbering: `docs/active/epic/`, the
`charter`/`vision`, the demand board, the PM desk, and this doc.

## The gates that hold it together
- **Mint-time** — a decomposed board must be graph-valid (every story resolves to its epic) or
  the mint is refused.
- **Budget (P7)** — a metered cast (`vend chain`, `vend steer`) honors its funded time/token
  envelope (a hard wall-clock latch, a token ceiling) and stops clean when spent.
- **Pre-sweep** — before an epic is marked done, every `done` ticket's work must be committed
  (`bun run check:presweep`): done means committed.
