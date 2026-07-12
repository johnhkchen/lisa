//! Typed ingestion boundary for hook- and adapter-generated signal files.
//!
//! This module normalizes filesystem records, not provider semantics. Lease
//! currency, seat ownership, phase transitions, and provider acknowledgement
//! admission remain with the scheduler consumers in `lib.rs`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use lisa_core::types::{AttemptLease, TicketId};

/// The logical signal family one consumer requests from the shared directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignalRequest {
    Heartbeats,
    ProcessStarts,
    ShellReady,
    CodexAcknowledgements,
    Awaiting,
    Idle,
    Transitions,
    Errors,
}

/// The two intentionally distinct filename authorities accepted for idle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdleTarget {
    Pane(u32),
    LegacyTicket(TicketId),
}

/// A filesystem signal after filename recognition and payload acquisition.
///
/// Lease JSON, raw provider JSON, and presence-only records remain separate so
/// consumers cannot accidentally treat provider-specific evidence uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SignalRecord {
    Heartbeat { pane_id: u32, lease: AttemptLease },
    ProcessStarted { pane_id: u32, lease: AttemptLease },
    ShellReady { pane_id: u32, lease: AttemptLease },
    CodexAcknowledgement { pane_id: u32, payload: String },
    Awaiting { pane_id: u32 },
    Idle { target: IdleTarget },
    Stopped { pane_id: u32 },
    Cleared { pane_id: u32 },
    Error { pane_id: u32 },
}

/// Consume the records owned by one signal consumer.
///
/// Each call deliberately performs its own on-demand scan. This preserves the
/// scheduler's existing poll boundaries and allows records created between
/// consumers in one tick to be observed by the later consumer.
pub(crate) fn ingest(dir: &Path, request: SignalRequest) -> Vec<SignalRecord> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    entries
        .flatten()
        .filter_map(|entry| ingest_path(entry.path(), request))
        .collect()
}

fn ingest_path(path: PathBuf, request: SignalRequest) -> Option<SignalRecord> {
    match request {
        SignalRequest::Heartbeats => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".heartbeat")?;
            ingest_lease(path, pane_id, |pane_id, lease| SignalRecord::Heartbeat {
                pane_id,
                lease,
            })
        }
        SignalRequest::ProcessStarts => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".started")?;
            ingest_lease(path, pane_id, |pane_id, lease| {
                SignalRecord::ProcessStarted { pane_id, lease }
            })
        }
        SignalRequest::ShellReady => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".shell-ready")?;
            ingest_lease(path, pane_id, |pane_id, lease| SignalRecord::ShellReady {
                pane_id,
                lease,
            })
        }
        SignalRequest::CodexAcknowledgements => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".ack")?;
            let payload = std::fs::read_to_string(&path).ok();
            let _ = std::fs::remove_file(path);
            payload.map(|payload| SignalRecord::CodexAcknowledgement { pane_id, payload })
        }
        SignalRequest::Awaiting => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".awaiting")?;
            let _ = std::fs::remove_file(path);
            Some(SignalRecord::Awaiting { pane_id })
        }
        SignalRequest::Idle => ingest_idle(path),
        SignalRequest::Transitions => ingest_transition(path),
        SignalRequest::Errors => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".error")?;
            let _ = std::fs::remove_file(path);
            Some(SignalRecord::Error { pane_id })
        }
    }
}

fn ingest_lease(
    path: PathBuf,
    pane_id: u32,
    record: impl FnOnce(u32, AttemptLease) -> SignalRecord,
) -> Option<SignalRecord> {
    let lease = std::fs::read_to_string(&path)
        .ok()
        .and_then(|body| serde_json::from_str::<AttemptLease>(&body).ok());
    let _ = std::fs::remove_file(path);
    lease.map(|lease| record(pane_id, lease))
}

fn ingest_idle(path: PathBuf) -> Option<SignalRecord> {
    let filename = path.file_name()?.to_str()?;
    let stem = filename.strip_suffix(".idle")?.to_string();

    // Idle historically deletes after the broad suffix match, before parsing a
    // pane id or resolving a legacy ticket name.
    let _ = std::fs::remove_file(path);

    let target = match stem.strip_prefix("pane-") {
        Some(pane_id) => IdleTarget::Pane(pane_id.parse().ok()?),
        None => IdleTarget::LegacyTicket(stem),
    };
    Some(SignalRecord::Idle { target })
}

fn ingest_transition(path: PathBuf) -> Option<SignalRecord> {
    let filename = path.file_name()?.to_str()?;
    let pane_and_kind = filename.strip_prefix("pane-").and_then(|pane_and_suffix| {
        pane_and_suffix
            .strip_suffix(".stopped")
            .map(|pane_id| (pane_id.to_string(), true))
            .or_else(|| {
                pane_and_suffix
                    .strip_suffix(".cleared")
                    .map(|pane_id| (pane_id.to_string(), false))
            })
    })?;

    // Transition signals are also deleted after broad suffix recognition but
    // before pane parsing; malformed recognized records remain one-shot.
    let _ = std::fs::remove_file(path);
    let pane_id = pane_and_kind.0.parse().ok()?;
    Some(if pane_and_kind.1 {
        SignalRecord::Stopped { pane_id }
    } else {
        SignalRecord::Cleared { pane_id }
    })
}

