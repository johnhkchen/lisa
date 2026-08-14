//! Whether the Lisa on this box is the Lisa its channel names.
//!
//! [`crate::channel`] holds the rules — which release each channel picks out of
//! the published list — and [`crate::upgrade`] is the command that acts on them.
//! This module is the third thing a fleet needs and the one that was missing:
//! the answer to *is this machine keeping the intention it recorded*, in a shape
//! `lisa doctor` can print as one row and `lisa doctor --json` can hand to a
//! script.
//!
//! It reports and nothing else. Nothing here writes config, downloads an
//! artifact, or runs an installer; a diagnostic that changes the system it is
//! diagnosing is a diagnostic you cannot trust twice.
//!
//! ## The five things that can be true
//!
//! | state | what it means |
//! | --- | --- |
//! | `level` | the installed version is the one the channel resolves to |
//! | `behind` | the channel names a newer release than the one installed |
//! | `ahead` | this build is newer than its channel — the dev desk, building `main` |
//! | `waiting` | the channel names no release this cycle (nightly, mid-soak) |
//! | `unresolved` | the release list could not be read at all — no network |
//!
//! Only `behind` carries a remedy, because it is the only one with a gap to
//! settle. `unresolved` is deliberately not `level`: a machine that could not
//! look has not been told it is current.

use semver::Version;

use crate::channel::{self, Channel, MachineConfig, Release};

/// What the machine config says about this box's channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Setting {
    /// The machine chose this channel.
    Chosen(Channel),
    /// Nothing has chosen. The state every machine in the fleet is in today,
    /// and never silently rendered as `stable`.
    Unset,
    /// A machine config that exists and cannot be read, carrying the reason.
    Unreadable(String),
}

impl Setting {
    /// The channel to resolve against: the chosen one, or [`Channel::DEFAULT`].
    pub(crate) fn effective(&self) -> Channel {
        match self {
            Setting::Chosen(channel) => *channel,
            Setting::Unset | Setting::Unreadable(_) => Channel::DEFAULT,
        }
    }

    /// The channel name for a machine reader, or `None` when none is set.
    pub(crate) fn name(&self) -> Option<&'static str> {
        match self {
            Setting::Chosen(channel) => Some(channel.as_str()),
            Setting::Unset | Setting::Unreadable(_) => None,
        }
    }

    /// How the row names the channel.
    fn label(&self) -> String {
        match self {
            Setting::Chosen(channel) => format!("channel {channel}"),
            Setting::Unset => format!("channel unset (treated as {})", Channel::DEFAULT),
            Setting::Unreadable(reason) => format!("channel unreadable ({reason})"),
        }
    }
}

/// Where this box stands against the channel it is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum State {
    /// The installed version is what the channel resolves to.
    Level { release: Release },
    /// The channel resolves to a newer release than the one installed.
    Behind { release: Release },
    /// This build is newer than anything the channel names.
    Ahead { release: Release },
    /// The channel resolves to no release right now, and why.
    Waiting { reason: String },
    /// The release list could not be read, and why.
    Unresolved { reason: String },
}

impl State {
    /// The one-word name a script reads.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            State::Level { .. } => "level",
            State::Behind { .. } => "behind",
            State::Ahead { .. } => "ahead",
            State::Waiting { .. } => "waiting",
            State::Unresolved { .. } => "unresolved",
        }
    }

    /// The release the channel resolves to, when it resolves to one.
    fn release(&self) -> Option<&Release> {
        match self {
            State::Level { release } | State::Behind { release } | State::Ahead { release } => {
                Some(release)
            }
            State::Waiting { .. } | State::Unresolved { .. } => None,
        }
    }
}

/// The whole answer: what is installed, what this machine asked for, and how
/// far apart those two are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Freshness {
    pub(crate) installed: Version,
    pub(crate) setting: Setting,
    pub(crate) state: State,
}

/// Compare the installed Lisa against the release list its channel selects from.
///
/// `releases` is a `Result` because "the list could not be read" is an answer
/// this check has to give honestly rather than a failure to hide: `now` and the
/// list are passed in so every case is testable without a clock or a network.
pub(crate) fn assess(
    installed: Version,
    setting: Setting,
    config: &MachineConfig,
    releases: Result<&[Release], String>,
    now: i64,
) -> Freshness {
    let selected = setting.effective();
    let state = match releases {
        Err(reason) => State::Unresolved { reason },
        Ok(releases) => {
            let resolution = channel::resolve(selected, releases, now, config.soak());
            match resolution.release() {
                None => State::Waiting {
                    reason: resolution
                        .waiting_reason()
                        .unwrap_or("the channel resolves to no release")
                        .to_string(),
                },
                Some(release) => {
                    let release = release.clone();
                    if release.version == installed {
                        State::Level { release }
                    } else if release.version > installed {
                        State::Behind { release }
                    } else {
                        State::Ahead { release }
                    }
                }
            }
        }
    };

    Freshness {
        installed,
        setting,
        state,
    }
}

