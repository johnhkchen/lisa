# T-015-02 Design: CONTRIBUTING.md and Docs Cleanup

## Decision 1: CONTRIBUTING.md Content and Tone

### Options Considered

**A. Minimal (build + PR only)**
Just prerequisites, build commands, and "fork, branch, PR" — 30-40 lines. Assumes contributors can figure out the rest.

**B. Comprehensive with ticket system explanation (recommended)**
Covers build, test, project structure, code style, PR workflow, AND explains Lisa's self-managed ticket system. ~80-100 lines. This is what the ticket requirements ask for.

**C. Verbose with architecture deep-dive**
Everything in B plus architecture details, design philosophy, plugin internals. 200+ lines. Overkill for a CONTRIBUTING file — that's what docs/ is for.

### Decision: Option B

The ticket explicitly lists 6 sections to cover. The ticket system explanation is unique to Lisa (it uses its own tool for development) and worth including — it's a conversation starter and helps contributors understand why `docs/active/` and `docs/archive/` exist.

## Decision 2: docs/archive/ Context

### Options Considered

**A. README.md in docs/archive/ (recommended by ticket)**
Short file explaining the archive is Lisa's development history — built by Lisa's own RDSPI workflow. Points visitors to the RDSPI workflow doc for context.

**B. Move archive to a separate branch**
Removes clutter from main, but makes the self-referential story invisible. Also complicates history.

**C. .gitignore and remove**
Destroys interesting history. No benefit.

### Decision: Option A

The ticket recommends this. The archive is a genuine demonstration of the tool — 30 tickets, 9 stories, 140 work artifacts produced by Lisa managing its own development. A short README makes this discoverable rather than confusing.

## Decision 3: specification.md and project-recap.md

### Options Considered

**A. Move both to docs/archive/**
They're historical. The specification uses the old "Ralph" name. The recap is development metrics. Both are interesting history but not useful for a contributor looking to get started.

**B. Keep and rename/edit**
Could rename specification.md to "design-document.md" and s/Ralph/Lisa/, but it's a 210-line historical document that doesn't reflect current state. Editing it creates a maintenance burden.

**C. Remove from tracking**
Loses history unnecessarily.

### Decision: Option A

Move both to `docs/archive/`. They join the other historical artifacts. The archive README (Decision 2) provides context for visitors who find them there.

### T-012-02 Overlap

T-012-02 also lists these files for evaluation. Since T-012-02 hasn't started and lists "move to docs/archive/" as its first option, moving them here is aligned. If T-012-02 runs later, it'll find them already handled.

## Decision 4: ROADMAP.md

Leave ROADMAP.md in place. T-012-02 explicitly owns the "moron" reference cleanup and other ROADMAP edits. This ticket won't touch it to avoid conflicts.

## Decision 5: docs/rdspi-workflow.md symlink

Leave the symlink. It's functional, the canonical copy is in `docs/knowledge/`, and changing it creates unnecessary churn.

## File Changes Summary

| Action | File |
|--------|------|
| Create | `CONTRIBUTING.md` (repo root) |
| Create | `docs/archive/README.md` |
| Move | `docs/specification.md` → `docs/archive/specification.md` |
| Move | `docs/project-recap.md` → `docs/archive/project-recap.md` |
| No change | `ROADMAP.md` (T-012-02 scope) |
| No change | `docs/rdspi-workflow.md` symlink |
