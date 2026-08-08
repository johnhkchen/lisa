//! `lisa clean` — the third command of the trio, and the only one that destroys
//! anything on a person's word rather than on proof.
//!
//! `lisa doctor` reports. `lisa init` carries forward what it can prove is safe.
//! `lisa clean` is where an operator says *yes, remove it* to the things init
//! deliberately would not.
//!
//! ## What may ever be a candidate
//!
//! One rule, and everything else in this module is that rule made mechanical:
//!
//! > **Lisa's litter is what Lisa wrote for one ticket that your board records as
//! > done, inside a directory Lisa created for that ticket — and nothing else is
//! > ever a candidate.**
//!
//! Two classes satisfy it: the five filenames the retired workflow produced under
//! `docs/active/work/{ticket}/`, and a finished ticket's attempt tree under
//! `.lisa/attempts/{ticket}/`. Pane signals are excluded because they are
//! pane-scoped rather than ticket-scoped, and are live state during a run;
//! `.lisa/completion-journal.jsonl`, `.lisa/provenance.jsonl` and `.lisa/hooks/`
//! are excluded because they are project state, not one ticket's leftovers; the
//! board and `.lisa.toml` are excluded because Lisa did not write them.
//!
//! Alongside litter, clean answers exactly the findings [`crate::currency`]
//! already routes to it — [`crate::currency::Remedy::Clean`] — which are the
//! retirements `lisa init` reported and preserved.
//!
//! **Clean removes files. It never edits a file's contents.** That single verb is
//! why a `.lisa.toml` key Lisa cannot lift out surgically is the operator's
//! one-line edit and not clean's business.
//!
//! ## The consent shape
//!
//! Default output is the plan, not the deletion. The plan is complete before
//! anything is touched and a bare run returns between printing it and executing
//! it, so "every removed path was named in the plan first" is a property of the
//! shape rather than a check that could be forgotten. The vocabulary is
//! [`crate::init`]'s — `remove`, `skip`, the `preserved:` prefix — because an
//! operator who knows one preview should recognise the other.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config;
use crate::currency::{self, Disposition, RetirementKind};

/// The five filenames the retired workflow produced.
///
/// `review.md` and `review-disposition.json` are absent deliberately: they are
/// the *current* workflow's artifacts and Lisa still reads them.
const RETIRED_WORKFLOW_NOTES: [&str; 5] = [
    "design.md",
    "plan.md",
    "progress.md",
    "research.md",
    "structure.md",
];

/// Where Lisa keeps one attempt's private working files, per ticket.
const ATTEMPTS_DIR: &str = ".lisa/attempts";
/// Where finished tickets are filed. Membership proves nothing; the `status:`
/// word does.
const ARCHIVE_TICKET_DIR: &str = "docs/archive/tickets";

const DEFAULT_TICKET_DIR: &str = "docs/active/tickets";
const DEFAULT_STORY_DIR: &str = "docs/active/stories";
const DEFAULT_WORK_DIR: &str = "docs/active/work";

/// How many `skip` lines to print before collapsing the rest into a count.
///
/// Refusals are informational and unbounded — this project's own board carries
/// 27 unfinished tickets — so they are capped, exactly as
/// `init::plan_retirements` caps retired-phase tickets, and for the same reason:
/// it is a decision about a preview, so it belongs to the thing rendering one.
/// Removals are never capped. A preview of a thousand deletions that hides nine
/// hundred of them is not a preview.
const RENDER_KEEP_LIMIT: usize = 5;

/// What clean does to one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanVerb {
    /// Delete one file.
    RemoveFile,
    /// Delete a directory that ends up empty because every entry in it is
    /// removed above. Executed with `remove_dir`, never `remove_dir_all`, so a
    /// wrong prediction fails instead of destroying something unplanned.
    RemoveEmptyDir,
    /// Delete a whole tree Lisa created for one finished ticket.
    RemoveTree,
    /// Considered and declined. Reasons carry init's `preserved:` prefix, so
    /// "this is yours" reads the same in both previews.
    Keep,
}

