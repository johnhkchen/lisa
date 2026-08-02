//! The journal completion seal: hash every retained artifact, publish Done.
//!
//! The journal's records, fold, and append live in
//! [`lisa_core::completion_journal`] because two crates write them — this
//! adapter and the operator recovery command. What stays here is the half that
//! is genuinely plugin-side: turning a ticket and its work directory into the
//! content hashes a journal-sealed completion is made of, and publishing the
//! Done ticket through the scheduler's own atomic publication machinery.

use std::fs;
use std::path::Path;

use lisa_core::completion::{CompletionContentHash, CompletionSeal, CompletionSealReceipt};
use lisa_core::ticket;
use sha2::{Digest, Sha256};

pub(crate) use lisa_core::completion_journal::{
    load, CompletionFailureClass, CompletionJournalAggregate, CompletionJournalTransition,
    FailureConsequence,
};

use crate::publication::{
    publication_nonce, PublicationErrors, PublicationPath, RustPublication, TemporaryName,
};

const TEMPORARY_PREFIX: &str = ".completion-journal.jsonl.tmp.";
const TICKET_TEMPORARY_PREFIX: &str = ".journal-completion-ticket.tmp.";

/// Append one validated transition, published the scheduler's way.
///
/// The validation is `lisa-core`'s: the whole prior history folds before any
/// byte is written, so this adapter cannot append a record its own fail-closed
/// load would later refuse.
pub(crate) fn append_with_seal(
    path: &Path,
    seal: CompletionSeal,
    transition: CompletionJournalTransition,
) -> Result<CompletionJournalAggregate, String> {
    lisa_core::completion_journal::append_with_seal_using(
        path,
        seal,
        transition,
        |destination, body| {
            RustPublication {
                path: PublicationPath {
                    destination: destination.to_path_buf(),
                    temporary_name: TemporaryName::Nonce {
                        prefix: TEMPORARY_PREFIX.to_string(),
                    },
                },
                body,
                errors: PublicationErrors {
                    write: "cannot write completion journal temporary",
                    publish: "cannot publish completion journal",
                },
            }
            .publish()
            .map(|_| ())
        },
    )
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn completion_content_path(project_root: &Path, path: &Path) -> Result<String, String> {
    // In the WASM sandbox, ticket and work paths are already project-relative
    // (the plugin's cwd is the /host project mount) while `project_root` is
    // the absolute HOST path kept for host-side run_command — stripping one
    // against the other can never succeed there. A relative input is accepted
    // as already project-relative; the traversal guard below still applies.
    // (2026-07-18 rc.3 field stall: every journal seal failed this strip.)
    // Zellij mounts the host project at /host inside the plugin sandbox, and
    // filesystem events hand the plugin /host-prefixed absolute paths while
    // `project_root` stays the host-absolute path kept for run_command.
    // (Field, 2026-07-18 rc.6: T-001's seal failed labeling
    // "/host/docs/…" against "/home/tester/demo" — named by the bounded
    // failure routing within seconds of the attempt.)
    const SANDBOX_MOUNT: &str = "/host";
    let relative = match path
        .strip_prefix(project_root)
        .or_else(|_| path.strip_prefix(SANDBOX_MOUNT))
    {
        Ok(relative) => relative,
        Err(_) if path.is_relative() => path,
        Err(_) => {
            return Err(format!(
                "completion content path {} is outside project root {}",
                path.display(),
                project_root.display()
            ));
        }
    };
    if relative.as_os_str().is_empty() {
        return Err(format!(
            "completion content path {} must name a file below project root {}",
            path.display(),
            project_root.display()
        ));
    }
    if relative
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "completion content path {} escapes the project root",
            path.display()
        ));
    }
    Ok(relative.display().to_string())
}

fn content_hash(
    project_root: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<CompletionContentHash, String> {
    CompletionContentHash::new(completion_content_path(project_root, path)?, sha256(bytes))
}

fn collect_work_hashes(
    project_root: &Path,
    directory: &Path,
    hashes: &mut Vec<CompletionContentHash>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "cannot enumerate completion artifact directory {}: {error}",
            directory.display()
        )
    })?;
    let mut entries = entries
        .map(|entry| {
            entry.map_err(|error| {
                format!(
                    "cannot inspect completion artifact entry under {}: {error}",
                    directory.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "cannot inspect completion artifact {}: {error}",
                path.display()
            )
        })?;
        if file_type.is_dir() {
            collect_work_hashes(project_root, &path, hashes)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            let bytes = fs::read(&path).map_err(|error| {
                format!(
                    "cannot read and hash completion artifact {}: {error}",
                    path.display()
                )
            })?;
            hashes.push(content_hash(project_root, &path, &bytes)?);
        } else {
            return Err(format!(
                "cannot hash unsupported completion artifact {}: expected a regular file",
                path.display()
            ));
        }
    }
    Ok(())
}

