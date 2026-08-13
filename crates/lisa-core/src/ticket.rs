//! Ticket parsing and management module for the Lisa Zellij plugin.
//!
//! This module handles reading, parsing, and updating tickets stored as markdown
//! files with YAML frontmatter. Tickets are the unit of work in Lisa's workflow.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::types::{Phase, Priority, Ticket, TicketStatus, TicketType};

/// Errors that can occur during ticket operations.
#[derive(Debug)]
pub enum TicketError {
    /// File I/O error
    Io(io::Error),
    /// File does not contain valid YAML frontmatter
    MissingFrontmatter,
    /// YAML parsing error
    YamlParse(String),
    /// Required field is missing from frontmatter
    MissingField(String),
    /// Invalid field value
    InvalidField {
        field: String,
        value: String,
        reason: String,
    },
    /// File path is not valid UTF-8
    InvalidPath(PathBuf),
}

impl std::fmt::Display for TicketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketError::Io(err) => write!(f, "I/O error: {}", err),
            TicketError::MissingFrontmatter => {
                write!(
                    f,
                    "File does not contain valid YAML frontmatter (missing --- delimiters)"
                )
            }
            TicketError::YamlParse(msg) => write!(f, "YAML parse error: {}", msg),
            TicketError::MissingField(field) => {
                write!(f, "Required field '{}' is missing from frontmatter", field)
            }
            TicketError::InvalidField {
                field,
                value,
                reason,
            } => {
                write!(
                    f,
                    "Invalid value '{}' for field '{}': {}",
                    value, field, reason
                )
            }
            TicketError::InvalidPath(path) => {
                write!(f, "Path is not valid UTF-8: {:?}", path)
            }
        }
    }
}

impl std::error::Error for TicketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TicketError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for TicketError {
    fn from(err: io::Error) -> Self {
        TicketError::Io(err)
    }
}

/// Parses a ticket from a markdown file with YAML frontmatter.
///
/// The expected format is:
/// ```text
/// ---
/// id: T-024-03
/// story: S-024
/// title: migrate-climate-calls
/// type: task
/// status: open
/// priority: high
/// phase: research
/// depends_on: [T-024-01, T-024-02]
/// ---
///
/// ## Context
/// ...
/// ```
///
/// The `blocks` field is optional. Lisa computes reverse dependency edges
/// (blocked-by relationships) from `depends_on` alone. If `blocks` is present
/// it will be parsed, but it is not required.
///
/// # Arguments
///
/// * `path` - Path to the markdown ticket file
///
/// # Returns
///
/// A `Ticket` struct populated from the frontmatter and content.
///
/// # Errors
///
/// Returns `TicketError` if:
/// - The file cannot be read
/// - The file doesn't contain valid YAML frontmatter
/// - Required fields are missing
/// - Field values are invalid
pub fn parse_ticket<P: AsRef<Path>>(path: P) -> Result<Ticket, TicketError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    parse_ticket_content(&content, path)
}

/// Parses ticket content from a string (for testing or when content is already loaded).
fn parse_ticket_content(content: &str, path: &Path) -> Result<Ticket, TicketError> {
    let (frontmatter, body) = extract_frontmatter(content)?;
    parse_frontmatter_into_ticket(&frontmatter, body, path)
}

/// Extracts YAML frontmatter and body content from a markdown string.
///
/// Returns (frontmatter_yaml, body_content).
fn extract_frontmatter(content: &str) -> Result<(String, String), TicketError> {
    let content = content.trim_start();

    // Must start with ---
    if !content.starts_with("---") {
        return Err(TicketError::MissingFrontmatter);
    }

    // Find the closing ---
    let after_opening = &content[3..];
    let closing_pos = after_opening
        .find("\n---")
        .ok_or(TicketError::MissingFrontmatter)?;

    let frontmatter = after_opening[..closing_pos].trim().to_string();
    let body = after_opening[closing_pos + 4..].trim().to_string();

    Ok((frontmatter, body))
}

