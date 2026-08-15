//! Whether the process a scheduler wrote down is still on this machine.
//!
//! ## The question the stamp cannot answer
//!
//! [`lisa_core::schedulers`] records two facts about a run: a `stamped_at` it
//! rewrites every poll, and a `zellij_pid` it writes once. Every reader used
//! the first and none used the second, and on 2026-08-14 that cost the
//! `renderer` board forty-five minutes with a ticket ready and nothing running:
//!
//! ```text
//! {"scheduler_id":"renderer-3-49ded6ab","session_name":"renderer-3",
//!  "zellij_pid":15340,"started_at":1786768820,"stamped_at":1786771390, …}
//! ```
//!
//! `stamped_at` was minutes old, pid 15340 was gone, and both recoveries Lisa
//! offers refused on the stamp. **A stamp says a process was alive when it
//! wrote; a pid is a question you can ask now** — and this module is where Lisa
//! asks it.
//!
//! ## What counts as an answer
//!
//! Three, never two. *Gone* frees seats, so it is only ever said on positive
//! evidence; *unknown* keeps them, and a machine Lisa could not ask is unknown
//! rather than empty. Nothing here decides a stamp is stale on its own: a
//! reader combines this answer with the stamp, and the seats go only when both
//! agree.
//!
//! - **A pid nobody holds** is the incident, and it is [`Presence::Gone`].
//! - **A pid somebody else holds** is the trap under it. Pids are reused, and
//!   a scheduler that reads a stranger's pid as its own Zellij server would
//!   fence a board forever. So the process has to *be* a Zellij, and it has to
//!   have been up since before the scheduler that recorded it — a Zellij
//!   younger than the run that wrote the number is a different Zellij.
//! - **A running Zellij session under the recorded name** is what says a run is
//!   here when the pid cannot be read, and it is also the veto: a pid that
//!   looks gone while its session is still listed is a contradiction Lisa
//!   reports as unknown rather than resolving in favour of deletion.
//!
//! Machine-wide by design, like [`crate::busy`]: a scheduler's session belongs
//! to a Zellij server, not to a project directory, and the two Zellijs a Lisa
//! box can have (the one on `PATH`, the one the Debian package pins) both get
//! asked.

use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::schedulers::SchedulerRecord;

/// The Zellij the Debian package pins, which is never on `PATH`.
const PACKAGED_ZELLIJ: &str = "/usr/libexec/lisa/zellij";

/// How much later than its scheduler a Zellij server may have started before
/// Lisa reads the pid as a reused one.
///
/// The server starts first and the plugin inside it loads a moment later, so
/// the honest comparison is "did this process start *after* the run that
/// recorded it". A minute of slack absorbs a slow load and a clock that reads
/// a second differently in two places.
const REUSE_SLACK_SECS: u64 = 60;

/// What this machine says about one scheduler's Zellij server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Presence {
    /// The process is there. Whatever its stamp says, this run exists.
    Running(String),
    /// The process is provably not there. Positive evidence, never an absence
    /// of evidence.
    Gone(String),
    /// Lisa could not find out. Readers keep the seats and fall back to the
    /// stamp.
    Unknown(String),
}

impl Presence {
    /// True only when Lisa can prove the process is gone.
    pub(crate) fn is_gone(&self) -> bool {
        matches!(self, Presence::Gone(_))
    }

    /// The sentence an operator reads: what Lisa asked and what it heard.
    pub(crate) fn because(&self) -> &str {
        match self {
            Presence::Running(why) | Presence::Gone(why) | Presence::Unknown(why) => why,
        }
    }
}

/// What a recorded pid is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Server {
    /// No process holds that number.
    Missing,
    /// A Zellij holds it, up for this long when it could be measured.
    Zellij { up_secs: Option<u64> },
    /// Something else holds it now.
    Other(String),
    /// This machine could not be asked about it.
    Unknown,
}

/// One reading of the machine, asked about many records.
///
/// The session listing is read at most once and then remembered, because a
/// listing that changed between two questions would let one command print two
/// verdicts about one board. It is also read *lazily*: `lisa status` asks about
/// liveness on every call, a pid probe costs a system call, and spawning
/// `zellij list-sessions` for a board whose pids all answer would be a
/// subprocess per status.
pub(crate) struct Machine {
    /// The Zellijs to ask, in the order they are asked.
    candidates: Vec<PathBuf>,
    /// Every Zellij session name this machine reports as running, or `None`
    /// when no Zellij could be run at all — which is *unknown*, not *none*.
    sessions: std::cell::OnceCell<Option<Vec<String>>>,
    /// How a pid is looked up. Injected so the decision below is testable
    /// without spawning processes with chosen ages.
    server: Box<dyn Fn(u32) -> Server>,
}

