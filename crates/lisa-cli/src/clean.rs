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
///
/// Shared with [`crate::seats`], which deletes a different class of file for a
/// different reason but must refuse a symlinked path for exactly the same one —
/// and must refuse it in the same words.
pub(crate) enum Reachability {
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
pub(crate) fn reachability(root: &Path, canonical_root: &Path, path: &Path) -> Reachability {
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
pub(crate) fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn plural(count: usize, one: &str, many: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::{inventory, Remedy};

    /// A ticket file, at whatever status the caller needs.
    fn ticket(id: &str, status: &str) -> String {
        format!(
            "---\nid: {id}\nstory: S-024\ntitle: migrate-climate-calls\ntype: task\nstatus: {status}\npriority: high\nphase: done\ndepends_on: []\n---\n\n## Context\n\nWork.\n"
        )
    }

    fn write(path: PathBuf, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// The five retired notes, plus whatever else the caller asked for.
    fn retired_notes(root: &Path, ticket_id: &str) {
        for name in RETIRED_WORKFLOW_NOTES {
            write(
                root.join(format!("docs/active/work/{ticket_id}/{name}")),
                &format!("# {name} for {ticket_id}\n"),
            );
        }
    }

    fn attempt(root: &Path, ticket_id: &str) {
        write(
            root.join(format!(".lisa/attempts/{ticket_id}/1/work/assignment.md")),
            "Do the work.\n",
        );
    }

    /// A project carrying every class this command reasons about at once.
    ///
    /// `T-024-01` is done and has notes, the current workflow's `review.md`, an
    /// operator's own note and a whole `harness/` subdirectory beside them.
    /// `T-024-02` is open, `T-024-03` is in review, `T-024-04` is done and has
    /// nothing but notes, and `T-024-99` has notes but no ticket anywhere.
    fn litter_fixture(root: &Path) {
        write(
            root.join(".lisa.toml"),
            &format!("version = \"{}\"\n", config::LISA_VERSION),
        );

        write(
            root.join("docs/active/tickets/T-024-01.md"),
            &ticket("T-024-01", "done"),
        );
        write(
            root.join("docs/active/tickets/T-024-02.md"),
            &ticket("T-024-02", "open"),
        );
        write(
            root.join("docs/active/tickets/T-024-03.md"),
            &ticket("T-024-03", "review"),
        );
        write(
            root.join("docs/archive/tickets/T-024-04.md"),
            &ticket("T-024-04", "done"),
        );
        write(
            root.join("docs/active/stories/S-024.md"),
            "---\nid: S-024\ntitle: climate\nstatus: done\n---\n\nThe story.\n",
        );

        for id in ["T-024-01", "T-024-02", "T-024-03", "T-024-04", "T-024-99"] {
            retired_notes(root, id);
        }
        write(
            root.join("docs/active/work/T-024-01/review.md"),
            "# Review\n\nWhat changed.\n",
        );
        write(
            root.join("docs/active/work/T-024-01/review-disposition.json"),
            "{\"disposition\":\"pass\",\"reason\":null}\n",
        );
        write(
            root.join("docs/active/work/T-024-01/operator-note.md"),
            "Ask Priya about the cache before touching this again.\n",
        );
        write(
            root.join("docs/active/work/T-024-01/harness/probe.sh"),
            "#!/bin/sh\necho probe\n",
        );

        attempt(root, "T-024-01");
        attempt(root, "T-024-02");

        write(
            root.join("README.md"),
            "# my-app\n\nRuns the climate suite.\n",
        );
        write(
            root.join("CLAUDE.md"),
            "# CLAUDE.md\n\nRun the suite first.\n",
        );
        write(
            root.join("docs/knowledge/our-notes.md"),
            "Our own notes, nothing to do with Lisa.\n",
        );
        write(root.join(".lisa/completion-journal.jsonl"), "{}\n");
        write(root.join(".lisa/provenance.jsonl"), "{}\n");
        write(root.join(".lisa/hooks/on-stop.sh"), "#!/bin/sh\n");
        write(root.join(".lisa/signals/pane-0.lease"), "0\n");
    }

    /// Every path under `root`: files with their exact bytes, directories and
    /// symlinks by name. Directories are in the snapshot so that removing an
    /// empty one cannot hide inside a file-only comparison.
    fn tree_snapshot(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, Option<Vec<u8>>)>) {
            for path in sorted_dir_entries(dir) {
                let relative = display_relative(root, &path);
                let metadata = fs::symlink_metadata(&path).unwrap();
                if metadata.file_type().is_symlink() {
                    out.push((format!("{relative} (symlink)"), None));
                } else if metadata.is_dir() {
                    out.push((format!("{relative}/"), None));
                    walk(&path, root, out);
                } else {
                    out.push((relative, Some(fs::read(&path).unwrap())));
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    fn clean_output(root: &Path, remove: bool) -> String {
        let mut output = Vec::new();
        run_clean_with_writer(root, remove, &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    fn removals(plan: &[CleanAction]) -> Vec<&CleanAction> {
        plan.iter()
            .filter(|action| action.verb.is_removal())
            .collect()
    }

    fn relative_removals(root: &Path, plan: &[CleanAction]) -> Vec<String> {
        removals(plan)
            .iter()
            .map(|action| display_relative(root, &action.path))
            .collect()
    }

    /// A `CLAUDE.md` byte-exact to what a 0.4.4 `generate_claude_md` wrote.
    fn generated_claude_md() -> String {
        format!(
            "{}my-app (Rust) — TODO: add a one-line project description here.\n\n### Build and Test\n\n```bash\n# Build\ncargo build\n\n# Run tests\ncargo test\n\n# Lint\ncargo clippy\n```\n\n### Source Layout\n\n```\nsrc:\n  main.rs\n```\n\n{}",
            include_str!("../data/legacy/claude-md-header-v0.4.4.md"),
            include_str!("../data/legacy/claude-md-tail.md"),
        )
    }

    /// **The default this ticket turns on.** A bare run is a preview, and a
    /// preview that changed one byte would be a lie about a deletion.
    #[test]
    fn a_bare_run_prints_the_plan_and_changes_not_one_byte() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());
        let before = tree_snapshot(dir.path());

        let output = clean_output(dir.path(), false);

        assert!(output.contains("Planned actions:"), "{output}");
        assert!(
            output.contains("  remove  docs/active/work/T-024-01/research.md"),
            "{output}"
        );
        assert!(
            output.contains("Dry run complete. No changes made."),
            "{output}"
        );
        assert_eq!(
            tree_snapshot(dir.path()),
            before,
            "a bare `lisa clean` must leave the tree byte-identical"
        );
    }

    /// Removal happens only under `--remove`, and nothing vanishes that the plan
    /// did not name first.
    #[test]
    fn every_removed_path_was_named_in_the_plan_first() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());

        let plan = plan_clean_actions(dir.path());
        let planned = relative_removals(dir.path(), &plan);
        let before = tree_snapshot(dir.path());

        let output = clean_output(dir.path(), true);
        assert!(output.contains("Removed "), "{output}");

        let after = tree_snapshot(dir.path());
        let survived: Vec<&String> = after.iter().map(|(path, _)| path).collect();
        for (path, _) in &before {
            if survived.contains(&path) {
                continue;
            }
            let bare = path.trim_end_matches('/');
            assert!(
                planned.iter().any(|named| named == bare)
                    || planned
                        .iter()
                        .any(|named| bare.starts_with(&format!("{named}/"))),
                "{path} vanished without a plan line naming it; plan was {planned:?}"
            );
        }
        for named in &planned {
            assert!(
                !survived
                    .iter()
                    .any(|path| path.trim_end_matches('/') == named),
                "{named} was planned for removal and is still there"
            );
        }
    }

    /// The board is the operator's, even the parts of it Lisa scheduled.
    #[test]
    fn the_board_is_never_a_candidate() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());
        let ticket_before = fs::read(dir.path().join("docs/active/tickets/T-024-01.md")).unwrap();
        let story_before = fs::read(dir.path().join("docs/active/stories/S-024.md")).unwrap();

        let plan = plan_clean_actions(dir.path());
        for action in &plan {
            let relative = display_relative(dir.path(), &action.path);
            assert!(
                !relative.starts_with("docs/active/tickets/")
                    && !relative.starts_with("docs/active/stories/")
                    && !relative.starts_with("docs/archive/tickets/"),
                "{relative} is on the board and must not appear in clean's plan at all"
            );
        }

        clean_output(dir.path(), true);
        assert_eq!(
            fs::read(dir.path().join("docs/active/tickets/T-024-01.md")).unwrap(),
            ticket_before
        );
        assert_eq!(
            fs::read(dir.path().join("docs/active/stories/S-024.md")).unwrap(),
            story_before
        );
    }

    /// Nothing outside the two per-ticket directories Lisa creates, and nothing
    /// inside them that Lisa did not write.
    #[test]
    fn nothing_outside_lisas_own_directories_is_a_candidate() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());

