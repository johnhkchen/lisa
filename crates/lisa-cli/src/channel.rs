//! Which Lisa this machine wants, and which release that resolves to.
//!
//! A channel is a property of the box, not of a project. `.lisa.toml`'s
//! `version` records the Lisa that set a *project* up and keeps that meaning;
//! nothing here reads or writes it. The channel lives in a per-user file under
//! [`ProjectDirs`], next to where every other tool this machine runs keeps its
//! own settings.
//!
//! ## The three channels are selection rules over one list of tags
//!
//! `auto-release.yml` tags `v<workspace version>` on every green `main`, so the
//! tag list *is* the release history. There are no per-channel artifacts to
//! build; a channel is a rule for picking one tag out of that list:
//!
//! | channel | resolves to |
//! | --- | --- |
//! | `canary` | the newest tag, prerelease or not |
//! | `nightly` | the newest tag, prerelease or not, once it has soaked |
//! | `stable` | the newest non-prerelease tag |
//!
//! A tag is a prerelease when its semantic version carries a prerelease suffix
//! (`v0.5.0-rc.2`), which is the same claim the workspace version string makes.
//! GitHub's own `prerelease` flag is not consulted, so a fixed list of tags
//! resolves identically in a test and on a box.
//!
//! ## Soak, and what "superseded" means
//!
//! Soak is time, because there is no telemetry to lean on: a tag becomes
//! nightly-eligible once it is older than [`DEFAULT_SOAK_HOURS`] hours, which
//! `soak_hours` in the machine config changes.
//!
//! Superseded is the other half, and it is what keeps a nightly box off a tag a
//! hotfix replaced twenty minutes later: **any tag that is not the newest one
//! has been superseded**, whether or not the tag above it has soaked. So
//! `nightly` never walks backwards down the list looking for something old
//! enough. It has exactly one candidate — the newest tag — and it either has
//! soaked, in which case that is the answer, or it has not, in which case the
//! machine stays where it is this cycle and [`Resolution::Waiting`] says how
//! much longer.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;
use semver::Version;
use serde::Deserialize;

/// Hours a tag must age before `nightly` will take it.
pub(crate) const DEFAULT_SOAK_HOURS: u64 = 24;

/// Environment override for the machine config directory, so a test can run
/// against a temporary one instead of the operator's real config.
const CONFIG_DIR_ENV: &str = "LISA_CONFIG_DIR";

/// The file the channel lives in, inside the machine config directory.
const CONFIG_FILE_NAME: &str = "config.toml";

/// What a machine subscribes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    /// The newest tag, prerelease or not. Breakage is the job.
    Canary,
    /// The newest tag once it has soaked. Real workloads, nothing in front of
    /// anyone.
    Nightly,
    /// The newest non-prerelease tag. Don't change just because.
    Stable,
}

impl Channel {
    /// Every channel, in the order an operator should read them: loosest first.
    pub(crate) const ALL: [Channel; 3] = [Channel::Canary, Channel::Nightly, Channel::Stable];

    /// The channel a machine that has never said anything is treated as being
    /// on. The conservative end on purpose: an unconfigured box never gets a
    /// prerelease it did not ask for.
    pub(crate) const DEFAULT: Channel = Channel::Stable;

    /// The name written in config and typed on the command line.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Channel::Canary => "canary",
            Channel::Nightly => "nightly",
            Channel::Stable => "stable",
        }
    }

    /// One line saying what this channel resolves to.
    pub(crate) const fn rule(self) -> &'static str {
        match self {
            Channel::Canary => "the newest release, prerelease or not",
            Channel::Nightly => "the newest release, prerelease or not, once it has soaked",
            Channel::Stable => "the newest release that is not a prerelease",
        }
    }

    /// Read a channel name, naming all three when it is not one of them.
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        Channel::ALL
            .into_iter()
            .find(|channel| channel.as_str() == raw)
            .ok_or_else(|| {
                format!("unknown channel {raw:?}; the channels are canary, nightly and stable",)
            })
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One published release, reduced to what channel resolution needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Release {
    /// The tag exactly as published, `v` prefix and all.
    pub(crate) tag: String,
    /// The tag's semantic version, which carries the stability claim.
    pub(crate) version: Version,
    /// When the release was published, in seconds since the Unix epoch.
    pub(crate) published_at: i64,
}

