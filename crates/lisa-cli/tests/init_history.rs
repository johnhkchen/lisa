use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HISTORY_OFFER: &str = "Bring project history along? Finished work can be undone, and you'll have a record of what the agents did. [Y/n]";
const HISTORY_DECLINED: &str = "Continuing without project history: finished work will be recorded in Lisa's journal but won't be undoable.";
const HISTORY_KEPT: &str = "Keeping project history — finished work will be undoable.";

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
    global_config: PathBuf,
    empty_path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(name);
        let home = temp.path().join("home");
        let global_config = temp.path().join("global.gitconfig");
        let empty_path = temp.path().join("empty-path");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&empty_path).unwrap();
        fs::write(
            &global_config,
            "[user]\n\tname = Existing Global\n\temail = global@example.invalid\n",
        )
        .unwrap();
        Self {
            _temp: temp,
            root,
            home,
            global_config,
            empty_path,
        }
    }

    fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &self.global_config);
        command
    }

    fn lisa(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_lisa"))
            .args(args)
            .output()
            .unwrap()
    }

    fn lisa_without_git(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_lisa"))
            .env("PATH", &self.empty_path)
            .args(args)
            .output()
            .unwrap()
    }

    fn git(&self, root: &Path, args: &[&str]) -> Output {
        self.command("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap()
    }

    fn git_ok(&self, root: &Path, args: &[&str]) -> String {
        let output = self.git(root, args);
        assert_success(&output, &format!("git {}", args.join(" ")));
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn init_repository(&self, root: &Path) {
        let output = self
            .command("git")
            .args(["init", "--quiet"])
            .arg(root)
            .output()
            .unwrap();
        assert_success(&output, "git init");
    }
}

fn assert_success(output: &Output, action: &str) {
    assert!(
        output.status.success(),
        "{action} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output, action: &str) {
    assert!(
        !output.status.success(),
        "{action} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    fn visit(base: &Path, current: &Path, entries: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
        let mut children: Vec<_> = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        children.sort();
        for path in children {
            let relative = path.strip_prefix(base).unwrap().to_path_buf();
            if path.is_dir() {
                entries.push((relative, None));
                visit(base, &path, entries);
            } else {
                entries.push((relative, Some(fs::read(&path).unwrap())));
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries
}

fn write_status_ticket(root: &Path) {
    fs::write(
        root.join("docs/active/tickets/T-FIXTURE.md"),
        "---\nid: T-FIXTURE\ntitle: history fixture\ntype: task\nstatus: open\npriority: high\nphase: ready\n---\n\nFixture\n",
    )
    .unwrap();
}

#[test]
fn bare_folder_default_creates_commit_ready_project_history() {
    let fixture = Fixture::new("accepted-project");
    let global_before = fs::read(&fixture.global_config).unwrap();
    fs::write(fixture.root.join("existing.txt"), "operator work\n").unwrap();

    let output = fixture.lisa(&["init", "--path", fixture.root.to_str().unwrap()]);
    assert_success(&output, "bare lisa init");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(HISTORY_KEPT));
    assert!(fixture.root.join(".git").is_dir());
    assert_eq!(
        fixture.git_ok(&fixture.root, &["config", "--local", "user.name"]),
        "Lisa (project history)"
    );
    assert_eq!(
        fixture.git_ok(&fixture.root, &["config", "--local", "user.email"]),
        "lisa@project"
    );
    assert_eq!(fs::read(&fixture.global_config).unwrap(), global_before);

    let head = fixture.git_ok(&fixture.root, &["rev-parse", "--verify", "HEAD"]);
    assert_eq!(head.len(), 40);
    assert_eq!(
        fixture.git_ok(
            &fixture.root,
            &["show", "-s", "--format=%an|%ae|%s", "HEAD"]
        ),
        "Lisa (project history)|lisa@project|Start project history"
    );
    assert_eq!(
        fixture.git_ok(&fixture.root, &["ls-tree", "-r", "--name-only", "HEAD"]),
        "",
        "the root commit must not claim existing or scaffolded files"
    );

    fs::write(fixture.root.join("completion-marker.txt"), "finished\n").unwrap();
    let completion = fixture.lisa(&[
        "commit-ticket",
        "--path",
        fixture.root.to_str().unwrap(),
        "--ticket-id",
        "T-FIXTURE",
        "--message",
        "Complete fixture work",
        "--include",
        "completion-marker.txt",
    ]);
    assert_success(&completion, "completion-style commit");
    assert_ne!(fixture.git_ok(&fixture.root, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fixture.git_ok(
            &fixture.root,
            &["show", "--pretty=format:", "--name-only", "HEAD"]
        ),
        "completion-marker.txt"
    );

    write_status_ticket(&fixture.root);
    let status = fixture.lisa(&["status", "--path", fixture.root.to_str().unwrap()]);
    assert_success(&status, "lisa status");
    assert!(String::from_utf8(status.stdout)
        .unwrap()
        .contains("completion seal: commit-sealed — finished work lands as history"));
}

#[test]
fn bare_folder_decline_stays_journal_only_and_explains_the_consequence() {
    let fixture = Fixture::new("declined-project");
    let output = fixture.lisa(&[
        "init",
        "--path",
        fixture.root.to_str().unwrap(),
        "--no-history",
    ]);
    assert_success(&output, "lisa init --no-history");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(HISTORY_DECLINED));
    assert!(!fixture.root.join(".git").exists());

    write_status_ticket(&fixture.root);
    let status = fixture.lisa(&["status", "--path", fixture.root.to_str().unwrap()]);
    assert_success(&status, "lisa status");
    assert!(String::from_utf8(status.stdout)
        .unwrap()
        .contains("completion seal: journal-only — finished work is recorded but not undoable"));
}

#[test]
fn bare_folder_without_git_uses_journal_and_explains_the_consequence() {
    let fixture = Fixture::new("no-git-default");
    let output = fixture.lisa_without_git(&["init", "--path", fixture.root.to_str().unwrap()]);
    assert_success(&output, "bare lisa init without git");
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains(HISTORY_DECLINED));
    assert!(!fixture.root.join(".git").exists());
    assert!(fixture.root.join("CLAUDE.md").exists());

    write_status_ticket(&fixture.root);
    let status = fixture.lisa_without_git(&["status", "--path", fixture.root.to_str().unwrap()]);
    assert_success(&status, "lisa status without git");
    assert!(String::from_utf8(status.stdout)
        .unwrap()
        .contains("completion seal: journal-only — finished work is recorded but not undoable"));
}

#[test]
fn explicit_with_history_without_git_names_the_remedy() {
    let fixture = Fixture::new("no-git-explicit-history");
    let output = fixture.lisa_without_git(&[
        "init",
        "--path",
        fixture.root.to_str().unwrap(),
        "--with-history",
    ]);
    assert_failure(&output, "lisa init --with-history without git");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Git is not available"));
    assert!(stderr.contains("Install or repair Git"));
    assert!(stderr.contains("--with-history"));
    assert!(stderr.contains("--no-history"));
    assert!(!fixture.root.join(".git").exists());
    assert!(!fixture.root.join("CLAUDE.md").exists());
}

#[test]
fn history_flags_remain_overrides_and_offer_copy_is_plain() {
    let fixture = Fixture::new("flag-contract");

    let conflict = fixture.lisa(&[
        "init",
        "--path",
        fixture.root.to_str().unwrap(),
        "--with-history",
        "--no-history",
    ]);
    assert_failure(&conflict, "conflicting history flags");

    let dry_run = fixture.lisa(&[
        "init",
        "--dry-run",
        "--path",
        fixture.root.to_str().unwrap(),
    ]);
    assert_success(&dry_run, "history-offer dry run");
    let stdout = String::from_utf8(dry_run.stdout).unwrap();
    assert!(stdout.contains("Project history would be kept."));
    assert!(!stdout.contains(HISTORY_OFFER));
    assert!(!HISTORY_OFFER.to_ascii_lowercase().contains("git"));
    assert!(!fixture.root.join(".git").exists());
}

#[test]
fn folder_inside_born_repository_leaves_repository_metadata_and_config_unchanged() {
    let fixture = Fixture::new("parent-repository");
    fixture.init_repository(&fixture.root);
    fixture.git_ok(
        &fixture.root,
        &["config", "--local", "user.name", "Operator"],
    );
    fixture.git_ok(
        &fixture.root,
        &[
            "config",
            "--local",
            "user.email",
            "operator@example.invalid",
        ],
    );
    fixture.git_ok(
        &fixture.root,
        &["commit", "--quiet", "--allow-empty", "-m", "root"],
    );

    let nested = fixture.root.join("nested-project");
    fs::create_dir_all(&nested).unwrap();
    let repository_before = snapshot_tree(&fixture.root.join(".git"));
    let config_before = fs::read(fixture.root.join(".git/config")).unwrap();
    let global_before = fs::read(&fixture.global_config).unwrap();
    let head_before = fixture.git_ok(&fixture.root, &["rev-parse", "HEAD"]);

    let output = fixture.lisa(&["init", "--path", nested.to_str().unwrap(), "--with-history"]);
    assert_success(&output, "lisa init inside existing repository");
    assert!(!nested.join(".git").exists());
    assert_eq!(
        fs::read(fixture.root.join(".git/config")).unwrap(),
        config_before
    );
    assert_eq!(fs::read(&fixture.global_config).unwrap(), global_before);
    assert_eq!(
        fixture.git_ok(&fixture.root, &["rev-parse", "HEAD"]),
        head_before
    );
    assert_eq!(snapshot_tree(&fixture.root.join(".git")), repository_before);
}

#[test]
fn existing_unborn_repository_acceptance_adds_commit_ready_local_identity() {
    let fixture = Fixture::new("unborn-fixtures");
    let declined = fixture.root.join("declined");
    let accepted = fixture.root.join("accepted");
    fs::create_dir_all(&declined).unwrap();
    fs::create_dir_all(&accepted).unwrap();

    for root in [&declined, &accepted] {
        fixture.init_repository(root);
        fs::write(root.join("already-staged.txt"), "operator staged work\n").unwrap();
        fixture.git_ok(root, &["add", "already-staged.txt"]);
    }
    fixture.git_ok(
        &declined,
        &["config", "--local", "user.name", "Existing Local"],
    );
    fixture.git_ok(
        &declined,
        &["config", "--local", "user.email", "local@example.invalid"],
    );
    fixture.git_ok(&accepted, &["config", "--local", "user.name", ""]);
    fixture.git_ok(&accepted, &["config", "--local", "user.email", ""]);
    assert_eq!(
        fixture.git_ok(&accepted, &["config", "--get", "user.email"]),
        "",
        "fixture must begin with the completion identity gap"
    );

    let declined_config = fs::read(declined.join(".git/config")).unwrap();
    let declined_index = fs::read(declined.join(".git/index")).unwrap();
    let decline = fixture.lisa(&["init", "--path", declined.to_str().unwrap(), "--no-history"]);
    assert_success(&decline, "decline in unborn repository");
    assert!(String::from_utf8(decline.stdout)
        .unwrap()
        .contains(HISTORY_DECLINED));
    assert_failure(
        &fixture.git(&declined, &["rev-parse", "--verify", "HEAD"]),
        "unborn HEAD after decline",
    );
    assert_eq!(
        fs::read(declined.join(".git/config")).unwrap(),
        declined_config
    );
    assert_eq!(
        fs::read(declined.join(".git/index")).unwrap(),
        declined_index
    );

    let accepted_index = fs::read(accepted.join(".git/index")).unwrap();
    let global_before = fs::read(&fixture.global_config).unwrap();
    let accept = fixture.lisa(&[
        "init",
        "--path",
        accepted.to_str().unwrap(),
        "--with-history",
    ]);
    assert_success(&accept, "accept in unborn repository");
    fixture.git_ok(&accepted, &["rev-parse", "--verify", "HEAD"]);
    assert_eq!(
        fixture.git_ok(&accepted, &["config", "--local", "user.name"]),
        "Lisa (project history)"
    );
    assert_eq!(
        fixture.git_ok(&accepted, &["config", "--local", "user.email"]),
        "lisa@project"
    );
    assert_eq!(fs::read(&fixture.global_config).unwrap(), global_before);
    assert_eq!(
        fs::read(accepted.join(".git/index")).unwrap(),
        accepted_index
    );
    assert_eq!(
        fixture.git_ok(&accepted, &["ls-tree", "-r", "--name-only", "HEAD"]),
        "",
        "pre-existing staged work must not enter the initial commit"
    );
    assert_eq!(
        fixture.git_ok(&accepted, &["show", "-s", "--format=%an|%ae|%s", "HEAD"]),
        "Lisa (project history)|lisa@project|Start project history"
    );
    assert_eq!(
        fixture.git_ok(&accepted, &["diff", "--cached", "--name-only"]),
        "already-staged.txt",
        "the existing ordinary index must remain intact"
    );

    fs::write(accepted.join("completion-marker.txt"), "finished\n").unwrap();
    let completion = fixture.lisa(&[
        "commit-ticket",
        "--path",
        accepted.to_str().unwrap(),
        "--ticket-id",
        "T-FIXTURE",
        "--message",
        "Complete unborn fixture work",
        "--include",
        "completion-marker.txt",
    ]);
    assert_success(
        &completion,
        "completion commit after accepted unborn history",
    );
    assert_eq!(
        fixture.git_ok(
            &accepted,
            &["show", "--pretty=format:", "--name-only", "HEAD"]
        ),
        "completion-marker.txt"
    );

    let doctor = fixture.lisa(&["doctor", "--path", accepted.to_str().unwrap()]);
    let doctor_stdout = String::from_utf8(doctor.stdout).unwrap();
    assert!(!doctor_stdout
        .contains("no commit identity is configured (git config user.email did not resolve)"));
}
