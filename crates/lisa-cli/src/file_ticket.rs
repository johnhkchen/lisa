//! `lisa file-ticket` — the word for putting new work on the board.
//!
//! ## The gap this closes
//!
//! Lisa has a word for everything that happens to a ticket — `validate`,
//! `status`, `unblock`, `already-done`, `reset-ticket`, `commit-ticket`,
//! `complete-ticket`, `claim`, `proposal` — and had none for *making* one.
//! Filing meant hand-authoring frontmatter, picking an id that does not
//! collide, remembering that the story's `tickets:` list wants the id too, and
//! finding out from `lisa validate` afterwards whether any of it was right.
//! That is fine for a person with an editor open and wrong for a program.
//!
//! ## Why it is its own word and not a `proposal` subcommand
//!
//! `lisa proposal apply|dismiss` settles advice attached to a ticket that
//! already exists and is waiting on a person: it takes a ticket id, reads a
//! disposition somebody else wrote, and never writes a ticket. Filing takes a
//! draft, has no ticket id yet, and writes one. They share the sense of "not
//! done yet" and nothing else — no argument, no input, no output. Hanging
//! filing off `proposal` would put two unrelated jobs behind one noun.
//!
//! ## What it checks before anything lands
//!
//! Everything is decided in memory first; a refusal writes nothing at all. The
//! checks are the ticket-scoped subset of `lisa validate`:
//!
//! - the draft's frontmatter parses, and every field value is one Lisa knows
//! - the story exists, so `story:` is never a dangling reference
//! - every `depends_on` id is really on the board
//! - the allocated id is free, and the file it would take is not there
//!
//! Board-wide verdicts are deliberately *not* borrowed. `lisa validate` fails a
//! board with no ready ticket; refusing to file into such a board would refuse
//! exactly the boards filing exists to refill. A missing Acceptance Criteria
//! section is a warning there and stays a warning here.
//!
//! ## Why it is safe beside a running loop
//!
//! Filing into a live board is the expected case, so this command never refuses
//! on account of a run — but it owes that run two things.
//!
//! The ticket file is written to a temporary name and renamed into place, so a
//! scheduler mid-scan reads either no file or a whole one, never half of one.
//!
//! The story file is the serialization point. Every filer takes an exclusive
//! lock on it and holds that lock across reading the board, allocating the id,
//! and both writes — so two callers filing at once cannot choose the same
//! number or clobber each other's line in `tickets:`. The story is rewritten in
//! place rather than renamed, precisely so the lock stays meaningful: a rename
//! would swap the inode out from under a waiting filer.
//!
//! The story list is written *before* the ticket, so the only order a running
//! loop can observe is the consistent one — a ticket its story already names.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fs2::FileExt;

use crate::config;

/// Frontmatter keys a draft may carry. Everything else is either Lisa's to
/// write or a typo, and both are worth saying out loud.
const DRAFT_KEYS: [&str; 7] = [
    "title",
    "type",
    "priority",
    "story",
    "depends_on",
    "agent",
    "model",
];

/// Keys Lisa owns. A draft carrying one is refused by name rather than being
/// silently overwritten, because a caller who wrote `phase:` believed it.
const LISA_OWNED_KEYS: [&str; 4] = ["id", "phase", "status", "blocks"];

/// The shape a refusal shows, so a caller never has to guess it.
const DRAFT_SHAPE: &str = "A draft looks like this:\n\n\
     ---\n\
     title: a-short-kebab-case-name\n\
     type: task\n\
     priority: medium\n\
     depends_on: []\n\
     ---\n\n\
     ## Context\n\n\
     Why this work matters.\n\n\
     ## Acceptance Criteria\n\n\
     - What has to be true when it is done.";

/// How long a filer waits for another filer to finish with the same story.
const STORY_LOCK_WAIT: Duration = Duration::from_secs(10);
const STORY_LOCK_POLL: Duration = Duration::from_millis(20);

/// What the caller asked to be filed.
pub struct FileTicketRequest<'a> {
    /// Project root.
    pub root: &'a Path,
    /// Story named on the command line, if any. The draft may name it instead.
    pub story: Option<String>,
    /// The draft, exactly as it arrived on stdin.
    pub draft: &'a str,
}

/// What was filed. The fields a program reads are the fields printed as prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiledTicket {
    pub ticket_id: String,
    /// Repository-relative path of the ticket that was written.
    pub path: String,
    pub story: String,
    /// Repository-relative path of the story whose list now names the ticket.
    pub story_path: String,
    /// False when the story already listed the id and nothing needed adding.
    pub story_list_updated: bool,
    /// Non-fatal remarks — the same wording `lisa validate` uses for warnings.
    pub warnings: Vec<String>,
}

impl FiledTicket {
    /// The document `--json` prints. Named fields only: a consumer of this is
    /// the reason the command exists.
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "ticket_id": self.ticket_id,
            "path": self.path,
            "story": self.story,
            "story_path": self.story_path,
            "story_list_updated": self.story_list_updated,
            "phase": "ready",
            "status": "open",
            "warnings": self.warnings,
        })
    }

    /// The sentences a person reads. Same facts, same order.
    pub fn prose(&self) -> String {
        let mut lines = vec![format!("Filed {} — {}", self.ticket_id, self.path)];
        lines.push(if self.story_list_updated {
            format!("{} now lists it.", self.story)
        } else {
            format!("{} already listed it.", self.story)
        });
        for warning in &self.warnings {
            lines.push(format!("Warning: {warning}"));
        }
        lines.push(
            "It is ready, so the next run picks it up. Check the board with `lisa status`."
                .to_string(),
        );
        lines.join("\n")
    }
}

