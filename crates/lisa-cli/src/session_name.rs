//! Names a run's Zellij session after the project it runs.
//!
//! Started without `--session`, Zellij invents an animal — `auspicious-panda`,
//! `polished-rustacean` — and that invention is what `zellij list-sessions`
//! prints, what the status bar shows, and what the terminal emulator puts in
//! its tab. Three loops on one desk are then three animals, none of which says
//! which repository it is. A session is a run of one project, and the project
//! is the stable fact about it.
//!
//! Zellij itself only refuses an empty name and a name containing `/`, but the
//! name also becomes a directory in its socket and cache directories and is
//! read back off a command line, so this module is narrower than Zellij is: an
//! ASCII base built from the project directory, never empty, never leading or
//! trailing `-`.
//!
//! Zellij refuses a duplicate name outright, and being dead is no exception —
//! `Session with name "steer" already exists, but is dead.` is exit 1, not a
//! start. A crashed run leaves exactly that behind, so a *dead* taken name
//! never stops the loop here: the next free numbered name is used instead and
//! the old session is left where the operator can still find it.
//!
//! A *running* session of this project's own name is the other case entirely,
//! and this module is where the loop learns about it. The session is where the
//! scheduler lives — the plugin runs inside the Zellij server, and the server
//! outlives every client — so a session still up under this board's name is a
//! scheduler still on this board, whether or not it has any work left to do.
//! [`BoardSessions::running_here`] is that fact, and `loop_cmd` refuses on it.
//! The numbering below therefore only ever numbers around the dead.

use std::path::Path;
use std::process::Command;

/// Base name for a project directory that yields no usable characters.
const FALLBACK_BASE: &str = "lisa";

/// Longest base name built from a directory name, in characters.
///
/// Long enough for any real repository directory, short enough to survive the
/// terminal tab that truncated `auspicious-p` out of `auspicious-panda`.
const MAX_BASE_CHARS: usize = 32;

/// Highest run number tried before the naming is left to Zellij.
const MAX_RUN_NUMBER: u32 = 99;

/// Characters kept verbatim in a session name.
fn is_kept(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

/// How this run's session got its name, and what to tell the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionNaming {
    /// The project's own name was free.
    Project(String),
    /// Another session — running, or `EXITED` after a crash — holds the
    /// project's name, so this run is the next numbered one.
    NextRun { name: String, base: String },
    /// Every numbered name is held. Zellij names this session itself.
    Exhausted { base: String },
}

impl SessionNaming {
    /// The name to pass to Zellij, or `None` to let Zellij invent one.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::Project(name) | Self::NextRun { name, .. } => Some(name),
            Self::Exhausted { .. } => None,
        }
    }

    /// One line for the loop's startup report, in the operator's terms.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Project(name) => name.clone(),
            Self::NextRun { name, base } => format!(
                "{name} (another session already holds \"{base}\" — `zellij list-sessions` shows it)"
            ),
            Self::Exhausted { base } => format!(
                "named by Zellij — \"{base}\" through \"{base}-{MAX_RUN_NUMBER}\" are all taken. \
                 Retire the finished ones with `zellij delete-session <name>` to get the project's \
                 name back."
            ),
        }
    }
}

/// One session Zellij is holding, and whether it is still running.
///
/// Two different facts live on one listing line. A name is *taken* whether the
/// session runs or exited, because Zellij will not reuse it either way; a
/// session is *running* when its server is still up, which is when a scheduler
/// can still be inside it. The loop needs both: a taken name costs a number, a
/// running one on this board stops the start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Session {
    name: String,
    running: bool,
}

/// What Zellij says about this board: the name this run would take, and every
/// session already running under one of this board's names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoardSessions {
    naming: SessionNaming,
    running_here: Vec<String>,
}

impl BoardSessions {
    /// How this run's session would be named.
    pub(crate) fn naming(&self) -> &SessionNaming {
        &self.naming
    }

    /// Sessions running right now under this board's own names — the project's
    /// name, or one of its numbered runs. Each one is a Zellij server that may
    /// still hold a scheduler, so each one is grounds to refuse a second loop.
    pub(crate) fn running_here(&self) -> &[String] {
        &self.running_here
    }
}

