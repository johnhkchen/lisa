//! Native completion-seal environment resolution.
//!
//! Configuration expresses intent (`auto`, `commit`, or `journal`). A real
//! loop calls this module once, receives an immutable pinned tier, and passes
//! that fact into the plugin. No downstream caller re-probes the environment.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lisa_core::completion::{
    resolve_completion_seal, CommitSealSupport, CommitSealUnavailable, CompletionSeal,
    CompletionSealMode, ResolvedCompletionSeal,
};

const IDENTITY_REMEDY: &str =
    "  git config user.name \"You\"\n  git config user.email you@example.com";

/// Completion seal and repository fact pinned by one loop-start probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunCompletionSeal {
    resolution: ResolvedCompletionSeal,
    git_root: Option<PathBuf>,
}

impl RunCompletionSeal {
    pub(crate) fn seal(&self) -> CompletionSeal {
        self.resolution.seal()
    }

    pub(crate) fn git_root(&self) -> Option<&Path> {
        self.git_root.as_deref()
    }
}

/// Resolve the completion tier shown by read-only state inspection commands.
///
/// Explicit modes are already tiers. Only `auto` needs the environment probe;
/// if an unexpected resolution error occurs, inspection fails closed to the
/// historical commit tier instead of weakening the displayed contract.
pub(crate) fn resolve_for_inspection(
    project_root: &Path,
    mode: CompletionSealMode,
) -> CompletionSeal {
    if let Some(seal) = mode.explicit_seal() {
        return seal;
    }
    resolve_for_run(project_root, mode)
        .map(|resolved| resolved.seal())
        .unwrap_or(CompletionSeal::Commit)
}

