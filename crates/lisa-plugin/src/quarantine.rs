//! Durable holding area for captures without confident ticket ownership.
//!
//! The scheduler decides whether a capture is attributable. This module only
//! maps the capture's opaque provider session to a safe path and appends the
//! source-ledger row once, even when the consumer rescans `captures.jsonl`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use lisa_core::capture::CaptureRecord;
use serde::{Deserialize, Serialize};

/// One capture held aside from ticket provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QuarantinedCaptureRecord {
    /// One-based physical line in the provider's append-only capture ledger.
    pub(crate) source_line: u64,
    /// The complete observed capture, preserved without attribution.
    pub(crate) capture: CaptureRecord,
}

/// Result of idempotently filing one capture-ledger row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AppendOutcome {
    /// This call durably appended the row.
    Appended(PathBuf),
    /// An earlier scan already appended this source line.
    AlreadyPresent(PathBuf),
}

/// Return a provider-local quarantine path keyed by an opaque session ID.
///
/// Only ASCII alphanumeric bytes, `-`, and `_` survive literally. Encoding all
/// other UTF-8 bytes prevents path traversal while keeping the mapping
/// injective and recognizable for ordinary provider session IDs.
pub(crate) fn session_path(provider_dir: &Path, session_id: &str) -> PathBuf {
    let encoded = if session_id.is_empty() {
        "%EMPTY".to_string()
    } else {
        encode_session_id(session_id)
    };
    provider_dir
        .join("quarantine")
        .join(format!("{encoded}.jsonl"))
}

fn encode_session_id(session_id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(session_id.len());
    for byte in session_id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

/// Append a capture to its session quarantine unless this source line is
/// already present.
pub(crate) fn append(
    provider_dir: &Path,
    source_line: u64,
    capture: &CaptureRecord,
) -> io::Result<AppendOutcome> {
    let path = session_path(provider_dir, &capture.session_id);

    match fs::read_to_string(&path) {
        Ok(raw) => {
            for existing in raw
                .lines()
                .filter_map(|line| serde_json::from_str::<QuarantinedCaptureRecord>(line).ok())
            {
                if existing.source_line != source_line {
                    continue;
                }
                if existing.capture == *capture {
                    return Ok(AppendOutcome::AlreadyPresent(path));
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "quarantine source line {source_line} already contains a different capture"
                    ),
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let record = QuarantinedCaptureRecord {
        source_line,
        capture: capture.clone(),
    };
    let mut line = serde_json::to_string(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    line.push('\n');

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;

    Ok(AppendOutcome::Appended(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture(session_id: &str) -> CaptureRecord {
        CaptureRecord {
            pane_id: 7,
            session_id: session_id.to_string(),
            captured_at: 1_752_345_600,
            input_tokens: 123,
            output_tokens: 45,
        }
    }

    #[test]
    fn session_paths_are_safe_and_injective_for_opaque_ids() {
        let dir = tempfile::tempdir().unwrap();
        let provider_dir = dir.path().join("codex");
        let cases = [
            ("session-abc_123", "session-abc_123.jsonl"),
            ("../escape", "%2E%2E%2Fescape.jsonl"),
            ("a/b\\c", "a%2Fb%5Cc.jsonl"),
            (".%", "%2E%25.jsonl"),
            ("café", "caf%C3%A9.jsonl"),
            ("", "%EMPTY.jsonl"),
            ("%EMPTY", "%25EMPTY.jsonl"),
        ];

        let mut paths = Vec::new();
        for (session_id, expected_file) in cases {
            let path = session_path(&provider_dir, session_id);
            assert_eq!(
                path.parent(),
                Some(provider_dir.join("quarantine").as_path())
            );
            assert_eq!(path.file_name().unwrap(), expected_file);
            paths.push(path);
        }

        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), cases.len(), "session keys must not collide");
    }

    #[test]
    fn append_is_idempotent_by_source_line_and_preserves_identical_rows() {
        let dir = tempfile::tempdir().unwrap();
        let provider_dir = dir.path().join("claude");
        let capture = capture("session-same");
        let path = session_path(&provider_dir, &capture.session_id);

        assert_eq!(
            append(&provider_dir, 1, &capture).unwrap(),
            AppendOutcome::Appended(path.clone())
        );
        let first_bytes = fs::read(&path).unwrap();
        let first: QuarantinedCaptureRecord =
            serde_json::from_slice(first_bytes.strip_suffix(b"\n").unwrap()).unwrap();
        assert_eq!(
            first,
            QuarantinedCaptureRecord {
                source_line: 1,
                capture: capture.clone(),
            }
        );

        assert_eq!(
            append(&provider_dir, 1, &capture).unwrap(),
            AppendOutcome::AlreadyPresent(path.clone())
        );
        assert_eq!(fs::read(&path).unwrap(), first_bytes);

        assert_eq!(
            append(&provider_dir, 2, &capture).unwrap(),
            AppendOutcome::Appended(path.clone())
        );
        let records: Vec<QuarantinedCaptureRecord> = fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].capture, records[1].capture);
        assert_eq!(records[0].source_line, 1);
        assert_eq!(records[1].source_line, 2);
    }
}
