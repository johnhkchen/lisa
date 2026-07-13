//! Exact, atomically published assignment-file references.
//!
//! Assignment identity is deliberately separate from lease authority: the
//! lease fences one ticket attempt, while the nonce distinguishes the exact
//! immutable assignment file a launcher and later claim refer to.

use std::path::{Path, PathBuf};

use lisa_core::types::AttemptLease;

use crate::publication::{PublicationErrors, PublicationPath, RustPublication, TemporaryName};

/// One successfully published assignment for an exact ticket attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssignmentRef {
    pub(crate) lease: AttemptLease,
    pub(crate) nonce: u128,
    pub(crate) path: PathBuf,
}

fn assignment_file_name(attempt_id: u64, nonce: u128) -> String {
    format!("assignment-{attempt_id}-{nonce}.md")
}

/// Publish complete assignment bytes through a same-directory temporary.
///
/// The returned path is safe to hand to a launcher: it exists only after the
/// complete sibling temporary has been renamed to the attempt/nonce-bound
/// destination.
pub(crate) fn write_assignment(
    artifact_dir: &Path,
    lease: &AttemptLease,
    nonce: u128,
    assignment: &[u8],
) -> Result<AssignmentRef, String> {
    std::fs::create_dir_all(artifact_dir).map_err(|error| {
        format!(
            "cannot create assignment directory {}: {error}",
            artifact_dir.display()
        )
    })?;

    let file_name = assignment_file_name(lease.attempt_id, nonce);
    let path = RustPublication {
        path: PublicationPath {
            destination: artifact_dir.join(&file_name),
            temporary_name: TemporaryName::Nonce {
                prefix: format!(".{file_name}.tmp."),
            },
        },
        body: assignment,
        errors: PublicationErrors {
            write: "cannot write assignment payload",
            publish: "cannot publish assignment payload",
        },
    }
    .publish()?;

    Ok(AssignmentRef {
        lease: lease.clone(),
        nonce,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn lease(attempt_id: u64) -> AttemptLease {
        AttemptLease {
            ticket_id: "T-045-'-$()`".to_string(),
            attempt_id,
        }
    }

    #[test]
    fn writes_ticket_attempt_nonce_assignment_and_reads_it_back_intact() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_dir = temp.path().join("attempt path with ' quote");
        let lease = lease(7);
        let nonce = 8_675_309_u128;
        let payload = format!(
            "Read everything exactly.\n{}",
            "quote:' double:\" dollar:$() backtick:`x` slash:\\\n\t\r\u{1b}".repeat(8_192)
        );

        let assignment =
            write_assignment(&artifact_dir, &lease, nonce, payload.as_bytes()).unwrap();

        assert_eq!(assignment.lease, lease);
        assert_eq!(assignment.nonce, nonce);
        assert_eq!(
            assignment.path,
            artifact_dir.join("assignment-7-8675309.md")
        );
        assert_eq!(fs::read(&assignment.path).unwrap(), payload.as_bytes());
        assert!(fs::read_dir(&artifact_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
    }

    #[test]
    fn interrupted_partial_temporary_never_becomes_the_published_assignment() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_dir = temp.path().join("interrupted attempt");
        fs::create_dir_all(&artifact_dir).unwrap();
        let lease = lease(11);
        let nonce = 42_u128;
        let destination = artifact_dir.join("assignment-11-42.md");
        let interrupted = artifact_dir.join(".assignment-11-42.md.tmp.interrupted");
        fs::write(&interrupted, b"partial and torn").unwrap();

        assert!(!destination.exists());
        assert_eq!(fs::read(&interrupted).unwrap(), b"partial and torn");

        let complete = b"complete assignment bytes\nwith a durable final line\n";
        let assignment = write_assignment(&artifact_dir, &lease, nonce, complete).unwrap();

        assert_eq!(assignment.path, destination);
        assert_eq!(fs::read(&assignment.path).unwrap(), complete);
        assert_eq!(
            fs::read(&interrupted).unwrap(),
            b"partial and torn",
            "an interrupted temporary is never mistaken for the durable reference"
        );
    }
}