fn prepare_done_ticket(ticket_file: &Path) -> Result<Vec<u8>, String> {
    let original = fs::read(ticket_file).map_err(|error| {
        format!(
            "cannot read completion ticket {}: {error}",
            ticket_file.display()
        )
    })?;
    let parent = ticket_file.parent().unwrap_or_else(|| Path::new(""));
    let prepared_path = parent.join(format!("{TICKET_TEMPORARY_PREFIX}{}", publication_nonce()));
    fs::write(&prepared_path, original).map_err(|error| {
        format!(
            "cannot write completion ticket preparation {}: {error}",
            prepared_path.display()
        )
    })?;

    let prepared = (|| {
        ticket::update_ticket_done(&prepared_path).map_err(|error| {
            format!(
                "cannot prepare completion ticket {}: {error}",
                ticket_file.display()
            )
        })?;
        fs::read(&prepared_path).map_err(|error| {
            format!(
                "cannot read prepared completion ticket {}: {error}",
                prepared_path.display()
            )
        })
    })();
    let cleanup = fs::remove_file(&prepared_path).map_err(|error| {
        format!(
            "cannot remove completion ticket preparation {}: {error}",
            prepared_path.display()
        )
    });
    match (prepared, cleanup) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => Err(format!("{primary}; cleanup also failed: {cleanup}")),
    }
}

fn complete_with_journal_seal_and_publish<F>(
    project_root: &Path,
    ticket_file: &Path,
    work_dir: &Path,
    publish: F,
) -> Result<CompletionSealReceipt, String>
where
    F: FnOnce(&Path, &[u8]) -> Result<(), String>,
{
    let done_ticket = prepare_done_ticket(ticket_file)?;
    let mut hashes = vec![content_hash(project_root, ticket_file, &done_ticket)?];
    collect_work_hashes(project_root, work_dir, &mut hashes)?;
    hashes.sort_by(|left, right| left.path().cmp(right.path()));
    let receipt = CompletionSealReceipt::journal(hashes)?;
    publish(ticket_file, &done_ticket)?;
    Ok(receipt)
}