impl Release {
    /// Build a release from a `v<semver>` tag, or `None` when the tag is not
    /// one of Lisa's. The release list carries whatever anyone ever tagged;
    /// resolution only considers tags it can order.
    pub(crate) fn from_tag(tag: &str, published_at: i64) -> Option<Self> {
        let version = Version::parse(tag.strip_prefix('v')?).ok()?;
        Some(Self {
            tag: tag.to_string(),
            version,
            published_at,
        })
    }

    /// Whether this tag claims to be unproven (`-rc.N` and friends).
    pub(crate) fn is_prerelease(&self) -> bool {
        !self.version.pre.is_empty()
    }
}

/// What a channel points at right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// The channel resolves to this release.
    Release(Release),
    /// The channel resolves to nothing right now, and this says why in one
    /// line an operator can read.
    Waiting(String),
}

impl Resolution {
    /// The release this channel names, if it names one.
    pub(crate) fn release(&self) -> Option<&Release> {
        match self {
            Resolution::Release(release) => Some(release),
            Resolution::Waiting(_) => None,
        }
    }

    /// Why the channel names nothing, if it names nothing.
    pub(crate) fn waiting_reason(&self) -> Option<&str> {
        match self {
            Resolution::Release(_) => None,
            Resolution::Waiting(reason) => Some(reason),
        }
    }
}

/// Order releases newest first: by semantic version, then by publish time so a
/// retag of the same version does not resolve to whichever one the API listed
/// first.
fn newest_first(releases: &[Release]) -> Vec<&Release> {
    let mut ordered: Vec<&Release> = releases.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .version
            .cmp(&left.version)
            .then(right.published_at.cmp(&left.published_at))
    });
    ordered
}

/// Apply a channel's rule to a release list.
///
/// `now` and `soak` are passed in rather than read from the clock so the rules
/// are testable against a fixed list.
pub(crate) fn resolve(
    channel: Channel,
    releases: &[Release],
    now: i64,
    soak: Duration,
) -> Resolution {
    let ordered = newest_first(releases);
    let Some(newest) = ordered.first().copied() else {
        return Resolution::Waiting("the release list is empty".to_string());
    };

    match channel {
        Channel::Canary => Resolution::Release(newest.clone()),
        Channel::Stable => ordered
            .iter()
            .find(|release| !release.is_prerelease())
            .map(|release| Resolution::Release((*release).clone()))
            .unwrap_or_else(|| {
                Resolution::Waiting(format!(
                    "every published release is a prerelease; the newest is {}",
                    newest.tag
                ))
            }),
        Channel::Nightly => {
            let soaked_at = newest.published_at.saturating_add(soak.as_secs() as i64);
            if now >= soaked_at {
                Resolution::Release(newest.clone())
            } else {
                Resolution::Waiting(format!(
                    "{} is the newest release and has {} of its {} soak window left; \
                     every older release has been superseded by it",
                    newest.tag,
                    human_hours(soaked_at - now),
                    human_hours(soak.as_secs() as i64),
                ))
            }
        }
    }
}

/// Render a span of seconds the way an operator reads a wait: whole hours once
/// there is at least one, minutes below that.
fn human_hours(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds >= 3600 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}m", (seconds + 59) / 60)
    }
}

/// What this machine says about the Lisa it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MachineConfig {
    /// The channel this machine chose, or `None` when nothing has chosen. The
    /// difference matters: unset is the state every machine is in today.
    pub(crate) channel: Option<Channel>,
    /// The soak window `nightly` waits out.
    pub(crate) soak_hours: u64,
    /// A project on this machine the unattended nightly run checks itself
    /// against after it moves — the box's own workload, so a release that
    /// breaks the work is caught by the work.
    pub(crate) nightly_project: Option<PathBuf>,
    /// What to run when an unattended run fails; the health record arrives on
    /// its stdin. This is where a machine's alarm reaches a person. Unset means
    /// a failing run still shouts locally — stderr, the system log, a desktop
    /// notification — but nothing leaves the box.
    pub(crate) alert_command: Option<String>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            channel: None,
            soak_hours: DEFAULT_SOAK_HOURS,
            nightly_project: None,
            alert_command: None,
        }
    }
}

