//! The environment an agent must not inherit from the session that started it.
//!
//! A coding agent publishes its own identity into its environment, and every
//! process it starts inherits it. That is correct for the agent's own tools and
//! wrong for a scheduler: when `lisa loop` is started *from inside* an agent
//! session, the marker travels down the whole tree — terminal multiplexer,
//! panes, and the fresh agents Lisa launches in them. Each of those agents then
//! reads the parent's identity as its own and treats itself as a nested child.
//!
//! Measured on 2026-08-11 (the field failure in `T-062-01-04`): a pane spawned
//! by `wezterm cli spawn` from inside a Claude Code session carried five
//! `CLAUDE_CODE_*` variables. The agents Lisa launched in that pane announced
//!
//! ```text
//! ⚠ Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION marker
//! ```
//!
//! and wrote no transcript, so the three tickets that failed underneath failed
//! invisibly. Restarted with those variables removed, the same tickets ran.
//!
//! Lisa cannot fix the environment it was started in, and it should not try. It
//! can refuse to pass it on, which is what this module names: the exact set of
//! variables that means "you are inside somebody else's agent session", so both
//! the loop and every launch payload can drop them on the way down.
//!
//! Deliberately a *set of names*, not a heuristic. `CODEX_HOME` is a legitimate
//! configuration variable an operator sets on purpose, so nothing here strips a
//! provider's configuration — only the per-session identity markers.

/// Name prefixes whose every variable marks a live agent session.
///
/// Claude Code publishes a family of them (`CLAUDE_CODE_SESSION_ID`,
/// `CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_ENTRYPOINT`,
/// `CLAUDE_CODE_MESSAGING_SOCKET`, and more between releases). Matching the
/// prefix rather than a fixed list is what keeps this correct when the provider
/// adds another one: a variable Lisa has never heard of is exactly the kind
/// that would otherwise be inherited silently.
pub const NESTED_SESSION_PREFIXES: &[&str] = &["CLAUDE_CODE_"];

/// Exact names that carry no prefix but mean the same thing.
pub const NESTED_SESSION_NAMES: &[&str] = &["CLAUDECODE"];

/// True when `name` marks the caller as running inside an agent session.
pub fn is_nested_session_marker(name: &str) -> bool {
    NESTED_SESSION_NAMES.contains(&name)
        || NESTED_SESSION_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

/// Every marker among `names`, sorted and deduplicated.
///
/// Sorted because this list is printed to an operator and written into launch
/// payloads: an order that changes between runs would make two identical
/// launches look different.
pub fn markers_among<I, S>(names: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut found: Vec<String> = names
        .into_iter()
        .map(|name| name.as_ref().to_string())
        .filter(|name| is_nested_session_marker(name))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Every marker in this process's own environment.
pub fn inherited_markers() -> Vec<String> {
    markers_among(std::env::vars().map(|(name, _)| name))
}

/// One operator sentence naming what was dropped, or `None` when the
/// environment was already clean and there is nothing to report.
pub fn describe_cleared(markers: &[String]) -> Option<String> {
    if markers.is_empty() {
        return None;
    }
    Some(format!(
        "Cleared {} inherited agent session marker{} so panes start clean: {}",
        markers.len(),
        if markers.len() == 1 { "" } else { "s" },
        markers.join(", "),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_family_is_matched_by_prefix_so_a_new_one_is_covered() {
        // Every variable measured in the field failure, plus one this build has
        // never seen: the prefix rule is what makes the unknown one safe.
        for name in [
            "CLAUDECODE",
            "CLAUDE_CODE_CHILD_SESSION",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_EXECPATH",
            "CLAUDE_CODE_MESSAGING_SOCKET",
            "CLAUDE_CODE_MESSAGING_TOKEN",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_CODE_SOMETHING_NOT_INVENTED_YET",
        ] {
            assert!(is_nested_session_marker(name), "{name} must be a marker");
        }
    }

    #[test]
    fn provider_configuration_and_ordinary_variables_are_not_markers() {
        // CODEX_HOME and ANTHROPIC_API_KEY are things an operator sets on
        // purpose. Stripping configuration would break the run this is meant
        // to protect.
        for name in [
            "CODEX_HOME",
            "ANTHROPIC_API_KEY",
            "CLAUDE_EFFORT",
            "PATH",
            "HOME",
            "LISA_PROJECT",
            "claude_code_session_id",
        ] {
            assert!(
                !is_nested_session_marker(name),
                "{name} must not be a marker"
            );
        }
    }

    #[test]
    fn markers_among_returns_a_sorted_deduplicated_list() {
        let found = markers_among([
            "PATH",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDECODE",
            "CLAUDE_CODE_SESSION_ID",
            "HOME",
        ]);
        assert_eq!(
            found,
            vec![
                "CLAUDECODE".to_string(),
                "CLAUDE_CODE_SESSION_ID".to_string()
            ]
        );
    }

    #[test]
    fn a_clean_environment_reports_nothing() {
        assert_eq!(markers_among(["PATH", "HOME"]), Vec::<String>::new());
        assert_eq!(describe_cleared(&[]), None);
    }

    #[test]
    fn the_operator_sentence_names_every_marker_it_dropped() {
        let sentence = describe_cleared(&[
            "CLAUDECODE".to_string(),
            "CLAUDE_CODE_CHILD_SESSION".to_string(),
        ])
        .unwrap();
        assert!(sentence.contains("2 inherited agent session markers"));
        assert!(sentence.contains("CLAUDECODE, CLAUDE_CODE_CHILD_SESSION"));

        let single = describe_cleared(&["CLAUDECODE".to_string()]).unwrap();
        assert!(single.contains("1 inherited agent session marker so"));
    }
}