/// Parse the pane id from one exact `pane-<u32>.<suffix>` signal filename.
pub(crate) fn pane_id_from_signal_filename(filename: &OsStr, suffix: &str) -> Option<u32> {
    filename
        .to_str()?
        .strip_prefix("pane-")?
        .strip_suffix(suffix)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn lease() -> AttemptLease {
        AttemptLease {
            ticket_id: "T-SIGNAL".to_string(),
            attempt_id: 17,
        }
    }

    #[test]
    fn lease_payload_is_typed_and_malformed_payload_is_still_consumed() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("pane-7.heartbeat");
        let malformed = dir.path().join("pane-8.heartbeat");
        fs::write(&valid, serde_json::to_string(&lease()).unwrap()).unwrap();
        fs::write(&malformed, "not a lease").unwrap();

        let records = ingest(dir.path(), SignalRequest::Heartbeats);

        assert_eq!(
            records,
            vec![SignalRecord::Heartbeat {
                pane_id: 7,
                lease: lease(),
            }]
        );
        assert!(!valid.exists());
        assert!(!malformed.exists());
    }

    #[test]
    fn raw_provider_payload_and_presence_remain_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let ack = dir.path().join("pane-7.ack");
        let awaiting = dir.path().join("pane-8.awaiting");
        fs::write(&ack, "{ provider payload }").unwrap();
        fs::write(&awaiting, "body is ignored").unwrap();

        assert_eq!(
            ingest(dir.path(), SignalRequest::CodexAcknowledgements),
            vec![SignalRecord::CodexAcknowledgement {
                pane_id: 7,
                payload: "{ provider payload }".to_string(),
            }]
        );
        assert_eq!(
            ingest(dir.path(), SignalRequest::Awaiting),
            vec![SignalRecord::Awaiting { pane_id: 8 }]
        );
        assert!(!ack.exists());
        assert!(!awaiting.exists());
    }

    #[test]
    fn strict_pane_recognition_retains_invalid_names() {
        let dir = tempfile::tempdir().unwrap();
        let invalid = dir.path().join("pane-seven.error");
        fs::write(&invalid, "ignored").unwrap();

        assert!(ingest(dir.path(), SignalRequest::Errors).is_empty());
        assert!(invalid.exists());
    }

    #[test]
    fn idle_preserves_pane_and_legacy_targets_but_deletes_malformed_panes() {
        let dir = tempfile::tempdir().unwrap();
        let pane = dir.path().join("pane-7.idle");
        let legacy = dir.path().join("T-LEGACY.idle");
        let malformed = dir.path().join("pane-seven.idle");
        fs::write(&pane, "").unwrap();
        fs::write(&legacy, "").unwrap();
        fs::write(&malformed, "").unwrap();

        let mut records = ingest(dir.path(), SignalRequest::Idle);
        records.sort_by_key(|record| format!("{record:?}"));
        let mut expected = vec![
            SignalRecord::Idle {
                target: IdleTarget::Pane(7),
            },
            SignalRecord::Idle {
                target: IdleTarget::LegacyTicket("T-LEGACY".to_string()),
            },
        ];
        expected.sort_by_key(|record| format!("{record:?}"));

        assert_eq!(records, expected);
        assert!(!pane.exists());
        assert!(!legacy.exists());
        assert!(!malformed.exists());
    }

    #[test]
    fn transitions_share_one_scan_and_delete_malformed_recognized_names() {
        let dir = tempfile::tempdir().unwrap();
        let stopped = dir.path().join("pane-7.stopped");
        let cleared = dir.path().join("pane-8.cleared");
        let malformed = dir.path().join("pane-seven.stopped");
        let unrelated = dir.path().join("pane-9.idle");
        for path in [&stopped, &cleared, &malformed, &unrelated] {
            fs::write(path, "body is ignored").unwrap();
        }

        let records = ingest(dir.path(), SignalRequest::Transitions);

        assert!(records.contains(&SignalRecord::Stopped { pane_id: 7 }));
        assert!(records.contains(&SignalRecord::Cleared { pane_id: 8 }));
        assert_eq!(records.len(), 2);
        assert!(!stopped.exists());
        assert!(!cleared.exists());
        assert!(!malformed.exists());
        assert!(unrelated.exists());
    }

    #[test]
    fn pane_signal_filename_parser_enforces_exact_grammar() {
        let cases = [
            ("pane-0.heartbeat", ".heartbeat", Some(0)),
            ("pane-42.started", ".started", Some(42)),
            ("pane-4294967295.error", ".error", Some(u32::MAX)),
            ("pane-0007.ack", ".ack", Some(7)),
            ("seat-7.ack", ".ack", None),
            ("pane-7.awaiting", ".ack", None),
            ("pane-7.ack.backup", ".ack", None),
            ("pane-.cleared", ".cleared", None),
            ("pane-seven.stopped", ".stopped", None),
            ("pane--1.error", ".error", None),
            ("pane- 1.error", ".error", None),
            ("pane-4294967296.error", ".error", None),
            ("pane-7.error", "", None),
        ];

        for (filename, suffix, expected) in cases {
            assert_eq!(
                pane_id_from_signal_filename(OsStr::new(filename), suffix),
                expected,
                "filename={filename:?}, suffix={suffix:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn pane_signal_filename_parser_rejects_non_utf8() {
        use std::os::unix::ffi::OsStringExt;

        let filename = std::ffi::OsString::from_vec(b"pane-7.\xfferror".to_vec());
        assert_eq!(pane_id_from_signal_filename(&filename, ".error"), None);
    }
}
