//! Execution-provenance ledger: append-only JSONL learning data.
//!
//! One [`ProvenanceRecord`] is appended to `.lisa/provenance.jsonl` per
//! completed ticket-run, written by the plugin *after* the run ends
//! (write-after; it never races the agent and never touches the agent-owned
//! ticket frontmatter — epic E-001 Decision 2). `.lisa/` gitignores only
//! `signals/`, so the ledger is committable, queryable-across-runs data.
//!
//! This module owns the schema and the `std::fs` append so both writers (the
//! plugin today, a future `lisa` query command) share one definition — the same
//! "one place both readers share" reasoning as [`crate::client`]. It knows
//! nothing of the scheduler; the plugin decides *when* to emit and reads the
//! per-provider usage artifact (`.lisa/codex/<t>.usage.json` from the Codex
//! wrapper, `.lisa/claude/<t>.usage.json` from the Claude Stop hook's
//! `capture-usage`, T-027-02), then hands a finished record here to write.
//!
//! The field table and jq/duckdb query examples live in
//! [`docs/knowledge/provenance-ledger.md`](../../../docs/knowledge/provenance-ledger.md).

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::AgentClient;

/// Schema version stamped on every record. Bump when the record shape changes so
/// readers can branch (e.g. T-027-02 cost fidelity, S-026 routing splitting
/// `requested` from `actual`).
pub const SCHEMA_VERSION: u32 = 1;

/// The `(method, provider, model)` a run resolved to. `model` is `None` until
/// model selection lands (S-026); `provider` is derived from the client. Today
/// a record's `requested` and `actual` routes are identical (requested == actual
/// until per-pane routing, T-026-01); both fields exist from day one so the
/// schema does not churn when routing splits them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// Integration method / client name: `"claude"` | `"codex"`.
    pub method: String,
    /// Vendor: `"anthropic"` | `"openai"`.
    pub provider: String,
    /// Specific model, or `None` until model selection exists.
    pub model: Option<String>,
}

impl Route {
    /// Derive the route from a resolved client. Both the requested and actual
    /// routes use this today.
    pub fn from_client(client: AgentClient) -> Route {
        let provider = match client {
            AgentClient::Claude => "anthropic",
            AgentClient::Codex => "openai",
        };
        Route {
            method: client.as_str().to_string(),
            provider: provider.to_string(),
            model: None,
        }
    }
}

/// Terminal outcome of a ticket-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunOutcome {
    /// Reached Done (review auto-complete, manual mark-done, or done-sweep).
    Done,
    /// Terminal failure: `.error` signal or stale-silence reclaim.
    Failed,
    /// Reclaimed for exceeding a session/per-phase timeout.
    TimedOut,
}

/// One append-only ledger record. Timestamps are UTC epoch seconds (matching the
/// `SystemTime` convention used across `lisa-core`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub schema_version: u32,
    pub ticket_id: String,
    pub outcome: RunOutcome,
    /// The route requested for this run (== `actual` until routing lands).
    pub requested: Route,
    /// The route that actually ran.
    pub actual: Route,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
    /// Input tokens, or `None` when unobtainable (never fabricated). For Claude
    /// this is the total input-side count (fresh + cache-creation + cache-read),
    /// summed from the transcript by `lisa capture-usage` (T-027-02).
    pub tokens_in: Option<u64>,
    /// Output tokens, or `None` when unobtainable.
    pub tokens_out: Option<u64>,
    /// Cost in USD, or `None` when unobtainable. Claude always carries `None`
    /// (no dependable dollar field in the transcript — derive downstream from
    /// tokens × pricing, T-027-02); Codex carries it only if the wrapper's usage
    /// object includes a cost field.
    pub cost_usd: Option<f64>,
    /// Count of threads already running when this run was spawned.
    pub concurrency_at_spawn: usize,
    pub pane_id: u32,
}

