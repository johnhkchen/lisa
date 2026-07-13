//! Provider-neutral, isolated Git transaction for ticket completion.
//!
//! Ticket content is staged in an alternate index. The ordinary index is used
//! only to snapshot foreign staged entries and, after `HEAD` advances, to
//! reconcile the exact committed paths to the new tree.

use fs2::FileExt;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static INDEX_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct CommitTransactionRequest {
    pub repo_root: PathBuf,
    pub ticket_id: String,
    pub message: String,
    pub includes: Vec<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct CompleteTicketRequest {
    pub repo_root: PathBuf,
    pub ticket_id: String,
    pub message: String,
    pub ticket_file: PathBuf,
    pub work_dir: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommitTransactionResult {
    pub commit_id: String,
    pub committed_paths: Vec<PathBuf>,
    previous_commit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommitTransactionError(String);

impl CommitTransactionError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CommitTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CommitTransactionError {}

struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
}

impl Repository {
    fn discover(requested_root: &Path) -> Result<Self, CommitTransactionError> {
        let requested_root = requested_root.canonicalize().map_err(|e| {
            CommitTransactionError::new(format!(
                "cannot resolve repository path {}: {e}",
                requested_root.display()
            ))
        })?;

        let root = run_git_at(
            &requested_root,
            None,
            "discover repository root",
            [OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
        )?;
        let root = output_string("discover repository root", &root)?;
        let root = PathBuf::from(root).canonicalize().map_err(|e| {
            CommitTransactionError::new(format!("cannot resolve Git repository root: {e}"))
        })?;

        let git_dir = run_git_at(
            &root,
            None,
            "discover Git directory",
            [OsStr::new("rev-parse"), OsStr::new("--absolute-git-dir")],
        )?;
        let git_dir = PathBuf::from(output_string("discover Git directory", &git_dir)?);

        Ok(Self { root, git_dir })
    }

    fn git<I, S>(
        &self,
        alternate_index: Option<&Path>,
        operation: &str,
        args: I,
    ) -> Result<Output, CommitTransactionError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        run_git_at(&self.root, alternate_index, operation, args)
    }
}

fn run_git_at<I, S>(
    root: &Path,
    alternate_index: Option<&Path>,
    operation: &str,
    args: I,
) -> Result<Output, CommitTransactionError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    if let Some(index) = alternate_index {
        command.env("GIT_INDEX_FILE", index);
    }

    let output = command.output().map_err(|e| {
        CommitTransactionError::new(format!(
            "failed to run Git while attempting to {operation}: {e}"
        ))
    })?;
    if output.status.success() {
        Ok(output)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(CommitTransactionError::new(format!(
            "Git failed to {operation} (status {}): {}",
            output.status,
            if stderr.is_empty() {
                "no stderr output"
            } else {
                &stderr
            }
        )))
    }
}

fn output_string(operation: &str, output: &Output) -> Result<String, CommitTransactionError> {
    let value = std::str::from_utf8(&output.stdout).map_err(|e| {
        CommitTransactionError::new(format!("Git output for {operation} was not UTF-8: {e}"))
    })?;
    let value = value.trim();
    if value.is_empty() {
        return Err(CommitTransactionError::new(format!(
            "Git returned no output while attempting to {operation}"
        )));
    }
    Ok(value.to_string())
}

fn normalize_includes(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, CommitTransactionError> {
    let mut normalized = BTreeSet::new();
    for path in paths {
        if path.as_os_str().is_empty() {
            return Err(CommitTransactionError::new(
                "ticket include path must not be empty",
            ));
        }
        if path.is_absolute() {
            return Err(CommitTransactionError::new(format!(
                "ticket include path must be repository-relative: {}",
                path.display()
            )));
        }

        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(value) => clean.push(value),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(CommitTransactionError::new(format!(
                        "ticket include path may not escape the repository: {}",
                        path.display()
                    )));
                }
            }
        }
        if clean.as_os_str().is_empty() {
            return Err(CommitTransactionError::new(
                "ticket include path may not select the whole repository",
            ));
        }
        normalized.insert(clean);
    }

    if normalized.is_empty() {
        return Err(CommitTransactionError::new(
            "at least one ticket include path is required",
        ));
    }
    Ok(normalized.into_iter().collect())
}

