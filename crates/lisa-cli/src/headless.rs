//! Running a board on a host that has no terminal.
//!
//! Zellij does two jobs in a Lisa run. It gives each agent a pane with a
//! terminal, which agents genuinely need, and it draws a dashboard, which is
//! for a person. On a host reached by `ssh -T`, inside a container, or in a
//! Codespace, the second job has no audience — and the first still has to
//! happen. Zellij refuses to start either way:
//!
//! ```text
//! could not enable raw mode: Os { code: 6, message: "Device not configured" }
//! ```
//!
//! So Lisa opens a terminal of its own. This module hands the Zellij client a
//! pseudo-terminal that Lisa owns, reads what the client draws on it and
//! throws it away, and keeps the first few kilobytes so a client that dies at
//! startup can still say why.
//!
//! **Nothing else about the run changes.** The panes Zellij opens inside that
//! terminal are ordinary Zellij panes with ordinary ptys of their own, the
//! plugin still runs in the Zellij server, and the hooks in those panes write
//! the same `pane-N.started` / `.alive` / `.heartbeat` files they always did.
//! The one thing lost is the dashboard, and only because nobody is there to
//! read it.

use std::process::ExitStatus;

/// The size Lisa gives the terminal it opens. Nobody is looking at it, but
/// Zellij lays the panes out against it and the agents inside them wrap their
/// output to it, so it has to be a plausible window rather than the 0×0 an
/// unconfigured pty starts at.
pub const HEADLESS_COLS: u16 = 200;
pub const HEADLESS_ROWS: u16 = 50;

/// How much of the client's output to keep. Enough for a startup failure,
/// which is printed before anything takes the screen over, and far too little
/// for a dashboard that redraws four times a second to fill.
const RETAINED_BYTES: usize = 8 * 1024;

/// What became of a client Lisa ran on a terminal of its own.
pub struct PtyRun {
    /// How the Zellij client exited.
    pub status: ExitStatus,
    /// True when Lisa stopped it on the operator's signal rather than the
    /// client stopping by itself. The run on the board is unaffected either
    /// way — it lives in the Zellij server, not in the client.
    pub interrupted: bool,
    /// The first bytes the client wrote, retained for a failure to explain
    /// itself with.
    head: Vec<u8>,
}

impl PtyRun {
    /// What the client printed into the terminal nobody watched, as something
    /// an operator can read. `None` when it printed nothing worth showing.
    pub fn transcript_head(&self) -> Option<String> {
        let text = legible(&self.head);
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

#[cfg(unix)]
pub use unix::run_on_pty;

#[cfg(not(unix))]
pub fn run_on_pty(_command: std::process::Command) -> Result<PtyRun, String> {
    Err(
        "A headless run needs a Unix pseudo-terminal, which this platform does not have."
            .to_string(),
    )
}

#[cfg(unix)]
mod unix {
    use super::{PtyRun, HEADLESS_COLS, HEADLESS_ROWS, RETAINED_BYTES};
    use std::ffi::CStr;
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use std::sync::{Arc, Mutex};

    /// The client Lisa is waiting on, for the signal handler to reach. Zero
    /// until there is one.
    static CLIENT_PID: AtomicI32 = AtomicI32::new(0);
    /// Set when a signal handler asked the client to stop, so the wait below
    /// can tell "the operator ended this" from "it fell over".
    static STOPPED: AtomicBool = AtomicBool::new(false);

    /// Ctrl-C, `kill`, or a dropped SSH connection. In a run with a window,
    /// closing the window stops the client and leaves the run on the board;
    /// this is the same act, so it does the same thing — pass the stop to the
    /// client and let the Zellij server carry on scheduling.
    extern "C" fn stop_client(_signal: libc::c_int) {
        STOPPED.store(true, Ordering::SeqCst);
        let pid = CLIENT_PID.load(Ordering::SeqCst);
        if pid > 0 {
            // Async-signal-safe: one atomic load and one kill(2).
            unsafe { libc::kill(pid, libc::SIGTERM) };
        }
    }

    /// Run `command` on a pseudo-terminal Lisa opens, drains, and discards.
    ///
    /// Returns when the client exits. The Zellij server it started does not
    /// exit with it, which is the property that makes this usable over SSH: a
    /// dropped connection costs the client and not the run.
    pub fn run_on_pty(mut command: Command) -> Result<PtyRun, String> {
        let (master, slave) = open_pty()?;
        set_window_size(&slave, HEADLESS_COLS, HEADLESS_ROWS)?;

        // The child gets its own session so the pty can become its controlling
        // terminal, claims it, and points all three standard descriptors at it.
        // Everything in the closure runs between fork and exec, so it is
        // syscalls on an already-open descriptor and nothing else.
        let slave_fd = slave.as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                for target in 0..3 {
                    if libc::dup2(slave_fd, target) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                if slave_fd > 2 {
                    libc::close(slave_fd);
                }
                Ok(())
            });
        }
        // A terminal Lisa opened is not the operator's terminal, and Zellij
        // reads both of these to decide what it may draw.
        command.env("TERM", "xterm-256color");
        command.env("COLUMNS", HEADLESS_COLS.to_string());
        command.env("LINES", HEADLESS_ROWS.to_string());

        let mut child = command.spawn().map_err(|error| {
            format!("Failed to start Zellij on a terminal Lisa opened: {error}")
        })?;
        // The child inherited this end at fork, so Lisa can let go of it — and
        // must, or the master would never reach end of file.
        drop(slave);

        STOPPED.store(false, Ordering::SeqCst);
        CLIENT_PID.store(child.id() as i32, Ordering::SeqCst);
        catch_stops();

        // Someone has to read the terminal or the client blocks writing to it.
        // The first few kilobytes are kept; the rest is dropped on the floor,
        // which is the whole point — it is a dashboard with no reader.
        let master = Arc::new(File::from(master));
        let head = Arc::new(Mutex::new(Vec::new()));
        {
            let master = Arc::clone(&master);
            let head = Arc::clone(&head);
            // Deliberately not joined. The Zellij server keeps the other end of
            // this pty open after the client exits, so the read would not
            // return; the run is over either way and the process is about to
            // print its summary and leave.
            std::thread::spawn(move || drain(&master, &head));
        }

        let status = child
            .wait()
            .map_err(|error| format!("Failed waiting on Zellij: {error}"))?;
        CLIENT_PID.store(0, Ordering::SeqCst);

        let head = head.lock().map(|bytes| bytes.clone()).unwrap_or_default();
        Ok(PtyRun {
            status,
            interrupted: STOPPED.load(Ordering::SeqCst),
            head,
        })
    }