impl CleanVerb {
    fn is_removal(self) -> bool {
        !matches!(self, CleanVerb::Keep)
    }
}

/// Which of clean's three subjects a removal belongs to.
///
/// Carried explicitly rather than inferred from the path: the summary line has to
/// name the kinds of thing it is about, and a command that deletes should not be
/// guessing its own categories back out of strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanClass {
    /// A retirement `lisa init` reported and would not carry out.
    Currency,
    /// The retired workflow's notes, for a ticket the board records as done.
    RetiredNotes,
    /// A finished ticket's attempt tree.
    AttemptFolder,
}

impl CleanClass {
    /// The phrase the summary line uses for this class.
    fn phrase(self) -> &'static str {
        match self {
            CleanClass::Currency => "documents an older Lisa left behind",
            CleanClass::RetiredNotes => "retired workflow notes",
            CleanClass::AttemptFolder => "finished attempt folders",
        }
    }
}

/// One line of clean's plan: what, where, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanAction {
    pub(crate) verb: CleanVerb,
    /// `None` for a `Keep`, which belongs to no class because clean is not
    /// acting on it.
    pub(crate) class: Option<CleanClass>,
    pub(crate) path: PathBuf,
    pub(crate) reason: String,
}

impl CleanAction {
    /// One plan line, in init's columns, with the path written the way an
    /// operator would type it.
    ///
    /// Relative rather than absolute: init prints whatever it joined, which is
    /// tolerable across thirty lines and unreadable across a thousand.
    pub(crate) fn line(&self, root: &Path) -> String {
        let path = display_relative(root, &self.path);
        match self.verb {
            CleanVerb::RemoveFile => format!("  remove  {path} ({})", self.reason),
            CleanVerb::RemoveEmptyDir | CleanVerb::RemoveTree => {
                format!("  remove  {path}/ ({})", self.reason)
            }
            CleanVerb::Keep => format!("  skip    {path} ({})", self.reason),
        }
    }
}

/// Where this project keeps the directories clean has to reason about.
struct ProjectDirs {
    tickets: PathBuf,
    stories: PathBuf,
    work: PathBuf,
    archive_tickets: PathBuf,
    attempts: PathBuf,
}

impl ProjectDirs {
    fn resolve(root: &Path) -> Self {
        let configured = config::load_config(root)
            .ok()
            .map(|validation| validation.config.dirs);
        let (tickets, stories, work) = match configured {
            Some(dirs) => (dirs.tickets, dirs.stories, dirs.work),
            None => (None, None, None),
        };
        ProjectDirs {
            tickets: root.join(tickets.unwrap_or_else(|| DEFAULT_TICKET_DIR.to_string())),
            stories: root.join(stories.unwrap_or_else(|| DEFAULT_STORY_DIR.to_string())),
            work: root.join(work.unwrap_or_else(|| DEFAULT_WORK_DIR.to_string())),
            archive_tickets: root.join(ARCHIVE_TICKET_DIR),
            attempts: root.join(ATTEMPTS_DIR),
        }
    }
}

/// Whether a path is safe to touch, and if not, why not in the operator's words.
enum Reachability {
    Safe,
    Refused(String),
}

/// Everything `lisa clean` would do at `root`, computed before anything is
/// touched.
///
/// Reads the filesystem; changes nothing; prints nothing. Removals come first in
/// class order and sorted within a class, then every refusal sorted by path, so
/// two runs over the same tree produce the same list in the same order.
pub(crate) fn plan_clean_actions(root: &Path) -> Vec<CleanAction> {
    let dirs = ProjectDirs::resolve(root);
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let board = board_status(&dirs);

    let mut plan = Plan::default();
    plan_currency(root, &canonical_root, &mut plan);
    plan_retired_notes(root, &canonical_root, &dirs, &board, &mut plan);
    plan_attempt_trees(root, &canonical_root, &dirs, &board, &mut plan);
    plan.into_vec()
}