impl Machine {
    /// Ask this machine, using whichever Zellij the project runs plus the ones
    /// a Lisa box can have.
    pub(crate) fn read(root: &Path) -> Self {
        Self {
            candidates: zellij_candidates(root),
            sessions: std::cell::OnceCell::new(),
            server: Box::new(look_up_pid),
        }
    }

    /// A machine that answers nothing: every record is judged by its stamp
    /// alone, which is the world every reader here described before it learned
    /// to ask, and the one a test states when the question is not the point.
    #[cfg(test)]
    pub(crate) fn unknown() -> Self {
        Self {
            candidates: Vec::new(),
            sessions: std::cell::OnceCell::from(None),
            server: Box::new(|_| Server::Unknown),
        }
    }

    /// A fixed reading, for tests.
    #[cfg(test)]
    pub(crate) fn fixed(
        sessions: Option<Vec<String>>,
        server: impl Fn(u32) -> Server + 'static,
    ) -> Self {
        Self {
            candidates: Vec::new(),
            sessions: std::cell::OnceCell::from(sessions),
            server: Box::new(server),
        }
    }

    /// Whether this machine reports a session of that name as running.
    ///
    /// `None` when there was no listing to read.
    fn session_running(&self, name: &str) -> Option<bool> {
        self.sessions
            .get_or_init(|| running_sessions(&self.candidates))
            .as_ref()
            .map(|running| running.iter().any(|session| session == name))
    }

    /// What this machine says about the process one record named.
    pub(crate) fn look(&self, record: &SchedulerRecord, now: u64) -> Presence {
        let session = record.session_name.as_deref();
        let verdict = match record.zellij_pid.map(|pid| (pid, (self.server)(pid))) {
            Some((pid, Server::Missing)) => Presence::Gone(format!(
                "the Zellij server it recorded, pid {pid}, is not a process on this machine any \
                 more"
            )),
            Some((pid, Server::Other(command))) => Presence::Gone(format!(
                "pid {pid} belongs to `{command}` now, not to the Zellij server this run wrote \
                 down — the number was reused"
            )),
            Some((pid, Server::Zellij { up_secs })) => match reused(up_secs, record, now) {
                Some(started_after) => Presence::Gone(format!(
                    "pid {pid} is a Zellij that started {started_after}s after this run did, so it \
                     is not the server this run wrote down — the number was reused"
                )),
                None => Presence::Running(format!(
                    "pid {pid}, the Zellij server it recorded, is still running here"
                )),
            },
            // No pid to ask about, or a machine that would not answer: the
            // session listing is what is left.
            Some((_, Server::Unknown)) | None => {
                match (session, session.and_then(|name| self.session_running(name))) {
                    (Some(name), Some(true)) => Presence::Running(format!(
                        "its Zellij session {name} is still running on this machine"
                    )),
                    (Some(name), Some(false)) => Presence::Gone(format!(
                        "no Zellij session called {name} is running on this machine"
                    )),
                    (None, _) => Presence::Unknown(
                        "this record names neither a Zellij session nor a server pid, so there is \
                     nothing to ask about"
                            .to_string(),
                    ),
                    (Some(_), None) => Presence::Unknown(
                        "no Zellij on this machine could be asked what it is running".to_string(),
                    ),
                }
            }
        };

        // The veto, and the reason there are two questions rather than one:
        // whatever the pid says, a session Zellij still lists is a run this
        // machine is holding. Measured on 2026-08-14 against a live loop, where
        // `ps -o comm=` truncated `/opt/homebrew/Cellar/…/zellij` to sixteen
        // characters and the pid read as a stranger's. Two answers that
        // disagree are not an answer, and the seats stay.
        if verdict.is_gone() && session.and_then(|name| self.session_running(name)) == Some(true) {
            return Presence::Unknown(format!(
                "{}, and yet a session called {} is still running here. Lisa cannot tell which is \
                 true, so nothing is treated as free",
                verdict.because(),
                session.unwrap_or("?")
            ));
        }
        verdict
    }
}

