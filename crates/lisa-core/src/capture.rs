//! Append-only token-capture observations awaiting ticket attribution.
//!
//! A capturing process knows which pane and provider session it observed, when
//! it captured the transcript, and the resulting token totals. It does not
//! know which ticket owned that pane at that time. [`CaptureRecord`] preserves
//! only those observed facts so a later consumer can perform attribution from
//! the scheduler's pane-ownership history.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One successful token-usage capture before ticket attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRecord {
    /// Terminal pane observed by the capturing process.
    pub pane_id: u32,
    /// Opaque provider/client session identifier.
    pub session_id: String,
    /// Capture time as UTC epoch seconds.
    pub captured_at: u64,
    /// Captured aggregate input-side token count.
    pub input_tokens: u64,
    /// Captured output-side token count.
    pub output_tokens: u64,
    /// Client that produced this record (`"claude"` or `"codex"`). Empty on
    /// records written before this field existed; the directory the record
    /// lives in (`.lisa/claude/` vs `.lisa/codex/`) still says which.
    #[serde(default)]
    pub client: String,
    /// Model the turn ran under, read from the transcript itself. `None`
    /// when the record predates this field, or the transcript did not say —
    /// never guessed from today's config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Effort level the turn ran under. Same provenance rule as `model`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// Append one capture as a compact, newline-terminated JSONL row.
///
/// The destination and its parent directories are created when absent. The
/// file is opened in append mode, so existing capture rows are never rewritten
/// or truncated.
pub fn append_capture_record(path: &Path, record: &CaptureRecord) -> std::io::Result<()> {
    let mut line = serde_json::to_string(record)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    line.push('\n');

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_capture() -> CaptureRecord {
        CaptureRecord {
            pane_id: 7,
            session_id: "session-first".to_string(),
            captured_at: 1_752_345_600,
            input_tokens: 1_024,
            output_tokens: 256,
            client: "claude".to_string(),
            model: Some("claude-opus-5".to_string()),
            effort: Some("high".to_string()),
        }
    }

    #[test]
    fn capture_record_round_trips_and_same_pane_appends_without_rewriting_first_row() {
        let first = sample_capture();
        let first_json = serde_json::to_string(&first).unwrap();
        let round_trip: CaptureRecord = serde_json::from_str(&first_json).unwrap();
        assert_eq!(round_trip, first);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/captures.jsonl");

        append_capture_record(&path, &first).unwrap();
        let first_write = fs::read(&path).unwrap();
        assert_eq!(first_write, format!("{first_json}\n").as_bytes());

        let second = CaptureRecord {
            pane_id: first.pane_id,
            session_id: "session-second".to_string(),
            captured_at: first.captured_at + 60,
            input_tokens: 2_048,
            output_tokens: 512,
            client: "claude".to_string(),
            model: None,
            effort: None,
        };
        append_capture_record(&path, &second).unwrap();

        let after_second_append = fs::read(&path).unwrap();
        assert!(
            after_second_append.starts_with(&first_write),
            "the second append must preserve every byte written by the first"
        );

        let rows: Vec<&[u8]> = after_second_append
            .split(|byte| *byte == b'\n')
            .filter(|row| !row.is_empty())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], first_json.as_bytes());

        let records: Vec<CaptureRecord> = rows
            .into_iter()
            .map(|row| serde_json::from_slice(row).unwrap())
            .collect();
        assert_eq!(records, vec![first, second]);
    }

    #[test]
    fn model_and_effort_are_omitted_from_json_when_unknown() {
        let record = CaptureRecord {
            model: None,
            effort: None,
            ..sample_capture()
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("\"model\""));
        assert!(!json.contains("\"effort\""));
    }

    #[test]
    fn a_pre_attribution_record_deserializes_with_unknown_client_model_and_effort() {
        // The exact shape capture_usage.rs wrote before this ticket landed —
        // never retroactively labelled with today's client, model, or effort.
        let pre_existing = r#"{"pane_id":0,"session_id":"s","captured_at":1,"input_tokens":2,"output_tokens":3}"#;
        let record: CaptureRecord = serde_json::from_str(pre_existing).unwrap();
        assert_eq!(record.client, "");
        assert_eq!(record.model, None);
        assert_eq!(record.effort, None);
    }
}
