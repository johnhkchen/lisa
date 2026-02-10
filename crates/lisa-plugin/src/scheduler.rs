//! Scheduler module for the Lisa/Ralph Zellij plugin.
//!
//! This module manages thread lifecycle for concurrent ticket processing.
//! "Ralph's role is scheduling and lifecycle, not prompt engineering."
//! The plugin spawns Claude Code sessions, but the agent reads the ticket,
//! CLAUDE.md project context, and docs/rdspi-workflow.md workflow definition itself.
//!
//! ## Commit Serialization
//!
//! Multiple threads can work on the same branch because independent tickets
//! (by DAG definition) shouldn't modify the same files. The dangerous window
//! is just `git add && git commit` - two agents doing that simultaneously
//! produces a dirty index error.
//!
//! The CommitLock uses flock(2) for file-based advisory locking on Unix systems.
//! For WASM plugins, commit serialization is handled by running git operations
//! via zellij's `run_command` with a wrapper script that acquires the lock.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use zellij_tile::prelude::*;

use lisa_core::dag::Dag;
use lisa_core::types::{Phase, TicketId, Thread, ThreadStatus};

// File system operations for commit locking (only available on unix, not in WASM)
#[cfg(all(unix, not(target_arch = "wasm32")))]
use std::fs::{File, OpenOptions};
#[cfg(all(unix, not(target_arch = "wasm32")))]
use std::io;
#[cfg(all(unix, not(target_arch = "wasm32")))]
use std::os::unix::io::AsRawFd;

/// File-based lock for serializing git commits across concurrent threads.
///
/// Uses flock(2) to prevent concurrent git add/commit conflicts on Unix systems.
/// For WASM plugins, this provides the path to the lock file which can be used
/// by wrapper scripts executed via zellij's `run_command`.
///
/// The lock file is created in the repository root as `.ralph-commit.lock`.
pub struct CommitLock {
    /// The lock file handle (kept open while lock is held).
    /// Only available on native Unix builds, not WASM.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    file: Option<File>,
    /// Path to the lock file.
    path: PathBuf,
    /// Whether the lock is held (for WASM builds where we can't check the file).
    #[cfg(target_arch = "wasm32")]
    held: bool,
}

impl CommitLock {
    /// Create a new commit lock instance.
    /// The lock file is created in the repository root as `.ralph-commit.lock`.
    pub fn new(repo_root: &Path) -> Self {
        Self {
            #[cfg(all(unix, not(target_arch = "wasm32")))]
            file: None,
            path: repo_root.join(".ralph-commit.lock"),
            #[cfg(target_arch = "wasm32")]
            held: false,
        }
    }

    /// Acquire the commit lock (blocking).
    ///
    /// On native Unix: Uses flock(2) for file-based locking.
    /// On WASM: Marks the lock as held (actual locking must be done via run_command).
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn acquire(&mut self) -> io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;

        let fd = file.as_raw_fd();
        // LOCK_EX = exclusive lock, blocking
        let result = unsafe { libc::flock(fd, libc::LOCK_EX) };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }

        self.file = Some(file);
        Ok(())
    }

    /// Acquire the commit lock (WASM version).
    ///
    /// In WASM, actual file locking must be done via zellij's run_command
    /// with a wrapper script. This just marks the lock as conceptually held.
    #[cfg(target_arch = "wasm32")]
    pub fn acquire(&mut self) -> Result<(), String> {
        self.held = true;
        Ok(())
    }

    /// Try to acquire the commit lock (non-blocking).
    ///
    /// Returns Ok(true) if lock acquired, Ok(false) if already held by another process.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn try_acquire(&mut self) -> io::Result<bool> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;

        let fd = file.as_raw_fd();
        // LOCK_EX | LOCK_NB = exclusive lock, non-blocking
        let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(false);
            }
            return Err(err);
        }

        self.file = Some(file);
        Ok(true)
    }

    /// Try to acquire the commit lock (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn try_acquire(&mut self) -> Result<bool, String> {
        if self.held {
            Ok(false)
        } else {
            self.held = true;
            Ok(true)
        }
    }

    /// Release the commit lock.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn release(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.take() {
            let fd = file.as_raw_fd();
            // LOCK_UN = unlock
            let result = unsafe { libc::flock(fd, libc::LOCK_UN) };
            if result != 0 {
                return Err(io::Error::last_os_error());
            }
            // File is dropped here, which also releases the lock
        }
        Ok(())
    }

    /// Release the commit lock (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn release(&mut self) -> Result<(), String> {
        self.held = false;
        Ok(())
    }

    /// Check if the lock is currently held by this instance.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn is_held(&self) -> bool {
        self.file.is_some()
    }

    /// Check if the lock is currently held (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Get the path to the lock file.
    ///
    /// This can be used by wrapper scripts to implement locking via flock.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
