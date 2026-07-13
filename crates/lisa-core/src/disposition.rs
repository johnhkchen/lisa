//! Fail-closed parsing for the machine-readable Review disposition.
//!
//! A disposition file grants completion authority only when it contains the
//! exact valid pass relationship. Missing, unreadable, malformed, unknown, or
//! contradictory input is represented as [`ReviewDisposition::Invalid`] so a
//! caller cannot confuse parser failure with agent approval.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// The validated outcome of a Review disposition file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewDisposition {
    /// The agent explicitly declared that Review passed.
    Pass,
    /// The agent blocked completion with an actionable reason.
    Block { reason: String },
    /// The file could not be trusted as either valid disposition.
    Invalid { reason: String },
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
        ("block", Value::String(reason)) if !reason.trim().is_empty() => {
            ReviewDisposition::Block { reason }
        }
        ("block", _) => {
            invalid("a blocking review disposition must have a non-empty string reason")
        }
        (unknown, _) => invalid(format!(
            "unknown review disposition {unknown:?}; expected \"pass\" or \"block\""
        )),
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
    fn parses_block_with_reason() {
        assert_eq!(
            parse_document(r#"{"disposition":"block","reason":"tests are failing"}"#),
            ReviewDisposition::Block {
                reason: "tests are failing".to_string(),
            }
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
