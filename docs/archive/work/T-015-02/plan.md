# T-015-02 Plan: CONTRIBUTING.md and Docs Cleanup

## Step 1: Move historical docs to archive

Move `docs/specification.md` and `docs/project-recap.md` to `docs/archive/` using `git mv`.

**Verification**: `ls docs/archive/specification.md docs/archive/project-recap.md` succeeds; `ls docs/specification.md docs/project-recap.md` fails.

## Step 2: Create docs/archive/README.md

Write a short README explaining the archive directory — what it contains, why it exists, and how it relates to the RDSPI workflow.

**Verification**: File exists, reads clearly to someone unfamiliar with the project.

## Step 3: Create CONTRIBUTING.md

Write the contributor guide at the repo root. Six sections per ticket requirements:
1. Building from source
2. Project structure
3. Running tests
4. Submitting changes
5. Code style
6. Ticket/story system

Source information from: CLAUDE.md (build/layout), justfile (commands), README.md (prerequisites), docs/knowledge/rdspi-workflow.md (workflow).

**Verification**: File exists at repo root, covers all 6 required sections, commands are accurate.

## Step 4: Verify acceptance criteria

Walk through each criterion:
- [ ] `CONTRIBUTING.md` exists at repo root
- [ ] Covers build, test, and contribution workflow
- [ ] `docs/archive/` has context for external visitors
- [ ] No orphaned or confusing docs visible to someone browsing the repo

## Testing Strategy

No code changes, so no unit/integration tests needed. Verification is file existence and content review. Build/test commands mentioned in CONTRIBUTING.md should be verified against justfile and CLAUDE.md for accuracy.
