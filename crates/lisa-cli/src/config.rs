use std::path::Path;

use serde::Deserialize;

use lisa_core::client::AgentClient;
use lisa_core::completion::CompletionSealMode;
use lisa_core::types::PluginConfig;

use crate::detect::{detect_agent_availability, AgentAvailability};
use crate::runtime::ZellijRuntimeRequest;

/// One operator-facing `.lisa.toml` setting.
///
/// This catalog is the shared source for validation, inert init stubs, and
/// operator documentation. `default` is a valid TOML right-hand side so a stub
/// can be enabled by removing its comment marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigKey {
    pub path: &'static str,
    pub section: &'static str,
    pub key: &'static str,
    pub default: &'static str,
    pub description: &'static str,
}

impl ConfigKey {
    /// Render an inert, directly enableable assignment with its purpose.
    pub(crate) fn commented_stub(self) -> String {
        format!("# {}\n# {} = {}", self.description, self.key, self.default)
    }
}

/// Every fixed key parsed from `.lisa.toml`, in operator-facing render order.
///
/// Map children under `phase_timeouts` and `provider_caps` are not fixed config
/// keys: their parent maps are cataloged here and their allowed child names are
/// validated separately.
pub(crate) const CONFIG_KEYS: &[ConfigKey] = &[
    ConfigKey {
        path: "version",
        section: "",
        key: "version",
        default: concat!("\"", env!("CARGO_PKG_VERSION"), "\""),
        description: "Tracks the Lisa version used to set up this project.",
    },
    ConfigKey {
        path: "dirs.tickets",
        section: "dirs",
        key: "tickets",
        default: "\"docs/active/tickets\"",
        description: "Chooses where Lisa reads ticket files.",
    },
    ConfigKey {
        path: "dirs.stories",
        section: "dirs",
        key: "stories",
        default: "\"docs/active/stories\"",
        description: "Chooses where Lisa reads story files.",
    },
    ConfigKey {
        path: "dirs.work",
        section: "dirs",
        key: "work",
        default: "\"docs/active/work\"",
        description: "Chooses where Lisa keeps work records.",
    },
    ConfigKey {
        path: "runtime.zellij",
        section: "runtime",
        key: "zellij",
        default: "\"managed\"",
        description: "Chooses how Lisa starts Zellij.",
    },
    ConfigKey {
        path: "agent.client",
        section: "agent",
        key: "client",
        default: "\"claude\"",
        description:
            "Chooses which coding agent Lisa drives. Omit it to detect agents on PATH; claude is the default when both are installed.",
    },
    ConfigKey {
        path: "agent.model",
        section: "agent",
        key: "model",
        default: "\"opus\"",
        description:
            "Chooses which model that agent runs. Omit it to use whatever the agent runs by default.",
    },
    ConfigKey {
        path: "agent.effort",
        section: "agent",
        key: "effort",
        default: "\"medium\"",
        description:
            "Chooses how hard that agent thinks (low, medium, high, xhigh, max). Omit it to use whatever the agent runs by default.",
    },
    ConfigKey {
        path: "guards.completion",
        section: "guards",
        key: "completion",
        default: "\"auto\"",
        description:
            "Controls how finished work is sealed. auto picks the strongest your project supports.",
    },
    ConfigKey {
        path: "triage.enabled",
        section: "triage",
        key: "enabled",
        default: "true",
        description: "Lets Lisa inspect work that needs you before asking for help.",
    },
    ConfigKey {
        path: "triage.timeout_secs",
        section: "triage",
        key: "timeout_secs",
        default: "120",
        description: "Limits how long Lisa can inspect work that needs you.",
    },
    ConfigKey {
        path: "scheduling.max_threads",
        section: "scheduling",
        key: "max_threads",
        default: "2",
        description: "Limits how many coding agents can work at once.",
    },
    ConfigKey {
        path: "scheduling.review_timeout_secs",
        section: "scheduling",
        key: "review_timeout_secs",
        default: "600",
        description: "Limits how long Lisa waits for review to finish.",
    },
    ConfigKey {
        path: "scheduling.session_timeout_secs",
        section: "scheduling",
        key: "session_timeout_secs",
        default: "3600",
        description: "Limits how long one coding-agent session can run.",
    },
    ConfigKey {
        path: "scheduling.wind_down_secs",
        section: "scheduling",
        key: "wind_down_secs",
        default: "300",
        description: "Sets aside time for an agent to wrap up before its session ends.",
    },
    ConfigKey {
        path: "scheduling.assignment_ack_timeout_secs",
        section: "scheduling",
        key: "assignment_ack_timeout_secs",
        default: "30",
        description: "Limits how long Lisa waits for an agent to accept assigned work.",
    },
    ConfigKey {
        path: "scheduling.phase_timeouts",
        section: "scheduling",
        key: "phase_timeouts",
        default: "{}",
        description: "Limits how long each kind of work can run.",
    },
    ConfigKey {
        path: "scheduling.provider_caps",
        section: "scheduling",
        key: "provider_caps",
        default: "{}",
        description: "Limits how many agents of each kind can work at once.",
    },
];

pub(crate) fn config_key(path: &str) -> Option<&'static ConfigKey> {
    CONFIG_KEYS.iter().find(|entry| entry.path == path)
}

fn is_known_top_level(key: &str) -> bool {
    CONFIG_KEYS
        .iter()
        .any(|entry| entry.path == key || entry.section == key)
}

fn is_known_section_key(section: &str, key: &str) -> bool {
    CONFIG_KEYS
        .iter()
        .any(|entry| entry.section == section && entry.key == key)
}

/// Top-level .lisa.toml structure.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct LisaConfig {
    pub version: Option<String>,
    #[serde(default)]
    pub dirs: DirsConfig,
    #[serde(default)]
    pub scheduling: SchedulingConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub guards: GuardsConfig,
    #[serde(default)]
    pub triage: TriageConfig,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct TriageConfig {
    pub enabled: Option<bool>,
    pub timeout_secs: Option<u64>,
}

/// Completion guard selection (`[guards]`).
///
/// Kept raw so invalid values surface through the same actionable semantic
/// validation path as `[agent].client`.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct GuardsConfig {
    pub completion: Option<String>,
}

/// Zellij runtime selection (`[runtime]`).
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    /// `managed` (default), `system`, or an absolute executable path.
    pub zellij: Option<String>,
}

/// Agent client selection section (`[agent]`).
///
/// `client` is kept a raw `String` here so an invalid value surfaces as an
/// actionable *validation* error (via [`validate_config`] / [`AgentClient::parse`])
/// rather than a raw serde deserialize failure.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub client: Option<String>,
    /// The model this board runs within its client, or `None` to leave the
    /// choice to the client's own default. Opaque to Lisa — provider model
    /// vocabulary lives with the provider — so it is passed through as written
    /// and only checked for being a non-empty name.
    pub model: Option<String>,
    /// How hard this board's agent thinks by default, or `None` to leave the
    /// choice to the client's own default (T-071-01-01). Unlike `model` this is
    /// *not* opaque: it is a fixed, Lisa-known vocabulary
    /// ([`lisa_core::effort::Effort`]), so [`validate_config`] refuses an
    /// unknown value outright rather than only checking it is non-empty.
    pub effort: Option<String>,
}

/// Directory configuration section.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct DirsConfig {
    pub tickets: Option<String>,
    pub stories: Option<String>,
    pub work: Option<String>,
}

/// Scheduling configuration section.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct SchedulingConfig {
    pub max_threads: Option<usize>,
    pub review_timeout_secs: Option<u64>,
    pub session_timeout_secs: Option<u64>,
    pub wind_down_secs: Option<u64>,
    pub assignment_ack_timeout_secs: Option<u64>,
    pub phase_timeouts: Option<std::collections::HashMap<String, u64>>,
    /// Optional per-provider concurrency sub-caps (T-026-02), keyed by raw
    /// client name (`claude` | `codex`). Kept as raw strings here so an invalid
    /// provider name or a `0` cap surfaces as an actionable *validation* error
    /// via [`validate_config`] rather than a raw serde failure, mirroring how
    /// `[agent].client` is handled.
    pub provider_caps: Option<std::collections::HashMap<String, usize>>,
}

/// Why the resolved agent client was selected for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientResolution {
    Cli,
    Config,
    Detected(AgentAvailability),
}