/// How much later than its scheduler this process started, when that is enough
/// to call the pid reused.
fn reused(up_secs: Option<u64>, record: &SchedulerRecord, now: u64) -> Option<u64> {
    let started = now.checked_sub(up_secs?)?;
    started
        .checked_sub(record.started_at)
        .filter(|after| *after > REUSE_SLACK_SECS)
}

/// The Zellijs worth asking: the one this project is configured to run, the one
/// on `PATH`, and the one the Debian package pins.
fn zellij_candidates(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(validation) = crate::config::load_config(root) {
        let resolved = crate::config::resolve_config(&validation.config, None, None);
        if let Ok(runtime) = crate::runtime::resolve_zellij_runtime(&resolved.zellij_runtime) {
            found.push(runtime.path);
        }
    }
    for candidate in [PathBuf::from("zellij"), PathBuf::from(PACKAGED_ZELLIJ)] {
        if !found.contains(&candidate) {
            found.push(candidate);
        }
    }
    found
}

/// Every session name these Zellijs report as running, or `None` when not one
/// of them could be run.
///
/// The distinction is the whole point: a machine with no Zellij on it has not
/// told Lisa that nothing is running, and reading its silence as *none* would
/// free seats under a live board.
fn running_sessions(candidates: &[PathBuf]) -> Option<Vec<String>> {
    let mut answered = false;
    let mut sessions: Vec<String> = Vec::new();
    for zellij in candidates {
        // `list-sessions` exits non-zero when there are no sessions at all, so
        // the exit status carries nothing here — running the program at all is
        // the answer, and the lines it printed are the rest.
        let Ok(output) = Command::new(zellij)
            .arg("list-sessions")
            .arg("--no-formatting")
            .output()
        else {
            continue;
        };
        answered = true;
        for name in
            crate::session_name::running_session_names(&String::from_utf8_lossy(&output.stdout))
        {
            if !sessions.contains(&name) {
                sessions.push(name);
            }
        }
    }
    answered.then_some(sessions)
}

/// Ask this machine what holds a pid.
#[cfg(unix)]
fn look_up_pid(pid: u32) -> Server {
    let Ok(signed) = i32::try_from(pid) else {
        return Server::Unknown;
    };
    if signed <= 0 {
        return Server::Unknown;
    }

    // SAFETY: kill(pid, 0) sends no signal. It is the standard Unix probe for
    // whether a pid is held at all, and it answers before `ps` is spawned.
    if unsafe { libc::kill(signed, 0) } != 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ESRCH) => Server::Missing,
            // Held by another user. A Zellij started by this scheduler is one
            // this account can signal, so somebody else's process on that
            // number is somebody else's process.
            Some(libc::EPERM) => Server::Other("a process owned by another user".to_string()),
            _ => Server::Unknown,
        };
    }
    identify(pid)
}

#[cfg(not(unix))]
fn look_up_pid(_pid: u32) -> Server {
    Server::Unknown
}

/// What `ps` says a live pid is, and how long it has been up.
///
/// `args` rather than `comm`, because `comm` is truncated: macOS cut
/// `/opt/homebrew/Cellar/zellij/0.44.3/bin/zellij` to `/opt/homebrew/Ce`, and a
/// live scheduler read as a stranger holding a reused pid. The whole command
/// line is longer than any truncation and its first word is still the program.
fn identify(pid: u32) -> Server {
    let Ok(output) = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("args=,etime=")
        .output()
    else {
        return Server::Unknown;
    };
    if !output.status.success() {
        // `ps` ran and found nothing: the process went away between the two
        // questions, which is still gone.
        return Server::Missing;
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let Some(line) = line.lines().next().map(str::trim).filter(|l| !l.is_empty()) else {
        return Server::Unknown;
    };
    read_ps_line(line)
}

/// Read one `ps -o args=,etime=` line: a whole command line, then an elapsed
/// time.
///
/// The command comes first and carries its own spaces, so the elapsed time is
/// taken off the end rather than the command off the front. What names the
/// program is the first word of it — a `zellij` in a later argument is a file
/// somebody is editing, not the server this record was written by.
fn read_ps_line(line: &str) -> Server {
    let Some((command, elapsed)) = line.rsplit_once(char::is_whitespace) else {
        return Server::Unknown;
    };
    let command = command.trim();
    let program = command.split_whitespace().next().unwrap_or(command);
    let is_zellij = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program)
        .contains("zellij");
    if !is_zellij {
        return Server::Other(command.to_string());
    }
    Server::Zellij {
        up_secs: parse_etime(elapsed.trim()),
    }
}

