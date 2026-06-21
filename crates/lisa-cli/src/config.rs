use std::path::Path;

use serde::Deserialize;

use lisa_core::types::PluginConfig;

/// Top-level .lisa.toml structure.
#[derive(Debug, Default, Deserialize)]
pub struct LisaConfig {
    pub version: Option<String>,
    #[serde(default)]
    pub dirs: DirsConfig,
    #[serde(default)]
    pub scheduling: SchedulingConfig,
}

/// Directory configuration section.
#[derive(Debug, Default, Deserialize)]
pub struct DirsConfig {
    pub tickets: Option<String>,
    pub stories: Option<String>,
    pub work: Option<String>,
}

/// Scheduling configuration section.
#[derive(Debug, Default, Deserialize)]
pub struct SchedulingConfig {
    pub max_threads: Option<usize>,
    pub auto_advance: Option<bool>,
    pub review_timeout_secs: Option<u64>,
    pub session_timeout_secs: Option<u64>,
    pub wind_down_secs: Option<u64>,
    pub phase_timeouts: Option<std::collections::HashMap<String, u64>>,
}

/// Fully resolved configuration with all defaults applied.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub ticket_dir: String,
    pub story_dir: String,
    pub work_dir: String,
    pub max_threads: usize,
    pub auto_advance: bool,
    pub review_timeout_secs: u64,
    pub session_timeout_secs: u64,
    pub wind_down_secs: u64,
    pub phase_timeouts: std::collections::HashMap<String, u64>,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            ticket_dir: PluginConfig::DEFAULT_TICKET_DIR.to_string(),
            story_dir: PluginConfig::DEFAULT_STORY_DIR.to_string(),
            work_dir: PluginConfig::DEFAULT_WORK_DIR.to_string(),
            max_threads: PluginConfig::DEFAULT_MAX_THREADS,
            auto_advance: false,
            review_timeout_secs: PluginConfig::DEFAULT_REVIEW_TIMEOUT_SECS,
            session_timeout_secs: PluginConfig::DEFAULT_SESSION_TIMEOUT_SECS,
            wind_down_secs: PluginConfig::DEFAULT_WIND_DOWN_SECS,
            phase_timeouts: std::collections::HashMap::new(),
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
/// Precedence (lowest to highest): defaults < .lisa.toml < CLI flags.
pub fn resolve_config(config: &LisaConfig, cli_max_threads: Option<usize>) -> ResolvedConfig {
    let defaults = ResolvedConfig::default();

    ResolvedConfig {
        ticket_dir: config.dirs.tickets.clone().unwrap_or(defaults.ticket_dir),
        story_dir: config.dirs.stories.clone().unwrap_or(defaults.story_dir),
        work_dir: config.dirs.work.clone().unwrap_or(defaults.work_dir),
        max_threads: cli_max_threads
            .or(config.scheduling.max_threads)
            .unwrap_or(defaults.max_threads),
        auto_advance: config
            .scheduling
            .auto_advance
            .unwrap_or(defaults.auto_advance),
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
        phase_timeouts: config.scheduling.phase_timeouts.clone().unwrap_or_default(),
    }
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
    let known_top = &["version", "dirs", "scheduling"];
    let known_dirs = &["tickets", "stories", "work"];
    let known_scheduling = &[
        "max_threads",
        "auto_advance",
        "review_timeout_secs",
        "session_timeout_secs",
        "wind_down_secs",
        "phase_timeouts",
    ];

    // Parse as generic Value to detect unknown keys
    let value: toml::Value = content
        .parse()
        .map_err(|e: toml::de::Error| format!("Invalid TOML: {}", e))?;

    let mut warnings = Vec::new();

    if let Some(table) = value.as_table() {
        for key in table.keys() {
            if !known_top.contains(&key.as_str()) {
                warnings.push(format!("Unknown config section: [{}]", key));
            }
        }

        if let Some(toml::Value::Table(dirs)) = table.get("dirs") {
            for key in dirs.keys() {
                if !known_dirs.contains(&key.as_str()) {
                    warnings.push(format!("Unknown key in [dirs]: {}", key));
                }
            }
        }

        if let Some(toml::Value::Table(sched)) = table.get("scheduling") {
            for key in sched.keys() {
                if !known_scheduling.contains(&key.as_str()) {
                    warnings.push(format!("Unknown key in [scheduling]: {}", key));
                }
            }

            // Validate phase names in [scheduling.phase_timeouts]
            let known_phases = &[
                "research",
                "design",
                "structure",
                "plan",
                "implement",
                "review",
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
        }
    }

    // Deserialize into typed struct
    let config: LisaConfig =
        toml::from_str(content).map_err(|e| format!("Invalid config value: {}", e))?;

    // Semantic validation
    if config.scheduling.max_threads == Some(0) {
        return Err("max_threads must be at least 1".to_string());
    }

    Ok(ConfigValidation { config, warnings })
}

/// The current Lisa CLI version, used for project version tracking.
pub const LISA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns true if `project_version` is older than `current_version`.
/// Uses simple tuple comparison of (major, minor, patch) parsed from semver strings.
/// Returns true (needs update) if parsing fails.
pub fn version_is_stale(project_version: &str, current_version: &str) -> bool {
    fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
        // Strip any pre-release / build-metadata suffix (e.g. "0.3.0-rc.1" or
        // "0.3.0+build") so prerelease versions parse to their core triple.
        let core = s.split(['-', '+']).next().unwrap_or(s);
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    }
    match (parse_semver(project_version), parse_semver(current_version)) {
        (Some(proj), Some(curr)) => proj < curr,
        _ => true,
    }
}