/// Fully resolved configuration with all defaults applied.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Machine-readable project setup/protocol version from `.lisa.toml`.
    pub project_version: String,
    pub ticket_dir: String,
    pub story_dir: String,
    pub work_dir: String,
    pub max_threads: usize,
    pub review_timeout_secs: u64,
    pub session_timeout_secs: u64,
    pub wind_down_secs: u64,
    pub assignment_ack_timeout_secs: u64,
    pub phase_timeouts: std::collections::HashMap<String, u64>,
    pub client: AgentClient,
    /// The model the board is configured to run within that client, or `None`
    /// when it leaves that to the client's default. A ticket's own `model:`
    /// frontmatter still outranks it at spawn.
    pub model: Option<String>,
    /// The effort the board is configured to run within that client, or `None`
    /// when it leaves that to the client's default (T-071-01-01). A ticket's
    /// own `effort:` frontmatter still outranks it at spawn, mirroring `model`.
    pub effort: Option<String>,
    pub(crate) client_resolution: ClientResolution,
    pub zellij_runtime: ZellijRuntimeRequest,
    /// Configured completion intent. A real loop pins this against its startup
    /// environment exactly once before any scheduler side effects.
    pub completion_mode: CompletionSealMode,
    /// Resolved per-provider concurrency sub-caps, keyed by raw client name.
    /// Empty when none configured (T-026-02).
    pub provider_caps: std::collections::HashMap<String, usize>,
    pub triage_enabled: bool,
    pub triage_timeout_secs: u64,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            project_version: LISA_VERSION.to_string(),
            ticket_dir: PluginConfig::DEFAULT_TICKET_DIR.to_string(),
            story_dir: PluginConfig::DEFAULT_STORY_DIR.to_string(),
            work_dir: PluginConfig::DEFAULT_WORK_DIR.to_string(),
            max_threads: PluginConfig::DEFAULT_MAX_THREADS,
            review_timeout_secs: PluginConfig::DEFAULT_REVIEW_TIMEOUT_SECS,
            session_timeout_secs: PluginConfig::DEFAULT_SESSION_TIMEOUT_SECS,
            wind_down_secs: PluginConfig::DEFAULT_WIND_DOWN_SECS,
            assignment_ack_timeout_secs: PluginConfig::DEFAULT_ASSIGNMENT_ACK_TIMEOUT_SECS,
            phase_timeouts: std::collections::HashMap::new(),
            client: AgentClient::default(),
            model: None,
            effort: None,
            client_resolution: ClientResolution::Detected(AgentAvailability::Both),
            zellij_runtime: crate::runtime::default_runtime_request(),
            completion_mode: CompletionSealMode::Auto,
            provider_caps: std::collections::HashMap::new(),
            triage_enabled: PluginConfig::DEFAULT_TRIAGE_ENABLED,
            triage_timeout_secs: PluginConfig::DEFAULT_TRIAGE_TIMEOUT_SECS,
        }
    }
}

impl ResolvedConfig {
    /// One brand-voiced sentence explaining the selected loop client.
    pub(crate) fn client_announcement(&self) -> &'static str {
        match self.client_resolution {
            ClientResolution::Cli => match self.client {
                AgentClient::Claude => "Driving Claude — selected by --client.",
                AgentClient::Codex => "Driving Codex — selected by --client.",
            },
            ClientResolution::Config => match self.client {
                AgentClient::Claude => "Driving Claude — selected in .lisa.toml.",
                AgentClient::Codex => "Driving Codex — selected in .lisa.toml.",
            },
            ClientResolution::Detected(AgentAvailability::Neither) => {
                "Driving Claude — neither agent is installed; claude is the default."
            }
            ClientResolution::Detected(AgentAvailability::ClaudeOnly) => {
                "Driving Claude — it's the agent installed here."
            }
            ClientResolution::Detected(AgentAvailability::CodexOnly) => {
                "Driving Codex — it's the agent installed here."
            }
            ClientResolution::Detected(AgentAvailability::Both) => {
                "Driving Claude — both agents are installed; claude is the default."
            }
        }
    }
}