/// Removals and refusals kept apart while planning, so ordering is decided once.
#[derive(Default)]
struct Plan {
    removals: Vec<CleanAction>,
    keeps: Vec<CleanAction>,
}

impl Plan {
    fn remove(&mut self, verb: CleanVerb, class: CleanClass, path: PathBuf, reason: String) {
        self.removals.push(CleanAction {
            verb,
            class: Some(class),
            path,
            reason,
        });
    }

    fn keep(&mut self, path: PathBuf, reason: String) {
        self.keeps.push(CleanAction {
            verb: CleanVerb::Keep,
            class: None,
            path,
            reason: format!("preserved: {reason}"),
        });
    }

    fn into_vec(mut self) -> Vec<CleanAction> {
        self.keeps.sort_by(|left, right| left.path.cmp(&right.path));
        self.removals.extend(self.keeps);
        self.removals
    }
}

/// The retirements `lisa init` reported and would not carry out.
///
/// Only two of the four retirement kinds are removals clean may make:
///
/// - `WorkflowDocument` — a path only Lisa ever wrote to, describing a workflow
///   Lisa no longer runs.
/// - `ContextFile { proven_generation: true }` — bytes that match a generator
///   Lisa shipped, so nothing of the operator's is in the file.
///
/// The rest are dropped rather than reported, because init already prints a
/// `skip` line for each and repeating it here would imply clean had an opinion it
/// does not have. `ContextFile { proven_generation: false }` is somebody's own
/// writing that merely bears a Lisa mark, and T-057-02-02 kept it out of the
/// inventory precisely so it could never reach a command that deletes;
/// `ConfigKey` is a content edit, not a file removal; `TicketPhase` is the board.
///
/// A `Disposition::RemoveFile` is dropped too — init resolves those itself, which
/// is what makes `lisa clean` after `lisa init` find nothing rather than race it.
fn plan_currency(root: &Path, canonical_root: &Path, plan: &mut Plan) {
    for retirement in currency::retirements(root) {
        let Disposition::Preserve { path, reason } = &retirement.disposition else {
            continue;
        };
        let init_said = reason.strip_prefix("preserved: ").unwrap_or(reason);
        let reason = match &retirement.kind {
            RetirementKind::WorkflowDocument => format!(
                "describes a workflow Lisa no longer runs, and init left it alone: {init_said}"
            ),
            RetirementKind::ContextFile {
                proven_generation: true,
            } => format!(
                "Lisa generated this file and stopped maintaining it, so nothing of yours is in \
                 it; init left it alone: {init_said}"
            ),
            RetirementKind::ContextFile { .. }
            | RetirementKind::ConfigKey
            | RetirementKind::TicketPhase { .. } => continue,
        };

        match reachability(root, canonical_root, path) {
            Reachability::Safe => plan.remove(
                CleanVerb::RemoveFile,
                CleanClass::Currency,
                path.clone(),
                reason,
            ),
            Reachability::Refused(why) => plan.keep(path.clone(), why),
        }
    }
}

