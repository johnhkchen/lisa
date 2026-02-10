use std::path::Path;

use serde::Deserialize;

use lisa_core::types::PluginConfig;

/// Top-level .lisa.toml structure.
#[derive(Debug, Default, Deserialize)]
pub struct LisaConfig {
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
}

/// Fully resolved configuration with all defaults applied.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub ticket_dir: String,
    pub story_dir: String,
    pub work_dir: String,
    pub max_threads: usize,
    pub auto_advance: bool,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            ticket_dir: PluginConfig::DEFAULT_TICKET_DIR.to_string(),
            story_dir: PluginConfig::DEFAULT_STORY_DIR.to_string(),
            work_dir: PluginConfig::DEFAULT_WORK_DIR.to_string(),
            max_threads: PluginConfig::DEFAULT_MAX_THREADS,
            auto_advance: false,
        }
    }
}

/// Load .lisa.toml from the project root.
///
/// Returns `Ok(LisaConfig::default())` if the file does not exist.
/// Returns `Err` if the file exists but cannot be parsed.
pub fn load_config(root: &Path) -> Result<LisaConfig, String> {
    let config_path = root.join(".lisa.toml");
    if !config_path.exists() {
        return Ok(LisaConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", config_path.display(), e))
}

/// Merge config file values with CLI overrides.
///
/// Precedence (lowest to highest): defaults < .lisa.toml < CLI flags.
pub fn resolve_config(config: &LisaConfig, cli_max_threads: Option<usize>) -> ResolvedConfig {
    let defaults = ResolvedConfig::default();

    ResolvedConfig {
        ticket_dir: config
            .dirs
            .tickets
            .clone()
            .unwrap_or(defaults.ticket_dir),
        story_dir: config.dirs.stories.clone().unwrap_or(defaults.story_dir),
        work_dir: config.dirs.work.clone().unwrap_or(defaults.work_dir),
        max_threads: cli_max_threads
            .or(config.scheduling.max_threads)
            .unwrap_or(defaults.max_threads),
        auto_advance: config
            .scheduling
            .auto_advance
            .unwrap_or(defaults.auto_advance),
    }
}

/// Returns the default .lisa.toml content for `lisa init`.
pub fn default_config_toml() -> &'static str {
    r#"# Lisa project configuration

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
# auto_advance = false
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
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.dirs.tickets, None);
        assert_eq!(config.scheduling.max_threads, None);
    }

    #[test]
    fn test_load_config_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".lisa.toml"),
            "[scheduling]\nmax_threads = 5\n",
        )
        .unwrap();
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.scheduling.max_threads, Some(5));
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
        let config: LisaConfig = toml::from_str(content).unwrap();
        assert_eq!(config.dirs.tickets, Some("docs/active/tickets".to_string()));
        assert_eq!(config.scheduling.max_threads, Some(2));
    }
}
