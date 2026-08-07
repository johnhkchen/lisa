//! Execution-provenance ledger: append-only JSONL learning data.
//!
//! The ledger contains terminal execution [`ProvenanceRecord`] rows,
//! pre-ownership [`AssignmentTransitionRecord`] rows, block retry/park/unpark
//! [`ParkingTransitionRecord`] rows, and completion-note acknowledgments. Use
//! [`ProvenanceLedgerRecord`] to read the mixed ledger. Terminal execution rows are written by
//! the plugin *after* the attempt ends
//! (write-after; they never race the agent and never touch the agent-owned
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
use crate::completion::CompletionSeal;
use crate::disposition::DispositionNote;
use crate::disposition::RemedyOwner;
use crate::notes::NoteAcknowledgmentRecord;
use crate::triage::TriageProposal;
use crate::types::AttemptLease;

/// Schema version stamped on every record. Bump when the record shape changes so
/// readers can branch (e.g. T-027-02 cost fidelity, S-026 routing splitting
/// `requested` from `actual`). Version 9 adds the usage-correction row that late-
/// joins a capture onto an already-completed ticket (T-051-03-01). Version 10
/// adds the check-override row that records an operator letting a parked ticket
/// run again over its own check (T-056-01-01).
pub const SCHEMA_VERSION: u32 = 10;

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

/// Explicit JSON-level kind for a pre-ownership provenance row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProvenanceRecordType {
    AssignmentTransition,
}

/// Explicit JSON-level kind for bounded blocked-work recovery and parking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParkingTransitionType {
    Retry,
    Park,
    Unpark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriageRecordType {
    TriageTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriageState {
    Started,
    Proposed,
    Failed,
    TimedOut,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalRecordType {
    ProposalAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalAction {
    Proposed,
    Attempted,
    Applied,
    Failed,
    Dismissed,
}

/// Explicit JSON-level kind for a late token-usage join (T-051-03-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageCorrectionType {
    UsageCorrection,
}

/// Stable, operator-visible assignment state retained in provenance.
///
/// This is evidence vocabulary, not scheduler authority: the plugin's private
/// assignment state machine may carry additional deadlines and retry counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssignmentState {
    ClaimTimedOut,
    DeliveryFailed,
    RecoveryFailed,
    StartupFailed,
}

/// One append-only ledger record. Timestamps are UTC epoch seconds (matching the
/// `SystemTime` convention used across `lisa-core`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect for this attempt. Missing on
    /// pre-ladder rows, which are commit-sealed by construction.
    #[serde(default)]
    pub seal: CompletionSeal,
    /// Criteria-versus-evidence note admitted with a successful completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_note: Option<DispositionNote>,
    pub ticket_id: String,
    /// Exact execution attempt that produced this terminal record.
    pub attempt_lease: AttemptLease,
    pub outcome: RunOutcome,
    /// Whether this is the ticket-level successful outcome. Schema-v2 writers
    /// set this only for a current-lease [`RunOutcome::Done`] publication.
    pub authoritative: bool,
    /// Whether scheduler teardown confirmed this attempt's pane was fenced.
    pub fenced: bool,
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

/// One attempt-scoped transition that ended before provider ownership.
///
/// Timestamps are UTC epoch seconds. `started_at` is the beginning of the
/// bounded transition, `ended_at` is the terminal observation, and
/// `wall_clock_secs` is their saturating difference supplied by the writer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentTransitionRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect for this attempt.
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: ProvenanceRecordType,
    pub ticket_id: String,
    pub attempt_lease: AttemptLease,
    pub pane_id: u32,
    /// Vendor serving the assignment (`"anthropic"` or `"openai"` today).
    pub provider: String,
    pub state: AssignmentState,
    pub reason: String,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
}

/// One attempt-scoped blocked-work retry or transition into/out of parked state.
///
/// Timestamps are UTC epoch seconds. The interval fields let an unpark row
/// carry queryable stranded time without joining separate point events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkingTransitionRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect for this attempt.
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: ParkingTransitionType,
    pub ticket_id: String,
    pub attempt_lease: AttemptLease,
    pub remedy_owner: RemedyOwner,
    /// One-based retry ordinal for a bounded agent-owned retry, or the final
    /// consumed count on its park row. Absent for owners that park immediately.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u8>,
    /// Configured per-loop retry bound paired with [`Self::retry_count`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_limit: Option<u8>,
    /// True only when external reality may later be probed for a remedy.
    #[serde(default, skip_serializing_if = "is_false")]
    pub recheck_eligible: bool,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
}

/// One bounded first-responder attempt against an already durable park.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageTransitionRecord {
    pub schema_version: u32,
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: TriageRecordType,
    pub ticket_id: String,
    pub source_attempt_lease: AttemptLease,
    pub route: Route,
    pub timeout_secs: u64,
    pub state: TriageState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub started_at: u64,
    pub ended_at: u64,
    pub wall_clock_secs: u64,
}

/// Creation or explicit operator disposition of one triage proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalActionRecord {
    pub schema_version: u32,
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: ProposalRecordType,
    pub ticket_id: String,
    pub source_attempt_lease: AttemptLease,
    pub action: ProposalAction,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<TriageProposal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub occurred_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperatorOverrideType {
    OperatorOverride,
}

/// One completion a person signed over a block or an unreadable review.
///
/// Distinct from [`ProvenanceRecord`] on purpose. That row describes an attempt
/// that ran, and requires a live thread and a current lease to write; the
/// tickets an override serves are precisely the ones whose agent is already
/// gone. Synthesizing a lease to reuse the execution shape would file a
/// fabricated run, so the receipt gets its own shape and carries only facts:
/// who signed, which catalog reason they chose, and the ask it overrode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOverrideRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect when the override was signed.
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: OperatorOverrideType,
    pub ticket_id: String,
    /// Who signed. `"operator"` for the mark-done key.
    pub actor: String,
    /// Stable catalog key, held fixed across copy rewordings.
    pub reason_id: String,
    /// The operator-facing copy, frozen at signing time so an old receipt stays
    /// readable after the catalog is reworded.
    pub reason: String,
    /// The ask or state this signature overrode, exactly as observed.
    pub overridden_ask: String,
    /// The note admitted with the completion.
    pub note: DispositionNote,
    pub occurred_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckOverrideType {
    CheckOverride,
}