/// The retired workflow's notes, for tickets the board records as done.
///
/// Named files only, never the directory: a work directory also holds the current
/// workflow's `review.md`, an operator's own notes, and whole subdirectories of
/// evidence. The five names are the warrant and the ticket being done is the
/// permission — neither alone is enough.
fn plan_retired_notes(
    root: &Path,
    canonical_root: &Path,
    dirs: &ProjectDirs,
    board: &BTreeMap<String, String>,
    plan: &mut Plan,
) {
    for ticket_dir in sorted_dir_entries(&dirs.work) {
        if !ticket_dir.is_dir() {
            continue;
        }
        let Some(ticket_id) = ticket_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let notes: Vec<PathBuf> = RETIRED_WORKFLOW_NOTES
            .iter()
            .map(|name| ticket_dir.join(name))
            .filter(|note| note.exists())
            .collect();
        if notes.is_empty() {
            continue;
        }

        if let Some(reason) = not_done_reason(board, ticket_id) {
            plan.keep(ticket_dir.clone(), reason);
            continue;
        }

        let mut planned = 0usize;
        for note in &notes {
            match reachability(root, canonical_root, note) {
                Reachability::Safe => {
                    planned += 1;
                    plan.remove(
                        CleanVerb::RemoveFile,
                        CleanClass::RetiredNotes,
                        note.clone(),
                        format!(
                            "{ticket_id} is done, and the workflow that wrote this stopped running \
                             in 0.5.0"
                        ),
                    );
                }
                Reachability::Refused(why) => plan.keep(note.clone(), why),
            }
        }

        // The directory ends up empty exactly when every entry in it is one of
        // the notes above — predictable at plan time, so it gets its own line.
        if planned == notes.len() && planned == sorted_dir_entries(&ticket_dir).len() {
            plan.remove(
                CleanVerb::RemoveEmptyDir,
                CleanClass::RetiredNotes,
                ticket_dir.clone(),
                "nothing left in it once the notes above are gone".to_string(),
            );
        }
    }
}

/// A finished ticket's attempt tree — Lisa's own working files, from panes that
/// are gone, already excluded from history by `.lisa/.gitignore`.
fn plan_attempt_trees(
    root: &Path,
    canonical_root: &Path,
    dirs: &ProjectDirs,
    board: &BTreeMap<String, String>,
    plan: &mut Plan,
) {
    for attempt_dir in sorted_dir_entries(&dirs.attempts) {
        if !attempt_dir.is_dir() {
            continue;
        }
        let Some(ticket_id) = attempt_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if let Some(reason) = not_done_reason(board, ticket_id) {
            plan.keep(attempt_dir.clone(), reason);
            continue;
        }

        match reachability(root, canonical_root, &attempt_dir) {
            Reachability::Safe => plan.remove(
                CleanVerb::RemoveTree,
                CleanClass::AttemptFolder,
                attempt_dir.clone(),
                format!(
                    "Lisa's own working files from the panes that ran {ticket_id}, which is done"
                ),
            ),
            Reachability::Refused(why) => plan.keep(attempt_dir.clone(), why),
        }
    }
}

/// `None` when the board records this ticket done; otherwise the sentence saying
/// what it records instead.
///
/// A ticket absent from the board is **not** done. The board recording nothing is
/// not the board recording it finished, and a lookup fails whenever a ticket was
/// renamed, filed elsewhere, or is still being drafted — so an orphaned work
/// directory is refused, and said so, rather than destroyed on a failed lookup.
fn not_done_reason(board: &BTreeMap<String, String>, ticket_id: &str) -> Option<String> {
    match board.get(ticket_id) {
        Some(status) if status == "done" => None,
        Some(status) => Some(format!(
            "your board records {ticket_id} as {status}, so this is live work"
        )),
        None => Some(format!(
            "nothing on your board records {ticket_id} finished"
        )),
    }
}

/// Every ticket id the board knows, with the `status:` word written in its file.
///
/// Both the active ticket directory and the archive are read, and only the word
/// decides: this project's own archive holds tickets that still say `open`.
///
/// The line is read raw rather than through `lisa_core::ticket::parse_ticket` on
/// purpose. A ticket Lisa cannot parse must read as *not done*, and a parse
/// failure that dropped the id from this map would be indistinguishable from a
/// ticket that was never there — a distinction [`not_done_reason`] depends on.
fn board_status(dirs: &ProjectDirs) -> BTreeMap<String, String> {
    let mut board = BTreeMap::new();
    for dir in [&dirs.tickets, &dirs.archive_tickets] {
        for path in sorted_dir_entries(dir) {
            if path.extension().is_none_or(|extension| extension != "md") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let id = frontmatter_value(&content, "id").or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            });
            let Some(id) = id else { continue };
            let status =
                frontmatter_value(&content, "status").unwrap_or_else(|| "unreadable".to_string());
            board.insert(id, status);
        }
    }
    board
}