/// A draft, after parsing and before Lisa has decided anything about it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Draft {
    title: Option<String>,
    ticket_type: Option<String>,
    priority: Option<String>,
    story: Option<String>,
    depends_on: Vec<String>,
    agent: Option<String>,
    model: Option<String>,
    body: String,
}

/// Split a draft into its frontmatter and its body.
///
/// Deliberately the same rule `lisa_core::ticket` applies to a real ticket: the
/// document opens with `---` and the frontmatter ends at the first line that is
/// `---`. A draft that parses here parses there.
fn split_draft(draft: &str) -> Result<(&str, &str), String> {
    let trimmed = draft.trim_start();
    if trimmed.is_empty() {
        return Err(format!(
            "Nothing arrived on stdin, so there was nothing to file. Pipe a draft in.\n\n{DRAFT_SHAPE}"
        ));
    }
    let Some(after_opening) = trimmed.strip_prefix("---") else {
        return Err(format!(
            "This draft has no frontmatter: it must open with a line that is exactly `---`.\n\n{DRAFT_SHAPE}"
        ));
    };
    let Some(closing) = after_opening.find("\n---") else {
        return Err(format!(
            "This draft's frontmatter never closes: add a line that is exactly `---` after the last field.\n\n{DRAFT_SHAPE}"
        ));
    };
    Ok((
        &after_opening[..closing],
        after_opening[closing + 4..].trim_start_matches(['-', '\r']),
    ))
}

/// Parse one `key: value` line, the way ticket frontmatter is parsed.
fn split_field(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    Some((line[..colon].trim(), line[colon + 1..].trim()))
}

/// Parse an inline `[a, b]` list.
fn parse_inline_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_draft(draft: &str) -> Result<Draft, String> {
    let (frontmatter, body) = split_draft(draft)?;
    let mut parsed = Draft {
        body: body.trim().to_string(),
        ..Draft::default()
    };
    let mut in_depends_list = false;

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(item) = trimmed.strip_prefix("- ") {
            if !in_depends_list {
                return Err(format!(
                    "This draft has a list item (`{trimmed}`) that belongs to no field.\n\n{DRAFT_SHAPE}"
                ));
            }
            let item = item.trim();
            if !item.is_empty() {
                parsed.depends_on.push(item.to_string());
            }
            continue;
        }
        in_depends_list = false;

        let Some((key, value)) = split_field(trimmed) else {
            return Err(format!(
                "This draft's frontmatter has a line Lisa cannot read: `{trimmed}`. Every line is `key: value`.\n\n{DRAFT_SHAPE}"
            ));
        };
        if LISA_OWNED_KEYS.contains(&key) {
            return Err(format!(
                "This draft sets `{key}:`, which is Lisa's to write, so nothing was filed. Take that line out and file it again. Lisa allocates the id, files the ticket as ready and open, and works out what it blocks from `depends_on`."
            ));
        }
        if !DRAFT_KEYS.contains(&key) {
            return Err(format!(
                "This draft sets `{key}:`, which is not a field Lisa knows, so nothing was filed. A draft may set: {}.",
                DRAFT_KEYS.join(", ")
            ));
        }
        let value = value.to_string();
        match key {
            "title" => parsed.title = non_empty(value),
            "type" => parsed.ticket_type = non_empty(value),
            "priority" => parsed.priority = non_empty(value),
            "story" => parsed.story = non_empty(value),
            "agent" => parsed.agent = non_empty(value),
            "model" => parsed.model = non_empty(value),
            "depends_on" => {
                if value.is_empty() {
                    in_depends_list = true;
                } else {
                    parsed.depends_on = parse_inline_list(&value);
                }
            }
            _ => unreachable!("DRAFT_KEYS and this match must stay in step"),
        }
    }

    if parsed.title.is_none() {
        return Err(format!(
            "This draft has no `title:`, so nothing was filed. A title is the short kebab-case name the board shows.\n\n{DRAFT_SHAPE}"
        ));
    }
    if parsed.body.is_empty() {
        return Err(format!(
            "This draft has frontmatter and no body, so nothing was filed. The Context and Acceptance Criteria come from you — Lisa files what it is given and writes no prose of its own.\n\n{DRAFT_SHAPE}"
        ));
    }
    Ok(parsed)
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Settle which story this ticket belongs to.
///
/// Two sources may name it and they must agree: silently preferring one would
/// file the ticket somewhere the caller did not ask for.
fn resolve_story(flag: Option<&str>, drafted: Option<&str>) -> Result<String, String> {
    match (flag, drafted) {
        (Some(flag), Some(drafted)) if flag != drafted => Err(format!(
            "--story says {flag} and the draft says {drafted}, so nothing was filed. Name the story once."
        )),
        (Some(value), _) | (None, Some(value)) => Ok(value.to_string()),
        (None, None) => Err(
            "No story was named, so nothing was filed. Pass --story S-000-00, or set `story:` in the draft. A ticket's id is allocated inside its story's numbering."
                .to_string(),
        ),
    }
}

