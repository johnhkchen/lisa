//! Typed sibling-temporary publication for scheduler-owned files.
//!
//! This module centralizes only the mechanism shared by the plugin's atomic
//! publication sites: resolve a temporary beside its destination, write a
//! complete payload, and replace the destination. Payload serialization,
//! directory creation, attempt authority, provenance append, and Git ref
//! transactions remain with their respective callers.

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// How one publication site names its same-directory temporary.
pub(crate) enum TemporaryName {
    /// `{prefix}{wall-clock nanosecond nonce}`.
    Nonce { prefix: String },
    /// `{prefix}{attempt_id}-{wall-clock nanosecond nonce}`.
    AttemptNonce { prefix: String, attempt_id: u64 },
    /// One deterministic sibling filename.
    Exact { file_name: String },
}

impl TemporaryName {
    fn resolve(self) -> Result<String, String> {
        let file_name = match self {
            Self::Nonce { prefix } => format!("{prefix}{}", publication_nonce()),
            Self::AttemptNonce { prefix, attempt_id } => {
                format!("{prefix}{attempt_id}-{}", publication_nonce())
            }
            Self::Exact { file_name } => file_name,
        };
        let mut components = Path::new(&file_name).components();
        if matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none() {
            Ok(file_name)
        } else {
            Err(format!(
                "invalid publication temporary name {file_name:?}: expected one filename component"
            ))
        }
    }
}

/// Destination plus the typed naming policy for its sibling temporary.
pub(crate) struct PublicationPath {
    pub(crate) destination: PathBuf,
    pub(crate) temporary_name: TemporaryName,
}

struct ResolvedPublicationPath {
    destination: PathBuf,
    temporary: PathBuf,
}

impl PublicationPath {
    fn resolve(self) -> Result<ResolvedPublicationPath, String> {
        let temporary = self
            .destination
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(self.temporary_name.resolve()?);
        Ok(ResolvedPublicationPath {
            destination: self.destination,
            temporary,
        })
    }
}

/// Site-specific operator labels for the two fallible publication operations.
pub(crate) struct PublicationErrors {
    pub(crate) write: &'static str,
    pub(crate) publish: &'static str,
}

/// A complete Rust-side sibling-temp publication request.
pub(crate) struct RustPublication<'a> {
    pub(crate) path: PublicationPath,
    pub(crate) body: &'a [u8],
    pub(crate) errors: PublicationErrors,
}

impl RustPublication<'_> {
    /// Write complete bytes and atomically replace the destination by rename.
    pub(crate) fn publish(self) -> Result<PathBuf, String> {
        let resolved = self.path.resolve()?;
        std::fs::write(&resolved.temporary, self.body).map_err(|error| {
            format!(
                "{} {}: {error}",
                self.errors.write,
                resolved.temporary.display()
            )
        })?;
        if let Err(error) = std::fs::rename(&resolved.temporary, &resolved.destination) {
            let _ = std::fs::remove_file(&resolved.temporary);
            return Err(format!(
                "{} {}: {error}",
                self.errors.publish,
                resolved.destination.display()
            ));
        }
        Ok(resolved.destination)
    }
}

// There is deliberately no shell-side counterpart to [`RustPublication`].
//
// One existed: it rendered `printf … > tmp && mv tmp <signal>` for Lisa to type
// into a pane, and the startup-recovery probe was its only caller. What it asked
// for was a signal file written by whoever was reading the pane's keystrokes —
// which against a live agent is either a forged readiness proof or, from an
// agent correctly taught never to write under `.lisa/`, a refusal (S-067-01).
// Every publication Lisa depends on is now performed by Lisa or by a hook, in
// this process, through `RustPublication`.

