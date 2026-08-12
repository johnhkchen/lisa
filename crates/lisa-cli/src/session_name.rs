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
//! start. A crashed run leaves exactly that behind, so a taken name never
//! stops the loop here: the next free numbered name is used instead and the
//! old session is left where the operator can still find it.

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

/// Decide this run's session name by asking Zellij which names are held.
pub(crate) fn resolve(root: &Path, zellij_path: &Path) -> SessionNaming {
    choose(&project_base(root), &existing_session_names(zellij_path))
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

/// Every session name Zellij currently holds, running or `EXITED`.
///
/// `list-sessions` exits non-zero when there are no sessions at all, so the
/// exit status carries no information this needs: the names it printed do. A
/// listing that cannot be read is an empty one, which at worst costs the run
/// the project's name and never the start.
fn existing_session_names(zellij_path: &Path) -> Vec<String> {
    let output = Command::new(zellij_path)
        .arg("list-sessions")
        .arg("--short")
        .arg("--no-formatting")
        .output();

    match output {
        Ok(output) => parse_session_names(&String::from_utf8_lossy(&output.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Read session names out of `zellij list-sessions --short --no-formatting`.
fn parse_session_names(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
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

    #[test]
    fn listing_reads_live_and_exited_names_alike() {
        let listing = "steer\nlisa\nauspicious-panda\n";

        assert_eq!(
            parse_session_names(listing),
            vec!["steer", "lisa", "auspicious-panda"]
        );
    }

    #[test]
    fn an_empty_or_annotated_listing_is_read_without_inventing_names() {
        assert!(parse_session_names("").is_empty());
        assert!(parse_session_names("\n\n").is_empty());
        // `--short` prints bare names; tolerate a marker if a version adds one.
        assert_eq!(parse_session_names("steer (current)\n"), vec!["steer"]);
    }

    #[test]
    fn an_unrunnable_zellij_costs_the_name_and_not_the_run() {
        let missing = PathBuf::from("/nonexistent/zellij-that-is-not-there");

        assert!(existing_session_names(&missing).is_empty());
    }
}
