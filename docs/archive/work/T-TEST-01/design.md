# T-TEST-01 Design: Top-Level Repository File Listing

## Decision

Create a single markdown file listing every top-level repository entry with a one-line description. The output is a reference document, not code, so the design choices are about format and scope.

## Options Considered

### Option A: Flat list with one-line descriptions
A simple markdown file with each entry on its own line: name, dash, description. Group into "Files" and "Directories" sections for readability.

**Pros:** Simple, scannable, easy to maintain. Matches the ticket's request ("one-line description of each").
**Cons:** No additional structure or metadata.

### Option B: Table format
A markdown table with columns for name, type, and description.

**Pros:** Compact, visually aligned.
**Cons:** Tables are harder to edit and don't render well in all contexts. Overkill for a simple listing.

### Option C: Nested tree with descriptions
A tree-like format showing directory contents one level deep.

**Pros:** Shows more structure.
**Cons:** Goes beyond the ticket scope ("top-level files and directories"). Harder to keep up to date.

## Chosen Approach: Option A

A flat grouped list is the most direct match for the ticket's acceptance criteria. Two sections — Files and Directories — with each entry as a bullet: `**name** — description`. Include dotfiles/directories since they are meaningful project configuration.

## Scope Decisions

- **Include dotfiles**: `.gitignore`, `.lisa.toml`, `.github/`, `.lisa/` are all meaningful configuration. Include them.
- **Exclude `target/`**: Build artifact directory, gitignored, no useful description beyond "build output."
- **Sort order**: Group by type (files first, directories second). Within each group, alphabetical.
- **Output location**: `docs/active/work/T-TEST-01/` alongside other phase artifacts. The file listing itself will be the implement-phase deliverable; `progress.md` will document completion.

## Rejected Alternatives

- Table format — unnecessary structure for a simple list
- Tree format — exceeds ticket scope
- Separate file outside work directory — no reason to place it elsewhere; the ticket's acceptance criteria track work artifacts