/// Stable plain-language description shared by doctor and status.
pub(crate) const fn visibility_line(seal: CompletionSeal) -> &'static str {
    match seal {
        CompletionSeal::Commit => "completion seal: commit-sealed — finished work lands as history",
        CompletionSeal::Journal => {
            "completion seal: journal-only — finished work is recorded but not undoable"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommitProbeOutcome {
    support: CommitSealSupport,
    git_root: Option<PathBuf>,
}

#[derive(Debug)]
enum GitCommandFailure {
    Spawn(std::io::Error),
    Exit(Output),
}

/// Resolve and pin the completion seal for one real loop invocation.
pub(crate) fn resolve_for_run(
    project_root: &Path,
    mode: CompletionSealMode,
) -> Result<RunCompletionSeal, String> {
    resolve_for_run_with(project_root, mode, probe_commit_support)
}

fn resolve_for_run_with<F>(
    project_root: &Path,
    mode: CompletionSealMode,
    probe: F,
) -> Result<RunCompletionSeal, String>
where
    F: FnOnce(&Path) -> CommitProbeOutcome,
{
    // A deliberate journal stance is honored without touching Git. The dummy
    // support value is ignored by the pure resolver for this mode.
    let outcome = if mode == CompletionSealMode::Journal {
        CommitProbeOutcome {
            support: CommitSealSupport::Unavailable(CommitSealUnavailable::RepositoryMissing),
            git_root: None,
        }
    } else {
        probe(project_root)
    };

    let resolution = resolve_completion_seal(mode, outcome.support)
        .map_err(|error| format_preflight_failure(error.reason()))?;
    Ok(RunCompletionSeal {
        resolution,
        git_root: outcome.git_root,
    })
}

fn probe_commit_support(project_root: &Path) -> CommitProbeOutcome {
    let repository = match run_git(project_root, &["rev-parse", "--show-toplevel"]) {
        Ok(output) => output,
        Err(_) => {
            return CommitProbeOutcome {
                support: CommitSealSupport::Unavailable(CommitSealUnavailable::RepositoryMissing),
                git_root: None,
            };
        }
    };

    let raw_root = String::from_utf8_lossy(&repository.stdout)
        .trim()
        .to_string();
    let git_root = match nonempty_path(&raw_root).and_then(|path| path.canonicalize().ok()) {
        Some(root) => root,
        None => {
            return CommitProbeOutcome {
                support: CommitSealSupport::Unavailable(
                    CommitSealUnavailable::TransactionUnavailable {
                        detail: "Git returned a repository root that could not be resolved"
                            .to_string(),
                    },
                ),
                git_root: None,
            };
        }
    };

    let identity = run_git(&git_root, &["config", "--get", "user.email"]);
    if !matches!(identity, Ok(ref output) if !output.stdout.is_empty() && !String::from_utf8_lossy(&output.stdout).trim().is_empty())
    {
        return CommitProbeOutcome {
            support: CommitSealSupport::Unavailable(CommitSealUnavailable::IdentityMissing),
            git_root: Some(git_root),
        };
    }

    if let Err(output) = run_git(&git_root, &["rev-parse", "--verify", "HEAD"]) {
        return CommitProbeOutcome {
            support: CommitSealSupport::Unavailable(
                CommitSealUnavailable::TransactionUnavailable {
                    detail: format!(
                        "HEAD is not available to the isolated transaction ({})",
                        git_failure_detail(&output)
                    ),
                },
            ),
            git_root: Some(git_root),
        };
    }

    let git_dir = match run_git(&git_root, &["rev-parse", "--absolute-git-dir"]) {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(output) => {
            return CommitProbeOutcome {
                support: CommitSealSupport::Unavailable(
                    CommitSealUnavailable::TransactionUnavailable {
                        detail: format!(
                            "the repository metadata path is unavailable ({})",
                            git_failure_detail(&output)
                        ),
                    },
                ),
                git_root: Some(git_root),
            };
        }
    };
    if match nonempty_path(&git_dir) {
        Some(path) => !path.is_dir(),
        None => true,
    } {
        return CommitProbeOutcome {
            support: CommitSealSupport::Unavailable(
                CommitSealUnavailable::TransactionUnavailable {
                    detail: "the repository metadata path is missing".to_string(),
                },
            ),
            git_root: Some(git_root),
        };
    }

    CommitProbeOutcome {
        support: CommitSealSupport::Available,
        git_root: Some(git_root),
    }
}

fn run_git(root: &Path, args: &[&str]) -> Result<Output, GitCommandFailure> {
    match Command::new("git").arg("-C").arg(root).args(args).output() {
        Ok(output) if output.status.success() => Ok(output),
        Ok(output) => Err(GitCommandFailure::Exit(output)),
        Err(error) => Err(GitCommandFailure::Spawn(error)),
    }
}

fn nonempty_path(value: &str) -> Option<&Path> {
    if value.is_empty() {
        None
    } else {
        Some(Path::new(value))
    }
}

fn git_failure_detail(failure: &GitCommandFailure) -> String {
    match failure {
        GitCommandFailure::Spawn(error) => format!("Git could not be started: {error}"),
        GitCommandFailure::Exit(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                format!("Git exited with {}", output.status)
            } else {
                stderr
            }
        }
    }
}

fn format_preflight_failure(reason: &CommitSealUnavailable) -> String {
    format!(
        "Completion seal preflight failed: [guards].completion = \"commit\" requires commit sealing, but {reason}.\n\nConfigure the identity used for finished work:\n{IDENTITY_REMEDY}"
    )
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn outcome(support: CommitSealSupport) -> CommitProbeOutcome {
        CommitProbeOutcome {
            support,
            git_root: Some(PathBuf::from("/repo")),
        }
    }

    #[test]
    fn auto_probes_once_and_pins_the_strongest_satisfiable_tier() {
        let calls = Cell::new(0);
        let commit = resolve_for_run_with(Path::new("/project"), CompletionSealMode::Auto, |_| {
            calls.set(calls.get() + 1);
            outcome(CommitSealSupport::Available)
        })
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(commit.seal(), CompletionSeal::Commit);
        assert_eq!(commit.git_root(), Some(Path::new("/repo")));

        let journal = resolve_for_run_with(Path::new("/project"), CompletionSealMode::Auto, |_| {
            outcome(CommitSealSupport::Unavailable(
                CommitSealUnavailable::RepositoryMissing,
            ))
        })
        .unwrap();
        assert_eq!(journal.seal(), CompletionSeal::Journal);
        assert_eq!(
            journal.resolution.commit_unavailable(),
            Some(&CommitSealUnavailable::RepositoryMissing)
        );
    }

    #[test]
    fn explicit_journal_is_honored_without_invoking_git_probe() {
        let result =
            resolve_for_run_with(Path::new("/project"), CompletionSealMode::Journal, |_| {
                panic!("explicit journal must not probe Git")
            })
            .unwrap();

        assert_eq!(result.seal(), CompletionSeal::Journal);
        assert_eq!(result.git_root(), None);
        assert_eq!(result.resolution.commit_unavailable(), None);
    }

    #[test]
    fn explicit_commit_probes_once_and_missing_identity_has_two_line_remedy() {
        let calls = Cell::new(0);
        let error = resolve_for_run_with(Path::new("/project"), CompletionSealMode::Commit, |_| {
            calls.set(calls.get() + 1);
            outcome(CommitSealSupport::Unavailable(
                CommitSealUnavailable::IdentityMissing,
            ))
        })
        .unwrap_err();

        assert_eq!(calls.get(), 1);
        assert!(error.contains("Completion seal preflight failed"));
        assert!(error.contains("[guards].completion = \"commit\""));
        assert!(error.contains("git config user.name \"You\""));
        assert!(error.contains("git config user.email you@example.com"));
    }

    #[test]
    fn visibility_copy_is_exact_and_journal_copy_never_names_git() {
        assert_eq!(
            visibility_line(CompletionSeal::Commit),
            "completion seal: commit-sealed — finished work lands as history"
        );
        let journal = visibility_line(CompletionSeal::Journal);
        assert_eq!(
            journal,
            "completion seal: journal-only — finished work is recorded but not undoable"
        );
        assert!(!journal.to_ascii_lowercase().contains("git"));
    }

    #[test]
    fn inspection_uses_explicit_tiers_without_requiring_a_repository() {
        let missing = Path::new("/path/that/does/not/exist");
        assert_eq!(
            resolve_for_inspection(missing, CompletionSealMode::Commit),
            CompletionSeal::Commit
        );
        assert_eq!(
            resolve_for_inspection(missing, CompletionSealMode::Journal),
            CompletionSeal::Journal
        );
        assert_eq!(
            resolve_for_inspection(missing, CompletionSealMode::Auto),
            CompletionSeal::Journal
        );
    }
}