        let untouchable = [
            "README.md",
            "CLAUDE.md",
            ".lisa.toml",
            "docs/knowledge/our-notes.md",
            ".lisa/completion-journal.jsonl",
            ".lisa/provenance.jsonl",
            ".lisa/hooks/on-stop.sh",
            ".lisa/signals/pane-0.lease",
            // Inside a done ticket's own work directory, beside the notes.
            "docs/active/work/T-024-01/review.md",
            "docs/active/work/T-024-01/review-disposition.json",
            "docs/active/work/T-024-01/operator-note.md",
            "docs/active/work/T-024-01/harness/probe.sh",
        ];
        let before: Vec<Vec<u8>> = untouchable
            .iter()
            .map(|path| fs::read(dir.path().join(path)).unwrap())
            .collect();

        let plan = plan_clean_actions(dir.path());
        let named = relative_removals(dir.path(), &plan);
        for path in untouchable {
            assert!(
                !named.iter().any(|removal| removal == path),
                "{path} must never be a candidate; plan named {named:?}"
            );
        }

        clean_output(dir.path(), true);
        for (path, expected) in untouchable.iter().zip(&before) {
            assert_eq!(
                &fs::read(dir.path().join(path)).unwrap(),
                expected,
                "{path} must survive byte-identical"
            );
        }
    }

    /// An in-flight ticket's notes are live state. So are the notes of a ticket
    /// the board says nothing about at all.
    #[test]
    fn an_unfinished_tickets_notes_are_never_candidates() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());

        let plan = plan_clean_actions(dir.path());
        let named = relative_removals(dir.path(), &plan);
        let keeps: Vec<String> = plan
            .iter()
            .filter(|action| !action.verb.is_removal())
            .map(|action| action.line(dir.path()))
            .collect();

        for unfinished in ["T-024-02", "T-024-03", "T-024-99"] {
            for name in RETIRED_WORKFLOW_NOTES {
                let path = format!("docs/active/work/{unfinished}/{name}");
                assert!(
                    !named.iter().any(|removal| removal == &path),
                    "{path} belongs to a ticket that is not done"
                );
                assert!(dir.path().join(&path).exists());
            }
            assert!(
                keeps.iter().any(|line| line.contains(unfinished)),
                "clean must say why it left {unfinished} alone; it said {keeps:?}"
            );
        }

        // Each refusal names what the board actually records, so the operator can
        // act on it rather than guess.
        let joined = keeps.join("\n");
        assert!(joined.contains("records T-024-02 as open"), "{joined}");
        assert!(joined.contains("records T-024-03 as review"), "{joined}");
        assert!(
            joined.contains("nothing on your board records T-024-99 finished"),
            "{joined}"
        );

        // And the unfinished ticket's attempt folder is live too.
        assert!(dir
            .path()
            .join(".lisa/attempts/T-024-02/1/work/assignment.md")
            .exists());
    }

    /// A done ticket whose work directory holds nothing but retired notes loses
    /// the directory too — predicted at plan time, never discovered afterwards.
    #[test]
    fn a_work_directory_with_nothing_left_in_it_goes_as_well() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());

        let plan = plan_clean_actions(dir.path());
        assert!(
            plan.iter()
                .any(|action| action.verb == CleanVerb::RemoveEmptyDir
                    && display_relative(dir.path(), &action.path) == "docs/active/work/T-024-04"),
            "T-024-04 has nothing but notes, so its directory is empty afterwards"
        );
        assert!(
            !plan
                .iter()
                .any(|action| action.verb == CleanVerb::RemoveEmptyDir
                    && display_relative(dir.path(), &action.path) == "docs/active/work/T-024-01"),
            "T-024-01 keeps its review, its note and its harness, so the directory stays"
        );

        clean_output(dir.path(), true);
        assert!(!dir.path().join("docs/active/work/T-024-04").exists());
        assert!(dir.path().join("docs/active/work/T-024-01").is_dir());
    }

    /// Lisa does not follow a link out of the project to delete anything — not at
    /// a leaf, and not from inside a tree it was about to remove whole.
    #[test]
    fn a_symlink_out_of_the_project_is_refused() {
        let outside = tempfile::tempdir().unwrap();
        write(outside.path().join("secret.md"), "Not Lisa's.\n");
        write(outside.path().join("vault/keep.md"), "Also not Lisa's.\n");

        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());
        fs::remove_file(dir.path().join("docs/active/work/T-024-01/plan.md")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.md"),
            dir.path().join("docs/active/work/T-024-01/plan.md"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("vault"),
            dir.path().join(".lisa/attempts/T-024-01/vault"),
        )
        .unwrap();

        let plan = plan_clean_actions(dir.path());
        let named = relative_removals(dir.path(), &plan);
        assert!(
            !named
                .iter()
                .any(|path| path == "docs/active/work/T-024-01/plan.md"),
            "a symlinked note is refused; plan named {named:?}"
        );
        assert!(
            !named.iter().any(|path| path == ".lisa/attempts/T-024-01"),
            "a tree with a symlink in it is refused whole; plan named {named:?}"
        );
        let refusals: String = plan
            .iter()
            .filter(|action| !action.verb.is_removal())
            .map(|action| action.line(dir.path()))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            refusals.matches("is a symlink").count(),
            2,
            "both refusals must say why, in the preview:\n{refusals}"
        );

        clean_output(dir.path(), true);
        assert_eq!(
            fs::read_to_string(outside.path().join("secret.md")).unwrap(),
            "Not Lisa's.\n"
        );
        assert_eq!(
            fs::read_to_string(outside.path().join("vault/keep.md")).unwrap(),
            "Also not Lisa's.\n"
        );
        assert!(dir.path().join(".lisa/attempts/T-024-01/vault").exists());
    }

    /// What `lisa init` just wrote cannot already be litter.
    #[test]
    fn a_fresh_init_project_has_nothing_to_remove() {
        let dir = tempfile::tempdir().unwrap();
        crate::init::run_init(dir.path(), false, crate::init::HistoryPreference::NoHistory)
            .expect("init must succeed in an empty directory");
        let before = tree_snapshot(dir.path());

        assert_eq!(plan_clean_actions(dir.path()), Vec::new());
        assert_eq!(clean_output(dir.path(), false), "Nothing to remove.\n");
        assert_eq!(clean_output(dir.path(), true), "Nothing to remove.\n");
        assert_eq!(tree_snapshot(dir.path()), before);
    }

    /// The promise `lisa doctor` has been making since T-057-02-01: every finding
    /// that names `lisa clean` is something `lisa clean` actually removes — and
    /// nothing clean removes as currency is unreported.
    #[test]
    fn every_finding_that_names_clean_is_a_removal_in_cleans_plan() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());
        // An edited retired workflow document, and a byte-exact generated
        // CLAUDE.md with a hand-written AGENTS.md pointing at it: the two cases
        // init reports and preserves.
        write(
            dir.path().join("docs/knowledge/rdspi-workflow.md"),
            &format!(
                "{}\n\nOur team's note.\n",
                crate::templates::LEGACY_WORKFLOWS[2]
            ),
        );
        write(dir.path().join("CLAUDE.md"), &generated_claude_md());
        write(
            dir.path().join("AGENTS.md"),
            "# AGENTS.md\n\nRead CLAUDE.md first.\n",
        );

        let currency = inventory(dir.path());
        let named_clean: Vec<&str> = currency
            .findings
            .iter()
            .filter(|finding| finding.remedy == Remedy::Clean)
            .map(|finding| finding.subject.as_str())
            .collect();
        assert_eq!(
            named_clean,
            vec!["CLAUDE.md", "docs/knowledge/rdspi-workflow.md"],
            "the fixture must actually produce the findings under test"
        );

        let plan = plan_clean_actions(dir.path());
        let currency_removals: Vec<String> = plan
            .iter()
            .filter(|action| action.class == Some(CleanClass::Currency))
            .map(|action| display_relative(dir.path(), &action.path))
            .collect();

        for subject in &named_clean {
            assert!(
                currency_removals.iter().any(|path| path == subject),
                "doctor tells the operator to run `lisa clean` for {subject}, and clean must \
                 remove it; it planned {currency_removals:?}"
            );
        }
        for removal in &currency_removals {
            assert!(
                named_clean.contains(&removal.as_str()),
                "clean removes {removal} as currency, so doctor must have reported it"
            );
        }

        clean_output(dir.path(), true);
        assert!(!dir.path().join("docs/knowledge/rdspi-workflow.md").exists());
        assert!(!dir.path().join("CLAUDE.md").exists());
        // The operator's own AGENTS.md stays — pointing at nothing, which the
        // plan line said out loud before it happened.
        assert_eq!(
            fs::read_to_string(dir.path().join("AGENTS.md")).unwrap(),
            "# AGENTS.md\n\nRead CLAUDE.md first.\n"
        );
    }

    /// Clean removes files; it does not rewrite the contents of one. A dead
    /// setting Lisa cannot lift out surgically is therefore the operator's edit,
    /// and doctor must say so rather than naming a command that would decline.
    #[test]
    fn a_config_key_lisa_cannot_lift_out_is_the_operators_edit_not_cleans() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path().join(".lisa.toml"),
            &format!(
                "version = \"{}\"\nscheduling = {{ auto_advance = true, max_threads = 2 }}\n",
                config::LISA_VERSION
            ),
        );

        let currency = inventory(dir.path());
        let finding = currency
            .findings
            .iter()
            .find(|finding| finding.subject.contains("auto_advance"))
            .expect("an inline dead key is still reported");
        match &finding.remedy {
            Remedy::Operator(edit) => {
                assert!(edit.contains("auto_advance"), "{edit}");
                assert!(
                    edit.contains("yourself") || edit.contains("Delete"),
                    "the remedy has to name the edit: {edit}"
                );
            }
            other => panic!("a key clean cannot remove must not name clean: {other:?}"),
        }

        // And clean has nothing to say about it at all.
        assert_eq!(plan_clean_actions(dir.path()), Vec::new());
        let before = fs::read_to_string(dir.path().join(".lisa.toml")).unwrap();
        clean_output(dir.path(), true);
        assert_eq!(
            fs::read_to_string(dir.path().join(".lisa.toml")).unwrap(),
            before,
            "clean never edits a file's contents"
        );
    }

    /// Voice: every line says why, and the summary alone is enough to decide on.
    #[test]
    fn every_removal_line_says_why_and_the_summary_stands_alone() {
        let dir = tempfile::tempdir().unwrap();
        litter_fixture(dir.path());

        let plan = plan_clean_actions(dir.path());
        for action in &plan {
            assert!(
                !action.reason.trim().is_empty(),
                "{} has no reason",
                action.path.display()
            );
            let line = action.line(dir.path());
            assert!(line.contains(&action.reason), "{line}");
            if !action.verb.is_removal() {
                assert!(
                    action.reason.starts_with("preserved: "),
                    "a refusal keeps init's prefix: {line}"
                );
            }
        }

        let summary = summary_line(&plan);
        assert!(summary.contains("files"), "{summary}");
        assert!(summary.contains("folders"), "{summary}");
        assert!(summary.contains("retired workflow notes"), "{summary}");
        assert!(summary.contains("finished attempt folders"), "{summary}");
        assert!(summary.contains("left alone"), "{summary}");
        assert!(
            !summary.contains('\n'),
            "the summary is one line: {summary}"
        );

        // The standing statement about what is never a candidate.
        let output = clean_output(dir.path(), false);
        assert!(
            output.contains(
                "Never a candidate: your board (docs/active/tickets/, docs/active/stories/)"
            ),
            "{output}"
        );
    }

    /// **End to end, closing the story.** A project 0.4.4 left behind becomes
    /// fully current through one `lisa init` and one `lisa clean --remove`, and
    /// every file a person wrote in it is still there byte-identical.
    #[test]
    fn the_0_4_4_fixture_ends_current_and_every_human_file_survives() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // --- what an older Lisa wrote ---
        write(
            root.join(".lisa.toml"),
            "# Two agents is all this laptop can take.\nversion = \"0.4.0\"\n\n[scheduling]\nmax_threads = 2\n# Left over from 0.4 — nobody remembers turning it on.\nauto_advance = true\n",
        );
        // Edited since Lisa wrote it, so init preserves it and clean removes it.
        write(
            root.join("docs/knowledge/rdspi-workflow.md"),
            &format!(
                "{}\n\nOur team's note about phase four.\n",
                crate::templates::LEGACY_WORKFLOWS[2]
            ),
        );
        write(
            root.join(".lisa/hooks/on-stop.sh"),
            crate::templates::LEGACY_ON_STOP_HOOKS[0],
        );
        // A byte-exact generation, with the operator's own AGENTS.md pointing at
        // it: init preserves it for that reason, and clean is what removes it.
        write(root.join("CLAUDE.md"), &generated_claude_md());
        for id in ["T-024-01", "T-024-02"] {
            retired_notes(root, id);
            attempt(root, id);
        }

        // --- what a person wrote ---
        let human: Vec<(&str, String)> = vec![
            (
                "README.md",
                "# my-app\n\nRuns the climate suite.\n".to_string(),
            ),
            (
                "AGENTS.md",
                "# AGENTS.md\n\nRead CLAUDE.md first, then the README.\n".to_string(),
            ),
            (
                "docs/knowledge/our-notes.md",
                "Our own notes. Nothing to do with Lisa.\n".to_string(),
            ),
            (
                "docs/active/tickets/T-024-01.md",
                ticket("T-024-01", "done"),
            ),
            (
                "docs/active/tickets/T-024-02.md",
                ticket("T-024-02", "open"),
            ),
            (
                "docs/active/stories/S-024.md",
                "---\nid: S-024\ntitle: climate\nstatus: done\n---\n\nThe story.\n".to_string(),
            ),
            (
                "docs/active/work/T-024-01/review.md",
                "# Review\n\nWhat changed, for a human.\n".to_string(),
            ),
            (
                "docs/active/work/T-024-01/operator-note.md",
                "Ask Priya about the cache before touching this again.\n".to_string(),
            ),
            (
                "docs/active/work/T-024-01/harness/probe.sh",
                "#!/bin/sh\necho probe\n".to_string(),
            ),
        ];
        for (path, content) in &human {
            write(root.join(path), content);
        }

        // --- the upgrade, exactly as an operator would run it ---
        crate::init::run_init(root, false, crate::init::HistoryPreference::NoHistory).unwrap();
        let preview = clean_output(root, false);
        assert!(
            preview.contains("docs/knowledge/rdspi-workflow.md")
                && preview.contains("CLAUDE.md")
                && preview.contains("docs/active/work/T-024-01/research.md")
                && preview.contains(".lisa/attempts/T-024-01/"),
            "the preview must name everything about to go:\n{preview}"
        );
        let removed = clean_output(root, true);
        assert!(removed.contains("Removed "), "{removed}");

        // --- `lisa doctor` reports the project fully current ---
        let currency = inventory(root);
        assert!(
            currency.is_current(),
            "one init and one clean must leave nothing behind: {:#?}",
            currency.findings
        );

        // --- and every file a person wrote is byte-identical ---
        for (path, content) in &human {
            assert_eq!(
                fs::read_to_string(root.join(path)).unwrap(),
                *content,
                "{path} must survive the whole upgrade byte-identical"
            );
        }
        // Including the open ticket's notes, and its attempt folder.
        for name in RETIRED_WORKFLOW_NOTES {
            assert!(root
                .join(format!("docs/active/work/T-024-02/{name}"))
                .exists());
        }
        assert!(root
            .join(".lisa/attempts/T-024-02/1/work/assignment.md")
            .exists());

        // What did go: the retired document, the generated context file, the done
        // ticket's notes, and its attempt folder.
        assert!(!root.join("docs/knowledge/rdspi-workflow.md").exists());
        assert!(!root.join("CLAUDE.md").exists());
        assert!(!root.join(".lisa/attempts/T-024-01").exists());
        for name in RETIRED_WORKFLOW_NOTES {
            assert!(!root
                .join(format!("docs/active/work/T-024-01/{name}"))
                .exists());
        }

        // A second clean has nothing left to remove. It still says what it left
        // alone — the open ticket's work — because that is the question an
        // operator running it twice is asking.
        let again = clean_output(root, false);
        assert!(again.starts_with("Nothing to remove.\n"), "{again}");
        assert!(!again.contains("  remove  "), "{again}");
        assert!(again.contains("records T-024-02 as open"), "{again}");
    }
}
