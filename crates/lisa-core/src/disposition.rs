//! Fail-closed parsing for the machine-readable Review disposition.
//!
//! A disposition file grants completion authority only when it contains the
//! exact valid pass relationship. Missing, unreadable, malformed, unknown, or
//! contradictory input is represented as [`ReviewDisposition::Invalid`] so a
//! caller cannot confuse parser failure with agent approval.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The party whose durable reality must change before a blocked ticket can
/// make progress again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemedyOwner {
    Agent,
    Operator,
    World,
}

/// A criteria-versus-evidence dispute that does not stop completed work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositionNote {
    criterion_quote: String,
    evidence_citation: String,
    summary: String,
}

impl DispositionNote {
    /// Build a note whose required fields all contain visible content.
    pub fn new(
        criterion_quote: impl Into<String>,
        evidence_citation: impl Into<String>,
        summary: impl Into<String>,
    ) -> Result<Self, String> {
        let criterion_quote = criterion_quote.into();
        if criterion_quote.trim().is_empty() {
            return Err("a note disposition requires a non-empty criterion quote".to_string());
        }
        let evidence_citation = evidence_citation.into();
        if evidence_citation.trim().is_empty() {
            return Err("a note disposition requires a non-empty evidence citation".to_string());
        }
        let summary = summary.into();
        if summary.trim().is_empty() {
            return Err("a note disposition requires a non-empty summary".to_string());
        }
        Ok(Self {
            criterion_quote,
            evidence_citation,
            summary,
        })
    }

    /// Borrow the disputed acceptance criterion exactly as supplied.
    pub fn criterion_quote(&self) -> &str {
        &self.criterion_quote
    }

    /// Borrow the project-relative evidence citation exactly as supplied.
    pub fn evidence_citation(&self) -> &str {
        &self.evidence_citation
    }

    /// Borrow the plain one-sentence summary exactly as supplied.
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// The validated outcome of a Review disposition file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewDisposition {
    /// The agent explicitly declared that Review passed.
    Pass,
    /// The work passed with a recorded criteria-versus-evidence dispute.
    Note(DispositionNote),
    /// The agent blocked completion with an actionable reason.
    Block {
        reason: String,
        remedy_owner: RemedyOwner,
        ask: String,
        steps: Option<Vec<String>>,
        check: Option<String>,
        /// True when missing or malformed remedy structure was replaced with
        /// the safe operator-owned legacy fallback.
        unstructured: bool,
    },
    /// The file could not be trusted as either valid disposition.
    Invalid { reason: String },
}

impl ReviewDisposition {
    /// Return whether this validated disposition authorizes completion.
    pub const fn authorizes_completion(&self) -> bool {
        matches!(self, Self::Pass | Self::Note(_))
    }
}

/// Read and validate a Review disposition file.
///
/// The accepted documents contain both `disposition` and `reason`. A pass must
/// have a null reason; a block must have a non-empty, non-whitespace reason.
/// Every read, JSON, or schema failure returns a non-passing `Invalid` value.
pub fn parse_review_disposition(path: impl AsRef<Path>) -> ReviewDisposition {
    let path = path.as_ref();
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            return ReviewDisposition::Invalid {
                reason: format!(
                    "could not read review disposition {}: {error}",
                    path.display()
                ),
            };
        }
    };

    let document = match serde_json::from_str(&contents) {
        Ok(document) => document,
        Err(error) => {
            return ReviewDisposition::Invalid {
                reason: format!("review disposition is malformed JSON: {error}"),
            };
        }
    };

    validate_document(document)
}

fn validate_document(document: Value) -> ReviewDisposition {
    let Value::Object(mut object) = document else {
        return invalid("review disposition must be a JSON object");
    };

    let Some(disposition) = object.remove("disposition") else {
        return invalid("review disposition is missing the disposition field");
    };
    let Some(reason) = object.remove("reason") else {
        return invalid("review disposition is missing the reason field");
    };
    let Value::String(disposition) = disposition else {
        return invalid("review disposition field must be a string");
    };

    match (disposition.as_str(), reason) {
        ("pass", Value::Null) => ReviewDisposition::Pass,
        ("pass", _) => invalid("a passing review disposition must have a null reason"),
        ("note", Value::Null) => validate_note_structure(&mut object),
        ("note", _) => invalid("a note review disposition must have a null reason"),
        ("block", Value::String(reason)) if !reason.trim().is_empty() => {
            validate_block_structure(reason, &mut object)
        }
        ("block", _) => {
            invalid("a blocking review disposition must have a non-empty string reason")
        }
        (unknown, _) => invalid(format!(
            "unknown review disposition {unknown:?}; expected \"pass\", \"note\", or \"block\""
        )),
    }
}

