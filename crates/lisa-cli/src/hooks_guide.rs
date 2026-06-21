use crate::templates;

/// Print the embedded hooks setup guide to stdout.
///
/// Pure dump — the guide is identical for every project, so no path or project
/// detection is needed. Returns `Result<(), String>` for dispatch uniformity with
/// the other command handlers; it cannot currently fail.
pub fn run_hooks_guide() -> Result<(), String> {
    print!("{}", templates::HOOKS_GUIDE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_hooks_guide_ok() {
        assert!(run_hooks_guide().is_ok());
    }

    #[test]
    fn test_hooks_guide_non_empty() {
        assert!(!templates::HOOKS_GUIDE.is_empty());
    }

    #[test]
    fn test_hooks_guide_contains_contract_markers() {
        let g = templates::HOOKS_GUIDE;
        // The on-notify contract markers required by the ticket.
        assert!(g.contains("on-notify"));
        assert!(g.contains("LISA_EVENT"));
        // Both event kinds documented.
        assert!(g.contains("complete"));
        assert!(g.contains("attention"));
        // All four lifecycle hooks named.
        for f in ["on-idle.sh", "on-stop.sh", "on-clear.sh", "on-heartbeat.sh"] {
            assert!(g.contains(f), "guide must mention {f}");
        }
        // The opt-in enable step.
        assert!(g.contains("cp .lisa/hooks/on-notify.sample"));
    }
}