/// Encode one arbitrary UTF-8 value as one POSIX shell argument.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn publication_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn publish_exact(
        destination: PathBuf,
        temporary: &str,
        body: &[u8],
    ) -> Result<PathBuf, String> {
        RustPublication {
            path: PublicationPath {
                destination,
                temporary_name: TemporaryName::Exact {
                    file_name: temporary.to_string(),
                },
            },
            body,
            errors: PublicationErrors {
                write: "cannot write direct publication",
                publish: "cannot publish direct publication",
            },
        }
        .publish()
    }

    #[test]
    fn repeated_publication_replaces_complete_bytes_without_duplicate_or_residue() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("atomic path ' ; $() `x`");
        fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("artifact.md");
        fs::write(&destination, b"old complete bytes\n").unwrap();

        publish_exact(
            destination.clone(),
            ".artifact.md.attempt-7.tmp",
            b"first complete bytes\n",
        )
        .unwrap();
        publish_exact(
            destination.clone(),
            ".artifact.md.attempt-7.tmp",
            b"second complete bytes\n",
        )
        .unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"second complete bytes\n");
        assert!(!directory.join(".artifact.md.attempt-7.tmp").exists());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    }

    #[test]
    fn failed_publication_preserves_destination_and_removes_complete_temporary() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("failed atomic publication");
        fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("artifact.md");
        fs::create_dir(&destination).unwrap();
        let sentinel = destination.join("prior-ticket-bytes");
        fs::write(&sentinel, b"prior bytes remain intact\n").unwrap();
        let temporary = directory.join(".artifact.md.attempt-8.tmp");

        let error = publish_exact(
            destination.clone(),
            ".artifact.md.attempt-8.tmp",
            b"new body must not become visible\n",
        )
        .unwrap_err();

        assert!(error.starts_with("cannot publish direct publication "));
        assert_eq!(fs::read(&sentinel).unwrap(), b"prior bytes remain intact\n");
        assert!(
            !temporary.exists(),
            "rename failure must remove the complete temp"
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    }

    #[test]
    fn hostile_temporary_paths_are_rejected_before_cross_ticket_mixing() {
        let temp = tempfile::tempdir().unwrap();
        let work = temp.path().join("canonical work");
        let ticket_a = work.join("T-A");
        let ticket_b = work.join("T-B");
        fs::create_dir_all(&ticket_a).unwrap();
        fs::create_dir_all(&ticket_b).unwrap();
        let artifact_a = ticket_a.join("research.md");
        let artifact_b = ticket_b.join("research.md");
        fs::write(&artifact_a, b"ticket A prior bytes\n").unwrap();
        fs::write(&artifact_b, b"ticket B prior bytes\n").unwrap();
        let secret_body = b"ticket A unpublished secret body\n";

        let error =
            publish_exact(artifact_a.clone(), "../T-B/research.md", secret_body).unwrap_err();

        assert!(error.starts_with("invalid publication temporary name "));
        assert!(!error.contains("unpublished secret"));
        assert_eq!(fs::read(&artifact_a).unwrap(), b"ticket A prior bytes\n");
        assert_eq!(fs::read(&artifact_b).unwrap(), b"ticket B prior bytes\n");
        assert_eq!(fs::read_dir(&ticket_a).unwrap().count(), 1);
        assert_eq!(fs::read_dir(&ticket_b).unwrap().count(), 1);
    }

    #[test]
    fn every_temporary_policy_rejects_non_sibling_components() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let absolute_prefix = temp.path().join("escaped-").to_string_lossy().into_owned();
        let policies = [
            TemporaryName::Exact {
                file_name: "../escaped.tmp".to_string(),
            },
            TemporaryName::Nonce {
                prefix: "nested/escaped.tmp.".to_string(),
            },
            TemporaryName::AttemptNonce {
                prefix: absolute_prefix,
                attempt_id: 9,
            },
        ];

        for policy in policies {
            let error = PublicationPath {
                destination: destination.clone(),
                temporary_name: policy,
            }
            .resolve()
            .err()
            .expect("non-sibling policy must be rejected");
            assert!(error.starts_with("invalid publication temporary name "));
        }
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    /// The structural half of removing the readiness probe (T-067-01-01).
    ///
    /// A pane-typed publication is the only mechanism Lisa ever had for asking
    /// the thing reading a pane's keystrokes to create a signal file, so the
    /// guarantee "no recovery path asks the pane's occupant to write a lisa
    /// signal" is kept by this module having no way to render one.
    #[test]
    fn no_publication_can_be_rendered_for_something_else_to_execute() {
        let source = include_str!("publication.rs");
        let production = source
            .split_once("#[cfg(test)]\nmod tests {")
            .expect("test module marker remains available")
            .0;

        assert!(!production.contains("struct ShellPublication"));
        assert!(
            !production.contains("command mv"),
            "no publication may be rendered as a shell command for a pane to run"
        );
    }
}
