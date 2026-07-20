//! The small catalog of reasons a person may sign a completion with.
//!
//! [`crate::disposition`] is the fail-closed reader for agent-authored verdicts:
//! its job is refusing to confuse a parser failure with approval. This module
//! runs the other direction — it builds a trusted [`DispositionNote`] from a
//! person's explicit choice, so a parked ticket can be resolved without a second
//! completion path.
//!
//! Two rules shape the catalog. Every reason accepts the work and says why the
//! ask does not apply; none hedges on quality, because doubting the work is
//! send-back's job, not a note's (the S-049-06 discipline, enforced downstream by
//! [`crate::disposition`]'s complaint-field rejection). And nothing here is
//! invented: the caller supplies the state it observed and the paths it read, and
//! this module only assembles and validates.

use serde::{Deserialize, Serialize};

use crate::disposition::DispositionNote;

/// One canned reason from the operator override catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverrideReason {
    /// The work already shows what the ask wanted; the written criterion was
    /// read more strictly than the ticket asked for.
    EvidenceSatisfies,
    /// The ask needs a check this machine cannot perform.
    CannotVerifyHere,
    /// The ask reaches past what this ticket can touch.
    BeyondTicketReach,
    /// No usable agent review exists to read.
    NoReviewOnFile,
}

impl OverrideReason {
    /// Every catalog entry, in presentation order.
    pub const ALL: [Self; 4] = [
        Self::EvidenceSatisfies,
        Self::CannotVerifyHere,
        Self::BeyondTicketReach,
        Self::NoReviewOnFile,
    ];

    /// The stable join key written to the ledger.
    ///
    /// Held fixed when the operator-facing copy is reworded, so an old receipt
    /// stays queryable against a new catalog.
    pub const fn id(self) -> &'static str {
        match self {
            Self::EvidenceSatisfies => "evidence-satisfies",
            Self::CannotVerifyHere => "cannot-verify-here",
            Self::BeyondTicketReach => "beyond-ticket-reach",
            Self::NoReviewOnFile => "no-review-on-file",
        }
    }

    /// The sentence a person reads and signs.
    ///
    /// Kitchen-table English: no mechanism words, and never a quality hedge.
    pub const fn summary(self) -> &'static str {
        match self {
            Self::EvidenceSatisfies => {
                "The work already covers this — the review asked for more than the ticket did."
            }
            Self::CannotVerifyHere => {
                "This can't be checked from this machine — accepted as far as it can be checked here."
            }
            Self::BeyondTicketReach => {
                "This is past what the ticket can reach — accepted as it stands."
            }
            Self::NoReviewOnFile => {
                "No agent review was left for this one — accepted on the work as it stands."
            }
        }
    }
}

/// Reasons that honestly fit a real block left by an agent.
const BLOCK_REASONS: &[OverrideReason] = &[
    OverrideReason::EvidenceSatisfies,
    OverrideReason::CannotVerifyHere,
    OverrideReason::BeyondTicketReach,
];

/// The only reason that fits a ticket with no readable review.
const NO_REVIEW_REASONS: &[OverrideReason] = &[OverrideReason::NoReviewOnFile];

/// The state an operator override is answering.
///
/// Each variant carries what can be honestly quoted for it. The fail-closed
/// variants exist because a missing file and an unreadable one are different
/// facts about the world, even though the parser reports both as invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverriddenAsk {
    /// An agent left a block. `ask` is the sentence being overridden and
    /// `reason` its technical companion.
    Block { ask: String, reason: String },
    /// No review file exists for this ticket.
    NoReviewOnFile,
    /// A review file exists but could not be read. `detail` is the reader's own
    /// description of the failure, never authored prose.
    UnreadableReview { detail: String },
}

impl OverriddenAsk {
    /// The reasons that fit this state.
    ///
    /// Deliberately not the whole catalog: signing a real block with "no review
    /// was left" would be a fabricated receipt, so the catalog is partitioned
    /// rather than offered whole.
    pub const fn applicable_reasons(&self) -> &'static [OverrideReason] {
        match self {
            Self::Block { .. } => BLOCK_REASONS,
            Self::NoReviewOnFile | Self::UnreadableReview { .. } => NO_REVIEW_REASONS,
        }
    }

    /// The entry a chooser preselects.
    pub const fn recommended_reason(&self) -> OverrideReason {
        match self {
            Self::Block { .. } => OverrideReason::EvidenceSatisfies,
            Self::NoReviewOnFile | Self::UnreadableReview { .. } => OverrideReason::NoReviewOnFile,
        }
    }

    /// The literal ask or state this override answers, for the receipt.
    pub fn overridden_ask(&self, ticket_id: &str) -> String {
        match self {
            Self::Block { ask, .. } => ask.clone(),
            Self::NoReviewOnFile => no_review_state(ticket_id),
            Self::UnreadableReview { detail } => detail.clone(),
        }
    }
}