/// Convert a `SystemTime` to UTC epoch seconds, saturating pre-epoch times to 0.
pub fn system_time_to_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Best-effort token/cost extraction from a Codex `usage` JSON value. Returns
/// `(tokens_in, tokens_out, cost_usd)`, each `None` when its field is absent or
/// non-numeric. Never fabricates a value.
///
/// The Codex usage shape is provisional (see `agent_exec.rs`), so this reads
/// whichever of a few known field names is present rather than a fixed schema.
pub fn extract_usage(usage: &Value) -> (Option<u64>, Option<u64>, Option<f64>) {
    let u64_field = |names: &[&str]| -> Option<u64> {
        names
            .iter()
            .find_map(|n| usage.get(*n).and_then(Value::as_u64))
    };
    let f64_field = |names: &[&str]| -> Option<f64> {
        names
            .iter()
            .find_map(|n| usage.get(*n).and_then(Value::as_f64))
    };
    let tokens_in = u64_field(&["input_tokens", "input"]);
    let tokens_out = u64_field(&["output_tokens", "output"]);
    let cost_usd = f64_field(&["cost_usd", "cost", "total_cost_usd"]);
    (tokens_in, tokens_out, cost_usd)
}

/// Append one record as a single JSON line to `path`, creating the file and its
/// parent directory if absent. True append — existing lines are never rewritten,
/// so retries/resets of the same ticket accumulate additional records.
pub fn append_record(path: &Path, record: &ProvenanceRecord) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProvenanceRecord {
        ProvenanceRecord {
            schema_version: SCHEMA_VERSION,
            ticket_id: "T-027-01".to_string(),
            outcome: RunOutcome::Done,
            requested: Route::from_client(AgentClient::Codex),
            actual: Route::from_client(AgentClient::Codex),
            started_at: 1_719_800_000,
            ended_at: 1_719_800_600,
            wall_clock_secs: 600,
            tokens_in: Some(12_000),
            tokens_out: Some(3_400),
            cost_usd: None,
            concurrency_at_spawn: 3,
            pane_id: 2,
        }
    }

    #[test]
    fn route_from_client_maps_provider() {
        let c = Route::from_client(AgentClient::Claude);
        assert_eq!(c.method, "claude");
        assert_eq!(c.provider, "anthropic");
        assert_eq!(c.model, None);
        let x = Route::from_client(AgentClient::Codex);
        assert_eq!(x.method, "codex");
        assert_eq!(x.provider, "openai");
    }

    #[test]
    fn outcome_serde_is_kebab() {
        assert_eq!(
            serde_json::to_string(&RunOutcome::TimedOut).unwrap(),
            "\"timed-out\""
        );
        assert_eq!(
            serde_json::to_string(&RunOutcome::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&RunOutcome::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn record_serializes_to_one_compact_line() {
        let json = serde_json::to_string(&sample()).unwrap();
        assert!(!json.contains('\n'), "record must be single-line: {json}");
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"outcome\":\"done\""));
        assert!(json.contains("\"cost_usd\":null"));
        // Round-trips back to an equal record.
        let back: ProvenanceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn extract_usage_reads_known_fields() {
        let u = serde_json::json!({"input_tokens": 10, "output_tokens": 5, "cost_usd": 0.02});
        assert_eq!(extract_usage(&u), (Some(10), Some(5), Some(0.02)));
        // Alternate field names.
        let u2 = serde_json::json!({"input": 7, "output": 9});
        assert_eq!(extract_usage(&u2), (Some(7), Some(9), None));
    }

    #[test]
    fn extract_usage_absent_is_none() {
        let u = serde_json::json!({"unrelated": true});
        assert_eq!(extract_usage(&u), (None, None, None));
    }

    #[test]
    fn append_creates_then_appends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/provenance.jsonl");

        append_record(&path, &sample()).unwrap();
        let mut second = sample();
        second.outcome = RunOutcome::Failed;
        second.ticket_id = "T-027-01".to_string();
        append_record(&path, &second).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "append must not rewrite: {contents}");

        // First line is intact and parses to the original record.
        let first: ProvenanceRecord = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first, sample());
        let parsed_second: ProvenanceRecord = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed_second.outcome, RunOutcome::Failed);
    }

    #[test]
    fn system_time_to_epoch_is_seconds() {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_719_800_000);
        assert_eq!(system_time_to_epoch(t), 1_719_800_000);
        assert_eq!(system_time_to_epoch(UNIX_EPOCH), 0);
    }
}
