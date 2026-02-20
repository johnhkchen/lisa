# T-012-02 Design: Clean up tracked local files and internal docs

## Decision Summary

This is a straightforward chore ticket. Most work is already done (archive moves staged). The remaining decisions are small.

## Decisions

### 1. `.gitignore` entry for settings.local.json

**Decision**: Add `.claude/settings.local.json` to project `.gitignore`.

Rationale: The user's global gitignore already covers this, but other contributors won't have that rule. Adding it to the project gitignore is defensive and costs nothing.

### 2. ROADMAP.md "moron" reference

**Decision**: Replace "moron project" with "an external Rust project" and "the moron Rust motion graphics engine" with "an external Rust project".

- Line 30: `### Sprint 4: First-Implementer Feedback (moron project)` → `### Sprint 4: First-Implementer Feedback (external project)`
- Line 31: `Applied feedback from first manual setup on the moron Rust motion graphics engine:` → `Applied feedback from first manual setup on an external Rust project:`

Rationale: The ticket says "generic description" or "remove". Generic description preserves context (it was real feedback from a real project) without the specific name.

### 3. specification.md and project-recap.md

**Decision**: No action needed — already moved to `docs/archive/` with a README explaining their context. The renames are staged and will be committed as part of this ticket's work.

### 4. Commit strategy

**Decision**: Single commit covering all changes:
- `.gitignore` update
- `docs/ROADMAP.md` edits
- Already-staged archive renames

This is a small, cohesive cleanup — one commit is appropriate.

## Rejected Alternatives

- **Delete specification.md/project-recap.md entirely**: These have historical value and someone already archived them properly.
- **Rewrite ROADMAP.md extensively**: Only the "moron" reference is externally awkward. The rest reads fine as development history.