/// Load .lisa.toml from the project root.
///
/// Returns config with empty warnings if the file does not exist.
/// Returns `Err` if the file exists but cannot be parsed or has invalid values.
pub fn load_config(root: &Path) -> Result<ConfigValidation, String> {
    let config_path = root.join(".lisa.toml");
    if !config_path.exists() {
        return Ok(ConfigValidation {
            config: LisaConfig::default(),
            warnings: vec![],
        });
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

    validate_config(&content).map_err(|e| format!("{}: {}", config_path.display(), e))
}

/// Merge config file values with CLI overrides.
///
/// Precedence (lowest to highest): PATH detection < .lisa.toml < CLI flags.
pub fn resolve_config(
    config: &LisaConfig,
    cli_max_threads: Option<usize>,
    cli_client: Option<AgentClient>,
) -> ResolvedConfig {
    let availability = if cli_client.is_some() || config.agent.client.is_some() {
        // Explicit selection wins without consulting the environment.
        AgentAvailability::Both
    } else {
        detect_agent_availability()
    };
    resolve_config_with_availability(config, cli_max_threads, cli_client, availability)
}

fn resolve_config_with_availability(
    config: &LisaConfig,
    cli_max_threads: Option<usize>,
    cli_client: Option<AgentClient>,
    availability: AgentAvailability,
) -> ResolvedConfig {
    let defaults = ResolvedConfig::default();

    // Client precedence mirrors max_threads: --client > [agent].client > PATH detection.
    // The config value has already been validated by `validate_config`; `.ok()`
    // is a defensive fallback (an unparseable value degrades to the supplied
    // availability rather than panicking here).
    let configured_client = config
        .agent
        .client
        .as_deref()
        .and_then(|s| AgentClient::parse(s).ok());
    let (client, client_resolution) = if let Some(client) = cli_client {
        (client, ClientResolution::Cli)
    } else if let Some(client) = configured_client {
        (client, ClientResolution::Config)
    } else {
        let client = match availability {
            AgentAvailability::CodexOnly => AgentClient::Codex,
            AgentAvailability::Neither
            | AgentAvailability::ClaudeOnly
            | AgentAvailability::Both => AgentClient::Claude,
        };
        (client, ClientResolution::Detected(availability))
    };

    ResolvedConfig {
        project_version: config
            .version
            .clone()
            .unwrap_or_else(|| defaults.project_version.clone()),
        client,
        model: config
            .agent
            .model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(str::to_string),
        effort: config
            .agent
            .effort
            .as_deref()
            .map(str::trim)
            .filter(|effort| !effort.is_empty())
            .map(str::to_string),
        client_resolution,
        zellij_runtime: match config.runtime.zellij.as_deref() {
            None => crate::runtime::default_runtime_request(),
            Some("managed") => ZellijRuntimeRequest::Managed,
            Some("system") => ZellijRuntimeRequest::System,
            Some(path) => ZellijRuntimeRequest::Pinned(path.into()),
        },
        completion_mode: config
            .guards
            .completion
            .as_deref()
            .and_then(|value| CompletionSealMode::parse(value).ok())
            .unwrap_or(defaults.completion_mode),
        ticket_dir: config.dirs.tickets.clone().unwrap_or(defaults.ticket_dir),
        story_dir: config.dirs.stories.clone().unwrap_or(defaults.story_dir),
        work_dir: config.dirs.work.clone().unwrap_or(defaults.work_dir),
        max_threads: cli_max_threads
            .or(config.scheduling.max_threads)
            .unwrap_or(defaults.max_threads),
        review_timeout_secs: config
            .scheduling
            .review_timeout_secs
            .unwrap_or(defaults.review_timeout_secs),
        session_timeout_secs: config
            .scheduling
            .session_timeout_secs
            .unwrap_or(defaults.session_timeout_secs),
        wind_down_secs: config
            .scheduling
            .wind_down_secs
            .unwrap_or(defaults.wind_down_secs),
        assignment_ack_timeout_secs: config
            .scheduling
            .assignment_ack_timeout_secs
            .unwrap_or(defaults.assignment_ack_timeout_secs),
        phase_timeouts: config.scheduling.phase_timeouts.clone().unwrap_or_default(),
        provider_caps: config.scheduling.provider_caps.clone().unwrap_or_default(),
        triage_enabled: config.triage.enabled.unwrap_or(defaults.triage_enabled),
        triage_timeout_secs: config
            .triage
            .timeout_secs
            .unwrap_or(defaults.triage_timeout_secs),
    }
}

/// The `[scheduling]` section holding the key 0.5.0 stopped reading.
pub(crate) const RETIRED_SCHEDULING_SECTION: &str = "scheduling";
/// The key itself. Parsed and ignored since 0.5.0; removable since T-057-02-02.
pub(crate) const RETIRED_SCHEDULING_KEY: &str = "auto_advance";

/// What lifting `scheduling.auto_advance` out of an operator's `.lisa.toml`
/// would cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RetiredKeyRemoval {
    /// The file does not set the key — or does not parse, in which case it is
    /// not this function's problem.
    Absent,
    /// The whole file, minus one line. Every other byte is the original's.
    Removed(String),
    /// The key is set in a shape no line deletion can express. The string says
    /// which, already worded for a preview line.
    NotSurgical(&'static str),
}

/// Remove `[scheduling] auto_advance` from a `.lisa.toml`, or refuse.
///
/// `.lisa.toml` is a file the operator edits: it holds their concurrency, model
/// routing and client choices, and their comments explaining why. So this is
/// line surgery in the manner of [`crate::init::upsert_missing_config_keys`],
/// never a `toml::Value` round trip — nothing is reserialized, so no comment,
/// no key order and no whitespace can be lost.
///
/// The post-condition is what makes it safe rather than clever: **every other
/// byte survives by construction, and every other key survives by assertion.**
/// When either cannot be guaranteed the answer is [`RetiredKeyRemoval::NotSurgical`]
/// and the file is left exactly as it is. A `.lisa.toml` that comes back with
/// its comments stripped is a worse outcome than a dead key sitting inert.
pub(crate) fn remove_retired_scheduling_key(existing: &str) -> RetiredKeyRemoval {
    let Ok(original) = existing.parse::<toml::Value>() else {
        return RetiredKeyRemoval::Absent;
    };
    if !sets_retired_scheduling_key(&original) {
        return RetiredKeyRemoval::Absent;
    }

    // Rejoining lines would rewrite every CRLF in the file into an LF, which is
    // a reformat of the whole thing to remove one line of it.
    if existing.contains('\r') {
        return RetiredKeyRemoval::NotSurgical(
            "removing it would rewrite every line ending in the file",
        );
    }

    let candidates = retired_key_lines(existing);
    let [line_to_drop] = candidates[..] else {
        return RetiredKeyRemoval::NotSurgical(if candidates.is_empty() {
            "the setting is not written as a line of its own"
        } else {
            "the file assigns the setting more than once"
        });
    };

    let kept: Vec<&str> = existing
        .lines()
        .enumerate()
        .filter(|(index, _)| *index != line_to_drop)
        .map(|(_, line)| line)
        .collect();
    let mut updated = kept.join("\n");
    if existing.ends_with('\n') {
        updated.push('\n');
    }

    let Ok(reparsed) = updated.parse::<toml::Value>() else {
        return RetiredKeyRemoval::NotSurgical("removing that line leaves the file unparseable");
    };
    if sets_retired_scheduling_key(&reparsed) {
        return RetiredKeyRemoval::NotSurgical("the setting survives its own line being removed");
    }
    if without_retired_scheduling_key(original) != without_retired_scheduling_key(reparsed) {
        return RetiredKeyRemoval::NotSurgical("removing that line would change another setting");
    }

    RetiredKeyRemoval::Removed(updated)
}

/// Does this parsed `.lisa.toml` actively set the retired scheduling key?
fn sets_retired_scheduling_key(table: &toml::Value) -> bool {
    table
        .get(RETIRED_SCHEDULING_SECTION)
        .and_then(|scheduling| scheduling.get(RETIRED_SCHEDULING_KEY))
        .is_some()
}

/// The same parsed file with the retired key gone, for comparing what a line
/// deletion actually did against what it was supposed to do.
///
/// A `[scheduling]` section left empty is dropped, so a file that set the key
/// through a top-level `scheduling.auto_advance` — where deleting the line
/// takes the whole table with it — compares equal to one that kept an empty
/// header behind.
fn without_retired_scheduling_key(mut value: toml::Value) -> toml::Value {
    let Some(toml::Value::Table(scheduling)) = value.get_mut(RETIRED_SCHEDULING_SECTION) else {
        return value;
    };
    scheduling.remove(RETIRED_SCHEDULING_KEY);
    if scheduling.is_empty() {
        if let Some(table) = value.as_table_mut() {
            table.remove(RETIRED_SCHEDULING_SECTION);
        }
    }
    value
}

/// Every line that could be the live assignment of the retired key.
///
/// Commented lines are not candidates: the caller has already established from
/// the parsed file that the key is live, so a commented one is somebody's note.
/// A dotted `scheduling.auto_advance` counts only above the first section
/// header, where it means the same table.
fn retired_key_lines(existing: &str) -> Vec<usize> {
    let dotted = format!("{RETIRED_SCHEDULING_SECTION}.{RETIRED_SCHEDULING_KEY}");
    let mut section: Option<String> = None;
    let mut candidates = Vec::new();

    for (index, line) in existing.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(header) = trimmed.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            section = Some(header.trim().to_string());
            continue;
        }
        let assigns = |name: &str| {
            trimmed
                .strip_prefix(name)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        };
        let is_candidate = match section.as_deref() {
            Some(RETIRED_SCHEDULING_SECTION) => assigns(RETIRED_SCHEDULING_KEY),
            None => assigns(&dotted),
            Some(_) => false,
        };
        if is_candidate {
            candidates.push(index);
        }
    }

    candidates
}

/// Result of config validation, containing the parsed config and any warnings.
#[derive(Debug)]
pub struct ConfigValidation {
    pub config: LisaConfig,
    pub warnings: Vec<String>,
}

