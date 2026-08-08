//! One account of how a project differs from what this binary would create.
//!
//! `lisa doctor` renders it, `lisa init` acts on the part it can prove is safe,
//! and `lisa clean` acts where a person has said to. All three read [`inventory`];
//! none of them re-derives staleness. That is the whole point of the module: if
//! each command computed its own answer, the three would disagree the first time
//! any of them changed, and an operator would be told to run a command that then
//! reports nothing to do.
//!
//! Nothing here prints. The function returns data; the caller decides how much
//! of it a human should see.
//!
//! ## The three categories
//!
//! - **Behind** — a Lisa-owned thing that exists, is byte-recognisable as a Lisa
//!   generation, and has a newer form. This is not judged here: it is whatever
//!   [`crate::init::plan_init_actions`] says it would update, so the diagnosis
//!   and the fix cannot drift apart.
//! - **Retired** — a Lisa-owned thing that exists and has no newer form, because
//!   Lisa stopped producing it: the old workflow document, `scheduling.auto_advance`,
//!   and a `CLAUDE.md`/`AGENTS.md` whose bytes match a generator Lisa no longer
//!   ships (see [`crate::legacy_context`]).
//! - **Stale content in files Lisa does not own** — a ticket whose `phase:` names
//!   a phase retired in 0.5.0. Lisa owns the vocabulary; the board is the
//!   operator's. Reported, never rewritten.
//!
//! ## How a finding gets its remedy
//!
//! Never by hard-coding a command next to a category. A finding's remedy is read
//! back off init's own plan: if init would resolve it, the remedy is `lisa init`,
//! and otherwise removal is the only thing that will, so the remedy is
//! `lisa clean`. Teaching init a new retirement (T-057-02-02) therefore moves the
//! remedy on its own, with no edit here and no window where doctor names a
//! command that does nothing.

use std::path::Path;

use lisa_core::types::RETIRED_PHASE_NAMES;

use crate::config;
use crate::init::{plan_init_actions, InitAction};
use crate::legacy_context::{is_generated_agents_md, is_generated_claude_md};

/// The document Lisa installed while the workflow was called RDSPI.
const RETIRED_WORKFLOW_DOCUMENT: &str = "docs/knowledge/rdspi-workflow.md";
/// Where 0.5.0 installs it instead.
const CURRENT_WORKFLOW_DOCUMENT: &str = "docs/knowledge/lisa-workflow.md";
/// The scheduling key 0.5.0 stopped reading.
const RETIRED_CONFIG_KEY: &str = "auto_advance";
/// Where tickets live when `.lisa.toml` does not say otherwise.
const DEFAULT_TICKET_DIR: &str = "docs/active/tickets";

/// Decides, from bytes alone, whether an agent-context file is one Lisa wrote.
type GenerationMatcher = fn(&str) -> bool;

/// Why a finding is in the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CurrencyKind {
    /// Lisa still owns this and ships a newer form of it.
    Behind,
    /// Lisa owned this and ships nothing in its place.
    Retired,
    /// A file Lisa does not own, carrying vocabulary Lisa retired.
    StaleContent,
}

impl CurrencyKind {
    /// A short operator-facing label, aligned for a column of findings.
    pub fn label(self) -> &'static str {
        match self {
            CurrencyKind::Behind => "behind",
            CurrencyKind::Retired => "retired",
            CurrencyKind::StaleContent => "ticket",
        }
    }
}

/// What resolves a finding.
///
/// Every finding carries one. A diagnosis whose remedy the reader has to infer
/// is worse than no diagnosis at all, so there is no "none" here — where no
/// command applies, [`Remedy::Operator`] states the edit in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remedy {
    /// `lisa init` carries this forward.
    Init,
    /// Nothing carries it forward; removal is the fix, under explicit consent.
    Clean,
    /// The file is the operator's. The string names the exact edit.
    Operator(String),
}

impl Remedy {
    /// The one line a reader acts on.
    pub fn line(&self) -> String {
        match self {
            Remedy::Init => "run `lisa init`".to_string(),
            Remedy::Clean => "run `lisa clean`".to_string(),
            Remedy::Operator(edit) => edit.clone(),
        }
    }
}

