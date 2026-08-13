//! The one place Lisa turns a computed answer into a machine-readable document.
//!
//! `lisa status` and `lisa validate` both already assemble their answers into
//! structs and then format them for a person. `--json` stops throwing that
//! structure away. Both commands render through [`render`] so the two cannot
//! drift into different spellings of the same concept: one envelope, one
//! version marker, one shape for a failure.
//!
//! The stability rules a consumer may build against live in
//! `lisa json-guide` (`crates/lisa-cli/data/json-guide.md`). Keep the two in
//! step — a shape nobody can find is not an interface.

use std::collections::BTreeMap;

use serde::Serialize;

/// Name of the document contract. Present in every document Lisa prints.
pub const JSON_SCHEMA: &str = "lisa.cli/v1";

/// Version of the document contract. Bumped only for a breaking change — a
/// field a consumer was told it could rely on changing meaning or disappearing.
/// Additive fields never bump it, which is why unknown fields must be ignored.
pub const JSON_SCHEMA_VERSION: u32 = 1;

/// Lisa's own release, so a consumer can tell which build answered.
pub const LISA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One complete document: the only thing `--json` writes to stdout.
///
/// `ok`, `error` and `data` are always present. A failure carries `ok: false`,
/// a non-null `error`, and a null `data`, so a caller parses one format
/// whatever the outcome.
#[derive(Debug, Clone, Serialize)]
pub struct JsonDocument {
    pub schema: &'static str,
    pub schema_version: u32,
    pub lisa_version: &'static str,
    pub command: &'static str,
    pub ok: bool,
    pub error: Option<JsonError>,
    pub data: Option<serde_json::Value>,
}

/// A machine-readable failure. The message is the same sentence the human
/// path would have printed to stderr.
#[derive(Debug, Clone, Serialize)]
pub struct JsonError {
    pub message: String,
}

/// The board counts both commands talk about, spelled once.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BoardCounts {
    pub total: usize,
    pub done: usize,
    pub in_progress: usize,
    pub ready: usize,
    pub blocked: usize,
}

/// One problem a command found, named by the file it is about and the reason.
#[derive(Debug, Clone, Serialize)]
pub struct ProblemView {
    pub path: String,
    pub category: String,
    pub severity: &'static str,
    pub message: String,
}

/// The resolved settings both commands print under `Config:`.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigView {
    pub max_threads: usize,
    /// Seconds, or `0` when the human output says `disabled`.
    pub session_timeout_secs: u64,
    pub phase_timeouts: BTreeMap<String, u64>,
    /// Which agent client this board runs — `claude`, `codex`. Always a name:
    /// a board that names none in `.lisa.toml` still resolves to one, and a
    /// consumer deciding where to send work is asking what would run, not what
    /// was written down.
    pub client: &'static str,
    /// The model that board runs within its client, or `null` when it leaves
    /// that to the client's own default. Board-level intent, answerable before
    /// anything has ever run here; a ticket may still route itself elsewhere.
    pub model: Option<String>,
}

/// Where the run on this board is, when there is one.
///
/// `run_summary` is about what a run *did*; this is about what a run *is* and
/// how to reach it. They are separate keys for that reason: a board with no run
/// still has a summary of the last one, and a board whose run has finished
/// every ticket still has a session an operator can attach to.
///
/// Everything here is portable. A session name means the same thing to a phone
/// on the other end of `gh codespace ssh` as it does on the machine that wrote
/// it; a pid or a socket path does not, and neither is here. `schedulers[]`
/// carries the machine-local identity for a reader that is on the machine.
#[derive(Debug, Clone, Serialize)]
pub struct RunLocationView {
    /// `working`, `idle`, `none`, or `unknown`. See
    /// [`crate::seats::RunReport::location`] for what decides each one.
    pub state: &'static str,
    /// The session holding this board, when exactly one does. `null` on a board
    /// with no run, on a contested board with two — `sessions` has both — and
    /// on a run Lisa knows is here but cannot place.
    pub session: Option<String>,
    /// Every session named by a scheduler on this board, sorted. Empty is not
    /// the same as no run: read `state` for that.
    pub sessions: Vec<String>,
    /// The exact command that opens `session`, or `null` when there is no one
    /// session to open. Lisa names commands it can stand behind rather than
    /// leaving a caller to assemble one.
    pub attach_command: Option<String>,
}

/// What one command has to say, and the exit status that goes with it.
///
/// `Answer` is "Lisa computed the answer" — including an answer of "no". A
/// `validate` run that found problems is an answer with exit status 1, not a
/// failure: the problems belong in the body where a consumer can read them.
/// `Failure` is "Lisa could not answer at all", and carries the sentence the
/// human path would have printed to stderr.
#[derive(Debug, Clone)]
pub enum Outcome {
    Answer {
        data: serde_json::Value,
        exit_code: i32,
    },
    Failure(String),
}

impl Outcome {
    /// An answer that succeeded.
    pub fn ok(data: serde_json::Value) -> Self {
        Self::Answer { data, exit_code: 0 }
    }