/// What the overridden check reported before a person decided anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckOverrideOutcome {
    /// The check ran and did not pass.
    Failed,
    /// The check stopped before it could look, so it judged nothing.
    Inconclusive,
    /// The check outlived its budget and was stopped.
    TimedOut,
    /// The check tried to change project files.
    ChangedFiles,
}

/// One parked ticket a person let run again over its own check.
///
/// Shaped like [`OperatorOverrideRecord`] and for the same reason: the attempt
/// that parked this ticket is already gone, so there is no current lease to
/// attribute the row to, and synthesizing one to reuse the execution shape would
/// file a run that never happened. The row carries only what was observed — who
/// overrode, what the check was, where it ran, and what it reported — so a
/// forced unblock stays distinguishable afterwards from a check that passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOverrideRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect when the check was overridden.
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: CheckOverrideType,
    pub ticket_id: String,
    /// Who overrode. `"operator"` for `lisa unblock --override-check`.
    pub actor: String,
    /// The check string exactly as the review disposition recorded it.
    pub check: String,
    /// The directory the check actually ran in.
    pub directory: String,
    pub result: CheckOverrideOutcome,
    /// The check's exit code, absent when it was stopped rather than exiting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// The sanitized output lines the operator saw, in the order shown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed: Vec<String>,
    pub occurred_at: u64,
}

/// One late token-usage join for an already-terminal ticket-run.
///
/// The terminal [`ProvenanceRecord`] is written at completion with null tokens by
/// construction: rest-before-retire lands the session's capture *after* the row.
/// This append-only correction carries the joined tokens without ever mutating
/// the original row's bytes. It is attributed to the exact owning attempt and
/// keyed to its source capture line so a rescan never writes it twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCorrectionRecord {
    pub schema_version: u32,
    /// Completion durability tier in effect for the corrected attempt.
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: UsageCorrectionType,
    /// The completed ticket these tokens belong to.
    pub ticket_id: String,
    /// The exact attempt whose pane-time reign owned the capture.
    pub attempt_lease: AttemptLease,
    /// Provider client name (`"claude"` | `"codex"`) — disambiguates
    /// [`Self::source_line`], which is per-client-file.
    pub method: String,
    /// Provider session the capture observed. Load-bearing for the corrected
    /// view: captures are cumulative per-session snapshots, so
    /// [`correct_usage`] keeps only the latest snapshot per
    /// `(method, session_id)` and sums across distinct sessions.
    pub session_id: String,
    pub pane_id: u32,
    /// One-based physical line in the provider's `captures.jsonl`. With
    /// [`Self::method`] this uniquely identifies the source capture, so the join
    /// is idempotent across rescans.
    pub source_line: u64,
    /// Capture time as UTC epoch seconds.
    pub captured_at: u64,
    /// Joined input tokens. Never fabricated: a correction exists only when a
    /// real capture was observed.
    pub tokens_in: u64,
    /// Joined output tokens.
    pub tokens_out: u64,
    /// When this correction was written, UTC epoch seconds.
    pub occurred_at: u64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// A row read from a potentially mixed-version, mixed-shape provenance ledger.
///
/// Untagged representation preserves the exact schema-v2 execution JSON shape,
/// which predates an explicit record discriminator. Assignment and parking
/// shapes have required, disjoint `record_type` enums and their own required
/// fields, so the variants remain distinguishable without rewriting old ledger
/// lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProvenanceLedgerRecord {
    NoteAcknowledgment(NoteAcknowledgmentRecord),
    AssignmentTransition(AssignmentTransitionRecord),
    ParkingTransition(ParkingTransitionRecord),
    TriageTransition(TriageTransitionRecord),
    ProposalAction(ProposalActionRecord),
    OperatorOverride(OperatorOverrideRecord),
    CheckOverride(CheckOverrideRecord),
    UsageCorrection(UsageCorrectionRecord),
    Execution(ProvenanceRecord),
}

/// Per-ticket token totals after layering usage corrections over the raw row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TicketUsage {
    /// Corrected input tokens, or `None` when neither a correction nor a legacy
    /// non-null row supplies them — never a fabricated zero.
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    /// How many corrections joined onto this ticket.
    pub correction_count: usize,
}

