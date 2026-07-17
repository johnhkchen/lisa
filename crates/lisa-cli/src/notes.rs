use std::path::{Path, PathBuf};

use lisa_core::notes::{acknowledge_note, collect_notes, QueuedNote};

const COMPLETION_JOURNAL: &str = ".lisa/completion-journal.jsonl";
const PROVENANCE_LEDGER: &str = ".lisa/provenance.jsonl";

fn durable_paths(root: &Path) -> (PathBuf, PathBuf) {
    (root.join(COMPLETION_JOURNAL), root.join(PROVENANCE_LEDGER))
}

pub(crate) fn note_lines(notes: &[QueuedNote]) -> Vec<String> {
    if notes.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!("Notes for you ({})", notes.len())];
    for entry in notes {
        lines.push(format!("{}  {}", entry.key.ticket_id, entry.note.summary()));
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

pub fn run_ack(root: &Path, ticket_id: &str) -> Result<(), String> {
    let (journal, ledger) = durable_paths(root);
    let entry = acknowledge_note(&journal, &ledger, ticket_id)?;
    println!("{} acknowledged.", entry.key.ticket_id);
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
}
