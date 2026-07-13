//! Regression lock for the legible `--help` surface (S-036-01, S-044-01).
//!
//! Pins four properties so they cannot silently regress:
//!   (a) all 12 of Lisa's own subcommands still resolve,
//!   (b) top-level help matches the operator-oriented snapshot,
//!   (c) the four machinery-invoked plumbing commands stay outside the
//!       operator listing and the three internal commands stay hidden out of it,
//!   (d) the about-line and operator help carry none of the banned category
//!       jargon.
//!
//! Black-box against the built binary (`CARGO_BIN_EXE_lisa`), matching the
//! convention of the other integration tests in this crate.

use std::process::{Command, Output};

/// The five commands an operator runs, foregrounded in `--help`.
const OPERATOR_COMMANDS: [&str; 5] = ["init", "validate", "status", "doctor", "loop"];

/// The four machinery-invoked hook/contract commands: visible but set apart
/// (banded below the operator block), never hidden — the loop launcher and
/// Claude hooks call these by name.
const HOOK_COMMANDS: [&str; 4] = [
    "agent-exec",
    "capture-usage",
    "commit-ticket",
    "complete-ticket",
];

/// Hidden out of the primary listing (`hide = true`) but still resolvable.
const HIDDEN_COMMANDS: [&str; 3] = ["setup-guide", "hooks-guide", "version"];

const PLUMBING_HEADING: &str = "Plumbing commands (called by Lisa and agent hooks):";

const TOP_LEVEL_HELP_SNAPSHOT: &str = r#"Everyday path: init → validate → status → loop

Runs your coding agents through a project's tickets.

Usage: lisa <COMMAND>

Commands:
  init      Set up a project to run with Lisa
  validate  Check your tickets and project setup for problems before a run
  status    Show which tickets are ready to run and which are waiting, and why
  doctor    Check that the tools Lisa needs are installed
  loop      Start a run: work through the ready tickets, in parallel where they don't collide
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Plumbing commands (called by Lisa and agent hooks):
  agent-exec       Run Codex and turn its output into Lisa's pane signals
  capture-usage    Record a native session's token usage from its Stop-hook payload on stdin
  commit-ticket    Commit this ticket's own files without touching the repo's ordinary git index
  complete-ticket  Mark a ticket done and commit its files in one step
"#;

/// Every own subcommand. Removing or renaming any one must fail this test.
const OWN_COMMANDS: [&str; 12] = [
    "init",
    "validate",
    "status",
    "doctor",
    "loop",
    "agent-exec",
    "capture-usage",
    "commit-ticket",
    "complete-ticket",
    "setup-guide",
    "hooks-guide",
    "version",
];

/// Category jargon banned from the about-line and operator-facing help (union of
/// the user-global brand voice and the E-036 epic). Matched case-insensitively
/// at word/phrase boundaries so `dag` catches `DAG-driven` without catching a
/// larger word.
const BANNED_JARGON: [&str; 9] = [
    "dag",
    "orchestrat",
    "scheduling",
    "leverage",
    "solutions",
    "deployment",
    "case study",
    "build log",
    "research release",
];

/// Invoke the real `lisa` binary with `args` and capture its output.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn lisa {:?}: {e}", args))
}

/// Run `lisa <args>`, require success, return stdout.
fn help_stdout(args: &[&str]) -> String {
    let out = run(args);
    assert!(
        out.status.success(),
        "`lisa {}` exited {:?}\nstderr:\n{}",
        args.join(" "),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// First banned term that occurs in `text` at a word/phrase boundary, if any.
fn find_jargon(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();
    for term in BANNED_JARGON {
        for (idx, _) in lower.match_indices(term) {
            let before_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
            let after = idx + term.len();
            let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return Some(term);
            }
        }
    }
    None
}

/// The anchor a command renders under in the `--help` listing: a newline, the
/// two-space command column, the name, and a trailing space before its padding.
/// Anchoring this way avoids matching a name inside a description or as the
/// prefix of a longer command name.
fn listing_offset(help: &str, command: &str) -> Option<usize> {
    help.find(&format!("\n  {command} "))
}

/// (a) Every one of the 12 own subcommands resolves — including the hidden
/// three, which `--help` reaches even though they are absent from the listing.
#[test]
fn all_twelve_subcommands_resolve() {
    assert_eq!(
        OWN_COMMANDS.len(),
        12,
        "the pinned command set must be exactly 12"
    );
    for cmd in OWN_COMMANDS {
        let out = run(&[cmd, "--help"]);
        assert!(
            out.status.success(),
            "subcommand `{cmd}` did not resolve (`lisa {cmd} --help` exited {:?}) — was it removed or renamed?\nstderr:\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

/// (b) The complete top-level help screen is a deliberate, reviewable surface.
#[test]
fn top_level_help_matches_snapshot() {
    assert_eq!(help_stdout(&["--help"]), TOP_LEVEL_HELP_SNAPSHOT);
}

/// (c) Plumbing has its own footer rather than leaking into the generated
/// operator list, and the three internal commands remain hidden entirely.
#[test]
fn plumbing_commands_are_separate_and_internal_hidden() {
    let help = help_stdout(&["--help"]);
    let (operator_help, plumbing_help) = help
        .split_once(PLUMBING_HEADING)
        .expect("top-level help is missing the plumbing section heading");

    for operator in OPERATOR_COMMANDS {
        assert!(
            listing_offset(operator_help, operator).is_some(),
            "operator command `{operator}` is missing from the generated command list",
        );
    }

    for hook in HOOK_COMMANDS {
        assert!(
            listing_offset(operator_help, hook).is_none(),
            "plumbing command `{hook}` leaked into the generated operator list",
        );
        assert!(
            listing_offset(plumbing_help, hook).is_some(),
            "plumbing command `{hook}` is missing from the plumbing section",
        );
    }

    for internal in HIDDEN_COMMANDS {
        assert!(
            listing_offset(&help, internal).is_none(),
            "internal command `{internal}` should be hidden out of the --help listing but appears in it",
        );
    }
}

/// (d) The about-line and each operator command's help contain no banned jargon.
/// Hook-command help is intentionally NOT gated — it carries domain vocabulary
/// (codex exec, provenance ledger) the epic deliberately left alone.
#[test]
fn about_line_and_operator_help_are_jargon_free() {
    let help = help_stdout(&["--help"]);

    let about = help
        .lines()
        .find(|line| line.to_lowercase().contains("coding agents"))
        .expect("`lisa --help` produced no about-line");
    // Positive anchor: the plain masthead must actually be present, so the test
    // can't pass by reading empty/rerouted output.
    assert!(
        about.to_lowercase().contains("coding agents"),
        "about-line is not the expected plain masthead: {about:?}",
    );
    assert!(
        find_jargon(about).is_none(),
        "about-line contains banned jargon {:?}: {about:?}",
        find_jargon(about).unwrap(),
    );

    for op in OPERATOR_COMMANDS {
        let text = help_stdout(&[op, "--help"]);
        assert!(
            find_jargon(&text).is_none(),
            "operator `{op} --help` contains banned jargon {:?}",
            find_jargon(&text).unwrap(),
        );
    }
}