/// Fold a mixed ledger into per-ticket token totals from the **corrected view**.
///
/// Each ticket is seeded from its authoritative `Done` execution row (raw tokens,
/// possibly `None`). Corrections are then layered by `ticket_id` — but a capture
/// row is a *cumulative snapshot*, not a delta: the Stop hook re-observes the
/// session's whole transcript every time it fires, so a multi-turn session
/// appends monotonically growing rows for the same session. Summing snapshots
/// double-counts (the first 0.4.4 field ledgers reported ~2× true usage). Per
/// `(method, session_id)` only the latest snapshot — ordered by
/// `(captured_at, source_line)` — is that session's truth; tokens sum across
/// *distinct* sessions only. `correction_count` still counts every correction
/// record (the append-only audit trail), not just surviving snapshots. When a
/// ticket has any correction its tokens override the raw null row; otherwise the
/// raw row tokens stand as a legacy fallback. A completed ticket with neither
/// stays `None`.
pub fn correct_usage<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceLedgerRecord>,
) -> std::collections::BTreeMap<String, TicketUsage> {
    use std::collections::BTreeMap;

    // Raw seed from authoritative Done rows; latest snapshot per session; and
    // the audit count of every correction record seen.
    let mut raw: BTreeMap<String, (Option<u64>, Option<u64>)> = BTreeMap::new();
    #[allow(clippy::type_complexity)]
    let mut latest: BTreeMap<String, BTreeMap<(String, String), (u64, u64, u64, u64)>> =
        BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for record in records {
        match record {
            ProvenanceLedgerRecord::Execution(exec)
                if exec.authoritative && exec.outcome == RunOutcome::Done =>
            {
                raw.insert(exec.ticket_id.clone(), (exec.tokens_in, exec.tokens_out));
            }
            ProvenanceLedgerRecord::UsageCorrection(correction) => {
                *counts.entry(correction.ticket_id.clone()).or_insert(0) += 1;
                let sessions = latest.entry(correction.ticket_id.clone()).or_default();
                let key = (correction.method.clone(), correction.session_id.clone());
                let is_newer = sessions.get(&key).is_none_or(|&(at, line, _, _)| {
                    (correction.captured_at, correction.source_line) >= (at, line)
                });
                if is_newer {
                    sessions.insert(
                        key,
                        (
                            correction.captured_at,
                            correction.source_line,
                            correction.tokens_in,
                            correction.tokens_out,
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    let mut view: BTreeMap<String, TicketUsage> = BTreeMap::new();
    for (ticket_id, (tokens_in, tokens_out)) in raw {
        view.insert(
            ticket_id,
            TicketUsage {
                tokens_in,
                tokens_out,
                correction_count: 0,
            },
        );
    }
    for (ticket_id, sessions) in latest {
        let (tokens_in, tokens_out) = sessions.values().fold(
            (0u64, 0u64),
            |(acc_in, acc_out), &(_, _, session_in, session_out)| {
                (
                    acc_in.saturating_add(session_in),
                    acc_out.saturating_add(session_out),
                )
            },
        );
        let correction_count = counts.get(&ticket_id).copied().unwrap_or(0);
        // Corrections override the raw seed even for a ticket with no Done row on
        // this ledger slice.
        view.insert(
            ticket_id,
            TicketUsage {
                tokens_in: Some(tokens_in),
                tokens_out: Some(tokens_out),
                correction_count,
            },
        );
    }
    view
}

/// Ticket ids that completed (authoritative `Done`) but still have no joined
/// input tokens in the corrected view — the countable capture gap. Sorted.
/// A `null` here is honestly unknown usage, never a fabricated zero.
pub fn usage_gap<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceLedgerRecord> + Clone,
) -> Vec<String> {
    let mut done: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for record in records.clone() {
        if let ProvenanceLedgerRecord::Execution(exec) = record {
            if exec.authoritative && exec.outcome == RunOutcome::Done {
                done.insert(exec.ticket_id.clone());
            }
        }
    }
    let view = correct_usage(records);
    done.into_iter()
        .filter(|ticket_id| {
            view.get(ticket_id)
                .map(|usage| usage.tokens_in.is_none())
                .unwrap_or(true)
        })
        .collect()
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
    append_serialized(path, record)
}

/// Append one pre-ownership assignment transition as a single JSONL row.
pub fn append_assignment_transition_record(
    path: &Path,
    record: &AssignmentTransitionRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

/// Append one park or unpark transition as a single JSONL row.
pub fn append_parking_transition_record(
    path: &Path,
    record: &ParkingTransitionRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

pub fn append_triage_transition_record(
    path: &Path,
    record: &TriageTransitionRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

pub fn append_proposal_action_record(
    path: &Path,
    record: &ProposalActionRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

/// Append one operator-signed completion override as a single JSONL row.
pub fn append_operator_override_record(
    path: &Path,
    record: &OperatorOverrideRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

/// Append one operator-forced unblock as a single JSONL row.
pub fn append_check_override_record(
    path: &Path,
    record: &CheckOverrideRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

/// Append one late token-usage join as a single JSONL row.
pub fn append_usage_correction_record(
    path: &Path,
    record: &UsageCorrectionRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

/// Append one exact completion-note acknowledgment as a single JSONL row.
pub fn append_note_acknowledgment_record(
    path: &Path,
    record: &NoteAcknowledgmentRecord,
) -> std::io::Result<()> {
    append_serialized(path, record)
}

fn append_serialized<T: Serialize + ?Sized>(path: &Path, record: &T) -> std::io::Result<()> {
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
            seal: CompletionSeal::Commit,
            completion_note: None,
            ticket_id: "T-027-01".to_string(),
            attempt_lease: AttemptLease::mint("T-027-01", None).unwrap(),
            outcome: RunOutcome::Done,
            authoritative: true,
            fenced: false,
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

    fn sample_assignment_transition() -> AssignmentTransitionRecord {
        AssignmentTransitionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Journal,
            record_type: ProvenanceRecordType::AssignmentTransition,
            ticket_id: "T-040-02-01".to_string(),
            attempt_lease: AttemptLease {
                ticket_id: "T-040-02-01".to_string(),
                attempt_id: 7,
            },
            pane_id: 12,
            provider: "openai".to_string(),
            state: AssignmentState::DeliveryFailed,
            reason: "provider did not acknowledge the bounded chat assignment".to_string(),
            started_at: 1_752_000_000,
            ended_at: 1_752_000_030,
            wall_clock_secs: 30,
        }
    }

    fn sample_parking_transition(record_type: ParkingTransitionType) -> ParkingTransitionRecord {
        ParkingTransitionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Journal,
            record_type,
            ticket_id: "T-048-01-02".to_string(),
            attempt_lease: AttemptLease {
                ticket_id: "T-048-01-02".to_string(),
                attempt_id: 4,
            },
            remedy_owner: RemedyOwner::Operator,
            retry_count: None,
            retry_limit: None,
            recheck_eligible: false,
            started_at: 1_752_700_000,
            ended_at: 1_752_700_125,
            wall_clock_secs: 125,
        }
    }

    const SCHEMA_V2_EXECUTION_JSON: &str = r#"{"schema_version":2,"ticket_id":"T-027-01","attempt_lease":{"ticket_id":"T-027-01","attempt_id":2},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"codex","provider":"openai","model":null},"actual":{"method":"codex","provider":"openai","model":null},"started_at":1719800000,"ended_at":1719800600,"wall_clock_secs":600,"tokens_in":12000,"tokens_out":3400,"cost_usd":null,"concurrency_at_spawn":3,"pane_id":2}"#;

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
        assert!(json.contains("\"schema_version\":10"));
        assert!(json.contains("\"seal\":\"commit\""));
        assert!(json.contains("\"attempt_lease\":{\"ticket_id\":\"T-027-01\",\"attempt_id\":1}"));
        assert!(json.contains("\"outcome\":\"done\""));
        assert!(json.contains("\"authoritative\":true"));
        assert!(json.contains("\"fenced\":false"));
        assert!(json.contains("\"cost_usd\":null"));
        // Round-trips back to an equal record.
        let back: ProvenanceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn completion_note_round_trips_and_legacy_rows_default_to_none() {
        let mut record = sample();
        record.completion_note = Some(
            DispositionNote::new(
                "approximately 200 MiB",
                "docs/active/work/T-046-06-03/cbt-0716-210943-closing-codex/run-record.md",
                "The 225 MiB measurement supports completion while the written gate is stale.",
            )
            .unwrap(),
        );
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"completion_note\""));
        assert!(json.contains("\"criterion_quote\":\"approximately 200 MiB\""));
        assert_eq!(
            serde_json::from_str::<ProvenanceRecord>(&json).unwrap(),
            record
        );

        let legacy: ProvenanceRecord = serde_json::from_str(SCHEMA_V2_EXECUTION_JSON).unwrap();
        assert_eq!(legacy.completion_note, None);
    }

    #[test]
    fn assignment_transition_serializes_to_one_compact_line() {
        let record = sample_assignment_transition();
        let json = serde_json::to_string(&record).unwrap();

        assert!(!json.contains('\n'), "record must be single-line: {json}");
        assert!(json.contains("\"schema_version\":10"));
        assert!(json.contains("\"seal\":\"journal\""));
        assert!(json.contains("\"record_type\":\"assignment-transition\""));
        assert!(json.contains("\"attempt_lease\":{\"ticket_id\":\"T-040-02-01\",\"attempt_id\":7}"));
        assert!(json.contains("\"pane_id\":12"));
        assert!(json.contains("\"provider\":\"openai\""));
        assert!(json.contains("\"state\":\"delivery-failed\""));
        assert!(json
            .contains("\"reason\":\"provider did not acknowledge the bounded chat assignment\""));
        assert!(json.contains("\"started_at\":1752000000"));
        assert!(json.contains("\"ended_at\":1752000030"));
        assert!(json.contains("\"wall_clock_secs\":30"));

        let back: AssignmentTransitionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn assignment_transition_appends_as_exactly_one_jsonl_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provenance.jsonl");
        let record = sample_assignment_transition();

        append_assignment_transition_record(&path, &record).unwrap();

        let bytes = fs::read(&path).unwrap();
        assert!(bytes.ends_with(b"\n"));
        assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 1);
        let back: AssignmentTransitionRecord = serde_json::from_slice(&bytes[..bytes.len() - 1])
            .expect("the only JSONL row remains a complete assignment record");
        assert_eq!(back, record);
    }

    #[test]
    fn parking_transitions_serialize_and_round_trip_as_compact_rows() {
        for record_type in [ParkingTransitionType::Park, ParkingTransitionType::Unpark] {
            let record = sample_parking_transition(record_type);
            let json = serde_json::to_string(&record).unwrap();

            assert!(!json.contains('\n'), "record must be single-line: {json}");
            assert!(json.contains("\"schema_version\":10"));
            assert!(json.contains("\"seal\":\"journal\""));
            assert!(json.contains(&format!(
                "\"record_type\":{}",
                serde_json::to_string(&record_type).unwrap()
            )));
            assert!(
                json.contains("\"attempt_lease\":{\"ticket_id\":\"T-048-01-02\",\"attempt_id\":4}")
            );
            assert!(json.contains("\"remedy_owner\":\"operator\""));
            assert!(!json.contains("\"retry_count\""));
            assert!(!json.contains("\"retry_limit\""));
            assert!(!json.contains("\"recheck_eligible\""));
            assert!(json.contains("\"started_at\":1752700000"));
            assert!(json.contains("\"ended_at\":1752700125"));
            assert!(json.contains("\"wall_clock_secs\":125"));

            let back: ParkingTransitionRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(back, record);
        }
    }

    #[test]
    fn block_retry_and_world_recheck_fields_are_explicit_and_round_trip() {
        let mut retry = sample_parking_transition(ParkingTransitionType::Retry);
        retry.remedy_owner = RemedyOwner::Agent;
        retry.retry_count = Some(2);
        retry.retry_limit = Some(2);
        let retry_json = serde_json::to_string(&retry).unwrap();
        assert!(retry_json.contains("\"record_type\":\"retry\""));
        assert!(retry_json.contains("\"remedy_owner\":\"agent\""));
        assert!(retry_json.contains("\"retry_count\":2"));
        assert!(retry_json.contains("\"retry_limit\":2"));
        assert!(!retry_json.contains("\"recheck_eligible\""));
        assert_eq!(
            serde_json::from_str::<ParkingTransitionRecord>(&retry_json).unwrap(),
            retry
        );

        let mut world = sample_parking_transition(ParkingTransitionType::Park);
        world.remedy_owner = RemedyOwner::World;
        world.recheck_eligible = true;
        let world_json = serde_json::to_string(&world).unwrap();
        assert!(world_json.contains("\"record_type\":\"park\""));
        assert!(world_json.contains("\"recheck_eligible\":true"));
        assert_eq!(
            serde_json::from_str::<ParkingTransitionRecord>(&world_json).unwrap(),
            world
        );
    }

    #[test]
    fn triage_attempt_and_proposal_action_append_as_distinct_mixed_rows() {
        use crate::triage::{PreparedStep, TriageProposal};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provenance.jsonl");
        let lease = AttemptLease {
            ticket_id: "T-046-06-03".to_string(),
            attempt_id: 4,
        };
        let proposal = TriageProposal {
            summary: "The criterion conflicts with the measured evidence.".to_string(),
            recommendation: "Amend the stale criterion.".to_string(),
            prepared_steps: vec![PreparedStep::Command {
                description: "Apply the amendment.".to_string(),
                command: "git apply amendment.patch".to_string(),
            }],
        };
        let attempt = TriageTransitionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: TriageRecordType::TriageTransition,
            ticket_id: lease.ticket_id.clone(),
            source_attempt_lease: lease.clone(),
            route: Route::from_client(AgentClient::Codex),
            timeout_secs: 120,
            state: TriageState::Proposed,
            reason: None,
            started_at: 10,
            ended_at: 12,
            wall_clock_secs: 2,
        };
        let action = ProposalActionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: ProposalRecordType::ProposalAction,
            ticket_id: lease.ticket_id.clone(),
            source_attempt_lease: lease,
            action: ProposalAction::Proposed,
            actor: "agent".to_string(),
            proposal: Some(proposal),
            step_count: None,
            applied_steps: Vec::new(),
            failed_step: None,
            failure_reason: None,
            occurred_at: 12,
        };
        append_triage_transition_record(&path, &attempt).unwrap();
        append_proposal_action_record(&path, &action).unwrap();
        let rows: Vec<ProvenanceLedgerRecord> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            rows,
            vec![
                ProvenanceLedgerRecord::TriageTransition(attempt),
                ProvenanceLedgerRecord::ProposalAction(action),
            ]
        );
    }

    #[test]
    fn proposal_apply_attempt_and_failure_round_trip_with_step_evidence() {
        use crate::triage::{PreparedStep, TriageProposal};

        let lease = AttemptLease {
            ticket_id: "T-049-08-02".to_string(),
            attempt_id: 7,
        };
        let proposal = TriageProposal {
            summary: "The operator can apply the prepared repair.".to_string(),
            recommendation: "Apply both prepared steps.".to_string(),
            prepared_steps: vec![PreparedStep::Command {
                description: "Apply the repair.".to_string(),
                command: "true".to_string(),
            }],
        };
        let attempted = ProposalActionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: ProposalRecordType::ProposalAction,
            ticket_id: lease.ticket_id.clone(),
            source_attempt_lease: lease.clone(),
            action: ProposalAction::Attempted,
            actor: "operator".to_string(),
            proposal: Some(proposal),
            step_count: Some(2),
            applied_steps: Vec::new(),
            failed_step: None,
            failure_reason: None,
            occurred_at: 20,
        };
        let failed = ProposalActionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: ProposalRecordType::ProposalAction,
            ticket_id: lease.ticket_id.clone(),
            source_attempt_lease: lease,
            action: ProposalAction::Failed,
            actor: "operator".to_string(),
            proposal: None,
            step_count: None,
            applied_steps: vec!["Apply the first repair.".to_string()],
            failed_step: Some("Apply the second repair.".to_string()),
            failure_reason: Some("prepared command exited with status 1".to_string()),
            occurred_at: 21,
        };

        let attempted_json = serde_json::to_string(&attempted).unwrap();
        assert!(attempted_json.contains("\"action\":\"attempted\""));
        assert!(attempted_json.contains("\"step_count\":2"));
        assert!(!attempted_json.contains("applied_steps"));
        let failed_json = serde_json::to_string(&failed).unwrap();
        assert!(failed_json.contains("\"action\":\"failed\""));
        assert!(failed_json.contains("\"applied_steps\":[\"Apply the first repair.\"]"));
        assert!(failed_json.contains("\"failed_step\":\"Apply the second repair.\""));
        assert_eq!(
            serde_json::from_str::<ProposalActionRecord>(&attempted_json).unwrap(),
            attempted
        );
        assert_eq!(
            serde_json::from_str::<ProposalActionRecord>(&failed_json).unwrap(),
            failed
        );
    }

    #[test]
    fn schema_four_parking_rows_default_additive_block_policy_fields() {
        let raw = r#"{"schema_version":4,"record_type":"park","ticket_id":"T-048-01-02","attempt_lease":{"ticket_id":"T-048-01-02","attempt_id":4},"remedy_owner":"operator","started_at":1752700000,"ended_at":1752700125,"wall_clock_secs":125}"#;
        let record: ParkingTransitionRecord = serde_json::from_str(raw).unwrap();
        assert_eq!(record.schema_version, 4);
        assert_eq!(record.seal, CompletionSeal::Commit);
        assert_eq!(record.retry_count, None);
        assert_eq!(record.retry_limit, None);
        assert!(!record.recheck_eligible);
    }

    #[test]
    fn parking_transitions_append_and_replay_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/provenance.jsonl");
        let park = sample_parking_transition(ParkingTransitionType::Park);
        let mut unpark = sample_parking_transition(ParkingTransitionType::Unpark);
        unpark.remedy_owner = RemedyOwner::World;
        unpark.recheck_eligible = true;

        append_parking_transition_record(&path, &park).unwrap();
        append_parking_transition_record(&path, &unpark).unwrap();

        let bytes = fs::read(&path).unwrap();
        assert!(bytes.ends_with(b"\n"));
        assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 2);
        let records: Vec<ProvenanceLedgerRecord> = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(
            records,
            vec![
                ProvenanceLedgerRecord::ParkingTransition(park),
                ProvenanceLedgerRecord::ParkingTransition(unpark),
            ]
        );
    }

    #[test]
    fn mixed_ledger_replays_execution_assignment_park_and_unpark_rows() {
        let legacy: ProvenanceRecord = serde_json::from_str(SCHEMA_V2_EXECUTION_JSON).unwrap();
        assert_eq!(legacy.schema_version, 2);
        assert_eq!(legacy.attempt_lease.attempt_id, 2);
        assert_eq!(legacy.outcome, RunOutcome::Done);
        assert!(legacy.authoritative);
        assert_eq!(legacy.seal, CompletionSeal::Commit);

        let mut transition = sample_assignment_transition();
        transition.schema_version = 3;
        let park = sample_parking_transition(ParkingTransitionType::Park);
        let mut unpark = sample_parking_transition(ParkingTransitionType::Unpark);
        unpark.remedy_owner = RemedyOwner::World;
        unpark.recheck_eligible = true;
        let ledger = format!(
            "{}\n{}\n{}\n{}\n",
            SCHEMA_V2_EXECUTION_JSON,
            serde_json::to_string(&transition).unwrap(),
            serde_json::to_string(&park).unwrap(),
            serde_json::to_string(&unpark).unwrap(),
        );
        let rows: Vec<ProvenanceLedgerRecord> = ledger
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows[0],
            ProvenanceLedgerRecord::Execution(legacy),
            "the unchanged schema-v2 line remains an execution record"
        );
        assert_eq!(
            rows[1],
            ProvenanceLedgerRecord::AssignmentTransition(transition),
            "the schema-v3 line is recognized as pre-ownership evidence"
        );
        assert_eq!(
            rows[2],
            ProvenanceLedgerRecord::ParkingTransition(park),
            "the current-schema park line is recognized as parking evidence"
        );
        assert_eq!(
            rows[3],
            ProvenanceLedgerRecord::ParkingTransition(unpark.clone()),
            "the current-schema unpark line is recognized as parking evidence"
        );
        let ProvenanceLedgerRecord::ParkingTransition(replayed) = &rows[3] else {
            panic!("expected an unpark transition")
        };
        assert_eq!(replayed.attempt_lease.attempt_id, 4);
        assert_eq!(replayed.remedy_owner, RemedyOwner::World);
        assert!(replayed.recheck_eligible);
        assert_eq!(replayed.started_at, 1_752_700_000);
        assert_eq!(replayed.ended_at, 1_752_700_125);
        assert_eq!(replayed.wall_clock_secs, 125);
    }

    #[test]
    fn pre_ladder_assignment_rows_default_to_commit_sealed() {
        let raw = r#"{"schema_version":3,"record_type":"assignment-transition","ticket_id":"T-040-02-01","attempt_lease":{"ticket_id":"T-040-02-01","attempt_id":7},"pane_id":12,"provider":"openai","state":"delivery-failed","reason":"legacy","started_at":1,"ended_at":2,"wall_clock_secs":1}"#;
        let record: AssignmentTransitionRecord = serde_json::from_str(raw).unwrap();
        assert_eq!(record.seal, CompletionSeal::Commit);
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

    fn sample_correction(
        ticket_id: &str,
        tokens_in: u64,
        tokens_out: u64,
    ) -> UsageCorrectionRecord {
        UsageCorrectionRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Journal,
            record_type: UsageCorrectionType::UsageCorrection,
            ticket_id: ticket_id.to_string(),
            attempt_lease: AttemptLease {
                ticket_id: ticket_id.to_string(),
                attempt_id: 3,
            },
            method: "claude".to_string(),
            session_id: "session-late".to_string(),
            pane_id: 4,
            source_line: 7,
            captured_at: 1_752_345_600,
            tokens_in,
            tokens_out,
            occurred_at: 1_752_345_900,
        }
    }

    fn done_row(
        ticket_id: &str,
        tokens_in: Option<u64>,
        tokens_out: Option<u64>,
    ) -> ProvenanceRecord {
        let route = Route::from_client(AgentClient::Claude);
        ProvenanceRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            completion_note: None,
            ticket_id: ticket_id.to_string(),
            attempt_lease: AttemptLease {
                ticket_id: ticket_id.to_string(),
                attempt_id: 1,
            },
            outcome: RunOutcome::Done,
            authoritative: true,
            fenced: false,
            requested: route.clone(),
            actual: route,
            started_at: 1_752_345_000,
            ended_at: 1_752_345_500,
            wall_clock_secs: 500,
            tokens_in,
            tokens_out,
            cost_usd: None,
            concurrency_at_spawn: 0,
            pane_id: 4,
        }
    }

    #[test]
    fn usage_correction_serializes_to_one_compact_line_and_round_trips() {
        let record = sample_correction("T-051-03-01", 1200, 340);
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains('\n'), "record must be single-line: {json}");
        assert!(json.contains("\"schema_version\":10"));
        assert!(json.contains("\"record_type\":\"usage-correction\""));
        assert!(json.contains("\"method\":\"claude\""));
        assert!(json.contains("\"source_line\":7"));
        assert!(json.contains("\"tokens_in\":1200"));
        let back: UsageCorrectionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn usage_correction_appends_and_replays_distinct_from_execution() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provenance.jsonl");
        let exec = done_row("T-051-03-01", None, None);
        let correction = sample_correction("T-051-03-01", 1200, 340);

        append_record(&path, &exec).unwrap();
        append_usage_correction_record(&path, &correction).unwrap();

        let rows: Vec<ProvenanceLedgerRecord> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            rows,
            vec![
                ProvenanceLedgerRecord::Execution(exec),
                ProvenanceLedgerRecord::UsageCorrection(correction),
            ]
        );
    }

    fn sample_operator_override(ticket_id: &str) -> OperatorOverrideRecord {
        OperatorOverrideRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: OperatorOverrideType::OperatorOverride,
            ticket_id: ticket_id.to_string(),
            actor: "operator".to_string(),
            reason_id: "cannot-verify-here".to_string(),
            reason: "This can't be checked from this machine — accepted as far as it can be checked here.".to_string(),
            overridden_ask: "Sign into Xcode with an Apple ID, then re-run the signed build."
                .to_string(),
            note: DispositionNote::new(
                "Sign into Xcode with an Apple ID, then re-run the signed build.",
                "docs/active/work/T-015-02-02/review-disposition.json",
                "This can't be checked from this machine — accepted as far as it can be checked here.",
            )
            .unwrap(),
            occurred_at: 1_752_800_000,
        }
    }

    #[test]
    fn operator_override_record_round_trips_through_the_mixed_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provenance.jsonl");
        let record = sample_operator_override("T-015-02-02");

        append_operator_override_record(&path, &record).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 1);
        assert!(body.contains("\"record_type\":\"operator-override\""));
        // The receipt's three obligations: who signed, which reason, what it
        // overrode.
        assert!(body.contains("\"actor\":\"operator\""));
        assert!(body.contains("\"reason_id\":\"cannot-verify-here\""));
        assert!(body.contains("Sign into Xcode with an Apple ID"));

        assert_eq!(
            serde_json::from_str::<ProvenanceLedgerRecord>(body.trim()).unwrap(),
            ProvenanceLedgerRecord::OperatorOverride(record)
        );
    }

    /// The untagged enum resolves by shape, so a new arm could silently absorb
    /// existing rows or be absorbed by an earlier one. Both directions checked.
    #[test]
    fn operator_override_row_does_not_absorb_or_get_absorbed() {
        let override_line =
            serde_json::to_string(&sample_operator_override("T-015-02-02")).unwrap();
        assert!(matches!(
            serde_json::from_str::<ProvenanceLedgerRecord>(&override_line).unwrap(),
            ProvenanceLedgerRecord::OperatorOverride(_)
        ));

        let existing: Vec<(&str, String)> = vec![
            (
                "execution",
                serde_json::to_string(&done_row("T-A", None, None)).unwrap(),
            ),
            (
                "assignment",
                serde_json::to_string(&sample_assignment_transition()).unwrap(),
            ),
            (
                "parking",
                serde_json::to_string(&sample_parking_transition(ParkingTransitionType::Park))
                    .unwrap(),
            ),
            (
                "usage-correction",
                serde_json::to_string(&sample_correction("T-A", 1, 2)).unwrap(),
            ),
        ];
        for (label, line) in existing {
            let parsed: ProvenanceLedgerRecord = serde_json::from_str(&line).unwrap();
            assert!(
                !matches!(parsed, ProvenanceLedgerRecord::OperatorOverride(_)),
                "the new arm absorbed an existing {label} row"
            );
        }
    }

    fn sample_check_override(ticket_id: &str) -> CheckOverrideRecord {
        CheckOverrideRecord {
            schema_version: SCHEMA_VERSION,
            seal: CompletionSeal::Commit,
            record_type: CheckOverrideType::CheckOverride,
            ticket_id: ticket_id.to_string(),
            actor: "operator".to_string(),
            check: "node scripts/check-touch.mjs".to_string(),
            directory: "/tmp/.tmpAbC123".to_string(),
            result: CheckOverrideOutcome::Inconclusive,
            exit_code: Some(2),
            observed: vec!["No build at dist/. Run: npm run build".to_string()],
            occurred_at: 1_752_900_000,
        }
    }

    #[test]
    fn check_override_record_round_trips_through_the_mixed_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provenance.jsonl");
        let record = sample_check_override("T-010-03");

        append_check_override_record(&path, &record).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 1);
        assert!(body.contains("\"record_type\":\"check-override\""));
        // The receipt's obligations: who overrode, what ran, where, and what it
        // reported.
        assert!(body.contains("\"actor\":\"operator\""));
        assert!(body.contains("\"check\":\"node scripts/check-touch.mjs\""));
        assert!(body.contains("\"directory\":\"/tmp/.tmpAbC123\""));
        assert!(body.contains("\"result\":\"inconclusive\""));
        assert!(body.contains("\"exit_code\":2"));
        assert!(body.contains("No build at dist/"));

        assert_eq!(
            serde_json::from_str::<ProvenanceLedgerRecord>(body.trim()).unwrap(),
            ProvenanceLedgerRecord::CheckOverride(record)
        );
    }

    /// The same both-directions guard the operator-override arm carries: an
    /// untagged arm must neither absorb an existing row nor be absorbed by one.
    #[test]
    fn check_override_row_does_not_absorb_or_get_absorbed() {
        let line = serde_json::to_string(&sample_check_override("T-010-03")).unwrap();
        assert!(matches!(
            serde_json::from_str::<ProvenanceLedgerRecord>(&line).unwrap(),
            ProvenanceLedgerRecord::CheckOverride(_)
        ));

        let existing: Vec<(&str, String)> = vec![
            (
                "execution",
                serde_json::to_string(&done_row("T-A", None, None)).unwrap(),
            ),
            (
                "assignment",
                serde_json::to_string(&sample_assignment_transition()).unwrap(),
            ),
            (
                "parking",
                serde_json::to_string(&sample_parking_transition(ParkingTransitionType::Park))
                    .unwrap(),
            ),
            (
                "usage-correction",
                serde_json::to_string(&sample_correction("T-A", 1, 2)).unwrap(),
            ),
            (
                "operator-override",
                serde_json::to_string(&sample_operator_override("T-A")).unwrap(),
            ),
        ];
        for (label, line) in existing {
            let parsed: ProvenanceLedgerRecord = serde_json::from_str(&line).unwrap();
            assert!(
                !matches!(parsed, ProvenanceLedgerRecord::CheckOverride(_)),
                "the new arm absorbed an existing {label} row"
            );
        }
    }

    /// An overridden check is not a run, so it must not reach the token view.
    #[test]
    fn usage_fold_ignores_check_override_rows() {
        let without = vec![
            ProvenanceLedgerRecord::Execution(done_row("T-A", None, None)),
            ProvenanceLedgerRecord::UsageCorrection(sample_correction("T-A", 100, 10)),
        ];
        let mut with = without.clone();
        with.push(ProvenanceLedgerRecord::CheckOverride(
            sample_check_override("T-A"),
        ));

        assert_eq!(correct_usage(&without), correct_usage(&with));
    }

    #[test]
    fn usage_fold_ignores_operator_override_rows() {
        let without = vec![
            ProvenanceLedgerRecord::Execution(done_row("T-A", None, None)),
            ProvenanceLedgerRecord::UsageCorrection(sample_correction("T-A", 100, 10)),
        ];
        let mut with = without.clone();
        with.push(ProvenanceLedgerRecord::OperatorOverride(
            sample_operator_override("T-A"),
        ));

        assert_eq!(correct_usage(&without), correct_usage(&with));
    }

    fn session_correction(
        ticket_id: &str,
        method: &str,
        session_id: &str,
        captured_at: u64,
        source_line: u64,
        tokens_in: u64,
        tokens_out: u64,
    ) -> UsageCorrectionRecord {
        let mut record = sample_correction(ticket_id, tokens_in, tokens_out);
        record.method = method.to_string();
        record.session_id = session_id.to_string();
        record.captured_at = captured_at;
        record.source_line = source_line;
        record
    }

    #[test]
    fn correct_usage_sums_across_distinct_sessions() {
        // Two different provider sessions worked this ticket (e.g. a park and
        // a fresh attempt): their latest snapshots sum.
        let rows = vec![
            ProvenanceLedgerRecord::Execution(done_row("T-A", None, None)),
            ProvenanceLedgerRecord::UsageCorrection(session_correction(
                "T-A",
                "claude",
                "session-1",
                100,
                1,
                100,
                10,
            )),
            ProvenanceLedgerRecord::UsageCorrection(session_correction(
                "T-A",
                "claude",
                "session-2",
                200,
                2,
                250,
                20,
            )),
        ];
        let view = correct_usage(&rows);
        let usage = view.get("T-A").unwrap();
        assert_eq!(usage.tokens_in, Some(350));
        assert_eq!(usage.tokens_out, Some(30));
        assert_eq!(usage.correction_count, 2);
    }

    #[test]
    fn correct_usage_takes_latest_snapshot_within_a_session() {
        // A capture row is a cumulative snapshot of the whole session
        // transcript — Stop fires per turn, so one session appends
        // monotonically growing rows. The field shape that caught this:
        // 6,761,596 then 7,809,647 input on one session; summing reported
        // 14,571,243 (~2x truth). The view must report the later snapshot.
        for reversed in [false, true] {
            let mut corrections = vec![
                session_correction("T-B", "claude", "session-7baf", 100, 1, 6_761_596, 33_049),
                session_correction("T-B", "claude", "session-7baf", 714, 2, 7_809_647, 40_000),
            ];
            if reversed {
                corrections.reverse();
            }
            let mut rows = vec![ProvenanceLedgerRecord::Execution(done_row(
                "T-B", None, None,
            ))];
            rows.extend(
                corrections
                    .into_iter()
                    .map(ProvenanceLedgerRecord::UsageCorrection),
            );
            let view = correct_usage(&rows);
            let usage = view.get("T-B").unwrap();
            assert_eq!(usage.tokens_in, Some(7_809_647), "reversed={reversed}");
            assert_eq!(usage.tokens_out, Some(40_000), "reversed={reversed}");
            assert_eq!(
                usage.correction_count, 2,
                "the audit count still sees every correction record"
            );
        }
    }

    #[test]
    fn correct_usage_treats_same_session_id_across_methods_as_distinct() {
        // The session id is provider-scoped; the same string under different
        // methods is two sessions, never one.
        let rows = vec![
            ProvenanceLedgerRecord::Execution(done_row("T-C", None, None)),
            ProvenanceLedgerRecord::UsageCorrection(session_correction(
                "T-C",
                "claude",
                "shared-id",
                100,
                1,
                1_000,
                100,
            )),
            ProvenanceLedgerRecord::UsageCorrection(session_correction(
                "T-C",
                "codex",
                "shared-id",
                200,
                1,
                2_000,
                200,
            )),
        ];
        let view = correct_usage(&rows);
        let usage = view.get("T-C").unwrap();
        assert_eq!(usage.tokens_in, Some(3_000));
        assert_eq!(usage.tokens_out, Some(300));
    }

    #[test]
    fn correct_usage_falls_back_to_legacy_non_null_row() {
        let rows = vec![ProvenanceLedgerRecord::Execution(done_row(
            "T-LEGACY",
            Some(12_000),
            Some(3_400),
        ))];
        let view = correct_usage(&rows);
        let usage = view.get("T-LEGACY").unwrap();
        assert_eq!(usage.tokens_in, Some(12_000));
        assert_eq!(usage.tokens_out, Some(3_400));
        assert_eq!(usage.correction_count, 0);
    }

    #[test]
    fn correct_usage_leaves_capture_never_ticket_null_and_gap_counts_it() {
        let rows = vec![
            ProvenanceLedgerRecord::Execution(done_row("T-NULL", None, None)),
            ProvenanceLedgerRecord::Execution(done_row("T-JOINED", None, None)),
            ProvenanceLedgerRecord::UsageCorrection(sample_correction("T-JOINED", 5, 1)),
        ];
        let view = correct_usage(&rows);
        assert_eq!(view.get("T-NULL").unwrap().tokens_in, None);
        assert_eq!(view.get("T-JOINED").unwrap().tokens_in, Some(5));
        assert_eq!(usage_gap(&rows), vec!["T-NULL".to_string()]);
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
    fn append_preserves_cross_ticket_lease_attribution_without_publication_residue() {
        let dir = tempfile::tempdir().unwrap();
        let ledger_dir = dir.path().join("ledger path ' ; $() `x`");
        let path = ledger_dir.join("provenance.jsonl");
        let ticket_a = sample();
        let mut ticket_b = sample();
        ticket_b.ticket_id = "T-027-02".to_string();
        ticket_b.attempt_lease = AttemptLease {
            ticket_id: "T-027-02".to_string(),
            attempt_id: 42,
        };
        ticket_b.outcome = RunOutcome::TimedOut;
        ticket_b.authoritative = false;
        ticket_b.fenced = true;
        ticket_b.pane_id = 9;

        append_record(&path, &ticket_a).unwrap();
        append_record(&path, &ticket_b).unwrap();

        let bytes = fs::read(&path).unwrap();
        assert!(bytes.ends_with(b"\n"));
        assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 2);
        let records: Vec<ProvenanceRecord> = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        assert_eq!(records, vec![ticket_a, ticket_b]);
        for record in &records {
            assert_eq!(record.ticket_id, record.attempt_lease.ticket_id);
        }
        assert_eq!(records[0].attempt_lease.attempt_id, 1);
        assert_eq!(records[1].attempt_lease.attempt_id, 42);
        assert_eq!(records[0].outcome, RunOutcome::Done);
        assert_eq!(records[1].outcome, RunOutcome::TimedOut);
        assert_eq!(fs::read_dir(&ledger_dir).unwrap().count(), 1);
    }

    #[test]
    fn append_failure_preserves_existing_target_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger path ' ; $()");
        fs::create_dir(&path).unwrap();
        let sentinel = path.join("existing-ledger-bytes");
        fs::write(&sentinel, b"prior provenance remains intact\n").unwrap();

        let error = append_record(&path, &sample()).unwrap_err();

        assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::IsADirectory
                    | std::io::ErrorKind::PermissionDenied
                    | std::io::ErrorKind::Other
            ),
            "unexpected append error: {error}"
        );
        assert!(path.is_dir(), "the colliding target remains a directory");
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            b"prior provenance remains intact\n",
            "a failed append must not disturb existing target contents"
        );
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    }

    #[test]
    fn system_time_to_epoch_is_seconds() {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(1_719_800_000);
        assert_eq!(system_time_to_epoch(t), 1_719_800_000);
        assert_eq!(system_time_to_epoch(UNIX_EPOCH), 0);
    }
}