/// The ticket-id prefix a story's tickets are numbered under.
///
/// `S-065-01` numbers `T-065-01-01`, `T-065-01-02`, … — the rule every ticket
/// on this board and in `screen-design` already follows.
fn ticket_prefix(story_id: &str) -> Result<String, String> {
    let rest = story_id.strip_prefix("S-").filter(|rest| !rest.is_empty());
    match rest {
        Some(rest) => Ok(format!("T-{rest}")),
        None => Err(format!(
            "{story_id} is not a story id, so nothing was filed. A story id looks like S-065-01, and its tickets are numbered T-065-01-01, T-065-01-02, and so on."
        )),
    }
}

/// The number an existing id or filename spends inside `prefix`, if it spends one.
///
/// Reads the leading digits after the prefix so both naming styles on this
/// board count: `T-065-01-02.md` and `T-062-01-03-the-plugin-pane.md`.
fn sequence_of(prefix: &str, candidate: &str) -> Option<(u32, usize)> {
    let rest = candidate.strip_prefix(prefix)?.strip_prefix('-')?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok().map(|value| (value, digits.len()))
}

/// Allocate the next free number under `prefix`.
///
/// Both the parsed ids and the raw filenames are counted, so a ticket file too
/// broken to parse still cannot have its number handed out a second time.
fn next_ticket_id(prefix: &str, ticket_ids: &[String], file_stems: &[String]) -> String {
    let mut highest = 0u32;
    let mut width = 2usize;
    for candidate in ticket_ids.iter().chain(file_stems) {
        if let Some((sequence, digits)) = sequence_of(prefix, candidate) {
            highest = highest.max(sequence);
            width = width.max(digits);
        }
    }
    format!("{prefix}-{:0width$}", highest + 1, width = width)
}

/// Render the ticket file. Frontmatter in the order this board writes it;
/// the body is the caller's, unedited.
fn render_ticket(id: &str, story: &str, draft: &Draft) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("id: {id}\n"));
    out.push_str(&format!("story: {story}\n"));
    out.push_str(&format!(
        "title: {}\n",
        draft.title.as_deref().unwrap_or_default()
    ));
    out.push_str(&format!(
        "type: {}\n",
        draft.ticket_type.as_deref().unwrap_or("task")
    ));
    out.push_str("status: open\n");
    out.push_str(&format!(
        "priority: {}\n",
        draft.priority.as_deref().unwrap_or("medium")
    ));
    out.push_str(&format!("depends_on: [{}]\n", draft.depends_on.join(", ")));
    if let Some(agent) = &draft.agent {
        out.push_str(&format!("agent: {agent}\n"));
    }
    if let Some(model) = &draft.model {
        out.push_str(&format!("model: {model}\n"));
    }
    out.push_str("phase: ready\n");
    out.push_str("---\n\n");
    out.push_str(draft.body.trim());
    out.push('\n');
    out
}

/// Add `ticket_id` to a story's `tickets:` list.
///
/// Returns `None` when the list already names it, so filing twice cannot write
/// the id twice. Both spellings of a YAML list are handled, because both are
/// accepted everywhere else Lisa reads one.
fn story_with_ticket(content: &str, ticket_id: &str) -> Result<Option<String>, String> {
    let trimmed = content.trim_start();
    let leading = &content[..content.len() - trimmed.len()];
    let after_opening = trimmed
        .strip_prefix("---")
        .ok_or_else(|| "the story file has no frontmatter (no opening `---`)".to_string())?;
    let closing = after_opening
        .find("\n---")
        .ok_or_else(|| "the story file's frontmatter never closes".to_string())?;
    let frontmatter = &after_opening[..closing];
    let rest = &after_opening[closing..];

    let mut lines: Vec<String> = frontmatter.lines().map(str::to_string).collect();
    let mut listed = false;
    let mut inline_index = None;
    let mut list_start = None;

    for (index, line) in lines.iter().enumerate() {
        let trimmed_line = line.trim();
        if let Some((key, value)) = split_field(trimmed_line) {
            if key == "tickets" {
                if value.is_empty() {
                    list_start = Some(index);
                } else {
                    inline_index = Some(index);
                    listed = parse_inline_list(value).iter().any(|id| id == ticket_id);
                }
            }
        }
    }

    if let Some(index) = inline_index {
        if listed {
            return Ok(None);
        }
        let line = &lines[index];
        let (key, value) = split_field(line.trim()).expect("the line parsed a moment ago");
        let indent = &line[..line.len() - line.trim_start().len()];
        let mut items = parse_inline_list(value);
        items.push(ticket_id.to_string());
        lines[index] = format!("{indent}{key}: [{}]", items.join(", "));
    } else if let Some(index) = list_start {
        // A multiline list: the items are the `- ` lines that follow.
        let mut last = index;
        let mut item_indent = "  ".to_string();
        for (offset, line) in lines.iter().enumerate().skip(index + 1) {
            let trimmed_line = line.trim();
            if let Some(item) = trimmed_line.strip_prefix("- ") {
                if item.trim() == ticket_id {
                    return Ok(None);
                }
                item_indent = line[..line.len() - line.trim_start().len()].to_string();
                last = offset;
            } else if !trimmed_line.is_empty() {
                break;
            }
        }
        lines.insert(last + 1, format!("{item_indent}- {ticket_id}"));
    } else {
        lines.push(format!("tickets: [{ticket_id}]"));
    }

    Ok(Some(format!("{leading}---{}{rest}", lines.join("\n"))))
}

