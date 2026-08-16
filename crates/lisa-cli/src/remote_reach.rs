//! Whether work made on this board can reach the remote it would land on.
//!
//! A run's whole output is commits. Lisa checks the binary, the runtime, the
//! agent, the seal and the pane routes, and used to check everything about a
//! run except the road its commits leave by. This is that check.
//!
//! Three things it is careful about, because each one is a way to be green and
//! wrong:
//!
//! 1. **It measures push, not fetch.** `git ls-remote` is one round trip and
//!    proves only that this shell can *read* the remote. A fine-grained token
//!    or a read-only deploy key reads and cannot write, and that is exactly the
//!    configuration a careful person lands. So the probe is
//!    `git push --dry-run`, and the word every outcome uses is *push*.
//! 2. **`cannot tell` is not `no`.** A refused credential and an unplugged
//!    network are different answers, and a board with no remote at all is fine
//!    — plenty of work is local and must not be flagged.
//! 3. **It says which shell it measured.** `lisa doctor` usually runs in the
//!    operator's terminal, with an ssh agent and an unlocked keychain. The pane
//!    an overnight run spawns may have neither, and this probe cannot get
//!    inside that pane. It answers for *here*, and says so rather than implying
//!    otherwise.
//!
//! Nothing here writes: `--dry-run` sends no objects and creates no branch,
//! `--no-verify` keeps a local pre-push hook from running, and Lisa never
//! stores, repairs or alters a credential.

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How long the one round trip is given before the answer becomes `cannot
/// tell`. Doctor is read by a person waiting at a terminal; a remote that has
/// not answered in ten seconds has not answered.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the probe is checked for having finished.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// The most stderr an operator should have to read back from git.
const MAX_DETAIL: usize = 400;

/// The road a remote goes over. Named in every outcome because on this desk
/// the two common ones have opposite answers per machine: a box that can push
/// over ssh may not be able to push over https, and the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protocol {
    Ssh,
    Https,
    Http,
    Git,
    Local,
    Unfamiliar,
}

impl Protocol {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Protocol::Ssh => "ssh",
            Protocol::Https => "https",
            Protocol::Http => "http",
            Protocol::Git => "the git protocol",
            Protocol::Local => "a local path",
            Protocol::Unfamiliar => "an unfamiliar transport",
        }
    }
}

/// Which road a remote URL takes.
pub(crate) fn classify_protocol(url: &str) -> Protocol {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("ssh://") {
        return Protocol::Ssh;
    }
    if lower.starts_with("https://") {
        return Protocol::Https;
    }
    if lower.starts_with("http://") {
        return Protocol::Http;
    }
    if lower.starts_with("git://") {
        return Protocol::Git;
    }
    if lower.starts_with("file://") || url.starts_with('/') || url.starts_with('.') {
        return Protocol::Local;
    }
    if lower.contains("://") {
        return Protocol::Unfamiliar;
    }
    // `git@github.com:owner/repo.git` — scp-like, and still ssh. Only a colon
    // that comes before any slash makes it one; `../sibling/repo` does not.
    match (url.find(':'), url.find('/')) {
        (Some(colon), Some(slash)) if colon < slash => Protocol::Ssh,
        (Some(_), None) => Protocol::Ssh,
        _ => Protocol::Local,
    }
}

/// A URL as it is safe to print. An https remote can carry a token in its
/// userinfo, and doctor's output gets pasted into issues.
pub(crate) fn redact(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let rest = &url[scheme_end + 3..];
    let host_end = rest.find('/').unwrap_or(rest.len());
    let Some(at) = rest[..host_end].rfind('@') else {
        return url.to_string();
    };
    format!("{}://***@{}", &url[..scheme_end], &rest[at + 1..])
}

/// What the probe found. `Refused` and `Unclear` are separate on purpose: a
/// credential that cannot push is a thing to fix, and a network that did not
/// answer is a thing to retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reach {
    /// No repository here at all — nothing to push, nothing to check.
    NoRepository,
    /// A repository with nowhere to push, which is a fine way to work.
    NoRemote { detail: String },
    /// A dry-run push was accepted.
    Reachable {
        protocol: Protocol,
        remote: String,
        url: String,
        branch: String,
    },
    /// A dry-run push was answered, and the answer was no.
    Refused {
        protocol: Protocol,
        remote: String,
        url: String,
        branch: String,
        detail: String,
    },
    /// No answer was obtained, so nothing is claimed either way.
    Unclear {
        protocol: Option<Protocol>,
        detail: String,
    },
}

