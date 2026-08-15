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
//! ## The day is in the name
//!
//! Numbering alone made the name count the dead and say nothing else. `steer-3`
//! meant two sessions of that name came before it and are gone — which is not a
//! fact anybody asks for, and stops being true the moment they are swept:
//! `zellij delete-all-sessions` on this desk cleared 320 `EXITED` sessions and
//! the next run was `steer` again. The same run was `steer-3` on a desk nobody
//! had cleaned and `steer` on one just swept.
//!
//! So the name carries the day the run started — `steer-0815` — and the number
//! is scoped to it: `steer-0815-2` is this project's second run *today*. The
//! number now counts something bounded, resets by itself at midnight, and no
//! longer depends on when anybody last swept.
//!
//! The day and not the hour. The only question a name has to answer is *is this
//! session from a previous day*; how many hours old a live one is, is already a
//! column in `zellij list-sessions` and a line in `lisa schedulers`, and four
//! more digits buy nothing in the one place that cannot afford them.
//!
//! Width is what it costs, and where it costs it is the point: the project
//! comes first and a terminal tab truncates from the right, so the date is what
//! a narrow tab eats — `steer-0815` reads as `steer-08…`, still the project. A
//! date that pushed the project name out of a tab would break the reason the
//! name exists at all.
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
///
/// The day rides on the end of this, not inside it. A `-0815` and at most a
/// `-99` behind it are five and three more characters, and they are deliberately
/// the ones a narrow tab loses first: the project name is why anybody reads the
/// name, and `zellij list-sessions` still carries the age of everything the tab
/// cut off.
const MAX_BASE_CHARS: usize = 32;

/// Highest run number tried before the naming is left to Zellij.
///
/// Ninety-nine runs of one project in one day is far past anything this desk
/// has done — the busiest was six — and the number is now bounded by the day
/// rather than by how long ago somebody last swept.
const MAX_RUN_NUMBER: u32 = 99;

/// Characters kept verbatim in a session name.
fn is_kept(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

/// The day a run started, as a session name carries it: a month and a day, no
/// year, four digits.
///
/// No year because the name is read off a tab and a year buys nothing there —
/// nothing on this desk survives twelve months, and everything that could is
/// caught by the stamp and by `zellij list-sessions` long before the date wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartDay {
    month: u32,
    day: u32,
}

impl StartDay {
    /// A day, or nothing when the numbers are not a calendar date.
    fn new(month: u32, day: u32) -> Option<Self> {
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(Self { month, day })
    }

    /// The four digits that go in a session name.
    pub(crate) fn stamp(self) -> String {
        format!("{:02}{:02}", self.month, self.day)
    }

    /// How a person says it out loud, and how Lisa writes it in a sentence.
    fn spoken(self) -> String {
        format!("{:02}-{:02}", self.month, self.day)
    }

    /// Read four digits back into a day.
    ///
    /// Exactly four, and a real month and day: this is what separates
    /// `steer-0815` from `lisa-field-current-8443`, which is somebody's
    /// hand-named session and not a date at all.
    fn read(raw: &str) -> Option<Self> {
        if raw.len() != 4 || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        Self::new(raw[..2].parse().ok()?, raw[2..].parse().ok()?)
    }
}

/// Whether this piece of a name is one of the run numbers the loop hands out.
fn is_run_number(raw: &str) -> bool {
    raw.parse::<u32>()
        .is_ok_and(|run| (2..=MAX_RUN_NUMBER).contains(&run))
}

/// Today, on this machine's clock.
pub(crate) fn today() -> Option<StartDay> {
    day_at(crate::seats::now_secs()?)
}