/// Read one scalar out of a markdown file's YAML frontmatter block.
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

/// Entries of `dir`, sorted, or empty when it cannot be read.
fn sorted_dir_entries(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    paths
}

/// The last gate before a path may enter the plan as a removal.
///
/// Four checks, and the first three are what make the fourth sufficient: a test
/// on the leaf alone is defeated by a symlinked parent, and a tree containing a
/// link out of the project cannot be described honestly by a plan that never
/// looked inside it.
fn reachability(root: &Path, canonical_root: &Path, path: &Path) -> Reachability {
    let Ok(relative) = path.strip_prefix(root) else {
        return Reachability::Refused("it sits outside the project root".to_string());
    };

    let mut walked = root.to_path_buf();
    for component in relative.components() {
        walked.push(component);
        match fs::symlink_metadata(&walked) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Reachability::Refused(format!(
                    "{} is a symlink, and Lisa will not follow one to delete anything",
                    display_relative(root, &walked)
                ));
            }
            Ok(_) => {}
            Err(_) => {
                return Reachability::Refused(
                    "Lisa could not read it to see where it leads".to_string(),
                )
            }
        }
    }

    if walked.is_dir() {
        if let Some(found) = first_symlink_within(&walked) {
            return Reachability::Refused(format!(
                "{} inside it is a symlink, and Lisa will not follow one to delete anything",
                display_relative(root, &found)
            ));
        }
    }

    match path.canonicalize() {
        Ok(real) if real.starts_with(canonical_root) => Reachability::Safe,
        Ok(_) => Reachability::Refused("its real location is outside the project".to_string()),
        Err(_) => Reachability::Refused("Lisa could not resolve its real location".to_string()),
    }
}

/// The first symlink anywhere below `dir`, if there is one. An unreadable entry
/// counts: clean cannot say what it is, so it will not delete around it.
fn first_symlink_within(dir: &Path) -> Option<PathBuf> {
    for path in sorted_dir_entries(dir) {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Some(path),
            Ok(metadata) if metadata.is_dir() => {
                if let Some(found) = first_symlink_within(&path) {
                    return Some(found);
                }
            }
            Ok(_) => {}
            Err(_) => return Some(path),
        }
    }
    None
}

/// Render a path the way an operator would type it: relative to the project.
fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {many}")
    }
}

/// The one line that lets a reader decide without reading the list.
///
/// Counts first, then which kinds of thing they are, then what is staying. A
/// reader who stops here knows the shape of the deletion and whose files are in
/// it.
fn summary_line(plan: &[CleanAction]) -> String {
    let count = |verb: CleanVerb| plan.iter().filter(|action| action.verb == verb).count();
    let files = count(CleanVerb::RemoveFile);
    let folders = count(CleanVerb::RemoveEmptyDir) + count(CleanVerb::RemoveTree);
    let kept = count(CleanVerb::Keep);

    if files == 0 && folders == 0 {
        return "Nothing to remove.".to_string();
    }

    let mut counted = vec![plural(files, "file", "files")];
    if folders > 0 {
        counted.push(plural(folders, "folder", "folders"));
    }

    let classes: Vec<&str> = [
        CleanClass::Currency,
        CleanClass::RetiredNotes,
        CleanClass::AttemptFolder,
    ]
    .into_iter()
    .filter(|class| plan.iter().any(|action| action.class == Some(*class)))
    .map(CleanClass::phrase)
    .collect();

    let mut line = format!(
        "{} to remove: {}. Lisa wrote all of it.",
        counted.join(" and "),
        classes.join(", ")
    );
    if kept > 0 {
        line.push_str(&format!(
            " {} left alone, each listed below with the reason.",
            plural(kept, "thing", "things")
        ));
    }
    line
}

