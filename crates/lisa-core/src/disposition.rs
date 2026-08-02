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

/// Who wrote a blocking disposition, and therefore what it is a statement about.
///
/// A reviewer's block is a judgement about the work. A block Lisa writes after
/// one of its own commands fails is a statement about a boundary that failed —
/// the work may be perfectly fine. The field operator read the second as the
/// first and went looking for what was wrong with twelve recipes; nothing was.
/// Keeping the two apart is a field, never a turn of phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DispositionOrigin {
    /// An agent's Review verdict on the work.
    #[default]
    Review,
    /// Lisa could not run one of its own commands to completion.
    InternalCommand,
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
    /// Completion is blocked with an actionable reason.
    Block {
        reason: String,
        remedy_owner: RemedyOwner,
        ask: String,
        steps: Option<Vec<String>>,
        check: Option<String>,
        /// True when missing or malformed remedy structure was replaced with
        /// the safe operator-owned legacy fallback.
        unstructured: bool,
        /// Whether this is a reviewer's verdict or a failed internal command.
        origin: DispositionOrigin,
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

/// Read and strictly validate a Review disposition as an authoring contract.
///
/// Unlike [`parse_review_disposition`], this entry point never coerces an
/// incomplete block into the legacy operator-owned fallback. It is intended
/// for the reviewer-side check that corrects newly written artifacts while
/// preserving the tolerant downstream parser for unchecked historical input.
pub fn check_review_disposition(path: impl AsRef<Path>) -> Result<ReviewDisposition, String> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let document: Value =
        serde_json::from_str(&contents).map_err(|error| format!("write valid JSON ({error})"))?;
    check_document(document)
}

fn check_document(document: Value) -> Result<ReviewDisposition, String> {
    let Value::Object(mut object) = document else {
        return Err("write one JSON object".to_string());
    };

    let disposition = take_non_empty_string(&mut object, "disposition", "review")?;
    match disposition.as_str() {
        "pass" => check_pass_document(&mut object),
        "note" => check_note_document(&mut object),
        "block" => check_block_document(&mut object),
        unknown => Err(format!(
            "change disposition {unknown:?} to \"pass\", \"note\", or \"block\""
        )),
    }
}

fn check_pass_document(
    object: &mut serde_json::Map<String, Value>,
) -> Result<ReviewDisposition, String> {
    match object.remove("reason") {
        Some(Value::Null) => {}
        _ => return Err("set a pass disposition's reason to null".to_string()),
    }
    if !object.is_empty() {
        return Err("keep pass exact: {\"disposition\":\"pass\",\"reason\":null}".to_string());
    }
    Ok(ReviewDisposition::Pass)
}

fn check_note_document(
    object: &mut serde_json::Map<String, Value>,
) -> Result<ReviewDisposition, String> {
    match object.remove("reason") {
        Some(Value::Null) => {}
        _ => return Err("set a note disposition's reason to null".to_string()),
    }

    let criterion_quote = take_non_empty_string(object, "criterion_quote", "note disposition")?;
    let evidence_citation = take_non_empty_string(object, "evidence_citation", "note disposition")?;
    let summary = take_non_empty_string(object, "summary", "note disposition")?;

    const COMPLAINT_FIELDS: &[&str] = &[
        "work_complaint",
        "complaint",
        "quality_complaint",
        "quality_concern",
        "work_concern",
    ];
    if COMPLAINT_FIELDS
        .iter()
        .any(|field| object.contains_key(*field))
    {
        return Err(
            "remove work-quality complaints from a note; use a block when the work itself needs changes"
                .to_string(),
        );
    }
    if !object.is_empty() {
        return Err(
            "remove extra note fields; keep only disposition, reason, criterion_quote, evidence_citation, and summary"
                .to_string(),
        );
    }

    let note = DispositionNote::new(criterion_quote, evidence_citation, summary)
        .expect("strict non-empty field checks must construct a note");
    Ok(ReviewDisposition::Note(note))
}

