//! Zellij runtime version parsing and support policy.
//!
//! Zellij reports its version as `zellij <semver>`. This module owns that
//! process-output grammar and the tested runtime range shared by CLI checks.

use std::fmt;

use semver::Version;

/// A parsed Zellij runtime version with semantic-version ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZellijVersion(Version);

impl ZellijVersion {
    /// Construct a stable release version for constants and tests.
    pub const fn release(major: u64, minor: u64, patch: u64) -> Self {
        Self(Version::new(major, minor, patch))
    }

    /// Parse the output of `zellij --version`.
    ///
    /// Surrounding and repeated whitespace are accepted, but the output must
    /// contain exactly the `zellij` product name and one semantic-version
    /// token. Requiring the product name prevents unrelated version output from
    /// being accepted as a compatible Zellij runtime.
    pub fn parse_command_output(output: &str) -> Result<Self, ParseZellijVersionError> {
        let mut fields = output.split_whitespace();
        let product = fields.next();
        let version = fields.next();

        if product != Some("zellij") || fields.next().is_some() {
            return Err(ParseZellijVersionError);
        }

        version
            .and_then(|version| Version::parse(version).ok())
            .map(Self)
            .ok_or(ParseZellijVersionError)
    }
}

impl fmt::Display for ZellijVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The output of `zellij --version` did not match `zellij <semver>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseZellijVersionError;

impl fmt::Display for ParseZellijVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected Zellij version output in the form `zellij <semver>`")
    }
}

impl std::error::Error for ParseZellijVersionError {}

/// The inclusive, open-ended range of Zellij runtimes Lisa supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZellijVersionRange {
    /// The oldest tested and supported Zellij release.
    pub minimum: ZellijVersion,
}

impl ZellijVersionRange {
    /// Return whether `version` is at or above the supported floor.
    pub fn contains(&self, version: &ZellijVersion) -> bool {
        version >= &self.minimum
    }
}

/// Lisa's tested Zellij runtime range.
///
/// The 0.43.0 floor matches the `zellij-tile = "0.43"` pin in
/// `crates/lisa-plugin/Cargo.toml`; review this single range declaration when
/// bumping that pin so the runtime floor and its documentation stay together.
/// Zellij 0.41.0 is the theoretical hard API floor because older hosts cannot
/// decode `write_chars_to_pane_id`/`write_to_pane_id`, but Lisa enforces the
/// tested SDK-aligned floor rather than the theoretical one.
pub const SUPPORTED_ZELLIJ_RANGE: ZellijVersionRange = ZellijVersionRange {
    minimum: ZellijVersion::release(0, 43, 0),
};

impl fmt::Display for ZellijVersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ">= {}", self.minimum)
    }
}

/// The support verdict for one `zellij --version` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZellijVersionVerdict {
    /// The output parsed and the detected version is supported.
    InRange(ZellijVersion),
    /// The output parsed, but the detected version is below the supported
    /// floor.
    BelowFloor(ZellijVersion),
    /// The output was not recognizable as a Zellij semantic version.
    Unparseable,
}

/// Parse and classify the output of `zellij --version` against Lisa's declared
/// supported range.
pub fn classify_zellij_version_output(output: &str) -> ZellijVersionVerdict {
    match ZellijVersion::parse_command_output(output) {
        Ok(version) if SUPPORTED_ZELLIJ_RANGE.contains(&version) => {
            ZellijVersionVerdict::InRange(version)
        }
        Ok(version) => ZellijVersionVerdict::BelowFloor(version),
        Err(_) => ZellijVersionVerdict::Unparseable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_releases_at_or_above_the_floor_are_in_range() {
        for output in ["zellij 0.43.0", "zellij 0.43.1\n", "  zellij   0.44.0  "] {
            assert!(
                matches!(
                    classify_zellij_version_output(output),
                    ZellijVersionVerdict::InRange(_)
                ),
                "expected {output:?} to be in range"
            );
        }
    }

    #[test]
    fn stable_release_below_the_floor_is_classified_separately() {
        assert_eq!(
            classify_zellij_version_output("zellij 0.40.1"),
            ZellijVersionVerdict::BelowFloor(ZellijVersion::release(0, 40, 1))
        );
    }

    #[test]
    fn prerelease_at_the_stable_floor_is_below_the_floor() {
        let verdict = classify_zellij_version_output("zellij 0.43.0-rc.1");
        assert!(matches!(verdict, ZellijVersionVerdict::BelowFloor(_)));
    }

    #[test]
    fn prerelease_above_the_floor_is_in_range() {
        let verdict = classify_zellij_version_output("zellij 0.44.0-rc.1");
        assert!(matches!(verdict, ZellijVersionVerdict::InRange(_)));
    }

    #[test]
    fn garbage_is_unparseable_and_never_passes() {
        for output in [
            "",
            "garbage",
            "0.43.0",
            "zellij",
            "zellij definitely-not-a-version",
            "not-zellij 0.43.0",
            "zellij 0.43.0 extra",
        ] {
            assert_eq!(
                classify_zellij_version_output(output),
                ZellijVersionVerdict::Unparseable,
                "expected {output:?} to fail closed"
            );
        }
    }

    #[test]
    fn comparison_uses_semantic_version_precedence() {
        let patch_nine = ZellijVersion::parse_command_output("zellij 0.43.9").unwrap();
        let patch_ten = ZellijVersion::parse_command_output("zellij 0.43.10").unwrap();
        let prerelease = ZellijVersion::parse_command_output("zellij 0.43.0-rc.1").unwrap();

        assert!(patch_ten > patch_nine);
        assert!(prerelease < ZellijVersion::release(0, 43, 0));
    }

    #[test]
    fn versions_and_range_have_canonical_display() {
        let version = ZellijVersion::parse_command_output("zellij 0.43.1+release.7").unwrap();
        assert_eq!(version.to_string(), "0.43.1+release.7");
        assert_eq!(SUPPORTED_ZELLIJ_RANGE.to_string(), ">= 0.43.0");
    }
}
