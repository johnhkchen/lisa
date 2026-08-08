//! Frozen copies of the agent-context files `lisa init` used to write.
//!
//! Lisa stopped generating `CLAUDE.md` and `AGENTS.md` in 0.5.0 (T-057-01-03).
//! The generators are gone; their *output* has to stay, because it is the only
//! thing that separates two files that look identical from the outside:
//!
//! - a `CLAUDE.md` Lisa wrote and the operator never touched — Lisa's litter,
//!   and Lisa's to retire;
//! - a `CLAUDE.md` the operator wrote — the project's standing instructions to
//!   every model that will ever read it, which Lisa must not so much as mention.
//!
//! Bytes are the whole warrant, exactly as [`crate::templates::LEGACY_WORKFLOWS`]
//! is the warrant for removing the old workflow document. **Nothing in
//! `data/legacy/` is editable.** It is not copy to keep current; it is evidence
//! of what Lisa once emitted, and an edit here silently reclassifies somebody's
//! file.
//!
//! `AGENTS.md` interpolated nothing, so it is an exact byte comparison.
//! `CLAUDE.md` interpolated the detected project's name, type label, build
//! commands and source layout, so a single frozen string cannot express it.
//! What is frozen instead is the generator's *shape*: the literal text it
//! always emitted, with a hole at each interpolation. A file matches only when
//! every literal is present, in order, anchored at both ends — so an operator
//! who rewrote any of Lisa's own prose falls out of the match and keeps their
//! file. The holes are the machine-derived spans (a build command, a directory
//! listing); an edit confined to one of those still reads as generated, which
//! is the one deliberate imprecision here and the reason the holes are drawn as
//! tightly as the format strings allow.

/// One span of a historical generator's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece {
    /// Text the generator always emitted verbatim.
    Lit(&'static str),
    /// A hole where the generator interpolated detected project data.
    Slot,
}

/// Every `CLAUDE.md` preamble Lisa shipped, from `# CLAUDE.md` through
/// `## Project`. The three generations differ only here: 0.2 opened straight
/// onto the project section, 0.4.0 added the purpose paragraph and a one-line
/// role sentence, and 0.4.4 replaced that sentence with the two-role contract.
const CLAUDE_HEADERS: &[&str] = &[
    include_str!("../data/legacy/claude-md-header-v0.2.md"),
    include_str!("../data/legacy/claude-md-header-v0.4.0.md"),
    include_str!("../data/legacy/claude-md-header-v0.4.4.md"),
];

/// The directory-conventions block and workflow pointer that closed every
/// generation. Anchoring the match to it is what stops a generated file the
/// operator appended their own notes to from being read as pure Lisa output.
const CLAUDE_TAIL: &str = include_str!("../data/legacy/claude-md-tail.md");

/// The project-type labels the generator could print. A closed set, so it is
/// matched literally rather than through a hole.
const CLAUDE_TYPE_LABELS: &[&str] = &["Rust", "Node.js", "Go", "Python", "unknown type"];

/// The text between the interpolated type label and the optional sections.
const CLAUDE_DESCRIPTION: &str = ") — TODO: add a one-line project description here.\n\n";

/// Every `AGENTS.md` Lisa shipped. The file was a pointer at `CLAUDE.md` with
/// no project-specific content, so these are exact bytes.
pub(crate) const LEGACY_AGENTS_CONTEXTS: &[&str] = &[
    include_str!("../data/legacy/agents-md-v0.4.0.md"),
    include_str!("../data/legacy/agents-md-v0.4.4.md"),
];

/// Match `content` against one frozen generation shape.
///
/// Literals that follow another literal must sit exactly at the cursor;
/// literals that follow a hole are searched for from the cursor, and a literal
/// that closes the pattern must land at the end of the file. The pattern has to
/// consume the whole file: leftover text on either side means the operator
/// added something, and an operator's addition is not Lisa's to reclaim.
fn matches_generation(content: &str, pattern: &[Piece]) -> bool {
    let mut cursor = 0;
    let mut after_slot = false;

    for (index, piece) in pattern.iter().enumerate() {
        let Piece::Lit(literal) = piece else {
            after_slot = true;
            continue;
        };
        let literal: &str = literal;

        let rest = &content[cursor..];
        let found = if !after_slot {
            rest.starts_with(literal).then_some(0)
        } else if index + 1 == pattern.len() {
            // The closing literal is anchored to the end of the file so the
            // preceding hole cannot swallow an appendix somebody wrote.
            rest.ends_with(literal).then(|| rest.len() - literal.len())
        } else {
            rest.find(literal)
        };

        match found {
            Some(at) => {
                cursor += at + literal.len();
                after_slot = false;
            }
            None => return false,
        }
    }

    !after_slot && cursor == content.len()
}

