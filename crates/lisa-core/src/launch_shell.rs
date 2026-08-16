//! What kind of shell a run was started from, as three plain facts.
//!
//! A run started from a terminal on the machine and a run started over `ssh`
//! are not the same run, and the difference is invisible once the run is going.
//! A GUI session can open the login keychain; an ssh session cannot, and gets
//! `User interaction is not allowed` — which is how `gh` came to report *"the
//! token in default is invalid"* about a token that was fine. A GUI session
//! usually has an `ssh-agent`; an ssh session usually has none, so a
//! passphrase-protected key is unusable however well it is installed at the far
//! end.
//!
//! [`crate::session_env`] already reasons about what a pane's environment
//! carries, and drops the markers that must not travel down. This is the same
//! reasoning applied to the thing *above* the pane: not what the run passes on,
//! but what it was handed.
//!
//! ## Three booleans, not a category
//!
//! Deliberately not a guess at "GUI" or "remote". Those are conclusions, and
//! the conclusions differ per machine and per tool. What is recorded is what
//! was measured:
//!
//! - `SSH_CONNECTION` was set,
//! - `SSH_AUTH_SOCK` was set,
//! - a terminal was attached.
//!
//! What they imply is the operator's business, and
//! [`crate::schedulers::SchedulerRecord`] is where they are kept so that a
//! question asked hours later — *why did that run fail to push when the
//! identical one yesterday did not* — has something to read.
//!
//! ## No values, ever
//!
//! Only presence. `SSH_AUTH_SOCK`'s value is a path to a live socket and
//! `SSH_CONNECTION`'s is a pair of addresses and ports; both end up in a record
//! that gets read aloud, pasted into a ticket, and committed. The fact that an
//! agent socket was there is what a reader needs; where it was is not.

use serde::{Deserialize, Serialize};

/// The name whose presence means this shell came in over `ssh`.
pub const SSH_CONNECTION: &str = "SSH_CONNECTION";

/// The name whose presence means this shell can reach an `ssh-agent`.
pub const SSH_AUTH_SOCK: &str = "SSH_AUTH_SOCK";

/// How a run's shell was arranged, recorded rather than interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchShell {
    /// `SSH_CONNECTION` was set: this shell arrived over the network.
    pub ssh_connection: bool,
    /// `SSH_AUTH_SOCK` was set: an agent was reachable for key material.
    pub ssh_agent: bool,
    /// A terminal was attached to the shell that started the run.
    pub tty: bool,
}

impl LaunchShell {
    /// Observe this process's own environment.
    ///
    /// `tty` is passed in rather than probed here because the caller is the one
    /// that knows which handle matters — `lisa loop` asks about the stdin it is
    /// about to hand Zellij, and a headless run opens a terminal of its own
    /// afterwards. What is recorded is the shell the run was *started from*,
    /// which is the thing that differs between a desk and an overnight `ssh`.
    pub fn observe(tty: bool) -> Self {
        Self::from_lookup(|name| std::env::var(name).ok(), tty)
    }

    /// The same observation against a stated environment, so the two shells
    /// this exists to tell apart can both be exercised on one machine.
    ///
    /// A variable set to the empty string counts as absent: an exported-but-
    /// empty `SSH_AUTH_SOCK` reaches no agent, and recording it as one would be
    /// the false green light this whole record is meant to avoid.
    pub fn from_lookup<F>(get: F, tty: bool) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let present = |name: &str| get(name).is_some_and(|value| !value.trim().is_empty());
        Self {
            ssh_connection: present(SSH_CONNECTION),
            ssh_agent: present(SSH_AUTH_SOCK),
            tty,
        }
    }

    /// The three facts as one config-map word, carrying no value of anything.
    ///
    /// `lisa loop` observes the shell on the host and the plugin writes the
    /// record from inside the Zellij server, so the observation has to survive
    /// the layout between them. One flat token rather than three keys, because
    /// three keys can arrive two-thirds present and there is no honest way to
    /// read that.
    pub fn encode(&self) -> String {
        format!(
            "ssh={},agent={},tty={}",
            yes_no(self.ssh_connection),
            yes_no(self.ssh_agent),
            yes_no(self.tty),
        )
    }

    /// Read back exactly what [`LaunchShell::encode`] wrote, or nothing.
    ///
    /// Strict on purpose. A half-understood token would become a record that
    /// says *no agent* when nobody looked, and a record that says no is
    /// supposed to mean somebody measured no.
    pub fn parse(text: &str) -> Option<Self> {
        let mut ssh_connection = None;
        let mut ssh_agent = None;
        let mut tty = None;
        for field in text.split(',') {
            let (key, value) = field.trim().split_once('=')?;
            let value = match value.trim() {
                "yes" => true,
                "no" => false,
                _ => return None,
            };
            let slot = match key.trim() {
                "ssh" => &mut ssh_connection,
                "agent" => &mut ssh_agent,
                "tty" => &mut tty,
                _ => return None,
            };
            if slot.replace(value).is_some() {
                return None;
            }
        }
        Some(Self {
            ssh_connection: ssh_connection?,
            ssh_agent: ssh_agent?,
            tty: tty?,
        })
    }

    /// The one line an operator reads, with all three facts in it.
    pub fn describe(&self) -> String {
        format!(
            "{}, {}, {}",
            if self.ssh_connection {
                "over ssh"
            } else {
                "not over ssh"
            },
            if self.ssh_agent {
                "with an ssh-agent"
            } else {
                "with no ssh-agent"
            },
            if self.tty {
                "with a terminal"
            } else {
                "with no terminal"
            },
        )
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "yes"
    } else {
        "no"
    }
}

