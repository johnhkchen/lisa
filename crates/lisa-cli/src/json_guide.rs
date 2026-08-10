use crate::templates;

/// Print the embedded `--json` guide to stdout.
///
/// Pure dump, like [`crate::hooks_guide`]: the contract is identical for every
/// project, so there is nothing to detect. Returns `Result<(), String>` for
/// dispatch uniformity with the other command handlers; it cannot currently
/// fail.
pub fn run_json_guide() -> Result<(), String> {
    print!("{}", templates::JSON_GUIDE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_output::{JSON_SCHEMA, JSON_SCHEMA_VERSION};

    #[test]
    fn json_guide_prints() {
        assert!(run_json_guide().is_ok());
        assert!(!templates::JSON_GUIDE.is_empty());
    }

    /// The guide names the exact marker the documents carry. A guide that
    /// disagrees with the envelope is worse than no guide.
    #[test]
    fn json_guide_names_the_live_schema_marker() {
        assert!(
            templates::JSON_GUIDE.contains(JSON_SCHEMA),
            "guide must name the schema marker {JSON_SCHEMA}"
        );
        assert!(
            templates::JSON_GUIDE.contains(&format!("\"schema_version\": {JSON_SCHEMA_VERSION}")),
            "guide must show the live schema_version {JSON_SCHEMA_VERSION}"
        );
    }

    /// The three stability rules a consumer needs are written down, not implied.
    #[test]
    fn json_guide_states_the_field_stability_rules() {
        let guide = templates::JSON_GUIDE;
        for marker in [
            "Ignore fields you do not know",
            "A field disappearing",
            "Exit status",
            "lisa status --json",
            "lisa validate --json",
        ] {
            assert!(guide.contains(marker), "guide must state: {marker}");
        }
    }
}
