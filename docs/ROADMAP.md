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

---

## Next: Awaiting Feedback

The following sprint candidates are ready to plan once initial feedback comes in from using the setup guide and running the plugin against a real project.

### Sprint Candidate: Plugin Testing (Integration)
Test the plugin in a live zellij session with real tickets.
- Load plugin with example tickets, verify dashboard renders
- Verify DAG computation matches expected dependency graph
- Test thread spawning with a simple ticket
- Verify filesystem watch detects artifact creation
- Test phase transition detection and dashboard updates

### Sprint Candidate: `lisa init` Command
Build the initialization tool based on setup guide pain points.
- Scaffold directory structure
- Generate CLAUDE.md from project detection (Cargo.toml, package.json, go.mod, etc.)
- Interactive ticket creation from user description
- DAG validation (cycle detection, missing refs, orphan tickets)
- Setup validation (CLAUDE.md exists, claude/zellij on PATH)

### Sprint Candidate: Review Gating
Make review behavior configurable and robust.
- Per-phase review gating configuration
- Auto-advance for Structure/Plan when configured
- Notification system for parked threads (which artifact to review, how long waiting)
- Review approval mechanism (update frontmatter vs plugin command)

### Sprint Candidate: Robustness
Handle real-world failure modes.
- Session crash recovery (detect incomplete phase, restart from last artifact)
- Graceful handling of malformed tickets (don't crash the plugin)
- Commit lock timeout and recovery
- Agent teams support for Research/Design phases (experimental)

### Sprint Candidate: Distribution
Make the plugin easy to install and update.
- GitHub release workflow for .wasm artifacts
- Versioning scheme
- Zellij plugin registry (when available)
- `lisa update` self-update mechanism

---

## Open Questions (Carry Forward)

From the design document, still to be validated empirically:

1. **Context limits**: Does a full RDSPI cycle fit in 1M tokens for real tickets?
2. **Parallelism limits**: Is 4 concurrent threads on one branch safe in practice?
3. **Agent teams ROI**: Do Research/Design swarms improve quality or just burn tokens?
4. **Worktree integration**: Should lisa manage worktrees for cross-story parallelism?
