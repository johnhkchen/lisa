# T-015-01 Plan: Rewrite README for External Audience

## Implementation Steps

### Step 1: Write the new README.md

Single step — this is a documentation-only change to one file. The content is fully specified in the structure artifact.

Write `README.md` with all nine sections in order:
1. Header + one-liner
2. What it does (3 paragraphs)
3. Prerequisites (Claude Code, Zellij, lisa doctor)
4. Install (shell installer, cargo install, from source)
5. Quick start (lisa init, example ticket, lisa loop)
6. How it works (workflow, scheduling, concurrency)
7. Project layout (crate tree)
8. Contributing (link)
9. License (MIT)

### Step 2: Verify acceptance criteria

Check against each criterion from the ticket:
- [ ] README follows the specified 9-section structure
- [ ] Install section covers all three paths (shell, cargo, source)
- [ ] No "Ralph", no sprint references, no internal jargon
- [ ] Quick start section is copy-pasteable (includes example ticket)
- [ ] Reads well to someone with zero context

## Testing Strategy

No code changes, so no automated tests. Verification is manual:
1. Read the README top-to-bottom as if discovering the project for the first time
2. Confirm all links are well-formed (relative link to CONTRIBUTING.md, external links to Zellij/Claude Code)
3. Confirm code blocks are syntactically correct (bash, yaml)
4. Confirm no references to Ralph, sprints, or internal development history

## Commit Plan

Single commit: "Rewrite README for external audience"