fn check_block_document(
    object: &mut serde_json::Map<String, Value>,
) -> Result<ReviewDisposition, String> {
    let reason = take_non_empty_string(object, "reason", "block disposition")?;
    let remedy_owner =
        match object.remove("remedy_owner") {
            Some(Value::String(owner)) if owner == "agent" => RemedyOwner::Agent,
            Some(Value::String(owner)) if owner == "operator" => RemedyOwner::Operator,
            Some(Value::String(owner)) if owner == "world" => RemedyOwner::World,
            _ => return Err(
                "set a block disposition's remedy_owner to \"agent\", \"operator\", or \"world\""
                    .to_string(),
            ),
        };
    let ask = take_non_empty_string(object, "ask", "block disposition")?;
    let steps = match object.remove("steps") {
        None => None,
        Some(Value::Array(values)) if !values.is_empty() => {
            let mut steps = Vec::with_capacity(values.len());
            for value in values {
                let Value::String(step) = value else {
                    return Err("make every block disposition step a non-empty string".to_string());
                };
                if step.trim().is_empty() {
                    return Err("make every block disposition step a non-empty string".to_string());
                }
                steps.push(step);
            }
            Some(steps)
        }
        Some(_) => return Err(
            "make block disposition steps a non-empty array of non-empty strings, or omit steps"
                .to_string(),
        ),
    };
    let check = match object.remove("check") {
        None => None,
        Some(Value::String(check)) if !check.trim().is_empty() => Some(check),
        Some(_) => {
            return Err(
                "make the block disposition check a non-empty read-only command, or omit check"
                    .to_string(),
            )
        }
    };
    if !object.is_empty() {
        return Err(
            "remove extra block fields; keep only disposition, reason, remedy_owner, ask, steps, and check"
                .to_string(),
        );
    }

    Ok(ReviewDisposition::Block {
        reason,
        remedy_owner,
        ask,
        steps,
        check,
        unstructured: false,
        // Authoring is a reviewer's act. `origin` is Lisa's to set, and the
        // extra-field rule above already refuses a document that claims it.
        origin: DispositionOrigin::Review,
    })
}

