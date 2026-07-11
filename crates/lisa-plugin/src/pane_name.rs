//! Deterministic, scheduler-owned names for coding terminal panes.
//!
//! Assigned names keep the actual resolved agent and complete ticket ID as
//! stable scan keys. Only the human title is sanitized and shortened. The
//! normal display bound is measured in Unicode scalar values so truncation
//! never slices invalid UTF-8. Canonical Lisa ticket IDs keep the immutable
//! prefix well below the bound; if malformed external input makes the prefix
//! itself too long, preserving the complete scan keys takes precedence.

use lisa_core::client::AgentClient;

/// Maximum normal pane-name length, measured in Unicode scalar values.
pub(crate) const MAX_PANE_NAME_CHARS: usize = 80;

/// Semantic pane lifecycle state consumed by the single name formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneName<'a> {
    Assigned {
        agent: AgentClient,
        ticket_id: &'a str,
        title: &'a str,
    },
    Idle {
        resident_agent: Option<AgentClient>,
    },
}

/// Format an assigned or idle pane name using the canonical lifecycle contract.
pub(crate) fn format_pane_name(input: PaneName<'_>) -> String {
    match input {
        PaneName::Idle {
            resident_agent: Some(agent),
        } => format!("{agent} · idle"),
        PaneName::Idle {
            resident_agent: None,
        } => "lisa · idle".to_string(),
        PaneName::Assigned {
            agent,
            ticket_id,
            title,
        } => {
            let prefix = format!("{agent} · {ticket_id} · ");
            let title = sanitize_title(title);
            let prefix_len = prefix.chars().count();
            let title_len = title.chars().count();

            if prefix_len + title_len <= MAX_PANE_NAME_CHARS {
                return prefix + &title;
            }

            // Keep the complete provider + ticket scan keys even for malformed,
            // pathologically long IDs. Canonical Lisa IDs never reach this case.
            if prefix_len >= MAX_PANE_NAME_CHARS {
                return prefix.trim_end().to_string();
            }

            let keep = MAX_PANE_NAME_CHARS - prefix_len - 1;
            let shortened: String = title.chars().take(keep).collect();
            format!("{prefix}{shortened}…")
        }
    }
}

fn sanitize_title(title: &str) -> String {
    let mut sanitized = String::new();
    let mut pending_space = false;

    for ch in title.chars() {
        if ch.is_control() || ch.is_whitespace() {
            pending_space = !sanitized.is_empty();
            continue;
        }
        if pending_space {
            sanitized.push(' ');
            pending_space = false;
        }
        sanitized.push(ch);
    }

    if sanitized.is_empty() {
        "untitled".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_assigned_and_idle_names() {
        assert_eq!(
            format_pane_name(PaneName::Assigned {
                agent: AgentClient::Claude,
                ticket_id: "T-032-01",
                title: "zellij-pane-lifecycle-names",
            }),
            "claude · T-032-01 · zellij-pane-lifecycle-names"
        );
        assert_eq!(
            format_pane_name(PaneName::Assigned {
                agent: AgentClient::Codex,
                ticket_id: "T-1",
                title: "work",
            }),
            "codex · T-1 · work"
        );
        assert_eq!(
            format_pane_name(PaneName::Idle {
                resident_agent: Some(AgentClient::Claude),
            }),
            "claude · idle"
        );
        assert_eq!(
            format_pane_name(PaneName::Idle {
                resident_agent: Some(AgentClient::Codex),
            }),
            "codex · idle"
        );
        assert_eq!(
            format_pane_name(PaneName::Idle {
                resident_agent: None,
            }),
            "lisa · idle"
        );
    }

    #[test]
    fn sanitizes_controls_and_collapses_whitespace() {
        let name = format_pane_name(PaneName::Assigned {
            agent: AgentClient::Codex,
            ticket_id: "T-1",
            title: " \n alpha\t\x1b beta\r\n gamma \u{85} ",
        });
        assert_eq!(name, "codex · T-1 · alpha beta gamma");
        assert!(!name.chars().any(char::is_control));
    }

    #[test]
    fn control_only_title_becomes_untitled() {
        assert_eq!(
            format_pane_name(PaneName::Assigned {
                agent: AgentClient::Claude,
                ticket_id: "T-1",
                title: "\n\t\x1b",
            }),
            "claude · T-1 · untitled"
        );
    }

    #[test]
    fn truncates_only_title_at_unicode_scalar_boundary() {
        let ticket_id = "T-032-01";
        let name = format_pane_name(PaneName::Assigned {
            agent: AgentClient::Claude,
            ticket_id,
            title: &"界".repeat(100),
        });
        assert_eq!(name.chars().count(), MAX_PANE_NAME_CHARS);
        assert!(name.starts_with(&format!("claude · {ticket_id} · ")));
        assert!(name.ends_with('…'));
    }

    #[test]
    fn exact_limit_is_not_truncated() {
        let prefix = "codex · T-1 · ";
        let title = "x".repeat(MAX_PANE_NAME_CHARS - prefix.chars().count());
        let name = format_pane_name(PaneName::Assigned {
            agent: AgentClient::Codex,
            ticket_id: "T-1",
            title: &title,
        });
        assert_eq!(name.chars().count(), MAX_PANE_NAME_CHARS);
        assert!(!name.ends_with('…'));
    }

    #[test]
    fn pathological_ticket_id_keeps_complete_scan_keys() {
        let ticket_id = format!("T-{}", "9".repeat(MAX_PANE_NAME_CHARS));
        let name = format_pane_name(PaneName::Assigned {
            agent: AgentClient::Codex,
            ticket_id: &ticket_id,
            title: "title",
        });
        assert!(name.starts_with("codex · "));
        assert!(name.contains(&ticket_id));
        assert!(!name.contains("title"));
    }
}