/// Parses YAML frontmatter into a Ticket struct.
fn parse_frontmatter_into_ticket(
    yaml: &str,
    content: String,
    path: &Path,
) -> Result<Ticket, TicketError> {
    let mut id: Option<String> = None;
    let mut story: Option<String> = None;
    let mut title: Option<String> = None;
    let mut ticket_type: Option<TicketType> = None;
    let mut status: Option<TicketStatus> = None;
    let mut priority: Option<Priority> = None;
    let mut phase: Option<Phase> = None;
    let mut depends_on: Vec<String> = Vec::new();
    let mut blocks: Vec<String> = Vec::new();
    let mut agent: Option<String> = None;
    let mut model: Option<String> = None;

    // Track which list field we're accumulating multiline items for.
    // YAML allows both inline `depends_on: [A, B]` and multiline:
    //   depends_on:
    //     - A
    //     - B
    let mut list_field: Option<&str> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            list_field = None;
            continue;
        }

        // Check for YAML list item (  - value)
        if let Some(item) = trimmed.strip_prefix("- ") {
            let item = item.trim();
            if !item.is_empty() {
                match list_field {
                    Some("depends_on") => depends_on.push(item.to_string()),
                    Some("blocks") => blocks.push(item.to_string()),
                    _ => {}
                }
            }
            continue;
        }

        // New key resets list accumulation
        list_field = None;

        if let Some((key, value)) = parse_yaml_line(trimmed) {
            match key {
                "id" => id = Some(value.to_string()),
                "story" => story = Some(value.to_string()),
                "title" => title = Some(value.to_string()),
                "type" => ticket_type = Some(parse_ticket_type(value)?),
                "status" => status = Some(parse_status(value)?),
                "priority" => priority = Some(parse_priority(value)?),
                "phase" => phase = Some(parse_phase(value)?),
                "depends_on" => {
                    let parsed = parse_string_vec(value)?;
                    if parsed.is_empty() && value.is_empty() {
                        // Empty value — expect multiline list items to follow
                        list_field = Some("depends_on");
                    } else {
                        depends_on = parsed;
                    }
                }
                "blocks" => {
                    let parsed = parse_string_vec(value)?;
                    if parsed.is_empty() && value.is_empty() {
                        list_field = Some("blocks");
                    } else {
                        blocks = parsed;
                    }
                }
                // Routing hints (T-026-01). Stored raw and unvalidated: an
                // invalid `agent` must not fail the ticket — it falls back to the
                // loop default at spawn (resolve_route), and the requested value
                // is preserved for the dashboard + provenance. An empty value
                // (e.g. a bare `agent:`) is treated as absent.
                "agent" => {
                    if !value.is_empty() {
                        agent = Some(value.to_string());
                    }
                }
                "model" if !value.is_empty() => {
                    model = Some(value.to_string());
                }
                _ => {
                    // Ignore unknown fields for forward compatibility
                }
            }
        }
    }

    // Validate required fields
    let id = id.ok_or_else(|| TicketError::MissingField("id".to_string()))?;
    let title = title.ok_or_else(|| TicketError::MissingField("title".to_string()))?;
    let ticket_type = ticket_type.ok_or_else(|| TicketError::MissingField("type".to_string()))?;
    let status = status.ok_or_else(|| TicketError::MissingField("status".to_string()))?;
    let priority = priority.ok_or_else(|| TicketError::MissingField("priority".to_string()))?;
    let phase = phase.ok_or_else(|| TicketError::MissingField("phase".to_string()))?;

    Ok(Ticket {
        id,
        story,
        title,
        ticket_type,
        status,
        priority,
        phase,
        depends_on,
        blocks,
        agent,
        model,
        content,
        file_path: path.to_path_buf(),
    })
}

/// Parses a simple YAML key: value line.
fn parse_yaml_line(line: &str) -> Option<(&str, &str)> {
    let colon_pos = line.find(':')?;
    let key = line[..colon_pos].trim();
    let value = line[colon_pos + 1..].trim();
    Some((key, value))
}

/// Parses a TicketType from a string.
fn parse_ticket_type(value: &str) -> Result<TicketType, TicketError> {
    match value.to_lowercase().as_str() {
        "task" => Ok(TicketType::Task),
        "bug" => Ok(TicketType::Bug),
        "feature" => Ok(TicketType::Feature),
        "spike" => Ok(TicketType::Spike),
        "chore" => Ok(TicketType::Chore),
        _ => Err(TicketError::InvalidField {
            field: "type".to_string(),
            value: value.to_string(),
            reason: "expected one of: task, bug, feature, spike, chore".to_string(),
        }),
    }
}

/// Parses a TicketStatus from a string.
fn parse_status(value: &str) -> Result<TicketStatus, TicketError> {
    match value.to_lowercase().as_str() {
        "open" => Ok(TicketStatus::Open),
        "in_progress" | "in-progress" => Ok(TicketStatus::InProgress),
        "blocked" => Ok(TicketStatus::Blocked),
        "review" => Ok(TicketStatus::Review),
        "done" => Ok(TicketStatus::Done),
        "cancelled" | "canceled" => Ok(TicketStatus::Cancelled),
        _ => Err(TicketError::InvalidField {
            field: "status".to_string(),
            value: value.to_string(),
            reason: "expected one of: open, in_progress, blocked, review, done, cancelled"
                .to_string(),
        }),
    }
}