/// Read this machine's config for the check, keeping an unreadable one as a
/// reportable state rather than an error that hides the rest of `doctor`.
pub(crate) fn machine_setting(config: &Result<MachineConfig, String>) -> Setting {
    match config {
        Ok(config) => match config.channel {
            Some(channel) => Setting::Chosen(channel),
            None => Setting::Unset,
        },
        Err(reason) => Setting::Unreadable(reason.clone()),
    }
}

impl Freshness {
    /// The row's detail: the channel, the version installed, and the version
    /// that channel currently resolves to — the Zellij row's shape, asked of
    /// Lisa itself.
    pub(crate) fn summary(&self) -> String {
        let head = format!("{}, installed {}", self.setting.label(), self.installed);
        let selected = self.setting.effective();
        match &self.state {
            State::Level { release } => {
                format!("{head}, {selected} resolves to {}", release.tag)
            }
            State::Behind { release } => format!(
                "{head}, {selected} resolves to {} — this machine is behind its channel",
                release.tag
            ),
            State::Ahead { release } => format!(
                "{head}, {selected} resolves to {} — this build is newer than its channel",
                release.tag
            ),
            State::Waiting { reason } => {
                format!("{head}, {selected} is not moving this cycle: {reason}")
            }
            State::Unresolved { reason } => format!(
                "{head}, {selected} could not be resolved: {reason}. \
                 That is what is unknown, not proof this machine is current"
            ),
        }
    }