/// Decide this run's session name, and find any session already running on this
/// board, by asking Zellij what it is holding.
pub(crate) fn resolve(root: &Path, zellij_path: &Path) -> BoardSessions {
    survey(&project_base(root), &existing_sessions(zellij_path))
}

/// Read one listing into the two facts the loop acts on.
fn survey(base: &str, sessions: &[Session]) -> BoardSessions {
    let taken: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();
    let running_here: Vec<String> = sessions
        .iter()
        .filter(|session| session.running && names_this_board(base, &session.name))
        .map(|session| session.name.clone())
        .collect();

    BoardSessions {
        naming: choose(base, &taken),
        running_here,
    }
}

/// Whether a session name is one this project's own naming could have produced:
/// its name, or one of its numbered runs.
///
/// Deliberately exact. `lisa-2` is this board's second run; `lisa-live-codex`
/// is somebody's hand-named session that merely starts the same way, and
/// refusing a start over it would be a lockout with no way out.
fn names_this_board(base: &str, name: &str) -> bool {
    if name == base {
        return true;
    }
    name.strip_prefix(base)
        .and_then(|rest| rest.strip_prefix('-'))
        .and_then(|run| run.parse::<u32>().ok())
        .is_some_and(|run| (2..=MAX_RUN_NUMBER).contains(&run))
}

/// The project's own session name: its directory, made safe to use as one.
fn project_base(root: &Path) -> String {
    let resolved = root.canonicalize();
    let path = resolved.as_deref().unwrap_or(root);
    let raw = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    sanitize(&raw)
}

/// Reduce a directory name to an unambiguously usable session name.
fn sanitize(raw: &str) -> String {
    let mut name = String::new();
    let mut pending_separator = false;

    for ch in raw.chars() {
        if is_kept(ch) {
            if pending_separator && !name.is_empty() {
                name.push('-');
            }
            pending_separator = false;
            name.push(ch);
        } else {
            // A space, a dot, a slash, an emoji: one separator, however many
            // characters produced it.
            pending_separator = true;
        }
    }

    let name: String = name.chars().take(MAX_BASE_CHARS).collect();
    // A leading `-` reads as a flag on Zellij's command line, and a trailing
    // one is only ever noise — including the one truncation can expose.
    let name = name.trim_matches('-');

    if name.is_empty() {
        FALLBACK_BASE.to_string()
    } else {
        name.to_string()
    }
}

/// Take the project's name if it is free, else the next free run number.
fn choose(base: &str, taken: &[String]) -> SessionNaming {
    if !taken.iter().any(|name| name == base) {
        return SessionNaming::Project(base.to_string());
    }

    for run in 2..=MAX_RUN_NUMBER {
        let candidate = format!("{base}-{run}");
        if !taken.contains(&candidate) {
            return SessionNaming::NextRun {
                name: candidate,
                base: base.to_string(),
            };
        }
    }

    SessionNaming::Exhausted {
        base: base.to_string(),
    }
}