impl Drop for CommitLock {
    fn drop(&mut self) {
        // Release lock on drop (ignore errors during cleanup)
        let _ = self.release();
    }
}

/// Configuration for the scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Directory containing ticket files.
    pub tickets_dir: PathBuf,
    /// Directory containing story files.
    pub stories_dir: PathBuf,
    /// Directory for work artifacts.
    pub work_dir: PathBuf,
    /// Repository root (for commit lock).
    pub repo_root: PathBuf,
    /// Maximum number of concurrent threads.
    pub max_concurrent_threads: usize,
    /// Path to the claude binary.
    pub claude_binary: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            tickets_dir: PathBuf::from("docs/active/tickets"),
            stories_dir: PathBuf::from("docs/active/stories"),
            work_dir: PathBuf::from("docs/active/work"),
            repo_root: PathBuf::from("."),
            max_concurrent_threads: 4,
            claude_binary: "claude".to_string(),
        }
    }
}

/// The main scheduler that manages thread lifecycle.
///
/// Responsibilities:
/// - Determine which tickets can start based on DAG dependencies
/// - Track running threads and their states
/// - Handle thread completion and parking
/// - Coordinate commit serialization
pub struct Scheduler {
    /// Configuration for the scheduler.
    config: SchedulerConfig,
    /// Currently running threads, keyed by ticket ID.
    threads: HashMap<TicketId, Thread>,
    /// Tickets that are parked (waiting for review).
    parked: HashSet<TicketId>,
    /// Tickets that have completed all phases.
    completed: HashSet<TicketId>,
    /// The commit lock for serializing git operations.
    commit_lock: CommitLock,
    /// Counter for generating unique command names.
    command_counter: u64,
}

impl Scheduler {
    /// Create a new scheduler with the given configuration.
    pub fn new(config: SchedulerConfig) -> Self {
        let commit_lock = CommitLock::new(&config.repo_root);
        Self {
            config,
            threads: HashMap::new(),
            parked: HashSet::new(),
            completed: HashSet::new(),
            commit_lock,
            command_counter: 0,
        }
    }

