//! Durable, level-triggered projection of informational completion notes.
//!
//! Confirmed note-bearing generations come from the completion journal. Exact
//! operator acknowledgments come from provenance. Combining those append-only
//! facts yields the active queue without mutating tickets or scheduler state.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::disposition::DispositionNote;
use crate::provenance::{append_note_acknowledgment_record, system_time_to_epoch, SCHEMA_VERSION};

/// Exact durable identity of one note-bearing completion generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteKey {
    pub ticket_id: String,
    pub attempt_id: String,
    pub generation: u64,
}

/// One active, unacknowledged informational note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedNote {
    pub key: NoteKey,
    pub note: DispositionNote,
}

/// Result of trying to settle one active note for a ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteAcknowledgment {
    /// The ticket has no active note, so no provenance row was appended.
    NothingToRead,
    /// One exact note was durably acknowledged.
    Acknowledged {
        note: QueuedNote,
        remaining: usize,
        was_oldest_of_multiple: bool,
    },
}

/// Provenance fact that settles one exact note-bearing completion generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteAcknowledgmentRecord {
    pub schema_version: u32,
    pub record_type: NoteAcknowledgmentType,
    pub ticket_id: String,
    pub attempt_id: String,
    pub generation: u64,
    pub acknowledged_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoteAcknowledgmentType {
    NoteAcknowledged,
}

#[derive(Debug, Deserialize)]
struct CompletionJournalProjection {
    state: String,
    completion_id: String,
    attempt_id: String,
    generation: u64,
    #[serde(default)]
    note: Option<DispositionNote>,
}

fn read_jsonl(path: &Path, label: &str) -> Result<Vec<(usize, String)>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot read {label} {}: {error}", path.display())),
    };
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(b"\n") {
        return Err(format!(
            "{label} {} has a torn final record",
            path.display()
        ));
    }

    bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
        .map(|(index, line)| {
            let line_number = index + 1;
            if line.is_empty() {
                return Err(format!(
                    "{label} {} line {line_number} is unexpectedly empty",
                    path.display()
                ));
            }
            String::from_utf8(line.to_vec())
                .map(|line| (line_number, line))
                .map_err(|error| {
                    format!(
                        "cannot decode {label} {} line {line_number}: {error}",
                        path.display()
                    )
                })
        })
        .collect()
}

fn confirmed_notes(path: &Path) -> Result<BTreeMap<NoteKey, DispositionNote>, String> {
    let mut notes = BTreeMap::new();
    for (line_number, line) in read_jsonl(path, "completion journal")? {
        let row: CompletionJournalProjection = serde_json::from_str(&line).map_err(|error| {
            format!(
                "cannot parse completion journal {} line {line_number}: {error}",
                path.display()
            )
        })?;
        if row.state != "confirmed" {
            continue;
        }
        let key = NoteKey {
            ticket_id: row.completion_id,
            attempt_id: row.attempt_id,
            generation: row.generation,
        };
        if let Some(note) = row.note {
            notes.insert(key, note);
        }
    }
    Ok(notes)
}

fn acknowledged_keys(path: &Path) -> Result<HashSet<NoteKey>, String> {
    let mut acknowledged = HashSet::new();
    for (line_number, line) in read_jsonl(path, "provenance ledger")? {
        let value: serde_json::Value = serde_json::from_str(&line).map_err(|error| {
            format!(
                "cannot parse provenance ledger {} line {line_number}: {error}",
                path.display()
            )
        })?;
        if value.get("record_type").and_then(|value| value.as_str()) != Some("note-acknowledged") {
            continue;
        }
        let row: NoteAcknowledgmentRecord = serde_json::from_value(value).map_err(|error| {
            format!(
                "cannot parse note acknowledgment {} line {line_number}: {error}",
                path.display()
            )
        })?;
        acknowledged.insert(NoteKey {
            ticket_id: row.ticket_id,
            attempt_id: row.attempt_id,
            generation: row.generation,
        });
    }
    Ok(acknowledged)
}

/// Reconstruct every active note from durable journal and provenance facts.
pub fn collect_notes(
    completion_journal: &Path,
    provenance: &Path,
) -> Result<Vec<QueuedNote>, String> {
    let acknowledged = acknowledged_keys(provenance)?;
    Ok(confirmed_notes(completion_journal)?
        .into_iter()
        .filter(|(key, _)| !acknowledged.contains(key))
        .map(|(key, note)| QueuedNote { key, note })
        .collect())
}