impl Reach {
    /// The line under the row: what was measured, and what came back.
    pub(crate) fn detail(&self) -> String {
        match self {
            Reach::NoRepository => {
                "no git repository here, so there is nothing to push".to_string()
            }
            Reach::NoRemote { detail } => detail.clone(),
            Reach::Reachable {
                protocol,
                remote,
                url,
                branch,
            } => format!(
                "a dry-run push of {branch} to {remote} ({}) over {} was accepted",
                redact(url),
                protocol.label()
            ),
            Reach::Refused {
                protocol,
                remote,
                url,
                branch,
                detail,
            } => format!(
                "a dry-run push of {branch} to {remote} ({}) over {} was refused: {detail}",
                redact(url),
                protocol.label()
            ),
            Reach::Unclear { protocol, detail } => match protocol {
                Some(protocol) => format!(
                    "could not tell whether a push over {} would land: {detail}",
                    protocol.label()
                ),
                None => format!("could not tell whether a push would land: {detail}"),
            },
        }
    }

    /// What a person can do about it, when there is something to do.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Reach::Refused { protocol, .. } => Some(format!(
                "Give this shell a credential that can push over {}, or point the remote at the road that works on this machine (`git remote set-url`). Lisa never installs, holds or repairs a credential — it only reports what this shell can already do. A run is not blocked by this: work still commits locally and waits.",
                protocol.label()
            )),
            _ => None,
        }
    }
}

/// Ask, once, whether a commit made here could reach its remote.
pub(crate) fn look(root: &Path) -> Reach {
    if git_output(root, &["rev-parse", "--show-toplevel"]).is_none() {
        return Reach::NoRepository;
    }

    let configured = git_output(root, &["remote"]).unwrap_or_default();
    if configured.trim().is_empty() {
        return Reach::NoRemote {
            detail: "no remote is configured, so work stays here — which is a fine way to run a \
                     board"
                .to_string(),
        };
    }

    let Some(branch) = git_output(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]) else {
        return Reach::Unclear {
            protocol: None,
            detail: "HEAD is detached here, so there is no branch to measure a push for"
                .to_string(),
        };
    };

    let remote = push_remote(root, &branch);
    let Some(url) = git_output(root, &["remote", "get-url", "--push", &remote]) else {
        return Reach::Unclear {
            protocol: None,
            detail: format!("{branch} would push to `{remote}`, and no URL is configured for it"),
        };
    };
    let protocol = classify_protocol(&url);

    match dry_run_push(root, &remote, &branch, protocol) {
        Probe::Accepted => Reach::Reachable {
            protocol,
            remote,
            url,
            branch,
        },
        Probe::Answered(stderr) => match read_refusal(&stderr) {
            Answer::Refused(detail) => Reach::Refused {
                protocol,
                remote,
                url,
                branch,
                detail,
            },
            Answer::Unclear(detail) => Reach::Unclear {
                protocol: Some(protocol),
                detail,
            },
        },
        Probe::NoAnswer(detail) => Reach::Unclear {
            protocol: Some(protocol),
            detail,
        },
    }
}

/// Where this branch would actually push, by git's own precedence.
fn push_remote(root: &Path, branch: &str) -> String {
    let keys = [
        format!("branch.{branch}.pushRemote"),
        "remote.pushDefault".to_string(),
        format!("branch.{branch}.remote"),
    ];
    for key in keys {
        if let Some(value) = git_output(root, &["config", "--get", &key]) {
            return value;
        }
    }
    "origin".to_string()
}

enum Probe {
    Accepted,
    /// git reached a verdict; the text is what it said.
    Answered(String),
    /// git never got to a verdict — it could not start, or never came back.
    NoAnswer(String),
}

