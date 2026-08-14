//! Which release `nightly` carries, decided once where the releases are.
//!
//! `channel.rs` states the nightly rule for a machine that resolves its own
//! channel: the newest release is the only candidate, and it is nightly's once
//! it has aged past [`channel::DEFAULT_SOAK_HOURS`]. A package-managed box
//! cannot run that rule — `brew upgrade` and `apt-get upgrade` install what the
//! formula and the suite name, and neither has a clock. So the publisher runs
//! the rule instead, once, and writes the answer down where both package
//! managers read it.
//!
//! ## The pointer
//!
//! [`POINTER_PATH`] holds one line: a release tag, or the literal
//! [`NOTHING_PROMOTED`] meaning nothing has been promoted yet. The tap's
//! `lisa-nightly` formula and the apt `nightly` suite are both built from it
//! (`T-069-01-01`, `T-069-01-02`), so the two package managers cannot disagree
//! about which release has soaked, and a release publish cannot undo a
//! promotion — it rebuilds `nightly` from the pointer rather than from a rule
//! of its own.
//!
//! ## One soak window, one superseded rule
//!
//! There is no second window here. This module calls
//! [`channel::resolve`] with [`Channel::Nightly`] — the same function a
//! curl-installed box calls — so "has it soaked" and "has it been superseded"
//! are answered by one piece of code and one number. Superseded is the rule
//! `channel.rs` states: **any release that is not the newest one has been
//! superseded**, whether or not the release above it has soaked. Two releases
//! inside one window therefore promote nothing; the older one is superseded the
//! moment the newer one is tagged, and the newer one waits out its own window.
//!
//! ## What the job reads, and when
//!
//! The current release list, at promotion time — not a snapshot taken when the
//! tag was cut. A release that was yanked, deleted, turned back into a draft, or
//! never finished uploading its artifacts is not in the list this reads, so it
//! can never be promoted; and a pointer that names one is retired back to
//! [`NOTHING_PROMOTED`] rather than left for the publish to fail closed on.
//!
//! ## Nothing to do changes nothing
//!
//! A decision that lands on the tag the pointer already names writes no file,
//! which leaves no commit, which leaves the tap and the archive alone. A history
//! full of no-op commits is one nobody reads during an incident.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::channel::{self, Channel, Release, Resolution};

/// The promotion pointer, relative to the repository root.
pub(crate) const POINTER_PATH: &str = "packaging/apt/nightly-tag.txt";

/// What the pointer holds when nothing has been promoted: `nightly` then
/// carries exactly what `stable` carries, which is the safe place to be.
pub(crate) const NOTHING_PROMOTED: &str = "stable";

/// What a release must carry before any channel may point at it: the four
/// Debian packages the apt suites pool, and the formula the tap renders. The
/// apt publish checks the same four `.deb` names before it builds a suite; a
/// release still uploading, or one whose build half failed, is not promotable
/// and not poolable, and both jobs say so rather than publishing a channel that
/// resolves to nothing.
const REQUIRED_ASSETS: [&str; 5] = [
    "lisa-amd64.deb",
    "lisa-arm64.deb",
    "lisa-runtime-zellij-amd64.deb",
    "lisa-runtime-zellij-arm64.deb",
    "lisa.rb",
];

/// One published release, with enough of its assets to say whether a channel
/// could point at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Published {
    /// The release itself, as channel resolution sees it.
    pub(crate) release: Release,
    /// Names of the assets that finished uploading.
    pub(crate) assets: Vec<String>,
}

impl Published {
    /// Build one from a tag and its asset names, or `None` when the tag is not
    /// one of Lisa's. The job builds these from the API response; this is how a
    /// test states a world in one line.
    #[cfg(test)]
    fn new(tag: &str, published_at: i64, assets: &[&str]) -> Option<Self> {
        Some(Self {
            release: Release::from_tag(tag, published_at)?,
            assets: assets.iter().map(|name| name.to_string()).collect(),
        })
    }