/// Parses a Priority from a string.
fn parse_priority(value: &str) -> Result<Priority, TicketError> {
    match value.to_lowercase().as_str() {
        "low" => Ok(Priority::Low),
        "medium" => Ok(Priority::Medium),
        "high" => Ok(Priority::High),
        "critical" => Ok(Priority::Critical),
        _ => Err(TicketError::InvalidField {
            field: "priority".to_string(),
            value: value.to_string(),
            reason: "expected one of: low, medium, high, critical".to_string(),
        }),
    }
}

/// Parses a Phase from a string.
///
/// Case-insensitive, then delegated to [`Phase::from_name`] — the single
/// phase-name table — so frontmatter parsing and the plugin's config-key
/// parsing can never accept different vocabularies. The retired 0.4 names
/// (`research`, `design`, `structure`, `plan`) parse to `Implement` there; they
/// are accepted but no longer advertised, because they are no longer written.
fn parse_phase(value: &str) -> Result<Phase, TicketError> {
    Phase::from_name(&value.to_lowercase()).ok_or_else(|| TicketError::InvalidField {
        field: "phase".to_string(),
        value: value.to_string(),
        reason: "expected one of: ready, implement, review, done".to_string(),
    })
}

/// Parses a YAML array of strings like `[T-024-01, T-024-02]` into a Vec.
fn parse_string_vec(value: &str) -> Result<Vec<String>, TicketError> {
    let value = value.trim();

    // Handle empty array
    if value == "[]" || value.is_empty() {
        return Ok(Vec::new());
    }

    // Remove brackets
    let inner = if value.starts_with('[') && value.ends_with(']') {
        &value[1..value.len() - 1]
    } else {
        value
    };

    // Split by comma and collect
    let items: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(items)
}

/// Scans a directory for all ticket files (*.md).
///
/// # Arguments
///
/// * `dir` - Path to the directory to scan
///
/// # Returns
///
/// A vector of parsed `Ticket` structs for all valid ticket files.
/// Invalid tickets are logged and skipped.
///
/// # Errors
///
/// Returns `TicketError::Io` if the directory cannot be read.
pub fn scan_tickets<P: AsRef<Path>>(dir: P) -> Result<Vec<Ticket>, TicketError> {
    let dir = dir.as_ref();
    let mut tickets = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        // Skip directories
        if path.is_dir() {
            continue;
        }

        match parse_ticket(&path) {
            Ok(ticket) => tickets.push(ticket),
            Err(e) => {
                // Log the error but continue processing other tickets
                eprintln!("Warning: Failed to parse ticket {:?}: {}", path, e);
            }
        }
    }

    Ok(tickets)
}

/// Result of scanning a directory for tickets, preserving per-file errors.
#[derive(Debug)]
pub struct ScanResult {
    /// Successfully parsed tickets.
    pub tickets: Vec<Ticket>,
    /// Per-file parse errors (file path, error).
    pub errors: Vec<(PathBuf, TicketError)>,
}

/// Scans a directory for ticket files, returning both parsed tickets and per-file errors.
///
/// Unlike `scan_tickets`, this function surfaces individual parse errors so callers
/// can log them as diagnostic events rather than silently skipping them.
///
/// # Errors
///
/// Returns `TicketError::Io` if the directory itself cannot be read.
pub fn scan_tickets_with_diagnostics<P: AsRef<Path>>(dir: P) -> Result<ScanResult, TicketError> {
    let dir = dir.as_ref();
    let mut tickets = Vec::new();
    let mut errors = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if path.is_dir() {
            continue;
        }

        match parse_ticket(&path) {
            Ok(ticket) => tickets.push(ticket),
            Err(e) => errors.push((path, e)),
        }
    }

    Ok(ScanResult { tickets, errors })
}