/// One round trip: everything a push does except send anything.
fn dry_run_push(root: &Path, remote: &str, branch: &str, protocol: Protocol) -> Probe {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args([
            "push",
            "--dry-run",
            // A local pre-push hook can build, write and take minutes. Doctor
            // measures the road, not the hook.
            "--no-verify",
            remote,
            &format!("HEAD:refs/heads/{branch}"),
        ])
        // Nothing here may stop and ask. A prompt no one can see is exactly
        // what an unattended pane would hit, so the probe refuses one too:
        // no terminal prompt, no askpass dialog, and no stdin to read from.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if protocol == Protocol::Ssh {
        // ssh reads a passphrase from /dev/tty, not stdin, so closing stdin is
        // not enough to keep it from hanging. BatchMode is added to whatever
        // ssh command this repository already uses rather than replacing it.
        command.env("GIT_SSH_COMMAND", batch_ssh_command(root));
    }

    match run_with_timeout(&mut command, PROBE_TIMEOUT) {
        Ok(output) if output.status.success() => Probe::Accepted,
        Ok(output) => Probe::Answered(String::from_utf8_lossy(&output.stderr).into_owned()),
        Err(detail) => Probe::NoAnswer(detail),
    }
}

fn batch_ssh_command(root: &Path) -> String {
    let base = std::env::var("GIT_SSH_COMMAND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_output(root, &["config", "--get", "core.sshCommand"]))
        .unwrap_or_else(|| "ssh".to_string());
    format!("{base} -o BatchMode=yes")
}

/// Run a command, and give up on it after `timeout`.
fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output, String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("git could not be started ({error})"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| format!("git's answer could not be read ({error})"))
            }
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "the remote did not answer within {} seconds",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Err(format!("git could not be waited on ({error})")),
        }
    }
}

/// A verdict git reached, read as either a no or a shrug.
enum Answer {
    Refused(String),
    Unclear(String),
}

/// Phrases that mean the road answered and the answer was no: a credential
/// that cannot push, or none this shell could offer.
const REFUSAL_PHRASES: &[&str] = &[
    "permission denied",
    "permission to",
    "access denied",
    "authentication failed",
    "not authorized",
    "unauthorized",
    "requested url returned error: 401",
    "requested url returned error: 403",
    "could not read username",
    "could not read password",
    "terminal prompts disabled",
    "repository not found",
    "does not appear to be a git repository",
    "read-only",
    "write access",
    "host key verification failed",
    "invalid username or token",
];

/// Phrases that mean no verdict was reached: the road was not there, not the
/// credential. These are `cannot tell`, never `no`.
const UNCLEAR_PHRASES: &[&str] = &[
    "could not resolve host",
    "name resolution",
    "connection refused",
    "connection timed out",
    "operation timed out",
    "timed out",
    "network is unreachable",
    "no route to host",
    "failed to connect",
    "connection reset",
    "connection closed",
    "certificate verification failed",
    "ssl",
    "proxy",
    "src refspec",
    "unable to look up",
];

fn read_refusal(stderr: &str) -> Answer {
    // Read the whole of what git said, not the shortened version an operator
    // gets to read: the sentence that decides this — `Host key verification
    // failed.`, `Permission denied (publickey).` — is often not the last one.
    let summary = summarize(stderr);
    let lower = stderr.to_ascii_lowercase();
    if REFUSAL_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        Answer::Refused(summary)
    } else if UNCLEAR_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        Answer::Unclear(summary)
    } else {
        // Anything unrecognised is a shrug, never a no. Reporting a refusal
        // Lisa did not actually measure is the failure mode this whole check
        // exists to avoid.
        Answer::Unclear(format!(
            "git did not finish the push and Lisa does not recognise why — {summary}"
        ))
    }
}

/// Progress chatter, which says nothing about whether the push could land.
const NOISE: &[&str] = &[
    "warning: permanently added",
    "enumerating objects",
    "counting objects",
    "compressing objects",
    "writing objects",
    "total ",
];

/// What git said, as much of it as is worth reading back.
///
/// Git's own words are kept rather than paraphrased — the operator is the one
/// who has to recognise their own machine in them — with the progress lines
/// dropped and a cap so one bad remote cannot take over the report.
fn summarize(stderr: &str) -> String {
    let kept: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !NOISE.iter().any(|noise| lower.contains(noise))
        })
        .collect();
    let chosen = if kept.is_empty() {
        "git said nothing".to_string()
    } else {
        kept.join(" ")
    };
    let chosen = chosen.split_whitespace().collect::<Vec<_>>().join(" ");
    if chosen.chars().count() > MAX_DETAIL {
        let truncated: String = chosen.chars().take(MAX_DETAIL).collect();
        format!("{truncated}...")
    } else {
        chosen
    }
}

