//! Typed ingestion boundary for hook- and adapter-generated signal files.
//!
//! This module normalizes filesystem records, not provider semantics. Lease
//! currency, seat ownership, phase transitions, and provider acknowledgement
//! admission remain with the scheduler consumers in `lib.rs`.
//!
//! Consumption is single-reader by construction: a record is read and the file
//! removed. That contract is right for one scheduler and says nothing about
//! which one, so a consumer may name itself through [`Consumer`]. Naming
//! changes nothing about what is consumed — it leaves a receipt in
//! `.lisa/schedulers/`, so a signal that vanished into a second scheduler stays
//! distinguishable from one that was never written.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use lisa_core::{
    claim::AssignmentClaim,
    schedulers::SignalReceipt,
    types::{AttemptLease, TicketId},
};

/// The logical signal family one consumer requests from the shared directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignalRequest {
    Alive,
    Heartbeats,
    ProcessStarts,
    Claims,
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
    /// A process ran a tool call in this pane. Presence-only by construction:
    /// the hook writes it before it knows whether it can name itself, so it is
    /// the one liveness record a resident predecessor can still produce.
    Alive {
        pane_id: u32,
    },
    Heartbeat {
        pane_id: u32,
        lease: AttemptLease,
    },
    ProcessStarted {
        pane_id: u32,
        lease: AttemptLease,
    },
    Claim {
        pane_id: u32,
        claim: AssignmentClaim,
    },
    CodexAcknowledgement {
        pane_id: u32,
        payload: String,
    },
    Awaiting {
        pane_id: u32,
    },
    Idle {
        target: IdleTarget,
    },
    Stopped {
        pane_id: u32,
    },
    Cleared {
        pane_id: u32,
    },
    Error {
        pane_id: u32,
    },
}

/// The scheduler doing the consuming, so a taken signal has a name on it.
///
/// Owned rather than borrowed so a caller can build one from `&self` and still
/// mutate itself inside the loop that walks the returned records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Consumer {
    /// This scheduler's id in `.lisa/schedulers/`.
    pub(crate) scheduler_id: String,
    /// The registry directory receipts are appended in.
    pub(crate) registry: PathBuf,
    /// Unix seconds to stamp receipts with — the tick's clock reading, so every
    /// receipt from one tick agrees with the stamp that tick wrote.
    pub(crate) now: u64,
}

impl SignalRequest {
    /// Whether losing one of these signals changes what happens to a pane.
    ///
    /// Heartbeats and `.alive` pings are excluded on purpose: another one
    /// arrives within seconds, so nothing turns on which scheduler took a
    /// particular one, and a ledger that recorded them would be unreadable by
    /// the time anybody needed it.
    fn is_decisive(self) -> bool {
        !matches!(self, Self::Alive | Self::Heartbeats)
    }
}

/// Consume the records owned by one signal consumer.
///
/// Each call deliberately performs its own on-demand scan. This preserves the
/// scheduler's existing poll boundaries and allows records created between
/// consumers in one tick to be observed by the later consumer.
pub(crate) fn ingest(
    dir: &Path,
    request: SignalRequest,
    consumer: Option<&Consumer>,
) -> Vec<SignalRecord> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    entries
        .flatten()
        .filter_map(|entry| ingest_path(entry.path(), request, consumer))
        .collect()
}

/// Remove one recognized signal file, and leave a receipt when the consumer
/// named itself and the record was decisive.
///
/// The receipt follows the removal and is conditional on it: a file that could
/// not be removed was not consumed, and claiming otherwise would put the
/// scheduler's name on a signal still sitting in the directory.
fn take(path: &Path, request: SignalRequest, consumer: Option<&Consumer>) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string);
    if std::fs::remove_file(path).is_err() {
        return false;
    }
    if let (true, Some(consumer), Some(name)) = (request.is_decisive(), consumer, name) {
        let _ = lisa_core::schedulers::append_receipt(
            &consumer.registry,
            &SignalReceipt::new(consumer.now, &consumer.scheduler_id, name),
        );
    }
    true
}