/// Settle an active note for `ticket_id` by appending exact provenance.
///
/// With no requested generation, the oldest active generation is selected.
/// Supplying a generation selects that exact active generation instead.
pub fn acknowledge_note(
    completion_journal: &Path,
    provenance: &Path,
    ticket_id: &str,
    generation: Option<u64>,
) -> Result<NoteAcknowledgment, String> {
    let mut matches: Vec<_> = collect_notes(completion_journal, provenance)?
        .into_iter()
        .filter(|entry| entry.key.ticket_id == ticket_id)
        .collect();
    matches.sort_by(|left, right| {
        left.key
            .generation
            .cmp(&right.key.generation)
            .then_with(|| left.key.cmp(&right.key))
    });

    let was_oldest_of_multiple = generation.is_none() && matches.len() > 1;
    let entry = match generation {
        None => match matches.first() {
            Some(entry) => entry.clone(),
            None => return Ok(NoteAcknowledgment::NothingToRead),
        },
        Some(requested) => match matches
            .iter()
            .find(|entry| entry.key.generation == requested)
        {
            Some(entry) => entry.clone(),
            None => {
                let mut listed = matches
                    .iter()
                    .map(|entry| entry.key.generation.to_string())
                    .collect::<Vec<_>>();
                listed.dedup();
                let suffix = if listed.is_empty() {
                    "No generations are listed.".to_string()
                } else {
                    format!("Listed generations: {}.", listed.join(", "))
                };
                return Err(format!(
                    "Generation {requested} is not listed for {ticket_id}. {suffix}"
                ));
            }
        },
    };
    append_note_acknowledgment_record(
        provenance,
        &NoteAcknowledgmentRecord {
            schema_version: SCHEMA_VERSION,
            record_type: NoteAcknowledgmentType::NoteAcknowledged,
            ticket_id: entry.key.ticket_id.clone(),
            attempt_id: entry.key.attempt_id.clone(),
            generation: entry.key.generation,
            acknowledged_at: system_time_to_epoch(SystemTime::now()),
        },
    )
    .map_err(|error| {
        format!(
            "cannot append provenance ledger {}: {error}",
            provenance.display()
        )
    })?;
    Ok(NoteAcknowledgment::Acknowledged {
        note: entry,
        remaining: matches.len() - 1,
        was_oldest_of_multiple,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = r#"{"criterion_quote":"approximately 200 MiB","evidence_citation":"review.md#measurement","summary":"The measurement and criterion text disagree."}"#;

    fn row(state: &str, attempt: &str, generation: u64, note: Option<&str>) -> String {
        let note = note
            .map(|value| format!(r#", "note":{value}"#))
            .unwrap_or_default();
        let mut row = format!(
            r#"{{"schema_version":5,"seal":"commit","state":"{state}","completion_id":"T-1","attempt_id":"{attempt}","generation":{generation}{note}}}"#
        );
        row.push('\n');
        row
    }

    #[test]
    fn only_confirmed_note_is_queued() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(
            &journal,
            format!(
                "{}{}{}",
                row("requested", "attempt-a", 1, Some(NOTE)),
                row("confirmed", "attempt-a", 1, Some(NOTE)),
                row("confirmed", "attempt-b", 2, None),
            ),
        )
        .unwrap();

        let notes = collect_notes(&journal, &ledger).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].key.attempt_id, "attempt-a");
        assert_eq!(
            notes[0].note.summary(),
            "The measurement and criterion text disagree."
        );
    }

    #[test]
    fn acknowledgment_clears_exact_generation_and_is_durable() {
        use crate::provenance::ProvenanceLedgerRecord;

        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(&journal, row("confirmed", "attempt-a", 1, Some(NOTE))).unwrap();

        assert!(matches!(
            acknowledge_note(&journal, &ledger, "T-1", None).unwrap(),
            NoteAcknowledgment::Acknowledged {
                remaining: 0,
                was_oldest_of_multiple: false,
                ..
            }
        ));
        assert!(collect_notes(&journal, &ledger).unwrap().is_empty());
        assert_eq!(
            acknowledge_note(&journal, &ledger, "T-1", None).unwrap(),
            NoteAcknowledgment::NothingToRead
        );

        let line = fs::read_to_string(&ledger).unwrap();
        let record: NoteAcknowledgmentRecord = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(record.ticket_id, "T-1");
        assert_eq!(record.attempt_id, "attempt-a");
        assert!(matches!(
            serde_json::from_str::<ProvenanceLedgerRecord>(line.trim()).unwrap(),
            ProvenanceLedgerRecord::NoteAcknowledgment(_)
        ));
    }

    #[test]
    fn old_ack_does_not_hide_later_generation() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(&journal, row("confirmed", "attempt-a", 1, Some(NOTE))).unwrap();
        acknowledge_note(&journal, &ledger, "T-1", None).unwrap();
        fs::write(
            &journal,
            format!(
                "{}{}",
                row("confirmed", "attempt-a", 1, Some(NOTE)),
                row("confirmed", "attempt-b", 2, Some(NOTE))
            ),
        )
        .unwrap();

        let notes = collect_notes(&journal, &ledger).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].key.attempt_id, "attempt-b");
    }

    #[test]
    fn bare_acknowledgment_selects_oldest_generation_and_drains() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(
            &journal,
            format!(
                "{}{}",
                row("confirmed", "z-old-attempt", 1, Some(NOTE)),
                row("confirmed", "a-new-attempt", 2, Some(NOTE))
            ),
        )
        .unwrap();

        let outcome = acknowledge_note(&journal, &ledger, "T-1", None).unwrap();
        let NoteAcknowledgment::Acknowledged {
            note,
            remaining,
            was_oldest_of_multiple,
        } = outcome
        else {
            panic!("expected an acknowledged note");
        };
        assert_eq!(note.key.generation, 1);
        assert_eq!(note.key.attempt_id, "z-old-attempt");
        assert_eq!(remaining, 1);
        assert!(was_oldest_of_multiple);

        let active = collect_notes(&journal, &ledger).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].key.generation, 2);

        acknowledge_note(&journal, &ledger, "T-1", None).unwrap();
        assert!(collect_notes(&journal, &ledger).unwrap().is_empty());
        assert_eq!(
            acknowledge_note(&journal, &ledger, "T-1", None).unwrap(),
            NoteAcknowledgment::NothingToRead
        );

        let records: Vec<NoteAcknowledgmentRecord> = fs::read_to_string(&ledger)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].generation, 1);
        assert_eq!(records[0].attempt_id, "z-old-attempt");
        assert_eq!(records[1].generation, 2);
        assert_eq!(records[1].attempt_id, "a-new-attempt");
    }

    #[test]
    fn explicit_generation_selects_exactly_and_unknown_names_listed_values() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(
            &journal,
            format!(
                "{}{}",
                row("confirmed", "attempt-a", 1, Some(NOTE)),
                row("confirmed", "attempt-b", 2, Some(NOTE))
            ),
        )
        .unwrap();

        assert_eq!(
            acknowledge_note(&journal, &ledger, "T-1", Some(9)).unwrap_err(),
            "Generation 9 is not listed for T-1. Listed generations: 1, 2."
        );
        assert!(!ledger.exists());

        let outcome = acknowledge_note(&journal, &ledger, "T-1", Some(2)).unwrap();
        let NoteAcknowledgment::Acknowledged {
            note,
            remaining,
            was_oldest_of_multiple,
        } = outcome
        else {
            panic!("expected an acknowledged note");
        };
        assert_eq!(note.key.generation, 2);
        assert_eq!(remaining, 1);
        assert!(!was_oldest_of_multiple);

        let active = collect_notes(&journal, &ledger).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].key.generation, 1);
    }

    #[test]
    fn torn_or_malformed_history_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("completion.jsonl");
        let ledger = dir.path().join("provenance.jsonl");
        fs::write(&journal, "{}").unwrap();
        assert!(collect_notes(&journal, &ledger)
            .unwrap_err()
            .contains("torn"));
        fs::write(&journal, "not-json\n").unwrap();
        assert!(collect_notes(&journal, &ledger)
            .unwrap_err()
            .contains("line 1"));
    }
}