    /// Create a scheduler with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SchedulerConfig::default())
    }

    /// Get the scheduler configuration.
    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    /// Get all currently running threads.
    pub fn running_threads(&self) -> impl Iterator<Item = &Thread> {
        self.threads.values().filter(|t| t.is_active())
    }

    /// Get the number of currently running threads.
    pub fn running_thread_count(&self) -> usize {
        self.threads.values().filter(|t| t.is_active()).count()
    }

    /// Get all parked ticket IDs.
    pub fn parked_tickets(&self) -> &HashSet<TicketId> {
        &self.parked
    }

    /// Get all completed ticket IDs.
    pub fn completed_tickets(&self) -> &HashSet<TicketId> {
        &self.completed
    }

    /// Get a thread by ticket ID.
    pub fn get_thread(&self, ticket_id: &TicketId) -> Option<&Thread> {
        self.threads.get(ticket_id)
    }

    /// Get a mutable reference to a thread by ticket ID.
    pub fn get_thread_mut(&mut self, ticket_id: &TicketId) -> Option<&mut Thread> {
        self.threads.get_mut(ticket_id)
    }

    /// Get all threads.
    pub fn threads(&self) -> &HashMap<TicketId, Thread> {
        &self.threads
    }

    /// Query the DAG for tickets with satisfied dependencies that are ready to start.
    ///
    /// A ticket is ready if:
    /// - Its status is "open" or "ready"
    /// - All tickets in its `depends_on` list are completed
    /// - It's not already running or parked
    pub fn get_next_ready_tickets(&self, dag: &Dag) -> Vec<TicketId> {
        dag.get_ready_tickets()
            .into_iter()
            .filter(|id| {
                // Not already running
                !self.threads.contains_key(id)
                    // Not parked
                    && !self.parked.contains(id)
                    // Not completed
                    && !self.completed.contains(id)
            })
            .collect()
    }

    /// Spawn a new thread for a ticket.
    ///
    /// Opens a Claude Code session via zellij's `open_command_pane`.
    /// The agent reads the ticket file, CLAUDE.md for project context,
    /// and docs/rdspi-workflow.md for the workflow definition.
    ///
    /// Returns the spawn result for tracking, or None if at capacity.
    pub fn spawn_thread(&mut self, ticket_id: TicketId, pane_id: u32) -> Option<SpawnResult> {
        // Check if we're at capacity
        if self.running_thread_count() >= self.config.max_concurrent_threads {
            return None;
        }

        // Check if already running
        if self.threads.contains_key(&ticket_id) {
            return None;
        }

        // Create the thread record using the Thread type from types.rs
        let thread = Thread::new(ticket_id.clone(), pane_id);

        // Generate a unique command ID for tracking
        self.command_counter += 1;

        // Store the thread
        self.threads.insert(ticket_id.clone(), thread);

        Some(SpawnResult {
            ticket_id,
            command_id: self.command_counter,
        })
    }

    /// Spawn a Claude Code session for a ticket via zellij.
    ///
    /// Uses zellij's `open_command_pane_floating` to run `claude --dangerously-skip-permissions`.
    /// The agent reads the ticket file, CLAUDE.md for project context,
    /// and docs/rdspi-workflow.md for the workflow definition.
    ///
    /// Note: The actual pane_id will be received via PaneUpdate events and should be
    /// associated with the ticket using `register_pane`.
    pub fn spawn_claude_session(&mut self, ticket_id: &TicketId) -> Option<u64> {
        // Check if we're at capacity
        if self.running_thread_count() >= self.config.max_concurrent_threads {
            return None;
        }

        // Check if already running
        if self.threads.contains_key(ticket_id) {
            return None;
        }

        // Build the command to spawn
        let command = self.build_claude_command(ticket_id);

        // Generate a unique command ID for tracking
        self.command_counter += 1;

        // Build context for tracking this command
        // This context is passed through zellij events so we can correlate
        // pane events back to the ticket
        let context = BTreeMap::from([
            ("ticket_id".to_string(), ticket_id.clone()),
            ("command_id".to_string(), self.command_counter.to_string()),
        ]);

        // Use zellij to open a command pane
        // The pane will run `claude --dangerously-skip-permissions`
        open_command_pane_floating(
            CommandToRun {
                path: PathBuf::from(&self.config.claude_binary),
                args: command.args,
                cwd: Some(self.config.repo_root.clone()),
            },
            Some(FloatingPaneCoordinates::default()),
            context,
        );

        Some(self.command_counter)
    }

    /// Register a pane with a ticket after the pane is created.
    ///
    /// This should be called when receiving a PaneUpdate event that indicates
    /// a new pane was created for a ticket.
    pub fn register_pane(&mut self, ticket_id: TicketId, pane_id: u32) {
        let thread = Thread::new(ticket_id.clone(), pane_id);
        self.threads.insert(ticket_id, thread);
    }

    /// Build the claude command arguments for a ticket.
    fn build_claude_command(&self, ticket_id: &TicketId) -> ClaudeCommand {
        // The key insight: "Ralph's role is scheduling and lifecycle, not prompt engineering."
        // We just open a session with --dangerously-skip-permissions.
        // The agent reads the ticket file, CLAUDE.md, and docs/rdspi-workflow.md.
        let ticket_path = self.config.tickets_dir.join(format!("{}.md", ticket_id));

        ClaudeCommand {
            args: vec![
                "--dangerously-skip-permissions".to_string(),
                // The agent will need to be told which ticket to work on.
                // This is done via the prompt, which references the ticket file.
                "--print".to_string(),
                format!(
                    "Read the ticket at {}, the project context in CLAUDE.md, and the RDSPI workflow in docs/rdspi-workflow.md. \
                     Start from the current phase indicated in the ticket frontmatter.",
                    ticket_path.display()
                ),
            ],
        }
    }

    /// Park a thread (mark as waiting for review).
    ///
    /// Called when a thread completes a phase that requires human review.
    pub fn park_thread(&mut self, ticket_id: &TicketId) -> bool {
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.park();
            self.parked.insert(ticket_id.clone());
            true
        } else {
            false
        }
    }

    /// Resume a parked thread.
    ///
    /// Called when human review is complete and the thread should continue.
    pub fn resume_thread(&mut self, ticket_id: &TicketId) -> bool {
        if self.parked.remove(ticket_id) {
            if let Some(thread) = self.threads.get_mut(ticket_id) {
                thread.resume();
                return true;
            }
        }
        false
    }

    /// Complete a thread (mark as done).
    ///
    /// Called when a thread finishes all phases successfully.
    pub fn complete_thread(&mut self, ticket_id: &TicketId) -> bool {
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.complete();
            self.parked.remove(ticket_id);
            self.completed.insert(ticket_id.clone());
            true
        } else {
            false
        }
    }

    /// Handle a thread that has exited (pane closed).
    ///
    /// This could be due to completion, error, or user intervention.
    pub fn handle_thread_exit(&mut self, ticket_id: &TicketId, exit_code: Option<i32>) {
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.mark_exited(exit_code);
            if thread.status == ThreadStatus::Completed {
                self.completed.insert(ticket_id.clone());
            }
        }
    }

    /// Handle a pane exiting by pane ID.
    ///
    /// Looks up the ticket associated with the pane and marks it appropriately.
    pub fn handle_pane_exit(&mut self, pane_id: u32, exit_code: Option<i32>) -> Option<TicketId> {
        // Find the ticket with this pane_id
        let ticket_id = self
            .threads
            .iter()
            .find(|(_, thread)| thread.pane_id == pane_id)
            .map(|(id, _)| id.clone())?;

        self.handle_thread_exit(&ticket_id, exit_code);
        Some(ticket_id)
    }

    /// Update a thread's current phase.
    pub fn update_thread_phase(&mut self, ticket_id: &TicketId, phase: Phase) {
        if let Some(thread) = self.threads.get_mut(ticket_id) {
            thread.current_phase = phase;
        }
    }

    /// Acquire the commit lock for serialized git operations.
    ///
    /// Call this before running `git add && git commit`.
    /// This prevents concurrent threads from corrupting the git index.
    ///
    /// On native Unix: Uses flock(2) for file-based locking.
    /// On WASM: Marks the lock as held; actual locking should use run_command
    /// with a wrapper script that calls flock on the lock file path.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn acquire_commit_lock(&mut self) -> Result<(), String> {
        self.commit_lock.acquire().map_err(|e| e.to_string())
    }

    /// Acquire the commit lock (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn acquire_commit_lock(&mut self) -> Result<(), String> {
        self.commit_lock.acquire()
    }

    /// Try to acquire the commit lock without blocking.
    ///
    /// Returns Ok(true) if acquired, Ok(false) if already held.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn try_acquire_commit_lock(&mut self) -> Result<bool, String> {
        self.commit_lock.try_acquire().map_err(|e| e.to_string())
    }

    /// Try to acquire the commit lock (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn try_acquire_commit_lock(&mut self) -> Result<bool, String> {
        self.commit_lock.try_acquire()
    }

    /// Release the commit lock.
    ///
    /// Call this after completing git operations.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    pub fn release_commit_lock(&mut self) -> Result<(), String> {
        self.commit_lock.release().map_err(|e| e.to_string())
    }

    /// Release the commit lock (WASM version).
    #[cfg(target_arch = "wasm32")]
    pub fn release_commit_lock(&mut self) -> Result<(), String> {
        self.commit_lock.release()
    }

    /// Check if the commit lock is currently held.
    pub fn is_commit_lock_held(&self) -> bool {
        self.commit_lock.is_held()
    }

    /// Get the path to the work directory for a ticket.
    pub fn ticket_work_dir(&self, ticket_id: &TicketId) -> PathBuf {
        self.config.work_dir.join(ticket_id)
    }

    /// Check if a phase artifact exists for a ticket.
    pub fn phase_artifact_exists(&self, ticket_id: &TicketId, phase: &Phase) -> bool {
        if let Some(artifact_name) = phase.artifact_filename() {
            let artifact_path = self.ticket_work_dir(ticket_id).join(artifact_name);
            artifact_path.exists()
        } else {
            false
        }
    }

    /// Schedule work: check DAG and spawn threads for ready tickets.
    ///
    /// This is the main scheduling loop called periodically by the plugin.
    /// Returns the list of tickets that were spawned.
    pub fn schedule(&mut self, dag: &Dag) -> Vec<TicketId> {
        let ready = self.get_next_ready_tickets(dag);
        let mut spawned = Vec::new();

        for ticket_id in ready {
            // Spawn a Claude session for this ticket
            if self.spawn_claude_session(&ticket_id).is_some() {
                spawned.push(ticket_id);
            }

            // Stop if we've hit capacity
            if self.running_thread_count() >= self.config.max_concurrent_threads {
                break;
            }
        }

        spawned
    }

    /// Remove completed threads from the active threads map.
    ///
    /// This cleanup should be called periodically to prevent unbounded growth.
    pub fn cleanup_completed_threads(&mut self) {
        let completed_ids: Vec<TicketId> = self
            .threads
            .iter()
            .filter(|(_, t)| t.status == ThreadStatus::Completed || t.status == ThreadStatus::Failed)
            .map(|(id, _)| id.clone())
            .collect();

        for id in completed_ids {
            self.threads.remove(&id);
        }
    }
}