/// The optional middle of a generated `CLAUDE.md`: a build section, a source
/// layout section, both, or neither, depending on what project detection found.
///
/// Each variant carries the fence scaffolding literally and leaves a hole only
/// where a detected command or directory listing went.
fn claude_middles() -> Vec<Vec<Piece>> {
    const BUILD_OPEN: &str = "### Build and Test\n\n```bash\n# Build\n";
    const BUILD_TESTS: &str = "\n\n# Run tests\n";
    const BUILD_LINT: &str = "\n\n# Lint\n";
    const LAYOUT_OPEN: &str = "### Source Layout\n\n```\n";

    let build = |close: &'static str| {
        vec![
            Piece::Lit(BUILD_OPEN),
            Piece::Slot,
            Piece::Lit(BUILD_TESTS),
            Piece::Slot,
            Piece::Lit(BUILD_LINT),
            Piece::Slot,
            Piece::Lit(close),
        ]
    };

    vec![
        // Both sections present.
        {
            let mut pieces = build("\n```\n\n### Source Layout\n\n```\n");
            pieces.push(Piece::Slot);
            pieces.push(Piece::Lit("\n```\n\n"));
            pieces
        },
        // Build commands only.
        build("\n```\n\n\n"),
        // Source layout only.
        vec![
            Piece::Lit("\n"),
            Piece::Lit(LAYOUT_OPEN),
            Piece::Slot,
            Piece::Lit("\n```\n\n"),
        ],
        // Neither: the two empty sections collapse to their separators.
        vec![Piece::Lit("\n\n")],
    ]
}

/// Is `content` byte-for-byte something a Lisa `generate_claude_md` once wrote?
///
/// False for anything else, including a generated file with one line edited —
/// unless the edit lands inside an interpolated span. Erring toward false is
/// deliberate: a miss leaves the operator's file where they put it, and a
/// false positive marks their writing as Lisa's litter.
pub(crate) fn is_generated_claude_md(content: &str) -> bool {
    let middles = claude_middles();
    for &header in CLAUDE_HEADERS {
        for &label in CLAUDE_TYPE_LABELS {
            for middle in &middles {
                let mut pattern = vec![
                    Piece::Lit(header),
                    Piece::Slot, // the detected project name
                    Piece::Lit(" ("),
                    Piece::Lit(label),
                    Piece::Lit(CLAUDE_DESCRIPTION),
                ];
                pattern.extend_from_slice(middle);
                pattern.push(Piece::Lit(CLAUDE_TAIL));
                if matches_generation(content, &pattern) {
                    return true;
                }
            }
        }
    }
    false
}