/// How the absence of a review is stated wherever it must be quoted.
fn no_review_state(ticket_id: &str) -> String {
    format!("no agent review on file for {ticket_id}")
}

/// A completed operator override: the reason signed, what it answered, and the
/// note that rides the ordinary completion path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorOverride {
    reason: OverrideReason,
    overridden_ask: String,
    note: DispositionNote,
}

impl OperatorOverride {
    /// The catalog entry the operator chose.
    pub const fn reason(&self) -> OverrideReason {
        self.reason
    }

    /// The ask or state this signature overrode, exactly as observed.
    pub fn overridden_ask(&self) -> &str {
        &self.overridden_ask
    }

    /// The note admitted with the completion.
    pub const fn note(&self) -> &DispositionNote {
        &self.note
    }
}

/// Build an override note from an observed state and a chosen reason.
///
/// `inspected_paths` are repository-relative paths the caller has **observed to
/// exist**. This function never probes the filesystem and never guesses: a path
/// reaches the citation only because a caller read it.
///
/// Fails when the reason does not fit the state, or when no inspected path was
/// supplied. The empty-field floor lives in [`DispositionNote::new`] and is not
/// duplicated here.
pub fn build_operator_override(
    ticket_id: &str,
    state: &OverriddenAsk,
    reason: OverrideReason,
    inspected_paths: &[String],
) -> Result<OperatorOverride, String> {
    if !state.applicable_reasons().contains(&reason) {
        return Err(format!(
            "the reason {} does not fit this ticket's state",
            reason.id()
        ));
    }

    let citation = inspected_paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if citation.is_empty() {
        return Err("an override note requires at least one inspected path".to_string());
    }

    let criterion_quote = match state {
        OverriddenAsk::Block { ask, .. } => ask.clone(),
        OverriddenAsk::NoReviewOnFile => no_review_state(ticket_id),
        OverriddenAsk::UnreadableReview { detail } => detail.clone(),
    };

    let note = DispositionNote::new(criterion_quote, citation, reason.summary())?;
    Ok(OperatorOverride {
        reason,
        overridden_ask: state.overridden_ask(ticket_id),
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPOSITION_PATH: &str = "docs/active/work/T-001/review-disposition.json";
    const WORK_DIR: &str = "docs/active/work/T-001";

    fn block() -> OverriddenAsk {
        OverriddenAsk::Block {
            ask: "Sign into Xcode with an Apple ID, then re-run the signed build.".to_string(),
            reason: "codesign refused: no signing identity found".to_string(),
        }
    }

    fn unreadable() -> OverriddenAsk {
        OverriddenAsk::UnreadableReview {
            detail: "review disposition is malformed JSON: expected value at line 1 column 1"
                .to_string(),
        }
    }

    fn paths(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// Criterion 5 as an executable check rather than a promise in review.md.
    #[test]
    fn catalog_copy_passes_the_kitchen_table_read() {
        const MECHANISM_WORDS: &[&str] = &["frontmatter", "disposition", "dag", "seal"];
        for reason in OverrideReason::ALL {
            let summary = reason.summary().to_lowercase();
            for word in MECHANISM_WORDS {
                assert!(
                    !summary
                        .split(|character: char| !character.is_ascii_alphanumeric())
                        .any(|token| token == *word),
                    "{} says {word:?}, which no one says at a kitchen table: {:?}",
                    reason.id(),
                    reason.summary()
                );
            }
        }
    }

    /// Doubting the work is send-back's job, not a note's (S-049-06).
    #[test]
    fn no_catalog_entry_hedges_on_quality() {
        const HEDGES: &[&str] = &[
            "maybe",
            "unsure",
            "probably",
            "risky",
            "questionable",
            "looks wrong",
            "doubt",
        ];
        for reason in OverrideReason::ALL {
            let summary = reason.summary().to_lowercase();
            for hedge in HEDGES {
                assert!(
                    !summary.contains(hedge),
                    "{} hedges with {hedge:?}: {:?}",
                    reason.id(),
                    reason.summary()
                );
            }
        }
    }

    #[test]
    fn catalog_ids_are_unique_and_every_reason_is_listed() {
        let mut ids: Vec<&str> = OverrideReason::ALL.iter().map(|r| r.id()).collect();
        ids.sort_unstable();
        let listed = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), listed, "catalog ids must be unique");
        assert_eq!(listed, 4);
        for reason in OverrideReason::ALL {
            assert!(!reason.summary().trim().is_empty());
        }
    }

    #[test]
    fn block_offers_three_reasons_and_recommends_evidence_satisfies() {
        let state = block();
        assert_eq!(
            state.applicable_reasons(),
            &[
                OverrideReason::EvidenceSatisfies,
                OverrideReason::CannotVerifyHere,
                OverrideReason::BeyondTicketReach,
            ]
        );
        assert_eq!(
            state.recommended_reason(),
            OverrideReason::EvidenceSatisfies
        );
    }

    #[test]
    fn fail_closed_states_offer_only_the_no_review_reason() {
        for state in [OverriddenAsk::NoReviewOnFile, unreadable()] {
            assert_eq!(
                state.applicable_reasons(),
                &[OverrideReason::NoReviewOnFile]
            );
            assert_eq!(state.recommended_reason(), OverrideReason::NoReviewOnFile);
        }
    }

    #[test]
    fn inapplicable_reason_is_refused() {
        let error = build_operator_override(
            "T-001",
            &block(),
            OverrideReason::NoReviewOnFile,
            &paths(&[DISPOSITION_PATH]),
        )
        .unwrap_err();
        assert!(error.contains("no-review-on-file"), "{error}");

        for state in [OverriddenAsk::NoReviewOnFile, unreadable()] {
            for reason in BLOCK_REASONS {
                assert!(
                    build_operator_override("T-001", &state, *reason, &paths(&[WORK_DIR])).is_err(),
                    "{} must not fit {state:?}",
                    reason.id()
                );
            }
        }
    }

    #[test]
    fn block_note_quotes_the_ask_and_cites_inspected_paths() {
        let state = block();
        let over = build_operator_override(
            "T-001",
            &state,
            OverrideReason::CannotVerifyHere,
            &paths(&[DISPOSITION_PATH, "docs/active/work/T-001/review.md"]),
        )
        .unwrap();

        assert_eq!(
            over.note().criterion_quote(),
            "Sign into Xcode with an Apple ID, then re-run the signed build."
        );
        assert_eq!(
            over.note().evidence_citation(),
            "docs/active/work/T-001/review-disposition.json, docs/active/work/T-001/review.md"
        );
        assert_eq!(
            over.note().summary(),
            OverrideReason::CannotVerifyHere.summary()
        );
        assert_eq!(over.reason(), OverrideReason::CannotVerifyHere);
        assert_eq!(
            over.overridden_ask(),
            "Sign into Xcode with an Apple ID, then re-run the signed build."
        );
    }

    #[test]
    fn missing_review_note_states_the_absence_plainly() {
        let over = build_operator_override(
            "T-001",
            &OverriddenAsk::NoReviewOnFile,
            OverrideReason::NoReviewOnFile,
            &paths(&[WORK_DIR]),
        )
        .unwrap();

        assert_eq!(
            over.note().criterion_quote(),
            "no agent review on file for T-001"
        );
        assert_eq!(over.note().evidence_citation(), WORK_DIR);
        assert_eq!(over.overridden_ask(), "no agent review on file for T-001");
    }

    #[test]
    fn unreadable_review_note_quotes_the_parse_failure() {
        let state = unreadable();
        let over = build_operator_override(
            "T-001",
            &state,
            OverrideReason::NoReviewOnFile,
            &paths(&[DISPOSITION_PATH]),
        )
        .unwrap();

        assert_eq!(
            over.note().criterion_quote(),
            "review disposition is malformed JSON: expected value at line 1 column 1"
        );
        assert_eq!(over.note().evidence_citation(), DISPOSITION_PATH);
    }

    /// Criterion 2's "no empty fields", across every shape the override serves.
    #[test]
    fn every_shape_produces_three_non_empty_fields() {
        let cases = [
            (block(), OverrideReason::EvidenceSatisfies, DISPOSITION_PATH),
            (
                OverriddenAsk::NoReviewOnFile,
                OverrideReason::NoReviewOnFile,
                WORK_DIR,
            ),
            (
                unreadable(),
                OverrideReason::NoReviewOnFile,
                DISPOSITION_PATH,
            ),
        ];
        for (state, reason, path) in cases {
            let over =
                build_operator_override("T-001", &state, reason, &paths(&[path])).unwrap();
            let note = over.note();
            assert!(!note.criterion_quote().trim().is_empty(), "{state:?}");
            assert!(!note.evidence_citation().trim().is_empty(), "{state:?}");
            assert!(!note.summary().trim().is_empty(), "{state:?}");
            assert!(!over.overridden_ask().trim().is_empty(), "{state:?}");
        }
    }

    #[test]
    fn empty_inspected_paths_is_refused() {
        for supplied in [paths(&[]), paths(&["", "   "])] {
            let error = build_operator_override(
                "T-001",
                &OverriddenAsk::NoReviewOnFile,
                OverrideReason::NoReviewOnFile,
                &supplied,
            )
            .unwrap_err();
            assert!(error.contains("inspected path"), "{error}");
        }
    }

    #[test]
    fn reason_ids_round_trip_through_serde() {
        for reason in OverrideReason::ALL {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, format!("\"{}\"", reason.id()));
            assert_eq!(
                serde_json::from_str::<OverrideReason>(&json).unwrap(),
                reason
            );
        }
    }
}
