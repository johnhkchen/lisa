//! Agent client selection vocabulary.
//!
//! `AgentClient` is the single shared value that names which agent client a loop
//! (or, later, a ticket) runs on. It lives in `lisa-core` because both readers
//! parse it here and nowhere else: the CLI (`.lisa.toml` → `ResolvedConfig`) and
//! the plugin (`PluginConfig::from_config_map`) are in different crates, and the
//! ticket for this seam (T-025-01) requires "one place both readers share, not
//! two ad-hoc parsers."
//!
//! Today the value is a **bare client name** (`claude` | `codex`). The extension
//! seam toward the S-026 `(method, provider, model)` routing vocabulary is
//! [`AgentClient::parse`]: its signature is what call sites depend on, so S-026
//! can grow the parsed representation without churning every reader.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Which agent client drives a session. Defaults to [`AgentClient::Claude`], so a
/// project that never opts in behaves exactly as it did before this seam existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentClient {
    /// Native Claude Code (hooks-driven TUI). The default and depth/reliability
    /// anchor leg.
    #[default]
    Claude,
    /// Native Codex interactive TUI with Lisa lifecycle hooks.
    Codex,
}

impl AgentClient {
    /// The valid client names, in the order surfaced to users.
    pub const VALID: &'static [&'static str] = &["claude", "codex"];

    /// Parse a client name. Trims surrounding whitespace and is case-insensitive.
    ///
    /// Returns an actionable error naming the offending value and the valid set,
    /// so `lisa validate` / `lisa loop` can surface it directly.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "claude" => Ok(AgentClient::Claude),
            "codex" => Ok(AgentClient::Codex),
            other => Err(format!(
                "unknown client '{}'; valid clients: {}",
                other,
                Self::VALID.join(", ")
            )),
        }
    }

    /// The canonical lowercase name, used when emitting the value back out (layout
    /// config block, doctor output).
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentClient::Claude => "claude",
            AgentClient::Codex => "codex",
        }
    }
}

impl fmt::Display for AgentClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_names() {
        assert_eq!(AgentClient::parse("claude").unwrap(), AgentClient::Claude);
        assert_eq!(AgentClient::parse("codex").unwrap(), AgentClient::Codex);
    }

    #[test]
    fn parse_is_case_and_whitespace_insensitive() {
        assert_eq!(AgentClient::parse("  Codex ").unwrap(), AgentClient::Codex);
        assert_eq!(AgentClient::parse("CLAUDE").unwrap(), AgentClient::Claude);
    }

    #[test]
    fn parse_unknown_is_actionable_error() {
        let err = AgentClient::parse("gpt").unwrap_err();
        assert!(err.contains("gpt"), "error names the bad value: {err}");
        assert!(
            err.contains("claude") && err.contains("codex"),
            "lists valid: {err}"
        );
    }

    #[test]
    fn default_is_claude() {
        assert_eq!(AgentClient::default(), AgentClient::Claude);
    }

    #[test]
    fn as_str_round_trips_through_parse() {
        for c in [AgentClient::Claude, AgentClient::Codex] {
            assert_eq!(AgentClient::parse(c.as_str()).unwrap(), c);
        }
    }

    #[test]
    fn serde_is_lowercase() {
        assert_eq!(
            serde_json::to_string(&AgentClient::Codex).unwrap(),
            "\"codex\""
        );
        let parsed: AgentClient = serde_json::from_str("\"claude\"").unwrap();
        assert_eq!(parsed, AgentClient::Claude);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(AgentClient::Codex.to_string(), "codex");
    }
}