    /// Open a pseudo-terminal and return both ends: the one Lisa reads, and the
    /// one the client will be handed as its stdin, stdout, and stderr.
    fn open_pty() -> Result<(OwnedFd, OwnedFd), String> {
        // SAFETY: the POSIX pty opening sequence, checked at every step. Each
        // raw fd is wrapped in an OwnedFd before the next fallible call, so no
        // error path leaks one.
        unsafe {
            let raw = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            if raw < 0 {
                return Err(format!(
                    "Lisa could not open a terminal to run Zellij on: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let master = OwnedFd::from_raw_fd(raw);
            if libc::grantpt(raw) != 0 || libc::unlockpt(raw) != 0 {
                return Err(format!(
                    "Lisa could not prepare the terminal it opened for Zellij: {}",
                    std::io::Error::last_os_error()
                ));
            }
            // `ptsname` hands back a pointer into static storage, so the name is
            // copied here, on this thread, before anything else can call it.
            let name = libc::ptsname(raw);
            if name.is_null() {
                return Err(format!(
                    "Lisa could not name the terminal it opened for Zellij: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let name = CStr::from_ptr(name).to_owned();
            // O_NOCTTY: this process keeps whatever controlling terminal it has
            // (usually none). The child claims this one for itself, after fork.
            let raw_slave = libc::open(name.as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
            if raw_slave < 0 {
                return Err(format!(
                    "Lisa could not open the terminal it made for Zellij ({}): {}",
                    name.to_string_lossy(),
                    std::io::Error::last_os_error()
                ));
            }
            Ok((master, OwnedFd::from_raw_fd(raw_slave)))
        }
    }

    /// Size the terminal before anything starts in it. Darwin answers
    /// `TIOCSWINSZ` on the master with `ENOTTY`, so the size goes on the slave
    /// — which is the end the client actually reads it from on both platforms.
    fn set_window_size(slave: &OwnedFd, cols: u16, rows: u16) -> Result<(), String> {
        let size = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: a TIOCSWINSZ on a pty Lisa just opened, with a fully
        // initialised winsize.
        let result = unsafe { libc::ioctl(slave.as_raw_fd(), libc::TIOCSWINSZ as _, &size) };
        if result < 0 {
            return Err(format!(
                "Lisa could not size the terminal it opened for Zellij: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    /// Ask for the stops that mean "the operator is done watching": Ctrl-C, a
    /// plain `kill`, and the hangup an SSH session sends when it drops.
    fn catch_stops() {
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            // SAFETY: installing a handler that only stores an atomic and
            // calls kill(2).
            unsafe { libc::signal(signal, stop_client as *const () as libc::sighandler_t) };
        }
    }

    fn drain(master: &File, head: &Mutex<Vec<u8>>) {
        let mut reader = master;
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if let Ok(mut head) = head.lock() {
                        if head.len() < RETAINED_BYTES {
                            let room = RETAINED_BYTES - head.len();
                            head.extend_from_slice(&buffer[..read.min(room)]);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                // A pty master reads EIO once the last slave closes; that is
                // this pty's end of file, not a fault.
                Err(_) => break,
            }
        }
    }
}

/// Terminal output as something a person can read: escape sequences dropped,
/// blank and repeated lines collapsed, and cut off well before it becomes a
/// screen dump.
fn legible(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut plain = String::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\u{1b}' => match characters.next() {
                // CSI: runs to a byte in @..~.
                Some('[') => {
                    for c in characters.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: runs to BEL or ESC.
                Some(']') => {
                    for c in characters.by_ref() {
                        if c == '\u{7}' || c == '\u{1b}' {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\n' | '\r' => plain.push('\n'),
            c if (c as u32) < 0x20 => {}
            c => plain.push(c),
        }
    }

    let mut lines: Vec<&str> = Vec::new();
    for line in plain.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if lines.last() == Some(&line) {
            continue;
        }
        lines.push(line);
        if lines.len() == 20 {
            break;
        }
    }
    lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn escape_sequences_are_dropped_and_plain_text_survives() {
        let raw =
            b"\x1b[2J\x1b[Hcould not enable raw mode: Os { code: 6 }\r\n\x1b]0;title\x07done\n";
        let text = legible(raw);
        assert_eq!(
            text, "  could not enable raw mode: Os { code: 6 }\n  done",
            "got: {text}"
        );
    }

    #[test]
    fn a_redrawing_dashboard_does_not_become_a_screen_dump() {
        let mut raw = Vec::new();
        for _ in 0..200 {
            raw.extend_from_slice(b"\x1b[2Jlisa \xe2\x80\xa2 2 running\n");
        }
        let text = legible(&raw);
        // The repeated row collapses to one, and the cap holds regardless.
        assert_eq!(text, "  lisa • 2 running");
        assert!(text.lines().count() <= 20);
    }

    #[test]
    fn nothing_printed_is_nothing_to_show() {
        let run = PtyRun {
            status: exited_ok(),
            interrupted: false,
            head: b"\x1b[2J\x1b[H".to_vec(),
        };
        assert!(run.transcript_head().is_none());
    }

    #[test]
    fn what_the_client_said_before_it_died_is_kept() {
        let run = PtyRun {
            status: exited_ok(),
            interrupted: false,
            head: b"thread 'main' panicked at src/main.rs:1\n".to_vec(),
        };
        assert_eq!(
            run.transcript_head().as_deref(),
            Some("  thread 'main' panicked at src/main.rs:1")
        );
    }

    /// A real `ExitStatus` without spawning anything interesting.
    fn exited_ok() -> std::process::ExitStatus {
        Command::new("true")
            .status()
            .or_else(|_| Command::new("/usr/bin/true").status())
            .expect("a `true` to exit")
    }

    /// The property the whole module exists for: a command that demands a
    /// terminal finds one, in a test process that has none.
    #[cfg(unix)]
    #[test]
    fn a_command_that_needs_a_terminal_gets_one() {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg("test -t 1 && tty");
        let run = run_on_pty(command).expect("a pty to open");
        assert!(
            run.status.success(),
            "stdout was not a terminal: {:?}",
            run.transcript_head()
        );
        let transcript = run.transcript_head().unwrap_or_default();
        assert!(
            transcript.contains("/dev/"),
            "the child should have named its terminal, got: {transcript}"
        );
        assert!(!run.interrupted);
    }

    /// The client's window is sized before it starts, because Zellij lays panes
    /// out against it and an unconfigured pty is 0×0.
    #[cfg(unix)]
    #[test]
    fn the_terminal_lisa_opens_has_a_plausible_size() {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg("stty size");
        let run = run_on_pty(command).expect("a pty to open");
        assert_eq!(
            run.transcript_head().as_deref(),
            Some(format!("  {HEADLESS_ROWS} {HEADLESS_COLS}").as_str())
        );
    }
}
