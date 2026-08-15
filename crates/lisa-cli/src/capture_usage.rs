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
//! capture time, totals, client, and — read straight from the same transcript,
//! never from a pane's current config — the model and effort the turn ran
//! under. Reading it from the transcript rather than joining it later means the
//! record stays right even if the pane is reconfigured mid-run: it says what
//! that turn actually ran, not what the pane runs now. A transcript that omits
//! either leaves it `None`, not a guess. An identified Stop with a missing,
//! unreadable, or empty transcript appends its reason to `no-captures.jsonl`
//! instead. Ticket attribution is deliberately deferred to the scheduler, which
//! has the pane-ownership history the hook process lacks.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;

use lisa_core::capture::{append_capture_record, CaptureRecord};
use lisa_core::provenance::system_time_to_epoch;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MISSING_TRANSCRIPT_PATH: &str = "missing-transcript-path";
const UNREADABLE_TRANSCRIPT: &str = "unreadable-transcript";
const EMPTY_TRANSCRIPT: &str = "empty-transcript";

/// Facts supplied by the native client's Stop-hook payload.
#[derive(Debug, Deserialize)]
struct StopPayload {
    #[serde(default)]
    transcript_path: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

/// One identified Stop whose transcript usage could not be observed.
#[derive(Debug, Serialize)]
struct NoCaptureMarker<'a> {
    pane_id: u32,
    session_id: &'a str,
    captured_at: u64,
    reason: &'static str,
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

/// What ran the turn, when the transcript says so. Never a guess: an absent
/// field stays `None` rather than inheriting today's config.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ModelAttribution {
    model: Option<String>,
    effort: Option<String>,
}

/// Read the model and effort a Claude transcript's turn ran under.
///
/// Each assistant line carries `message.model` and a top-level `effort`
/// alongside the usage this hook already sums. A turn's assistant lines all
/// ran under the same model/effort in practice, so the latest non-empty
/// sighting of each is taken — the same "last one wins" rule the usage sum
/// and the Codex cumulative reader already use.
fn claude_transcript_attribution(jsonl: &str) -> ModelAttribution {
    let mut attribution = ModelAttribution::default();
    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        if let Some(model) = event
            .pointer("/message/model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            attribution.model = Some(model.to_string());
        }
        if let Some(effort) = event
            .get("effort")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            attribution.effort = Some(effort.to_string());
        }
    }
    attribution
}

/// Read the model and effort a Codex rollout's most recent turn ran under.
///
/// `turn_context` events carry `payload.model` and `payload.effort` (or, on
/// clients that only nest it under collaboration mode,
/// `payload.collaboration_mode.settings.reasoning_effort`). The rollout
/// accumulates every turn in the session, so the latest `turn_context` is the
/// one that describes the turn this Stop just observed.
fn codex_transcript_attribution(jsonl: &str) -> ModelAttribution {
    let mut attribution = ModelAttribution::default();
    for line in jsonl.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("turn_context") {
            continue;
        }
        let Some(payload) = event.get("payload") else {
            continue;
        };
        if let Some(model) = payload
            .get("model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            attribution.model = Some(model.to_string());
        }
        let effort = payload
            .get("effort")
            .and_then(Value::as_str)
            .or_else(|| {
                payload
                    .pointer("/collaboration_mode/settings/reasoning_effort")
                    .and_then(Value::as_str)
            })
            .filter(|s| !s.is_empty());
        if let Some(effort) = effort {
            attribution.effort = Some(effort.to_string());
        }
    }
    attribution
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

/// Append one durable no-capture marker and make it immediately visible.
fn append_no_capture_marker(
    client_dir: &Path,
    pane_id: u32,
    session_id: &str,
    captured_at: u64,
    reason: &'static str,
    diagnostics: &mut impl Write,
) -> std::io::Result<()> {
    let marker = NoCaptureMarker {
        pane_id,
        session_id,
        captured_at,
        reason,
    };
    let mut line = serde_json::to_string(&marker)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    line.push('\n');

    fs::create_dir_all(client_dir)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(client_dir.join("no-captures.jsonl"))?;
    file.write_all(line.as_bytes())?;

    writeln!(
        diagnostics,
        "lisa capture-usage: no capture for pane {pane_id} session {session_id}: {reason}"
    )?;
    Ok(())
}

fn capture_usage_from(
    cwd: &Path,
    mut input: impl Read,
    is_codex: bool,
    pane_id: Option<&str>,
    captured_at: u64,
    diagnostics: &mut impl Write,
) -> std::io::Result<()> {
    let mut stdin = String::new();
    input.read_to_string(&mut stdin).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("could not read Stop payload: {error}"),
        )
    })?;
    let payload: StopPayload = serde_json::from_str(&stdin).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid Stop payload: {error}"),
        )
    })?;
    let Some(session_id) = payload.session_id.filter(|value| !value.is_empty()) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Stop payload is missing session_id",
        ));
    };
    let Some(pane_id) = pane_id
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "LISA_PANE_ID is missing or invalid",
        ));
    };
    let client_dir = cwd
        .join(".lisa")
        .join(if is_codex { "codex" } else { "claude" });
    let Some(transcript_path) = payload.transcript_path.filter(|value| !value.is_empty()) else {
        return append_no_capture_marker(
            &client_dir,
            pane_id,
            &session_id,
            captured_at,
            MISSING_TRANSCRIPT_PATH,
            diagnostics,
        );
    };
    let jsonl = match std::fs::read_to_string(&transcript_path) {
        Ok(s) => s,
        Err(_) => {
            return append_no_capture_marker(
                &client_dir,
                pane_id,
                &session_id,
                captured_at,
                UNREADABLE_TRANSCRIPT,
                diagnostics,
            );
        }
    };
    let (usage, attribution) = if is_codex {
        (
            codex_transcript_usage(&jsonl),
            codex_transcript_attribution(&jsonl),
        )
    } else {
        (
            sum_claude_transcript_usage(&jsonl),
            claude_transcript_attribution(&jsonl),
        )
    };
    // Nothing observed → record why no totals were written rather than
    // fabricating a measured zero or silently dropping the Stop.
    if usage == UsageTotals::default() {
        return append_no_capture_marker(
            &client_dir,
            pane_id,
            &session_id,
            captured_at,
            EMPTY_TRANSCRIPT,
            diagnostics,
        );
    }
    append_capture_record(
        &client_dir.join("captures.jsonl"),
        &CaptureRecord {
            pane_id,
            session_id,
            captured_at,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            client: if is_codex { "codex" } else { "claude" }.to_string(),
            model: attribution.model,
            effort: attribution.effort,
        },
    )
}