fn ingest_path(
    path: PathBuf,
    request: SignalRequest,
    consumer: Option<&Consumer>,
) -> Option<SignalRecord> {
    match request {
        SignalRequest::Alive => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".alive")?;
            take(&path, request, consumer);
            Some(SignalRecord::Alive { pane_id })
        }
        SignalRequest::Heartbeats => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".heartbeat")?;
            ingest_lease(path, pane_id, request, consumer, |pane_id, lease| {
                SignalRecord::Heartbeat { pane_id, lease }
            })
        }
        SignalRequest::ProcessStarts => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".started")?;
            ingest_lease(path, pane_id, request, consumer, |pane_id, lease| {
                SignalRecord::ProcessStarted { pane_id, lease }
            })
        }
        SignalRequest::Claims => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".claim")?;
            let claim = std::fs::read_to_string(&path)
                .ok()
                .and_then(|body| serde_json::from_str::<AssignmentClaim>(&body).ok());
            take(&path, request, consumer);
            claim.map(|claim| SignalRecord::Claim { pane_id, claim })
        }
        SignalRequest::CodexAcknowledgements => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".ack")?;
            let payload = std::fs::read_to_string(&path).ok();
            take(&path, request, consumer);
            payload.map(|payload| SignalRecord::CodexAcknowledgement { pane_id, payload })
        }
        SignalRequest::Awaiting => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".awaiting")?;
            take(&path, request, consumer);
            Some(SignalRecord::Awaiting { pane_id })
        }
        SignalRequest::Idle => ingest_idle(path, request, consumer),
        SignalRequest::Transitions => ingest_transition(path, request, consumer),
        SignalRequest::Errors => {
            let pane_id = pane_id_from_signal_filename(path.file_name()?, ".error")?;
            take(&path, request, consumer);
            Some(SignalRecord::Error { pane_id })
        }
    }
}

fn ingest_lease(
    path: PathBuf,
    pane_id: u32,
    request: SignalRequest,
    consumer: Option<&Consumer>,
    record: impl FnOnce(u32, AttemptLease) -> SignalRecord,
) -> Option<SignalRecord> {
    let lease = std::fs::read_to_string(&path)
        .ok()
        .and_then(|body| serde_json::from_str::<AttemptLease>(&body).ok());
    take(&path, request, consumer);
    lease.map(|lease| record(pane_id, lease))
}

fn ingest_idle(
    path: PathBuf,
    request: SignalRequest,
    consumer: Option<&Consumer>,
) -> Option<SignalRecord> {
    let filename = path.file_name()?.to_str()?;
    let stem = filename.strip_suffix(".idle")?.to_string();

    // Idle historically deletes after the broad suffix match, before parsing a
    // pane id or resolving a legacy ticket name.
    take(&path, request, consumer);

    let target = match stem.strip_prefix("pane-") {
        Some(pane_id) => IdleTarget::Pane(pane_id.parse().ok()?),
        None => IdleTarget::LegacyTicket(stem),
    };
    Some(SignalRecord::Idle { target })
}