/// Resolve a path supplied relative to the caller's requested project root,
/// then express it relative to the enclosing Git repository root.
///
/// Lisa projects may live below the repository root (for example,
/// `games/midsummer`). Git pathspecs are always repository-relative, so passing
/// the project-relative `docs/...` path through unchanged would select the
/// wrong location after repository discovery.
fn completion_repo_relative_path(
    repo: &Repository,
    requested_root: &Path,
    path: PathBuf,
    description: &str,
) -> Result<PathBuf, CommitTransactionError> {
    let path = normalize_includes(vec![path])?
        .pop()
        .ok_or_else(|| CommitTransactionError::new(format!("{description} path is required")))?;
    let requested_root = requested_root.canonicalize().map_err(|e| {
        CommitTransactionError::new(format!(
            "cannot resolve requested project root {}: {e}",
            requested_root.display()
        ))
    })?;
    let absolute = requested_root.join(&path).canonicalize().map_err(|e| {
        CommitTransactionError::new(format!(
            "cannot resolve {description} {} from project root {}: {e}",
            path.display(),
            requested_root.display()
        ))
    })?;
    absolute
        .strip_prefix(&repo.root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            CommitTransactionError::new(format!(
                "{description} {} is outside Git repository {}",
                absolute.display(),
                repo.root.display()
            ))
        })
}

fn has_done_frontmatter(content: &str) -> bool {
    let trimmed = content.trim_start();
    let Some(after_opening) = trimmed.strip_prefix("---") else {
        return false;
    };
    let Some(closing) = after_opening.find("\n---") else {
        return false;
    };
    let frontmatter = &after_opening[..closing];
    let mut phase_done = false;
    let mut status_done = false;
    for line in frontmatter.lines() {
        let line = line.trim();
        phase_done |= line == "phase: done";
        status_done |= line == "status: done";
    }
    phase_done && status_done
}

struct TransactionLock {
    file: File,
    path: PathBuf,
    locked: bool,
}

impl TransactionLock {
    fn acquire(root: &Path) -> Result<Self, CommitTransactionError> {
        let path = root.join(".lisa-commit.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                CommitTransactionError::new(format!(
                    "cannot open commit transaction lock {}: {e}",
                    path.display()
                ))
            })?;
        file.try_lock_exclusive().map_err(|e| {
            CommitTransactionError::new(format!(
                "cannot acquire commit transaction lock {}: {e}",
                path.display()
            ))
        })?;
        Ok(Self {
            file,
            path,
            locked: true,
        })
    }

    fn finish(&mut self) -> Result<(), CommitTransactionError> {
        if self.locked {
            FileExt::unlock(&self.file).map_err(|e| {
                CommitTransactionError::new(format!(
                    "cannot release commit transaction lock {}: {e}",
                    self.path.display()
                ))
            })?;
            self.locked = false;
        }
        Ok(())
    }
}

impl Drop for TransactionLock {
    fn drop(&mut self) {
        if self.locked {
            let _ = FileExt::unlock(&self.file);
        }
    }
}

struct AlternateIndex {
    path: PathBuf,
    cleaned: bool,
}