fn validate_note_structure(object: &mut serde_json::Map<String, Value>) -> ReviewDisposition {
    let note = (|| {
        let note = DispositionNote::new(
            non_empty_string(object.remove("criterion_quote")?)?,
            non_empty_string(object.remove("evidence_citation")?)?,
            non_empty_string(object.remove("summary")?)?,
        )
        .ok()?;
        object.is_empty().then_some(note)
    })();

    match note {
        Some(note) => ReviewDisposition::Note(note),
        None => invalid(
            "a note review disposition requires non-empty criterion_quote, evidence_citation, and summary string fields",
        ),
    }
}

fn validate_block_structure(
    reason: String,
    object: &mut serde_json::Map<String, Value>,
) -> ReviewDisposition {
    let structure = (|| {
        let remedy_owner = match object.remove("remedy_owner")? {
            Value::String(owner) if owner == "agent" => RemedyOwner::Agent,
            Value::String(owner) if owner == "operator" => RemedyOwner::Operator,
            Value::String(owner) if owner == "world" => RemedyOwner::World,
            _ => return None,
        };
        let ask = non_empty_string(object.remove("ask")?)?;
        let steps = match object.remove("steps") {
            None => None,
            Some(Value::Array(values)) => Some(
                values
                    .into_iter()
                    .map(non_empty_string)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Some(_) => return None,
        };
        let check = match object.remove("check") {
            None => None,
            Some(value) => Some(non_empty_string(value)?),
        };
        Some((remedy_owner, ask, steps, check))
    })();

    match structure {
        Some((remedy_owner, ask, steps, check)) => ReviewDisposition::Block {
            reason,
            remedy_owner,
            ask,
            steps,
            check,
            unstructured: false,
        },
        None => unstructured_block(reason),
    }
}

fn non_empty_string(value: Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn unstructured_block(reason: String) -> ReviewDisposition {
    ReviewDisposition::Block {
        ask: reason.clone(),
        reason,
        remedy_owner: RemedyOwner::Operator,
        steps: None,
        check: None,
        unstructured: true,
    }
}

fn invalid(reason: impl Into<String>) -> ReviewDisposition {
    ReviewDisposition::Invalid {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_document(document: &str) -> ReviewDisposition {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review-disposition.json");
        fs::write(&path, document).unwrap();
        parse_review_disposition(path)
    }

    fn assert_invalid(disposition: ReviewDisposition) {
        assert!(
            matches!(disposition, ReviewDisposition::Invalid { .. }),
            "expected an invalid disposition, got {disposition:?}"
        );
    }

    #[test]
    fn parses_pass() {
        assert_eq!(
            parse_document(r#"{"disposition":"pass","reason":null}"#),
            ReviewDisposition::Pass
        );
    }

    #[test]
    fn parses_criteria_evidence_note_without_normalizing_content() {
        assert_eq!(
            parse_document(
                r#"{"disposition":"note","reason":null,"criterion_quote":"  approximately 200 MiB  ","evidence_citation":"docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md","summary":"The 225 MiB measurement supports completion while the written gate is stale."}"#,
            ),
            ReviewDisposition::Note(
                DispositionNote::new(
                    "  approximately 200 MiB  ",
                    "docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md",
                    "The 225 MiB measurement supports completion while the written gate is stale.",
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn malformed_note_shapes_are_invalid() {
        for document in [
            r#"{"disposition":"note","reason":null,"evidence_citation":"docs/evidence.md","summary":"The criterion is stale."}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","summary":"The criterion is stale."}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":"docs/evidence.md"}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":" ","evidence_citation":"docs/evidence.md","summary":"The criterion is stale."}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":" ","summary":"The criterion is stale."}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":"docs/evidence.md","summary":" "}"#,
            r#"{"disposition":"note","reason":"work is questionable","criterion_quote":"criterion","evidence_citation":"docs/evidence.md","summary":"The criterion is stale."}"#,
            r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":"docs/evidence.md","summary":"The criterion is stale.","work_complaint":"The implementation looks risky."}"#,
        ] {
            assert_invalid(parse_document(document));
        }
    }

    #[test]
    fn parses_block_with_reason() {
        assert_eq!(
            parse_document(r#"{"disposition":"block","reason":"tests are failing"}"#),
            ReviewDisposition::Block {
                reason: "tests are failing".to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: "tests are failing".to_string(),
                steps: None,
                check: None,
                unstructured: true,
            }
        );
    }

    #[test]
    fn parses_all_remedy_owners() {
        for (owner, expected) in [
            ("agent", RemedyOwner::Agent),
            ("operator", RemedyOwner::Operator),
            ("world", RemedyOwner::World),
        ] {
            assert_eq!(
                parse_document(&format!(
                    r#"{{"disposition":"block","reason":"blocked","remedy_owner":"{owner}","ask":"Apply the remedy."}}"#
                )),
                ReviewDisposition::Block {
                    reason: "blocked".to_string(),
                    remedy_owner: expected,
                    ask: "Apply the remedy.".to_string(),
                    steps: None,
                    check: None,
                    unstructured: false,
                }
            );
        }
    }

    #[test]
    fn parses_optional_steps_and_check_without_normalizing_content() {
        assert_eq!(
            parse_document(
                r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Publish the release.","steps":["Run just release","  Verify the URL  "],"check":"curl -fsS https://example.test/release"}"#,
            ),
            ReviewDisposition::Block {
                reason: "release missing".to_string(),
                remedy_owner: RemedyOwner::World,
                ask: "Publish the release.".to_string(),
                steps: Some(vec![
                    "Run just release".to_string(),
                    "  Verify the URL  ".to_string(),
                ]),
                check: Some("curl -fsS https://example.test/release".to_string()),
                unstructured: false,
            }
        );
    }

    #[test]
    fn legacy_block_preserves_reason_bytes_in_reason_and_fallback_ask() {
        let reason = "  retain these bytes  ";
        assert_eq!(
            parse_document(&format!(
                r#"{{"disposition":"block","reason":{}}}"#,
                serde_json::to_string(reason).unwrap()
            )),
            ReviewDisposition::Block {
                reason: reason.to_string(),
                remedy_owner: RemedyOwner::Operator,
                ask: reason.to_string(),
                steps: None,
                check: None,
                unstructured: true,
            }
        );
    }

    #[test]
    fn missing_or_malformed_structure_uses_complete_operator_fallback() {
        let documents = [
            r#"{"disposition":"block","reason":"raw reason","ask":"Do it."}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent"}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"nobody","ask":"Do it."}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":7,"ask":"Do it."}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent","ask":"  "}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent","ask":7}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent","ask":"Do it.","steps":"one"}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent","ask":"Do it.","steps":["valid",7]}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"agent","ask":"Do it.","steps":["  "]}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"world","ask":"Do it.","check":7}"#,
            r#"{"disposition":"block","reason":"raw reason","remedy_owner":"world","ask":"Do it.","check":"  ","steps":["discard me"]}"#,
        ];

        for document in documents {
            assert_eq!(
                parse_document(document),
                ReviewDisposition::Block {
                    reason: "raw reason".to_string(),
                    remedy_owner: RemedyOwner::Operator,
                    ask: "raw reason".to_string(),
                    steps: None,
                    check: None,
                    unstructured: true,
                },
                "unexpected parse result for {document}"
            );
        }
    }

    #[test]
    fn check_content_is_never_executed_during_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let sentinel = dir.path().join("parser-must-not-create-this");
        let check = format!("touch {}", sentinel.display());
        let document = serde_json::json!({
            "disposition": "block",
            "reason": "wait for external state",
            "remedy_owner": "world",
            "ask": "Wait for the external state.",
            "check": check,
        });
        let path = dir.path().join("review-disposition.json");
        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();

        let parsed = parse_review_disposition(&path);

        assert!(matches!(
            parsed,
            ReviewDisposition::Block {
                remedy_owner: RemedyOwner::World,
                unstructured: false,
                ..
            }
        ));
        assert!(
            !sentinel.exists(),
            "parsing must store check content without executing it"
        );
    }

    #[test]
    fn missing_file_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        assert_invalid(parse_review_disposition(
            dir.path().join("review-disposition.json"),
        ));
    }

    #[test]
    fn malformed_json_is_invalid() {
        assert_invalid(parse_document("{this is not JSON"));
    }

    #[test]
    fn block_without_reason_is_invalid() {
        for document in [
            r#"{"disposition":"block"}"#,
            r#"{"disposition":"block","reason":null}"#,
            r#"{"disposition":"block","reason":""}"#,
            r#"{"disposition":"block","reason":"   "}"#,
        ] {
            assert_invalid(parse_document(document));
        }
    }

    #[test]
    fn pass_with_block_reason_is_invalid() {
        assert_invalid(parse_document(
            r#"{"disposition":"pass","reason":"tests are failing"}"#,
        ));
    }

    #[test]
    fn pass_without_reason_is_invalid() {
        assert_invalid(parse_document(r#"{"disposition":"pass"}"#));
    }

    #[test]
    fn unknown_disposition_is_invalid() {
        assert_invalid(parse_document(r#"{"disposition":"approve","reason":null}"#));
    }

    #[test]
    fn non_object_document_is_invalid() {
        assert_invalid(parse_document("[]"));
    }
}
