//! Per-ticket `(provider, model)` route resolution (T-026-01).
//!
//! A ticket may name a routing hint in its frontmatter (`agent:` / `model:`).
//! At spawn time lisa resolves that hint against the **loop-level default**
//! (T-025-01) into a concrete [`ResolvedRoute`]: which agent client will
//! actually run, which model rides within it, and — crucially — whether a
//! substitution happened so the fallback is visible.
//!
//! # Precedence (acceptance criterion 2)
//!
//! `ticket agent → loop default → native Claude`. "Native Claude" is not a
//! third code branch: the loop default (`PluginConfig.client`) itself defaults
//! to Claude, so two tiers of code express three tiers of intent.
//!
//! # Fallback, never failure (epic Decision 3)
//!
//! An **invalid** `agent` (a value [`AgentClient::parse`] rejects) falls back to
//! the loop default with [`ResolvedRoute::substituted`] set — it never fails the
//! ticket. The raw requested value is preserved in
//! [`ResolvedRoute::requested_agent`] so the substitution surfaces in the
//! dashboard and the provenance ledger (requested vs. actual, T-027-01).
//!
//! **Unavailable** routes (a provider whose binary is absent, or a model the
//! provider rejects) cannot be probed from inside the WASM plugin at spawn, so
//! they are *not* validated here — a missing binary is a `lisa doctor` concern
//! and a bad model surfaces at runtime in the signal files / provenance. The
//! resolver is a vocabulary resolver, not an availability oracle.
//!
//! # Vocabulary-only
//!
//! The resolver never interprets the model string; it passes it through so the
//! adapter can map it to the right flag (`--model` for Claude, the model flag
//! for the Codex wrapper). This keeps model vocabulary out of `lisa-core`.

use serde::{Deserialize, Serialize};

use crate::client::AgentClient;
use crate::types::Ticket;

/// The concrete route a ticket resolved to at spawn, plus what was requested so
/// fallbacks are visible. Shared by the spawn-time adapter resolver and the
/// provenance ledger (T-027-01), which records requested vs. actual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRoute {
    /// The agent client that will actually run this ticket.
    pub agent: AgentClient,
    /// The model to run within the agent's provider, or `None` for the
    /// provider's own default. Opaque; the adapter maps it to a flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The raw `agent:` value requested by the ticket, as written, or `None` if
    /// the ticket named no agent. Preserved even when invalid so the requested
    /// vs. actual route is visible in the dashboard and provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_agent: Option<String>,
    /// True iff the ticket requested an agent that was invalid and we fell back
    /// to the loop default.
    #[serde(default)]
    pub substituted: bool,
    /// A human-readable reason when `substituted`, for the activity log and the
    /// dashboard. `None` when no substitution happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl ResolvedRoute {
    /// Resolve a ticket's routing hint against the loop-level default agent.
    ///
    /// See the [module docs](self) for precedence and fallback semantics.
    pub fn resolve(ticket: &Ticket, default_agent: AgentClient) -> Self {
        let model = ticket.model.clone();
        match ticket.agent.as_deref() {
            // No hint → the loop default. Identical to the pre-routing path.
            None => Self {
                agent: default_agent,
                model,
                requested_agent: None,
                substituted: false,
                note: None,
            },
            Some(raw) => match AgentClient::parse(raw) {
                Ok(agent) => Self {
                    agent,
                    model,
                    requested_agent: Some(raw.to_string()),
                    substituted: false,
                    note: None,
                },
                // Invalid provider → fall back to the loop default, surfaced.
                Err(err) => Self {
                    agent: default_agent,
                    model,
                    requested_agent: Some(raw.to_string()),
                    substituted: true,
                    note: Some(format!(
                        "ticket route {}; using loop default '{}'",
                        err, default_agent
                    )),
                },
            },
        }
    }

    /// A compact cell for the dashboard thread table: `claude`, `codex/gpt-5`,
    /// or `codex/gpt-5*` when the route was a substituted fallback.
    pub fn display_cell(&self) -> String {
        let mut s = match &self.model {
            Some(m) => format!("{}/{}", self.agent, m),
            None => self.agent.to_string(),
        };
        if self.substituted {
            s.push('*');
        }
        s
    }
}