    /// An answer whose verdict is "no", keeping the command's existing exit
    /// status.
    pub fn verdict(data: serde_json::Value, exit_code: i32) -> Self {
        Self::Answer { data, exit_code }
    }
}

impl JsonDocument {
    fn envelope(command: &'static str) -> Self {
        Self {
            schema: JSON_SCHEMA,
            schema_version: JSON_SCHEMA_VERSION,
            lisa_version: LISA_VERSION,
            command,
            ok: true,
            error: None,
            data: None,
        }
    }
}

/// Build the document for one command outcome.
///
/// Kept separate from printing so the shape can be asserted without a process.
pub fn document(command: &'static str, outcome: &Outcome) -> JsonDocument {
    let mut document = JsonDocument::envelope(command);
    match outcome {
        Outcome::Answer { data, .. } => document.data = Some(data.clone()),
        Outcome::Failure(message) => {
            document.ok = false;
            document.error = Some(JsonError {
                message: message.clone(),
            });
        }
    }
    document
}

/// Serialize one document to the exact bytes `--json` writes.
///
/// One line, one document, nothing else — a caller can pipe stdout without
/// filtering. Serialization of an already-built `serde_json::Value` cannot
/// fail, so a failure here is reported as a document rather than swallowed.
pub fn render(document: &JsonDocument) -> String {
    match serde_json::to_string(document) {
        Ok(body) => format!("{body}\n"),
        Err(error) => {
            let fallback = JsonDocument {
                data: None,
                ok: false,
                error: Some(JsonError {
                    message: format!("Failed to serialize {} document: {error}", document.command),
                }),
                ..JsonDocument::envelope(document.command)
            };
            // The fallback carries only strings, so this cannot fail in turn.
            serde_json::to_string(&fallback)
                .map(|body| format!("{body}\n"))
                .unwrap_or_else(|_| String::from("{\"ok\":false}\n"))
        }
    }
}

/// Print one document and return the process exit code that goes with it.
///
/// The exit code keeps its existing meaning — `--json` adds a body to that
/// contract, never replaces it. A command that could not answer at all exits 1,
/// the same as the human path.
pub fn emit(command: &'static str, outcome: Outcome) -> i32 {
    print!("{}", render(&document(command, &outcome)));
    match outcome {
        Outcome::Answer { exit_code, .. } => exit_code,
        Outcome::Failure(_) => 1,
    }
}

/// Emit one command's result, where a `Result` already carries the whole story.
pub fn emit_result(command: &'static str, outcome: Result<serde_json::Value, String>) -> i32 {
    emit(
        command,
        match outcome {
            Ok(data) => Outcome::ok(data),
            Err(message) => Outcome::Failure(message),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(command: &'static str, outcome: Outcome) -> serde_json::Value {
        let rendered = render(&document(command, &outcome));
        assert!(rendered.ends_with('\n'), "document must end with a newline");
        assert_eq!(
            rendered.trim_end_matches('\n').lines().count(),
            1,
            "a document is exactly one line: {rendered}"
        );
        serde_json::from_str(&rendered).expect("document must parse")
    }

    #[test]
    fn every_document_carries_the_version_marker_and_command() {
        for (command, outcome) in [
            ("status", Outcome::ok(serde_json::json!({"counts": {}}))),
            ("validate", Outcome::Failure("nope".to_string())),
        ] {
            let value = parse(command, outcome);
            assert_eq!(value["schema"], JSON_SCHEMA);
            assert_eq!(value["schema_version"], JSON_SCHEMA_VERSION);
            assert_eq!(value["lisa_version"], LISA_VERSION);
            assert_eq!(value["command"], command);
        }
    }

    /// A failure is still one document, not a bare message: the caller parses
    /// one format whatever the outcome.
    #[test]
    fn a_failure_is_a_document_with_a_message_and_no_data() {
        let value = parse(
            "status",
            Outcome::Failure("Ticket directory not found: docs".to_string()),
        );
        assert_eq!(value["ok"], false);
        assert_eq!(
            value["error"]["message"],
            "Ticket directory not found: docs"
        );
        assert!(value["data"].is_null());
    }

    #[test]
    fn a_success_carries_data_and_a_null_error() {
        let value = parse(
            "validate",
            Outcome::ok(serde_json::json!({"verdict": "passed"})),
        );
        assert_eq!(value["ok"], true);
        assert!(value["error"].is_null());
        assert_eq!(value["data"]["verdict"], "passed");
    }

    /// A verdict of "no" is an answer, not a failure: the problems stay
    /// readable in the body while the exit status keeps saying no.
    #[test]
    fn a_no_verdict_is_an_answer_that_still_exits_nonzero() {
        let outcome = Outcome::verdict(serde_json::json!({"verdict": "failed"}), 1);
        let value = parse("validate", outcome.clone());
        assert_eq!(value["ok"], true, "Lisa answered; the answer was no");
        assert!(value["error"].is_null());
        assert_eq!(value["data"]["verdict"], "failed");
        assert!(matches!(outcome, Outcome::Answer { exit_code: 1, .. }));
    }
}