/// Hash every retained completion artifact and atomically publish Done ticket bytes.
pub(crate) fn complete_with_journal_seal(
    project_root: &Path,
    ticket_file: &Path,
    work_dir: &Path,
) -> Result<CompletionSealReceipt, String> {
    complete_with_journal_seal_and_publish(
        project_root,
        ticket_file,
        work_dir,
        |destination, body| {
            RustPublication {
                path: PublicationPath {
                    destination: destination.to_path_buf(),
                    temporary_name: TemporaryName::Nonce {
                        prefix: TICKET_TEMPORARY_PREFIX.to_string(),
                    },
                },
                body,
                errors: PublicationErrors {
                    write: "cannot write completed ticket temporary",
                    publish: "cannot publish completed ticket",
                },
            }
            .publish()
            .map(|_| ())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_path_accepts_project_relative_inputs_from_the_sandbox() {
        // Field shape (2026-07-18 rc.3 stall): the plugin's cwd is the /host
        // project mount, so seal inputs are project-relative while
        // project_root is the absolute HOST path — the strip can never match.
        let root = Path::new("/home/tester/demo");
        assert_eq!(
            completion_content_path(root, Path::new("docs/active/tickets/T-001.md")).unwrap(),
            "docs/active/tickets/T-001.md"
        );
        assert_eq!(
            completion_content_path(root, Path::new("/home/tester/demo/review.md")).unwrap(),
            "review.md"
        );
        // Zellij filesystem events deliver /host-prefixed absolute paths —
        // the actual field shape from the 2026-07-18 rc.6 leg.
        assert_eq!(
            completion_content_path(root, Path::new("/host/docs/active/tickets/T-001.md")).unwrap(),
            "docs/active/tickets/T-001.md"
        );
        assert!(completion_content_path(root, Path::new("../escape.md"))
            .unwrap_err()
            .contains("escapes the project root"));
        assert!(
            completion_content_path(root, Path::new("/host/../etc/passwd")).is_err(),
            "traversal under the sandbox mount must still be rejected"
        );
        assert!(completion_content_path(root, Path::new("/etc/passwd"))
            .unwrap_err()
            .contains("outside project root"));
    }

    #[test]
    fn repo_less_journal_seal_hashes_final_ticket_and_every_nested_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-JOURNAL.md");
        let work = project.join("work/T-JOURNAL");
        fs::create_dir_all(work.join("nested")).unwrap();
        let original = b"---\nid: T-JOURNAL\nstatus: open\nphase: review\n---\nBody\n";
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        fs::write(&ticket, original).unwrap();
        fs::write(work.join("review.md"), b"# Review\n").unwrap();
        fs::write(work.join("nested/evidence.bin"), [0, 1, 2, 255]).unwrap();

        let receipt = complete_with_journal_seal(project, &ticket, &work).unwrap();

        assert!(!project.join(".git").exists());
        let done = fs::read(&ticket).unwrap();
        let done_text = String::from_utf8_lossy(&done);
        assert!(done_text.contains("status: done"));
        assert!(done_text.contains("phase: done"));
        assert_eq!(receipt.seal(), CompletionSeal::Journal);
        let hashes = receipt.content_hashes();
        assert_eq!(hashes.len(), 3);
        assert_eq!(hashes[0].path(), "tickets/T-JOURNAL.md");
        assert_eq!(hashes[0].sha256(), sha256(&done));
        assert_eq!(hashes[1].path(), "work/T-JOURNAL/nested/evidence.bin");
        assert_eq!(
            hashes[1].sha256(),
            sha256(&fs::read(work.join("nested/evidence.bin")).unwrap())
        );
        assert_eq!(hashes[2].path(), "work/T-JOURNAL/review.md");
        assert_eq!(
            hashes[2].sha256(),
            sha256(&fs::read(work.join("review.md")).unwrap())
        );

        let sealed_review_hash = hashes[2].sha256().to_string();
        fs::write(work.join("review.md"), b"# Mutated after seal\n").unwrap();
        assert_ne!(
            sealed_review_hash,
            sha256(&fs::read(work.join("review.md")).unwrap()),
            "post-seal mutation must make the recorded content hash detectably stale"
        );
        assert_eq!(
            fs::read_dir(ticket.parent().unwrap()).unwrap().count(),
            1,
            "journal completion must remove every sibling temporary"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_journal_artifact_names_the_path_and_preserves_review_ticket() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-UNREADABLE.md");
        let work = project.join("work/T-UNREADABLE");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        let original = b"---\nid: T-UNREADABLE\nstatus: open\nphase: review\n---\nBody\n";
        fs::write(&ticket, original).unwrap();
        let unreadable = work.join("missing-evidence.md");
        symlink("does-not-exist", &unreadable).unwrap();

        let error = complete_with_journal_seal(project, &ticket, &work).unwrap_err();

        assert!(error.contains("cannot read and hash completion artifact"));
        assert!(error.contains("missing-evidence.md"));
        assert_eq!(fs::read(&ticket).unwrap(), original);
        assert_eq!(
            fs::read_dir(ticket.parent().unwrap()).unwrap().count(),
            1,
            "failed hashing must clean the prepared ticket sibling"
        );
    }

    #[test]
    fn interrupted_ticket_publication_preserves_exact_review_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let ticket = project.join("tickets/T-INTERRUPTED.md");
        let work = project.join("work/T-INTERRUPTED");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        let original = b"---\nid: T-INTERRUPTED\nstatus: open\nphase: review\n---\nBody\n";
        fs::write(&ticket, original).unwrap();
        fs::write(work.join("review.md"), b"# Review\n").unwrap();

        let error = complete_with_journal_seal_and_publish(
            project,
            &ticket,
            &work,
            |destination, prepared| {
                assert_eq!(fs::read(destination).unwrap(), original);
                assert!(String::from_utf8_lossy(prepared).contains("status: done"));
                Err("hostile interruption before atomic rename".to_string())
            },
        )
        .unwrap_err();

        assert_eq!(error, "hostile interruption before atomic rename");
        assert_eq!(fs::read(&ticket).unwrap(), original);
        assert_eq!(fs::read_dir(ticket.parent().unwrap()).unwrap().count(), 1);
    }
}