/// Read `ps`'s `etime`: `[[DD-]HH:]MM:SS`.
fn parse_etime(elapsed: &str) -> Option<u64> {
    let (days, clock) = match elapsed.split_once('-') {
        Some((days, clock)) => (days.parse::<u64>().ok()?, clock),
        None => (0, elapsed),
    };
    let mut secs = 0u64;
    for part in clock.split(':') {
        secs = secs
            .checked_mul(60)?
            .checked_add(part.parse::<u64>().ok()?)?;
    }
    Some(days * 86_400 + secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_786_771_628;

    /// The record from the incident, verbatim.
    fn renderer_3() -> SchedulerRecord {
        SchedulerRecord::new(
            "renderer-3-49ded6ab",
            Some("renderer-3".to_string()),
            Some(15340),
            1_786_768_820,
            1_786_771_390,
            5,
        )
    }

    fn machine(sessions: Option<&[&str]>, server: Server) -> Machine {
        let sessions = sessions.map(|names| names.iter().map(|n| n.to_string()).collect());
        Machine::fixed(sessions, move |_| server.clone())
    }

    /// The whole ticket in one assertion: the stamp is minutes old, the pid is
    /// gone, and Lisa now says so.
    #[test]
    fn a_scheduler_whose_zellij_server_is_gone_is_gone() {
        let presence = machine(Some(&[]), Server::Missing).look(&renderer_3(), NOW);

        assert!(presence.is_gone(), "{presence:?}");
        assert!(presence.because().contains("pid 15340"));
        assert!(presence.because().contains("not a process on this machine"));
    }

    /// The trap under the fix: a number this machine handed to something else.
    #[test]
    fn a_pid_that_belongs_to_something_else_now_is_not_a_running_scheduler() {
        let presence =
            machine(Some(&[]), Server::Other("node".to_string())).look(&renderer_3(), NOW);

        assert!(presence.is_gone(), "{presence:?}");
        assert!(presence.because().contains("`node`"));
        assert!(presence.because().contains("reused"));
    }

    /// And the harder half of it: the right program on a reused number. A
    /// Zellij that started after the run that wrote the number down is a
    /// different Zellij.
    #[test]
    fn a_zellij_younger_than_the_run_that_recorded_it_is_a_reused_pid() {
        let record = renderer_3();
        // Up for ten minutes; the record's run started 47 minutes ago.
        let presence = machine(Some(&[]), Server::Zellij { up_secs: Some(600) }).look(&record, NOW);

        assert!(presence.is_gone(), "{presence:?}");
        assert!(presence.because().contains("started"));
        assert!(presence.because().contains("reused"));
    }

    #[test]
    fn the_zellij_server_that_has_been_up_since_the_run_started_is_running() {
        let presence = machine(
            Some(&["renderer-3"]),
            Server::Zellij {
                up_secs: Some(3 * 3600),
            },
        )
        .look(&renderer_3(), NOW);

        assert!(!presence.is_gone(), "{presence:?}");
        assert!(matches!(presence, Presence::Running(_)));
    }

    /// A live run is protected before anything else. A pid Lisa cannot read
    /// falls back to the session listing, and a listed session is a run.
    #[test]
    fn a_session_still_listed_keeps_a_run_alive_when_the_pid_cannot_be_read() {
        let presence = machine(Some(&["renderer-3"]), Server::Unknown).look(&renderer_3(), NOW);

        assert!(matches!(presence, Presence::Running(_)), "{presence:?}");
        assert!(presence.because().contains("renderer-3"));
    }

    /// Two answers that disagree are not an answer. The seats stay.
    ///
    /// Every way the pid can read as gone is vetoed the same way, because the
    /// first live loop this was tried against read as a stranger's pid rather
    /// than a missing one — a truncated `ps` line — and a run with panes in it
    /// would have had its seats freed.
    #[test]
    fn any_dead_pid_under_a_running_session_is_reported_as_unknown() {
        for gone in [
            Server::Missing,
            Server::Other("/opt/homebrew/Ce".to_string()),
            Server::Zellij { up_secs: Some(5) },
        ] {
            let presence = machine(Some(&["renderer-3"]), gone.clone()).look(&renderer_3(), NOW);

            assert!(matches!(presence, Presence::Unknown(_)), "{gone:?}");
            assert!(!presence.is_gone(), "{gone:?}");
            assert!(presence.because().contains("cannot tell which is true"));
        }
    }

    /// A machine with no Zellij to ask has not said that nothing is running.
    #[test]
    fn a_machine_that_could_not_be_asked_is_unknown_and_never_gone() {
        let presence = machine(None, Server::Unknown).look(&renderer_3(), NOW);

        assert!(matches!(presence, Presence::Unknown(_)), "{presence:?}");
        assert!(presence.because().contains("could be asked"));
        assert!(!Machine::unknown().look(&renderer_3(), NOW).is_gone());
    }

    /// A record with no pid — an older Lisa wrote it — is answered by the
    /// listing alone, in both directions.
    #[test]
    fn a_record_without_a_pid_is_answered_by_the_session_listing() {
        let mut record = renderer_3();
        record.zellij_pid = None;

        let gone = machine(Some(&["something-else"]), Server::Unknown).look(&record, NOW);
        assert!(gone.is_gone(), "{gone:?}");
        assert!(gone
            .because()
            .contains("no Zellij session called renderer-3"));

        let running = machine(Some(&["renderer-3"]), Server::Unknown).look(&record, NOW);
        assert!(matches!(running, Presence::Running(_)), "{running:?}");
    }

    /// A record that names nothing at all can be asked nothing at all.
    #[test]
    fn a_record_naming_neither_a_session_nor_a_pid_is_unknown() {
        let mut record = renderer_3();
        record.zellij_pid = None;
        record.session_name = None;

        let presence = machine(Some(&[]), Server::Unknown).look(&record, NOW);
        assert!(matches!(presence, Presence::Unknown(_)), "{presence:?}");
    }

    #[test]
    fn ps_lines_read_as_the_program_and_how_long_it_has_been_up() {
        assert_eq!(
            read_ps_line("/opt/homebrew/bin/zellij 01-10:40:03"),
            Server::Zellij {
                up_secs: Some(86_400 + 10 * 3600 + 40 * 60 + 3)
            }
        );
        assert_eq!(
            read_ps_line("zellij 04:21"),
            Server::Zellij {
                up_secs: Some(4 * 60 + 21)
            }
        );
        assert_eq!(
            read_ps_line("/bin/zsh 00:00"),
            Server::Other("/bin/zsh".to_string())
        );
        // A path with a space in it still ends with its elapsed time.
        assert_eq!(
            read_ps_line("/Applications/My Terminal.app/node 10:00"),
            Server::Other("/Applications/My Terminal.app/node".to_string())
        );
        assert_eq!(read_ps_line("zellij"), Server::Unknown);
    }

    /// The line this machine actually printed for a live scheduler on
    /// 2026-08-14, with the arguments a Zellij server carries. Read from
    /// `comm` it arrived truncated to `/opt/homebrew/Ce` and the run read as
    /// dead; read from `args` the program is still the program.
    #[test]
    fn a_zellij_server_with_its_arguments_is_still_a_zellij_server() {
        assert_eq!(
            read_ps_line(
                "/opt/homebrew/Cellar/zellij/0.44.3/bin/zellij --server \
                 /Users/johnchen/Library/Caches/org.Zellij-Contributors.zellij/0.44.3/repro 00:19"
            ),
            Server::Zellij { up_secs: Some(19) }
        );
        // And the looseness that would let a full command line lie: what names
        // the program is its first word.
        assert_eq!(
            read_ps_line("vim docs/zellij-notes.md 03:00"),
            Server::Other("vim docs/zellij-notes.md".to_string())
        );
    }

    #[test]
    fn an_unreadable_elapsed_time_still_leaves_a_running_zellij_running() {
        assert_eq!(parse_etime("bogus"), None);
        assert_eq!(parse_etime("2-00:00:01"), Some(2 * 86_400 + 1));
        let presence =
            machine(Some(&[]), Server::Zellij { up_secs: None }).look(&renderer_3(), NOW);
        assert!(matches!(presence, Presence::Running(_)), "{presence:?}");
    }

    /// This machine, asked about itself: `lisa` is running, so its own pid is
    /// held by something that is not a Zellij.
    #[test]
    fn this_process_is_a_real_pid_that_is_not_a_zellij_server() {
        match look_up_pid(std::process::id()) {
            Server::Other(command) => assert!(!command.contains("zellij"), "{command}"),
            // A test binary `ps` could not be asked about is not a failure of
            // the decision this module makes.
            other => assert!(matches!(other, Server::Unknown), "{other:?}"),
        }
    }
}