    /// The required assets this release does not have.
    fn missing(&self) -> Vec<&'static str> {
        REQUIRED_ASSETS
            .into_iter()
            .filter(|required| !self.assets.iter().any(|name| name == required))
            .collect()
    }

    /// Whether every channel artifact this release needs is really published.
    fn is_complete(&self) -> bool {
        self.missing().is_empty()
    }
}

/// One entry of the GitHub releases API, reduced to what promotion reads.
#[derive(Debug, Deserialize)]
struct Entry {
    tag_name: String,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<EntryAsset>,
}

/// One release asset. `state` is `uploaded` once the bytes are really there;
/// an asset still being written is not something a channel may point at.
#[derive(Debug, Deserialize)]
struct EntryAsset {
    name: String,
    #[serde(default)]
    state: Option<String>,
}

/// Read a releases-API response, dropping drafts, tags that are not Lisa
/// releases, and assets that have not finished uploading.
///
/// A body holding several concatenated arrays is read too, so the output of
/// `gh api --paginate` is as acceptable as one page.
pub(crate) fn parse_releases(body: &str) -> Result<Vec<Published>, String> {
    let mut published = Vec::new();
    for page in serde_json::Deserializer::from_str(body).into_iter::<Vec<Entry>>() {
        let page = page.map_err(|error| format!("the release list was not readable: {error}"))?;
        for entry in page {
            if entry.draft {
                continue;
            }
            let Some(published_at) = entry
                .published_at
                .as_deref()
                .and_then(channel::parse_rfc3339_utc)
            else {
                continue;
            };
            let Some(release) = Release::from_tag(&entry.tag_name, published_at) else {
                continue;
            };
            published.push(Published {
                release,
                assets: entry
                    .assets
                    .into_iter()
                    .filter(|asset| asset.state.as_deref().unwrap_or("uploaded") == "uploaded")
                    .map(|asset| asset.name)
                    .collect(),
            });
        }
    }
    Ok(published)
}

/// What a promotion run does to the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// The pointer moves up to a release that has soaked.
    Promote,
    /// The pointer named a release that is no longer published, and nothing has
    /// soaked to replace it, so nightly goes back to carrying what stable does.
    Retire,
    /// The pointer already says the right thing. Nothing is written.
    Unchanged,
}

impl Action {
    /// The word a workflow branches on.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Action::Promote => "promote",
            Action::Retire => "retire",
            Action::Unchanged => "unchanged",
        }
    }

    /// Whether this action changes the pointer.
    pub(crate) const fn changes_the_pointer(self) -> bool {
        !matches!(self, Action::Unchanged)
    }
}

/// What one promotion run decided, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    /// What this run does.
    pub(crate) action: Action,
    /// What the pointer said when the run started.
    pub(crate) pointer: String,
    /// What the pointer should say now.
    pub(crate) target: String,
    /// The newest release carrying a complete asset set — what `canary` is on,
    /// and the release a tap publish renders `lisa-canary` from.
    pub(crate) canary: Option<String>,
    /// The newest release of any kind, complete or not. Different from
    /// [`Decision::canary`] exactly while a release is still uploading.
    pub(crate) newest: Option<String>,
    /// The window this run judged against.
    pub(crate) soak_hours: u64,
    /// How long the promoted release had already been eligible when this run
    /// promoted it. Anything above one missed run means the schedule is not
    /// keeping up, which is the failure that otherwise reads as healthy.
    pub(crate) overdue_hours: i64,
    /// One line a person can read.
    pub(crate) reason: String,
}

/// How to name the pointer's current value in a sentence.
fn describe(pointer: &str) -> String {
    if pointer == NOTHING_PROMOTED {
        "the newest stable release".to_string()
    } else {
        pointer.to_string()
    }
}