/// What a reader says about a record that carries no observation at all.
///
/// A record written before Lisa observed this, and a record that observed a
/// shell with no agent in it, are different statements and must not read the
/// same. This is the sentence for the first one.
pub const NOT_RECORDED: &str = "not recorded (an older Lisa started this run)";

#[cfg(test)]
mod tests {
    use super::*;

    /// The desk's two shells, side by side: the same board started from a GUI
    /// terminal and over `ssh` records two different things.
    #[test]
    fn a_gui_shell_and_an_ssh_shell_are_recorded_differently() {
        let gui = LaunchShell::from_lookup(
            |name| match name {
                SSH_AUTH_SOCK => Some("/private/tmp/com.apple.launchd.7Qh/Listeners".to_string()),
                _ => None,
            },
            true,
        );
        let remote = LaunchShell::from_lookup(
            |name| match name {
                SSH_CONNECTION => Some("192.168.1.44 51988 192.168.1.9 22".to_string()),
                _ => None,
            },
            false,
        );

        assert_eq!(
            gui,
            LaunchShell {
                ssh_connection: false,
                ssh_agent: true,
                tty: true
            }
        );
        assert_eq!(
            remote,
            LaunchShell {
                ssh_connection: true,
                ssh_agent: false,
                tty: false
            }
        );
        assert_ne!(gui, remote);
        assert_eq!(
            gui.describe(),
            "not over ssh, with an ssh-agent, with a terminal"
        );
        assert_eq!(
            remote.describe(),
            "over ssh, with no ssh-agent, with no terminal"
        );
    }

    /// An exported-but-empty variable reaches nothing, and a record that called
    /// it an agent would be the false green light this exists to avoid.
    #[test]
    fn an_empty_variable_is_an_absent_one() {
        let observed = LaunchShell::from_lookup(
            |name| match name {
                SSH_AUTH_SOCK => Some(String::new()),
                SSH_CONNECTION => Some("   ".to_string()),
                _ => None,
            },
            true,
        );
        assert!(!observed.ssh_agent);
        assert!(!observed.ssh_connection);
    }

    /// The whole reason presence is recorded and values are not: this token is
    /// committed, printed, and pasted into tickets.
    #[test]
    fn nothing_encoded_carries_the_value_of_anything() {
        let socket = "/private/tmp/com.apple.launchd.7Qh/Listeners";
        let connection = "192.168.1.44 51988 192.168.1.9 22";
        let observed = LaunchShell::from_lookup(
            |name| match name {
                SSH_AUTH_SOCK => Some(socket.to_string()),
                SSH_CONNECTION => Some(connection.to_string()),
                _ => None,
            },
            true,
        );

        let encoded = observed.encode();
        let json = serde_json::to_string(&observed).unwrap();
        for carrier in [encoded.as_str(), json.as_str(), &observed.describe()] {
            assert!(!carrier.contains(socket), "leaked the socket: {carrier}");
            assert!(
                !carrier.contains("192.168"),
                "leaked the address: {carrier}"
            );
        }
        assert_eq!(encoded, "ssh=yes,agent=yes,tty=yes");
    }

    #[test]
    fn every_shape_survives_the_layout_between_the_loop_and_the_plugin() {
        for ssh_connection in [false, true] {
            for ssh_agent in [false, true] {
                for tty in [false, true] {
                    let observed = LaunchShell {
                        ssh_connection,
                        ssh_agent,
                        tty,
                    };
                    assert_eq!(LaunchShell::parse(&observed.encode()), Some(observed));
                }
            }
        }
    }

    /// A token that is not exactly what Lisa wrote says nothing, rather than
    /// two-thirds of something.
    #[test]
    fn a_half_understood_token_is_read_as_no_observation_at_all() {
        for text in [
            "",
            "ssh=yes",
            "ssh=yes,agent=no",
            "ssh=yes,agent=no,tty=maybe",
            "ssh=yes,agent=no,tty=yes,extra=no",
            "ssh=yes,ssh=no,tty=yes",
            "yes,no,yes",
        ] {
            assert_eq!(
                LaunchShell::parse(text),
                None,
                "{text:?} must not read as an observation"
            );
        }
    }

    /// Whitespace a KDL value picked up on the way through is not a reason to
    /// throw the observation away.
    #[test]
    fn a_token_with_spaces_around_it_still_reads() {
        assert_eq!(
            LaunchShell::parse(" ssh=no , agent=yes , tty=yes "),
            Some(LaunchShell {
                ssh_connection: false,
                ssh_agent: true,
                tty: true
            })
        );
    }
}
