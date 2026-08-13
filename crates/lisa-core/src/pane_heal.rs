//! Asking a running loop to put back a pane it lost, and reading its answer.
//!
//! `.lisa-layout.kdl` declares how many coding panes a run has and creates them
//! once, at `lisa loop`. Nothing has ever created one since, so a pane that dies
//! — fenced by Lisa, crashed with its terminal, closed by hand — is gone for the
//! session's lifetime and the board quietly runs on fewer seats than it says it
//! has. The loop now notices that for itself; this module is the other door in,
//! for a tool like `rail` that noticed first.
//!
//! ## Who does what
//!
//! Creating geometry inside Zellij is the plugin's job and nobody else's. `rail`
//! forbids itself from splitting panes (`no-zellij-split`) and it is right to:
//! the plugin runs *inside* the Zellij server and is the only thing that can put
//! a pane back where the layout wanted it. So the ask is a request, not a
//! command — the requester writes a file and the scheduler decides.
//!
//! ```text
//! rail  ─── lisa heal-panes ──▶ .lisa/pane-heal.request   (the ask)
//!                                        │
//!                                   plugin poll tick
//!                                        │
//!         lisa heal-panes ◀── .lisa/pane-heal.answer      (healed | refused | already-fine)
//! ```
//!
//! ## Three answers and nothing else
//!
//! - [`PaneHealAnswer::Healed`] — the board was short, the scheduler asked
//!   Zellij for a pane, and the pane arrived and joined the stack.
//! - [`PaneHealAnswer::AlreadyFine`] — the board has every pane its layout
//!   declared. Nothing was created; asking again will say the same thing.
//! - [`PaneHealAnswer::Refused`] — the scheduler will not, and says why: its
//!   regeneration budget is spent, or the layout it was launched from never said
//!   how many panes it wanted. A refusal always names the way through.
//!
//! There is deliberately no fourth answer for *the scheduler never replied*. A
//! request that goes unanswered is the requester's timeout to report, and it
//! means something different — no loop is running, or its plugin is wedged —
//! than anything a scheduler could have said.
//!
//! ## Single reader, single writer
//!
//! The request is consumed the way pane signals are: read, then removed, by the
//! one scheduler that got to it first. The answer names the nonce it is
//! answering, so a requester never reads a receipt for somebody else's ask, and
//! names the scheduler that wrote it, so a board held by two runs says which one
//! did the work.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Where a requester leaves its ask, relative to the project root.
pub const PANE_HEAL_REQUEST_FILE: &str = ".lisa/pane-heal.request";

/// Where the answering scheduler leaves its receipt, relative to the project
/// root.
pub const PANE_HEAL_ANSWER_FILE: &str = ".lisa/pane-heal.answer";

/// The shape both sides of the ask agree on. Bumped only when a field's meaning
/// changes.
pub const SCHEMA_VERSION: u32 = 1;

/// One ask, from whoever noticed the board was short.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHealRequest {
    /// The shape of this record.
    pub schema_version: u32,
    /// Identifies this exact ask, so its answer cannot be confused with the
    /// answer to another one.
    pub nonce: String,
    /// Unix seconds at which the ask was written.
    pub asked_at: u64,
    /// Who asked, for the activity feed. Free text: `rail`, `operator`, a script
    /// name. Never interpreted.
    pub asked_by: String,
}

impl PaneHealRequest {
    /// An ask stamped `asked_at`, identified by `nonce`.
    pub fn new(nonce: impl Into<String>, asked_at: u64, asked_by: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            nonce: nonce.into(),
            asked_at,
            asked_by: asked_by.into(),
        }
    }
}

/// What a scheduler did about one ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaneHealAnswer {
    /// The board was short and is not any more.
    Healed,
    /// The board already had every pane its layout declared.
    AlreadyFine,
    /// The scheduler will not heal, and `detail` says why.
    Refused,
}

impl PaneHealAnswer {
    /// The stable token written to the receipt and printed by `--json`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Healed => "healed",
            Self::AlreadyFine => "already-fine",
            Self::Refused => "refused",
        }
    }

    /// Whether the requester should treat this as a successful ask.
    ///
    /// A board that was already whole is not a failure — it is the answer the
    /// requester wanted. Only a refusal is worth an exit code.
    pub const fn is_satisfied(self) -> bool {
        matches!(self, Self::Healed | Self::AlreadyFine)
    }
}

/// One scheduler's receipt for one ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHealReceipt {
    /// The shape of this record.
    pub schema_version: u32,
    /// The [`PaneHealRequest::nonce`] this answers.
    pub nonce: String,
    /// What was done.
    pub answer: PaneHealAnswer,
    /// One plain sentence a person can act on. Always populated for a refusal.
    pub detail: String,
    /// How many coding panes the run's layout declared, or `None` when the
    /// layout it was launched from never said.
    pub declared: Option<usize>,
    /// How many the scheduler could see when it answered.
    pub present: usize,
    /// The scheduler that answered, as `.lisa/schedulers/` names it.
    pub scheduler: Option<String>,
    /// Unix seconds at which the receipt was written.
    pub answered_at: u64,
}

impl PaneHealReceipt {
    /// A receipt answering `nonce`.
    pub fn new(
        nonce: impl Into<String>,
        answer: PaneHealAnswer,
        detail: impl Into<String>,
        declared: Option<usize>,
        present: usize,
        scheduler: Option<String>,
        answered_at: u64,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            nonce: nonce.into(),
            answer,
            detail: detail.into(),
            declared,
            present,
            scheduler,
            answered_at,
        }
    }
}

/// The request path under `root`.
pub fn request_path(root: &Path) -> PathBuf {
    root.join(PANE_HEAL_REQUEST_FILE)
}