/// What the ticket directory looks like right now, without reading a ticket.
///
/// Parsing every ticket answers "what is on the board". This answers the much
/// cheaper question a scheduler with nothing to do actually needs: *has
/// anything changed since I last looked?* It is a `read_dir` plus one
/// `metadata` per `.md` file — no file contents are read and no ticket is
/// parsed — so it stays affordable at a poll cadence on a board that may sit
/// finished for hours.
///
/// The value is a hash of every `.md` entry's name, length, and modification
/// time, summed so the arbitrary order `read_dir` returns them in cannot change
/// it. It is only ever compared with another fingerprint taken by the same
/// process, so the hasher's per-process seed does not matter and the number is
/// never written down.
///
/// `None` means the directory could not be described — it is missing, or a
/// file's metadata or modification time is unavailable on this platform. A
/// caller must read that as *changed*: failing toward doing the work costs a
/// rescan, and failing the other way would fence a scheduler off from tickets
/// it can plainly see.
pub fn scan_fingerprint<P: AsRef<Path>>(dir: P) -> Option<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut total: u64 = 0;
    let mut files: u64 = 0;

    for entry in fs::read_dir(dir.as_ref()).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let metadata = entry.metadata().ok()?;
        if metadata.is_dir() {
            continue;
        }

        let mut hasher = DefaultHasher::new();
        entry.file_name().hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        // A modification time from before the epoch is not evidence of
        // anything, so it hashes as zero rather than discarding the whole
        // fingerprint; a modification time this platform cannot report at all
        // does discard it, because then nothing here can see an edit.
        metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
            .hash(&mut hasher);

        total = total.wrapping_add(hasher.finish());
        files = files.wrapping_add(1);
    }

    Some(total ^ files.rotate_left(32))
}

/// Recursively scans a directory tree for all ticket files (*.md).
///
/// # Arguments
///
/// * `dir` - Path to the root directory to scan
///
/// # Returns
///
/// A vector of parsed `Ticket` structs for all valid ticket files.
/// Invalid tickets are logged and skipped.
pub fn scan_tickets_recursive<P: AsRef<Path>>(dir: P) -> Result<Vec<Ticket>, TicketError> {
    let dir = dir.as_ref();
    let mut tickets = Vec::new();
    scan_tickets_recursive_inner(dir, &mut tickets)?;
    Ok(tickets)
}

fn scan_tickets_recursive_inner(dir: &Path, tickets: &mut Vec<Ticket>) -> Result<(), TicketError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_tickets_recursive_inner(&path, tickets)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            match parse_ticket(&path) {
                Ok(ticket) => tickets.push(ticket),
                Err(e) => {
                    eprintln!("Warning: Failed to parse ticket {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

/// Updates a ticket's phase field in the file.
///
/// This reads the file, updates the phase in the frontmatter, and writes it back.
/// The rest of the file content is preserved exactly.
///
/// # Arguments
///
/// * `path` - Path to the ticket file
/// * `new_phase` - The new phase to set
///
/// # Errors
///
/// Returns `TicketError` if:
/// - The file cannot be read or written
/// - The file doesn't contain valid YAML frontmatter
pub fn update_ticket_phase<P: AsRef<Path>>(path: P, new_phase: Phase) -> Result<(), TicketError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;

    let updated = update_frontmatter_field(&content, "phase", &phase_to_string(new_phase))?;

    fs::write(path, updated)?;
    Ok(())
}

/// Atomically prepares a ticket's completion frontmatter.
///
/// Both `phase` and `status` are transformed in memory before the file is
/// written, so readers never observe a successful half-update from this call.
pub fn update_ticket_done<P: AsRef<Path>>(path: P) -> Result<(), TicketError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    let with_phase = update_frontmatter_field(&content, "phase", "done")?;
    let updated = update_frontmatter_field(&with_phase, "status", "done")?;

    fs::write(path, updated)?;
    Ok(())
}

/// Updates a single field in the YAML frontmatter.
fn update_frontmatter_field(
    content: &str,
    field: &str,
    new_value: &str,
) -> Result<String, TicketError> {
    let content_trimmed = content.trim_start();

    // Must start with ---
    if !content_trimmed.starts_with("---") {
        return Err(TicketError::MissingFrontmatter);
    }

    // Find the closing ---
    let after_opening = &content_trimmed[3..];
    let closing_pos = after_opening
        .find("\n---")
        .ok_or(TicketError::MissingFrontmatter)?;

    let frontmatter = &after_opening[..closing_pos];
    let rest = &after_opening[closing_pos..];

    // Update the field in frontmatter
    let updated_frontmatter = update_yaml_field(frontmatter, field, new_value);

    // Reconstruct the file
    let leading_whitespace = &content[..content.len() - content_trimmed.len()];
    Ok(format!(
        "{}---{}{}",
        leading_whitespace, updated_frontmatter, rest
    ))
}

/// Updates a field in a YAML string, or adds it if not present.
fn update_yaml_field(yaml: &str, field: &str, new_value: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;
    let field_prefix = format!("{}:", field);

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&field_prefix) {
            // Replace this line
            let indent = &line[..line.len() - line.trim_start().len()];
            lines.push(format!("{}{}: {}", indent, field, new_value));
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }

    // Add the field if it wasn't found
    if !found {
        lines.push(format!("{}: {}", field, new_value));
    }

    // Preserve the original line ending style
    if yaml.ends_with('\n') {
        lines.join("\n") + "\n"
    } else {
        lines.join("\n")
    }
}