/// Take the story's lock, waiting out another filer rather than failing on it.
fn lock_story(file: &File, story_path: &Path) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => {
                return Err(format!(
                    "Lisa could not lock {} to file against it: {error}",
                    story_path.display()
                ));
            }
        }
        if started.elapsed() >= STORY_LOCK_WAIT {
            return Err(format!(
                "Another `lisa file-ticket` has held {} for {} seconds, so nothing was filed. Try again; if it never clears, look for a stuck lisa process.",
                story_path.display(),
                STORY_LOCK_WAIT.as_secs()
            ));
        }
        std::thread::sleep(STORY_LOCK_POLL);
    }
}

/// A ticket written to a temporary name, not yet on the board.
///
/// Staging is what makes "validated before it lands" literal rather than
/// approximate. The candidate is written, then read back through
/// [`lisa_core::ticket::parse_ticket`] — the same reader every scheduler and
/// every `lisa validate` uses — and only a file that survives that round trip
/// is given the name that puts it on the board. The temporary name is not a
/// `.md` file, so a scan running beside this one never sees it.
struct StagedTicket {
    temporary: PathBuf,
    placed: bool,
}

impl StagedTicket {
    fn write(ticket_path: &Path, content: &str) -> Result<Self, String> {
        let directory = ticket_path.parent().unwrap_or(Path::new("."));
        let name = ticket_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ticket.md".to_string());
        let temporary = directory.join(format!(".{name}.filing-{}", std::process::id()));

        let write = || -> Result<(), io::Error> {
            let mut file = File::create(&temporary)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()
        };
        if let Err(error) = write() {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "Lisa could not write {}: {error}. Nothing was filed.",
                ticket_path.display()
            ));
        }

        let staged = Self {
            temporary,
            placed: false,
        };
        lisa_core::ticket::parse_ticket(&staged.temporary).map_err(|error| {
            format!("This draft would not make a ticket Lisa can read: {error}. Nothing was filed.")
        })?;
        Ok(staged)
    }

    /// Give the staged file its real name. One rename, so a scan mid-flight
    /// reads either no ticket or the whole ticket.
    fn place(mut self, ticket_path: &Path) -> Result<(), String> {
        fs::rename(&self.temporary, ticket_path).map_err(|error| {
            format!(
                "Lisa could not put the ticket at {}: {error}",
                ticket_path.display()
            )
        })?;
        self.placed = true;
        Ok(())
    }
}

impl Drop for StagedTicket {
    fn drop(&mut self) {
        if !self.placed {
            let _ = fs::remove_file(&self.temporary);
        }
    }
}

/// Everything the board says about itself that filing needs to know.
struct Board {
    ticket_ids: Vec<String>,
    file_stems: Vec<String>,
}

fn read_board(ticket_dir: &Path) -> Result<Board, String> {
    let scan = lisa_core::ticket::scan_tickets_with_diagnostics(ticket_dir).map_err(|error| {
        format!(
            "Lisa could not read the board at {}: {error}",
            ticket_dir.display()
        )
    })?;
    let mut file_stems = Vec::new();
    for entry in fs::read_dir(ticket_dir)
        .map_err(|error| format!("Lisa could not read {}: {error}", ticket_dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
            file_stems.push(stem.to_string());
        }
    }
    Ok(Board {
        ticket_ids: scan.tickets.into_iter().map(|ticket| ticket.id).collect(),
        file_stems,
    })
}

/// File one drafted ticket, or refuse and write nothing.
pub fn file_ticket(request: FileTicketRequest<'_>) -> Result<FiledTicket, String> {
    let draft = parse_draft(request.draft)?;
    let story_id = resolve_story(request.story.as_deref(), draft.story.as_deref())?;
    let prefix = ticket_prefix(&story_id)?;

    let validation = config::load_config(request.root)?;
    let resolved = config::resolve_config(&validation.config, None, None);
    let ticket_dir = request.root.join(&resolved.ticket_dir);
    if !ticket_dir.is_dir() {
        return Err(format!(
            "There is no ticket folder at {}, so nothing was filed. Run `lisa init` first.",
            resolved.ticket_dir
        ));
    }
    let story_relative = format!("{}/{story_id}.md", resolved.story_dir.trim_end_matches('/'));
    let story_path = request.root.join(&story_relative);

    // The story file is both the thing that must exist and the lock every filer
    // takes, so opening it is the first thing that can refuse.
    let mut story_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&story_path)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => format!(
                "There is no story {story_id} at {story_relative}, so nothing was filed. Write the story first, or file against a story that is already there."
            ),
            _ => format!("Lisa could not open {story_relative}: {error}"),
        })?;
    lock_story(&story_file, &story_path)?;
    let filed = file_under_story_lock(
        &mut story_file,
        FilingContext {
            ticket_dir: &ticket_dir,
            ticket_dir_relative: &resolved.ticket_dir,
            story_id: &story_id,
            story_relative: &story_relative,
            prefix: &prefix,
            draft: &draft,
        },
    );
    let unlock = FileExt::unlock(&story_file);
    let filed = filed?;
    unlock.map_err(|error| {
        format!(
            "Lisa filed {} but could not release the lock on {story_relative}: {error}",
            filed.ticket_id
        )
    })?;
    Ok(filed)
}