fn ingest_transition(
    path: PathBuf,
    request: SignalRequest,
    consumer: Option<&Consumer>,
) -> Option<SignalRecord> {
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
    take(&path, request, consumer);
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

    fn consumer(registry: &Path, scheduler_id: &str, now: u64) -> Consumer {
        Consumer {
            scheduler_id: scheduler_id.to_string(),
            registry: registry.to_path_buf(),
            now,
        }
    }

    /// The incident, reproduced at the boundary it happened at: two schedulers
    /// scanning one signal directory. The first one to look takes
    /// `pane-1.started`, the second finds nothing and cannot tell that outcome
    /// from a startup that never happened — except by the receipt.
    #[test]
    fn two_schedulers_on_one_directory_split_the_started_signal() {
        let signals = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let zombie = consumer(registry.path(), "fascinating-drum", 1_000);
        let live = consumer(registry.path(), "blossoming-cymbal", 1_001);
        fs::write(
            signals.path().join("pane-1.started"),
            serde_json::to_string(&lease()).unwrap(),
        )
        .unwrap();

        let taken = ingest(signals.path(), SignalRequest::ProcessStarts, Some(&zombie));
        let missed = ingest(signals.path(), SignalRequest::ProcessStarts, Some(&live));

        assert_eq!(
            taken,
            vec![SignalRecord::ProcessStarted {
                pane_id: 1,
                lease: lease(),
            }]
        );
        assert!(
            missed.is_empty(),
            "the single-consumer contract is unchanged: the second reader gets nothing"
        );

        let receipt = lisa_core::schedulers::taken_by_another(
            registry.path(),
            "blossoming-cymbal",
            "pane-1.started",
            0,
        )
        .expect("the live scheduler can find out where its start signal went");
        assert_eq!(receipt.scheduler_id, "fascinating-drum");
        assert_eq!(receipt.pane_id, Some(1));
    }

    /// Every few seconds forever is not evidence anybody can read, so the
    /// ledger records the decisive families and leaves the chatter alone.
    #[test]
    fn heartbeats_and_alive_pings_leave_no_receipts() {
        let signals = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let consumer = consumer(registry.path(), "only", 500);
        fs::write(signals.path().join("pane-2.alive"), "ping").unwrap();
        fs::write(
            signals.path().join("pane-2.heartbeat"),
            serde_json::to_string(&lease()).unwrap(),
        )
        .unwrap();
        fs::write(signals.path().join("pane-2.stopped"), "").unwrap();

        ingest(signals.path(), SignalRequest::Alive, Some(&consumer));
        ingest(signals.path(), SignalRequest::Heartbeats, Some(&consumer));
        ingest(signals.path(), SignalRequest::Transitions, Some(&consumer));

        let receipts = lisa_core::schedulers::read_receipts(registry.path());
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.signal.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-2.stopped"]
        );
    }

    /// A consumer that does not name itself consumes exactly as it always did.
    #[test]
    fn an_unnamed_consumer_writes_no_receipts_at_all() {
        let signals = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        fs::write(signals.path().join("pane-3.error"), "").unwrap();

        assert_eq!(
            ingest(signals.path(), SignalRequest::Errors, None),
            vec![SignalRecord::Error { pane_id: 3 }]
        );
        assert!(lisa_core::schedulers::read_receipts(registry.path()).is_empty());
    }

    #[test]
    fn lease_payload_is_typed_and_malformed_payload_is_still_consumed() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("pane-7.heartbeat");
        let malformed = dir.path().join("pane-8.heartbeat");
        fs::write(&valid, serde_json::to_string(&lease()).unwrap()).unwrap();
        fs::write(&malformed, "not a lease").unwrap();

        let records = ingest(dir.path(), SignalRequest::Heartbeats, None);

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
    fn alive_is_presence_only_and_never_ingested_as_a_heartbeat() {
        let dir = tempfile::tempdir().unwrap();
        let alive = dir.path().join("pane-7.alive");
        let heartbeat = dir.path().join("pane-8.heartbeat");
        fs::write(&alive, "2026-01-01T00:00:00Z\n").unwrap();
        fs::write(&heartbeat, serde_json::to_string(&lease()).unwrap()).unwrap();

        // The two families are separate scans: an `.alive` body is never parsed
        // as a lease, and a heartbeat scan leaves it alone.
        assert_eq!(
            ingest(dir.path(), SignalRequest::Heartbeats, None),
            vec![SignalRecord::Heartbeat {
                pane_id: 8,
                lease: lease(),
            }]
        );
        assert!(alive.exists());

        assert_eq!(
            ingest(dir.path(), SignalRequest::Alive, None),
            vec![SignalRecord::Alive { pane_id: 7 }]
        );
        assert!(!alive.exists());
    }

    #[test]
    fn raw_provider_payload_and_presence_remain_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let ack = dir.path().join("pane-7.ack");
        let awaiting = dir.path().join("pane-8.awaiting");
        fs::write(&ack, "{ provider payload }").unwrap();
        fs::write(&awaiting, "body is ignored").unwrap();

        assert_eq!(
            ingest(dir.path(), SignalRequest::CodexAcknowledgements, None),
            vec![SignalRecord::CodexAcknowledgement {
                pane_id: 7,
                payload: "{ provider payload }".to_string(),
            }]
        );
        assert_eq!(
            ingest(dir.path(), SignalRequest::Awaiting, None),
            vec![SignalRecord::Awaiting { pane_id: 8 }]
        );
        assert!(!ack.exists());
        assert!(!awaiting.exists());
    }

    #[test]
    fn claim_payload_is_typed_and_malformed_payload_is_still_consumed() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("pane-7.claim");
        let malformed = dir.path().join("pane-8.claim");
        let claim = AssignmentClaim {
            ticket_id: "T-CLAIM".to_string(),
            attempt_id: 17,
            nonce: u128::from(u64::MAX) + 42,
        };
        fs::write(&valid, serde_json::to_string(&claim).unwrap()).unwrap();
        fs::write(&malformed, "not a claim").unwrap();

        assert_eq!(
            ingest(dir.path(), SignalRequest::Claims, None),
            vec![SignalRecord::Claim { pane_id: 7, claim }]
        );
        assert!(!valid.exists());
        assert!(!malformed.exists());
    }

    #[test]
    fn strict_pane_recognition_retains_invalid_names() {
        let dir = tempfile::tempdir().unwrap();
        let invalid = dir.path().join("pane-seven.error");
        fs::write(&invalid, "ignored").unwrap();

        assert!(ingest(dir.path(), SignalRequest::Errors, None).is_empty());
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

        let mut records = ingest(dir.path(), SignalRequest::Idle, None);
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

        let records = ingest(dir.path(), SignalRequest::Transitions, None);

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