/// The answer path under `root`.
pub fn answer_path(root: &Path) -> PathBuf {
    root.join(PANE_HEAL_ANSWER_FILE)
}

/// Publish an ask under `root`, replacing any older one.
///
/// Written through a temporary file and renamed, so a scheduler polling the
/// directory never reads half an ask. Publishing also removes the previous
/// answer: a receipt left over from the last ask is exactly what a requester
/// must not mistake for this one's, and the nonce alone should not have to
/// carry that.
pub fn publish_request(root: &Path, request: &PaneHealRequest) -> Result<(), String> {
    let path = request_path(root);
    let parent = path
        .parent()
        .ok_or_else(|| format!("pane-heal request path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let _ = std::fs::remove_file(answer_path(root));
    let body = serde_json::to_string(request)
        .map_err(|error| format!("could not encode pane-heal request: {error}"))?;
    let temporary = path.with_extension(format!("request.tmp.{}", request.nonce));
    std::fs::write(&temporary, body)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("could not publish {}: {error}", path.display())
    })
}

/// Take the outstanding ask under `root`, removing it.
///
/// Single-reader by construction, exactly like a pane signal: the scheduler that
/// reads it is the one that owes an answer. An unreadable or malformed file is
/// removed and reported as absent — a request nobody can parse is not an ask a
/// scheduler can answer, and leaving it there would re-fail every poll.
pub fn take_request(root: &Path) -> Option<PaneHealRequest> {
    let path = request_path(root);
    let body = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    serde_json::from_str(&body).ok()
}

/// Leave a receipt under `root`.
pub fn publish_receipt(root: &Path, receipt: &PaneHealReceipt) -> Result<(), String> {
    let path = answer_path(root);
    let parent = path
        .parent()
        .ok_or_else(|| format!("pane-heal answer path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let body = serde_json::to_string(receipt)
        .map_err(|error| format!("could not encode pane-heal receipt: {error}"))?;
    let temporary = path.with_extension(format!("answer.tmp.{}", receipt.nonce));
    std::fs::write(&temporary, body)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        format!("could not publish {}: {error}", path.display())
    })
}

/// Read the receipt for `nonce`, if one has been left.
///
/// A receipt for a different nonce is not this ask's answer and reads as absent.
/// Nothing is removed: the requester is not the only reader, and a receipt is
/// evidence rather than a signal.
pub fn read_receipt(root: &Path, nonce: &str) -> Option<PaneHealReceipt> {
    let body = std::fs::read_to_string(answer_path(root)).ok()?;
    let receipt: PaneHealReceipt = serde_json::from_str(&body).ok()?;
    (receipt.nonce == nonce).then_some(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn an_ask_round_trips_through_the_shape_both_sides_read() {
        let root = temp_root();
        let request = PaneHealRequest::new("n-1", 1_786_487_257, "rail");
        publish_request(root.path(), &request).unwrap();
        assert_eq!(take_request(root.path()), Some(request));
    }

    #[test]
    fn an_ask_is_consumed_by_the_first_reader() {
        let root = temp_root();
        publish_request(root.path(), &PaneHealRequest::new("n-1", 10, "rail")).unwrap();
        assert!(take_request(root.path()).is_some());
        assert_eq!(
            take_request(root.path()),
            None,
            "a second scheduler must not answer an ask that was already taken"
        );
    }

    #[test]
    fn an_unparseable_ask_is_removed_rather_than_re_failed_every_poll() {
        let root = temp_root();
        std::fs::create_dir_all(root.path().join(".lisa")).unwrap();
        std::fs::write(request_path(root.path()), "{not json").unwrap();
        assert_eq!(take_request(root.path()), None);
        assert!(!request_path(root.path()).exists());
    }

    #[test]
    fn a_receipt_only_answers_its_own_nonce() {
        let root = temp_root();
        let receipt = PaneHealReceipt::new(
            "n-2",
            PaneHealAnswer::Healed,
            "put pane 4 back",
            Some(4),
            4,
            Some("lisa-1".to_string()),
            20,
        );
        publish_receipt(root.path(), &receipt).unwrap();
        assert_eq!(read_receipt(root.path(), "n-2"), Some(receipt));
        assert_eq!(
            read_receipt(root.path(), "n-1"),
            None,
            "a receipt for another ask is not this ask's answer"
        );
    }

    #[test]
    fn publishing_an_ask_clears_the_last_ones_receipt() {
        let root = temp_root();
        publish_receipt(
            root.path(),
            &PaneHealReceipt::new(
                "old",
                PaneHealAnswer::AlreadyFine,
                "nothing to do",
                Some(4),
                4,
                None,
                1,
            ),
        )
        .unwrap();
        publish_request(root.path(), &PaneHealRequest::new("new", 2, "rail")).unwrap();
        assert!(
            !answer_path(root.path()).exists(),
            "a stale receipt must not be sitting there when the next ask goes out"
        );
    }

    #[test]
    fn the_three_answers_keep_their_stable_tokens() {
        assert_eq!(PaneHealAnswer::Healed.id(), "healed");
        assert_eq!(PaneHealAnswer::AlreadyFine.id(), "already-fine");
        assert_eq!(PaneHealAnswer::Refused.id(), "refused");
        assert_eq!(
            serde_json::to_string(&PaneHealAnswer::AlreadyFine).unwrap(),
            "\"already-fine\""
        );
        assert!(PaneHealAnswer::Healed.is_satisfied());
        assert!(PaneHealAnswer::AlreadyFine.is_satisfied());
        assert!(!PaneHealAnswer::Refused.is_satisfied());
    }
}