struct FilingContext<'a> {
    ticket_dir: &'a Path,
    ticket_dir_relative: &'a str,
    story_id: &'a str,
    story_relative: &'a str,
    prefix: &'a str,
    draft: &'a Draft,
}

/// The part that reads and writes the board, with the story lock held.
fn file_under_story_lock(
    story_file: &mut File,
    context: FilingContext<'_>,
) -> Result<FiledTicket, String> {
    let board = read_board(context.ticket_dir)?;

    let missing: Vec<&String> = context
        .draft
        .depends_on
        .iter()
        .filter(|dependency| !board.ticket_ids.contains(dependency))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "This draft waits on {}, which {} on the board, so nothing was filed.",
            missing
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            if missing.len() == 1 {
                "is not"
            } else {
                "are not"
            }
        ));
    }

    let ticket_id = next_ticket_id(context.prefix, &board.ticket_ids, &board.file_stems);
    let ticket_relative = format!(
        "{}/{ticket_id}.md",
        context.ticket_dir_relative.trim_end_matches('/')
    );
    let ticket_path = context.ticket_dir.join(format!("{ticket_id}.md"));
    if board.ticket_ids.iter().any(|id| id == &ticket_id) || ticket_path.exists() {
        return Err(format!(
            "Lisa worked out {ticket_id} as the next id and something is already there, so nothing was filed. Run `lisa validate` and look for a duplicate id."
        ));
    }

    // Write the candidate under a name the board does not read, and parse it
    // back with the board's own reader. A ticket Lisa could not read is refused
    // while it is still nameless.
    let content = render_ticket(&ticket_id, context.story_id, context.draft);
    let staged = StagedTicket::write(&ticket_path, &content)?;

    let mut warnings = Vec::new();
    if !context.draft.body.contains("Acceptance Criteria")
        && !context.draft.body.contains("acceptance criteria")
    {
        warnings.push(format!(
            "{ticket_relative}: frontmatter (warning): missing Acceptance Criteria section"
        ));
    }

    let mut original_story = String::new();
    story_file
        .read_to_string(&mut original_story)
        .map_err(|error| format!("Lisa could not read {}: {error}", context.story_relative))?;
    let updated_story = story_with_ticket(&original_story, &ticket_id).map_err(|error| {
        format!(
            "Lisa could not add {ticket_id} to {}: {error}. Nothing was filed.",
            context.story_relative
        )
    })?;

    // The story goes first, so the only half-done state a running loop can
    // observe is a story naming a ticket that is a moment from existing —
    // never a ticket its story does not know about.
    if let Some(ref updated) = updated_story {
        rewrite_in_place(story_file, updated).map_err(|error| {
            format!(
                "Lisa could not write {}: {error}. Nothing was filed.",
                context.story_relative
            )
        })?;
    }

    if let Err(error) = staged.place(&ticket_path) {
        if updated_story.is_some() {
            if let Err(restore) = rewrite_in_place(story_file, &original_story) {
                return Err(format!(
                    "{error}. {} still lists {ticket_id}, and Lisa could not put it back: {restore}. Remove that id by hand.",
                    context.story_relative
                ));
            }
        }
        return Err(format!("{error}. Nothing was filed."));
    }

    Ok(FiledTicket {
        ticket_id,
        path: ticket_relative,
        story: context.story_id.to_string(),
        story_path: context.story_relative.to_string(),
        story_list_updated: updated_story.is_some(),
        warnings,
    })
}