/// Run a read-only git command and return its trimmed stdout, or `None` when
/// it failed or said nothing.
fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_url_names_the_road_it_takes() {
        assert_eq!(
            classify_protocol("git@github.com:johnhkchen/lisa.git"),
            Protocol::Ssh
        );
        assert_eq!(
            classify_protocol("ssh://git@github.com/johnhkchen/lisa.git"),
            Protocol::Ssh
        );
        assert_eq!(
            classify_protocol("https://github.com/johnhkchen/lisa.git"),
            Protocol::Https
        );
        assert_eq!(classify_protocol("http://127.0.0.1:9/repo"), Protocol::Http);
        assert_eq!(classify_protocol("git://example.com/repo"), Protocol::Git);
        assert_eq!(classify_protocol("/srv/git/lisa.git"), Protocol::Local);
        assert_eq!(classify_protocol("../sibling/lisa.git"), Protocol::Local);
        assert_eq!(
            classify_protocol("file:///srv/git/lisa.git"),
            Protocol::Local
        );
    }

    #[test]
    fn a_token_in_a_url_is_never_printed_back() {
        assert_eq!(
            redact("https://johnhkchen:ghp_secret@github.com/johnhkchen/lisa.git"),
            "https://***@github.com/johnhkchen/lisa.git"
        );
        assert_eq!(
            redact("https://github.com/johnhkchen/lisa.git"),
            "https://github.com/johnhkchen/lisa.git"
        );
        assert_eq!(
            redact("git@github.com:johnhkchen/lisa.git"),
            "git@github.com:johnhkchen/lisa.git"
        );
    }

    fn refusal(stderr: &str) -> String {
        match read_refusal(stderr) {
            Answer::Refused(detail) => detail,
            Answer::Unclear(detail) => panic!("expected a refusal, got `cannot tell`: {detail}"),
        }
    }

    fn unclear(stderr: &str) -> String {
        match read_refusal(stderr) {
            Answer::Unclear(detail) => detail,
            Answer::Refused(detail) => panic!("expected `cannot tell`, got a refusal: {detail}"),
        }
    }

    #[test]
    fn a_credential_that_cannot_push_is_a_no() {
        let detail = refusal(
            "remote: Permission to johnhkchen/lisa.git denied to deploy-key.\n\
             fatal: unable to access 'https://github.com/johnhkchen/lisa.git/': The requested URL returned error: 403\n",
        );
        assert!(detail.contains("denied"), "{detail}");

        refusal("git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.\n");
        refusal(
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled\n",
        );
        refusal("remote: Repository not found.\nfatal: repository 'https://github.com/x/y.git/' not found\n");
        refusal("Host key verification failed.\nfatal: Could not read from remote repository.\n");
    }

    #[test]
    fn a_road_that_was_never_there_is_cannot_tell() {
        let detail = unclear(
            "fatal: unable to access 'https://github.com/johnhkchen/lisa.git/': Could not resolve host: github.com\n",
        );
        assert!(detail.contains("Could not resolve host"), "{detail}");

        unclear("ssh: connect to host github.com port 22: Operation timed out\nfatal: Could not read from remote repository.\n");
        unclear("fatal: unable to access 'http://127.0.0.1:9/repo/': Failed to connect to 127.0.0.1 port 9: Connection refused\n");
    }

    #[test]
    fn an_answer_lisa_cannot_read_is_never_reported_as_a_no() {
        let detail = unclear("fatal: something entirely new happened\n");
        assert!(
            detail.contains("does not recognise"),
            "an unrecognised answer has to say so: {detail}"
        );
    }

    #[test]
    fn a_board_with_no_remote_is_not_flagged() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "--quiet"])
            .status()
            .unwrap()
            .success());

        match look(dir.path()) {
            Reach::NoRemote { detail } => assert!(detail.contains("fine"), "{detail}"),
            other => panic!("a board with no remote is fine, not {other:?}"),
        }
    }

    #[test]
    fn a_folder_that_is_not_a_repository_has_nothing_to_check() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(look(dir.path()), Reach::NoRepository);
    }

    #[test]
    #[cfg(unix)]
    fn the_probe_gives_up_rather_than_hanging() {
        let mut command = Command::new("sleep");
        command.arg("30");
        let detail = run_with_timeout(&mut command, Duration::from_millis(200))
            .expect_err("a command that outlives its budget is not an answer");
        assert!(detail.contains("did not answer"), "{detail}");
    }
}
