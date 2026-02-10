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

### Sprint 5: Workflow Separation (in progress)
- Extract RDSPI workflow from CLAUDE.md into standalone docs/rdspi-workflow.md
- CLAUDE.md now project-specific only (description, build, layout)
- Scheduler references both CLAUDE.md and rdspi-workflow.md
- Setup guide template cut in half — no more workflow boilerplate to copy

---

## Next Sprint Candidates

Prioritized based on first-implementer feedback and design maturity.

### Sprint Candidate: `lisa init` CLI
Highest-value next step per feedback. Requires a binary target.
- Project type detection (Cargo.toml, package.json, go.mod, pyproject.toml)
- Directory scaffolding (active + archive)
- CLAUDE.md generation (project-specific only, workflow is separate)
- `.lisa.toml` config file (versionable, replaces zellij plugin config)
- DAG validation (cycles, missing refs, orphan tickets)
- Interactive first story/ticket creation
- `--dry-run` mode
- Enforce filesystem-safe IDs

### Sprint Candidate: Plugin Integration Testing
Test the plugin in a live zellij session.
- Load plugin with example tickets, verify dashboard renders
- DAG computation matches expected dependency graph
- Thread spawning with a simple ticket
- Filesystem watch detects artifact creation
- Phase transition detection and dashboard updates

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