    /// The command that settles the gap, when there is a gap to settle.
    ///
    /// `doctor`'s idiom: a finding never appears without the thing to do about
    /// it. A machine that has not chosen a channel is asked to choose one in the
    /// same command, because moving it without one only re-runs the accident
    /// that made the fleet stale.
    pub(crate) fn remedy(&self) -> Option<String> {
        let State::Behind { release } = &self.state else {
            return None;
        };
        Some(match self.setting {
            Setting::Chosen(_) => {
                format!("Move this machine to {}:\n    lisa upgrade", release.tag)
            }
            _ => format!(
                "Say which Lisa this machine wants, and move to it in the same command:\n    \
                 lisa upgrade --channel <name>    ({})",
                Channel::ALL
                    .iter()
                    .map(|channel| channel.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
    }

    /// The same fields, for a script reading `lisa doctor --json`.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let release = self.state.release();
        serde_json::json!({
            "installed": self.installed.to_string(),
            "channel": self.setting.name(),
            "effective_channel": self.setting.effective().as_str(),
            "channel_error": match &self.setting {
                Setting::Unreadable(reason) => Some(reason.clone()),
                _ => None,
            },
            "state": self.state.name(),
            "resolved_tag": release.map(|release| release.tag.clone()),
            "resolved_version": release.map(|release| release.version.to_string()),
            "detail": self.summary(),
            "remedy": self.remedy(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;
    const NOW: i64 = 1_786_000_000;

    fn config() -> MachineConfig {
        MachineConfig::default()
    }

    /// The fleet as measured on 2026-08-14: v0.4.4 is the newest release that
    /// is not a prerelease, v0.5.0-rc.2 is the newest tag, and both are old
    /// enough to have soaked.
    fn fleet() -> Vec<Release> {
        vec![
            Release::from_tag("v0.4.4", NOW - 26 * DAY).unwrap(),
            Release::from_tag("v0.5.0-rc.2", NOW - 5 * DAY).unwrap(),
        ]
    }

    fn assess_at(
        installed: &str,
        setting: Setting,
        releases: Result<&[Release], String>,
    ) -> Freshness {
        assess(
            Version::parse(installed).unwrap(),
            setting,
            &config(),
            releases,
            NOW,
        )
    }

    /// The machine the story is about: installed a month ago from the shell
    /// installer, never told what it is for, and 26 days behind without anything
    /// on it saying so.
    #[test]
    fn a_box_behind_its_channel_is_named_and_carries_the_command_that_settles_it() {
        let releases = fleet();
        let check = assess_at("0.4.3", Setting::Chosen(Channel::Stable), Ok(&releases));

        assert_eq!(check.state.name(), "behind");
        let summary = check.summary();
        assert!(summary.contains("channel stable"), "{summary}");
        assert!(summary.contains("installed 0.4.3"), "{summary}");
        assert!(summary.contains("resolves to v0.4.4"), "{summary}");

        let remedy = check.remedy().expect("a behind box has something to do");
        assert!(remedy.contains("lisa upgrade"), "{remedy}");
        assert!(remedy.contains("v0.4.4"), "{remedy}");
    }

    #[test]
    fn a_box_level_with_its_channel_is_quiet_and_has_nothing_to_do() {
        let releases = fleet();
        let check = assess_at("0.4.4", Setting::Chosen(Channel::Stable), Ok(&releases));

        assert_eq!(check.state.name(), "level");
        assert_eq!(check.remedy(), None);
        assert!(check.summary().contains("resolves to v0.4.4"));
    }

    /// The dev desk builds `main`, so it is routinely ahead of every channel.
    /// That is not drift and must never read as one.
    #[test]
    fn a_build_newer_than_its_channel_is_said_plainly_and_is_not_a_gap() {
        let releases = fleet();
        let check = assess_at(
            "0.6.0-rc.1",
            Setting::Chosen(Channel::Stable),
            Ok(&releases),
        );

        assert_eq!(check.state.name(), "ahead");
        assert_eq!(check.remedy(), None);
        assert!(check.summary().contains("newer than its channel"));
    }

    #[test]
    fn an_unset_channel_reads_as_unset_and_asks_for_one_when_it_is_behind() {
        let releases = fleet();
        let check = assess_at("0.4.3", Setting::Unset, Ok(&releases));

        let summary = check.summary();
        assert!(summary.contains("channel unset"), "{summary}");
        assert!(
            summary.contains("treated as stable"),
            "an unset channel still has to say which rule was applied: {summary}"
        );
        assert_eq!(check.state.name(), "behind");

        let remedy = check.remedy().unwrap();
        assert!(remedy.contains("lisa upgrade --channel <name>"), "{remedy}");
        assert!(remedy.contains("canary, nightly, stable"), "{remedy}");
        assert!(check.to_json()["channel"].is_null());
        assert_eq!(check.to_json()["effective_channel"], "stable");
    }

    /// A machine that has not chosen and happens to be level is still level:
    /// drift reporting that cries every run is drift reporting nobody reads.
    #[test]
    fn an_unset_channel_that_is_level_stays_quiet() {
        let releases = fleet();
        let check = assess_at("0.4.4", Setting::Unset, Ok(&releases));

        assert_eq!(check.state.name(), "level");
        assert_eq!(check.remedy(), None);
        assert!(check.summary().contains("channel unset"));
    }

    #[test]
    fn with_no_network_it_says_so_and_claims_nothing_about_being_level() {
        let check = assess_at(
            "0.4.4",
            Setting::Chosen(Channel::Nightly),
            Err("cannot read the release list at https://api.github.com/…: dns error".to_string()),
        );

        assert_eq!(check.state.name(), "unresolved");
        assert_eq!(check.remedy(), None);
        let summary = check.summary();
        assert!(summary.contains("installed 0.4.4"), "{summary}");
        assert!(summary.contains("could not be resolved"), "{summary}");
        assert!(summary.contains("dns error"), "{summary}");
        assert!(
            !summary.contains("resolves to v"),
            "an unreachable list must not name a resolved release: {summary}"
        );

        let json = check.to_json();
        assert_eq!(json["state"], "unresolved");
        assert!(json["resolved_tag"].is_null());
        assert_eq!(json["installed"], "0.4.4");
    }

    /// Nightly mid-soak resolves to nothing. The box is not behind — there is
    /// nothing yet for it to be behind — and `doctor` says which it is.
    #[test]
    fn a_channel_that_is_holding_this_cycle_is_not_reported_as_a_gap() {
        let mut releases = fleet();
        releases.push(Release::from_tag("v0.5.0-rc.3", NOW - HOUR).unwrap());
        let check = assess_at("0.4.4", Setting::Chosen(Channel::Nightly), Ok(&releases));

        assert_eq!(check.state.name(), "waiting");
        assert_eq!(check.remedy(), None);
        assert!(check.summary().contains("not moving this cycle"));
        assert!(check.summary().contains("v0.5.0-rc.3"));
    }

    #[test]
    fn an_unreadable_machine_config_is_named_rather_than_read_as_unset() {
        let check = assess_at(
            "0.4.4",
            Setting::Unreadable("unknown channel \"beta\"".to_string()),
            Err("not looked up".to_string()),
        );

        let summary = check.summary();
        assert!(summary.contains("channel unreadable"), "{summary}");
        assert!(summary.contains("beta"), "{summary}");
        assert_eq!(check.to_json()["channel_error"], "unknown channel \"beta\"");
    }

    #[test]
    fn the_json_carries_every_field_the_row_shows() {
        let releases = fleet();
        let json = assess_at("0.4.3", Setting::Chosen(Channel::Canary), Ok(&releases)).to_json();

        assert_eq!(json["installed"], "0.4.3");
        assert_eq!(json["channel"], "canary");
        assert_eq!(json["effective_channel"], "canary");
        assert_eq!(json["state"], "behind");
        assert_eq!(json["resolved_tag"], "v0.5.0-rc.2");
        assert_eq!(json["resolved_version"], "0.5.0-rc.2");
        assert!(json["remedy"].as_str().unwrap().contains("lisa upgrade"));
        assert!(json["channel_error"].is_null());
    }
}