/// Returns the default .lisa.toml content for `lisa init`.
pub fn default_config_toml() -> String {
    format!(
        r#"# Lisa project configuration
version = "{}"

[dirs]"#,
        LISA_VERSION,
    ) + r#"
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
# auto_advance = false
# review_timeout_secs = 600
# session_timeout_secs = 3600
# wind_down_secs = 300

# [scheduling.phase_timeouts]
# research = 300
# design = 300
# implement = 1800
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let toml_str = r#"
[dirs]
tickets = "my/tickets"
stories = "my/stories"
work = "my/work"

[scheduling]
max_threads = 4
auto_advance = true
"#;
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.dirs.tickets, Some("my/tickets".to_string()));
        assert_eq!(config.dirs.stories, Some("my/stories".to_string()));
        assert_eq!(config.dirs.work, Some("my/work".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(4));
        assert_eq!(config.scheduling.auto_advance, Some(true));
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
        assert_eq!(config.scheduling.auto_advance, None);
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
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.ticket_dir, "docs/active/tickets");
        assert_eq!(resolved.story_dir, "docs/active/stories");
        assert_eq!(resolved.work_dir, "docs/active/work");
        assert_eq!(resolved.max_threads, 2);
        assert!(!resolved.auto_advance);
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
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.ticket_dir, "custom/tickets");
        assert_eq!(resolved.story_dir, "docs/active/stories"); // default
        assert_eq!(resolved.max_threads, 6);
    }

    #[test]
    fn test_resolve_cli_overrides_config_file() {
        let toml_str = "[scheduling]\nmax_threads = 6\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, Some(3));
        assert_eq!(resolved.max_threads, 3); // CLI wins
    }

    #[test]
    fn test_resolve_cli_overrides_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, Some(10));
        assert_eq!(resolved.max_threads, 10);
    }

    #[test]
    fn test_default_config_toml_parses() {
        let content = default_config_toml();
        let config: LisaConfig = toml::from_str(&content).unwrap();
        assert_eq!(config.dirs.tickets, Some("docs/active/tickets".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(2));
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
auto_advance = true
"#;
        let result = validate_config(toml_str).unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.scheduling.max_threads, Some(4));
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
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.review_timeout_secs, 600);
    }

    #[test]
    fn test_resolve_review_timeout_from_config() {
        let toml_str = "[scheduling]\nreview_timeout_secs = 60\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.review_timeout_secs, 60);
    }

    #[test]
    fn test_validate_review_timeout_known_key() {
        let result = validate_config("[scheduling]\nreview_timeout_secs = 300\n").unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.config.scheduling.review_timeout_secs, Some(300));
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
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.session_timeout_secs, 3600);
    }

    #[test]
    fn test_resolve_session_timeout_from_config() {
        let toml_str = "[scheduling]\nsession_timeout_secs = 900\n";
        let config: LisaConfig = toml::from_str(toml_str).unwrap();
        let resolved = resolve_config(&config, None);
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
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.phase_timeouts.get("research"), Some(&300));
        assert_eq!(resolved.phase_timeouts.get("implement"), Some(&1800));
    }

    #[test]
    fn test_resolve_phase_timeouts_empty_default() {
        let config = LisaConfig::default();
        let resolved = resolve_config(&config, None);
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
}