/// One way this project differs from what the running binary would create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyFinding {
    pub kind: CurrencyKind,
    /// What the finding is about — a repository-relative path, or a config key.
    pub subject: String,
    /// One sentence saying what is stale about it.
    pub detail: String,
    pub remedy: Remedy,
}

/// What `.lisa.toml` records about the Lisa that set this project up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedVersion {
    /// No `.lisa.toml`: not a Lisa project, so there is nothing to be current with.
    NoProject,
    /// `.lisa.toml` exists but could not be read or parsed.
    Unreadable,
    /// Initialised before Lisa recorded a version. Not an error — just old.
    PreVersioning,
    /// Recorded, and older than this binary.
    Behind { recorded: String },
    /// Recorded, and this binary's own version.
    Current { recorded: String },
}

/// A structured account of how this project differs from a fresh `lisa init`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCurrency {
    pub recorded_version: RecordedVersion,
    /// Every finding, ordered behind → retired → stale content, then by subject.
    pub findings: Vec<CurrencyFinding>,
    /// This binary's version, so a renderer never has to look it up.
    pub current_version: &'static str,
}

impl ProjectCurrency {
    /// True when nothing about this project is behind the running binary.
    pub fn is_current(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Take an account of how the project at `root` differs from what this binary
/// would create. Reads the filesystem; changes nothing; prints nothing.
pub fn inventory(root: &Path) -> ProjectCurrency {
    let current_version = config::LISA_VERSION;
    let config_path = root.join(".lisa.toml");

    if !config_path.exists() {
        return ProjectCurrency {
            recorded_version: RecordedVersion::NoProject,
            findings: Vec::new(),
            current_version,
        };
    }

    let config_text = std::fs::read_to_string(&config_path).ok();
    let config_table = config_text
        .as_deref()
        .and_then(|text| text.parse::<toml::Value>().ok());

    let recorded_version = match (&config_text, &config_table) {
        (None, _) | (_, None) => RecordedVersion::Unreadable,
        (Some(text), Some(_)) => match toml::from_str::<config::LisaConfig>(text)
            .ok()
            .and_then(|parsed| parsed.version)
        {
            None => RecordedVersion::PreVersioning,
            Some(recorded) if config::version_is_stale(&recorded, current_version) => {
                RecordedVersion::Behind { recorded }
            }
            Some(recorded) => RecordedVersion::Current { recorded },
        },
    };

    let plan = plan_init_actions(root);
    let mut findings = Vec::new();

    let version_finding = version_finding(&recorded_version, current_version);
    let config_already_reported = version_finding.is_some();
    findings.extend(version_finding);
    findings.extend(behind_findings(root, &plan, config_already_reported));
    findings.extend(retired_workflow_finding(root, &plan));
    findings.extend(retired_context_findings(root, &plan));
    findings.extend(retired_config_key_finding(&plan, config_table.as_ref()));
    findings.extend(stale_ticket_findings(root));

    findings.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.subject.cmp(&right.subject))
    });

    ProjectCurrency {
        recorded_version,
        findings,
        current_version,
    }
}

/// What the recorded version says, as a finding a reader can act on.
///
/// An absent `version` key is "initialised before Lisa recorded one", not a
/// fault: Lisa did not always write it, and a project that predates the key is
/// simply old.
fn version_finding(
    recorded_version: &RecordedVersion,
    current_version: &str,
) -> Option<CurrencyFinding> {
    let detail = match recorded_version {
        RecordedVersion::Behind { recorded } => {
            format!("Set up by Lisa {recorded}; this Lisa is {current_version}.")
        }
        RecordedVersion::PreVersioning => format!(
            "Set up before Lisa recorded a version, so it predates {current_version} by an unknown distance."
        ),
        RecordedVersion::Current { .. }
        | RecordedVersion::Unreadable
        | RecordedVersion::NoProject => return None,
    };

    Some(CurrencyFinding {
        kind: CurrencyKind::Behind,
        subject: ".lisa.toml".to_string(),
        detail,
        remedy: Remedy::Init,
    })
}

