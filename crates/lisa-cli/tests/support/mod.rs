//! Shared fixture for the concurrent-seal tests of S-055-01.
//!
//! Four commit transactions dispatched at once against one repository is the
//! situation the whole story is named for, and all three of its tickets need to
//! reproduce it. This module owns that setup so they reproduce the same one.
//!
//! `flock` is per open file description, so four threads in one process contend
//! for Lisa's commit guard exactly as four `lisa` processes would — while still
//! handing each caller its own `Result` to assert on.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Barrier;

use lisa_cli::commit_transaction::{
    CommitTransactionError, CommitTransactionResult, CompleteTicketRequest,
};
use lisa_core::completion::{AttemptId, CompletionGenerationId, CompletionId};
use tempfile::TempDir;

/// Error fragments that mean a transaction died on the guard instead of waiting
/// its turn. No path through a healthy transaction may produce any of them.
pub const GUARD_COLLISION_SIGNATURES: [&str; 3] = [
    "os error 35",
    "resource temporarily unavailable",
    "temporarily locked",
];

/// A throwaway Git repository seeded with N tickets that are ready to seal.
pub struct SealFixture {
    temp: TempDir,
    ticket_ids: Vec<String>,
}

impl SealFixture {
    /// Seed one reviewable ticket and one work artifact per id, take a base
    /// commit, then dirty every work artifact so each seal has a real diff.
    ///
    /// The dirtying matters: without it every transaction hits the empty-include
    /// diff error, which is a different ticket's subject.
    pub fn new(ticket_ids: &[&str]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let fixture = Self {
            temp,
            ticket_ids: ticket_ids.iter().map(|id| (*id).to_string()).collect(),
        };
        fixture.git(["init", "--quiet"]);
        fixture.git(["config", "user.name", "Lisa Test"]);
        fixture.git(["config", "user.email", "lisa@example.test"]);

        for id in &fixture.ticket_ids {
            fixture.write(
                &fixture.ticket_path(id),
                &format!(
                    "---\nid: {id}\ntitle: concurrent seal\ntype: task\nstatus: open\npriority: high\nphase: review\n---\n\nBody\n"
                ),
            );
            fixture.write(
                &format!("{}/research.md", fixture.work_dir(id)),
                &format!("# Research {id}\nbase\n"),
            );
        }
        fixture.git(["add", "-A"]);
        fixture.git(["commit", "--quiet", "-m", "base"]);

        for id in &fixture.ticket_ids {
            fixture.write(
                &format!("{}/review.md", fixture.work_dir(id)),
                &format!("# Review {id}\nready to seal\n"),
            );
        }
        fixture
    }

    pub fn root(&self) -> &Path {
        self.temp.path()
    }

    pub fn ticket_ids(&self) -> &[String] {
        &self.ticket_ids
    }

    pub fn ticket_path(&self, ticket_id: &str) -> String {
        format!("docs/active/tickets/{ticket_id}.md")
    }

    pub fn work_dir(&self, ticket_id: &str) -> String {
        format!("docs/active/work/{ticket_id}")
    }

    pub fn write(&self, path: &str, content: &str) {
        let path = self.root().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    pub fn git<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    pub fn git_string<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        String::from_utf8(self.git(args).stdout)
            .unwrap()
            .trim()
            .to_string()
    }

    /// The completion message this fixture seals `ticket_id` with.
    pub fn complete_message(&self, ticket_id: &str) -> String {
        format!("Complete {ticket_id}")
    }

    pub fn completion_key(&self, ticket_id: &str, generation: u64) -> CompletionGenerationId {
        CompletionGenerationId::new(
            CompletionId::new(ticket_id),
            AttemptId::new("1"),
            generation,
        )
    }

    pub fn complete_request(&self, ticket_id: &str, generation: u64) -> CompleteTicketRequest {
        CompleteTicketRequest {
            repo_root: self.root().to_path_buf(),
            ticket_id: ticket_id.to_string(),
            message: self.complete_message(ticket_id),
            ticket_file: PathBuf::from(self.ticket_path(ticket_id)),
            work_dir: PathBuf::from(self.work_dir(ticket_id)),
            completion_key: self.completion_key(ticket_id, generation),
        }
    }

    pub fn head_commit_count(&self) -> usize {
        self.git_string(["rev-list", "--count", "HEAD"])
            .parse()
            .unwrap()
    }

    pub fn commit_subjects(&self) -> Vec<String> {
        self.git_string(["log", "--format=%s"])
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// The blob at HEAD for a repository-relative path, or `None` when absent.
    pub fn show_at_head(&self, path: &str) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["show", &format!("HEAD:{path}")])
            .output()
            .unwrap();
        output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).unwrap())
    }

    pub fn assert_no_commit_lock(&self) {
        assert!(
            !self.root().join(".lisa-commit.lock").exists(),
            ".lisa-commit.lock remained after the transactions drained"
        );
    }
}

/// Release `count` threads simultaneously through a barrier and collect their
/// results in dispatch order.
///
/// The barrier is the point: staggered starts would not reproduce the collision
/// these tests exist to create.
pub fn dispatch_together<T, F>(count: usize, body: F) -> Vec<T>
where
    F: Fn(usize) -> T + Sync + Send,
    T: Send,
{
    let barrier = Barrier::new(count);
    let body = &body;
    let barrier = &barrier;
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..count)
            .map(|index| {
                scope.spawn(move || {
                    barrier.wait();
                    body(index)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok(value) => value,
                Err(panic) => std::panic::resume_unwind(panic),
            })
            .collect()
    })
}

/// No transaction may report a guard collision on any path.
pub fn assert_no_guard_collision(
    results: &[Result<CommitTransactionResult, CommitTransactionError>],
) {
    for (index, result) in results.iter().enumerate() {
        let Err(error) = result else { continue };
        let detail = error.to_string().to_ascii_lowercase();
        for signature in GUARD_COLLISION_SIGNATURES {
            assert!(
                !detail.contains(signature),
                "transaction {index} died on the guard ({signature}): {error}"
            );
        }
    }
}
