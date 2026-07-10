//! Claude-side token capture for the provenance ledger (T-027-02).
//!
//! The native analogue of the Codex wrapper's usage capture
//! ([`crate::agent_exec`]'s `persist_run_artifacts`). Claude Code has no
//! `codex exec --json` stream to read; the only per-session usage surface is the
//! **transcript JSONL** at `transcript_path`, which the Stop-hook payload names.
//!
//! `lisa capture-usage` is invoked by the Claude `Stop` hook with that payload on
//! stdin. It reads `transcript_path`, sums the per-message `message.usage` across
//! the transcript, and writes `.lisa/claude/<key>.usage.json` in the same nested
//! `{ ..., usage: { input_tokens, output_tokens } }` shape the plugin's reader
//! and [`lisa_core::provenance::extract_usage`] already consume — so the Claude
//! path reuses the Codex reader spine verbatim (T-027-02 design).
//!
//! Write-after and never-fabricate: Stop fires at *turn* boundaries (not per
//! tool call — the heartbeat hook stays trivial), and the artifact is overwritten
//! each Stop with the cumulative transcript total. The plugin reads it only at
//! terminal teardown, so last-write-wins is the final total and nothing races the
//! agent-owned frontmatter. Every missing input (no `transcript_path`, unreadable
//! transcript, malformed lines, no assistant messages) writes nothing and returns
//! `Ok(())` — tokens stay `null`, never guessed.

use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

/// The Stop-hook stdin payload. Only `transcript_path` is required; everything
/// else Claude Code sends (session_id, cwd, …) is ignored.
#[derive(Debug, Deserialize)]
struct StopPayload {
    #[serde(default)]
    transcript_path: Option<String>,
}

/// Summed, provider-native totals in the shape the plugin reader expects.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ClaudeUsage {
    /// All input-side tokens: fresh + cache-creation + cache-read (design Q3).
    input_tokens: u64,
    output_tokens: u64,
}

/// Sum `message.usage` over every assistant line of a transcript JSONL.
///
/// Defensive against external drift: a non-parseable or non-assistant line is
/// skipped, an absent token field counts as 0. Drift degrades to an under-count,
/// never a crash or a fabricated value.
fn sum_transcript_usage(jsonl: &str) -> ClaudeUsage {
    let mut total = ClaudeUsage::default();
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

/// Build the artifact JSON `{ key, usage: { input_tokens, output_tokens } }` —
/// the same nested-`usage` shape `provenance::extract_usage` reads.
fn usage_artifact(key: &str, u: &ClaudeUsage) -> Value {
    serde_json::json!({
        "key": key,
        "usage": {
            "input_tokens": u.input_tokens,
            "output_tokens": u.output_tokens,
        },
    })
}

/// Resolve the artifact key the way the Codex wrapper does
/// (`LISA_TICKET_ID` → `pane-<LISA_PANE_ID>` → `"last"`), so the plugin's reader
/// finds `<ticket>.usage.json` under `.lisa/claude` exactly as under `.lisa/codex`.
fn resolve_key() -> String {
    let ticket = std::env::var("LISA_TICKET_ID")
        .ok()
        .filter(|s| !s.is_empty());
    let pane = std::env::var("LISA_PANE_ID").ok().filter(|s| !s.is_empty());
    ticket
        .or_else(|| pane.map(|p| format!("pane-{}", p)))
        .unwrap_or_else(|| "last".to_string())
}

/// Read the Stop-hook payload from stdin, sum the transcript's usage, and write
/// `.lisa/claude/<key>.usage.json` under `cwd`. Best-effort throughout: any
/// absent input returns `Ok(())` writing nothing.
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
    let jsonl = match std::fs::read_to_string(&transcript_path) {
        Ok(s) => s,
        Err(_) => return Ok(()), // transcript gone/unreadable → leave tokens null
    };
    let usage = sum_transcript_usage(&jsonl);
    // Nothing observed → do not write a zero-token artifact (never fabricate a
    // "we measured 0" where we actually measured nothing).
    if usage == ClaudeUsage::default() {
        return Ok(());
    }
    let key = resolve_key();
    let claude_dir = cwd.join(".lisa").join("claude");
    std::fs::create_dir_all(&claude_dir)?;
    let artifact = usage_artifact(&key, &usage);
    std::fs::write(
        claude_dir.join(format!("{}.usage.json", key)),
        serde_json::to_string_pretty(&artifact).unwrap_or_else(|_| "{}".to_string()),
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
        let u = sum_transcript_usage(TRANSCRIPT);
        // input: (10+2+100) + (5+0+50) = 167 ; output: 30 + 7 = 37
        assert_eq!(u.input_tokens, 167);
        assert_eq!(u.output_tokens, 37);
    }

    #[test]
    fn skips_malformed_and_non_assistant_lines() {
        let jsonl = "not json\n{\"type\":\"user\"}\n{\"type\":\"assistant\",\"message\":{\"usage\":{\"input_tokens\":4,\"output_tokens\":1}}}\n";
        let u = sum_transcript_usage(jsonl);
        assert_eq!(u.input_tokens, 4);
        assert_eq!(u.output_tokens, 1);
    }

    #[test]
    fn empty_or_no_assistant_is_zero() {
        assert_eq!(sum_transcript_usage(""), ClaudeUsage::default());
        assert_eq!(
            sum_transcript_usage("{\"type\":\"user\",\"message\":{}}\n"),
            ClaudeUsage::default()
        );
    }

    #[test]
    fn missing_token_fields_count_as_zero_not_crash() {
        let jsonl = "{\"type\":\"assistant\",\"message\":{\"usage\":{\"output_tokens\":9}}}\n";
        let u = sum_transcript_usage(jsonl);
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 9);
    }

    #[test]
    fn artifact_shape_matches_extract_usage() {
        // The artifact's nested `usage` must be readable by the shared extractor
        // the plugin uses — this is the cross-crate contract.
        let u = ClaudeUsage {
            input_tokens: 167,
            output_tokens: 37,
        };
        let artifact = usage_artifact("T-027-02", &u);
        let usage = artifact.get("usage").unwrap();
        let (tin, tout, cost) = lisa_core::provenance::extract_usage(usage);
        assert_eq!(tin, Some(167));
        assert_eq!(tout, Some(37));
        assert_eq!(cost, None); // Claude records no dollar cost (design Q3)
        assert_eq!(artifact.get("key").unwrap(), "T-027-02");
    }
}