/// Free-function alias for [`ResolvedRoute::resolve`], for call sites that read
/// better as `resolve_route(ticket, default)`.
pub fn resolve_route(ticket: &Ticket, default_agent: AgentClient) -> ResolvedRoute {
    ResolvedRoute::resolve(ticket, default_agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket_with(agent: Option<&str>, model: Option<&str>) -> Ticket {
        let mut t = Ticket::new("T-026-01", "routed");
        t.agent = agent.map(|s| s.to_string());
        t.model = model.map(|s| s.to_string());
        t
    }

    #[test]
    fn no_hint_resolves_to_loop_default() {
        let t = ticket_with(None, None);
        let r = resolve_route(&t, AgentClient::Claude);
        assert_eq!(r.agent, AgentClient::Claude);
        assert!(!r.substituted);
        assert_eq!(r.requested_agent, None);
        assert_eq!(r.model, None);
        assert_eq!(r.note, None);
    }

    #[test]
    fn no_hint_uses_whatever_default_is() {
        // Under a Codex loop default, an unrouted ticket runs Codex.
        let t = ticket_with(None, None);
        let r = resolve_route(&t, AgentClient::Codex);
        assert_eq!(r.agent, AgentClient::Codex);
        assert!(!r.substituted);
    }

    #[test]
    fn valid_ticket_agent_overrides_default() {
        let t = ticket_with(Some("codex"), None);
        let r = resolve_route(&t, AgentClient::Claude);
        assert_eq!(r.agent, AgentClient::Codex);
        assert!(!r.substituted);
        assert_eq!(r.requested_agent.as_deref(), Some("codex"));
    }

    #[test]
    fn model_passes_through_untouched() {
        let t = ticket_with(Some("claude"), Some("opus"));
        let r = resolve_route(&t, AgentClient::Claude);
        assert_eq!(r.model.as_deref(), Some("opus"));
        // Model rides even when the agent falls back.
        let t2 = ticket_with(Some("bogus"), Some("gpt-5"));
        let r2 = resolve_route(&t2, AgentClient::Codex);
        assert_eq!(r2.model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn invalid_agent_falls_back_to_default_and_is_surfaced() {
        let t = ticket_with(Some("gpt"), None);
        let r = resolve_route(&t, AgentClient::Claude);
        assert_eq!(r.agent, AgentClient::Claude, "fell back to loop default");
        assert!(r.substituted);
        assert_eq!(r.requested_agent.as_deref(), Some("gpt"));
        let note = r.note.expect("substitution carries a note");
        assert!(note.contains("gpt"), "note names the bad value: {note}");
        assert!(note.contains("claude"), "note names the default: {note}");
    }

    #[test]
    fn invalid_agent_falls_back_to_codex_default() {
        // Fallback target is whatever the loop default is, not hardcoded Claude.
        let t = ticket_with(Some("nope"), None);
        let r = resolve_route(&t, AgentClient::Codex);
        assert_eq!(r.agent, AgentClient::Codex);
        assert!(r.substituted);
    }

    #[test]
    fn display_cell_variants() {
        let claude = resolve_route(&ticket_with(Some("claude"), None), AgentClient::Claude);
        assert_eq!(claude.display_cell(), "claude");

        let codex_model = resolve_route(
            &ticket_with(Some("codex"), Some("gpt-5")),
            AgentClient::Claude,
        );
        assert_eq!(codex_model.display_cell(), "codex/gpt-5");

        let substituted = resolve_route(
            &ticket_with(Some("gpt"), Some("gpt-5")),
            AgentClient::Claude,
        );
        // Fell back to claude, but model still rides; trailing * marks the fallback.
        assert_eq!(substituted.display_cell(), "claude/gpt-5*");
    }
}
