# Roadmap

## Completed

### Sprint 0: Codebase Scaffolding
- Core types: Phase, Ticket, Thread, PluginConfig, ActivityEvent
- Ticket parsing: YAML frontmatter extraction, directory scanning, phase/status updates
- DAG computation: dependency graph, cycle detection, topological sort, critical path
- Scheduler: thread lifecycle, commit lock (flock/WASM), Claude session spawning
- UI dashboard: DAG visualization, active/parked threads, activity log, quick-jump
- Plugin entry: ZellijPlugin impl with event handling

### Sprint 1: Foundation
- Fixed lib.rs <-> ui.rs signature bridge
- Created CLAUDE.md with RDSPI workflow definition
- Set up example ticket directory structure with sample tickets
- Verified wasm32-wasip1 compilation, 63 tests passing

### Sprint 2: Core Plugin Logic
- Implemented `rebuild_dag`: scans ticket directory, builds DAG, logs errors
- Implemented `schedule_ready_tickets`: queries DAG, spawns Claude sessions via zellij
- Implemented `handle_filesystem_update`: detects artifacts, tracks phase changes

### Sprint 3: Documentation
- Lisa-loop codebase setup guide for other projects
- CLAUDE.md template for project adoption
- Notes for future `lisa init` command

### Sprint 4: First-Implementer Feedback (moron project)
Applied feedback from first manual setup on the moron Rust motion graphics engine:
- Fixed doc inconsistencies: phase example, max_threads default (4 → 2), filesystem-safe ID note
- Made `blocks` optional — DAG fully computed from `depends_on` alone (65 tests)
- Added archiving section to setup guide (docs/archive/ convention)
- Added mid-flight ticket modification guidance (reset to ready if past Design)
- Added optional `tickets` field to story format for human convenience

### Sprint 5: Workflow Separation
- Extract RDSPI workflow from CLAUDE.md into standalone docs/rdspi-workflow.md
- CLAUDE.md now project-specific only (description, build, layout)
- Scheduler references both CLAUDE.md and rdspi-workflow.md
- Setup guide template cut in half — no more workflow boilerplate to copy

### Sprint 6: `lisa init` CLI
Cargo workspace restructure + CLI implementation.
- Restructured into 3-crate workspace: lisa-core, lisa-plugin, lisa-cli
- lisa-core: shared types, ticket parsing, DAG (no zellij deps)
- lisa-plugin: WASM plugin (scheduler, UI, ZellijPlugin impl)
- lisa-cli: binary with clap, two subcommands (init, validate)
- Project type detection: Rust, Node, Go, Python (by marker files)
- `lisa init`: detect project, scaffold directories, generate CLAUDE.md, copy workflow
- `lisa validate`: scan tickets, build DAG, check cycles/missing deps/acceptance criteria
- `--dry-run` mode shows planned actions without executing
- Never overwrites existing files
- 84 tests across workspace (43 core + 22 plugin + 19 CLI)

### Sprint 7: `lisa loop` + End-to-End First Run
Close the loop: `lisa init` → write tickets → `lisa loop` → agents work.
- WASM embedding: build.rs copies plugin to OUT_DIR, include_bytes! in CLI binary
- `lisa loop` command: validates prereqs, writes WASM to temp, generates KDL layout, execs zellij
- Plugin fix: `schedule_ready_tickets()` called on load so pre-existing tickets start immediately
- `just build-cli` builds plugin first, then CLI with embedded WASM
- `just release` recipe for full distribution build

### S-005: Scheduling Observability & Reliability
Fix scheduling bugs found after running S-001–S-004, add logging.
- Scheduling decision logging: Info/PollSummary events for scheduling pipeline visibility
- Fixed phase-change detection: first-seen tickets now detected, slot release unconditional
- Thread lifecycle cleanup: completed threads removed, stale entries audited
- Sweep safety nets: `sweep_stale_slots()` and `audit_threads()` catch orphaned state
- 182 tests (up from 88)

---

## Active

### S-006: Dogfood Integration Testing
Run lisa on itself. Add diagnostic tooling to make the live run observable.
- T-006-01: `lisa status` CLI command — offline DAG/ticket inspection
- T-006-02: Plugin startup diagnostics — log what was loaded on init
- T-006-03: Session launch command audit — log/verify agent spawn commands
- T-006-04: Runtime state snapshot — dump full plugin state on keypress

---

## Next Sprint Candidates

Prioritized based on first-implementer feedback and design maturity.

### Sprint Candidate: Review Gating
Make review behavior configurable.
- Per-phase review gating configuration
- Auto-advance for Structure/Plan when configured
- Notification system for parked threads
- Review approval mechanism

### Sprint Candidate: Robustness
Handle real-world failure modes.
- Session crash recovery (restart from last artifact)
- Graceful handling of malformed tickets
- Commit lock timeout and recovery

### Sprint Candidate: Distribution
- GitHub release workflow for .wasm artifacts
- Versioning scheme
- `lisa update` self-update mechanism

---

## Open Questions

1. **Context limits**: Does a full RDSPI cycle fit in 1M tokens for real tickets?
2. **Parallelism limits**: Is 2 concurrent threads safe? When to go higher?
3. **Agent teams ROI**: Do Research/Design swarms improve quality or just burn tokens?
4. **Worktree integration**: Should lisa manage worktrees for cross-story parallelism?
5. **Ticket ID scheme**: Global sequential IDs (T-001, T-002) vs story-prefixed (T-001-01)? Feedback suggests decoupling from stories.
