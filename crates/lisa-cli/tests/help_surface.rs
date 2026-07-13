//! Regression lock for the legible `--help` surface (S-036-01, S-044-01).
//!
//! Pins five properties so they cannot silently regress:
//!   (a) all 12 of Lisa's own subcommands still resolve,
//!   (b) top-level help matches the operator-oriented snapshot,
//!   (c) each operator command keeps its purpose and concrete example,
//!   (d) the four machinery-invoked plumbing commands stay outside the
//!       operator listing and the three internal commands stay hidden out of it,
//!   (e) the about-line and operator help carry none of the banned category
//!       jargon.
//!
//! Black-box against the built binary (`CARGO_BIN_EXE_lisa`), matching the
//! convention of the other integration tests in this crate.

use std::process::{Command, Output};

/// The five commands an operator runs, foregrounded in `--help`.
const OPERATOR_COMMANDS: [&str; 5] = ["init", "validate", "status", "doctor", "loop"];

/// The four machinery-invoked commands: omitted from Clap's generated list but
/// still shown in the curated plumbing footer and directly invokable by name.
const PLUMBING_COMMANDS: [&str; 4] = [
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

struct OperatorHelpSnapshot {
    command: &'static str,
    expected: &'static str,
}

const OPERATOR_HELP_SNAPSHOTS: [OperatorHelpSnapshot; 5] = [
    OperatorHelpSnapshot {
        command: "init",
        expected: r#"Set up a project to run with Lisa

Usage: lisa init [OPTIONS]

Options:
      --dry-run      Show what would be done without making changes
      --path <PATH>  Path to the project root (defaults to current directory) [default: .]
  -h, --help         Print help

Example: lisa init --path ./my-project
"#,
    },
    OperatorHelpSnapshot {
        command: "validate",
        expected: r#"Check your tickets and project setup for problems before a run

Usage: lisa validate [OPTIONS]

Options:
      --path <PATH>  Path to the project root (defaults to current directory) [default: .]
      --check-tools  Also check that zellij and claude are on PATH
  -h, --help         Print help

Example: lisa validate --path ./my-project --check-tools
"#,
    },
    OperatorHelpSnapshot {
        command: "status",
        expected: r#"Show which tickets are ready to run and which are waiting, and why

Usage: lisa status [OPTIONS]

Options:
      --path <PATH>      Path to the project root (defaults to current directory) [default: .]
      --ticket <TICKET>  Show retained pre-ownership failures for this ticket
      --ledger <LEDGER>  Provenance ledger to read (defaults to .lisa/provenance.jsonl)
  -h, --help             Print help

Example: lisa status --path ./my-project
"#,
    },
    OperatorHelpSnapshot {
        command: "doctor",
        expected: r#"Check that the tools Lisa needs are installed

Usage: lisa doctor [OPTIONS]

Options:
      --path <PATH>  Path to the project root (defaults to current directory) [default: .]
  -h, --help         Print help

Example: lisa doctor --path ./my-project
"#,
    },
    OperatorHelpSnapshot {
        command: "loop",
        expected: r#"Start a run: work through the ready tickets, in parallel where they don't collide

Usage: lisa loop [OPTIONS]

Options:
      --path <PATH>                Path to the project root (defaults to current directory) [default: .]
      --max-threads <MAX_THREADS>  Maximum concurrent Claude sessions (overrides .lisa.toml)
      --client <CLIENT>            Agent client to drive (claude | codex); overrides .lisa.toml [agent].client
      --dry-run                    Show what would be done without launching zellij
  -h, --help                       Print help

Example: lisa loop --path ./my-project --max-threads 3
"#,
    },
];

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

/// (c) Every operator command's complete help is a deliberate, reviewable
/// surface, including a positive lock on both purpose and concrete example.
#[test]
fn operator_help_matches_snapshots() {
    assert_eq!(
        OPERATOR_HELP_SNAPSHOTS.len(),
        OPERATOR_COMMANDS.len(),
        "every operator command must have a help snapshot"
    );

    for (operator, snapshot) in OPERATOR_COMMANDS.iter().zip(&OPERATOR_HELP_SNAPSHOTS) {
        assert_eq!(
            operator, &snapshot.command,
            "operator help snapshots must stay in canonical command order"
        );
        assert_eq!(
            help_stdout(&[snapshot.command, "--help"]),
            snapshot.expected,
            "operator command `{}` help changed",
            snapshot.command
        );
    }
}

/// (d) Plumbing has its own footer rather than leaking into the generated
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

    for plumbing in PLUMBING_COMMANDS {
        assert!(
            listing_offset(operator_help, plumbing).is_none(),
            "plumbing command `{plumbing}` leaked into the generated operator list",
        );
        assert!(
            listing_offset(plumbing_help, plumbing).is_some(),
            "plumbing command `{plumbing}` is missing from the plumbing section",
        );
    }

    for internal in HIDDEN_COMMANDS {
        assert!(
            listing_offset(&help, internal).is_none(),
            "internal command `{internal}` should be hidden out of the --help listing but appears in it",
        );
    }
}

/// (e) The about-line and each operator command's help contain no banned jargon.
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