/// Is `content` byte-for-byte an `AGENTS.md` Lisa once wrote?
pub(crate) fn is_generated_agents_md(content: &str) -> bool {
    LEGACY_AGENTS_CONTEXTS.contains(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rebuild a 0.4.4 `CLAUDE.md` the way `generate_claude_md` did, so the
    /// tests below assert against generator output rather than against the
    /// matcher's own idea of it.
    fn generated_claude_md(
        header: &str,
        name: &str,
        label: &str,
        build: Option<(&str, &str, &str)>,
        layout: Option<&str>,
    ) -> String {
        let build_section = build
            .map(|(b, t, l)| {
                format!("### Build and Test\n\n```bash\n# Build\n{b}\n\n# Run tests\n{t}\n\n# Lint\n{l}\n```\n")
            })
            .unwrap_or_default();
        let layout_section = layout
            .map(|l| format!("### Source Layout\n\n```\n{l}\n```\n"))
            .unwrap_or_default();
        format!(
            "{header}{name} ({label}{CLAUDE_DESCRIPTION}{build_section}\n{layout_section}\n{CLAUDE_TAIL}"
        )
    }

    #[test]
    fn frozen_headers_close_on_the_project_heading() {
        // A stray editor stripping the blank line after `## Project` would
        // silently stop every generated file from matching. Fail loudly here
        // instead of quietly preserving Lisa's own litter forever.
        for &header in CLAUDE_HEADERS {
            assert!(
                header.ends_with("## Project\n\n"),
                "frozen header must end at the project heading: {header:?}"
            );
            assert!(header.starts_with("# CLAUDE.md\n\n"));
        }
    }

    #[test]
    fn every_generation_shape_is_recognized() {
        for &header in CLAUDE_HEADERS {
            for &label in CLAUDE_TYPE_LABELS {
                let cases = [
                    generated_claude_md(
                        header,
                        "my-app",
                        label,
                        Some(("cargo build", "cargo test", "cargo clippy")),
                        Some("src:\n  lib.rs\n  main.rs"),
                    ),
                    generated_claude_md(
                        header,
                        "my-app",
                        label,
                        Some(("npm run build", "npm test", "npm run lint")),
                        None,
                    ),
                    generated_claude_md(header, "my-app", label, None, Some("src:\n  main.go")),
                    generated_claude_md(header, "mystery", label, None, None),
                ];
                for generated in cases {
                    assert!(
                        is_generated_claude_md(&generated),
                        "generator output must be recognized: {generated:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_edited_generation_is_not_recognized() {
        let generated = generated_claude_md(
            CLAUDE_HEADERS[2],
            "my-app",
            "Rust",
            Some(("cargo build", "cargo test", "cargo clippy")),
            Some("src:\n  main.rs"),
        );
        assert!(is_generated_claude_md(&generated));

        // One line of Lisa's own prose replaced by the operator's.
        let edited = generated.replace(
            "TODO: add a one-line project description here.",
            "A scheduler for climate model runs.",
        );
        assert_ne!(edited, generated);
        assert!(!is_generated_claude_md(&edited));

        // An appendix after the closing block is the operator's writing too.
        let appended = format!("{generated}\n## House rules\n\nAlways run the linter.\n");
        assert!(!is_generated_claude_md(&appended));

        // And a preamble before it.
        let prepended = format!("<!-- reviewed 2026-01-01 -->\n{generated}");
        assert!(!is_generated_claude_md(&prepended));
    }

    #[test]
    fn hand_written_context_is_not_recognized() {
        assert!(!is_generated_claude_md(
            "# CLAUDE.md\n\n## Project\n\nA garden planner.\n"
        ));
        assert!(!is_generated_claude_md(""));
        assert!(!is_generated_agents_md(
            "# AGENTS.md\n\nRead the README first.\n"
        ));
    }

    #[test]
    fn frozen_agents_generations_are_recognized_exactly() {
        for &generation in LEGACY_AGENTS_CONTEXTS {
            assert!(is_generated_agents_md(generation));
            assert!(generation.starts_with("# AGENTS.md\n\n"));
            assert!(generation.contains("docs/knowledge/rdspi-workflow.md"));
            // The pointer at CLAUDE.md is what makes retiring one without the
            // other a dangling reference — T-057-02-02 owns that rule, and it
            // needs this text to still be here.
            assert!(generation.contains("[CLAUDE.md](CLAUDE.md)"));
        }
        // A single added newline is a different file.
        assert!(!is_generated_agents_md(&format!(
            "{}\n",
            LEGACY_AGENTS_CONTEXTS[1]
        )));
    }

    #[test]
    fn matcher_requires_the_whole_file() {
        let pattern = [Piece::Lit("head:"), Piece::Slot, Piece::Lit(":tail")];
        assert!(matches_generation("head:anything:tail", &pattern));
        assert!(matches_generation("head::tail", &pattern));
        assert!(!matches_generation("head:anything:tail extra", &pattern));
        assert!(!matches_generation("prefix head:anything:tail", &pattern));
        assert!(!matches_generation("head:anything", &pattern));
    }
}
