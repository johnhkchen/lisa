# Design: T-007-01 — lisa-setup-guide-command

## Decision Summary

Single `setup_guide.rs` module with one public `run_setup_guide()` function. Output is a
single Markdown document to stdout, structured as numbered steps. Uses existing detect/
templates/config modules. Skips directory/file creation steps when init has already run.

---

## Option A: Monolithic String Builder

Build the entire guide as one `String` using `format!()` and helper functions, print once.

Pros:
- Simple, easy to test (capture output string, assert on contents)
- No intermediate abstractions
- Matches the "output to stdout" requirement directly

Cons:
- Large string literal blocks in code
- Hard to unit-test individual sections in isolation

## Option B: Section-based Builder

Define a `GuideSection` struct with title + body. Build a `Vec<GuideSection>`, then
render all at once with step numbering.

Pros:
- Individual sections testable
- Conditional inclusion (skip/include) is clean
- Auto-numbering means adding/removing sections doesn't break step numbers

Cons:
- More abstraction for what is fundamentally a text-output command
- Section struct adds code that serves only this module

## Option C: Template File

Use a separate template file (like rdspi-workflow.md) with placeholders, embed at compile
time, and do string replacement.

Pros:
- Guide content lives in a readable markdown file
- Easy for non-programmers to edit

Cons:
- Placeholder syntax is ad-hoc
- Conditional sections (skip when init'd) require a mini template language
- Overkill for this use case

---

## Decision: Option B (Section-based Builder)

**Rationale:** The guide has ~9 content sections, some of which are conditionally included
based on whether `lisa init` has already run. A section-based approach makes conditional
inclusion clean and auto-numbering correct. The `GuideSection` struct is minimal (2 fields).

Option A is a close second and would work fine, but conditional step numbering with raw
string concatenation is error-prone (hard-coded "Step 3" becomes wrong when Step 2 is
skipped).

Option C rejected — the guide has too much dynamic content (detected project info,
conditional sections) for a static template.

---

## Output Format Design

The guide is a single Markdown document printed to stdout. Structure:

```
# Lisa Setup Guide for {project_name} ({project_type})

## Step 1: Create directory structure
...

## Step 2: Create CLAUDE.md
...
(skipped if already exists — replaced with "CLAUDE.md already exists, review it...")

## Step N: Validate
When done, run: `lisa validate`
```

Each step uses `## Step N: Title` heading for LLM-friendly parsing.
Code blocks use triple backticks with language hints.
File contents are in fenced code blocks so an LLM can extract and write them.

---

## Conditional Skip Logic

Check existence of:
- `docs/active/tickets/` — proxy for "init already ran for directories"
- `CLAUDE.md` — proxy for "init already ran for CLAUDE.md"
- `.lisa.toml` — proxy for "init already ran for config"
- `docs/rdspi-workflow.md` — proxy for "workflow already created"

When something exists, the step body changes to a shorter "already exists, review and
update if needed" message instead of the full creation instructions. The step is still
included (not omitted) so the guide stays self-contained — the LLM still knows what
that file should contain.

---

## Content Sections (in order)

1. **Create directory structure** — `mkdir -p` commands. Skip body if dirs exist.
2. **Create .lisa.toml** — default config content. Skip body if file exists.
3. **Create CLAUDE.md** — generated template with detected commands. Skip if exists.
4. **RDSPI Workflow** — full embedded content. This is informational context, always
   included. Tells the LLM "this is the workflow your tickets will follow."
5. **Ticket format** — frontmatter fields, body structure, `depends_on` rules.
   Extracted from rdspi-workflow.md but presented as actionable instructions.
6. **Story format** — when to use stories, frontmatter fields, body conventions.
7. **Dependency modeling** — the "same files = missing edge" rule, DAG concurrency.
8. **Archiving** — move done items from active/ to archive/.
9. **Validate** — "Run `lisa validate` to check your setup."

Sections 4-8 are always included (they're knowledge, not file operations).
Sections 1-3 adapt based on existing files.

---

## Module Interface

```rust
// setup_guide.rs

pub fn run_setup_guide(root: &Path) -> Result<(), String>
```

Internally:
- Calls `detect::detect_project(root)`
- Checks file/dir existence for conditional sections
- Builds `Vec<GuideSection>` with content
- Renders to stdout with auto-numbered step headings

```rust
struct GuideSection {
    title: String,
    body: String,
}

fn render_guide(project_name: &str, project_type: &str, sections: Vec<GuideSection>) -> String
```

---

## Testing Strategy

- Test `render_guide()` with mock sections → verify numbering, structure
- Test full `run_setup_guide()` output by capturing the returned string
  (refactor: have internal function return String, public function prints it)
- Test with Rust project (detected commands appear in CLAUDE.md)
- Test with Node project (different commands)
- Test with Unknown project (no build section)
- Test with already-initialized project (sections adapted)
- Test that RDSPI workflow content is included
- Test that ticket format section is included
- No filesystem side effects to verify (stdout only)

---

## Rejected Alternatives

**Writing to a file instead of stdout:** The ticket says stdout. This enables
`lisa setup-guide | pbcopy` or piping directly to an LLM.

**Reusing init's plan_init_actions():** The init module plans filesystem operations.
The setup-guide outputs text instructions. The overlap is just "which dirs to create"
— cheaper to duplicate the dir list than couple the modules.

**Including the guide in `lisa init --verbose`:** Different concerns. `init` does the
work; `setup-guide` tells an LLM how to do the *content* work (writing tickets,
filling in CLAUDE.md). They complement each other.