/// The local day a moment falls on.
///
/// Local and not UTC, because the reader is looking at a tab on this desk. A run
/// started at six in the evening in California is `0814`; UTC would name it
/// `0815`, and a date that disagrees with the wall clock behind the screen
/// answers no question anybody asked.
#[cfg(unix)]
pub(crate) fn day_at(now: u64) -> Option<StartDay> {
    let clock = libc::time_t::try_from(now).ok()?;
    // SAFETY: `localtime_r` writes only into the `tm` the caller owns, which is
    // what makes it the reentrant one. A zeroed `tm` is a valid destination.
    let mut broken_down: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&clock, &mut broken_down) }.is_null() {
        return None;
    }
    StartDay::new(
        u32::try_from(broken_down.tm_mon).ok()? + 1,
        u32::try_from(broken_down.tm_mday).ok()?,
    )
}

/// The same day, on a platform with no `localtime_r` to ask.
///
/// UTC is the best this can do here, which is right for most of the day and a
/// few hours out at its edges. Lisa's runs are Unix runs; this exists so the
/// crate still compiles rather than because anybody reads it.
#[cfg(not(unix))]
pub(crate) fn day_at(now: u64) -> Option<StartDay> {
    let (_, month, day) = crate::channel::civil_from_days(i64::try_from(now).ok()? / 86_400);
    StartDay::new(u32::try_from(month).ok()?, u32::try_from(day).ok()?)
}

/// What a session's name says about the day it started.
///
/// For a reader holding nothing but the name — a line of `zellij list-sessions`,
/// the string `--stop` was given, the refusal a second `lisa loop` prints. It
/// does not need the project's base name, because the day is the tail of the
/// name whatever the front of it is.
///
/// `None` for a name from before this was in it. A dateless `steer-3` says
/// nothing about when, which is the whole reason the date was added.
pub(crate) fn started_on(name: &str) -> Option<StartDay> {
    let mut tail = name.rsplit('-');
    let last = tail.next()?;
    if let Some(day) = StartDay::read(last) {
        return Some(day);
    }
    // `steer-0815-2`: the run number is last and the day is behind it.
    is_run_number(last).then_some(())?;
    StartDay::read(tail.next()?)
}

/// The one line a reader gets from the name alone, when the name says this
/// session started before today.
///
/// `None` when the name carries no day, and when the day it carries is today's:
/// a session started this morning is the ordinary case and saying so on every
/// line would bury the one line that matters.
pub(crate) fn previous_day_note(name: &str, today: StartDay) -> Option<String> {
    let started = started_on(name).filter(|started| *started != today)?;
    Some(format!(
        "its name says it started on {}, and today is {} — this session is from a previous day",
        started.spoken(),
        today.spoken()
    ))
}

/// How this run's session got its name, and what to tell the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionNaming {
    /// This project's name for today was free — the ordinary first run of a day.
    Project(String),
    /// Another session — running, or `EXITED` after a crash — already holds
    /// today's name, so this run is the next numbered one of the day.
    NextRun { name: String, stem: String },
    /// Every numbered name for today is held. Zellij names this session itself.
    Exhausted { stem: String },
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
            Self::NextRun { name, stem } => format!(
                "{name} (another session already holds \"{stem}\" — `zellij list-sessions` shows it)"
            ),
            Self::Exhausted { stem } => format!(
                "named by Zellij — \"{stem}\" through \"{stem}-{MAX_RUN_NUMBER}\" are all taken. \
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
    survey(
        &project_base(root),
        today(),
        &existing_sessions(zellij_path),
    )
}

/// Read one listing into the two facts the loop acts on.
fn survey(base: &str, today: Option<StartDay>, sessions: &[Session]) -> BoardSessions {
    let taken: Vec<String> = sessions.iter().map(|s| s.name.clone()).collect();
    let running_here: Vec<String> = sessions
        .iter()
        .filter(|session| session.running && names_this_board(base, &session.name))
        .map(|session| session.name.clone())
        .collect();

    BoardSessions {
        naming: choose(base, today, &taken),
        running_here,
    }
}