impl MachineConfig {
    /// The channel to act on: the chosen one, or [`Channel::DEFAULT`].
    pub(crate) fn effective_channel(&self) -> Channel {
        self.channel.unwrap_or(Channel::DEFAULT)
    }

    /// The soak window as a duration.
    pub(crate) fn soak(&self) -> Duration {
        Duration::from_secs(self.soak_hours.saturating_mul(3600))
    }
}

/// The on-disk shape, kept separate so an unreadable value is a named error
/// rather than a silent default.
#[derive(Debug, Deserialize)]
struct MachineConfigFile {
    channel: Option<String>,
    soak_hours: Option<u64>,
    nightly_project: Option<PathBuf>,
    alert_command: Option<String>,
}

/// The directory the machine config lives in: `$LISA_CONFIG_DIR` when set,
/// otherwise the per-user config directory `directories` picks for this OS.
pub(crate) fn config_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os(CONFIG_DIR_ENV) {
        return Ok(PathBuf::from(dir));
    }
    ProjectDirs::from("io", "johnhkchen", "lisa")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or_else(|| {
            format!(
                "cannot find a per-user config directory on this machine; \
                 set {CONFIG_DIR_ENV} to the directory Lisa should keep its channel in"
            )
        })
}

/// The machine config file itself.
pub(crate) fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

/// Read the machine config from an exact path. A missing file is not an error:
/// it is a machine that has not chosen, which is a state with its own meaning.
pub(crate) fn load_from(path: &Path) -> Result<MachineConfig, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MachineConfig::default())
        }
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };

    let parsed: MachineConfigFile =
        toml::from_str(&raw).map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let channel = match parsed.channel {
        Some(raw) => {
            Some(Channel::parse(&raw).map_err(|error| format!("{}: {error}", path.display()))?)
        }
        None => None,
    };

    let soak_hours = match parsed.soak_hours {
        Some(0) => {
            return Err(format!(
                "{}: soak_hours = 0 would let nightly take a release the moment it is \
                 tagged; use canary for that",
                path.display()
            ))
        }
        Some(hours) => hours,
        None => DEFAULT_SOAK_HOURS,
    };

    Ok(MachineConfig {
        channel,
        soak_hours,
        nightly_project: parsed.nightly_project,
        alert_command: parsed.alert_command,
    })
}