/// Everything init would refresh in place, taken straight from its plan.
///
/// `.lisa.toml` is handled apart from the rest. Its recorded version is already
/// a finding of its own, and one upgrade reported twice against the same file
/// reads as two problems — so the settings line is emitted only when the
/// version had nothing to say, which is the case a current version hides: keys
/// Lisa added since are upserted without a version bump.
fn behind_findings(
    root: &Path,
    plan: &[InitAction],
    config_already_reported: bool,
) -> Vec<CurrencyFinding> {
    let mut findings = Vec::new();
    for action in plan {
        let InitAction::UpdateFile { path, .. } = action else {
            continue;
        };
        let subject = relative(root, path);
        if subject == ".lisa.toml" {
            if !config_already_reported {
                findings.push(CurrencyFinding {
                    kind: CurrencyKind::Behind,
                    subject,
                    detail: "Settings Lisa added since this file was written are missing from it."
                        .to_string(),
                    remedy: Remedy::Init,
                });
            }
            continue;
        }
        findings.push(CurrencyFinding {
            kind: CurrencyKind::Behind,
            subject,
            detail: "An older Lisa generation is installed here; a newer one exists.".to_string(),
            remedy: Remedy::Init,
        });
    }
    findings
}

/// The workflow document under the name it carried through 0.4.
///
/// The path is one Lisa created, so its presence is always reported — unlike
/// `CLAUDE.md`, whose path belongs to the project. What the bytes decide is the
/// remedy: matching a shipped generation, init removes it; edited, only an
/// explicit removal will.
fn retired_workflow_finding(root: &Path, plan: &[InitAction]) -> Option<CurrencyFinding> {
    let path = root.join(RETIRED_WORKFLOW_DOCUMENT);
    if !path.exists() {
        return None;
    }

    let removed_by_init = plan_removes(plan, &path);
    Some(CurrencyFinding {
        kind: CurrencyKind::Retired,
        subject: RETIRED_WORKFLOW_DOCUMENT.to_string(),
        detail: if removed_by_init {
            format!("Describes a workflow Lisa no longer runs; {CURRENT_WORKFLOW_DOCUMENT} replaced it.")
        } else {
            format!(
                "Describes a workflow Lisa no longer runs, and has been edited since Lisa wrote it, so init leaves it alone. {CURRENT_WORKFLOW_DOCUMENT} replaced it."
            )
        },
        remedy: if removed_by_init {
            Remedy::Init
        } else {
            Remedy::Clean
        },
    })
}

/// `CLAUDE.md` and `AGENTS.md`, reported only when their bytes prove Lisa wrote
/// them.
///
/// This is the sharp edge of the whole inventory. The path is the project's, not
/// Lisa's, so silence is the default: a hand-written context file is the
/// operator's standing instructions to every model that reads the repository,
/// and Lisa has no business having an opinion about it. Only a byte-match
/// against a generation Lisa shipped turns it into a finding.
fn retired_context_findings(root: &Path, plan: &[InitAction]) -> Vec<CurrencyFinding> {
    let candidates: [(&str, GenerationMatcher); 2] = [
        ("CLAUDE.md", is_generated_claude_md),
        ("AGENTS.md", is_generated_agents_md),
    ];

    candidates
        .into_iter()
        .filter_map(|(name, is_generated)| {
            let path = root.join(name);
            let content = std::fs::read_to_string(&path).ok()?;
            if !is_generated(&content) {
                return None;
            }
            Some(CurrencyFinding {
                kind: CurrencyKind::Retired,
                subject: name.to_string(),
                detail: format!(
                    "Lisa generated this file and stopped maintaining it; the bytes are still \
                     exactly as Lisa wrote them, so nothing of yours is in it. What agents read \
                     now is {CURRENT_WORKFLOW_DOCUMENT}."
                ),
                remedy: if plan_removes(plan, &path) {
                    Remedy::Init
                } else {
                    Remedy::Clean
                },
            })
        })
        .collect()
}