/// Whether a session name is one this project's own naming could have produced:
/// its name, a day of it, or one of that day's numbered runs.
///
/// Every shape Lisa has ever given this board, not only the one it gives today.
/// A `steer-3` still running here was started by an older Lisa and is still a
/// scheduler on this board, and a refusal that stopped recognising it would let
/// a second one onto a live board — the 2026-08-12 incident, reintroduced by an
/// upgrade.
///
/// Deliberately exact in the other direction too. `lisa-0815-2` is this board's
/// second run today; `lisa-live-codex` is somebody's hand-named session that
/// merely starts the same way, and refusing a start over it would be a lockout
/// with no way out.
fn names_this_board(base: &str, name: &str) -> bool {
    if name == base {
        return true;
    }
    let Some(rest) = name
        .strip_prefix(base)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    // `steer-3` — a name an older Lisa gave this board.
    if is_run_number(rest) {
        return true;
    }
    match rest.split_once('-') {
        // `steer-0815-2`
        Some((day, run)) => StartDay::read(day).is_some() && is_run_number(run),
        // `steer-0815`
        None => StartDay::read(rest).is_some(),
    }
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

/// Take the project's name for today if it is free, else the next free run
/// number of the day.
///
/// A machine that cannot say what day it is falls all the way back to the old
/// dateless naming rather than refusing: a clock is not a reason not to start a
/// run, and a name with no date in it still names the project.
fn choose(base: &str, today: Option<StartDay>, taken: &[String]) -> SessionNaming {
    let stem = match today {
        Some(day) => format!("{base}-{}", day.stamp()),
        None => base.to_string(),
    };

    if !taken.contains(&stem) {
        return SessionNaming::Project(stem);
    }

    for run in 2..=MAX_RUN_NUMBER {
        let candidate = format!("{stem}-{run}");
        if !taken.contains(&candidate) {
            return SessionNaming::NextRun {
                name: candidate,
                stem,
            };
        }
    }

    SessionNaming::Exhausted { stem }
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

    /// 15 August, the day the operator asked for this.
    const AUG_15: Option<StartDay> = Some(StartDay { month: 8, day: 15 });

    fn taken(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_first_run_of_a_day_is_the_project_and_the_day() {
        let naming = choose("steer", AUG_15, &taken(&["auspicious-panda"]));

        assert_eq!(naming, SessionNaming::Project("steer-0815".to_string()));
        assert_eq!(naming.name(), Some("steer-0815"));
        assert_eq!(naming.describe(), "steer-0815");
    }

    #[test]
    fn a_crashed_runs_exited_session_costs_a_number_not_the_start() {
        let naming = choose("steer", AUG_15, &taken(&["steer-0815"]));

        assert_eq!(naming.name(), Some("steer-0815-2"));
        assert!(
            naming.describe().contains("already holds \"steer-0815\""),
            "the startup line must say why the name changed: {}",
            naming.describe()
        );
    }

    /// The ordinary case on this desk, and the one `MMDD` alone cannot express:
    /// `renderer` had six sessions on 2026-08-14. Both names say the day, and
    /// the number behind it tells them apart.
    #[test]
    fn two_runs_of_one_project_on_one_day_both_say_the_day_and_differ() {
        let first = choose("renderer", AUG_15, &[]);
        assert_eq!(first.name(), Some("renderer-0815"));

        let second = choose("renderer", AUG_15, &taken(&["renderer-0815"]));
        assert_eq!(second.name(), Some("renderer-0815-2"));

        assert_ne!(first.name(), second.name());
        for naming in [&first, &second] {
            assert_eq!(
                started_on(naming.name().unwrap()),
                AUG_15,
                "{:?} does not say when it started",
                naming.name()
            );
        }
    }

    #[test]
    fn each_further_run_of_one_day_takes_the_next_number() {
        assert_eq!(
            choose("steer", AUG_15, &taken(&["steer-0815", "steer-0815-2"])).name(),
            Some("steer-0815-3")
        );
    }

    /// Yesterday's sessions are not this day's, so today starts at one again
    /// however many are still lying around. This is the trap the date closes:
    /// the number no longer depends on when anybody last swept.
    #[test]
    fn yesterdays_sessions_do_not_number_todays() {
        let yesterday = taken(&["steer-0814", "steer-0814-2", "steer-0814-3"]);

        assert_eq!(
            choose("steer", AUG_15, &yesterday).name(),
            Some("steer-0815")
        );
    }

    #[test]
    fn a_gap_left_by_a_retired_session_is_reused() {
        let taken = taken(&["steer-0815", "steer-0815-3"]);

        assert_eq!(choose("steer", AUG_15, &taken).name(), Some("steer-0815-2"));
    }

    /// A clock that cannot be read costs the date, never the run.
    #[test]
    fn a_machine_that_cannot_say_what_day_it_is_still_starts() {
        assert_eq!(choose("steer", None, &[]).name(), Some("steer"));
        assert_eq!(
            choose("steer", None, &taken(&["steer"])).name(),
            Some("steer-2")
        );
    }

    #[test]
    fn every_number_taken_hands_the_naming_back_to_zellij() {
        let mut all = vec!["steer-0815".to_string()];
        all.extend((2..=MAX_RUN_NUMBER).map(|run| format!("steer-0815-{run}")));

        let naming = choose("steer", AUG_15, &all);

        assert_eq!(
            naming,
            SessionNaming::Exhausted {
                stem: "steer-0815".to_string()
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
    fn a_name_says_the_day_it_started_and_only_a_real_one() {
        assert_eq!(started_on("steer-0815"), AUG_15);
        assert_eq!(started_on("steer-0815-2"), AUG_15);
        assert_eq!(started_on("steer-0815-99"), AUG_15);
        assert_eq!(started_on("a-very-long-name-0101"), StartDay::new(1, 1));
        assert_eq!(started_on("steer-1231"), StartDay::new(12, 31));

        // Names from before the date was in them say nothing about when, which
        // is exactly the complaint.
        assert_eq!(started_on("steer"), None);
        assert_eq!(started_on("steer-3"), None);
        assert_eq!(started_on("auspicious-panda"), None);
        // Four digits that are not a calendar day, and a hand-named session
        // that ends in some other number entirely.
        assert_eq!(started_on("steer-9999"), None);
        assert_eq!(started_on("steer-0015"), None);
        assert_eq!(started_on("steer-0800"), None);
        assert_eq!(started_on("lisa-field-current-8443"), None);
        assert_eq!(started_on("steer-0815-100"), None);
    }

    /// The staleness check itself, from the name and nothing else.
    #[test]
    fn a_name_from_a_previous_day_says_so_and_todays_stays_quiet() {
        let today = StartDay { month: 8, day: 15 };

        let note = previous_day_note("renderer-0814-3", today).expect("a previous day");
        assert!(note.contains("08-14"), "{note}");
        assert!(note.contains("08-15"), "{note}");
        assert!(note.contains("from a previous day"), "{note}");

        assert_eq!(previous_day_note("renderer-0815", today), None);
        assert_eq!(previous_day_note("renderer-0815-2", today), None);
        // Nothing to say about a name that carries no day at all.
        assert_eq!(previous_day_note("renderer-3", today), None);
        assert_eq!(previous_day_note("auspicious-panda", today), None);
    }

    /// This machine, asked what day it is: whatever it answers has to be a day.
    #[test]
    fn this_machines_own_clock_reads_as_a_calendar_day() {
        let day = today().expect("this machine knows what day it is");

        assert!((1..=12).contains(&day.month), "{day:?}");
        assert!((1..=31).contains(&day.day), "{day:?}");
        assert_eq!(StartDay::read(&day.stamp()), Some(day));
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

        // `lisa-2` was named by an older Lisa and is still running here, so it
        // still holds this board — and today's name is free, because a name
        // from before the date was in it is not one of today's.
        let board = survey("lisa", AUG_15, &sessions);
        assert_eq!(board.running_here(), ["lisa-2"]);
        assert_eq!(board.naming().name(), Some("lisa-0815"));

        // The same listing says nothing against a different project whose own
        // session is dead.
        let other = survey("auspicious-panda", AUG_15, &sessions);
        assert!(other.running_here().is_empty());
        assert_eq!(other.naming().name(), Some("auspicious-panda-0815"));
    }

    /// A run started today, still up, holds the board when the next `lisa loop`
    /// of the same day comes along — which is the case the refusal exists for.
    #[test]
    fn todays_own_session_still_running_holds_the_board() {
        let running = [session("steer-0815", true), session("steer-0815-2", true)];

        let board = survey("steer", AUG_15, &running);
        assert_eq!(board.running_here(), ["steer-0815", "steer-0815-2"]);
        assert_eq!(board.naming().name(), Some("steer-0815-3"));
    }

    /// A crashed run's `EXITED` session still holds the name and still costs a
    /// number — the case the numbering exists for, and the only one left now
    /// that a running session refuses the start outright.
    #[test]
    fn an_exited_session_takes_the_name_without_holding_the_board() {
        let board = survey("steer", AUG_15, &[session("steer-0815", false)]);

        assert!(board.running_here().is_empty());
        assert_eq!(board.naming().name(), Some("steer-0815-2"));
    }

    #[test]
    fn only_this_boards_own_names_count_as_this_board() {
        // Today's shapes.
        assert!(names_this_board("lisa", "lisa-0815"));
        assert!(names_this_board("lisa", "lisa-0815-2"));
        assert!(names_this_board("lisa", "lisa-1231-99"));
        // And every shape an older Lisa gave this board, because one of those
        // still running is still a scheduler holding it.
        assert!(names_this_board("lisa", "lisa"));
        assert!(names_this_board("lisa", "lisa-2"));
        assert!(names_this_board("lisa", "lisa-99"));
        // Past the numbering, so not a name this board would ever take.
        assert!(!names_this_board("lisa", "lisa-100"));
        assert!(!names_this_board("lisa", "lisa-1"));
        assert!(!names_this_board("lisa", "lisa-0815-100"));
        // Four digits that are not a day.
        assert!(!names_this_board("lisa", "lisa-9999"));
        // Hand-named sessions that merely start the same way, and a project
        // whose name merely starts with this one's.
        assert!(!names_this_board("lisa", "lisa-live-codex-7409"));
        assert!(!names_this_board("lisa", "lisa-field-current-8443"));
        assert!(!names_this_board("lisa", "lisa-0815-nightly"));
        assert!(!names_this_board("lisa", "lisandra"));
        assert!(!names_this_board("lisa", "screen-design"));
    }

    /// The name has to stay a legal Zellij session name and a legal directory
    /// with the date on it, not only without.
    #[test]
    fn a_dated_name_is_still_a_legal_session_name() {
        let long = "a-very-long-repository-directory-name-that-nobody-would-truncate-well";
        for base in [sanitize(long), sanitize("b28.dev"), sanitize("🐼")] {
            let mut all = vec![format!("{base}-0815")];
            all.extend((2..=MAX_RUN_NUMBER).map(|run| format!("{base}-0815-{run}")));

            for name in &all {
                assert!(!name.is_empty());
                assert!(name.chars().all(is_kept), "{name:?} is not plain ASCII");
                assert!(
                    !name.starts_with('-') && !name.ends_with('-'),
                    "{name:?} leads or trails with a dash"
                );
                assert!(!name.contains('/'), "{name:?} contains a slash");
            }
            // The widest a name can get: base, day, and the highest run number.
            assert_eq!(
                all.last().unwrap().chars().count(),
                base.chars().count() + 5 + 3
            );
        }
    }
}