/// Validate TOML config content: check for unknown keys and semantic constraints.
///
/// Returns the parsed config plus any warnings about unknown keys.
/// Returns `Err` for parse failures or invalid values (e.g. max_threads = 0).
pub fn validate_config(content: &str) -> Result<ConfigValidation, String> {
    // Parse as generic Value to detect unknown keys
    let value: toml::Value = content
        .parse()
        .map_err(|e: toml::de::Error| format!("Invalid TOML: {}", e))?;

    let mut warnings = Vec::new();

    if let Some(table) = value.as_table() {
        for key in table.keys() {
            if !is_known_top_level(key) {
                warnings.push(format!("Unknown config section: [{}]", key));
            }
        }

        if let Some(toml::Value::Table(dirs)) = table.get("dirs") {
            for key in dirs.keys() {
                if !is_known_section_key("dirs", key) {
                    warnings.push(format!("Unknown key in [dirs]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(agent)) = table.get("agent") {
            for key in agent.keys() {
                if !is_known_section_key("agent", key) {
                    warnings.push(format!("Unknown key in [agent]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(runtime)) = table.get("runtime") {
            for key in runtime.keys() {
                if !is_known_section_key("runtime", key) {
                    warnings.push(format!("Unknown key in [runtime]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(guards)) = table.get("guards") {
            for key in guards.keys() {
                if !is_known_section_key("guards", key) {
                    warnings.push(format!("Unknown key in [guards]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(triage)) = table.get("triage") {
            for key in triage.keys() {
                if !is_known_section_key("triage", key) {
                    warnings.push(format!("Unknown key in [triage]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(sched)) = table.get("scheduling") {
            for key in sched.keys() {
                if !is_known_section_key("scheduling", key) {
                    warnings.push(format!("Unknown key in [scheduling]: {}", key));
                }
            }

            // Validate phase names in [scheduling.phase_timeouts].
            //
            // The board has two working phases. The four retired in 0.5.0 stay
            // accepted here for the same reason `Phase` still deserializes
            // them: a 0.4 project's config must not start warning about keys
            // that were correct when it wrote them. They resolve to
            // `Phase::Implement`, exactly as the frontmatter values do.
            let known_phases = &[
                "implement",
                "review",
                "research",
                "design",
                "structure",
                "plan",
            ];
            if let Some(toml::Value::Table(pt)) = sched.get("phase_timeouts") {
                for key in pt.keys() {
                    if !known_phases.contains(&key.as_str()) {
                        warnings.push(format!(
                            "Unknown phase in [scheduling.phase_timeouts]: {}",
                            key
                        ));
                    }
                }
            }

            // Validate provider names in [scheduling.provider_caps] (T-026-02).
            if let Some(toml::Value::Table(caps)) = sched.get("provider_caps") {
                for key in caps.keys() {
                    if !AgentClient::VALID.contains(&key.as_str()) {
                        warnings.push(format!(
                            "Unknown provider in [scheduling.provider_caps]: {}",
                            key
                        ));
                    }
                }
            }
        }
    }

    // Deserialize into typed struct
    let config: LisaConfig =
        toml::from_str(content).map_err(|e| format!("Invalid config value: {}", e))?;

    // Semantic validation
    if config.scheduling.max_threads == Some(0) {
        return Err("max_threads must be at least 1".to_string());
    }
    if config.scheduling.assignment_ack_timeout_secs == Some(0) {
        return Err("assignment_ack_timeout_secs must be at least 1".to_string());
    }
    if config.triage.timeout_secs == Some(0) {
        return Err("triage timeout_secs must be at least 1".to_string());
    }
    if let Some(client) = &config.agent.client {
        AgentClient::parse(client)?;
    }
    // Lisa never interprets a model name — that vocabulary belongs to the
    // provider — so the only thing it can honestly check is that a named model
    // is actually a name.
    if config
        .agent
        .model
        .as_deref()
        .is_some_and(|model| model.trim().is_empty())
    {
        return Err("[agent].model must name a model, or be left out entirely".to_string());
    }
    // Unlike model, effort is *not* provider vocabulary — it is a small, fixed
    // set Lisa itself defines — so a bad value is refused right here, naming
    // the key, rather than surfacing at the far end of a run that has already
    // started four panes (T-071-01-01 AC).
    if let Some(effort) = config.agent.effort.as_deref() {
        lisa_core::effort::Effort::parse(effort).map_err(|e| format!("[agent].effort: {e}"))?;
    }
    if let Some(zellij) = config.runtime.zellij.as_deref() {
        if !matches!(zellij, "managed" | "system") && !Path::new(zellij).is_absolute() {
            return Err(format!(
                "[runtime].zellij must be `managed`, `system`, or an absolute path (got {zellij:?})"
            ));
        }
    }
    if let Some(completion) = config.guards.completion.as_deref() {
        CompletionSealMode::parse(completion)?;
    }
    // A per-provider cap of 0 would starve that provider forever; reject it with
    // the same "at least 1" contract as max_threads (T-026-02).
    if let Some(caps) = &config.scheduling.provider_caps {
        for (name, cap) in caps {
            if *cap == 0 {
                return Err(format!("provider cap for '{}' must be at least 1", name));
            }
        }
    }

    Ok(ConfigValidation { config, warnings })
}

/// The current Lisa CLI version, used for project version tracking.
pub const LISA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns true if `project_version` is older than `current_version`.
/// Compares SemVer core and prerelease identifiers, including rc generations.
/// Returns true (needs update) if parsing fails.
pub fn version_is_stale(project_version: &str, current_version: &str) -> bool {
    #[derive(Debug, PartialEq, Eq)]
    struct ParsedVersion<'a> {
        core: (u32, u32, u32),
        prerelease: Option<Vec<&'a str>>,
    }

    fn parse_semver(s: &str) -> Option<ParsedVersion<'_>> {
        let without_build = s.split('+').next().unwrap_or(s);
        let (core, prerelease) = match without_build.split_once('-') {
            Some((core, prerelease)) if !prerelease.is_empty() => {
                (core, Some(prerelease.split('.').collect()))
            }
            Some(_) => return None,
            None => (without_build, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(ParsedVersion {
            core: (
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ),
            prerelease,
        })
    }

    fn prerelease_cmp(left: &[&str], right: &[&str]) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        for (left, right) in left.iter().zip(right.iter()) {
            let ordering = match (left.parse::<u64>(), right.parse::<u64>()) {
                (Ok(left), Ok(right)) => left.cmp(&right),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => left.cmp(right),
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        left.len().cmp(&right.len())
    }

    match (parse_semver(project_version), parse_semver(current_version)) {
        (Some(project), Some(current)) => match project.core.cmp(&current.core) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => match (&project.prerelease, &current.prerelease) {
                (None, None) => false,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (Some(project), Some(current)) => {
                    prerelease_cmp(project, current) == std::cmp::Ordering::Less
                }
            },
        },
        _ => true,
    }
}

/// Returns the default .lisa.toml content for `lisa init`.
pub fn default_config_toml() -> String {
    let key = |path| config_key(path).expect("every rendered config key must be cataloged");

    format!(
        r#"# Lisa project configuration
# {}
version = {}

[dirs]
# {}
tickets = {}
# {}
stories = {}
# {}
work = {}

[runtime]
{}

[agent]
{}
{}
{}

[guards]
{}

[triage]
{}
{}

[scheduling]
# {}
max_threads = {}
{}
{}
{}
{}
{}
{}
"#,
        key("version").description,
        key("version").default,
        key("dirs.tickets").description,
        key("dirs.tickets").default,
        key("dirs.stories").description,
        key("dirs.stories").default,
        key("dirs.work").description,
        key("dirs.work").default,
        key("runtime.zellij").commented_stub(),
        key("agent.client").commented_stub(),
        key("agent.model").commented_stub(),
        key("agent.effort").commented_stub(),
        key("guards.completion").commented_stub(),
        key("triage.enabled").commented_stub(),
        key("triage.timeout_secs").commented_stub(),
        key("scheduling.max_threads").description,
        key("scheduling.max_threads").default,
        key("scheduling.review_timeout_secs").commented_stub(),
        key("scheduling.session_timeout_secs").commented_stub(),
        key("scheduling.wind_down_secs").commented_stub(),
        key("scheduling.assignment_ack_timeout_secs").commented_stub(),
        key("scheduling.phase_timeouts").commented_stub(),
        key("scheduling.provider_caps").commented_stub(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    const README: &str = include_str!("../../../README.md");

    const COMPLETE_CONFIG_FIXTURE: &str = r#"version = "0.4.4"

[dirs]
tickets = "tickets"
stories = "stories"
work = "work"

[runtime]
zellij = "managed"

[agent]
client = "codex"
model = "gpt-5-mini"
effort = "high"

[guards]
completion = "journal"

[triage]
enabled = false
timeout_secs = 9

[scheduling]
max_threads = 3
review_timeout_secs = 10
session_timeout_secs = 20
wind_down_secs = 5
assignment_ack_timeout_secs = 2
phase_timeouts = {}
provider_caps = {}
"#;

    #[derive(Debug, PartialEq, Eq)]
    struct ReadmeConfigRow {
        default: String,
        description: String,
    }

    fn strip_code_ticks(cell: &str) -> &str {
        cell.strip_prefix('`')
            .and_then(|cell| cell.strip_suffix('`'))
            .unwrap_or(cell)
    }

    fn parse_readme_config_table(
        readme: &str,
    ) -> Result<BTreeMap<String, ReadmeConfigRow>, String> {
        const HEADER: &str = "| Key | Default | Description |";
        const SEPARATOR: &str = "|-----|---------|-------------|";

        let lines: Vec<&str> = readme.lines().collect();
        let header_index = lines
            .iter()
            .position(|line| *line == HEADER)
            .ok_or_else(|| "README configuration table header is missing".to_string())?;

        if lines.get(header_index + 1) != Some(&SEPARATOR) {
            return Err("README configuration table separator is missing".to_string());
        }

        let mut rows = BTreeMap::new();
        for line in lines
            .iter()
            .skip(header_index + 2)
            .take_while(|line| line.starts_with('|'))
        {
            let body = line
                .strip_prefix('|')
                .and_then(|line| line.strip_suffix('|'))
                .ok_or_else(|| format!("malformed README configuration row: {line}"))?;
            let cells: Vec<&str> = body.split('|').map(str::trim).collect();
            if cells.len() != 3 {
                return Err(format!("malformed README configuration row: {line}"));
            }

            let path = strip_code_ticks(cells[0]);
            if path.is_empty() {
                return Err("README configuration table contains an empty key".to_string());
            }

            let row = ReadmeConfigRow {
                default: strip_code_ticks(cells[1]).to_string(),
                description: cells[2].to_string(),
            };
            if rows.insert(path.to_string(), row).is_some() {
                return Err(format!(
                    "README configuration table contains duplicate key `{path}`"
                ));
            }
        }

        Ok(rows)
    }

    fn readme_default(default: &str) -> &str {
        default
            .strip_prefix('"')
            .and_then(|default| default.strip_suffix('"'))
            .unwrap_or(default)
    }

    fn verify_readme_config_table(catalog: &[ConfigKey], readme: &str) -> Result<(), String> {
        let rows = parse_readme_config_table(readme)?;

        for entry in catalog {
            let row = rows.get(entry.path).ok_or_else(|| {
                format!(
                    "README configuration table is missing description for `{}`",
                    entry.path
                )
            })?;
            if row.description.is_empty() {
                return Err(format!(
                    "README configuration table is missing description for `{}`",
                    entry.path
                ));
            }

            // The version key's catalog default is the literal crate version;
            // rendering that literal into the README would force a README edit
            // on every release bump (first tripped by the 0.4.4-rc.2 cut).
            // The README describes it symbolically instead.
            let expected_default = if entry.path == "version" {
                "the installed Lisa version"
            } else {
                readme_default(entry.default)
            };
            if row.default != expected_default {
                return Err(format!(
                    "README configuration default for `{}` is {:?}; expected {:?}",
                    entry.path, row.default, expected_default
                ));
            }
            if row.description != entry.description {
                return Err(format!(
                    "README configuration description for `{}` is {:?}; expected {:?}",
                    entry.path, row.description, entry.description
                ));
            }
        }

        if let Some(extra) = rows
            .keys()
            .find(|path| !catalog.iter().any(|entry| entry.path == path.as_str()))
        {
            return Err(format!(
                "README configuration table documents unknown key `{extra}`"
            ));
        }

        Ok(())
    }

    fn parsed_fixed_paths(content: &str) -> BTreeSet<String> {
        let value: toml::Value = content.parse().unwrap();
        let root = value.as_table().unwrap();
        let mut paths = BTreeSet::new();

        for (top_level, value) in root {
            if top_level == "version" {
                paths.insert(top_level.clone());
                continue;
            }

            let section = value
                .as_table()
                .unwrap_or_else(|| panic!("[{top_level}] must be a table in the complete fixture"));
            for key in section.keys() {
                paths.insert(format!("{top_level}.{key}"));
            }
        }

        paths
    }

    #[test]
    fn config_catalog_covers_every_parsed_key_exactly_once() {
        validate_config(COMPLETE_CONFIG_FIXTURE).unwrap();

        let parsed = parsed_fixed_paths(COMPLETE_CONFIG_FIXTURE);
        let catalog: BTreeSet<String> = CONFIG_KEYS
            .iter()
            .map(|entry| entry.path.to_string())
            .collect();

        assert_eq!(
            catalog.len(),
            CONFIG_KEYS.len(),
            "config catalog contains a duplicate dotted path"
        );
        assert_eq!(
            catalog, parsed,
            "config catalog and the complete parsed-key fixture differ"
        );

        let section_keys: BTreeSet<(&str, &str)> = CONFIG_KEYS
            .iter()
            .map(|entry| (entry.section, entry.key))
            .collect();
        assert_eq!(
            section_keys.len(),
            CONFIG_KEYS.len(),
            "config catalog contains a duplicate section/key pair"
        );
    }

    #[test]
    fn config_catalog_defaults_are_valid_toml() {
        for entry in CONFIG_KEYS {
            let assignment = if entry.section.is_empty() {
                format!("{} = {}\n", entry.key, entry.default)
            } else {
                format!("[{}]\n{} = {}\n", entry.section, entry.key, entry.default)
            };
            assignment.parse::<toml::Value>().unwrap_or_else(|error| {
                panic!(
                    "{} has invalid TOML default {:?}: {error}",
                    entry.path, entry.default
                )
            });
        }
    }

    #[test]
    fn config_catalog_descriptions_pass_brand_voice_checks() {
        const DIRECT_VERBS: &[&str] = &[
            "Tracks ",
            "Chooses ",
            "Controls ",
            "Lets ",
            "Limits ",
            "Sets ",
        ];
        const BANNED: &[&str] = &[
            "dag",
            "orchestrat",
            "scheduling",
            "leverage",
            "solutions",
            "deployment",
            "case study",
            "build log",
            "research release",
        ];

        for entry in CONFIG_KEYS {
            assert!(
                !entry.description.contains(['\n', '\r']),
                "{} description must stay on one line: {:?}",
                entry.path,
                entry.description
            );
            assert!(
                entry.description.ends_with('.'),
                "{} description must be a complete sentence: {:?}",
                entry.path,
                entry.description
            );
            assert!(
                DIRECT_VERBS
                    .iter()
                    .any(|verb| entry.description.starts_with(verb)),
                "{} description must lead with a direct verb: {:?}",
                entry.path,
                entry.description
            );

            let lower = entry.description.to_ascii_lowercase();
            for banned in BANNED {
                assert!(
                    !lower.contains(banned),
                    "{} description contains banned voice term {banned:?}: {:?}",
                    entry.path,
                    entry.description
                );
            }
        }
    }

    #[test]
    fn default_config_renders_every_catalog_description_and_default() {
        let rendered = default_config_toml();

        for entry in CONFIG_KEYS {
            assert!(
                rendered.contains(entry.description),
                "fresh config is missing the {} description",
                entry.path
            );
            assert!(
                rendered.contains(&format!("{} = {}", entry.key, entry.default)),
                "fresh config is missing the {} default",
                entry.path
            );
        }
    }

    #[test]
    fn readme_config_table_matches_catalog() {
        verify_readme_config_table(CONFIG_KEYS, README).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn readme_config_table_names_missing_fake_description() {
        let mut catalog = CONFIG_KEYS.to_vec();
        catalog.push(ConfigKey {
            path: "fake.missing_description",
            section: "fake",
            key: "missing_description",
            default: "false",
            description: "Controls a fake setting.",
        });

        let error = verify_readme_config_table(&catalog, README).unwrap_err();
        assert!(error.contains("missing description"), "{error}");
        assert!(error.contains("fake.missing_description"), "{error}");
    }

    #[test]
    fn test_parse_full_config() {
        let toml_str = r#"
[dirs]
tickets = "my/tickets"
stories = "my/stories"
work = "my/work"

[scheduling]
max_threads = 4
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.dirs.tickets, Some("my/tickets".to_string()));
        assert_eq!(config.dirs.stories, Some("my/stories".to_string()));
        assert_eq!(config.dirs.work, Some("my/work".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(4));
    }

    #[test]
    fn test_parse_empty_config() {
        let config: LisaConfig = toml::from_str("").unwrap();
        assert_eq!(config.dirs.tickets, None);
        assert_eq!(config.scheduling.max_threads, None);
    }

    #[test]
    fn test_parse_partial_config() {
        let toml_str = r#"
[scheduling]
max_threads = 8
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.dirs.tickets, None);
        assert_eq!(config.scheduling.max_threads, Some(8));
    }

    #[test]
    fn test_load_config_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let validation = load_config(dir.path()).unwrap();
        assert_eq!(validation.config.dirs.tickets, None);
        assert_eq!(validation.config.scheduling.max_threads, None);
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_load_config_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 5\n",
        )
        .unwrap();
        let validation = load_config(dir.path()).unwrap();
        assert_eq!(validation.config.scheduling.max_threads, Some(5));
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "not valid toml {{{").unwrap();
        let result = load_config(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_defaults() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.ticket_dir, "docs/active/tickets");
        assert_eq!(resolved.story_dir, "docs/active/stories");
        assert_eq!(resolved.work_dir, "docs/active/work");
        assert_eq!(resolved.max_threads, 2);
    }

    #[test]
    fn test_resolve_config_file_overrides_defaults() {
        let toml_str = r#"
[dirs]
tickets = "custom/tickets"

[scheduling]
max_threads = 6
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.ticket_dir, "custom/tickets");
        assert_eq!(resolved.story_dir, "docs/active/stories"); // default
        assert_eq!(resolved.max_threads, 6);
    }

    #[test]
    fn test_resolve_cli_overrides_config_file() {
        let toml_str = "[scheduling]\nmax_threads = 6\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, Some(3), None);
        assert_eq!(resolved.max_threads, 3); // CLI wins
    }

    #[test]
    fn test_resolve_cli_overrides_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, Some(10), None);
        assert_eq!(resolved.max_threads, 10);
    }

    #[test]
    fn test_default_config_toml_parses() {
        let content = default_config_toml();
        let config: LisaConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.dirs.tickets, Some("docs/active/tickets".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(2));
        assert_eq!(
            resolve_config(&config, None, None).zellij_runtime,
            crate::runtime::default_runtime_request()
        );
    }

    #[test]
    fn test_resolve_zellij_runtime_modes_and_precedence() {
        let default = resolve_config(&LisaConfig::default(), None, None);
        assert_eq!(
            default.zellij_runtime,
            crate::runtime::default_runtime_request()
        );

        let managed: LisaConfig = toml::from_str("[runtime]\nzellij = \"managed\"\n").unwrap();
        assert_eq!(
            resolve_config(&managed, None, None).zellij_runtime,
            ZellijRuntimeRequest::Managed
        );

        let system: LisaConfig = toml::from_str("[runtime]\nzellij = \"system\"\n").unwrap();
        assert_eq!(
            resolve_config(&system, None, None).zellij_runtime,
            ZellijRuntimeRequest::System
        );

        let pinned: LisaConfig =
            toml::from_str("[runtime]\nzellij = \"/opt/lisa/zellij\"\n").unwrap();
        assert_eq!(
            resolve_config(&pinned, None, None).zellij_runtime,
            ZellijRuntimeRequest::Pinned("/opt/lisa/zellij".into())
        );
    }

    #[test]
    fn test_completion_guard_defaults_to_auto_and_resolves_all_valid_values() {
        assert_eq!(
            resolve_config(&LisaConfig::default(), None, None).completion_mode,
            CompletionSealMode::Auto
        );

        for (raw, expected) in [
            ("auto", CompletionSealMode::Auto),
            ("commit", CompletionSealMode::Commit),
            ("journal", CompletionSealMode::Journal),
        ] {
            let validation = validate_config(&format!("[guards]\ncompletion = {raw:?}\n")).unwrap();
            assert!(validation.warnings.is_empty());
            assert_eq!(
                resolve_config(&validation.config, None, None).completion_mode,
                expected
            );
        }
    }

    #[test]
    fn test_completion_guard_unknown_value_is_actionable_validation_error() {
        let error = validate_config("[guards]\ncompletion = \"best-effort\"\n").unwrap_err();
        assert!(error.contains("[guards].completion"));
        assert!(error.contains("best-effort"));
        assert!(error.contains("auto, commit, journal"));
    }

    #[test]
    fn test_completion_guard_unknown_key_warns_and_default_template_is_inert() {
        let validation = validate_config("[guards]\nseal = \"commit\"\n").unwrap();
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("[guards]"));
        assert!(validation.warnings[0].contains("seal"));

        let generated = default_config_toml();
        let config: LisaConfig = toml::from_str(&generated).unwrap();
        assert_eq!(config.guards.completion, None);
        assert_eq!(
            resolve_config(&config, None, None).completion_mode,
            CompletionSealMode::Auto
        );
    }

    #[test]
    fn test_agent_model_resolves_as_written_or_stays_absent() {
        assert_eq!(
            resolve_config(&LisaConfig::default(), None, None).model,
            None
        );

        let named =
            validate_config("[agent]\nclient = \"codex\"\nmodel = \"gpt-5-mini\"\n").unwrap();
        assert!(named.warnings.is_empty());
        assert_eq!(
            resolve_config(&named.config, None, None).model.as_deref(),
            Some("gpt-5-mini")
        );

        // Lisa never interprets the name, so an unfamiliar one is passed
        // through rather than second-guessed.
        let unfamiliar = validate_config("[agent]\nmodel = \"some-future-model\"\n").unwrap();
        assert_eq!(
            resolve_config(&unfamiliar.config, None, None)
                .model
                .as_deref(),
            Some("some-future-model")
        );
    }

    #[test]
    fn test_agent_model_must_name_something() {
        let error = validate_config("[agent]\nmodel = \"  \"\n").unwrap_err();
        assert!(error.contains("[agent].model"), "{error}");
    }

    #[test]
    fn test_agent_effort_resolves_as_written_or_stays_absent() {
        assert_eq!(
            resolve_config(&LisaConfig::default(), None, None).effort,
            None
        );

        let named = validate_config("[agent]\nmodel = \"opus\"\neffort = \"high\"\n").unwrap();
        assert!(named.warnings.is_empty());
        assert_eq!(
            resolve_config(&named.config, None, None).effort.as_deref(),
            Some("high")
        );
    }

    #[test]
    fn test_agent_effort_rejects_unknown_levels_naming_the_key() {
        let error = validate_config("[agent]\neffort = \"extreme\"\n").unwrap_err();
        assert!(error.contains("[agent].effort"), "{error}");
        assert!(error.contains("extreme"), "{error}");
        assert!(error.contains("low"), "{error}");
        assert!(error.contains("max"), "{error}");
    }

    #[test]
    fn test_agent_effort_accepts_every_valid_level() {
        for level in lisa_core::effort::Effort::VALID {
            let validated = validate_config(&format!("[agent]\neffort = \"{level}\"\n")).unwrap();
            assert!(validated.warnings.is_empty());
            assert_eq!(
                resolve_config(&validated.config, None, None)
                    .effort
                    .as_deref(),
                Some(*level)
            );
        }
    }

    #[test]
    fn test_validate_runtime_unknown_keys_warn_without_failing() {
        let result =
            validate_config("[runtime]\nzellij = \"managed\"\nchannel = \"beta\"\n").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("channel"));
        assert!(result.warnings[0].contains("[runtime]"));
    }

    #[test]
    fn test_validate_runtime_rejects_relative_pin() {
        let error = validate_config("[runtime]\nzellij = \"bin/zellij\"\n").unwrap_err();
        assert!(error.contains("[runtime].zellij"));
        assert!(error.contains("absolute path"));
        assert!(error.contains("bin/zellij"));
    }

    #[test]
    fn test_validate_unknown_top_level_key() {
        let result = validate_config("[unknown_section]\nfoo = 1\n").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("unknown_section"));
    }

    #[test]
    fn test_validate_unknown_dirs_key() {
        let result = validate_config("[dirs]\nfoo = \"bar\"\n").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("foo"));
        assert!(result.warnings[0].contains("[dirs]"));
    }

    #[test]
    fn test_validate_unknown_scheduling_key() {
        let result = validate_config("[scheduling]\nmax_thread = 4\n").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("max_thread"));
        assert!(result.warnings[0].contains("[scheduling]"));
    }

    #[test]
    fn test_validate_max_threads_zero() {
        let result = validate_config("[scheduling]\nmax_threads = 0\n");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("max_threads must be at least 1"));
    }

    #[test]
    fn test_validate_negative_max_threads() {
        let result = validate_config("[scheduling]\nmax_threads = -1\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_config_no_warnings() {
        let toml_str = r#"
[dirs]
tickets = "my/tickets"

[scheduling]
max_threads = 4
"#;
        let result = validate_config(toml_str).unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.scheduling.max_threads, Some(4));
    }

    /// The fixture the ticket asks for: comments interleaved with keys, a
    /// custom value, an unrelated section, and a key Lisa has never heard of.
    const INTERLEAVED_CONFIG: &str = r#"# Set up by hand in 2025. Do not reformat.
version = "0.4.4"

[dirs]
tickets = "planning/tickets"   # we keep the board out of docs/

# Two agents is all this laptop can take.
[scheduling]
max_threads = 2
# Left over from 0.4 — nobody remembers turning it on.
auto_advance = true
review_timeout_secs = 900

[experimental]
# Not a key Lisa knows. It is still the operator's file.
my_own_setting = "keep me"
"#;

    #[test]
    fn removing_the_retired_key_keeps_every_other_byte() {
        let RetiredKeyRemoval::Removed(updated) = remove_retired_scheduling_key(INTERLEAVED_CONFIG)
        else {
            panic!("a plain assignment must lift out cleanly");
        };

        // The one line is gone and nothing else moved: comments, custom values,
        // key order, the unknown section, and the trailing newline.
        assert_eq!(
            updated,
            INTERLEAVED_CONFIG.replace("auto_advance = true\n", "")
        );
        assert!(!updated.contains("auto_advance"));
        assert!(updated.contains("# Left over from 0.4 — nobody remembers turning it on."));
        assert!(updated.contains("# Set up by hand in 2025. Do not reformat."));
        assert!(
            updated.contains("tickets = \"planning/tickets\"   # we keep the board out of docs/")
        );
        assert!(updated.contains("my_own_setting = \"keep me\""));

        // And every surviving key still parses to exactly what it did before.
        let before: toml::Value = INTERLEAVED_CONFIG.parse().unwrap();
        let after: toml::Value = updated.parse().unwrap();
        assert_eq!(
            after["scheduling"]["max_threads"],
            before["scheduling"]["max_threads"]
        );
        assert_eq!(
            after["scheduling"]["review_timeout_secs"],
            before["scheduling"]["review_timeout_secs"]
        );
        assert_eq!(after["dirs"], before["dirs"]);
        assert_eq!(after["experimental"], before["experimental"]);
        assert_eq!(after["version"], before["version"]);

        // Idempotent: a second pass finds nothing to do.
        assert_eq!(
            remove_retired_scheduling_key(&updated),
            RetiredKeyRemoval::Absent
        );
    }

    #[test]
    fn a_file_that_does_not_set_the_key_is_left_alone() {
        for untouched in [
            "",
            "[scheduling]\nmax_threads = 2\n",
            // A commented assignment is somebody's note, not a live setting.
            "[scheduling]\n# auto_advance = true\nmax_threads = 2\n",
            // A key of the same name in another section is another key.
            "[triage]\nauto_advance = true\n",
            // Not parseable at all: doctor's config check owns that, not this.
            "[scheduling\nmax_threads =\n",
        ] {
            assert_eq!(
                remove_retired_scheduling_key(untouched),
                RetiredKeyRemoval::Absent,
                "must be Absent: {untouched:?}"
            );
        }
    }

    #[test]
    fn a_shape_no_line_deletion_can_express_is_refused() {
        // An inline table: the key has no line of its own, so deleting a line
        // cannot express the removal and rewriting one would reformat the
        // operator's file.
        let inline = "version = \"0.4.4\"\nscheduling = { auto_advance = true, max_threads = 2 }\n";
        assert_eq!(
            remove_retired_scheduling_key(inline),
            RetiredKeyRemoval::NotSurgical("the setting is not written as a line of its own")
        );

        // A quoted key is a shape the line scanner deliberately does not claim.
        let quoted = "[scheduling]\n\"auto_advance\" = true\nmax_threads = 2\n";
        assert_eq!(
            remove_retired_scheduling_key(quoted),
            RetiredKeyRemoval::NotSurgical("the setting is not written as a line of its own")
        );

        // CRLF: removing one line must not rewrite every other line's ending.
        let crlf = "[scheduling]\r\nauto_advance = true\r\nmax_threads = 2\r\n";
        assert!(matches!(
            remove_retired_scheduling_key(crlf),
            RetiredKeyRemoval::NotSurgical(_)
        ));
    }

    #[test]
    fn a_dotted_assignment_takes_its_empty_table_with_it() {
        // The only assignment in the file. Deleting the line removes the whole
        // `scheduling` table, which the parse-equivalence check must accept
        // rather than read as "another setting changed".
        let dotted = "version = \"0.4.4\"\nscheduling.auto_advance = true\n";
        let RetiredKeyRemoval::Removed(updated) = remove_retired_scheduling_key(dotted) else {
            panic!("a dotted assignment on its own line lifts out too");
        };
        assert_eq!(updated, "version = \"0.4.4\"\n");
    }

    #[test]
    fn a_reopened_scheduling_section_still_finds_its_key() {
        // `[scheduling.phase_timeouts]` is a different table; the key after it
        // belongs to whatever header follows, not to `[scheduling]`.
        let nested =
            "[scheduling]\nmax_threads = 2\n\n[scheduling.phase_timeouts]\nimplement = 60\n";
        assert_eq!(
            remove_retired_scheduling_key(nested),
            RetiredKeyRemoval::Absent
        );

        let nested_then_key = "[scheduling.phase_timeouts]\nimplement = 60\n\n[scheduling]\nauto_advance = true\nmax_threads = 2\n";
        let RetiredKeyRemoval::Removed(updated) = remove_retired_scheduling_key(nested_then_key)
        else {
            panic!("the key under the real [scheduling] header lifts out");
        };
        assert_eq!(
            updated,
            "[scheduling.phase_timeouts]\nimplement = 60\n\n[scheduling]\nmax_threads = 2\n"
        );
    }

    #[test]
    fn retired_auto_advance_key_loads_and_is_ignored() {
        // A board written against 0.4 still sets `auto_advance`. Removing the
        // key is not a reason to refuse the project: the file loads, the value
        // does nothing, and the operator is told once that it does nothing.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 4\nauto_advance = true\n",
        )
        .unwrap();

        let validation = load_config(dir.path()).expect("a retired key must not fail the load");
        assert_eq!(validation.config.scheduling.max_threads, Some(4));
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("auto_advance"));
        assert!(validation.warnings[0].contains("[scheduling]"));

        // Every other scheduling value still resolves to its default.
        let resolved = resolve_config(&validation.config, None, None);
        assert_eq!(resolved.max_threads, 4);
        assert_eq!(
            resolved.review_timeout_secs,
            PluginConfig::DEFAULT_REVIEW_TIMEOUT_SECS
        );
        assert_eq!(
            resolved.session_timeout_secs,
            PluginConfig::DEFAULT_SESSION_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_validate_multiple_warnings() {
        let toml_str = r#"
[dirs]
tickets = "t"
bad_key = "x"

[scheduling]
max_thread = 4

[extra]
foo = 1
"#;
        let result = validate_config(toml_str).unwrap();
        assert_eq!(result.warnings.len(), 3);
    }

    #[test]
    fn test_load_config_with_warnings() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_thread = 4\n",
        )
        .unwrap();
        let validation = load_config(dir.path()).unwrap();
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("max_thread"));
    }

    #[test]
    fn test_parse_review_timeout_secs() {
        let toml_str = "[scheduling]\nreview_timeout_secs = 120\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scheduling.review_timeout_secs, Some(120));
    }

    #[test]
    fn test_resolve_review_timeout_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.review_timeout_secs, 600);
    }

    #[test]
    fn test_resolve_review_timeout_from_config() {
        let toml_str = "[scheduling]\nreview_timeout_secs = 60\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.review_timeout_secs, 60);
    }

    #[test]
    fn test_validate_review_timeout_known_key() {
        let result = validate_config("[scheduling]\nreview_timeout_secs = 300\n").unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.scheduling.review_timeout_secs, Some(300));
    }

    #[test]
    fn test_assignment_ack_timeout_config_contract() {
        let config: LisaConfig =
            toml::from_str("[scheduling]\nassignment_ack_timeout_secs = 7\n").unwrap();
        assert_eq!(config.scheduling.assignment_ack_timeout_secs, Some(7));
        assert_eq!(
            resolve_config(&config, None, None).assignment_ack_timeout_secs,
            7
        );

        let defaults = resolve_config(&LisaConfig::default(), None, None);
        assert_eq!(defaults.assignment_ack_timeout_secs, 30);

        let validated = validate_config("[scheduling]\nassignment_ack_timeout_secs = 7\n").unwrap();
        assert!(validated.warnings.is_empty());
        assert!(
            validate_config("[scheduling]\nassignment_ack_timeout_secs = 0\n")
                .unwrap_err()
                .contains("assignment_ack_timeout_secs must be at least 1")
        );
        assert!(default_config_toml().contains("# assignment_ack_timeout_secs = 30"));
    }

    #[test]
    fn test_parse_session_timeout_secs() {
        let toml_str = "[scheduling]\nsession_timeout_secs = 3600\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scheduling.session_timeout_secs, Some(3600));
    }

    #[test]
    fn test_resolve_session_timeout_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.session_timeout_secs, 3600);
    }

    #[test]
    fn test_resolve_session_timeout_from_config() {
        let toml_str = "[scheduling]\nsession_timeout_secs = 900\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.session_timeout_secs, 900);
    }

    #[test]
    fn test_validate_session_timeout_known_key() {
        let result = validate_config("[scheduling]\nsession_timeout_secs = 1800\n").unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.scheduling.session_timeout_secs, Some(1800));
    }

    #[test]
    fn test_validate_version_is_known_key() {
        let result = validate_config("version = \"0.2.1\"\n").unwrap();
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_default_config_toml_has_version() {
        let content = default_config_toml();
        assert!(content.contains(&format!("version = \"{}\"", LISA_VERSION)));
    }

    #[test]
    fn test_parse_config_with_version() {
        let toml_str = "version = \"0.2.1\"\n\n[scheduling]\nmax_threads = 4\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.version, Some("0.2.1".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(4));
        assert_eq!(resolve_config(&config, None, None).project_version, "0.2.1");
    }

    #[test]
    fn version_staleness_compares_release_candidate_generations() {
        assert!(version_is_stale("0.4.0-rc.5", "0.4.0-rc.8"));
        assert!(version_is_stale("0.4.0-rc.7", "0.4.0-rc.8"));
        assert!(!version_is_stale("0.4.0-rc.8", "0.4.0-rc.8"));
        assert!(!version_is_stale("0.4.0-rc.9", "0.4.0-rc.8"));
    }

    #[test]
    fn version_staleness_obeys_semver_prerelease_ordering() {
        assert!(version_is_stale("0.4.0-rc.8", "0.4.0"));
        assert!(!version_is_stale("0.4.0", "0.4.0-rc.8"));
        assert!(version_is_stale("0.4.0-alpha.10", "0.4.0-beta.1"));
        assert!(version_is_stale("0.4.0-rc.8", "0.4.1-alpha.1"));
        assert!(!version_is_stale("0.5.0-alpha.1", "0.4.9"));
        assert!(version_is_stale("not-semver", "0.4.0-rc.8"));
    }

    /// The comparison the 0.5.0 upgrade path hangs off.
    ///
    /// A project written by 0.4.4 has to read as stale against `0.5.0-rc.1`,
    /// because that is the signal `lisa doctor`, `lisa init`, and `lisa loop`
    /// all key their upgrade behaviour on. A prerelease suffix is exactly the
    /// kind of string a naive compare gets backwards — `0.5.0-rc.1` sorts
    /// *before* `0.5.0` and *after* every `0.4.x` — so both directions are
    /// asserted with literals rather than reasoned about.
    #[test]
    fn a_0_4_4_project_is_stale_against_the_0_5_0_release_candidate() {
        assert!(version_is_stale("0.4.4", "0.5.0-rc.1"));
        assert!(!version_is_stale("0.5.0-rc.1", "0.5.0-rc.1"));

        // The comparison must not invert: an rc.1 project read by an older
        // 0.4.4 binary is ahead, not behind.
        assert!(!version_is_stale("0.5.0-rc.1", "0.4.4"));

        // And the rc still yields to the stable it precedes, and to its own
        // later generations.
        assert!(version_is_stale("0.5.0-rc.1", "0.5.0"));
        assert!(version_is_stale("0.5.0-rc.1", "0.5.0-rc.2"));

        // Tied to the binary actually shipping, so the assertion keeps meaning
        // something after the next bump.
        assert!(version_is_stale("0.4.4", LISA_VERSION));
        assert!(!version_is_stale(LISA_VERSION, LISA_VERSION));
    }

    #[test]
    fn test_parse_phase_timeouts() {
        let toml_str = r#"
[scheduling]
session_timeout_secs = 900

[scheduling.phase_timeouts]
research = 300
implement = 1800
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let pt = config.scheduling.phase_timeouts.unwrap();
        assert_eq!(pt.get("research"), Some(&300));
        assert_eq!(pt.get("implement"), Some(&1800));
        assert_eq!(pt.len(), 2);
    }

    #[test]
    fn test_resolve_phase_timeouts() {
        let toml_str = r#"
[scheduling.phase_timeouts]
research = 300
implement = 1800
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.phase_timeouts.get("research"), Some(&300));
        assert_eq!(resolved.phase_timeouts.get("implement"), Some(&1800));
    }

    #[test]
    fn test_resolve_phase_timeouts_empty_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None, None);
        assert!(resolved.phase_timeouts.is_empty());
    }

    #[test]
    fn test_validate_phase_timeouts_known_key() {
        let toml_str = r#"
[scheduling]
session_timeout_secs = 900

[scheduling.phase_timeouts]
research = 300
implement = 1800
"#;
        let result = validate_config(toml_str).unwrap();
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_phase_timeouts_unknown_phase() {
        let toml_str = r#"
[scheduling.phase_timeouts]
research = 300
compile = 1800
"#;
        let result = validate_config(toml_str).unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("compile"));
        assert!(result.warnings[0].contains("[scheduling.phase_timeouts]"));
    }

    #[test]
    fn test_parse_agent_client() {
        let toml_str = "[agent]\nclient = \"codex\"\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.agent.client, Some("codex".to_string()));
    }

    #[test]
    fn test_resolve_client_from_detected_availability() {
        let config = LisaConfig::default();
        for (availability, client, announcement) in [
            (
                AgentAvailability::Neither,
                AgentClient::Claude,
                "Driving Claude — neither agent is installed; claude is the default.",
            ),
            (
                AgentAvailability::ClaudeOnly,
                AgentClient::Claude,
                "Driving Claude — it's the agent installed here.",
            ),
            (
                AgentAvailability::CodexOnly,
                AgentClient::Codex,
                "Driving Codex — it's the agent installed here.",
            ),
            (
                AgentAvailability::Both,
                AgentClient::Claude,
                "Driving Claude — both agents are installed; claude is the default.",
            ),
        ] {
            let resolved = resolve_config_with_availability(&config, None, None, availability);
            assert_eq!(resolved.client, client);
            assert_eq!(resolved.client_announcement(), announcement);
        }
    }

    #[test]
    fn test_resolve_client_from_config() {
        let toml_str = "[agent]\nclient = \"codex\"\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved =
            resolve_config_with_availability(&config, None, None, AgentAvailability::ClaudeOnly);
        assert_eq!(resolved.client, AgentClient::Codex);
        assert_eq!(
            resolved.client_announcement(),
            "Driving Codex — selected in .lisa.toml."
        );
    }

    #[test]
    fn test_resolve_cli_client_overrides_config() {
        let toml_str = "[agent]\nclient = \"codex\"\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config_with_availability(
            &config,
            None,
            Some(AgentClient::Claude),
            AgentAvailability::CodexOnly,
        );
        assert_eq!(resolved.client, AgentClient::Claude); // CLI wins
        assert_eq!(
            resolved.client_announcement(),
            "Driving Claude — selected by --client."
        );
    }

    #[test]
    fn test_validate_invalid_client_is_error() {
        let result = validate_config("[agent]\nclient = \"gpt\"\n");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("gpt"));
        assert!(err.contains("claude") && err.contains("codex"));
    }

    #[test]
    fn test_validate_valid_client_no_warning() {
        let result = validate_config("[agent]\nclient = \"codex\"\n").unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.agent.client, Some("codex".to_string()));
    }

    #[test]
    fn test_validate_unknown_agent_key() {
        let result = validate_config("[agent]\nprovider = \"x\"\n").unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("provider"));
        assert!(result.warnings[0].contains("[agent]"));
    }

    #[test]
    fn test_default_config_toml_agent_example_is_inert() {
        // The [agent] example ships commented, so a fresh config has no
        // persistent client selection and still follows PATH detection.
        let content = default_config_toml();
        let config: LisaConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.agent.client, None);
        assert!(content.contains("Omit it to detect agents on PATH"));
        let resolved =
            resolve_config_with_availability(&config, None, None, AgentAvailability::CodexOnly);
        assert_eq!(resolved.client, AgentClient::Codex);
    }

    #[test]
    fn test_parse_provider_caps() {
        let toml_str = r#"
[scheduling.provider_caps]
claude = 8
codex = 4
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let caps = config.scheduling.provider_caps.unwrap();
        assert_eq!(caps.get("claude"), Some(&8));
        assert_eq!(caps.get("codex"), Some(&4));
    }

    #[test]
    fn test_resolve_provider_caps() {
        let toml_str = r#"
[scheduling.provider_caps]
codex = 6
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None, None);
        assert_eq!(resolved.provider_caps.get("codex"), Some(&6));
    }

    #[test]
    fn test_resolve_provider_caps_empty_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None, None);
        assert!(resolved.provider_caps.is_empty());
    }

    #[test]
    fn test_validate_provider_caps_known_no_warning() {
        let toml_str = "[scheduling.provider_caps]\nclaude = 8\ncodex = 4\n";
        let result = validate_config(toml_str).unwrap();
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_provider_caps_unknown_provider_warns() {
        let toml_str = "[scheduling.provider_caps]\ngpt = 8\n";
        let result = validate_config(toml_str).unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("gpt"));
        assert!(result.warnings[0].contains("[scheduling.provider_caps]"));
    }

    #[test]
    fn test_validate_provider_cap_zero_errors() {
        let result = validate_config("[scheduling.provider_caps]\ncodex = 0\n");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("codex"));
        assert!(err.contains("at least 1"));
    }

    #[test]
    fn test_default_config_toml_provider_caps_inert() {
        // The provider_caps example ships commented, so a fresh config resolves
        // to no caps (no accidental opt-in).
        let content = default_config_toml();
        let config: LisaConfig = toml::from_str(&content).unwrap();
        assert!(config.scheduling.provider_caps.is_none());
        let resolved = resolve_config(&config, None, None);
        assert!(resolved.provider_caps.is_empty());
    }

    #[test]
    fn test_parse_partial_phase_timeouts() {
        let toml_str = r#"
[scheduling.phase_timeouts]
implement = 1800
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let pt = config.scheduling.phase_timeouts.unwrap();
        assert_eq!(pt.len(), 1);
        assert_eq!(pt.get("implement"), Some(&1800));
    }

    #[test]
    fn triage_config_defaults_resolves_and_validates_bounds() {
        let defaults = resolve_config(&LisaConfig::default(), None, None);
        assert!(defaults.triage_enabled);
        assert_eq!(defaults.triage_timeout_secs, 120);

        let validation = validate_config("[triage]\nenabled = false\ntimeout_secs = 9\n").unwrap();
        assert!(validation.warnings.is_empty());
        let resolved = resolve_config(&validation.config, None, None);
        assert!(!resolved.triage_enabled);
        assert_eq!(resolved.triage_timeout_secs, 9);

        assert!(validate_config("[triage]\ntimeout_secs = 0\n")
            .unwrap_err()
            .contains("at least 1"));
        let warning = validate_config("[triage]\nmystery = true\n").unwrap();
        assert!(warning.warnings[0].contains("[triage]"));
    }
}
