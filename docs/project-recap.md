# Lisa: Project Recap

## What It Is

Lisa is a Zellij WASM plugin and CLI tool that implements DAG-driven concurrent task scheduling for the RDSPI workflow (Research, Design, Structure, Plan, Implement). It orchestrates multiple Claude Code sessions in parallel, one per ticket, managing the full lifecycle: spawning sessions, detecting phase completion, parking threads at review points, and scheduling dependent work as tickets complete.

It carries between projects as a single binary with zero project-specific dependencies. A project's only responsibility is having tickets in the right markdown frontmatter format.

## What Was Built

### By the numbers

| Metric | Value |
|---|---|
| Calendar time | 2 days (Feb 9-10, 2026) |
| Commits | 40 |
| Rust source files | 16 across 3 crates |
| Lines of Rust | ~12,200 (3,100 core + 5,500 plugin + 3,600 CLI) |
| Tests | 267 passing |
| WASM plugin binary | 1.1 MB |
| Stories completed | 9 |
| Tickets completed | 30 |
| RDSPI work artifacts | 140 files (5 per ticket: research, design, structure, plan, progress) |
| Total files in repo | 215, ~32,000 lines |

### The three crates

- **lisa-core** — Types, ticket parsing from YAML frontmatter, DAG computation (cycle detection, topological sort, critical path). No Zellij dependencies; fully unit-testable on native target.
- **lisa-plugin** — Zellij WASM plugin. ZellijPlugin trait implementation, scheduling logic, phase detection (artifact-based + idle signal hooks), TUI dashboard with DAG visualization, attention banners, activity log.
- **lisa-cli** — `lisa init` (scaffold a project), `lisa validate` (compiler-style readiness checks), `lisa loop` (embed WASM, generate KDL layout, exec Zellij). Project type detection for Rust, Node, Go, Python.

### Sprint history

**Sprints 0-4** laid the foundation: core types, DAG computation, scheduler, plugin entry point, dashboard UI, setup documentation, and first-implementer feedback from a real external project.

**Sprint 5 (S-005)** fixed scheduling bugs found after running sprints 1-4: scheduling decision logging, fixed phase-change detection for first-seen tickets, thread lifecycle cleanup, safety sweep functions. Test count jumped from 88 to 182.

**Sprint 6 (S-006)** was the first dogfood run — lisa building itself. Added `lisa status` command, startup diagnostics, session launch auditing, and the runtime state snapshot dump (Shift+D).

**Sprint 7 (S-007)** was distribution: `lisa setup-guide` for LLM-friendly onboarding, enhanced `lisa validate`, crates.io publishing prep, GitHub CI/release workflows.

**Sprint 8 (S-008)** solved the biggest pain point: unreliable phase transitions. Integrated Claude Code's `idle_prompt` notification hook so the plugin detects when an agent finishes work. Implement-to-Review now advances automatically instead of requiring manual frontmatter editing.

**Sprint 9 (S-009)** was first-user readiness: fixed the `--print` flag that was silently breaking idle hooks, removed 800 lines of dead code, hardened `lisa init` for external projects, and made `lisa validate` output machine-parseable so an LLM can iteratively fix issues (like `cargo check` for lisa readiness).

### Bugs found and fixed through dogfooding

These are the issues that only surfaced by running the tool on real work:

- **`--print` mode breaking idle hooks** — Sessions exited after one response instead of staying interactive. The idle signal infrastructure (S-008) was dead code in practice. Fixed by dropping the flag.
- **Review tickets re-scheduled on restart** — Plugin restart created Running threads for Review-phase tickets, which then couldn't be marked done (the modal filtered out Running threads). Fixed by excluding Review from schedulable phases.
- **Silent no-op on mark-done** — Pressing 'd' with no eligible tickets did nothing and gave no feedback. Added activity log message.
- **Phase sync duplication** — Investigated but found to be a false alarm; the existing guard condition already prevented it. Documented as correct defensive behavior.
- **13 dead code warnings** — Entire scheduler.rs module was superseded; 6 unused UI items. All removed, WASM build now warning-free.

### How the work was done

The project used its own RDSPI workflow throughout. Every ticket went through Research (map the codebase), Design (evaluate options), Structure (file-level plan), Plan (sequenced steps), and Implement. Each phase produced a ~200-line artifact stored in `docs/active/work/{ticket-id}/`.

From S-005 onward, tickets were executed by lisa itself — the tool scheduling its own development tickets across concurrent Claude Code sessions. The DAG determined execution order; independent tickets ran in parallel (up to 2 concurrent threads), dependent tickets waited.

The later sprints (S-008, S-009) ran entirely through `lisa loop`: write story and tickets, run `lisa loop`, review results, archive, commit, push. Hotfixes between loop runs addressed issues discovered in real time.

### What's next

The immediate next step is running `lisa init` on an external project and going through the full pipeline: init, validate (iteratively via LLM), write tickets, loop. The open questions are empirical: how well does it handle large tickets (context limits), how far can parallelism scale (2 threads is safe, 4 is untested), and whether the RDSPI phases need tuning for different project types.
