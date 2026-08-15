//! Effort-level vocabulary (T-071-01-01).
//!
//! `Effort` names how hard an agent should think, mirroring [`crate::client::AgentClient`]'s
//! shape: a small fixed vocabulary, parsed once at `.lisa.toml` read time so an
//! unknown value is refused with the file and key named, rather than failing at
//! the far end of a run that has already started four panes.
//!
//! Unlike a model name — provider vocabulary Lisa deliberately never
//! interprets — effort is a fixed set the client itself defines. Storage stays
//! a raw `Option<String>` everywhere else (`Ticket`, `PluginConfig`,
//! `ResolvedRoute`), exactly mirroring `model`'s plumbing; this module exists
//! only to give that string a validated, actionable gate.

use std::fmt;

/// One effort level, in the vocabulary the installed `claude` accepts today
/// (`claude --help`: `low`, `medium`, `high`, `xhigh`, `max`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    /// The valid effort names, in the order surfaced to users.
    pub const VALID: &'static [&'static str] = &["low", "medium", "high", "xhigh", "max"];

    /// Parse an effort name. Trims surrounding whitespace and is
    /// case-insensitive.
    ///
    /// Returns an actionable error naming the offending value and the valid
    /// set, so `.lisa.toml` validation can surface it directly.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Effort::Low),
            "medium" => Ok(Effort::Medium),
            "high" => Ok(Effort::High),
            "xhigh" => Ok(Effort::Xhigh),
            "max" => Ok(Effort::Max),
            other => Err(format!(
                "unknown effort '{}'; valid levels: {}",
                other,
                Self::VALID.join(", ")
            )),
        }
    }

    /// The canonical lowercase name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_levels() {
        assert_eq!(Effort::parse("low").unwrap(), Effort::Low);
        assert_eq!(Effort::parse("medium").unwrap(), Effort::Medium);
        assert_eq!(Effort::parse("high").unwrap(), Effort::High);
        assert_eq!(Effort::parse("xhigh").unwrap(), Effort::Xhigh);
        assert_eq!(Effort::parse("max").unwrap(), Effort::Max);
    }

    #[test]
    fn parse_is_case_and_whitespace_insensitive() {
        assert_eq!(Effort::parse("  High ").unwrap(), Effort::High);
    }

    #[test]
    fn parse_unknown_names_the_value_and_valid_set() {
        let error = Effort::parse("extreme").unwrap_err();
        assert!(error.contains("extreme"), "{error}");
        assert!(error.contains("low"), "{error}");
        assert!(error.contains("max"), "{error}");
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(Effort::High.to_string(), "high");
    }
}