/// Rewrite an already-open, already-locked file without changing its identity.
///
/// A rename would be atomic and would also replace the inode every other filer
/// is waiting on, which is the one thing this file must not do.
fn rewrite_in_place(file: &mut File, content: &str) -> Result<(), io::Error> {
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

/// Read the whole draft from stdin.
pub fn read_draft() -> Result<String, String> {
    let mut draft = String::new();
    io::stdin()
        .read_to_string(&mut draft)
        .map_err(|error| format!("Lisa could not read the draft from stdin: {error}"))?;
    Ok(draft)
}

/// Execute the command and print its answer. Returns the process exit code.
pub fn run_file_ticket(root: &Path, story: Option<String>, json: bool) -> i32 {
    let outcome = read_draft().and_then(|draft| {
        file_ticket(FileTicketRequest {
            root,
            story,
            draft: &draft,
        })
    });

    if json {
        return crate::json_output::emit_result("file-ticket", outcome.map(|filed| filed.json()));
    }
    match outcome {
        Ok(filed) => {
            println!("{}", filed.prose());
            0
        }
        Err(message) => {
            eprintln!("{message}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAFT: &str = "---\ntitle: a-new-thing\ntype: task\npriority: high\n---\n\n## Context\n\nWhy.\n\n## Acceptance Criteria\n\n- It works.\n";

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        fs::create_dir_all(dir.path().join("docs/active/stories")).unwrap();
        fs::write(
            dir.path().join("docs/active/stories/S-065-01.md"),
            "---\nid: S-065-01\ntitle: a-story\ntype: story\nstatus: open\npriority: high\ntickets: [T-065-01-01]\n---\n\n**Scope:** words.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("docs/active/tickets/T-065-01-01.md"),
            "---\nid: T-065-01-01\nstory: S-065-01\ntitle: the-first\ntype: task\nstatus: open\npriority: high\ndepends_on: []\nphase: ready\n---\n\n## Acceptance Criteria\n\n- Done.\n",
        )
        .unwrap();
        dir
    }

    fn file(dir: &Path, draft: &str) -> Result<FiledTicket, String> {
        file_ticket(FileTicketRequest {
            root: dir,
            story: Some("S-065-01".to_string()),
            draft,
        })
    }

    fn story_text(dir: &Path) -> String {
        fs::read_to_string(dir.join("docs/active/stories/S-065-01.md")).unwrap()
    }

    /// The whole point, end to end: a draft on stdin becomes a ticket the board
    /// can read, with an id nobody had to choose.
    #[test]
    fn a_draft_becomes_a_ticket_with_an_id_lisa_allocated() {
        let dir = project();
        let filed = file(dir.path(), DRAFT).unwrap();

        assert_eq!(filed.ticket_id, "T-065-01-02");
        assert_eq!(filed.path, "docs/active/tickets/T-065-01-02.md");
        assert!(filed.warnings.is_empty(), "{:?}", filed.warnings);

        let written = fs::read_to_string(dir.path().join(&filed.path)).unwrap();
        let ticket = lisa_core::ticket::parse_ticket(dir.path().join(&filed.path)).unwrap();
        assert_eq!(ticket.id, "T-065-01-02");
        assert_eq!(ticket.story.as_deref(), Some("S-065-01"));
        assert_eq!(ticket.title, "a-new-thing");
        assert_eq!(ticket.phase, lisa_core::types::Phase::Ready);
        assert_eq!(ticket.status, lisa_core::types::TicketStatus::Open);
        assert!(written.contains("## Context"), "{written}");
        assert!(
            !written.contains("Lisa"),
            "Lisa must not compose prose of its own: {written}"
        );
    }

    /// The tie `lisa validate` never caught: the story's list learns the id.
    #[test]
    fn the_storys_ticket_list_learns_the_new_id() {
        let dir = project();
        let filed = file(dir.path(), DRAFT).unwrap();

        assert!(filed.story_list_updated);
        assert!(
            story_text(dir.path()).contains("tickets: [T-065-01-01, T-065-01-02]"),
            "{}",
            story_text(dir.path())
        );
    }

    #[test]
    fn a_multiline_ticket_list_gains_one_more_item() {
        let dir = project();
        fs::write(
            dir.path().join("docs/active/stories/S-065-01.md"),
            "---\nid: S-065-01\ntitle: a-story\ntype: story\nstatus: open\npriority: high\ntickets:\n  - T-065-01-01\n---\n\nScope.\n",
        )
        .unwrap();

        file(dir.path(), DRAFT).unwrap();

        let story = story_text(dir.path());
        assert!(
            story.contains("  - T-065-01-01\n  - T-065-01-02\n"),
            "{story}"
        );
        assert!(
            story.contains("Scope."),
            "the story's prose survived: {story}"
        );
    }

    /// Filing is bookkeeping, so a story with no list at all gets one.
    #[test]
    fn a_story_with_no_list_gets_one() {
        let dir = project();
        fs::write(
            dir.path().join("docs/active/stories/S-065-01.md"),
            "---\nid: S-065-01\ntitle: a-story\ntype: story\nstatus: open\npriority: high\n---\n\nScope.\n",
        )
        .unwrap();

        file(dir.path(), DRAFT).unwrap();

        assert!(story_text(dir.path()).contains("tickets: [T-065-01-02]"));
    }

    #[test]
    fn ids_step_past_the_highest_number_in_use_whatever_the_filename_says() {
        let dir = project();
        fs::write(
            dir.path().join("docs/active/tickets/T-065-01-07-a-long-name.md"),
            "---\nid: T-065-01-07\nstory: S-065-01\ntitle: seven\ntype: task\nstatus: open\npriority: low\ndepends_on: []\nphase: done\n---\n\n## Acceptance Criteria\n\n- Done.\n",
        )
        .unwrap();

        assert_eq!(file(dir.path(), DRAFT).unwrap().ticket_id, "T-065-01-08");
    }

    /// A file too broken to parse still cannot have its number reissued.
    #[test]
    fn an_unparsable_ticket_file_still_holds_its_number() {
        let dir = project();
        fs::write(
            dir.path().join("docs/active/tickets/T-065-01-04.md"),
            "this file has no frontmatter at all\n",
        )
        .unwrap();

        assert_eq!(file(dir.path(), DRAFT).unwrap().ticket_id, "T-065-01-05");
    }

    #[test]
    fn a_ticket_can_wait_on_one_that_is_already_there() {
        let dir = project();
        let draft = "---\ntitle: second\ndepends_on: [T-065-01-01]\n---\n\n## Acceptance Criteria\n\n- It waits.\n";
        let filed = file(dir.path(), draft).unwrap();

        let ticket = lisa_core::ticket::parse_ticket(dir.path().join(&filed.path)).unwrap();
        assert_eq!(ticket.depends_on, vec!["T-065-01-01".to_string()]);
        // The unstated fields are Lisa's working defaults, not blanks.
        assert_eq!(ticket.ticket_type, lisa_core::types::TicketType::Task);
        assert_eq!(ticket.priority, lisa_core::types::Priority::Medium);
    }

    /// Refused before it lands, and nothing at all is written — not the ticket,
    /// not the story line.
    #[test]
    fn a_dependency_that_is_not_on_the_board_is_refused_and_nothing_is_written() {
        let dir = project();
        let draft = "---\ntitle: hopeful\ndepends_on: [T-999-99-99]\n---\n\n## Acceptance Criteria\n\n- No.\n";
        let error = file(dir.path(), draft).unwrap_err();

        assert!(error.contains("T-999-99-99"), "{error}");
        assert!(error.contains("nothing was filed"), "{error}");
        assert!(!dir
            .path()
            .join("docs/active/tickets/T-065-01-02.md")
            .exists());
        assert!(!story_text(dir.path()).contains("T-065-01-02"));
    }

    #[test]
    fn a_value_lisa_cannot_read_is_refused_with_the_reason() {
        let dir = project();
        let draft = "---\ntitle: odd\npriority: urgent\n---\n\n## Acceptance Criteria\n\n- No.\n";
        let error = file(dir.path(), draft).unwrap_err();

        assert!(error.contains("priority"), "{error}");
        assert!(error.contains("low, medium, high, critical"), "{error}");
        assert!(!dir
            .path()
            .join("docs/active/tickets/T-065-01-02.md")
            .exists());
    }

    #[test]
    fn a_draft_that_allocates_its_own_id_is_refused() {
        let dir = project();
        let draft = "---\nid: T-065-01-99\ntitle: mine\n---\n\n## Acceptance Criteria\n\n- No.\n";
        let error = file(dir.path(), draft).unwrap_err();

        assert!(error.contains("`id:`"), "{error}");
        assert!(error.contains("Lisa allocates the id"), "{error}");
    }

    #[test]
    fn a_draft_that_sets_phase_or_status_is_refused() {
        let dir = project();
        for field in ["phase: implement", "status: done", "blocks: [T-065-01-01]"] {
            let draft =
                format!("---\ntitle: mine\n{field}\n---\n\n## Acceptance Criteria\n\n- No.\n");
            let error = file(dir.path(), &draft).unwrap_err();
            assert!(error.contains("Lisa's to write"), "{field}: {error}");
        }
    }

    #[test]
    fn a_field_lisa_does_not_know_is_named_rather_than_dropped() {
        let dir = project();
        let draft =
            "---\ntitle: mine\ndepends: [T-065-01-01]\n---\n\n## Acceptance Criteria\n\n- No.\n";
        let error = file(dir.path(), draft).unwrap_err();

        assert!(error.contains("`depends:`"), "{error}");
        assert!(error.contains("depends_on"), "{error}");
    }

    #[test]
    fn a_draft_with_no_body_is_refused_because_lisa_writes_no_prose() {
        let dir = project();
        let error = file(dir.path(), "---\ntitle: empty\n---\n").unwrap_err();
        assert!(error.contains("no body"), "{error}");
    }

    #[test]
    fn a_draft_with_no_frontmatter_is_shown_the_shape() {
        let dir = project();
        let error = file(dir.path(), "## Context\n\nJust prose.\n").unwrap_err();
        assert!(error.contains("no frontmatter"), "{error}");
        assert!(error.contains("title: a-short-kebab-case-name"), "{error}");
    }

    #[test]
    fn an_empty_draft_says_so() {
        let dir = project();
        let error = file(dir.path(), "   \n").unwrap_err();
        assert!(error.contains("Nothing arrived on stdin"), "{error}");
    }

    #[test]
    fn a_story_that_is_not_there_is_refused_before_anything_is_written() {
        let dir = project();
        let error = file_ticket(FileTicketRequest {
            root: dir.path(),
            story: Some("S-999-99".to_string()),
            draft: DRAFT,
        })
        .unwrap_err();

        assert!(error.contains("S-999-99"), "{error}");
        assert!(error.contains("nothing was filed"), "{error}");
        assert_eq!(
            fs::read_dir(dir.path().join("docs/active/tickets"))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn two_names_for_the_story_must_agree() {
        let dir = project();
        let draft = "---\ntitle: mine\nstory: S-064-01\n---\n\n## Acceptance Criteria\n\n- No.\n";
        let error = file(dir.path(), draft).unwrap_err();

        assert!(error.contains("S-064-01"), "{error}");
        assert!(error.contains("S-065-01"), "{error}");
    }

    #[test]
    fn the_draft_may_name_the_story_on_its_own() {
        let dir = project();
        let draft = "---\ntitle: mine\nstory: S-065-01\n---\n\n## Acceptance Criteria\n\n- Yes.\n";
        let filed = file_ticket(FileTicketRequest {
            root: dir.path(),
            story: None,
            draft,
        })
        .unwrap();

        assert_eq!(filed.ticket_id, "T-065-01-02");
    }

    #[test]
    fn filing_with_no_story_at_all_says_what_to_pass() {
        let dir = project();
        let error = file_ticket(FileTicketRequest {
            root: dir.path(),
            story: None,
            draft: DRAFT,
        })
        .unwrap_err();

        assert!(error.contains("--story"), "{error}");
    }

    /// A missing Acceptance Criteria section is a warning in `lisa validate`,
    /// so it is a warning here — filing does not invent a stricter board.
    #[test]
    fn a_draft_with_no_acceptance_criteria_is_filed_with_a_warning() {
        let dir = project();
        let draft = "---\ntitle: thin\n---\n\n## Context\n\nJust context.\n";
        let filed = file(dir.path(), draft).unwrap();

        assert_eq!(filed.warnings.len(), 1, "{:?}", filed.warnings);
        assert!(filed.warnings[0].contains("Acceptance Criteria"));
        assert!(dir.path().join(&filed.path).exists());
    }

    /// Routing hints belong to the caller and survive the trip.
    #[test]
    fn routing_hints_are_carried_through() {
        let dir = project();
        let draft = "---\ntitle: routed\nagent: codex\nmodel: gpt-5\n---\n\n## Acceptance Criteria\n\n- Routed.\n";
        let filed = file(dir.path(), draft).unwrap();

        let ticket = lisa_core::ticket::parse_ticket(dir.path().join(&filed.path)).unwrap();
        assert_eq!(ticket.agent.as_deref(), Some("codex"));
        assert_eq!(ticket.model.as_deref(), Some("gpt-5"));
    }

    /// Two callers filing at once is the case the buttons make ordinary. Neither
    /// may take the other's number, and the story must end up naming both.
    #[test]
    fn two_filers_at_once_get_two_ids_and_the_story_lists_both() {
        let dir = project();
        let root = dir.path().to_path_buf();
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let root = root.clone();
                std::thread::spawn(move || {
                    file_ticket(FileTicketRequest {
                        root: &root,
                        story: Some("S-065-01".to_string()),
                        draft: &format!(
                            "---\ntitle: filer-{index}\n---\n\n## Acceptance Criteria\n\n- {index}.\n"
                        ),
                    })
                })
            })
            .collect();

        let filed: Vec<FiledTicket> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap().unwrap())
            .collect();

        let mut ids: Vec<String> = filed.iter().map(|f| f.ticket_id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4, "two filers took the same id: {ids:?}");

        let story = story_text(dir.path());
        for id in &ids {
            assert!(story.contains(id.as_str()), "{story} is missing {id}");
            assert!(dir
                .path()
                .join(format!("docs/active/tickets/{id}.md"))
                .exists());
        }
    }

    /// Filing the same story twice never writes the id twice.
    #[test]
    fn a_ticket_already_listed_is_not_listed_again() {
        let story = "---\nid: S-065-01\ntickets: [T-065-01-01, T-065-01-02]\n---\n\nScope.\n";
        assert_eq!(story_with_ticket(story, "T-065-01-02").unwrap(), None);
    }

    #[test]
    fn a_prefix_comes_from_the_story_id() {
        assert_eq!(ticket_prefix("S-065-01").unwrap(), "T-065-01");
        assert_eq!(ticket_prefix("S-024").unwrap(), "T-024");
        assert!(ticket_prefix("T-065-01").is_err());
        assert!(ticket_prefix("S-").is_err());
    }

    #[test]
    fn numbering_keeps_the_width_the_board_already_uses() {
        assert_eq!(next_ticket_id("T-065-01", &[], &[]), "T-065-01-01");
        assert_eq!(
            next_ticket_id("T-065-01", &["T-065-01-009".to_string()], &[]),
            "T-065-01-010"
        );
        // A different story's numbers are not this story's numbers.
        assert_eq!(
            next_ticket_id("T-065-01", &["T-064-01-07".to_string()], &[]),
            "T-065-01-01"
        );
    }

    #[test]
    fn the_answer_a_program_reads_says_the_same_thing_as_the_prose() {
        let dir = project();
        let filed = file(dir.path(), DRAFT).unwrap();
        let json = filed.json();

        assert_eq!(json["ticket_id"], "T-065-01-02");
        assert_eq!(json["path"], "docs/active/tickets/T-065-01-02.md");
        assert_eq!(json["story"], "S-065-01");
        assert_eq!(json["story_list_updated"], true);
        assert_eq!(json["phase"], "ready");
        assert_eq!(json["status"], "open");
        let prose = filed.prose();
        assert!(prose.contains("T-065-01-02"), "{prose}");
        assert!(
            prose.contains("docs/active/tickets/T-065-01-02.md"),
            "{prose}"
        );
        assert!(prose.contains("S-065-01"), "{prose}");
    }
}