/// Apply the nightly rule to the release list as it stands right now.
///
/// `now` and `soak` are passed in rather than read from the clock so a fixed
/// list decides the same way in a test as it does in the job.
pub(crate) fn decide(pointer: &str, releases: &[Published], now: i64, soak: Duration) -> Decision {
    let soak_hours = soak.as_secs() as i64 / 3600;
    let ordered = newest_first(releases);
    let newest = ordered.first().map(|entry| entry.release.tag.clone());
    let canary = ordered
        .iter()
        .find(|entry| entry.is_complete())
        .map(|entry| entry.release.tag.clone());

    let unchanged = |reason: String| Decision {
        action: Action::Unchanged,
        pointer: pointer.to_string(),
        target: pointer.to_string(),
        canary: canary.clone(),
        newest: newest.clone(),
        soak_hours: soak_hours.max(0) as u64,
        overdue_hours: 0,
        reason,
    };

    // Nothing to act on. A run that cannot see a single publishable release is
    // looking at a world it does not understand -- an outage, a bad token, an
    // empty page -- and the safe move is to leave the fleet where it is.
    if canary.is_none() {
        return unchanged(
            "no published release carries the complete asset set, so nightly stays where it is"
                .to_string(),
        );
    }

    // A pointer naming a release that is no longer published -- yanked,
    // deleted, turned back into a draft, or stripped of its artifacts -- is not
    // a pointer any more. Fall back to stable and let the ordinary rule pick it
    // up again from there.
    let held = pointer == NOTHING_PROMOTED
        || ordered
            .iter()
            .any(|entry| entry.is_complete() && entry.release.tag == pointer);
    let effective = if held { pointer } else { NOTHING_PROMOTED };

    // The rule itself, and it is channel.rs's: the newest release is the only
    // candidate, and it is nightly's once it has soaked. Everything below it
    // has been superseded, however long it has been sitting there.
    let candidate = match channel::resolve(Channel::Nightly, &all(releases), now, soak) {
        Resolution::Waiting(reason) => Err(reason),
        Resolution::Release(newest) => ordered
            .iter()
            .find(|entry| entry.release.tag == newest.tag)
            .copied()
            .ok_or_else(|| format!("{} is not in the release list", newest.tag))
            .and_then(|entry| {
                if entry.is_complete() {
                    Ok(entry)
                } else {
                    Err(format!(
                        "{} is the newest release and has soaked, but it is missing {}; \
                         nothing older can be promoted in its place because it supersedes them",
                        entry.release.tag,
                        entry.missing().join(", "),
                    ))
                }
            }),
    };

    let (target, overdue_hours) = match &candidate {
        Ok(entry) => (
            entry.release.tag.clone(),
            (now - entry.release.published_at - soak.as_secs() as i64).max(0) / 3600,
        ),
        Err(_) => (effective.to_string(), 0),
    };

    let action = if target == pointer {
        Action::Unchanged
    } else if target == NOTHING_PROMOTED {
        Action::Retire
    } else {
        Action::Promote
    };

    let reason = match (action, &candidate) {
        (Action::Promote, _) => format!(
            "{target} is the newest release and has cleared its {soak_hours}h soak window; \
             nightly moves from {} to it",
            describe(pointer),
        ),
        (Action::Retire, _) => format!(
            "{pointer} is no longer a published release with a complete asset set, and \
             nothing newer has soaked; nightly goes back to carrying the newest stable release"
        ),
        (Action::Unchanged, Ok(_)) => format!("nightly already carries {pointer}"),
        (Action::Unchanged, Err(waiting)) => waiting.clone(),
    };

    Decision {
        action,
        pointer: pointer.to_string(),
        target,
        canary,
        newest,
        soak_hours: soak_hours.max(0) as u64,
        overdue_hours,
        reason,
    }
}

/// The releases as plain channel releases, for [`channel::resolve`].
fn all(releases: &[Published]) -> Vec<Release> {
    releases.iter().map(|entry| entry.release.clone()).collect()
}