/// Converts a Phase to its string representation.
///
/// The retired 0.4 names are absent by construction: this is the writer, and a
/// name that cannot be written here can never reappear on a board.
fn phase_to_string(phase: Phase) -> String {
    match phase {
        Phase::Ready => "ready",
        Phase::Implement => "implement",
        Phase::Review => "review",
        Phase::Done => "done",
    }
    .to_string()
}

/// Updates a ticket's status field in the file.
///
/// # Arguments
///
/// * `path` - Path to the ticket file
/// * `new_status` - The new status to set
pub fn update_ticket_status<P: AsRef<Path>>(
    path: P,
    new_status: TicketStatus,
) -> Result<(), TicketError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;

    let status_str = match new_status {
        TicketStatus::Open => "open",
        TicketStatus::InProgress => "in_progress",
        TicketStatus::Blocked => "blocked",
        TicketStatus::Review => "review",
        TicketStatus::Done => "done",
        TicketStatus::Cancelled => "cancelled",
    };

    let updated = update_frontmatter_field(&content, "status", status_str)?;

    fs::write(path, updated)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TICKET: &str = r#"---
id: T-024-03
story: S-024
title: migrate-climate-calls
type: task
status: open
priority: high
phase: research
depends_on: [T-024-01, T-024-02]
blocks: [T-024-06]
---

## Context

The climate API calls are scattered across three services and use inconsistent
error handling. They need to be consolidated into a single climate service module.

## Acceptance Criteria