/// Read the Stop-hook payload from stdin, sum the transcript's usage, and write
/// one append-only outcome row under `cwd`.
///
/// Successful observations go to `.lisa/<client>/captures.jsonl`; identified
/// Stops without observable usage go to `.lisa/<client>/no-captures.jsonl`.
/// Missing capture identity and persistence failures are returned to the caller.
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()> {
    let is_codex = std::env::var("LISA_AGENT_CLIENT").is_ok_and(|v| v == "codex");
    let pane_id = std::env::var("LISA_PANE_ID").ok();
    let stdin = std::io::stdin();
    let stderr = std::io::stderr();
    capture_usage_from(
        cwd,
        stdin.lock(),
        is_codex,
        pane_id.as_deref(),
        system_time_to_epoch(SystemTime::now()),
        &mut stderr.lock(),
    )
}

/// Deterministic access to the native Stop processor for cross-crate regression
/// tests. This is intentionally unavailable without the `test-support` feature.
#[cfg(feature = "test-support")]
#[doc(hidden)]
#[allow(dead_code)]
pub fn run_capture_usage_for_test(
    cwd: &Path,
    input: impl Read,
    is_codex: bool,
    pane_id: u32,
    captured_at: u64,
    diagnostics: &mut impl Write,
) -> std::io::Result<()> {
    let pane_id = pane_id.to_string();
    capture_usage_from(
        cwd,
        input,
        is_codex,
        Some(&pane_id),
        captured_at,
        diagnostics,
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

    // --- model / effort attribution -----------------------------------------

    #[test]
    fn claude_reads_model_and_effort_from_the_assistant_line() {
        let jsonl = r#"{"type":"assistant","effort":"high","message":{"model":"claude-opus-5","usage":{"input_tokens":1,"output_tokens":1}}}"#;
        let a = claude_transcript_attribution(jsonl);
        assert_eq!(a.model.as_deref(), Some("claude-opus-5"));
        assert_eq!(a.effort.as_deref(), Some("high"));
    }

    #[test]
    fn claude_attribution_takes_the_latest_assistant_line() {
        let jsonl = r#"{"type":"assistant","effort":"high","message":{"model":"claude-opus-5","usage":{"input_tokens":1,"output_tokens":1}}}
{"type":"assistant","effort":"medium","message":{"model":"claude-sonnet-5","usage":{"input_tokens":1,"output_tokens":1}}}"#;
        let a = claude_transcript_attribution(jsonl);
        assert_eq!(a.model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(a.effort.as_deref(), Some("medium"));
    }

    #[test]
    fn claude_attribution_is_unknown_never_guessed_when_absent() {
        let jsonl = "{\"type\":\"assistant\",\"message\":{\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n";
        let a = claude_transcript_attribution(jsonl);
        assert_eq!(a.model, None);
        assert_eq!(a.effort, None);
        // Malformed/non-assistant lines are skipped like the usage reader.
        assert_eq!(
            claude_transcript_attribution(""),
            ModelAttribution::default()
        );
        assert_eq!(
            claude_transcript_attribution("not json\n{\"type\":\"user\"}\n"),
            ModelAttribution::default()
        );
    }

    #[test]
    fn codex_reads_model_and_effort_from_turn_context() {
        let jsonl = r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"xhigh"}}"#;
        let a = codex_transcript_attribution(jsonl);
        assert_eq!(a.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(a.effort.as_deref(), Some("xhigh"));
    }

    #[test]
    fn codex_effort_falls_back_to_collaboration_mode_reasoning_effort() {
        let jsonl = r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol","collaboration_mode":{"settings":{"reasoning_effort":"xhigh"}}}}"#;
        let a = codex_transcript_attribution(jsonl);
        assert_eq!(a.model.as_deref(), Some("gpt-5.6-sol"));
        assert_eq!(a.effort.as_deref(), Some("xhigh"));
    }

    #[test]
    fn codex_attribution_takes_the_latest_turn_context() {
        let jsonl = r#"{"type":"turn_context","payload":{"model":"gpt-5.6-sol","effort":"xhigh"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"ignored"}}
{"type":"turn_context","payload":{"model":"gpt-5-mini","effort":"low"}}"#;
        let a = codex_transcript_attribution(jsonl);
        assert_eq!(a.model.as_deref(), Some("gpt-5-mini"));
        assert_eq!(a.effort.as_deref(), Some("low"));
    }

    #[test]
    fn codex_attribution_is_unknown_never_guessed_when_absent() {
        let jsonl = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1,"output_tokens":1}}}}"#;
        let a = codex_transcript_attribution(jsonl);
        assert_eq!(a.model, None);
        assert_eq!(a.effort, None);
    }
}