/// Newest first, by the same order channel resolution uses.
fn newest_first(releases: &[Published]) -> Vec<&Published> {
    let mut ordered: Vec<&Published> = releases.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .release
            .version
            .cmp(&left.release.version)
            .then(right.release.published_at.cmp(&left.release.published_at))
    });
    ordered
}

/// Read the pointer file, refusing anything that is not a tag or the literal
/// [`NOTHING_PROMOTED`]. A pointer nobody can parse is a nightly channel nobody
/// can build.
pub(crate) fn read_pointer(path: &Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let pointer = raw.trim();
    if pointer.is_empty() {
        return Err(format!(
            "{} is empty; it holds a release tag, or {NOTHING_PROMOTED} when nothing has been \
             promoted",
            path.display()
        ));
    }
    if pointer != NOTHING_PROMOTED && Release::from_tag(pointer, 0).is_none() {
        return Err(format!(
            "{} says {pointer:?}, which is neither a v<semver> release tag nor {NOTHING_PROMOTED}",
            path.display()
        ));
    }
    Ok(pointer.to_string())
}

/// Write the pointer, and say whether anything actually changed on disk.
/// Identical contents are left alone so a no-op promotion leaves no commit.
pub(crate) fn write_pointer(path: &Path, target: &str) -> Result<bool, String> {
    let next = format!("{target}\n");
    if std::fs::read_to_string(path).is_ok_and(|current| current == next) {
        return Ok(false);
    }
    std::fs::write(path, next)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

/// What `lisa promote-nightly` was asked to do.
#[derive(Debug, Clone)]
pub(crate) struct PromoteArgs {
    /// The releases-API response to judge, or `-` for standard input.
    ///
    /// Handed in rather than fetched: the caller is a workflow that already
    /// holds a token for this repository, and an authenticated `gh api` is
    /// both paginated and outside the anonymous rate limit that would
    /// otherwise decide whether a promotion happens.
    pub(crate) releases: PathBuf,
    /// The pointer file to read, and to write when `write` is set.
    pub(crate) pointer: PathBuf,
    /// Write the decision to the pointer file.
    pub(crate) write: bool,
    /// Emit the decision as JSON for a workflow to branch on.
    pub(crate) json: bool,
    /// Judge against this instant rather than the clock.
    pub(crate) now: Option<i64>,
}

/// Decide, and optionally write. The exit code is zero whenever the decision was
/// reached, including when the decision is to change nothing.
pub(crate) fn run_promote(args: PromoteArgs) -> Result<String, String> {
    let body = if args.releases.as_os_str() == "-" {
        std::io::read_to_string(std::io::stdin())
            .map_err(|error| format!("cannot read the release list on stdin: {error}"))?
    } else {
        std::fs::read_to_string(&args.releases)
            .map_err(|error| format!("cannot read {}: {error}", args.releases.display()))?
    };

    let releases = parse_releases(&body)?;
    let pointer = read_pointer(&args.pointer)?;
    let soak = Duration::from_secs(channel::DEFAULT_SOAK_HOURS * 3600);
    let now = args.now.unwrap_or_else(channel::now_unix);
    let decision = decide(&pointer, &releases, now, soak);

    let wrote = if args.write && decision.action.changes_the_pointer() {
        write_pointer(&args.pointer, &decision.target)?
    } else {
        false
    };

    Ok(if args.json {
        render_json(&decision, &args.pointer, wrote, now)
    } else {
        render_human(&decision, &args.pointer, wrote)
    })
}

/// The decision as JSON, which is what the promotion workflow branches on.
fn render_json(decision: &Decision, pointer_path: &Path, wrote: bool, now: i64) -> String {
    let quoted = |value: &Option<String>| match value {
        Some(value) => serde_json::Value::String(value.clone()),
        None => serde_json::Value::Null,
    };
    serde_json::json!({
        "action": decision.action.as_str(),
        "pointer": decision.pointer,
        "target": decision.target,
        "canary": quoted(&decision.canary),
        "newest": quoted(&decision.newest),
        "soak_hours": decision.soak_hours,
        "overdue_hours": decision.overdue_hours,
        "reason": decision.reason,
        "pointer_path": pointer_path.display().to_string(),
        "wrote": wrote,
        "decided_at": channel::format_rfc3339_utc(now),
    })
    .to_string()
}

/// The decision as a person reads it, in the job log and on a terminal.
fn render_human(decision: &Decision, pointer_path: &Path, wrote: bool) -> String {
    let mut lines = vec![
        format!("action:  {}", decision.action.as_str()),
        format!("nightly: {}", describe(&decision.target)),
        format!("was:     {}", describe(&decision.pointer)),
        format!(
            "canary:  {}",
            decision.canary.as_deref().unwrap_or("nothing publishable")
        ),
        format!("why:     {}", decision.reason),
    ];
    if decision.overdue_hours > 0 {
        lines.push(format!(
            "late:    {} became promotable {}h ago",
            decision.target, decision.overdue_hours
        ));
    }
    lines.push(if wrote {
        format!("wrote:   {}", pointer_path.display())
    } else if decision.action.changes_the_pointer() {
        format!(
            "wrote:   nothing; --write is what moves {}",
            pointer_path.display()
        )
    } else {
        format!(
            "wrote:   nothing ({} already says this)",
            pointer_path.display()
        )
    });
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * HOUR;
    const NOW: i64 = 1_786_000_000;

    fn soak() -> Duration {
        Duration::from_secs(channel::DEFAULT_SOAK_HOURS * 3600)
    }

    fn stamp(offset: i64) -> i64 {
        NOW + offset
    }

    /// Every asset a channel needs, which is what a finished release has.
    fn complete(tag: &str, offset: i64) -> Published {
        Published::new(tag, stamp(offset), &REQUIRED_ASSETS).expect(tag)
    }

    /// The live list, measured 2026-08-14: v0.4.4 is the newest stable and
    /// v0.5.0-rc.2 is the newest release of any kind, five days old.
    fn measured() -> Vec<Published> {
        vec![
            complete("v0.4.3", -40 * DAY),
            complete("v0.4.4", -26 * DAY),
            complete("v0.5.0-rc.1", -9 * DAY),
            complete("v0.5.0-rc.2", -5 * DAY),
        ]
    }

    #[test]
    fn a_soaked_release_is_promoted_off_stable() {
        let decision = decide(NOTHING_PROMOTED, &measured(), NOW, soak());
        assert_eq!(decision.action, Action::Promote);
        assert_eq!(decision.target, "v0.5.0-rc.2");
        assert_eq!(decision.canary.as_deref(), Some("v0.5.0-rc.2"));
        assert!(
            decision.reason.contains("soak window"),
            "{}",
            decision.reason
        );
    }

    #[test]
    fn a_promotion_that_has_nothing_to_do_writes_nothing() {
        let decision = decide("v0.5.0-rc.2", &measured(), NOW, soak());
        assert_eq!(decision.action, Action::Unchanged);
        assert!(!decision.action.changes_the_pointer());
        assert_eq!(decision.target, "v0.5.0-rc.2");
        assert_eq!(decision.reason, "nightly already carries v0.5.0-rc.2");
    }

    #[test]
    fn a_release_inside_its_window_is_not_promoted_yet() {
        let mut releases = measured();
        releases.push(complete("v0.5.0-rc.3", -3 * HOUR));

        let decision = decide(NOTHING_PROMOTED, &releases, NOW, soak());
        assert_eq!(decision.action, Action::Unchanged);
        assert_eq!(decision.target, NOTHING_PROMOTED);
        assert!(decision.reason.contains("21h"), "{}", decision.reason);
    }

    #[test]
    fn two_releases_inside_one_window_promote_neither() {
        // rc.2 cleared the window ten minutes ago. rc.3 replaced it twenty
        // minutes after it was published and has not cleared anything. A rule
        // that promoted "the newest release that has soaked" would put the
        // fleet on the release the hotfix exists to replace.
        let releases = vec![
            complete("v0.4.4", -26 * DAY),
            complete("v0.5.0-rc.2", -24 * HOUR - 10 * 60),
            complete("v0.5.0-rc.3", -24 * HOUR + 10 * 60),
        ];

        let decision = decide(NOTHING_PROMOTED, &releases, NOW, soak());
        assert_eq!(decision.action, Action::Unchanged);
        assert_eq!(decision.target, NOTHING_PROMOTED);
        assert!(
            decision.reason.contains("superseded"),
            "{}",
            decision.reason
        );

        // Ten minutes later rc.3 clears its own window, and it is what lands.
        let later = decide(NOTHING_PROMOTED, &releases, NOW + 11 * 60, soak());
        assert_eq!(later.action, Action::Promote);
        assert_eq!(later.target, "v0.5.0-rc.3");
    }

    #[test]
    fn a_superseded_release_is_never_promoted_even_after_it_soaks() {
        // rc.2 soaked long ago; rc.3 superseded it and is still inside its
        // window. Nightly waits rather than walking back down the list.
        let releases = vec![
            complete("v0.5.0-rc.2", -5 * DAY),
            complete("v0.5.0-rc.3", -2 * HOUR),
        ];
        let decision = decide(NOTHING_PROMOTED, &releases, NOW, soak());
        assert_eq!(decision.action, Action::Unchanged);
        assert!(
            decision.reason.contains("v0.5.0-rc.3"),
            "{}",
            decision.reason
        );
    }

    #[test]
    fn a_yanked_release_is_never_promoted() {
        // The newest release was pulled after it soaked: it is simply not in
        // the list this run reads, so the release below it is the candidate.
        let releases = vec![
            complete("v0.4.4", -26 * DAY),
            complete("v0.5.0-rc.1", -9 * DAY),
        ];
        let decision = decide(NOTHING_PROMOTED, &releases, NOW, soak());
        assert_eq!(decision.target, "v0.5.0-rc.1");

        // And a release that never finished uploading is not promotable at all.
        let half_uploaded = vec![
            complete("v0.4.4", -26 * DAY),
            Published::new("v0.5.0-rc.2", stamp(-5 * DAY), &["lisa-amd64.deb"]).unwrap(),
        ];
        let decision = decide(NOTHING_PROMOTED, &half_uploaded, NOW, soak());
        assert_eq!(decision.action, Action::Unchanged);
        assert!(
            decision.reason.contains("lisa-arm64.deb"),
            "{}",
            decision.reason
        );
    }

    #[test]
    fn a_pointer_whose_release_was_pulled_is_retired_rather_than_left_dangling() {
        // nightly was on rc.2 and rc.2 has been deleted. Nothing newer has
        // soaked, so nightly goes back to stable rather than naming a release
        // the publish would fail closed on.
        let releases = vec![
            complete("v0.4.4", -26 * DAY),
            complete("v0.5.0-rc.3", -2 * HOUR),
        ];
        let decision = decide("v0.5.0-rc.2", &releases, NOW, soak());
        assert_eq!(decision.action, Action::Retire);
        assert_eq!(decision.target, NOTHING_PROMOTED);
        assert!(
            decision.reason.contains("no longer a published release"),
            "{}",
            decision.reason
        );
    }

    #[test]
    fn a_pointer_whose_release_was_pulled_moves_straight_up_when_something_has_soaked() {
        let releases = vec![
            complete("v0.4.4", -26 * DAY),
            complete("v0.5.0-rc.3", -2 * DAY),
        ];
        let decision = decide("v0.5.0-rc.2", &releases, NOW, soak());
        assert_eq!(decision.action, Action::Promote);
        assert_eq!(decision.target, "v0.5.0-rc.3");
    }

    #[test]
    fn a_release_list_that_says_nothing_moves_nothing() {
        for releases in [
            Vec::new(),
            vec![Published::new("v0.5.0", NOW, &[]).unwrap()],
        ] {
            let decision = decide("v0.4.4", &releases, NOW, soak());
            assert_eq!(decision.action, Action::Unchanged);
            assert_eq!(decision.target, "v0.4.4");
            assert!(
                decision.reason.contains("complete asset set"),
                "{}",
                decision.reason
            );
        }
    }

    #[test]
    fn a_late_promotion_says_how_late_it_was() {
        let releases = vec![complete("v0.5.0-rc.2", -5 * DAY)];
        let decision = decide(NOTHING_PROMOTED, &releases, NOW, soak());
        assert_eq!(decision.action, Action::Promote);
        assert_eq!(decision.overdue_hours, 4 * 24);
    }

    #[test]
    fn the_window_judged_is_the_one_channel_rs_states() {
        // Not a second number: the same constant a curl-installed box uses.
        let decision = decide(NOTHING_PROMOTED, &measured(), NOW, soak());
        assert_eq!(decision.soak_hours, channel::DEFAULT_SOAK_HOURS);
    }

    #[test]
    fn drafts_and_unfinished_assets_are_not_part_of_the_world() {
        let body = r#"[
          {"tag_name": "v0.6.0", "published_at": "2026-08-13T00:00:00Z", "draft": true,
           "assets": [{"name": "lisa.rb", "state": "uploaded"}]},
          {"tag_name": "v0.5.0-rc.2", "published_at": "2026-08-09T00:00:00Z", "draft": false,
           "assets": [{"name": "lisa.rb", "state": "uploaded"},
                      {"name": "lisa-amd64.deb", "state": "starter"}]},
          {"tag_name": "nightly", "published_at": "2026-08-09T00:00:00Z", "draft": false,
           "assets": []}
        ]"#;

        let releases = parse_releases(body).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].release.tag, "v0.5.0-rc.2");
        assert_eq!(releases[0].assets, vec!["lisa.rb".to_string()]);
    }

    #[test]
    fn several_pages_of_release_list_read_as_one_list() {
        let body = r#"[{"tag_name": "v0.5.0-rc.2", "published_at": "2026-08-09T00:00:00Z"}]
                      [{"tag_name": "v0.4.4", "published_at": "2026-07-19T00:00:00Z"}]"#;
        let releases = parse_releases(body).unwrap();
        assert_eq!(releases.len(), 2);
    }

    #[test]
    fn an_unreadable_release_list_is_an_error_not_an_empty_world() {
        assert!(parse_releases("not json").is_err());
    }

    #[test]
    fn a_pointer_is_a_tag_or_the_word_stable() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nightly-tag.txt");

        std::fs::write(&path, "stable\n").unwrap();
        assert_eq!(read_pointer(&path).unwrap(), "stable");

        std::fs::write(&path, "  v0.5.0-rc.2\n\n").unwrap();
        assert_eq!(read_pointer(&path).unwrap(), "v0.5.0-rc.2");

        std::fs::write(&path, "\n").unwrap();
        assert!(read_pointer(&path).unwrap_err().contains("empty"));

        std::fs::write(&path, "latest\n").unwrap();
        assert!(read_pointer(&path).unwrap_err().contains("neither"));
    }

    #[test]
    fn writing_the_same_pointer_twice_touches_the_file_once() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nightly-tag.txt");
        std::fs::write(&path, "stable\n").unwrap();

        assert!(write_pointer(&path, "v0.5.0-rc.2").unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v0.5.0-rc.2\n");
        assert!(!write_pointer(&path, "v0.5.0-rc.2").unwrap());
    }
}
