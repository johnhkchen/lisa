//! Durable, validated advice attached to an operator-owned parked ticket.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::AttemptLease;

pub const TRIAGE_PROPOSAL_FILE: &str = "triage-proposal.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageProposal {
    pub summary: String,
    pub recommendation: String,
    pub prepared_steps: Vec<PreparedStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PreparedStep {
    Command {
        description: String,
        command: String,
    },
    FileEdit {
        description: String,
        path: PathBuf,
        old: String,
        new: String,
    },
}

impl PreparedStep {
    pub fn description(&self) -> &str {
        match self {
            Self::Command { description, .. } | Self::FileEdit { description, .. } => description,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        visible("prepared step description", self.description())?;
        match self {
            Self::Command { command, .. } => visible("prepared command", command),
            Self::FileEdit { path, old, new, .. } => {
                safe_relative_path(path)?;
                visible("file edit old text", old)?;
                if old == new {
                    return Err("file edit old and new text must differ".to_string());
                }
                Ok(())
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Command {
                description,
                command,
            } => format!("{description} Run: `{command}`"),
            Self::FileEdit {
                description, path, ..
            } => format!("{description} Edit: {}", path.display()),
        }
    }
}

impl TriageProposal {
    pub fn validate(&self) -> Result<(), String> {
        visible("proposal summary", &self.summary)?;
        visible("proposal recommendation", &self.recommendation)?;
        if !is_one_sentence(&self.summary) {
            return Err("proposal summary must be one plain sentence".to_string());
        }
        if self.prepared_steps.is_empty() {
            return Err("proposal must contain at least one prepared step".to_string());
        }
        for step in &self.prepared_steps {
            step.validate()?;
        }
        Ok(())
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let proposal: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid triage proposal JSON: {error}"))?;
        proposal.validate()?;
        Ok(proposal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalState {
    Pending,
    Applied,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredTriageProposal {
    pub ticket_id: String,
    pub source_attempt_lease: AttemptLease,
    pub state: ProposalState,
    pub proposal: TriageProposal,
}

impl StoredTriageProposal {
    pub fn validate(&self) -> Result<(), String> {
        visible("proposal ticket id", &self.ticket_id)?;
        if self.source_attempt_lease.ticket_id != self.ticket_id {
            return Err("proposal ticket and source attempt ticket must match".to_string());
        }
        if self.source_attempt_lease.attempt_id == 0 {
            return Err("proposal source attempt must be positive".to_string());
        }
        self.proposal.validate()
    }
}

pub fn read_stored_proposal(path: &Path) -> Result<Option<StoredTriageProposal>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let stored: StoredTriageProposal = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid stored triage proposal: {error}"))?;
    stored.validate()?;
    Ok(Some(stored))
}

pub fn write_stored_proposal(path: &Path, stored: &StoredTriageProposal) -> Result<(), String> {
    stored.validate()?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("proposal path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let body = serde_json::to_vec_pretty(stored)
        .map_err(|error| format!("could not serialize proposal: {error}"))?;
    let temporary = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(&temporary, body)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not publish {}: {error}", path.display())
    })
}

fn visible(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must contain visible text"))
    } else {
        Ok(())
    }
}

fn is_one_sentence(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.contains('\n') {
        return false;
    }
    let endings = trimmed
        .chars()
        .filter(|character| matches!(character, '.' | '!' | '?'))
        .count();
    endings <= 1
}

fn safe_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("file edit path must be repository-relative".to_string());
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe file edit path: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> TriageProposal {
        TriageProposal {
            summary: "The written limit conflicts with the measured evidence.".to_string(),
            recommendation: "Amend the stale acceptance sentence.".to_string(),
            prepared_steps: vec![PreparedStep::FileEdit {
                description: "Use the calibrated limit.".to_string(),
                path: PathBuf::from("docs/ticket.md"),
                old: "about 200 MiB".to_string(),
                new: "at most 300 MiB".to_string(),
            }],
        }
    }

    #[test]
    fn proposal_round_trips_and_validates() {
        let proposal = proposal();
        let body = serde_json::to_vec(&proposal).unwrap();
        assert_eq!(TriageProposal::parse(&body).unwrap(), proposal);
    }

    #[test]
    fn rejects_multiple_sentences_empty_steps_and_hostile_paths() {
        let mut invalid = proposal();
        invalid.summary = "One sentence. Another sentence.".to_string();
        assert!(invalid
            .validate()
            .unwrap_err()
            .contains("one plain sentence"));

        invalid = proposal();
        invalid.prepared_steps.clear();
        assert!(invalid.validate().unwrap_err().contains("at least one"));

        invalid = proposal();
        let PreparedStep::FileEdit { path, .. } = &mut invalid.prepared_steps[0] else {
            unreachable!()
        };
        *path = PathBuf::from("../outside");
        assert!(invalid.validate().unwrap_err().contains("unsafe"));
    }

    #[test]
    fn stored_proposal_is_atomically_replaced_across_states() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(TRIAGE_PROPOSAL_FILE);
        let mut stored = StoredTriageProposal {
            ticket_id: "T-1".to_string(),
            source_attempt_lease: AttemptLease {
                ticket_id: "T-1".to_string(),
                attempt_id: 2,
            },
            state: ProposalState::Pending,
            proposal: proposal(),
        };
        write_stored_proposal(&path, &stored).unwrap();
        assert_eq!(read_stored_proposal(&path).unwrap(), Some(stored.clone()));

        stored.state = ProposalState::Dismissed;
        write_stored_proposal(&path, &stored).unwrap();
        assert_eq!(read_stored_proposal(&path).unwrap(), Some(stored));
    }
}