/// `scheduling.auto_advance`, which 0.5.0 parses and ignores.
///
/// The remedy comes from init's planned `.lisa.toml`: if the file init would
/// write no longer sets the key, init is the fix. Until it does, only removal is.
fn retired_config_key_finding(
    plan: &[InitAction],
    config_table: Option<&toml::Value>,
) -> Option<CurrencyFinding> {
    if !sets_retired_key(config_table?) {
        return None;
    }

    let init_drops_it = plan.iter().any(|action| match action {
        InitAction::UpdateFile { path, content } => {
            path.file_name().is_some_and(|name| name == ".lisa.toml")
                && content
                    .parse::<toml::Value>()
                    .is_ok_and(|planned| !sets_retired_key(&planned))
        }
        _ => false,
    });

    Some(CurrencyFinding {
        kind: CurrencyKind::Retired,
        subject: format!(".lisa.toml [scheduling] {RETIRED_CONFIG_KEY}"),
        detail: "Lisa stopped reading this setting in 0.5.0. It is inert, not harmful — the \
                 board advances on its artifacts either way."
            .to_string(),
        remedy: if init_drops_it {
            Remedy::Init
        } else {
            Remedy::Clean
        },
    })
}

/// Is the retired scheduling key actively set in this parsed `.lisa.toml`?
fn sets_retired_key(table: &toml::Value) -> bool {
    table
        .get("scheduling")
        .and_then(|scheduling| scheduling.get(RETIRED_CONFIG_KEY))
        .is_some()
}

/// Tickets whose `phase:` names one of the phases retired in 0.5.0.
///
/// The board is the operator's file, and a retired value still loads — it reads
/// as `implement`, and Lisa rewrites the row itself the next time it moves the
/// ticket. So this is reported and never fixed by a Lisa command; the remedy
/// names the edit and says plainly that waiting is also a fix.
fn stale_ticket_findings(root: &Path) -> Vec<CurrencyFinding> {
    let ticket_dir = root.join(configured_ticket_dir(root));
    let Ok(entries) = std::fs::read_dir(&ticket_dir) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(phase) = frontmatter_value(&content, "phase") else {
            continue;
        };
        if !RETIRED_PHASE_NAMES.contains(&phase.as_str()) {
            continue;
        }
        findings.push(CurrencyFinding {
            kind: CurrencyKind::StaleContent,
            subject: relative(root, &path),
            detail: format!("Records `phase: {phase}`, a phase Lisa retired in 0.5.0."),
            remedy: Remedy::Operator(format!(
                "no Lisa command touches your board. Lisa already reads `{phase}` as \
                 `implement` and rewrites the row itself the next time it moves this ticket; \
                 set `phase: implement` to settle it now"
            )),
        });
    }
    findings
}

/// Where this project keeps its tickets, falling back to the default when
/// `.lisa.toml` cannot answer.
fn configured_ticket_dir(root: &Path) -> String {
    config::load_config(root)
        .ok()
        .and_then(|validation| validation.config.dirs.tickets)
        .unwrap_or_else(|| DEFAULT_TICKET_DIR.to_string())
}

/// Read one scalar value out of a markdown file's YAML frontmatter block.
///
/// Deliberately raw: [`lisa_core::types::Phase`] maps every retired name
/// forward to `implement`, so a parsed ticket can no longer say which word is
/// actually written in the file.
fn frontmatter_value(content: &str, key: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            return None;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == key {
            return Some(value.trim().trim_matches(['"', '\'']).to_string());
        }
    }
    None
}

/// Does init's plan remove exactly this path?
fn plan_removes(plan: &[InitAction], path: &Path) -> bool {
    plan.iter().any(
        |action| matches!(action, InitAction::RemoveFile { path: planned, .. } if planned == path),
    )
}