/// Every session Zellij currently holds, running or `EXITED`.
///
/// The long listing rather than `--short`, because `--short` prints names only
/// and the marker that separates a running session from a dead one is the rest
/// of the line.
///
/// `list-sessions` exits non-zero when there are no sessions at all, so the
/// exit status carries no information this needs: the lines it printed do. A
/// listing that cannot be read is an empty one — at worst that costs the run
/// the project's name, and a Zellij that cannot be run is a Zellij holding
/// nothing.
fn existing_sessions(zellij_path: &Path) -> Vec<Session> {
    let output = Command::new(zellij_path)
        .arg("list-sessions")
        .arg("--no-formatting")
        .output();

    match output {
        Ok(output) => parse_sessions(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Read `zellij list-sessions --no-formatting`, one session per line:
///
/// ```text
/// lisa [Created 19h 22m ago] (EXITED - attach to resurrect)
/// lisa-2 [Created 6m 18s ago] (current)
/// ```
///
/// A session counts as running unless the line says it exited. That direction
/// is the safe one: a marker a future Zellij renames reads as a running
/// session, which costs a refusal an operator can clear, rather than as a dead
/// one, which costs a second scheduler nobody notices.
/// The names of the sessions in a listing whose servers are still up.
///
/// One grammar, one reader: [`crate::busy`] asks this machine whether any run
/// is live before an upgrade swaps the binary under it, and it has to read the
/// `EXITED` marker exactly the way the loop does.
pub(crate) fn running_session_names(listing: &str) -> Vec<String> {
    parse_sessions(listing)
        .into_iter()
        .filter(|session| session.running)
        .map(|session| session.name)
        .collect()
}

fn parse_sessions(listing: &str) -> Vec<Session> {
    listing
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            Some(Session {
                name: name.to_string(),
                running: !line.contains("EXITED"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_session_is_named_after_the_project_directory() {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().join("steer");
        std::fs::create_dir(&root).expect("project dir");

        assert_eq!(project_base(&root), "steer");
    }

    #[test]
    fn a_dot_relative_path_still_names_the_project_it_points_at() {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().join("steer");
        std::fs::create_dir(&root).expect("project dir");

        // `lisa loop --path .` reaches run_loop as `<cwd>/.`, whose last
        // component is `.` and whose project is not.
        assert_eq!(project_base(&root.join(".")), "steer");
    }

    #[test]
    fn hostile_directory_names_stay_usable_session_names() {
        for raw in [
            "my project",
            "b28.dev",
            "-dash-led",
            "..",
            ".",
            ".hidden",
            "",
            "   ",
            "///",
            "a/b",
            "üñïçø∂é",
            "🐼",
            "with\ttab",
            "trailing-",
            "Lisa",
            "under_score",
            "a-very-long-repository-directory-name-that-nobody-would-truncate-well",
        ] {
            let name = sanitize(raw);
            assert!(!name.is_empty(), "{raw:?} produced an empty name");
            assert!(
                name.chars().all(is_kept),
                "{raw:?} produced {name:?}, which is not plain ASCII"
            );
            assert!(
                !name.starts_with('-') && !name.ends_with('-'),
                "{raw:?} produced {name:?}, which leads or trails with a dash"
            );
            assert!(
                name.chars().count() <= MAX_BASE_CHARS,
                "{raw:?} produced {name:?}, which is longer than {MAX_BASE_CHARS}"
            );
        }
    }

    #[test]
    fn sanitizing_keeps_the_project_recognisable() {
        assert_eq!(sanitize("my project"), "my-project");
        assert_eq!(sanitize("b28.dev"), "b28-dev");
        assert_eq!(sanitize("-dash-led"), "dash-led");
        assert_eq!(sanitize(".hidden"), "hidden");
        assert_eq!(sanitize("Lisa"), "Lisa");
        assert_eq!(sanitize("under_score"), "under_score");
        assert_eq!(sanitize("repos/steer"), "repos-steer");
        assert_eq!(sanitize("a  b   c"), "a-b-c");
    }

    #[test]
    fn a_name_with_nothing_usable_in_it_falls_back_to_lisa() {
        assert_eq!(sanitize(".."), FALLBACK_BASE);
        assert_eq!(sanitize("🐼"), FALLBACK_BASE);
        assert_eq!(sanitize(""), FALLBACK_BASE);
    }

    #[test]
    fn truncation_never_leaves_a_trailing_dash() {
        let raw = format!("{}-tail", "x".repeat(MAX_BASE_CHARS - 1));
        let name = sanitize(&raw);

        assert_eq!(name.chars().count(), MAX_BASE_CHARS - 1);
        assert!(!name.ends_with('-'));
    }

    #[test]
    fn a_free_project_name_is_the_session_name() {
        let naming = choose("steer", &["auspicious-panda".to_string()]);

        assert_eq!(naming, SessionNaming::Project("steer".to_string()));
        assert_eq!(naming.name(), Some("steer"));
        assert_eq!(naming.describe(), "steer");
    }

    #[test]
    fn a_crashed_runs_exited_session_costs_a_number_not_the_start() {
        let naming = choose("steer", &["steer".to_string()]);

        assert_eq!(naming.name(), Some("steer-2"));
        assert!(
            naming.describe().contains("already holds \"steer\""),
            "the startup line must say why the name changed: {}",
            naming.describe()
        );
    }

    #[test]
    fn each_further_run_of_one_project_takes_the_next_number() {
        let taken = vec!["steer".to_string(), "steer-2".to_string()];

        assert_eq!(choose("steer", &taken).name(), Some("steer-3"));
    }

    #[test]
    fn a_gap_left_by_a_retired_session_is_reused() {
        let taken = vec!["steer".to_string(), "steer-3".to_string()];

        assert_eq!(choose("steer", &taken).name(), Some("steer-2"));
    }

    #[test]
    fn every_number_taken_hands_the_naming_back_to_zellij() {
        let mut taken = vec!["steer".to_string()];
        taken.extend((2..=MAX_RUN_NUMBER).map(|run| format!("steer-{run}")));

        let naming = choose("steer", &taken);

        assert_eq!(
            naming,
            SessionNaming::Exhausted {
                base: "steer".to_string()
            }
        );
        // No name means no --session, which is the old animal-named start:
        // worse to read, still a start.
        assert_eq!(naming.name(), None);
        assert!(
            naming.describe().contains("zellij delete-session"),
            "the exhausted line must name the way out: {}",
            naming.describe()
        );
    }

    fn session(name: &str, running: bool) -> Session {
        Session {
            name: name.to_string(),
            running,
        }
    }

    /// Measured against Zellij 0.44.3 on 2026-08-13.
    const LISTING: &str = "\
auspicious-panda [Created 1day 17h 32m 54s ago] (EXITED - attach to resurrect)
lisa [Created 19h 22m 14s ago] (EXITED - attach to resurrect)
screen-design [Created 18h 38m 36s ago]
lisa-2 [Created 6m 18s ago] (current)
";

    #[test]
    fn listing_reads_which_sessions_are_still_running() {
        assert_eq!(
            parse_sessions(LISTING),
            vec![
                session("auspicious-panda", false),
                session("lisa", false),
                session("screen-design", true),
                session("lisa-2", true),
            ]
        );
    }

    #[test]
    fn an_empty_or_annotated_listing_is_read_without_inventing_names() {
        assert!(parse_sessions("").is_empty());
        assert!(parse_sessions("\n\n").is_empty());
        // A bare name — what `--short` prints — is a session Lisa knows
        // nothing against, so it counts as running.
        assert_eq!(parse_sessions("steer\n"), vec![session("steer", true)]);
    }

    #[test]
    fn an_unrunnable_zellij_costs_the_name_and_not_the_run() {
        let missing = PathBuf::from("/nonexistent/zellij-that-is-not-there");

        assert!(existing_sessions(&missing).is_empty());
    }

    /// The 2026-08-12 incident, read off a real listing: a board whose run had
    /// finished, whose session was still up, and whose next `lisa loop` took
    /// the name `lisa-2` and started a second scheduler. The session is the
    /// evidence, and it is on this line whether the run has work left or not.
    #[test]
    fn a_session_still_running_on_this_board_is_reported_alongside_the_name() {
        let sessions = parse_sessions(LISTING);

        let board = survey("lisa", &sessions);
        assert_eq!(board.running_here(), ["lisa-2"]);
        assert_eq!(board.naming().name(), Some("lisa-3"));

        // The same listing says nothing against a different project whose own
        // session is dead.
        let other = survey("auspicious-panda", &sessions);
        assert!(other.running_here().is_empty());
        assert_eq!(other.naming().name(), Some("auspicious-panda-2"));
    }

    /// A crashed run's `EXITED` session still holds the name and still costs a
    /// number — the case the numbering exists for, and the only one left now
    /// that a running session refuses the start outright.
    #[test]
    fn an_exited_session_takes_the_name_without_holding_the_board() {
        let board = survey("steer", &[session("steer", false)]);

        assert!(board.running_here().is_empty());
        assert_eq!(board.naming().name(), Some("steer-2"));
    }

    #[test]
    fn only_this_boards_own_names_count_as_this_board() {
        assert!(names_this_board("lisa", "lisa"));
        assert!(names_this_board("lisa", "lisa-2"));
        assert!(names_this_board("lisa", "lisa-99"));
        // Past the numbering, so not a name this board would ever take.
        assert!(!names_this_board("lisa", "lisa-100"));
        assert!(!names_this_board("lisa", "lisa-1"));
        // Hand-named sessions that merely start the same way, and a project
        // whose name merely starts with this one's.
        assert!(!names_this_board("lisa", "lisa-live-codex-7409"));
        assert!(!names_this_board("lisa", "lisa-field-current-8443"));
        assert!(!names_this_board("lisa", "lisandra"));
        assert!(!names_this_board("lisa", "screen-design"));
    }
}