/// Quote a value as a TOML basic string, so a path with a space or a command
/// with a quote in it survives the round trip.
fn toml_string(raw: &str) -> String {
    let mut quoted = String::with_capacity(raw.len() + 2);
    quoted.push('"');
    for ch in raw.chars() {
        match ch {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            '\r' => quoted.push_str("\\r"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// The optional lines: written when this machine has set them, absent when it
/// has not, so the file never claims a setting nobody made.
fn render_optional(config: &MachineConfig) -> String {
    let mut lines = String::new();

    lines.push_str(
        "\n# The project an unattended nightly run checks itself against after it moves.\n\
         # Set it to a board this machine actually works, so a release that breaks the\n\
         # work is caught by the work: lisa nightly install --project <path>\n",
    );
    match &config.nightly_project {
        Some(path) => lines.push_str(&format!(
            "nightly_project = {}\n",
            toml_string(&path.to_string_lossy())
        )),
        None => lines.push_str("# nightly_project = \"/path/to/a/board\"\n"),
    }

    lines.push_str(
        "\n# What to run when an unattended run fails. The health record arrives on its\n\
         # stdin. Point it at something you actually read; without it a failure only\n\
         # shouts on this box.\n",
    );
    match &config.alert_command {
        Some(command) => lines.push_str(&format!("alert_command = {}\n", toml_string(command))),
        None => lines.push_str(
            "# alert_command = \"curl -sS -X POST -d @- https://example.invalid/hook\"\n",
        ),
    }

    lines
}

/// Render the machine config as the file an operator will open and edit.
fn render(config: &MachineConfig) -> String {
    let channel = config.effective_channel();
    format!(
        "# Lisa machine config: which Lisa this box wants.\n\
         #\n\
         # This is machine state. A project's .lisa.toml `version` records the Lisa\n\
         # that set that project up and is a different fact entirely.\n\
         #\n\
         # channel = \"canary\"   {}\n\
         # channel = \"nightly\"  {}\n\
         # channel = \"stable\"   {}\n\
         channel = \"{channel}\"\n\
         \n\
         # Hours a release must age before the nightly channel will take it.\n\
         soak_hours = {}\n{}",
        Channel::Canary.rule(),
        Channel::Nightly.rule(),
        Channel::Stable.rule(),
        config.soak_hours,
        render_optional(config),
    )
}

/// Write the machine config to an exact path, creating its directory.
pub(crate) fn save_to(path: &Path, config: &MachineConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, render(config))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Record this machine's channel, keeping whatever soak window it already had.
pub(crate) fn set_channel_at(path: &Path, channel: Channel) -> Result<MachineConfig, String> {
    let mut config = load_from(path)?;
    config.channel = Some(channel);
    save_to(path, &config)?;
    Ok(config)
}

/// The current time in seconds since the Unix epoch.
pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// Parse the RFC 3339 UTC timestamps the GitHub releases API publishes
/// (`2026-08-14T09:12:33Z`) into seconds since the Unix epoch.
///
/// Only the shape GitHub emits is accepted. A fractional-second or offset form
/// would need a real date library, and taking one on for one field of one API
/// is not worth the dependency.
pub(crate) fn parse_rfc3339_utc(raw: &str) -> Option<i64> {
    let raw = raw.strip_suffix('Z')?;
    let (date, time) = raw.split_once('T')?;

    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut time_parts = time.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;
    if time_parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

/// Render epoch seconds back as the same UTC grammar
/// [`parse_rfc3339_utc`] reads, so a record a person opens carries a time they
/// can read rather than a number they have to convert.
pub(crate) fn format_rfc3339_utc(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let seconds = epoch.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60,
    )
}

/// The civil date a count of days since 1970-01-01 lands on, by Howard
/// Hinnant's `civil_from_days` — the exact inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Days between 1970-01-01 and the given civil date, by Howard Hinnant's
/// `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;

    /// The list that started the story, measured 2026-08-14: v0.4.4 is the
    /// newest stable and 26 days old, v0.5.0-rc.2 is the newest tag and 5 days
    /// old, and nobody chose either.
    fn measured_fleet_releases() -> Vec<Release> {
        vec![
            Release::from_tag("v0.4.3", stamp(-40 * DAY)).unwrap(),
            Release::from_tag("v0.4.4", stamp(-26 * DAY)).unwrap(),
            Release::from_tag("v0.5.0-rc.1", stamp(-9 * DAY)).unwrap(),
            Release::from_tag("v0.5.0-rc.2", stamp(-5 * DAY)).unwrap(),
        ]
    }

    /// A fixed "now" so the fixtures read as ages rather than dates.
    const NOW: i64 = 1_786_000_000;

    fn stamp(offset: i64) -> i64 {
        NOW + offset
    }

    fn soak() -> Duration {
        Duration::from_secs(DEFAULT_SOAK_HOURS * HOUR as u64)
    }

    fn resolved_tag(channel: Channel, releases: &[Release], now: i64) -> String {
        match resolve(channel, releases, now, soak()) {
            Resolution::Release(release) => release.tag,
            Resolution::Waiting(reason) => panic!("{channel} resolved to nothing: {reason}"),
        }
    }

    #[test]
    fn stable_takes_the_newest_non_prerelease() {
        assert_eq!(
            resolved_tag(Channel::Stable, &measured_fleet_releases(), NOW),
            "v0.4.4"
        );
    }

    #[test]
    fn canary_takes_the_newest_tag_prerelease_or_not() {
        assert_eq!(
            resolved_tag(Channel::Canary, &measured_fleet_releases(), NOW),
            "v0.5.0-rc.2"
        );
    }

    #[test]
    fn nightly_takes_the_newest_tag_once_it_has_soaked() {
        assert_eq!(
            resolved_tag(Channel::Nightly, &measured_fleet_releases(), NOW),
            "v0.5.0-rc.2"
        );
    }

    #[test]
    fn a_stable_release_outranks_its_own_release_candidates() {
        let mut releases = measured_fleet_releases();
        releases.push(Release::from_tag("v0.5.0", stamp(-DAY)).unwrap());
        assert_eq!(resolved_tag(Channel::Stable, &releases, NOW), "v0.5.0");
        assert_eq!(resolved_tag(Channel::Canary, &releases, NOW), "v0.5.0");
    }

    #[test]
    fn nightly_waits_out_the_soak_window_on_a_fresh_tag() {
        let mut releases = measured_fleet_releases();
        releases.push(Release::from_tag("v0.5.0-rc.3", stamp(-3 * HOUR)).unwrap());

        let resolution = resolve(Channel::Nightly, &releases, NOW, soak());
        let reason = resolution
            .waiting_reason()
            .expect("a three-hour-old tag has not soaked for twenty-four hours");
        assert!(reason.contains("v0.5.0-rc.3"), "reason: {reason}");
        assert!(reason.contains("21h"), "reason: {reason}");

        // Canary is the channel that does take it immediately.
        assert_eq!(resolved_tag(Channel::Canary, &releases, NOW), "v0.5.0-rc.3");
    }

    #[test]
    fn nightly_never_falls_back_to_the_tag_a_hotfix_superseded() {
        // rc.2 has soaked. rc.3 replaced it twenty minutes later and has not.
        let releases = vec![
            Release::from_tag("v0.4.4", stamp(-26 * DAY)).unwrap(),
            Release::from_tag("v0.5.0-rc.2", stamp(-5 * DAY)).unwrap(),
            Release::from_tag("v0.5.0-rc.3", stamp(-5 * DAY + 20 * 60)).unwrap(),
        ];

        // The edge that matters: rc.2 has just cleared the window and rc.3,
        // published twenty minutes after it, has not. A rule that fell back to
        // "the newest tag that has soaked" would install the release the hotfix
        // was published to replace.
        let hotfixed = vec![
            Release::from_tag("v0.4.4", stamp(-26 * DAY)).unwrap(),
            Release::from_tag("v0.5.0-rc.2", stamp(-24 * HOUR - 10 * 60)).unwrap(),
            Release::from_tag("v0.5.0-rc.3", stamp(-24 * HOUR + 10 * 60)).unwrap(),
        ];

        assert_eq!(
            resolved_tag(Channel::Nightly, &releases, NOW),
            "v0.5.0-rc.3"
        );

        let resolution = resolve(Channel::Nightly, &hotfixed, NOW, soak());
        let reason = resolution
            .waiting_reason()
            .expect("rc.3 has not soaked, and rc.2 is superseded by it");
        assert!(reason.contains("superseded"), "reason: {reason}");
    }

    #[test]
    fn the_soak_boundary_is_the_window_exactly() {
        let releases = vec![Release::from_tag("v0.5.0-rc.3", stamp(-24 * HOUR)).unwrap()];
        assert_eq!(
            resolved_tag(Channel::Nightly, &releases, NOW),
            "v0.5.0-rc.3"
        );

        let a_second_early = vec![Release::from_tag("v0.5.0-rc.3", stamp(-24 * HOUR + 1)).unwrap()];
        assert!(resolve(Channel::Nightly, &a_second_early, NOW, soak())
            .release()
            .is_none());
    }

    #[test]
    fn a_configured_soak_window_moves_the_boundary() {
        let releases = vec![Release::from_tag("v0.5.0-rc.3", stamp(-2 * HOUR)).unwrap()];
        let one_hour = Duration::from_secs(HOUR as u64);
        assert_eq!(
            resolve(Channel::Nightly, &releases, NOW, one_hour)
                .release()
                .map(|release| release.tag.clone()),
            Some("v0.5.0-rc.3".to_string())
        );
    }

    #[test]
    fn stable_says_so_when_every_release_is_a_prerelease() {
        let releases = vec![Release::from_tag("v0.1.0-rc.1", stamp(-9 * DAY)).unwrap()];
        let resolution = resolve(Channel::Stable, &releases, NOW, soak());
        assert!(resolution
            .waiting_reason()
            .unwrap()
            .contains("every published release is a prerelease"));
    }

    #[test]
    fn an_empty_release_list_resolves_to_nothing_on_every_channel() {
        for channel in Channel::ALL {
            assert!(resolve(channel, &[], NOW, soak()).release().is_none());
        }
    }

    #[test]
    fn tags_that_are_not_lisa_versions_are_not_releases() {
        assert!(Release::from_tag("nightly", NOW).is_none());
        assert!(Release::from_tag("v0.5", NOW).is_none());
        assert!(Release::from_tag("0.5.0", NOW).is_none());
        assert!(Release::from_tag("v0.5.0", NOW).is_some());
    }

    #[test]
    fn a_machine_that_has_not_chosen_reads_as_unset_and_acts_as_stable() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");

        let config = load_from(&path).unwrap();
        assert_eq!(config.channel, None);
        assert_eq!(config.effective_channel(), Channel::Stable);
        assert_eq!(config.soak_hours, DEFAULT_SOAK_HOURS);
    }

    #[test]
    fn setting_a_channel_round_trips_and_keeps_the_soak_window() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nested").join("config.toml");

        save_to(
            &path,
            &MachineConfig {
                channel: Some(Channel::Stable),
                soak_hours: 6,
                ..MachineConfig::default()
            },
        )
        .unwrap();
        set_channel_at(&path, Channel::Nightly).unwrap();

        let config = load_from(&path).unwrap();
        assert_eq!(config.channel, Some(Channel::Nightly));
        assert_eq!(config.soak_hours, 6);
        assert_eq!(config.soak(), Duration::from_secs(6 * 3600));

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("channel = \"nightly\""), "{raw}");
        assert!(raw.contains("soak_hours = 6"), "{raw}");
    }

    #[test]
    fn what_the_nightly_arrangement_records_survives_the_next_channel_change() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");

        save_to(
            &path,
            &MachineConfig {
                channel: Some(Channel::Nightly),
                soak_hours: DEFAULT_SOAK_HOURS,
                nightly_project: Some(PathBuf::from("/Users/someone/a board")),
                alert_command: Some("curl -d @- \"https://example.invalid/hook\"".to_string()),
            },
        )
        .unwrap();

        // Rewriting the file for a channel change must not drop the two
        // settings the unattended run depends on.
        set_channel_at(&path, Channel::Canary).unwrap();

        let config = load_from(&path).unwrap();
        assert_eq!(config.channel, Some(Channel::Canary));
        assert_eq!(
            config.nightly_project,
            Some(PathBuf::from("/Users/someone/a board"))
        );
        assert_eq!(
            config.alert_command.as_deref(),
            Some("curl -d @- \"https://example.invalid/hook\"")
        );
    }

    #[test]
    fn a_machine_with_no_nightly_settings_leaves_them_commented_out() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        save_to(&path, &MachineConfig::default()).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("# nightly_project ="), "{raw}");
        assert!(raw.contains("# alert_command ="), "{raw}");

        let config = load_from(&path).unwrap();
        assert_eq!(config.nightly_project, None);
        assert_eq!(config.alert_command, None);
    }

    #[test]
    fn a_time_written_down_reads_back_as_the_same_instant() {
        for stamp in [
            "2026-08-14T09:12:33Z",
            "1970-01-01T00:00:00Z",
            "2000-02-29T23:59:59Z",
        ] {
            let epoch = parse_rfc3339_utc(stamp).expect(stamp);
            assert_eq!(format_rfc3339_utc(epoch), stamp);
        }
    }

    #[test]
    fn an_unreadable_channel_is_a_named_error_not_a_silent_default() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "channel = \"experimental\"\n").unwrap();

        let error = load_from(&path).unwrap_err();
        assert!(error.contains("experimental"), "{error}");
        assert!(error.contains("canary, nightly and stable"), "{error}");
    }

    #[test]
    fn a_zero_soak_window_is_refused_and_points_at_canary() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "channel = \"nightly\"\nsoak_hours = 0\n").unwrap();

        let error = load_from(&path).unwrap_err();
        assert!(error.contains("canary"), "{error}");
    }

    #[test]
    fn github_timestamps_parse_to_epoch_seconds() {
        assert_eq!(parse_rfc3339_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_rfc3339_utc("2026-08-14T00:00:00Z"),
            Some(1_786_665_600)
        );
        assert_eq!(
            parse_rfc3339_utc("2026-08-14T09:12:33Z"),
            Some(1_786_665_600 + 9 * HOUR + 12 * 60 + 33)
        );
        // A leap day, to catch an off-by-one in the civil-date arithmetic.
        assert_eq!(
            parse_rfc3339_utc("2024-02-29T00:00:00Z"),
            Some(1_709_164_800)
        );

        for malformed in [
            "2026-08-14",
            "2026-08-14T09:12:33+02:00",
            "2026-13-01T00:00:00Z",
            "not a time",
        ] {
            assert_eq!(parse_rfc3339_utc(malformed), None, "{malformed}");
        }
    }
}