impl AlternateIndex {
    fn reserve(git_dir: &Path) -> Result<Self, CommitTransactionError> {
        for _ in 0..100 {
            let sequence = INDEX_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = git_dir.join(format!(
                "lisa-ticket-index-{}-{sequence}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => {
                    fs::remove_file(&path).map_err(|e| {
                        CommitTransactionError::new(format!(
                            "cannot prepare alternate Git index {}: {e}",
                            path.display()
                        ))
                    })?;
                    return Ok(Self {
                        path,
                        cleaned: false,
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(CommitTransactionError::new(format!(
                        "cannot reserve alternate Git index in {}: {e}",
                        git_dir.display()
                    )));
                }
            }
        }
        Err(CommitTransactionError::new(format!(
            "cannot reserve a unique alternate Git index in {}",
            git_dir.display()
        )))
    }

    fn cleanup(&mut self) -> Result<(), CommitTransactionError> {
        let mut errors = Vec::new();
        for path in [
            &self.path,
            &PathBuf::from(format!("{}.lock", self.path.display())),
        ] {
            if let Err(e) = fs::remove_file(path) {
                if e.kind() != io::ErrorKind::NotFound {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
        }
        if errors.is_empty() {
            self.cleaned = true;
            Ok(())
        } else {
            Err(CommitTransactionError::new(format!(
                "cannot clean alternate Git index: {}",
                errors.join("; ")
            )))
        }
    }
}

impl Drop for AlternateIndex {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StagedSnapshot {
    paths: Vec<PathBuf>,
    entries: Vec<u8>,
}

fn parse_nul_paths(bytes: &[u8], operation: &str) -> Result<Vec<PathBuf>, CommitTransactionError> {
    let mut paths = Vec::new();
    for raw in bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
    {
        let path = std::str::from_utf8(raw).map_err(|e| {
            CommitTransactionError::new(format!(
                "Git path output for {operation} was not UTF-8: {e}"
            ))
        })?;
        paths.push(PathBuf::from(path));
    }
    paths.sort();
    Ok(paths)
}

fn staged_paths(
    repo: &Repository,
    alternate_index: Option<&Path>,
) -> Result<Vec<PathBuf>, CommitTransactionError> {
    let output = repo.git(
        alternate_index,
        "list staged paths",
        [
            OsStr::new("diff"),
            OsStr::new("--cached"),
            OsStr::new("--name-only"),
            OsStr::new("-z"),
        ],
    )?;
    parse_nul_paths(&output.stdout, "list staged paths")
}

fn staged_snapshot(repo: &Repository) -> Result<StagedSnapshot, CommitTransactionError> {
    let paths = staged_paths(repo, None)?;
    if paths.is_empty() {
        return Ok(StagedSnapshot {
            paths,
            entries: Vec::new(),
        });
    }

    let mut args: Vec<&OsStr> = vec![
        OsStr::new("ls-files"),
        OsStr::new("--stage"),
        OsStr::new("-z"),
        OsStr::new("--"),
    ];
    args.extend(paths.iter().map(|path| path.as_os_str()));
    let entries = repo.git(None, "snapshot staged entries", args)?.stdout;
    Ok(StagedSnapshot { paths, entries })
}

fn run_transaction_body(
    repo: &Repository,
    request: &CommitTransactionRequest,
    includes: &[PathBuf],
    alternate_index: &Path,
) -> Result<CommitTransactionResult, CommitTransactionError> {
    let head = repo.git(
        None,
        "resolve HEAD",
        [
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new("HEAD"),
        ],
    )?;
    let old_head = output_string("resolve HEAD", &head)?;
    let original_staged = staged_snapshot(repo)?;

    repo.git(
        Some(alternate_index),
        "initialize alternate index from HEAD",
        [OsStr::new("read-tree"), OsStr::new(&old_head)],
    )?;

    let mut add_args: Vec<&OsStr> = vec![OsStr::new("add"), OsStr::new("-A"), OsStr::new("--")];
    add_args.extend(includes.iter().map(|path| path.as_os_str()));
    repo.git(Some(alternate_index), "stage ticket-owned paths", add_args)?;

    let committed_paths = staged_paths(repo, Some(alternate_index))?;
    if committed_paths.is_empty() {
        return Err(CommitTransactionError::new(format!(
            "ticket {} has no changes in the requested include paths",
            request.ticket_id
        )));
    }

    let ordinary: BTreeSet<&PathBuf> = original_staged.paths.iter().collect();
    let overlap: Vec<String> = committed_paths
        .iter()
        .filter(|path| ordinary.contains(path))
        .map(|path| path.display().to_string())
        .collect();
    if !overlap.is_empty() {
        return Err(CommitTransactionError::new(format!(
            "ticket {} overlaps paths already staged in the ordinary index: {}",
            request.ticket_id,
            overlap.join(", ")
        )));
    }

    let tree = repo.git(
        Some(alternate_index),
        "write ticket tree",
        [OsStr::new("write-tree")],
    )?;
    let tree = output_string("write ticket tree", &tree)?;
    let commit = repo.git(
        Some(alternate_index),
        "create ticket commit",
        [
            OsStr::new("commit-tree"),
            OsStr::new(&tree),
            OsStr::new("-p"),
            OsStr::new(&old_head),
            OsStr::new("-m"),
            OsStr::new(&request.message),
        ],
    )?;
    let commit_id = output_string("create ticket commit", &commit)?;

    repo.git(
        None,
        "advance HEAD to ticket commit",
        [
            OsStr::new("update-ref"),
            OsStr::new("HEAD"),
            OsStr::new(&commit_id),
            OsStr::new(&old_head),
        ],
    )?;

    let mut reset_args: Vec<&OsStr> = vec![
        OsStr::new("reset"),
        OsStr::new("--quiet"),
        OsStr::new("HEAD"),
        OsStr::new("--"),
    ];
    reset_args.extend(committed_paths.iter().map(|path| path.as_os_str()));
    if let Err(error) = repo.git(
        None,
        "reconcile committed paths in ordinary index",
        reset_args,
    ) {
        return Err(rollback_after_ref_advance(
            repo,
            &old_head,
            &commit_id,
            &committed_paths,
            error,
        ));
    }

    let final_staged = match staged_snapshot(repo) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(rollback_after_ref_advance(
                repo,
                &old_head,
                &commit_id,
                &committed_paths,
                error,
            ));
        }
    };
    if final_staged != original_staged {
        return Err(rollback_after_ref_advance(
            repo,
            &old_head,
            &commit_id,
            &committed_paths,
            CommitTransactionError::new("ordinary staged entries changed during verification"),
        ));
    }

    Ok(CommitTransactionResult {
        commit_id,
        committed_paths,
        previous_commit_id: old_head,
    })
}

fn rollback_advanced_head(
    repo: &Repository,
    old_head: &str,
    commit_id: &str,
    committed_paths: &[PathBuf],
) -> Result<(), CommitTransactionError> {
    repo.git(
        None,
        "roll back failed ticket commit",
        [
            OsStr::new("update-ref"),
            OsStr::new("HEAD"),
            OsStr::new(old_head),
            OsStr::new(commit_id),
        ],
    )?;
    let mut reset_args: Vec<&OsStr> = vec![
        OsStr::new("reset"),
        OsStr::new("--quiet"),
        OsStr::new("HEAD"),
        OsStr::new("--"),
    ];
    reset_args.extend(committed_paths.iter().map(|path| path.as_os_str()));
    repo.git(
        None,
        "reconcile ticket paths after commit rollback",
        reset_args,
    )?;
    Ok(())
}

fn rollback_after_ref_advance(
    repo: &Repository,
    old_head: &str,
    commit_id: &str,
    committed_paths: &[PathBuf],
    primary: CommitTransactionError,
) -> CommitTransactionError {
    match rollback_advanced_head(repo, old_head, commit_id, committed_paths) {
        Ok(()) => CommitTransactionError::new(format!(
            "ticket commit {commit_id} was rolled back after failure: {primary}"
        )),
        Err(rollback) => CommitTransactionError::new(format!(
            "ticket commit {commit_id} advanced HEAD and {primary}; rollback also failed: {rollback}"
        )),
    }
}

pub(crate) fn commit_ticket(
    request: CommitTransactionRequest,
) -> Result<CommitTransactionResult, CommitTransactionError> {
    if request.ticket_id.trim().is_empty() {
        return Err(CommitTransactionError::new("ticket ID must not be empty"));
    }
    if request.message.trim().is_empty() {
        return Err(CommitTransactionError::new(
            "commit message must not be empty",
        ));
    }
    let includes = normalize_includes(request.includes.clone())?;
    let repo = Repository::discover(&request.repo_root)?;
    let mut lock = TransactionLock::acquire(&repo.root)?;
    let mut alternate_index = match AlternateIndex::reserve(&repo.git_dir) {
        Ok(index) => index,
        Err(primary) => {
            return match lock.finish() {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(CommitTransactionError::new(format!(
                    "{primary}; cleanup also failed: {cleanup}"
                ))),
            };
        }
    };

    let mut primary = run_transaction_body(&repo, &request, &includes, &alternate_index.path);
    let index_cleanup = alternate_index.cleanup();
    let unlock = lock.finish();

    let mut cleanup_errors = Vec::new();
    if let Err(error) = index_cleanup {
        cleanup_errors.push(error.to_string());
    }
    if let Err(error) = unlock {
        cleanup_errors.push(error.to_string());
    }

    if !cleanup_errors.is_empty() {
        if let Ok(result) = &primary {
            let cleanup = CommitTransactionError::new(format!(
                "transaction cleanup failed: {}",
                cleanup_errors.join("; ")
            ));
            primary = Err(rollback_after_ref_advance(
                &repo,
                &result.previous_commit_id,
                &result.commit_id,
                &result.committed_paths,
                cleanup,
            ));
            cleanup_errors.clear();
        }
    }

    match (primary, cleanup_errors.is_empty()) {
        (Ok(result), true) => Ok(result),
        (Ok(_), false) => Err(CommitTransactionError::new(format!(
            "ticket transaction completed but cleanup failed: {}",
            cleanup_errors.join("; ")
        ))),
        (Err(error), true) => Err(error),
        (Err(error), false) => Err(CommitTransactionError::new(format!(
            "{error}; cleanup also failed: {}",
            cleanup_errors.join("; ")
        ))),
    }
}

/// Prepare both completion frontmatter fields and commit the loop-owned ticket
/// paths through the isolated transaction.
///
/// If the transaction does not report success, the exact original ticket bytes
/// are restored before the error is returned.
pub(crate) fn complete_ticket(
    request: CompleteTicketRequest,
) -> Result<CommitTransactionResult, CommitTransactionError> {
    let repo = Repository::discover(&request.repo_root)?;
    let ticket_file = completion_repo_relative_path(
        &repo,
        &request.repo_root,
        request.ticket_file,
        "completion ticket file",
    )?;
    if ticket_file.extension() != Some(OsStr::new("md")) {
        return Err(CommitTransactionError::new(
            "completion ticket file must be a Markdown path",
        ));
    }
    let work_dir = completion_repo_relative_path(
        &repo,
        &request.repo_root,
        request.work_dir,
        "completion work directory",
    )?;
    if ticket_file == work_dir {
        return Err(CommitTransactionError::new(
            "completion ticket file and work directory must be distinct",
        ));
    }
    let includes = vec![ticket_file.clone(), work_dir];
    let ticket_path = repo.root.join(&ticket_file);
    let original = fs::read(&ticket_path).map_err(|e| {
        CommitTransactionError::new(format!(
            "cannot read completion ticket {}: {e}",
            ticket_path.display()
        ))
    })?;

    let already_done = std::str::from_utf8(&original).is_ok_and(has_done_frontmatter);
    if already_done {
        let mut status_args: Vec<&OsStr> = vec![
            OsStr::new("status"),
            OsStr::new("--porcelain"),
            OsStr::new("-z"),
            OsStr::new("--"),
        ];
        status_args.extend(includes.iter().map(|path| path.as_os_str()));
        let status = repo.git(None, "verify already completed ticket paths", status_args)?;
        if status.stdout.is_empty() {
            let head = repo.git(
                None,
                "verify already completed ticket commit",
                [
                    OsStr::new("rev-parse"),
                    OsStr::new("--verify"),
                    OsStr::new("HEAD"),
                ],
            )?;
            let commit_id = output_string("verify already completed ticket commit", &head)?;
            return Ok(CommitTransactionResult {
                previous_commit_id: commit_id.clone(),
                commit_id,
                committed_paths: Vec::new(),
            });
        }
    }

    lisa_core::ticket::update_ticket_done(&ticket_path).map_err(|e| {
        CommitTransactionError::new(format!(
            "cannot prepare completion frontmatter for {}: {e}",
            ticket_path.display()
        ))
    })?;

    let result = commit_ticket(CommitTransactionRequest {
        repo_root: repo.root,
        ticket_id: request.ticket_id,
        message: request.message,
        includes,
    });

    match result {
        Ok(result) => Ok(result),
        Err(primary) => match fs::write(&ticket_path, original) {
            Ok(()) => Err(primary),
            Err(rollback) => Err(CommitTransactionError::new(format!(
                "{primary}; restoring non-done ticket {} also failed: {rollback}",
                ticket_path.display()
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    struct GitRepo {
        temp: TempDir,
    }

    impl GitRepo {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let repo = Self { temp };
            repo.git(["init", "--quiet"]);
            repo.git(["config", "user.name", "Lisa Test"]);
            repo.git(["config", "user.email", "lisa@example.test"]);
            repo
        }

        fn root(&self) -> &Path {
            self.temp.path()
        }

        fn write(&self, path: &str, content: &str) {
            let path = self.root().join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }

        fn git<I, S>(&self, args: I) -> Output
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            let output = Command::new("git")
                .arg("-C")
                .arg(self.root())
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output
        }

        fn git_string<I, S>(&self, args: I) -> String
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            String::from_utf8(self.git(args).stdout)
                .unwrap()
                .trim()
                .to_string()
        }

        fn base_commit(&self) {
            self.git(["add", "-A"]);
            self.git(["commit", "--quiet", "-m", "base"]);
        }

        fn request(&self, includes: &[&str]) -> CommitTransactionRequest {
            CommitTransactionRequest {
                repo_root: self.root().to_path_buf(),
                ticket_id: "T-031-01".to_string(),
                message: "Complete T-031-01".to_string(),
                includes: includes.iter().map(PathBuf::from).collect(),
            }
        }
    }

    #[test]
    fn normalizes_and_deduplicates_includes() {
        assert_eq!(
            normalize_includes(vec![
                PathBuf::from("src/./lib.rs"),
                PathBuf::from("docs/work"),
                PathBuf::from("src/lib.rs"),
            ])
            .unwrap(),
            vec![PathBuf::from("docs/work"), PathBuf::from("src/lib.rs")]
        );
    }

    #[test]
    fn rejects_unsafe_includes() {
        for path in ["", ".", "..", "../outside", "src/../../outside"] {
            assert!(
                normalize_includes(vec![PathBuf::from(path)]).is_err(),
                "{path}"
            );
        }
        assert!(normalize_includes(vec![PathBuf::from("/absolute")]).is_err());
    }

    #[test]
    fn foreign_staged_entry_is_preserved_and_excluded() {
        let repo = GitRepo::new();
        repo.write("foreign.txt", "foreign base\n");
        repo.write("src/ticket.txt", "ticket base\n");
        repo.write(
            "docs/active/tickets/T-031-01.md",
            "---\nid: T-031-01\nphase: review\n---\n",
        );
        repo.base_commit();

        repo.write("foreign.txt", "foreign staged\n");
        repo.git(["add", "foreign.txt"]);
        let foreign_stage_before = repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"]);
        let old_head = repo.git_string(["rev-parse", "HEAD"]);

        repo.write("src/ticket.txt", "ticket committed\n");
        repo.write("src/new-ticket.txt", "new ticket code\n");
        repo.write(
            "docs/active/work/T-031-01/review.md",
            "# Review\nComplete.\n",
        );
        repo.write(
            "docs/active/tickets/T-031-01.md",
            "---\nid: T-031-01\nphase: done\n---\n",
        );
        repo.write("unrelated.txt", "must stay uncommitted\n");

        let result = commit_ticket(repo.request(&[
            "src/ticket.txt",
            "src/new-ticket.txt",
            "docs/active/work/T-031-01",
            "docs/active/tickets/T-031-01.md",
        ]))
        .unwrap();

        assert_ne!(result.commit_id, old_head);
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), result.commit_id);
        assert_eq!(
            repo.git_string(["show", "HEAD:src/ticket.txt"]),
            "ticket committed"
        );
        assert_eq!(
            repo.git_string(["show", "HEAD:src/new-ticket.txt"]),
            "new ticket code"
        );
        assert_eq!(
            repo.git_string(["show", "HEAD:docs/active/work/T-031-01/review.md"]),
            "# Review\nComplete."
        );
        assert_eq!(
            repo.git_string(["show", "HEAD:foreign.txt"]),
            "foreign base",
            "foreign staged content entered the ticket commit"
        );
        let unrelated_in_head = Command::new("git")
            .arg("-C")
            .arg(repo.root())
            .args(["cat-file", "-e", "HEAD:unrelated.txt"])
            .output()
            .unwrap();
        assert!(!unrelated_in_head.status.success());

        assert_eq!(
            repo.git_string(["diff", "--cached", "--name-only"]),
            "foreign.txt"
        );
        let foreign_stage_after = repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"]);
        assert_eq!(foreign_stage_after.stdout, foreign_stage_before.stdout);
        assert_eq!(
            repo.git_string(["diff", "--cached", "--", "src/ticket.txt"]),
            ""
        );

        let git_dir = repo.git_string(["rev-parse", "--absolute-git-dir"]);
        assert!(fs::read_dir(git_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("lisa-ticket-index-")));
    }

    #[test]
    fn staged_overlap_fails_without_moving_head_or_index() {
        let repo = GitRepo::new();
        repo.write("ticket.txt", "base\n");
        repo.base_commit();
        repo.write("ticket.txt", "already staged\n");
        repo.git(["add", "ticket.txt"]);
        let head = repo.git_string(["rev-parse", "HEAD"]);
        let stage = repo.git(["ls-files", "--stage", "-z", "--", "ticket.txt"]);

        let error = commit_ticket(repo.request(&["ticket.txt"]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("already staged"), "{error}");
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), head);
        assert_eq!(
            repo.git(["ls-files", "--stage", "-z", "--", "ticket.txt"])
                .stdout,
            stage.stdout
        );
    }

    #[test]
    fn held_lock_returns_actionable_error() {
        let repo = GitRepo::new();
        repo.write("ticket.txt", "base\n");
        repo.base_commit();
        repo.write("ticket.txt", "changed\n");

        let lock_path = repo.root().join(".lisa-commit.lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .unwrap();
        lock.try_lock_exclusive().unwrap();

        let error = commit_ticket(repo.request(&["ticket.txt"]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("cannot acquire commit transaction lock"),
            "{error}"
        );
        FileExt::unlock(&lock).unwrap();
    }

    #[test]
    fn unchanged_paths_fail_without_moving_head() {
        let repo = GitRepo::new();
        repo.write("ticket.txt", "base\n");
        repo.base_commit();
        let head = repo.git_string(["rev-parse", "HEAD"]);

        let error = commit_ticket(repo.request(&["ticket.txt"]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("has no changes"), "{error}");
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), head);
    }

    #[test]
    fn commit_failure_is_actionable_and_leaves_head_and_index_usable() {
        let repo = GitRepo::new();
        repo.write("ticket.txt", "base\n");
        repo.base_commit();
        repo.write("ticket.txt", "changed\n");
        repo.git(["config", "user.name", ""]);
        let head = repo.git_string(["rev-parse", "HEAD"]);

        let error = commit_ticket(repo.request(&["ticket.txt"]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("create ticket commit"), "{error}");
        assert!(error.contains("empty ident name"), "{error}");
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), head);
        assert_eq!(repo.git_string(["diff", "--cached", "--name-only"]), "");
        assert_eq!(
            repo.git_string(["status", "--short", "--", "ticket.txt"]),
            "M ticket.txt"
        );

        let git_dir = repo.git_string(["rev-parse", "--absolute-git-dir"]);
        assert!(fs::read_dir(git_dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("lisa-ticket-index-")
        }));
    }

    #[test]
    fn invalid_repository_is_actionable() {
        let temp = tempfile::tempdir().unwrap();
        let error = commit_ticket(CommitTransactionRequest {
            repo_root: temp.path().to_path_buf(),
            ticket_id: "T-031-01".to_string(),
            message: "message".to_string(),
            includes: vec![PathBuf::from("ticket.txt")],
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("discover repository root"), "{error}");
    }

    #[test]
    fn complete_ticket_commits_done_frontmatter_and_all_work_artifacts() {
        let repo = GitRepo::new();
        let ticket = "docs/active/tickets/T-031-02-gate-done-on-commit.md";
        let work = "docs/active/work/T-031-02";
        repo.write(
            ticket,
            "---\nid: T-031-02\nstatus: open\nphase: review\n---\nBody\n",
        );
        repo.write("foreign.txt", "foreign base\n");
        repo.base_commit();

        repo.write("foreign.txt", "foreign staged\n");
        repo.git(["add", "foreign.txt"]);
        let foreign_before = repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"]);
        for artifact in [
            "research.md",
            "design.md",
            "structure.md",
            "plan.md",
            "progress.md",
            "review.md",
        ] {
            repo.write(&format!("{work}/{artifact}"), &format!("# {artifact}\n"));
        }

        let result = complete_ticket(CompleteTicketRequest {
            repo_root: repo.root().to_path_buf(),
            ticket_id: "T-031-02".to_string(),
            message: "Complete T-031-02".to_string(),
            ticket_file: PathBuf::from(ticket),
            work_dir: PathBuf::from(work),
        })
        .unwrap();

        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), result.commit_id);
        let committed_ticket = repo.git_string(["show", &format!("HEAD:{ticket}")]);
        assert!(committed_ticket.contains("phase: done"));
        assert!(committed_ticket.contains("status: done"));
        for artifact in [
            "research.md",
            "design.md",
            "structure.md",
            "plan.md",
            "progress.md",
            "review.md",
        ] {
            assert_eq!(
                repo.git_string(["show", &format!("HEAD:{work}/{artifact}")]),
                format!("# {artifact}")
            );
        }
        assert_eq!(
            repo.git_string(["show", "HEAD:foreign.txt"]),
            "foreign base"
        );
        assert_eq!(
            repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"])
                .stdout,
            foreign_before.stdout
        );
    }