- All climate API calls route through `src/services/climate.ts`
- Retry logic with exponential backoff on all external calls
- Existing tests pass, new integration tests for the consolidated service
"#;

    #[test]
    fn test_extract_frontmatter() {
        let (yaml, body) = extract_frontmatter(SAMPLE_TICKET).unwrap();

        assert!(yaml.contains("id: T-024-03"));
        assert!(yaml.contains("phase: research"));
        assert!(body.contains("## Context"));
        assert!(body.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_extract_frontmatter_missing_opening() {
        let content = "no frontmatter here";
        assert!(matches!(
            extract_frontmatter(content),
            Err(TicketError::MissingFrontmatter)
        ));
    }

    #[test]
    fn test_extract_frontmatter_missing_closing() {
        let content = "---\nid: test\nno closing delimiter";
        assert!(matches!(
            extract_frontmatter(content),
            Err(TicketError::MissingFrontmatter)
        ));
    }

    #[test]
    fn test_parse_ticket_content() {
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(SAMPLE_TICKET, path).unwrap();

        assert_eq!(ticket.id, "T-024-03");
        assert_eq!(ticket.story, Some("S-024".to_string()));
        assert_eq!(ticket.title, "migrate-climate-calls");
        assert!(matches!(ticket.ticket_type, TicketType::Task));
        assert!(matches!(ticket.status, TicketStatus::Open));
        assert!(matches!(ticket.priority, Priority::High));
        // SAMPLE_TICKET still says `phase: research`; it loads as Implement.
        assert!(matches!(ticket.phase, Phase::Implement));
        assert!(ticket.depends_on.contains(&"T-024-01".to_string()));
        assert!(ticket.depends_on.contains(&"T-024-02".to_string()));
        assert!(ticket.blocks.contains(&"T-024-06".to_string()));
        assert!(ticket.content.contains("## Context"));
    }

    #[test]
    fn test_parse_string_vec() {
        let result = parse_string_vec("[T-024-01, T-024-02]").unwrap();
        assert!(result.contains(&"T-024-01".to_string()));
        assert!(result.contains(&"T-024-02".to_string()));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_empty_array() {
        let result = parse_string_vec("[]").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_phases() {
        assert!(matches!(parse_phase("ready"), Ok(Phase::Ready)));
        assert!(matches!(parse_phase("implement"), Ok(Phase::Implement)));
        assert!(matches!(parse_phase("review"), Ok(Phase::Review)));
        assert!(matches!(parse_phase("done"), Ok(Phase::Done)));

        // Case insensitive
        assert!(matches!(parse_phase("IMPLEMENT"), Ok(Phase::Implement)));
        assert!(matches!(parse_phase("Review"), Ok(Phase::Review)));
    }

    /// The migration this change exists for. Four retired phase names must
    /// resolve to Implement through *both* parse entry points, so a board
    /// written against 0.4 loads whichever way Lisa reaches it.
    #[test]
    fn retired_phase_names_map_forward_through_both_parsers() {
        for name in ["research", "design", "structure", "plan"] {
            assert!(
                matches!(parse_phase(name), Ok(Phase::Implement)),
                "frontmatter parser rejected or misread `{name}`"
            );
            assert_eq!(
                Phase::from_name(name),
                Some(Phase::Implement),
                "config-key parser rejected or misread `{name}`"
            );
            // And case-insensitively, the way frontmatter is actually written.
            assert!(matches!(
                parse_phase(&name.to_uppercase()),
                Ok(Phase::Implement)
            ));
        }
    }

    /// The mapping widens what is accepted; it does not make parsing permissive.
    #[test]
    fn unknown_phase_is_still_rejected() {
        for name in ["speculate", "", "implemented", "reviewing"] {
            match parse_phase(name) {
                Err(TicketError::InvalidField { field, value, .. }) => {
                    assert_eq!(field, "phase");
                    assert_eq!(value, name);
                }
                other => panic!("`{name}` should not parse, got {other:?}"),
            }
            assert_eq!(Phase::from_name(name), None);
        }
    }

    /// A board self-heals: loaded at a retired phase, rewritten as `implement`.
    #[test]
    fn ticket_at_retired_phase_is_rewritten_as_implement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("T-900.md");
        fs::write(
            &path,
            "---\nid: T-900\ntitle: legacy-board-row\ntype: task\nstatus: open\n\
             priority: high\nphase: plan\ndepends_on: []\n---\n\n## Context\n",
        )
        .unwrap();

        let ticket = parse_ticket(&path).unwrap();
        assert_eq!(ticket.phase, Phase::Implement);

        update_ticket_phase(&path, ticket.phase).unwrap();

        let rewritten = fs::read_to_string(&path).unwrap();
        assert!(
            rewritten.contains("phase: implement"),
            "frontmatter was not migrated:\n{rewritten}"
        );
        for retired in [
            "phase: plan",
            "phase: research",
            "phase: design",
            "phase: structure",
        ] {
            assert!(
                !rewritten.contains(retired),
                "`{retired}` survived the rewrite:\n{rewritten}"
            );
        }
    }

    #[test]
    fn test_update_yaml_field() {
        let yaml = "id: test\nphase: research\npriority: high";
        let updated = update_yaml_field(yaml, "phase", "design");

        assert!(updated.contains("phase: design"));
        assert!(updated.contains("id: test"));
        assert!(updated.contains("priority: high"));
        assert!(!updated.contains("phase: research"));
    }

    #[test]
    fn test_update_yaml_field_add_new() {
        let yaml = "id: test\npriority: high";
        let updated = update_yaml_field(yaml, "phase", "design");

        assert!(updated.contains("phase: design"));
        assert!(updated.contains("id: test"));
        assert!(updated.contains("priority: high"));
    }

    #[test]
    fn test_update_frontmatter_field() {
        let updated = update_frontmatter_field(SAMPLE_TICKET, "phase", "design").unwrap();

        assert!(updated.contains("phase: design"));
        assert!(!updated.contains("phase: research"));
        assert!(updated.contains("id: T-024-03"));
        assert!(updated.contains("## Context"));
    }

    #[test]
    fn test_update_ticket_done_updates_both_fields_in_one_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("T-024-03.md");
        fs::write(&path, SAMPLE_TICKET).unwrap();

        update_ticket_done(&path).unwrap();

        let updated = fs::read_to_string(&path).unwrap();
        assert!(updated.contains("phase: done"));
        assert!(updated.contains("status: done"));
        assert!(updated.contains("id: T-024-03"));
        assert!(updated.contains("## Context"));
        assert!(!updated.contains("phase: research"));
        assert!(!updated.contains("status: open"));
    }

    #[test]
    fn test_update_ticket_done_does_not_write_malformed_ticket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("T-001.md");
        let original = "id: T-001\nphase: review\nstatus: open\n";
        fs::write(&path, original).unwrap();

        assert!(matches!(
            update_ticket_done(&path),
            Err(TicketError::MissingFrontmatter)
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn test_phase_to_string() {
        assert_eq!(phase_to_string(Phase::Ready), "ready");
        assert_eq!(phase_to_string(Phase::Implement), "implement");
        assert_eq!(phase_to_string(Phase::Review), "review");
        assert_eq!(phase_to_string(Phase::Done), "done");

        // The retired names have no arm here, so they can never be written
        // back to a board.
        for phase in Phase::all() {
            let written = phase_to_string(*phase);
            assert!(
                !["research", "design", "structure", "plan"].contains(&written.as_str()),
                "phase_to_string emitted a retired name: {written}"
            );
        }
    }

    #[test]
    fn test_missing_required_field() {
        let content = r#"---
id: T-001
title: test
type: task
status: open
priority: high
---
Body
"#;
        let path = Path::new("/test/ticket.md");
        let result = parse_ticket_content(content, path);

        assert!(matches!(result, Err(TicketError::MissingField(ref f)) if f == "phase"));
    }

    #[test]
    fn test_invalid_field_value() {
        let content = r#"---
id: T-001
title: test
type: task
status: open
priority: high
phase: invalid_phase
---
Body
"#;
        let path = Path::new("/test/ticket.md");
        let result = parse_ticket_content(content, path);

        assert!(matches!(
            result,
            Err(TicketError::InvalidField { field, .. }) if field == "phase"
        ));
    }

    #[test]
    fn test_optional_story_field() {
        let content = r#"---
id: T-001
title: test
type: task
status: open
priority: high
phase: ready
---
Body
"#;
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.story, None);
    }

    #[test]
    fn test_parse_ticket_without_blocks_field() {
        let content = r#"---
id: T-001
story: S-001
title: no-blocks-field
type: task
status: open
priority: high
phase: ready
depends_on: [T-000]
---

## Context

This ticket has no blocks field at all.

## Acceptance Criteria

- Parsing succeeds with an empty blocks vec
"#;
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.id, "T-001");
        assert_eq!(ticket.title, "no-blocks-field");
        assert!(
            ticket.blocks.is_empty(),
            "blocks should be empty when field is absent"
        );
        assert_eq!(ticket.depends_on, vec!["T-000".to_string()]);
    }

    #[test]
    fn test_ticket_error_display() {
        let err = TicketError::MissingField("id".to_string());
        assert_eq!(
            err.to_string(),
            "Required field 'id' is missing from frontmatter"
        );

        let err = TicketError::InvalidField {
            field: "phase".to_string(),
            value: "bad".to_string(),
            reason: "not valid".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value 'bad' for field 'phase': not valid"
        );
    }

    #[test]
    fn test_scan_with_diagnostics_clean() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\nBody\n",
        ).unwrap();
        fs::write(
            dir.path().join("T-002.md"),
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: medium\nphase: ready\n---\nBody\n",
        ).unwrap();

        let result = scan_tickets_with_diagnostics(dir.path()).unwrap();
        assert_eq!(result.tickets.len(), 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_scan_with_diagnostics_parse_error() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // Valid ticket
        fs::write(
            dir.path().join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\nBody\n",
        ).unwrap();
        // Invalid ticket (missing required fields)
        fs::write(
            dir.path().join("T-BAD.md"),
            "---\nid: T-BAD\ntitle: bad\n---\nNo type/status/priority/phase\n",
        )
        .unwrap();

        let result = scan_tickets_with_diagnostics(dir.path()).unwrap();
        assert_eq!(result.tickets.len(), 1);
        assert_eq!(result.tickets[0].id, "T-001");
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].0, dir.path().join("T-BAD.md"));
    }

    #[test]
    fn a_fingerprint_answers_has_anything_changed_without_reading_a_ticket() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let ticket = |id: &str, phase: &str| {
            format!(
                "---\nid: {id}\ntitle: t\ntype: task\nstatus: open\npriority: high\nphase: {phase}\n---\nBody\n"
            )
        };

        // An empty directory has a fingerprint, and taking it twice agrees.
        let empty = scan_fingerprint(dir.path()).unwrap();
        assert_eq!(scan_fingerprint(dir.path()).unwrap(), empty);

        // A ticket filed onto the board changes it. This is the case a drained
        // scheduler wakes on.
        fs::write(dir.path().join("T-001.md"), ticket("T-001", "ready")).unwrap();
        let one = scan_fingerprint(dir.path()).unwrap();
        assert_ne!(one, empty);
        assert_eq!(scan_fingerprint(dir.path()).unwrap(), one);

        // So does a second one, and so does removing it again.
        fs::write(dir.path().join("T-002.md"), ticket("T-002", "ready")).unwrap();
        let two = scan_fingerprint(dir.path()).unwrap();
        assert_ne!(two, one);
        fs::remove_file(dir.path().join("T-002.md")).unwrap();
        assert_eq!(scan_fingerprint(dir.path()).unwrap(), one);

        // An edit that leaves the file exactly as long — a phase advancing from
        // `ready` to `implem` would be one — still changes it, because the
        // modification time is part of the answer.
        fs::write(dir.path().join("T-001.md"), ticket("T-001", "donex")).unwrap();
        assert_ne!(scan_fingerprint(dir.path()).unwrap(), one);

        // Files that are not tickets are not on the board.
        let unrelated = scan_fingerprint(dir.path()).unwrap();
        fs::write(dir.path().join("notes.txt"), "scratch").unwrap();
        assert_eq!(scan_fingerprint(dir.path()).unwrap(), unrelated);

        // A directory that is not there cannot be described, and says so rather
        // than reporting "nothing changed".
        assert!(scan_fingerprint(dir.path().join("gone")).is_none());
    }

    #[test]
    fn test_scan_with_diagnostics_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_tickets_with_diagnostics(dir.path()).unwrap();
        assert!(result.tickets.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_parse_multiline_depends_on() {
        let content = "---\nid: T-003\ntitle: multi-dep\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on:\n  - T-001\n  - T-002\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.depends_on.len(), 2);
        assert!(ticket.depends_on.contains(&"T-001".to_string()));
        assert!(ticket.depends_on.contains(&"T-002".to_string()));
    }

    #[test]
    fn test_parse_multiline_blocks() {
        let content = "---\nid: T-001\ntitle: multi-block\ntype: task\nstatus: open\npriority: high\nphase: ready\nblocks:\n  - T-002\n  - T-003\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.blocks.len(), 2);
        assert!(ticket.blocks.contains(&"T-002".to_string()));
        assert!(ticket.blocks.contains(&"T-003".to_string()));
    }

    #[test]
    fn test_parse_multiline_single_item() {
        let content = "---\nid: T-002\ntitle: single-dep\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on:\n  - T-001\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.depends_on, vec!["T-001".to_string()]);
    }

    #[test]
    fn test_parse_multiline_and_inline_both_work() {
        // Inline format still works
        let inline = "---\nid: T-002\ntitle: inline\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: [T-001]\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let t1 = parse_ticket_content(inline, path).unwrap();
        assert_eq!(t1.depends_on, vec!["T-001".to_string()]);

        // Multiline format also works
        let multiline = "---\nid: T-002\ntitle: multiline\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on:\n  - T-001\n---\nBody\n";
        let t2 = parse_ticket_content(multiline, path).unwrap();
        assert_eq!(t2.depends_on, vec!["T-001".to_string()]);
    }

    #[test]
    fn test_parse_multiline_empty_list() {
        // `depends_on: []` should still produce empty vec
        let content = "---\nid: T-001\ntitle: empty\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on: []\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert!(ticket.depends_on.is_empty());
    }

    #[test]
    fn test_parse_routing_fields_inline() {
        let content = "---\nid: T-026-01\ntitle: routed\ntype: feature\nstatus: open\npriority: medium\nphase: ready\nagent: codex\nmodel: gpt-5\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert_eq!(ticket.agent.as_deref(), Some("codex"));
        assert_eq!(ticket.model.as_deref(), Some("gpt-5"));
    }

    #[test]
    fn test_parse_routing_fields_absent() {
        // A ticket with no routing hints parses with both fields None.
        let content = "---\nid: T-001\ntitle: unrouted\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert_eq!(ticket.agent, None);
        assert_eq!(ticket.model, None);
    }

    #[test]
    fn test_parse_invalid_agent_still_parses_raw() {
        // An unknown agent value must NOT fail parsing — it is stored raw and
        // validated (with fallback) at spawn. This is the "never fail the
        // ticket" contract (epic Decision 3).
        let content = "---\nid: T-001\ntitle: bad-agent\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: gpt\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert_eq!(ticket.agent.as_deref(), Some("gpt"));
    }

    #[test]
    fn test_parse_empty_agent_treated_as_absent() {
        let content = "---\nid: T-001\ntitle: bare\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent:\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert_eq!(ticket.agent, None);
    }

    #[test]
    fn test_parse_routing_fields_coexist_with_multiline_depends_on() {
        // The "inline + multiline" parsing criterion: routing scalars parse
        // correctly alongside a multiline list block.
        let content = "---\nid: T-002\ntitle: mixed\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: claude\nmodel: opus\ndepends_on:\n  - T-001\n  - T-000\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();
        assert_eq!(ticket.agent.as_deref(), Some("claude"));
        assert_eq!(ticket.model.as_deref(), Some("opus"));
        assert_eq!(ticket.depends_on.len(), 2);
        assert!(ticket.depends_on.contains(&"T-000".to_string()));
    }

    #[test]
    fn test_parse_multiline_list_resets_on_new_key() {
        // depends_on multiline list should stop when a new key appears
        let content = "---\nid: T-002\ntitle: reset\ntype: task\nstatus: open\npriority: high\nphase: ready\ndepends_on:\n  - T-001\nblocks:\n  - T-003\n---\nBody\n";
        let path = Path::new("/test/ticket.md");
        let ticket = parse_ticket_content(content, path).unwrap();

        assert_eq!(ticket.depends_on, vec!["T-001".to_string()]);
        assert_eq!(ticket.blocks, vec!["T-003".to_string()]);
    }
}
