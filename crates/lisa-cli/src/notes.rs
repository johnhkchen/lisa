use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lisa_core::notes::{acknowledge_note, collect_notes, NoteAcknowledgment, QueuedNote};

const COMPLETION_JOURNAL: &str = ".lisa/completion-journal.jsonl";
const PROVENANCE_LEDGER: &str = ".lisa/provenance.jsonl";

fn durable_paths(root: &Path) -> (PathBuf, PathBuf) {
    (root.join(COMPLETION_JOURNAL), root.join(PROVENANCE_LEDGER))
}

pub(crate) fn note_lines(notes: &[QueuedNote]) -> Vec<String> {
    if notes.is_empty() {
        return Vec::new();
    }
    let mut ticket_counts = HashMap::new();
    for entry in notes {
        *ticket_counts
            .entry(entry.key.ticket_id.as_str())
            .or_insert(0) += 1;
    }
    let mut lines = vec![format!("Notes for you ({})", notes.len())];
    for entry in notes {
        if ticket_counts[entry.key.ticket_id.as_str()] > 1 {
            lines.push(format!(
                "{}  Generation {}  {}",
                entry.key.ticket_id,
                entry.key.generation,
                entry.note.summary()
            ));
        } else {
            lines.push(format!("{}  {}", entry.key.ticket_id, entry.note.summary()));
        }
        lines.push(format!(
            "       Criterion: “{}”",
            entry.note.criterion_quote()
        ));
        lines.push(format!(
            "       Evidence: {}",
            entry.note.evidence_citation()
        ));
    }
    lines
}

pub(crate) fn print_notes(notes: &[QueuedNote]) {
    let lines = note_lines(notes);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        println!("{line}");
    }
    println!();
}

pub fn run_list(root: &Path) -> Result<(), String> {
    let (journal, ledger) = durable_paths(root);
    let notes = collect_notes(&journal, &ledger)?;
    if notes.is_empty() {
        println!("Nothing to read.");
    } else {
        print_notes(&notes);
    }
    Ok(())
}

pub fn run_ack(root: &Path, ticket_id: &str, generation: Option<u64>) -> Result<(), String> {
    let (journal, ledger) = durable_paths(root);
    match acknowledge_note(&journal, &ledger, ticket_id, generation)? {
        NoteAcknowledgment::NothingToRead => println!("Nothing to read for {ticket_id}."),
        NoteAcknowledgment::Acknowledged {
            note,
            remaining,
            was_oldest_of_multiple: true,
        } => {
            if remaining == 1 {
                println!("Marked the oldest note read — 1 more remains.");
            } else {
                println!("Marked the oldest note read — {remaining} more remain.");
            }
            debug_assert!(generation.is_none());
            debug_assert_eq!(note.key.ticket_id, ticket_id);
        }
        NoteAcknowledgment::Acknowledged { note, .. } => match generation {
            Some(generation) => println!(
                "{} generation {generation} acknowledged.",
                note.key.ticket_id
            ),
            None => println!("{} acknowledged.", note.key.ticket_id),
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::disposition::DispositionNote;
    use lisa_core::notes::NoteKey;

    #[test]
    fn formatter_leads_with_plain_summary() {
        let notes = vec![QueuedNote {
            key: NoteKey {
                ticket_id: "T-046-06-03".to_string(),
                attempt_id: "attempt-a".to_string(),
                generation: 1,
            },
            note: DispositionNote::new(
                "approximately 200 MiB",
                "review.md#measurement",
                "The recorded measurement and criterion text disagree.",
            )
            .unwrap(),
        }];

        assert_eq!(
            note_lines(&notes),
            vec![
                "Notes for you (1)",
                "T-046-06-03  The recorded measurement and criterion text disagree.",
                "       Criterion: “approximately 200 MiB”",
                "       Evidence: review.md#measurement",
            ]
        );
        assert!(note_lines(&[]).is_empty());
    }

    #[test]
    fn formatter_labels_only_tickets_with_multiple_active_notes() {
        fn queued(ticket_id: &str, generation: u64, summary: &str) -> QueuedNote {
            QueuedNote {
                key: NoteKey {
                    ticket_id: ticket_id.to_string(),
                    attempt_id: format!("attempt-{generation}"),
                    generation,
                },
                note: DispositionNote::new(
                    "approximately 200 MiB",
                    "review.md#measurement",
                    summary,
                )
                .unwrap(),
            }
        }

        let notes = vec![
            queued("T-MULTI", 1, "The first note."),
            queued("T-MULTI", 2, "The second note."),
            queued("T-SINGLE", 7, "The other ticket's note."),
        ];
        let lines = note_lines(&notes);
        assert_eq!(lines[0], "Notes for you (3)");
        assert_eq!(lines[1], "T-MULTI  Generation 1  The first note.");
        assert_eq!(lines[4], "T-MULTI  Generation 2  The second note.");
        assert_eq!(lines[7], "T-SINGLE  The other ticket's note.");
    }
}