fn take_non_empty_string(
    object: &mut serde_json::Map<String, Value>,
    field: &str,
    class: &str,
) -> Result<String, String> {
    match object.remove(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!(
            "add a non-empty string {field} field to the {class}"
        )),
    }
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
    // Read origin before the structure closure so an unreadable value fails
    // toward Review — a document that cannot say it came from a failed command
    // is never granted that excuse.
    let origin = match object.remove("origin") {
        None => Some(DispositionOrigin::Review),
        Some(Value::String(value)) if value == "review" => Some(DispositionOrigin::Review),
        Some(Value::String(value)) if value == "internal-command" => {
            Some(DispositionOrigin::InternalCommand)
        }
        Some(_) => None,
    };
    let Some(origin) = origin else {
        return unstructured_block(reason);
    };
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
            origin,
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
        origin: DispositionOrigin::Review,
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

    fn check_authored_document(document: &str) -> Result<ReviewDisposition, String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review-disposition.json");
        fs::write(&path, document).unwrap();
        check_review_disposition(path)
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
                origin: DispositionOrigin::Review,
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
                    origin: DispositionOrigin::Review,
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
                origin: DispositionOrigin::Review,
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
                origin: DispositionOrigin::Review,
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
                    origin: DispositionOrigin::Review,
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

    #[test]
    fn strict_authoring_check_accepts_all_three_complete_classes() {
        assert_eq!(
            check_authored_document(r#"{"disposition":"pass","reason":null}"#).unwrap(),
            ReviewDisposition::Pass
        );

        assert!(matches!(
            check_authored_document(
                r#"{"disposition":"block","reason":"release missing","remedy_owner":"world","ask":"Publish the release.","steps":["Run just release"],"check":"test -f release"}"#,
            )
            .unwrap(),
            ReviewDisposition::Block {
                remedy_owner: RemedyOwner::World,
                unstructured: false,
                ..
            }
        ));

        assert!(matches!(
            check_authored_document(
                r#"{"disposition":"note","reason":null,"criterion_quote":"approximately 200 MiB","evidence_citation":"docs/evidence.md","summary":"The measurement supports completion while the criterion is stale."}"#,
            )
            .unwrap(),
            ReviewDisposition::Note(_)
        ));
    }

    #[test]
    fn strict_authoring_check_names_schema_fixes() {
        let cases = [
            ("{not json", "write valid JSON"),
            (
                r#"{"disposition":"pass","reason":null,"summary":"extra"}"#,
                "keep pass exact",
            ),
            (
                r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","summary":"The criterion is stale."}"#,
                "evidence_citation",
            ),
            (
                r#"{"disposition":"note","reason":null,"criterion_quote":"criterion","evidence_citation":"docs/evidence.md","summary":"The implementation is poor.","work_complaint":"Tests are failing."}"#,
                "remove work-quality complaints",
            ),
            (
                r#"{"disposition":"block","reason":"tests are failing"}"#,
                "remedy_owner",
            ),
            (
                r#"{"disposition":"block","reason":"tests are failing","remedy_owner":"agent","ask":"Fix it.","steps":[]}"#,
                "steps a non-empty array",
            ),
            (
                r#"{"disposition":"block","reason":"tests are failing","remedy_owner":"agent","ask":"Fix it.","surprise":true}"#,
                "remove extra block fields",
            ),
        ];

        for (document, expected_fix) in cases {
            let error = check_authored_document(document).unwrap_err();
            assert!(
                error.contains(expected_fix),
                "{error:?} did not name fix {expected_fix:?} for {document}"
            );
        }
    }

    #[test]
    fn strict_check_rejects_but_fallback_parser_preserves_legacy_block() {
        let document = r#"{"disposition":"block","reason":"Run the old test."}"#;

        assert!(check_authored_document(document)
            .unwrap_err()
            .contains("remedy_owner"));
        assert!(matches!(
            parse_document(document),
            ReviewDisposition::Block {
                unstructured: true,
                ..
            }
        ));
    }

    #[test]
    fn a_recording_failure_and_a_reviewers_verdict_are_separable_by_field() {
        let reviewer = r#"{"disposition":"block","reason":"the new test fails","remedy_owner":"agent","ask":"Fix the failing test."}"#;
        let recording = r#"{"disposition":"block","origin":"internal-command","reason":"Lisa could not record T-002-05's finished work.","remedy_owner":"operator","ask":"Run `lisa already-done T-002-05` if this ticket's work is already saved in history."}"#;

        assert!(matches!(
            parse_document(reviewer),
            ReviewDisposition::Block {
                origin: DispositionOrigin::Review,
                ..
            }
        ));
        assert!(matches!(
            parse_document(recording),
            ReviewDisposition::Block {
                origin: DispositionOrigin::InternalCommand,
                ..
            }
        ));
        // An explicit "review" is the same fact as saying nothing.
        assert_eq!(
            parse_document(
                r#"{"disposition":"block","origin":"review","reason":"the new test fails","remedy_owner":"agent","ask":"Fix the failing test."}"#
            ),
            parse_document(reviewer)
        );
    }

    #[test]
    fn an_unreadable_origin_never_claims_to_be_a_failed_command() {
        for document in [
            r#"{"disposition":"block","origin":"machine","reason":"raw reason","remedy_owner":"operator","ask":"Do it."}"#,
            r#"{"disposition":"block","origin":7,"reason":"raw reason","remedy_owner":"operator","ask":"Do it."}"#,
            r#"{"disposition":"block","origin":null,"reason":"raw reason","remedy_owner":"operator","ask":"Do it."}"#,
        ] {
            assert_eq!(
                parse_document(document),
                ReviewDisposition::Block {
                    reason: "raw reason".to_string(),
                    remedy_owner: RemedyOwner::Operator,
                    ask: "raw reason".to_string(),
                    steps: None,
                    check: None,
                    unstructured: true,
                    origin: DispositionOrigin::Review,
                },
                "unexpected parse result for {document}"
            );
        }
    }

    #[test]
    fn a_reviewer_cannot_author_the_origin_field() {
        // Origin is Lisa's to set. The strict authoring check's extra-field
        // rule is what keeps an agent from writing "this was a transport
        // failure" over its own verdict.
        let error = check_authored_document(
            r#"{"disposition":"block","origin":"internal-command","reason":"tests are failing","remedy_owner":"agent","ask":"Fix it."}"#,
        )
        .unwrap_err();

        assert!(error.contains("remove extra block fields"), "{error}");
    }
}
