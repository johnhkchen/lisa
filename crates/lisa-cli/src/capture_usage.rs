//! Native-TUI token capture for the provenance ledger.
//!
//! Both native clients expose a **transcript JSONL** at the `transcript_path`
//! named by their Stop-hook payload. The headless Codex fallback separately
//! captures usage from its JSON event stream.
//!
//! `lisa capture-usage` is invoked by both native TUI `Stop` hooks. Claude
//! transcripts carry per-assistant-message usage that must be summed. Codex
//! rollouts carry cumulative `event_msg/token_count` records, so the last
//! `total_token_usage` record wins. `LISA_AGENT_CLIENT=codex` selects the latter
//! parser and `.lisa/codex/`; absence preserves the Claude behavior.
//!
//! Write-after and never-fabricate: Stop fires at *turn* boundaries (not per
//! tool call — the heartbeat hook stays trivial). Each successful observation is
//! appended to `.lisa/<client>/captures.jsonl` with its pane, provider session,
//! capture time, and totals. Ticket attribution is deliberately deferred to the
//! scheduler, which has the pane-ownership history the hook process lacks. Every
//! missing input (no `transcript_path`, unreadable transcript, malformed lines,
//! no assistant messages) currently writes nothing and returns `Ok(())`.

use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

use lisa_core::capture::{append_capture_record, CaptureRecord};
use lisa_core::provenance::system_time_to_epoch;
use serde::Deserialize;
use serde_json::Value;

/// Facts supplied by the native client's Stop-hook payload.
#[derive(Debug, Deserialize)]
struct StopPayload {
    #[serde(default)]
    transcript_path: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

/// Summed, provider-native totals in the shape the plugin reader expects.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct UsageTotals {
    /// All input-side tokens: fresh + cache-creation + cache-read (design Q3).
    input_tokens: u64,
    output_tokens: u64,
}

/// Sum `message.usage` over every assistant line of a transcript JSONL.
///
/// Defensive against external drift: a non-parseable or non-assistant line is
/// skipped, an absent token field counts as 0. Drift degrades to an under-count,
/// never a crash or a fabricated value.
fn sum_claude_transcript_usage(jsonl: &str) -> UsageTotals {
    let mut total = UsageTotals::default();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        // Assistant messages carry the billed usage; other entry types do not.
        if event.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(usage) = event.get("message").and_then(|m| m.get("usage")) else {
            continue;
        };
        let field = |name: &str| usage.get(name).and_then(Value::as_u64).unwrap_or(0);
        total.input_tokens += field("input_tokens")
            + field("cache_creation_input_tokens")
            + field("cache_read_input_tokens");
        total.output_tokens += field("output_tokens");
    }
    total
}

/// Read the last cumulative token total from a Codex rollout transcript.
fn codex_transcript_usage(jsonl: &str) -> UsageTotals {
    let mut latest = UsageTotals::default();
    for line in jsonl.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("event_msg")
            || event.pointer("/payload/type").and_then(Value::as_str) != Some("token_count")
        {
            continue;
        }
        let Some(usage) = event.pointer("/payload/info/total_token_usage") else {
            continue;
        };
        latest = UsageTotals {
            input_tokens: usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        };
    }
    latest
}

/// Read the Stop-hook payload from stdin, sum the transcript's usage, and write
/// one append-only `.lisa/<client>/captures.jsonl` row under `cwd`. Best-effort
/// throughout: any absent input returns `Ok(())` writing nothing.
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()> {
    let mut stdin = String::new();
    if std::io::stdin().read_to_string(&mut stdin).is_err() {
        return Ok(());
    }
    let payload: StopPayload = match serde_json::from_str(&stdin) {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };
    let Some(transcript_path) = payload.transcript_path else {
        return Ok(());
    };
    let Some(session_id) = payload.session_id.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(pane_id) = std::env::var("LISA_PANE_ID")
        .ok()
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return Ok(());
    };
    let jsonl = match std::fs::read_to_string(&transcript_path) {
        Ok(s) => s,
        Err(_) => return Ok(()), // transcript gone/unreadable → leave tokens null
    };
    let is_codex = std::env::var("LISA_AGENT_CLIENT").is_ok_and(|v| v == "codex");
    let usage = if is_codex {
        codex_transcript_usage(&jsonl)
    } else {
        sum_claude_transcript_usage(&jsonl)
    };
    // Nothing observed → do not write a zero-token artifact (never fabricate a
    // "we measured 0" where we actually measured nothing).
    if usage == UsageTotals::default() {
        return Ok(());
    }
    let client_dir = cwd
        .join(".lisa")
        .join(if is_codex { "codex" } else { "claude" });
    append_capture_record(
        &client_dir.join("captures.jsonl"),
        &CaptureRecord {
            pane_id,
            session_id,
            captured_at: system_time_to_epoch(SystemTime::now()),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSCRIPT: &str = r#"{"type":"user","message":{"role":"user","content":"hi"}}
{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":10,"cache_creation_input_tokens":2,"cache_read_input_tokens":100,"output_tokens":30}}}
{"type":"system","subtype":"info"}
{"type":"assistant","message":{"role":"assistant","usage":{"input_tokens":5,"cache_read_input_tokens":50,"output_tokens":7}}}
"#;

    #[test]
    fn sums_all_input_classes_and_output() {
        let u = sum_claude_transcript_usage(TRANSCRIPT);
        // input: (10+2+100) + (5+0+50) = 167 ; output: 30 + 7 = 37
        assert_eq!(u.input_tokens, 167);
        assert_eq!(u.output_tokens, 37);
    }

    #[test]
    fn skips_malformed_and_non_assistant_lines() {
        let jsonl = "not json\n{\"type\":\"user\"}\n{\"type\":\"assistant\",\"message\":{\"usage\":{\"input_tokens\":4,\"output_tokens\":1}}}\n";
        let u = sum_claude_transcript_usage(jsonl);
        assert_eq!(u.input_tokens, 4);
        assert_eq!(u.output_tokens, 1);
    }

    #[test]
    fn empty_or_no_assistant_is_zero() {
        assert_eq!(sum_claude_transcript_usage(""), UsageTotals::default());
        assert_eq!(
            sum_claude_transcript_usage("{\"type\":\"user\",\"message\":{}}\n"),
            UsageTotals::default()
        );
    }

    #[test]
    fn missing_token_fields_count_as_zero_not_crash() {
        let jsonl = "{\"type\":\"assistant\",\"message\":{\"usage\":{\"output_tokens\":9}}}\n";
        let u = sum_claude_transcript_usage(jsonl);
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 9);
    }

    #[test]
    fn codex_uses_latest_cumulative_token_count() {
        let jsonl = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"output_tokens":2}}}}
{"type":"event_msg","payload":{"type":"agent_message","message":"ignored"}}
{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":25,"cached_input_tokens":8,"output_tokens":7}}}}"#;
        assert_eq!(
            codex_transcript_usage(jsonl),
            UsageTotals {
                input_tokens: 25,
                output_tokens: 7,
            }
        );
    }
}