/// Render a path the way an operator would type it: relative to the project.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A project shaped like one 0.4.4 left behind: an old recorded version, the
    /// workflow document under its old name, a generated context file, the dead
    /// scheduling key, and a ticket at a retired phase.
    fn fixture_0_4_4(root: &Path) {
        fs::create_dir_all(root.join("docs/knowledge")).unwrap();
        fs::create_dir_all(root.join("docs/active/tickets")).unwrap();
        fs::create_dir_all(root.join(".lisa/hooks")).unwrap();
        // An older recorded version than any binary that can run this test. The
        // fixture is 0.4.4-*shaped*; pinning the string to 0.4.4 would make the
        // version assertion evaporate on exactly the release that ships it.
        fs::write(
            root.join(".lisa.toml"),
            "version = \"0.4.0\"\n\n[scheduling]\nmax_threads = 2\nauto_advance = true\n",
        )
        .unwrap();
        fs::write(
            root.join(RETIRED_WORKFLOW_DOCUMENT),
            crate::templates::LEGACY_WORKFLOWS[2],
        )
        .unwrap();
        fs::write(
            root.join(".lisa/hooks/on-stop.sh"),
            crate::templates::LEGACY_ON_STOP_HOOKS[0],
        )
        .unwrap();
        fs::write(
            root.join("docs/active/tickets/T-024-01.md"),
            "---\nid: T-024-01\nstory: S-024\ntitle: migrate-climate-calls\ntype: task\nstatus: open\npriority: high\nphase: structure\ndepends_on: []\n---\n\n## Context\n\nWork.\n",
        )
        .unwrap();
    }

    fn generated_claude_md() -> String {
        format!(
            "{}my-app (Rust) — TODO: add a one-line project description here.\n\n### Build and Test\n\n```bash\n# Build\ncargo build\n\n# Run tests\ncargo test\n\n# Lint\ncargo clippy\n```\n\n### Source Layout\n\n```\nsrc:\n  main.rs\n```\n\n{}",
            include_str!("../data/legacy/claude-md-header-v0.4.4.md"),
            include_str!("../data/legacy/claude-md-tail.md"),
        )
    }

    fn find<'a>(currency: &'a ProjectCurrency, subject: &str) -> Option<&'a CurrencyFinding> {
        currency
            .findings
            .iter()
            .find(|finding| finding.subject == subject)
    }

    #[test]
    fn a_0_4_4_project_reports_one_of_each_category() {
        let dir = tempfile::tempdir().unwrap();
        fixture_0_4_4(dir.path());

        let currency = inventory(dir.path());

        assert_eq!(
            currency.recorded_version,
            RecordedVersion::Behind {
                recorded: "0.4.0".to_string()
            }
        );
        assert!(!currency.is_current());

        // Behind: a hook script whose bytes are a shipped prior generation.
        let behind = find(&currency, ".lisa/hooks/on-stop.sh")
            .expect("a prior hook generation is behind, not retired");
        assert_eq!(behind.kind, CurrencyKind::Behind);
        assert_eq!(behind.remedy, Remedy::Init);

        // Retired: the workflow document under the name Lisa stopped using.
        let retired =
            find(&currency, RETIRED_WORKFLOW_DOCUMENT).expect("retired workflow document");
        assert_eq!(retired.kind, CurrencyKind::Retired);
        assert_eq!(retired.remedy, Remedy::Init, "init already removes it");
        assert!(retired.detail.contains(CURRENT_WORKFLOW_DOCUMENT));

        // Stale content inside a file Lisa does not own.
        let stale =
            find(&currency, "docs/active/tickets/T-024-01.md").expect("ticket at a retired phase");
        assert_eq!(stale.kind, CurrencyKind::StaleContent);
        assert!(stale.detail.contains("structure"));
        assert!(matches!(stale.remedy, Remedy::Operator(_)));

        // And the dead scheduling key.
        let key = currency
            .findings
            .iter()
            .find(|finding| finding.subject.contains(RETIRED_CONFIG_KEY))
            .expect("retired scheduling key");
        assert_eq!(key.kind, CurrencyKind::Retired);
    }

    #[test]
    fn every_finding_names_a_remedy() {
        let dir = tempfile::tempdir().unwrap();
        fixture_0_4_4(dir.path());
        fs::write(dir.path().join("CLAUDE.md"), generated_claude_md()).unwrap();

        let currency = inventory(dir.path());
        assert!(!currency.findings.is_empty());
        for finding in &currency.findings {
            let line = finding.remedy.line();
            assert!(
                !line.trim().is_empty(),
                "{} has an empty remedy",
                finding.subject
            );
            assert!(
                !finding.detail.trim().is_empty(),
                "{} has an empty detail",
                finding.subject
            );
        }
    }

    #[test]
    fn a_generated_context_file_is_retired_and_a_hand_written_one_is_invisible() {
        let dir = tempfile::tempdir().unwrap();
        fixture_0_4_4(dir.path());

        // Byte-identical to a 0.4.x `generate_claude_md` output.
        fs::write(dir.path().join("CLAUDE.md"), generated_claude_md()).unwrap();
        fs::write(
            dir.path().join("AGENTS.md"),
            crate::legacy_context::LEGACY_AGENTS_CONTEXTS[1],
        )
        .unwrap();

        let currency = inventory(dir.path());
        let claude = find(&currency, "CLAUDE.md").expect("a generated CLAUDE.md is Lisa's litter");
        assert_eq!(claude.kind, CurrencyKind::Retired);
        assert_eq!(
            claude.remedy,
            Remedy::Clean,
            "init does not retire context files yet, so removal is the only fix"
        );
        assert_eq!(
            find(&currency, "AGENTS.md").map(|finding| finding.kind),
            Some(CurrencyKind::Retired)
        );

        // The operator's own writing at the same paths.
        fs::write(
            dir.path().join("CLAUDE.md"),
            "# CLAUDE.md\n\nRun the climate suite before every commit.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("AGENTS.md"),
            "# AGENTS.md\n\nStart with the README.\n",
        )
        .unwrap();

        let currency = inventory(dir.path());
        assert!(
            find(&currency, "CLAUDE.md").is_none(),
            "a hand-written CLAUDE.md is the operator's and is not reported at all"
        );
        assert!(find(&currency, "AGENTS.md").is_none());
    }

    #[test]
    fn a_missing_version_key_is_pre_versioning_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 2\n",
        )
        .unwrap();

        let currency = inventory(dir.path());
        assert_eq!(currency.recorded_version, RecordedVersion::PreVersioning);
        let finding = find(&currency, ".lisa.toml").expect("pre-versioning is reported");
        assert_eq!(finding.kind, CurrencyKind::Behind);
        assert_eq!(finding.remedy, Remedy::Init);
        assert!(finding.detail.contains("before Lisa recorded a version"));
    }

    #[test]
    fn an_edited_retired_document_needs_removal_rather_than_init() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs/knowledge")).unwrap();
        fs::write(
            dir.path().join(".lisa.toml"),
            format!("version = \"{}\"\n", config::LISA_VERSION),
        )
        .unwrap();
        fs::write(
            dir.path().join(RETIRED_WORKFLOW_DOCUMENT),
            format!(
                "{}\n\nOur team's note.\n",
                crate::templates::LEGACY_WORKFLOWS[2]
            ),
        )
        .unwrap();

        let currency = inventory(dir.path());
        let finding = find(&currency, RETIRED_WORKFLOW_DOCUMENT).expect("still reported");
        assert_eq!(finding.kind, CurrencyKind::Retired);
        assert_eq!(finding.remedy, Remedy::Clean);
    }

    #[test]
    fn a_fresh_init_project_is_current() {
        let dir = tempfile::tempdir().unwrap();
        crate::init::run_init(dir.path(), false, crate::init::HistoryPreference::NoHistory)
            .expect("init must succeed in an empty directory");

        let currency = inventory(dir.path());
        assert_eq!(
            currency.findings,
            Vec::new(),
            "what init just wrote cannot already be stale"
        );
        assert!(currency.is_current());
        assert_eq!(
            currency.recorded_version,
            RecordedVersion::Current {
                recorded: config::LISA_VERSION.to_string()
            }
        );
    }

    #[test]
    fn a_directory_with_no_lisa_toml_has_nothing_to_be_current_with() {
        let dir = tempfile::tempdir().unwrap();
        let currency = inventory(dir.path());
        assert_eq!(currency.recorded_version, RecordedVersion::NoProject);
        assert!(currency.is_current());
    }

    #[test]
    fn frontmatter_reads_the_written_word_not_the_parsed_phase() {
        assert_eq!(
            frontmatter_value("---\nid: T-1\nphase: structure\n---\n\nBody\n", "phase").as_deref(),
            Some("structure")
        );
        assert_eq!(
            frontmatter_value("---\nphase: \"plan\"\n---\n", "phase").as_deref(),
            Some("plan")
        );
        // A `phase:` in the body is not frontmatter.
        assert_eq!(
            frontmatter_value("---\nid: T-1\n---\n\nphase: structure\n", "phase"),
            None
        );
        assert_eq!(frontmatter_value("no frontmatter here\n", "phase"), None);
    }
}