fn write_line(out: &mut impl Write, args: fmt::Arguments<'_>) -> Result<(), String> {
    writeln!(out, "{args}").map_err(|error| format!("Failed to write clean output: {error}"))
}

/// Execute the clean command, writing operator-facing output to stdout.
pub fn run_clean(root: &Path, remove: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    run_clean_with_writer(root, remove, &mut out)
}

/// Print the plan, and carry it out only when `remove` is true.
///
/// The plan is complete before the first mutation, and the preview run returns
/// before the execute loop, so a removed path that was never printed is not
/// reachable from here.
fn run_clean_with_writer(root: &Path, remove: bool, out: &mut impl Write) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let plan = plan_clean_actions(root);
    write_line(out, format_args!("{}", summary_line(&plan)))?;
    if plan.is_empty() {
        return Ok(());
    }

    let (removals, keeps): (Vec<&CleanAction>, Vec<&CleanAction>) =
        plan.iter().partition(|action| action.verb.is_removal());

    write_line(out, format_args!(""))?;
    write_line(out, format_args!("Planned actions:"))?;
    for action in &removals {
        write_line(out, format_args!("{}", action.line(root)))?;
    }
    for action in keeps.iter().take(RENDER_KEEP_LIMIT) {
        write_line(out, format_args!("{}", action.line(root)))?;
    }
    if keeps.len() > RENDER_KEEP_LIMIT {
        write_line(
            out,
            format_args!(
                "  skip    {} more, each for the same kind of reason",
                keeps.len() - RENDER_KEEP_LIMIT
            ),
        )?;
    }
    write_line(out, format_args!(""))?;

    if removals.is_empty() {
        return Ok(());
    }

    let dirs = ProjectDirs::resolve(root);
    write_line(
        out,
        format_args!(
            "Never a candidate: your board ({}/, {}/), your settings, and anything Lisa did not \
             write.",
            display_relative(root, &dirs.tickets),
            display_relative(root, &dirs.stories)
        ),
    )?;
    write_line(out, format_args!(""))?;

    if !remove {
        write_line(
            out,
            format_args!("Dry run complete. No changes made. Add --remove to carry this list out."),
        )?;
        return Ok(());
    }

    let mut removed_files = 0usize;
    let mut removed_folders = 0usize;
    let mut failures = Vec::new();
    for action in &removals {
        let outcome = match action.verb {
            CleanVerb::RemoveFile => fs::remove_file(&action.path).map(|()| &mut removed_files),
            CleanVerb::RemoveTree => {
                fs::remove_dir_all(&action.path).map(|()| &mut removed_folders)
            }
            // `remove_dir`, never `remove_dir_all`: if a file appeared between
            // the plan and now, this fails and the directory stays rather than
            // clean deleting something no line named.
            CleanVerb::RemoveEmptyDir => {
                fs::remove_dir(&action.path).map(|()| &mut removed_folders)
            }
            CleanVerb::Keep => continue,
        };
        match outcome {
            Ok(counter) => *counter += 1,
            Err(error) => failures.push(format!(
                "  {} ({error})",
                display_relative(root, &action.path)
            )),
        }
    }

    let mut counted = vec![plural(removed_files, "file", "files")];
    if removed_folders > 0 {
        counted.push(plural(removed_folders, "folder", "folders"));
    }
    write_line(
        out,
        format_args!(
            "Removed {}. Everything else is as it was.",
            counted.join(" and ")
        ),
    )?;

    if !failures.is_empty() {
        write_line(out, format_args!(""))?;
        write_line(out, format_args!("Could not remove:"))?;
        for failure in &failures {
            write_line(out, format_args!("{failure}"))?;
        }
        return Err(format!(
            "{} of {} planned removals did not happen; every one is listed above",
            failures.len(),
            removals.len()
        ));
    }

    Ok(())
}