/// Result of spawning a thread.
#[derive(Debug, Clone)]
pub struct SpawnResult {
    /// The ticket ID for the spawned thread.
    pub ticket_id: TicketId,
    /// Unique command ID for tracking.
    pub command_id: u64,
}

/// Internal command structure for claude invocation.
struct ClaudeCommand {
    args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = Scheduler::with_defaults();
        assert_eq!(scheduler.running_thread_count(), 0);
        assert!(scheduler.parked_tickets().is_empty());
        assert!(scheduler.completed_tickets().is_empty());
    }

    #[test]
    fn test_thread_lifecycle() {
        let config = SchedulerConfig {
            repo_root: PathBuf::from("/tmp"),
            ..Default::default()
        };
        let mut scheduler = Scheduler::new(config);

        let ticket_id = "T-001".to_string();

        // Initially no threads
        assert_eq!(scheduler.running_thread_count(), 0);

        // Park (fails because no thread exists)
        assert!(!scheduler.park_thread(&ticket_id));

        // Complete (fails because no thread exists)
        assert!(!scheduler.complete_thread(&ticket_id));

        // Register a pane for the ticket
        scheduler.register_pane(ticket_id.clone(), 42);
        assert_eq!(scheduler.running_thread_count(), 1);

        // Now park should succeed
        assert!(scheduler.park_thread(&ticket_id));
        assert!(scheduler.parked_tickets().contains(&ticket_id));

        // Resume the thread
        assert!(scheduler.resume_thread(&ticket_id));
        assert!(!scheduler.parked_tickets().contains(&ticket_id));

        // Complete the thread
        assert!(scheduler.complete_thread(&ticket_id));
        assert!(scheduler.completed_tickets().contains(&ticket_id));
    }

    #[test]
    fn test_commit_lock_path() {
        let config = SchedulerConfig {
            repo_root: PathBuf::from("/my/repo"),
            ..Default::default()
        };
        let scheduler = Scheduler::new(config);

        assert_eq!(
            scheduler.commit_lock.path(),
            Path::new("/my/repo/.ralph-commit.lock")
        );
    }

    #[test]
    fn test_ticket_work_dir() {
        let config = SchedulerConfig {
            work_dir: PathBuf::from("docs/active/work"),
            ..Default::default()
        };
        let scheduler = Scheduler::new(config);

        let ticket_id = "T-024-03".to_string();
        assert_eq!(
            scheduler.ticket_work_dir(&ticket_id),
            PathBuf::from("docs/active/work/T-024-03")
        );
    }

    #[test]
    fn test_spawn_thread_capacity() {
        let config = SchedulerConfig {
            max_concurrent_threads: 2,
            repo_root: PathBuf::from("/tmp"),
            ..Default::default()
        };
        let mut scheduler = Scheduler::new(config);

        // Spawn first thread
        let result1 = scheduler.spawn_thread("T-001".to_string(), 1);
        assert!(result1.is_some());

        // Spawn second thread
        let result2 = scheduler.spawn_thread("T-002".to_string(), 2);
        assert!(result2.is_some());

        // Third should fail (at capacity)
        let result3 = scheduler.spawn_thread("T-003".to_string(), 3);
        assert!(result3.is_none());
    }

    #[test]
    fn test_handle_pane_exit() {
        let config = SchedulerConfig {
            repo_root: PathBuf::from("/tmp"),
            ..Default::default()
        };
        let mut scheduler = Scheduler::new(config);

        // Register a thread
        scheduler.register_pane("T-001".to_string(), 42);

        // Handle successful exit
        let ticket = scheduler.handle_pane_exit(42, Some(0));
        assert_eq!(ticket, Some("T-001".to_string()));

        // Thread should be marked completed
        let thread = scheduler.get_thread(&"T-001".to_string()).unwrap();
        assert_eq!(thread.status, ThreadStatus::Completed);
    }

    #[test]
    fn test_phase_artifact_check() {
        let config = SchedulerConfig {
            work_dir: PathBuf::from("/nonexistent/work"),
            ..Default::default()
        };
        let scheduler = Scheduler::new(config);

        // Artifact should not exist for nonexistent path
        assert!(!scheduler.phase_artifact_exists(&"T-001".to_string(), &Phase::Research));
    }
}
