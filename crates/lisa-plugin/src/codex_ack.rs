//! Ticket-scoped acknowledgment detection for native Codex assignments.
//!
//! Codex reports a submitted prompt through its `UserPromptSubmit` lifecycle
//! hook. Lisa adds an opaque assignment marker to that prompt, then compares
//! the lifecycle payload with the ticket and assignment generation currently
//! pending in the scheduler. This module deliberately has no scheduler, hook
//! transport, terminal-rendering, or Claude handshake dependencies.

use serde::{Deserialize, Serialize};

const ASSIGNMENT_PREFIX: &str = "LISA_ASSIGNMENT ";

/// Identity Lisa assigns before submitting work to Codex.
///
/// A ticket may be delivered more than once, so ticket identity alone is not
/// sufficient to reject delayed lifecycle events from an older attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodexAssignmentRef<'a> {
    pub ticket_id: &'a str,
    pub generation: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct AssignmentMarker {
    ticket_id: String,
    generation: u64,
}

#[derive(Debug, Deserialize)]
struct LifecycleEvent {
    hook_event_name: String,
    prompt: Option<String>,
}

/// Append the canonical Lisa assignment marker on its own prompt line.
///
/// Serialization, rather than string interpolation, keeps arbitrary ticket IDs
/// unambiguous. Generation allocation belongs to the scheduler integration.
pub(crate) fn tag_codex_assignment(prompt: &str, assignment: CodexAssignmentRef<'_>) -> String {
    let marker = AssignmentMarker {
        ticket_id: assignment.ticket_id.to_string(),
        generation: assignment.generation,
    };
    let encoded = serde_json::to_string(&marker).expect("assignment marker is serializable");
    let separator = if prompt.ends_with('\n') { "" } else { "\n" };
    format!("{prompt}{separator}{ASSIGNMENT_PREFIX}{encoded}")
}

/// Return true only for Codex lifecycle evidence matching the pending assignment.
///
/// Invalid JSON, non-submit events, missing prompts, malformed markers, and
/// ticket/generation mismatches all fail closed as `false`.
pub(crate) fn detect_codex_ack(payload_json: &str, pending: CodexAssignmentRef<'_>) -> bool {
    let Ok(event) = serde_json::from_str::<LifecycleEvent>(payload_json) else {
        return false;
    };
    if event.hook_event_name != "UserPromptSubmit" {
        return false;
    }
    let Some(prompt) = event.prompt else {
        return false;
    };

    prompt.lines().any(|line| {
        let Some(encoded) = line.strip_prefix(ASSIGNMENT_PREFIX) else {
            return false;
        };
        let Ok(marker) = serde_json::from_str::<AssignmentMarker>(encoded) else {
            return false;
        };
        marker.ticket_id == pending.ticket_id && marker.generation == pending.generation
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATCHING: &str = include_str!("../tests/fixtures/codex_ack/matching-prompt-submit.json");
    const STILL_IDLE: &str = include_str!("../tests/fixtures/codex_ack/still-idle-clear.json");
    const STALE_TICKET: &str =
        include_str!("../tests/fixtures/codex_ack/stale-previous-ticket.json");
    const STALE_GENERATION: &str =
        include_str!("../tests/fixtures/codex_ack/stale-previous-generation.json");

    fn pending() -> CodexAssignmentRef<'static> {
        CodexAssignmentRef {
            ticket_id: "T-033-01-02",
            generation: 42,
        }
    }

    #[test]
    fn matching_fixture_acknowledges_pending_assignment() {
        assert!(detect_codex_ack(MATCHING, pending()));
    }

    #[test]
    fn clear_fixture_is_not_acknowledgment() {
        assert!(!detect_codex_ack(STILL_IDLE, pending()));
    }

    #[test]
    fn previous_ticket_fixture_is_stale() {
        assert!(!detect_codex_ack(STALE_TICKET, pending()));
    }

    #[test]
    fn previous_generation_fixture_is_stale() {
        assert!(!detect_codex_ack(STALE_GENERATION, pending()));
    }

    #[test]
    fn malformed_payload_fails_closed() {
        assert!(!detect_codex_ack("{not-json", pending()));
        assert!(!detect_codex_ack(r#"{"hook_event_name":42}"#, pending()));
    }

    #[test]
    fn matching_marker_on_another_event_is_not_acknowledgment() {
        let tagged = tag_codex_assignment("new work", pending());
        let payload = serde_json::json!({
            "hook_event_name": "Stop",
            "prompt": tagged,
            "turn_id": "turn-current"
        });
        assert!(!detect_codex_ack(&payload.to_string(), pending()));
    }

    #[test]
    fn marker_must_start_its_own_prompt_line() {
        let marker = serde_json::to_string(&AssignmentMarker {
            ticket_id: pending().ticket_id.to_string(),
            generation: pending().generation,
        })
        .unwrap();
        let payload = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": format!("prose before {ASSIGNMENT_PREFIX}{marker}"),
        });
        assert!(!detect_codex_ack(&payload.to_string(), pending()));
    }

    #[test]
    fn marker_in_an_unrelated_field_is_not_acknowledgment() {
        let tagged = tag_codex_assignment("new work", pending());
        let payload = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "last_assistant_message": tagged,
        });
        assert!(!detect_codex_ack(&payload.to_string(), pending()));
    }

    #[test]
    fn tag_round_trips_json_sensitive_ticket_id_and_extra_event_fields() {
        let assignment = CodexAssignmentRef {
            ticket_id: "T-quoted-\"slash\\",
            generation: u64::MAX,
        };
        let tagged = tag_codex_assignment("do the ticket", assignment);
        assert!(tagged.starts_with("do the ticket\nLISA_ASSIGNMENT "));

        let payload = serde_json::json!({
            "session_id": "session-current",
            "turn_id": "turn-current",
            "hook_event_name": "UserPromptSubmit",
            "prompt": tagged,
            "future_codex_field": { "ignored": true },
        });
        assert!(detect_codex_ack(&payload.to_string(), assignment));
    }
}