    #[test]
    fn complete_ticket_normalizes_nested_project_paths_to_git_root() {
        let repo = GitRepo::new();
        let project = repo.root().join("games/midsummer");
        let project_ticket = "docs/active/tickets/T-009-02-01.md";
        let project_work = "docs/active/work/T-009-02-01";
        let repo_ticket = format!("games/midsummer/{project_ticket}");
        let repo_work = format!("games/midsummer/{project_work}");

        repo.write(
            &repo_ticket,
            "---\nid: T-009-02-01\nstatus: open\nphase: review\n---\nBody\n",
        );
        repo.write("docs/root-sentinel.md", "root docs remain untouched\n");
        repo.base_commit();
        repo.write(&format!("{repo_work}/review.md"), "# Review\n");

        let result = complete_ticket(CompleteTicketRequest {
            repo_root: project,
            ticket_id: "T-009-02-01".to_string(),
            message: "Complete T-009-02-01".to_string(),
            ticket_file: PathBuf::from(project_ticket),
            work_dir: PathBuf::from(project_work),
        })
        .unwrap();

        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), result.commit_id);
        assert_eq!(
            result.committed_paths,
            vec![
                PathBuf::from(&repo_ticket),
                PathBuf::from(format!("{repo_work}/review.md")),
            ]
        );
        assert!(repo
            .git_string(["show", &format!("HEAD:{repo_ticket}")])
            .contains("phase: done"));
        assert_eq!(
            repo.git_string(["show", "HEAD:docs/root-sentinel.md"]),
            "root docs remain untouched"
        );
    }

    #[test]
    fn complete_ticket_failure_restores_exact_non_done_ticket() {
        let repo = GitRepo::new();
        let ticket = "docs/active/tickets/T-031-02.md";
        let original = "---\nid: T-031-02\nstatus: open\nphase: review\n---\nBody\n";
        repo.write(ticket, original);
        repo.write("docs/active/work/T-031-02/review.md", "# Review\n");
        repo.base_commit();
        repo.git(["config", "user.name", ""]);
        let head = repo.git_string(["rev-parse", "HEAD"]);

        let error = complete_ticket(CompleteTicketRequest {
            repo_root: repo.root().to_path_buf(),
            ticket_id: "T-031-02".to_string(),
            message: "Complete T-031-02".to_string(),
            ticket_file: PathBuf::from(ticket),
            work_dir: PathBuf::from("docs/active/work/T-031-02"),
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("create ticket commit"), "{error}");
        assert_eq!(
            fs::read(repo.root().join(ticket)).unwrap(),
            original.as_bytes()
        );
        assert_eq!(repo.git_string(["rev-parse", "HEAD"]), head);
    }

    #[test]
    fn compensating_rollback_restores_head_paths_and_foreign_stage() {
        let repo = GitRepo::new();
        repo.write("ticket.txt", "base\n");
        repo.write("foreign.txt", "foreign base\n");
        repo.base_commit();
        repo.write("foreign.txt", "foreign staged\n");
        repo.git(["add", "foreign.txt"]);
        let foreign_before = repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"]);
        repo.write("ticket.txt", "completed\n");

        let result = commit_ticket(repo.request(&["ticket.txt"])).unwrap();
        let context = Repository::discover(repo.root()).unwrap();
        rollback_advanced_head(
            &context,
            &result.previous_commit_id,
            &result.commit_id,
            &result.committed_paths,
        )
        .unwrap();

        assert_eq!(
            repo.git_string(["rev-parse", "HEAD"]),
            result.previous_commit_id
        );
        assert_eq!(repo.git_string(["show", "HEAD:ticket.txt"]), "base");
        assert_eq!(
            repo.git_string(["status", "--short", "--", "ticket.txt"]),
            "M ticket.txt"
        );
        assert_eq!(
            repo.git(["ls-files", "--stage", "-z", "--", "foreign.txt"])
                .stdout,
            foreign_before.stdout
        );
    }

    #[test]
    fn already_committed_done_ticket_returns_verified_head_without_new_commit() {
        let repo = GitRepo::new();
        let ticket = "docs/active/tickets/T-031-02.md";
        repo.write(
            ticket,
            "---\nid: T-031-02\ntitle: done\ntype: task\nstatus: done\npriority: high\nphase: done\n---\nBody\n",
        );
        repo.write("docs/active/work/T-031-02/review.md", "# Review\n");
        repo.base_commit();
        let head = repo.git_string(["rev-parse", "HEAD"]);

        let result = complete_ticket(CompleteTicketRequest {
            repo_root: repo.root().to_path_buf(),
            ticket_id: "T-031-02".to_string(),
            message: "Complete T-031-02".to_string(),
            ticket_file: PathBuf::from(ticket),
            work_dir: PathBuf::from("docs/active/work/T-031-02"),
        })
        .unwrap();

        assert_eq!(result.commit_id, head);
        assert!(result.committed_paths.is_empty());
        assert_eq!(repo.git_string(["rev-list", "--count", "HEAD"]), "1");
    }
}
