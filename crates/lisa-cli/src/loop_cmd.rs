use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

use crate::config::ResolvedConfig;
use crate::templates::PLUGIN_WASM;
use lisa_core::client::AgentClient;
use lisa_core::completion::CompletionSeal;

/// How a loop was asked to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoopRequest {
    /// Print what a run would do and start nothing.
    pub dry_run: bool,
    /// Run where there is no terminal to run in. Lisa opens one of its own,
    /// nobody watches it, and the dashboard has no audience — see
    /// [`crate::headless`].
    pub headless: bool,
}

/// Where the Zellij client gets the terminal it will not start without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launch {
    /// The caller's own terminal, exactly as every run has always worked.
    Inherited,
    /// One Lisa opens, drains, and discards.
    OwnedPty,
}

/// What an operator is told when they start a run from somewhere with no
/// terminal and have not asked for a headless one.
///
/// This is the message that replaces a measured failure — `could not enable
/// raw mode: Os { code: 6, message: "Device not configured" }`, followed by
/// `zellij exited with status: exit status: 101` — which said nothing about
/// what to do next.
fn no_terminal_refusal() -> String {
    "Lisa found no terminal to start this run in, and Zellij will not start without one.\n\n  \
     Run it without one instead:  lisa loop --headless\n\n\
     A headless run opens a terminal of its own that nobody is looking at, so the agents still \
     get the panes they need. What you give up is the dashboard: watch the board with `lisa \
     status`, or `lisa status --json` from a program, and end the run with `zellij kill-session \
     <name>` or `lisa schedulers --stop <id>`."
        .to_string()
}

/// Decide where the terminal comes from, and refuse rather than let Zellij
/// fail obscurely.
///
/// A headless run is asked for and never inferred: the pane-per-agent
/// arrangement with a dashboard beside it is the good one, and it stays the
/// default everywhere a terminal exists.
fn resolve_launch(headless: bool, has_terminal: bool) -> Result<Launch, String> {
    if headless {
        Ok(Launch::OwnedPty)
    } else if has_terminal {
        Ok(Launch::Inherited)
    } else {
        Err(no_terminal_refusal())
    }
}

/// What a headless run says in place of the dashboard it is not drawing.
fn headless_announcement(session: Option<&str>) -> String {
    let watch = match session {
        Some(name) => format!(
            "  Watch it:  lisa status  (or `lisa status --json` from a program)\n  \
             Look at it, if you ever get a terminal:  zellij attach {name}\n  \
             End it:    zellij kill-session {name}"
        ),
        None => "  Watch it:  lisa status  (or `lisa status --json` from a program)\n  \
                 End it:    lisa schedulers --stop <id>"
            .to_string(),
    };
    format!(
        "Headless: Lisa opened a terminal of its own, and no dashboard is being drawn.\n\
         The agents still run in Zellij panes and their hooks write the same signals.\n{watch}\n\
         Ctrl-C stops watching; the run stays on the board until its session ends."
    )
}

/// Providers that may be scheduled in this loop: the loop default plus every
/// valid per-ticket route. Preflight must cover all of them, not just the default.
fn configured_clients(root: &Path, config: &ResolvedConfig) -> HashSet<AgentClient> {
    let mut clients = HashSet::from([config.client]);
    let ticket_dir = root.join(&config.ticket_dir);
    if let Ok(tickets) = lisa_core::ticket::scan_tickets(&ticket_dir) {
        for ticket in tickets {
            clients.insert(lisa_core::route::resolve_route(&ticket, config.client).agent);
        }
    }
    clients
}

/// Refuse to launch a scheduler whose project setup predates the binary's
/// machine-readable protocol version. `lisa init` upgrades known templates and
/// preserves customized files, while the live assignment prompt independently
/// carries mandatory protocol instructions.
fn validate_project_protocol(config: &ResolvedConfig) -> Result<(), String> {
    if crate::config::version_is_stale(&config.project_version, crate::config::LISA_VERSION) {
        return Err(format!(
            "Project Lisa protocol {} is older than this binary ({}). Run `lisa init` and review its reported file actions before starting the loop.",
            config.project_version,
            crate::config::LISA_VERSION,
        ));
    }
    Ok(())
}

/// Something already on this board, as the refusal reads it out.
struct BoardHolder {
    /// Who it is and what says so, in one line.
    line: String,
    /// The Zellij session it runs in, when Lisa knows one. Both commands an
    /// operator wants — go look at it, end it — are named after this.
    session: Option<String>,
}

/// Refuse to start a second scheduler on a board that already has one, and say
/// which one, how to look at it, and how to end it.
///
/// Two independent kinds of evidence, because one of them was not enough. A
/// *stamp* says a scheduler is working here; it is fresh only while the board
/// has work left, and on 2026-08-12 a board that had finished 204 of 204
/// tickets read as empty while its session, its Zellij server, and the plugin
/// inside it were all still up — so `lisa loop` started a second scheduler on
/// it. A *running session* under this board's own name says a scheduler is
/// resident here whether or not it has anything left to do, and that is the
/// question that mattered: **a finished run is not an absent one.**
///
/// The stamp window is the scheduler's own cadence, not the project's
/// wind-down: a loop the operator just quit must become startable again in
/// about a minute, and a wind-down of five minutes would make a restart after a
/// crash feel broken. A stamp dated ahead of this machine's clock is not
/// evidence of anything and never blocks a start.
///
/// Refusing is the answer here rather than attaching: `lisa loop` was asked to
/// start a run, and silently handing the caller somebody else's running board
/// instead is a different thing than it asked for — impossible, besides, for a
/// caller with no terminal. So the refusal names `zellij attach` and the next
/// keystroke is the operator's.
fn second_scheduler_refusal(
    root: &Path,
    running_here: &[String],
    current_session: Option<&str>,
) -> Option<String> {
    let now = crate::seats::now_secs()?;
    let roster = lisa_core::schedulers::read_roster(root);
    let mut holders: Vec<BoardHolder> = Vec::new();
    let mut spoken_for: HashSet<&str> = HashSet::new();

    for record in &roster {
        if !record.is_live(
            now,
            lisa_core::schedulers::live_window_secs(record.poll_interval_secs, 0),
        ) {
            continue;
        }
        if let Some(session) = record.session_name.as_deref() {
            spoken_for.insert(session);
        }
        let age = record
            .age_secs(now)
            .map(crate::seats::humanize)
            .unwrap_or_else(|| "?".to_string());
        holders.push(BoardHolder {
            line: format!(
                "{} — last seen {age} ago{}",
                record.label(),
                match record.zellij_pid {
                    Some(pid) => format!(", zellij server pid {pid}"),
                    None => String::new(),
                }
            ),
            session: record.session_name.clone(),
        });
    }

    // A session on this board that no fresh stamp accounts for: a run that
    // finished its tickets and stayed resident, or one whose scheduler stopped
    // stamping for any other reason. The session is still there, and so is
    // whatever is inside it.
    for session in running_here {
        if spoken_for.contains(session.as_str()) {
            continue;
        }
        holders.push(BoardHolder {
            line: format!(
                "{session} — a Zellij session still open on this board{}{}",
                match last_stamp_age(&roster, session, now) {
                    Some(age) => format!(", whose scheduler last checked in {age} ago"),
                    None => String::new(),
                },
                match current_session {
                    Some(current) if current == session => " — the one this terminal is in",
                    _ => "",
                }
            ),
            session: Some(session.clone()),
        });
    }

    if holders.is_empty() {
        return None;
    }

    let mut message = format!(
        "{} already running on this board, so Lisa did not start another one. Two schedulers \
         split the signals the panes write between them, and neither can tell a signal the other \
         took from one that never arrived.\n",
        match holders.len() {
            1 => "There is a run".to_string(),
            count => format!("There are {count} runs"),
        }
    );
    for holder in &holders {
        message.push_str(&format!("\n  {}", holder.line));
        match &holder.session {
            Some(session) => message.push_str(&format!(
                "\n    look at it with: zellij attach {session}\
                 \n    stop it with:    zellij kill-session {session}"
            )),
            None => message.push_str(
                "\n    Lisa was never told this one's session name; find it with \
                 `zellij list-sessions`",
            ),
        }
    }
    message.push_str(
        "\n\nA run stays on the board until its session ends. Closing the window stops the \
         client, not the run, and a run that has finished every ticket is still sitting there \
         with nothing to do. `lisa schedulers` lists them, and `lisa schedulers --stop <id>` \
         ends one. If you just stopped it, give it a minute and try again.",
    );
    Some(message)
}

/// How long ago the freshest scheduler that ever ran in this session stamped.
fn last_stamp_age(
    roster: &[lisa_core::schedulers::SchedulerRecord],
    session: &str,
    now: u64,
) -> Option<String> {
    roster
        .iter()
        .filter(|record| record.session_name.as_deref() == Some(session))
        .filter_map(|record| record.age_secs(now))
        .min()
        .map(crate::seats::humanize)
}

fn format_dependency_preflight_error(client: AgentClient, failures: &[String]) -> String {
    format!(
        "Dependency preflight failed for {client}:\n{}\n\nRun `lisa doctor` for the complete dependency report and install guidance.",
        failures.join("\n")
    )
}

fn embedded_wasm_error() -> String {
    format!(
        "WASM plugin not embedded in this binary; it contains the empty development placeholder.\n\n  Reinstall Lisa with the shell installer:\n    {}",
        crate::doctor::LISA_SHELL_INSTALL_COMMAND
    )
}

/// Run the lisa loop: write embedded WASM, generate a layout, run Zellij, and
/// report the final board state when the session exits.
///
/// In dry-run mode, scans tickets, builds the DAG, and prints what would happen
/// without writing files or launching zellij.
pub fn run_loop(root: &Path, config: &ResolvedConfig, request: LoopRequest) -> Result<(), String> {
    // Validate project structure first (cheap, no external deps)
    if !root.join(".lisa.toml").exists() {
        return Err("No .lisa.toml found. Run `lisa init` first.".to_string());
    }
    if !root.join(&config.ticket_dir).exists() {
        return Err(format!(
            "No {}/ directory. Run `lisa init` first.",
            config.ticket_dir
        ));
    }

    validate_project_protocol(config)?;

    if request.dry_run {
        return run_dry(root, config);
    }

    // Freeze one configured runtime decision before any launch side effects.
    // The resulting path is the exact executable reported by doctor and run
    // below; managed and pinned modes never fall back to PATH. It is resolved
    // here, first, because the refusal below has to be able to ask this exact
    // Zellij what it is holding.
    let zellij_runtime = crate::runtime::resolve_zellij_runtime(&config.zellij_runtime)?;

    // Name the session after the project before anything is spawned: the name
    // is what `zellij list-sessions`, the status bar, and the terminal tab all
    // show, and a taken name has to cost a number rather than the start. The
    // plugin is told the same name in its layout below, because it is the only
    // way a scheduler can record which session to stop it by.
    //
    // The same listing says which sessions are still *running* under this
    // board's names, which is the fact the refusal below turns on.
    let sessions = crate::session_name::resolve(root, &zellij_runtime.path);

    // Two schedulers on one board split `.lisa/signals/` between them, and
    // neither can tell a signal the other took from one that was never
    // written. Nothing in a running loop reveals that, so the check belongs
    // here — the last moment before a second one exists.
    if let Some(refusal) = second_scheduler_refusal(
        root,
        sessions.running_here(),
        crate::schedulers::current_session().as_deref(),
    ) {
        return Err(refusal);
    }

    println!("{}", config.client_announcement());

    // Resolve configured guard intent against the environment exactly once.
    // The immutable result is retained through dependency checks and layout
    // generation; the plugin receives only the pinned tier and never probes.
    let completion = crate::completion_seal::resolve_for_run(root, config.completion_mode)?;
    let git_root = match completion.git_root() {
        Some(root) => root.to_path_buf(),
        None => root.canonicalize().map_err(|error| {
            format!(
                "Failed to resolve project root {} for journal completion: {error}",
                root.display()
            )
        })?,
    };

    let clients = configured_clients(root, config);

    // Validate every provider the DAG can route to (skip in dry-run — user may
    // not have them installed). This matters for mixed Claude+Codex loops whose
    // loop default names only one of the two binaries.
    for client in &clients {
        crate::doctor::check_required_deps(*client, completion.seal() == CompletionSeal::Commit)
            .map_err(|failures| format_dependency_preflight_error(*client, &failures))?;
    }

    // Claude's bypass-permissions mode needs a one-time human confirmation
    // that Lisa deliberately never grants (field stall, 2026-07-18: two panes
    // frozen at a dialog no one could see). Refuse to launch into that wall
    // and hand the operator the one command instead.
    if clients.contains(&AgentClient::Claude) && !crate::doctor::claude_bypass_accepted() {
        return Err(format!(
            "Lisa can't run Claude unattended on this machine yet — Claude asks for a \
             one-time confirmation first, and that choice is yours to make, not Lisa's.\n\n\
             {}\n\nThen run `lisa loop` again.",
            crate::doctor::CLAUDE_FIRST_RUN_REMEDY
        ));
    }

    // The native Codex TUI can stop at its directory-trust prompt before Lisa's
    // injected ticket is handled, so pre-seed trust for every loop that may route
    // to Codex. Best-effort — the launch command independently bypasses approval,
    // sandbox, and generated-hook trust prompts for the managed pane.
    if clients.contains(&AgentClient::Codex) {
        if !root.join(".codex/hooks.json").exists() {
            return Err(
                "No .codex/hooks.json found for the native Codex TUI. Run `lisa init` first."
                    .to_string(),
            );
        }
        crate::doctor::pregrant_codex_trust(root);
    }

    // Check the WASM plugin is actually embedded (not a dev placeholder)
    if PLUGIN_WASM.is_empty() {
        return Err(embedded_wasm_error());
    }

    // Write WASM to a content-hashed temp path so Zellij's plugin cache is
    // busted whenever the plugin binary changes.
    let hash = wasm_hash(PLUGIN_WASM);
    let wasm_filename = format!("lisa-plugin-{:016x}.wasm", hash);
    let wasm_path = std::env::temp_dir().join(&wasm_filename);
    std::fs::write(&wasm_path, PLUGIN_WASM).map_err(|e| {
        format!(
            "Failed to write WASM plugin to {}: {}",
            wasm_path.display(),
            e
        )
    })?;

    // Remove stale lisa-plugin-*.wasm files from temp dir
    clean_stale_wasm_files(&wasm_filename);

    // Clean Zellij's compiled plugin cache so it picks up the new WASM
    crate::doctor::clean_zellij_plugin_cache();

    // Pre-grant the plugin's permissions so the dashboard renders without relying
    // on Zellij's interactive permission prompt (which does not reliably complete
    // in every environment — a fresh content-hashed plugin would otherwise show a
    // blank pane). Best-effort; the prompt remains the fallback.
    crate::doctor::pregrant_plugin_permissions(&wasm_path);

    // Capture the absolute `lisa` binary path now (on the host, before exec) so
    // native agent hooks can invoke `lisa capture-usage` without assuming `lisa`
    // is on the pane shell's PATH. current_exe() failure degrades to bare `lisa`.
    let lisa_bin = std::env::current_exe().ok();

    // Generate KDL layout
    let naming = sessions.naming();
    let layout = generate_layout(
        &wasm_path,
        lisa_bin.as_deref(),
        &git_root,
        completion.seal(),
        naming.name(),
        config,
    );
    let layout_path = root.join(".lisa-layout.kdl");
    std::fs::write(&layout_path, &layout)
        .map_err(|e| format!("Failed to write layout to {}: {}", layout_path.display(), e))?;

    // Where the terminal comes from. Last of the refusals, because it is the
    // only one about the launch itself: everything a project can be wrong
    // about has already been said by here, and a caller with no terminal hears
    // this instead of Zellij's raw-mode error two steps later.
    let launch = resolve_launch(request.headless, std::io::stdin().is_terminal())?;

    println!("Lisa loop starting...");
    println!("  WASM plugin: {}", wasm_path.display());
    println!("  Layout: {}", layout_path.display());
    println!("  Session: {}", naming.describe());
    println!("  Zellij mode: {}", zellij_runtime.mode);
    println!("  Zellij version: {}", zellij_runtime.version);
    println!("  Zellij path: {}", zellij_runtime.path.display());
    println!(
        "  Max threads: {} (panes: {})",
        config.max_threads,
        config.max_threads * 2
    );
    if let Some(caps) = format_provider_caps(config) {
        println!("  Provider caps: {}", caps);
    }
    if launch == Launch::OwnedPty {
        println!();
        println!("{}", headless_announcement(naming.name()));
    }
    println!();

    // The one thing an agent must not inherit is the identity of the session
    // that started the loop. Read it here, on the host, before anything is
    // spawned: everything below this line — zellij, the panes, the agents in
    // them — descends from this process's environment.
    let inherited = lisa_core::session_env::inherited_markers();
    if let Some(sentence) = lisa_core::session_env::describe_cleared(&inherited) {
        println!("  {sentence}");
        println!();
    }

    crate::run_summary::record_run_baseline(root)?;
    let exit = run_zellij(
        &zellij_runtime.path,
        root,
        &layout_path,
        naming.name(),
        &inherited,
        launch,
    )?;

    let tickets = lisa_core::ticket::scan_tickets(root.join(&config.ticket_dir))
        .map_err(|error| format!("Failed to scan tickets after Lisa loop: {error}"))?;
    crate::run_summary::print_run_summary(root, &tickets, Path::new(&config.work_dir))?;

    if exit.status.success() || exit.stopped_on_request {
        if exit.stopped_on_request {
            println!();
            println!(
                "Stopped watching. The run is still on the board until its session ends{}",
                match naming.name() {
                    Some(name) => format!(" — end it with `zellij kill-session {name}`."),
                    None => ". `lisa schedulers` lists it.".to_string(),
                }
            );
        }
        Ok(())
    } else {
        Err(zellij_failure(&exit))
    }
}

/// A Zellij that stopped badly, said as much of itself as it managed to print.
///
/// In a run with a window the operator watched it happen. In a headless one
/// nobody did, so the first thing the client wrote into the terminal Lisa
/// opened is the only account there is.
fn zellij_failure(exit: &ZellijExit) -> String {
    match &exit.transcript_head {
        Some(text) => format!(
            "zellij exited with status: {}\n\nWhat it printed into the terminal Lisa opened for \
             it, before it stopped:\n{text}",
            exit.status
        ),
        None => format!("zellij exited with status: {}", exit.status),
    }
}

/// Dry-run mode: scan tickets, build DAG, print summary.
fn run_dry(root: &Path, config: &ResolvedConfig) -> Result<(), String> {
    let ticket_dir = root.join(&config.ticket_dir);
    let tickets = lisa_core::ticket::scan_tickets(&ticket_dir)
        .map_err(|e| format!("Failed to scan tickets: {}", e))?;

    if tickets.is_empty() {
        println!("No tickets found in {}", ticket_dir.display());
        return Ok(());
    }

    let dag = lisa_core::dag::Dag::from_tickets(tickets)
        .map_err(|e| format!("Failed to build DAG: {:?}", e))?;

    let stats = dag.stats();
    let mut ready = dag.get_ready_tickets();
    ready.sort();

    println!("lisa loop --dry-run");
    println!();
    println!(
        "Tickets:  {} total, {} done, {} ready, {} in-progress, {} blocked",
        stats.total_tickets,
        stats.done_tickets,
        stats.ready_tickets,
        stats.in_progress_tickets,
        stats.blocked_tickets,
    );
    println!("Critical path length: {}", stats.critical_path_length);
    println!(
        "Max threads: {} (panes: {})",
        config.max_threads,
        config.max_threads * 2
    );
    if let Some(caps) = format_provider_caps(config) {
        println!("Provider caps: {}", caps);
    }
    println!();

    // Show execution order (topological sort of non-done tickets)
    let topo = dag
        .topological_sort()
        .map_err(|e| format!("Cycle detected in DAG: {:?}", e))?;

    println!("Execution order:");
    for id in &topo {
        if let Some(t) = dag.get_ticket(id) {
            let state = if t.phase == lisa_core::types::Phase::Done {
                "done".to_string()
            } else if ready.contains(id) {
                "ready".to_string()
            } else if t.phase.is_active() {
                "in-progress".to_string()
            } else {
                let deps = dag.get_dependencies(id);
                let pending: Vec<_> = deps
                    .iter()
                    .filter(|d| {
                        dag.get_ticket(d)
                            .map(|dt| dt.phase != lisa_core::types::Phase::Done)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                format!("blocked by {}", pending.join(", "))
            };
            println!("  {} — {} ({})", t.id, t.title, state);
        }
    }

    // Show generated layout (using a placeholder WASM path)
    let wasm_path = std::env::temp_dir().join("lisa-plugin.wasm");
    let lisa_bin = std::env::current_exe().ok();
    // Dry-run remains useful for a freshly initialized, not-yet-committed
    // project, while real loops require a discoverable repository.
    let git_root = discover_git_root(root).unwrap_or_else(|_| root.to_path_buf());
    let completion_seal = config
        .completion_mode
        .explicit_seal()
        .unwrap_or(CompletionSeal::Commit);
    // A dry run starts nothing, so it names no session: the layout it prints is
    // the shape, not the run.
    let layout = generate_layout(
        &wasm_path,
        lisa_bin.as_deref(),
        &git_root,
        completion_seal,
        None,
        config,
    );
    println!();
    println!("Generated layout:");
    println!("{}", layout);

    Ok(())
}

/// FNV-1a hash of WASM bytes + CLI version. Including the version ensures
/// a new filename on every release, busting Zellij's path-based cache even
/// when the WASM binary is byte-identical (e.g. same-version reinstall via brew).
fn wasm_hash(wasm: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &byte in wasm {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    for &byte in env!("CARGO_PKG_VERSION").as_bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Delete old `lisa-plugin-*.wasm` files in the temp directory that don't
/// match the current filename.
fn clean_stale_wasm_files(current_filename: &str) {
    let tmp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("lisa-plugin-")
                && name.ends_with(".wasm")
                && name.as_ref() != current_filename
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Format the per-provider caps for operator display (`claude=8, codex=4`),
/// sorted for determinism. Returns `None` when no caps are configured so the
/// caller can omit the line entirely (T-026-02).
fn format_provider_caps(config: &ResolvedConfig) -> Option<String> {
    if config.provider_caps.is_empty() {
        return None;
    }
    let mut caps: Vec<(&String, &usize)> = config.provider_caps.iter().collect();
    caps.sort_by(|a, b| a.0.cmp(b.0));
    Some(
        caps.iter()
            .map(|(name, cap)| format!("{}={}", name, cap))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn discover_git_root(project_root: &Path) -> Result<std::path::PathBuf, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| format!("Failed to discover Git root: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to discover Git root from {}: {}",
            project_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return Err("Failed to discover Git root: git returned an empty path".to_string());
    }
    std::path::PathBuf::from(root)
        .canonicalize()
        .map_err(|error| {
            format!(
                "Failed to resolve discovered Git root for {}: {error}",
                project_root.display()
            )
        })
}

fn generate_layout(
    wasm_path: &Path,
    lisa_bin: Option<&Path>,
    git_root: &Path,
    completion_seal: CompletionSeal,
    session_name: Option<&str>,
    config: &ResolvedConfig,
) -> String {
    // Create 2x max_threads pane slots so transitions don't block scheduling.
    // Extra idle panes absorb new tickets while finishing panes wind down.
    let pane_count = config.max_threads * 2;
    let mut agent_panes = String::new();
    for _ in 0..pane_count {
        // No expanded=true: zellij expands the focused (first) pane by default,
        // and a hardcoded expanded attribute gets re-applied on every terminal
        // resize, snapping focus back to the first pane.
        agent_panes.push_str("            pane\n");
    }

    // The same number, told to the plugin, so a run that loses a pane can put it
    // back. The layout is the only thing that knows what *this* run asked for,
    // and this line is written from the loop above rather than recomputed from
    // `max_threads` — a scheduler and its layout that each derive the count
    // separately are two numbers that can disagree, and the whole point of
    // healing is that the layout wins.
    let agent_pane_count_line = format!("                agent_panes \"{pane_count}\"\n");

    // The absolute `lisa` path (current_exe), exported by native adapters for
    // lifecycle-hook usage capture. An absent key falls back to PATH.
    let lisa_bin_line = match lisa_bin {
        Some(p) => format!("                lisa_bin \"{}\"\n", p.display()),
        None => String::new(),
    };

    // The session this loop is about to start. The plugin runs inside the
    // Zellij server and cannot ask which session that is, so a scheduler that
    // is handed no name records none — and `lisa schedulers` says so rather
    // than printing a name that would not stop anything.
    let session_name_line = match session_name {
        Some(name) => format!("                session_name \"{name}\"\n"),
        None => String::new(),
    };

    // The model this board runs within its client, when it names one. Absent
    // key → the plugin leaves the choice to the client's own default, which is
    // the pre-0.5 behaviour byte for byte. Quoted defensively: a model name is
    // provider vocabulary Lisa passes through without interpreting.
    let model_line = match config.model.as_deref() {
        Some(model) => format!(
            "                model \"{}\"\n",
            model.replace('\\', "\\\\").replace('"', "\\\"")
        ),
        None => String::new(),
    };

    // Per-provider concurrency caps (T-026-02), emitted as `provider_cap_<name>`
    // keys the plugin parses back into PluginConfig.provider_caps. Sorted for
    // deterministic layout output. Empty map → no lines → layout byte-for-byte
    // unchanged for single-provider (uncapped) loops.
    let mut provider_cap_lines = String::new();
    let mut caps: Vec<(&String, &usize)> = config.provider_caps.iter().collect();
    caps.sort_by(|a, b| a.0.cmp(b.0));
    for (name, cap) in caps {
        provider_cap_lines.push_str(&format!(
            "                provider_cap_{} \"{}\"\n",
            name, cap
        ));
    }

    format!(
        r#"layout {{
    default_tab_template {{
        children
        pane size=1 borderless=true {{
            plugin location="compact-bar"
        }}
    }}
    tab name="lisa" {{
        pane stacked=true size="70%" {{
{agent_panes}        }}
        pane size="30%" name="lisa · starting" {{
            plugin location="file://{wasm_path}" {{
                git_root "{git_root}"
                completion_seal "{completion_seal}"
                ticket_dir "{ticket_dir}"
                story_dir  "{story_dir}"
                work_dir   "{work_dir}"
                max_threads "{max_threads}"
                review_timeout_secs "{review_timeout_secs}"
                session_timeout_secs "{session_timeout_secs}"
                wind_down_secs "{wind_down_secs}"
                assignment_ack_timeout_secs "{assignment_ack_timeout_secs}"
                triage_enabled "{triage_enabled}"
                triage_timeout_secs "{triage_timeout_secs}"
                client "{client}"
{agent_pane_count_line}{model_line}{provider_cap_lines}{lisa_bin_line}{session_name_line}            }}
        }}
    }}
}}
"#,
        agent_panes = agent_panes,
        agent_pane_count_line = agent_pane_count_line,
        wasm_path = wasm_path.display(),
        git_root = git_root.display(),
        completion_seal = completion_seal,
        lisa_bin_line = lisa_bin_line,
        session_name_line = session_name_line,
        model_line = model_line,
        provider_cap_lines = provider_cap_lines,
        ticket_dir = config.ticket_dir,
        story_dir = config.story_dir,
        work_dir = config.work_dir,
        max_threads = config.max_threads,
        review_timeout_secs = config.review_timeout_secs,
        session_timeout_secs = config.session_timeout_secs,
        wind_down_secs = config.wind_down_secs,
        assignment_ack_timeout_secs = config.assignment_ack_timeout_secs,
        triage_enabled = config.triage_enabled,
        triage_timeout_secs = config.triage_timeout_secs,
        client = config.client.as_str(),
    )
}

/// How the Zellij client ended, and what it had to say for itself.
struct ZellijExit {
    status: std::process::ExitStatus,
    /// True when Lisa asked it to stop — Ctrl-C, a `kill`, or a dropped SSH
    /// connection on a headless run. Not a failure: closing the window has
    /// always stopped the client and left the run.
    stopped_on_request: bool,
    /// The first of what a headless client printed, kept so a client that dies
    /// at startup can still explain itself to a caller who saw nothing.
    transcript_head: Option<String>,
}

fn run_zellij(
    zellij_path: &Path,
    root: &Path,
    layout_path: &Path,
    session_name: Option<&str>,
    inherited_markers: &[String],
    launch: Launch,
) -> Result<ZellijExit, String> {
    let mut command = zellij_command(
        zellij_path,
        root,
        layout_path,
        session_name,
        inherited_markers,
    );
    match launch {
        Launch::Inherited => command
            .status()
            .map(|status| ZellijExit {
                status,
                stopped_on_request: false,
                transcript_head: None,
            })
            .map_err(|error| format!("Failed to run Zellij at {}: {error}", zellij_path.display())),
        Launch::OwnedPty => {
            let run = crate::headless::run_on_pty(command)?;
            Ok(ZellijExit {
                status: run.status,
                stopped_on_request: run.interrupted,
                transcript_head: run.transcript_head(),
            })
        }
    }
}

fn zellij_command(
    zellij_path: &Path,
    root: &Path,
    layout_path: &Path,
    session_name: Option<&str>,
    inherited_markers: &[String],
) -> Command {
    let mut command = Command::new(zellij_path);
    // Everything the session ever spawns inherits this environment, so the
    // markers are dropped once, here, rather than at each of the places that
    // later starts an agent. A pane's own launch line strips them again — the
    // second line of defence covers a session an operator started themselves.
    for marker in inherited_markers {
        command.env_remove(marker);
    }
    // `--layout` with `--session` means "add these tabs to that session", so it
    // fails with `Session 'steer' not found` instead of starting one (measured
    // against Zellij 0.44.3). `--new-session-with-layout` is the pairing that
    // starts a named session. With no name to pass, Zellij invents one from
    // `--layout` exactly as it always did.
    match session_name {
        Some(session_name) => {
            command
                .arg("--session")
                .arg(session_name)
                .arg("--new-session-with-layout")
                .arg(layout_path);
        }
        None => {
            command.arg("--layout").arg(layout_path);
        }
    }
    command.current_dir(root);
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn default_config() -> ResolvedConfig {
        ResolvedConfig::default()
    }

    fn test_layout(wasm_path: &Path, lisa_bin: Option<&Path>, config: &ResolvedConfig) -> String {
        generate_layout(
            wasm_path,
            lisa_bin,
            Path::new("/repo"),
            CompletionSeal::Commit,
            Some("lisa"),
            config,
        )
    }

    /// The plugin cannot ask Zellij which session it is in, so the layout is
    /// the only way it ever learns — and the name is what `lisa schedulers`
    /// later hands an operator to stop it with.
    #[test]
    fn the_layout_tells_the_plugin_which_session_it_is_running_in() {
        let layout = test_layout(Path::new("/tmp/lisa-plugin.wasm"), None, &default_config());
        assert!(layout.contains("session_name \"lisa\""));

        let unnamed = generate_layout(
            Path::new("/tmp/lisa-plugin.wasm"),
            None,
            Path::new("/repo"),
            CompletionSeal::Commit,
            None,
            &default_config(),
        );
        assert!(
            !unnamed.contains("session_name"),
            "a run Zellij named itself records no session rather than a wrong one"
        );
    }

    /// A run on a host with no terminal is asked for, never guessed at. The
    /// pane-per-agent arrangement with a dashboard beside it is the good one,
    /// and every machine that can have it keeps it.
    #[test]
    fn headless_is_asked_for_and_a_terminal_is_used_wherever_there_is_one() {
        assert_eq!(resolve_launch(false, true), Ok(Launch::Inherited));
        assert_eq!(resolve_launch(true, true), Ok(Launch::OwnedPty));
        assert_eq!(resolve_launch(true, false), Ok(Launch::OwnedPty));
    }

    /// The measured failure this ticket is named for:
    ///
    /// ```text
    /// could not enable raw mode: Os { code: 6, message: "Device not configured" }
    /// Error: zellij exited with status: exit status: 101
    /// ```
    ///
    /// A caller with no terminal now hears it from Lisa instead, with the way
    /// through in the sentence.
    #[test]
    fn a_caller_with_no_terminal_is_told_the_word_that_starts_a_run_anyway() {
        let refusal = resolve_launch(false, false).unwrap_err();
        assert!(refusal.contains("no terminal"), "{refusal}");
        assert!(refusal.contains("lisa loop --headless"), "{refusal}");
        assert!(
            refusal.contains("lisa status"),
            "it must name what replaces the dashboard: {refusal}"
        );
        assert!(
            refusal.contains("zellij kill-session"),
            "and how a run with nobody watching it ends: {refusal}"
        );
    }

    /// The dashboard is a person's view and a headless host has no person. Say
    /// what is gone and what to read instead, rather than leaving the operator
    /// to notice the silence.
    #[test]
    fn a_headless_run_says_what_it_is_not_drawing_and_what_to_read_instead() {
        let named = headless_announcement(Some("steer"));
        assert!(named.contains("no dashboard is being drawn"), "{named}");
        assert!(named.contains("lisa status --json"), "{named}");
        assert!(named.contains("zellij attach steer"), "{named}");
        assert!(named.contains("zellij kill-session steer"), "{named}");
        assert!(
            named.contains("hooks write the same signals"),
            "a pane is still a Zellij pane: {named}"
        );

        // A session Zellij named itself has no name Lisa can print, so the
        // unnamed form must not invent one.
        let unnamed = headless_announcement(None);
        assert!(!unnamed.contains("zellij attach"), "{unnamed}");
        assert!(unnamed.contains("lisa schedulers --stop"), "{unnamed}");
    }

    /// Nobody is looking at a headless run, so a client that dies at startup
    /// has to carry its own account of it out.
    #[test]
    fn a_headless_client_that_dies_reports_what_it_printed() {
        let exit = ZellijExit {
            status: std::process::Command::new("/usr/bin/false")
                .status()
                .expect("a `false` to exit"),
            stopped_on_request: false,
            transcript_head: Some("  could not enable raw mode".to_string()),
        };
        let error = zellij_failure(&exit);
        assert!(error.contains("zellij exited with status"), "{error}");
        assert!(error.contains("could not enable raw mode"), "{error}");

        let silent = ZellijExit {
            transcript_head: None,
            ..exit
        };
        assert_eq!(
            zellij_failure(&silent),
            format!("zellij exited with status: {}", silent.status)
        );
    }

    /// The refusal an operator sees when they start a second loop on a board
    /// that already has one — the whole point of the registry.
    #[test]
    fn a_live_scheduler_on_the_board_refuses_a_second_loop_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        let now = crate::seats::now_secs().unwrap();
        lisa_core::schedulers::write_record(
            &lisa_core::schedulers::roster_dir(dir.path()),
            &lisa_core::schedulers::SchedulerRecord::new(
                "drum",
                Some("fascinating-drum".to_string()),
                Some(30186),
                now - 17 * 3600,
                now - 3,
                5,
            ),
        )
        .unwrap();

        let refusal =
            second_scheduler_refusal(dir.path(), &[], None).expect("a live scheduler refuses");
        assert!(refusal.contains("There is a run already running on this board"));
        assert!(refusal.contains("fascinating-drum (drum)"));
        assert!(refusal.contains("zellij kill-session fascinating-drum"));
        assert!(refusal.contains("zellij attach fascinating-drum"));
        assert!(refusal.contains("Closing the window stops the client, not the run"));
    }

    /// The failure this ticket is named for. The board had finished — 204 of
    /// 204 tickets — so its scheduler had stopped stamping while its Zellij
    /// session, and the plugin inside it, kept running. Every stamp said no
    /// scheduler was here; the session said otherwise, and the session was
    /// right. A finished run is not an absent one.
    #[test]
    fn a_finished_run_still_sitting_in_its_session_refuses_the_next_loop() {
        let dir = tempfile::tempdir().unwrap();
        let now = crate::seats::now_secs().unwrap();
        lisa_core::schedulers::write_record(
            &lisa_core::schedulers::roster_dir(dir.path()),
            &lisa_core::schedulers::SchedulerRecord::new(
                "lisa-9c1f4b0a",
                Some("lisa".to_string()),
                Some(9450),
                now - 4 * 3600,
                now - 25 * 60,
                5,
            ),
        )
        .unwrap();

        // The stamp is 25 minutes cold, so the roster alone refuses nothing.
        assert!(second_scheduler_refusal(dir.path(), &[], None).is_none());

        let refusal = second_scheduler_refusal(dir.path(), &["lisa".to_string()], None)
            .expect("a session still running on this board refuses");
        assert!(refusal.contains("There is a run already running on this board"));
        assert!(refusal.contains("lisa — a Zellij session still open on this board"));
        assert!(
            refusal.contains("last checked in 25m ago"),
            "the stale stamp is still worth reading out: {refusal}"
        );
        assert!(refusal.contains("zellij attach lisa"));
        assert!(refusal.contains("zellij kill-session lisa"));
        assert!(refusal.contains("finished every ticket is still sitting there"));
    }

    /// A board with no registry at all — a session started by a Lisa that
    /// predates it, or one whose stamp was never written — still refuses.
    /// The session is evidence on its own.
    #[test]
    fn a_running_session_refuses_even_with_nothing_in_the_registry() {
        let dir = tempfile::tempdir().unwrap();

        let refusal = second_scheduler_refusal(
            dir.path(),
            &["lisa".to_string(), "lisa-2".to_string()],
            Some("lisa-2"),
        )
        .expect("a running session refuses on its own");
        assert!(refusal.contains("There are 2 runs already running on this board"));
        assert!(
            !refusal.contains("checked in"),
            "no stamp to read: {refusal}"
        );
        assert!(
            refusal.contains(
                "lisa-2 — a Zellij session still open on this board — the one this \
                              terminal is in"
            ),
            "the session the operator is sitting in says so: {refusal}"
        );
    }

    /// One run, one entry: the live stamp and the running session are the same
    /// scheduler seen twice, and an operator reading two would go looking for a
    /// second one that is not there.
    #[test]
    fn a_stamping_scheduler_and_its_session_are_one_run_not_two() {
        let dir = tempfile::tempdir().unwrap();
        let now = crate::seats::now_secs().unwrap();
        lisa_core::schedulers::write_record(
            &lisa_core::schedulers::roster_dir(dir.path()),
            &lisa_core::schedulers::SchedulerRecord::new(
                "lisa-9c1f4b0a",
                Some("lisa".to_string()),
                Some(9450),
                now - 600,
                now - 3,
                5,
            ),
        )
        .unwrap();

        let refusal = second_scheduler_refusal(dir.path(), &["lisa".to_string()], None).unwrap();
        assert!(refusal.contains("There is a run already running on this board"));
        assert_eq!(refusal.matches("zellij kill-session lisa").count(), 1);
    }

    /// A record from a run that stopped must not lock the operator out of
    /// starting a new one — including a minute after they quit the last.
    #[test]
    fn a_scheduler_that_stopped_stamping_does_not_refuse_the_next_loop() {
        let dir = tempfile::tempdir().unwrap();
        let now = crate::seats::now_secs().unwrap();
        lisa_core::schedulers::write_record(
            &lisa_core::schedulers::roster_dir(dir.path()),
            &lisa_core::schedulers::SchedulerRecord::new(
                "gone",
                Some("lisa".to_string()),
                None,
                now - 3600,
                now - 300,
                5,
            ),
        )
        .unwrap();

        assert!(second_scheduler_refusal(dir.path(), &[], None).is_none());
        assert!(
            second_scheduler_refusal(tempfile::tempdir().unwrap().path(), &[], None).is_none(),
            "a board no scheduler has written on refuses nothing"
        );
    }

    #[test]
    fn test_generate_layout() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let mut config = default_config();
        config.max_threads = 3;
        let layout = test_layout(&wasm_path, None, &config);

        assert!(layout.contains("file:///tmp/lisa-plugin.wasm"));
        assert!(layout.contains("git_root \"/repo\""));
        assert!(layout.contains("completion_seal \"commit\""));
        assert!(layout.contains("ticket_dir \"docs/active/tickets\""));
        assert!(layout.contains("story_dir  \"docs/active/stories\""));
        assert!(layout.contains("work_dir   \"docs/active/work\""));
        assert!(layout.contains("max_threads \"3\""));
        // Retired in 0.5.0: the layout must stop carrying it, not merely stop reading it.
        assert!(!layout.contains("auto_advance"));
        assert!(layout.contains("review_timeout_secs \"600\""));
        assert!(layout.contains("session_timeout_secs \"3600\""));
        assert!(layout.contains("assignment_ack_timeout_secs \"30\""));
        assert!(
            layout.contains("compact-bar"),
            "layout should include status bar"
        );
        // max_threads=3 should produce 6 pane lines (2x)
        let pane_count = layout.matches("            pane").count();
        assert_eq!(pane_count, 6, "Expected 6 panes (2 * max_threads=3)");
    }

    /// The count the plugin heals towards is the count the layout made.
    ///
    /// Not an equality between two derivations of `max_threads * 2`: the whole
    /// point of `agent_panes` is that a run which has *lost* panes can tell what
    /// it started with, so the number is read off the pane lines this very
    /// layout emitted. Any future change that makes the stack a different size
    /// — a spare fewer, a swap layout, a per-provider pane — has to keep these
    /// two agreeing or this fails.
    #[test]
    fn the_layout_declares_exactly_as_many_panes_as_it_creates() {
        for max_threads in [1, 2, 3, 8] {
            let mut config = default_config();
            config.max_threads = max_threads;
            let layout = test_layout(Path::new("/tmp/lisa-plugin.wasm"), None, &config);

            let created = layout
                .lines()
                .filter(|line| line.trim_end() == "            pane")
                .count();
            assert!(created > 0, "max_threads={max_threads} created no panes");
            assert!(
                layout.contains(&format!("agent_panes \"{created}\"")),
                "max_threads={max_threads} created {created} panes but declared something else:\n{layout}"
            );
        }
    }

    #[test]
    fn test_generate_layout_carries_the_pinned_journal_tier() {
        let layout = generate_layout(
            Path::new("/tmp/lisa-plugin.wasm"),
            None,
            Path::new("/project"),
            CompletionSeal::Journal,
            Some("lisa"),
            &default_config(),
        );

        assert!(layout.contains("completion_seal \"journal\""));
        assert!(!layout.contains("completion_seal \"auto\""));
    }

    #[test]
    fn test_zellij_command_uses_resolved_absolute_path() {
        for zellij_path in [
            Path::new("/data/lisa/runtime/zellij-0.43.1/zellij"),
            Path::new("/opt/pinned/zellij"),
        ] {
            let command = zellij_command(
                zellij_path,
                Path::new("/repo"),
                Path::new("/repo/.lisa-layout.kdl"),
                None,
                &[],
            );
            assert_eq!(command.get_program(), zellij_path.as_os_str());
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                vec!["--layout", "/repo/.lisa-layout.kdl"]
            );
            assert_eq!(command.get_current_dir(), Some(Path::new("/repo")));
        }
    }

    #[test]
    fn test_zellij_command_names_the_session_after_the_project() {
        let command = zellij_command(
            Path::new("/opt/pinned/zellij"),
            Path::new("/repo"),
            Path::new("/repo/.lisa-layout.kdl"),
            Some("steer"),
            &[],
        );

        // Measured against Zellij 0.44.3: `--layout` with `--session` adds tabs
        // to a session of that name and fails when there is none, so a named
        // start has to say `--new-session-with-layout`.
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![
                "--session",
                "steer",
                "--new-session-with-layout",
                "/repo/.lisa-layout.kdl"
            ]
        );
    }

    /// The field failure in T-062-01-04: the loop was started from inside a
    /// Claude Code session, the session's markers reached every pane, and each
    /// agent Lisa launched treated itself as a nested child — transcripts off,
    /// three tickets failing with nothing written down. The session Lisa starts
    /// must not carry them.
    #[test]
    fn test_zellij_session_does_not_inherit_agent_session_markers() {
        let markers = vec![
            "CLAUDECODE".to_string(),
            "CLAUDE_CODE_CHILD_SESSION".to_string(),
            "CLAUDE_CODE_SESSION_ID".to_string(),
        ];
        let command = zellij_command(
            Path::new("/opt/pinned/zellij"),
            Path::new("/repo"),
            Path::new("/repo/.lisa-layout.kdl"),
            None,
            &markers,
        );

        let removed: Vec<String> = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(removed, markers);
    }

    #[test]
    fn test_zellij_command_touches_no_environment_when_nothing_was_inherited() {
        let command = zellij_command(
            Path::new("/opt/pinned/zellij"),
            Path::new("/repo"),
            Path::new("/repo/.lisa-layout.kdl"),
            None,
            &[],
        );
        assert_eq!(command.get_envs().count(), 0);
    }

    #[test]
    fn test_generate_layout_default_client_is_claude() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let layout = test_layout(&wasm_path, None, &config);
        assert!(layout.contains("client \"claude\""));
    }

    #[test]
    fn test_generate_layout_codex_client() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = ResolvedConfig {
            client: lisa_core::client::AgentClient::Codex,
            ..default_config()
        };
        let layout = test_layout(&wasm_path, None, &config);
        assert!(layout.contains("client \"codex\""));
    }

    /// A board that names a model hands it to the scheduler; one that does not
    /// emits no key at all, so the layout is byte-for-byte what it always was
    /// and the client keeps choosing its own model.
    #[test]
    fn test_generate_layout_carries_the_configured_model_or_no_key() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let named = ResolvedConfig {
            model: Some("gpt-5-mini".to_string()),
            ..default_config()
        };
        assert!(test_layout(&wasm_path, None, &named).contains("model \"gpt-5-mini\""));

        let unnamed = test_layout(&wasm_path, None, &default_config());
        assert!(
            !unnamed
                .lines()
                .any(|line| line.trim().starts_with("model ")),
            "an unnamed model must emit no key:\n{unnamed}"
        );
    }

    #[test]
    fn test_configured_clients_includes_mixed_ticket_route() {
        let dir = tempfile::tempdir().unwrap();
        let tickets = dir.path().join("docs/active/tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("T-001.md"),
            "---\nid: T-001\ntitle: routed\ntype: task\nstatus: open\npriority: high\nphase: ready\nagent: codex\n---\n",
        )
        .unwrap();

        let clients = configured_clients(dir.path(), &default_config());
        assert!(clients.contains(&AgentClient::Claude));
        assert!(clients.contains(&AgentClient::Codex));
    }

    #[test]
    fn test_configured_clients_single_provider_stays_single() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        let clients = configured_clients(dir.path(), &default_config());
        assert_eq!(clients, HashSet::from([AgentClient::Claude]));
    }

    #[test]
    fn test_generate_layout_emits_provider_caps() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let mut config = default_config();
        config.provider_caps.insert("codex".to_string(), 8);
        config.provider_caps.insert("claude".to_string(), 4);
        let layout = test_layout(&wasm_path, None, &config);
        assert!(layout.contains("provider_cap_codex \"8\""));
        assert!(layout.contains("provider_cap_claude \"4\""));
        // Sorted deterministically: claude before codex.
        let claude_at = layout.find("provider_cap_claude").unwrap();
        let codex_at = layout.find("provider_cap_codex").unwrap();
        assert!(claude_at < codex_at, "caps emitted in sorted order");
    }

    #[test]
    fn test_generate_layout_omits_provider_caps_when_empty() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let layout = test_layout(&wasm_path, None, &config);
        // Byte-for-byte regression guard: uncapped loops emit no cap keys.
        assert!(!layout.contains("provider_cap_"));
    }

    #[test]
    fn test_format_provider_caps() {
        let mut config = default_config();
        assert_eq!(format_provider_caps(&config), None);
        config.provider_caps.insert("codex".to_string(), 8);
        config.provider_caps.insert("claude".to_string(), 4);
        assert_eq!(
            format_provider_caps(&config).as_deref(),
            Some("claude=4, codex=8")
        );
    }

    #[test]
    fn test_format_dependency_preflight_error_preserves_zellij_details() {
        let failures = vec![
            "  zellij       unsupported\n    configured system Zellij was not found\n    Remedy: Set `[runtime] zellij = \"managed\"` to use Lisa's managed runtime"
                .to_string(),
        ];

        let error = format_dependency_preflight_error(AgentClient::Claude, &failures);

        assert!(error.contains("Dependency preflight failed for claude"));
        assert!(error.contains("configured system Zellij was not found"));
        assert!(error.contains("managed runtime"));
        assert!(error.contains("lisa doctor"));
    }

    #[test]
    fn test_embedded_wasm_error_names_shell_installer_not_source_build() {
        let error = embedded_wasm_error();

        assert!(error.contains("WASM plugin not embedded"));
        assert!(error.contains("empty development placeholder"));
        assert!(error.contains(crate::doctor::LISA_SHELL_INSTALL_COMMAND));
        assert!(error.contains("lisa-cli-installer.sh"));
        assert!(!error.contains("git clone"));
        assert!(!error.contains("just release"));
    }

    #[test]
    fn test_generate_layout_emits_lisa_bin_when_supplied() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let bin = PathBuf::from("/opt/bin/lisa");
        let layout = test_layout(&wasm_path, Some(&bin), &config);
        assert!(layout.contains("lisa_bin \"/opt/bin/lisa\""));
    }

    #[test]
    fn test_generate_layout_omits_lisa_bin_when_absent() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let layout = test_layout(&wasm_path, None, &config);
        assert!(!layout.contains("lisa_bin"));
    }

    #[test]
    fn test_generate_layout_default_threads() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let layout = test_layout(&wasm_path, None, &config);
        assert!(layout.contains("max_threads \"2\""));
        // max_threads=2 should produce 4 pane lines (2x)
        let pane_count = layout.matches("            pane").count();
        assert_eq!(pane_count, 4, "Expected 4 panes (2 * max_threads=2)");
    }

    /// Zellij titles an unnamed plugin pane with its `file:` location, which is
    /// a temp path and a content hash. The plugin retitles itself on its first
    /// render; this is the floor under that, and it also stands if the plugin
    /// never loads.
    #[test]
    fn the_plugin_pane_is_named_before_the_plugin_can_name_it() {
        let wasm_path = PathBuf::from("/var/folders/kn/T/lisa-plugin-8fd16992c4a2b9fd.wasm");
        let layout = test_layout(&wasm_path, None, &default_config());

        assert!(
            layout.contains(r#"pane size="30%" name="lisa · starting" {"#),
            "the dashboard pane carries a name of its own:\n{layout}"
        );
    }

    #[test]
    fn test_generate_layout_custom_dirs() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = ResolvedConfig {
            ticket_dir: "custom/tickets".to_string(),
            story_dir: "custom/stories".to_string(),
            work_dir: "custom/work".to_string(),
            ..default_config()
        };
        let layout = test_layout(&wasm_path, None, &config);
        assert!(layout.contains("ticket_dir \"custom/tickets\""));
        assert!(layout.contains("story_dir  \"custom/stories\""));
        assert!(layout.contains("work_dir   \"custom/work\""));
    }

    #[test]
    fn test_discover_git_root_from_nested_project() {
        let dir = tempfile::tempdir().unwrap();
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir.path())
            .status()
            .unwrap();
        assert!(status.success());
        let project = dir.path().join("games/midsummer");
        std::fs::create_dir_all(&project).unwrap();

        assert_eq!(
            discover_git_root(&project).unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_wasm_hash_includes_version() {
        let wasm = b"fake-wasm-bytes";
        let hash_with_version = wasm_hash(wasm);

        // Compute hash without version (raw FNV-1a of just WASM bytes)
        let hash_without_version = {
            let mut h: u64 = 0xcbf29ce484222325;
            for &byte in wasm.iter() {
                h ^= byte as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };

        assert_ne!(
            hash_with_version, hash_without_version,
            "Hash should differ when version is folded in"
        );
    }

    #[test]
    fn test_run_loop_refuses_uninitialised_project() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, LoopRequest::default());
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains(".lisa.toml"), "error was: {error}");
        assert!(!error.contains("CLAUDE.md"), "error was: {error}");
    }

    #[test]
    fn test_run_loop_missing_tickets_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, LoopRequest::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tickets"));
    }

    #[test]
    fn test_dry_run_refuses_uninitialised_project() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        let result = run_loop(
            dir.path(),
            &config,
            LoopRequest {
                dry_run: true,
                ..LoopRequest::default()
            },
        );
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains(".lisa.toml"), "error was: {error}");
        assert!(!error.contains("CLAUDE.md"), "error was: {error}");
    }

    /// The regression this ticket is named for: `lisa init` no longer writes a
    /// context file, so a correctly initialised project has none. The loop must
    /// start anyway.
    #[test]
    fn test_loop_starts_without_a_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        assert!(!dir.path().join("CLAUDE.md").exists());

        let config = default_config();
        assert!(run_loop(
            dir.path(),
            &config,
            LoopRequest {
                dry_run: true,
                ..LoopRequest::default()
            },
        )
        .is_ok());
    }

    #[test]
    fn test_dry_run_empty_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        let config = default_config();
        let result = run_loop(
            dir.path(),
            &config,
            LoopRequest {
                dry_run: true,
                ..LoopRequest::default()
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_loop_rejects_stale_release_candidate_protocol() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        let config = ResolvedConfig {
            project_version: "0.4.0-rc.5".to_string(),
            ..default_config()
        };

        let error = run_loop(
            dir.path(),
            &config,
            LoopRequest {
                dry_run: true,
                ..LoopRequest::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("0.4.0-rc.5"));
        assert!(error.contains(crate::config::LISA_VERSION));
        assert!(error.contains("lisa init"));
    }

    #[test]
    fn test_dry_run_with_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".lisa.toml"), "").unwrap();
        let tickets_dir = dir.path().join("docs/active/tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        // Create a done root ticket and a ready ticket depending on it
        std::fs::write(
            tickets_dir.join("T-001.md"),
            "\
---
id: T-001
title: root-ticket
type: task
status: done
priority: high
phase: done
depends_on: []
---

Root ticket.
",
        )
        .unwrap();

        std::fs::write(
            tickets_dir.join("T-002.md"),
            "\
---
id: T-002
title: child-ticket
type: task
status: open
priority: medium
phase: ready
depends_on: [T-001]
---

Child ticket.
",
        )
        .unwrap();

        let config = default_config();
        let result = run_loop(
            dir.path(),
            &config,
            LoopRequest {
                dry_run: true,
                ..LoopRequest::default()
            },
        );
        assert!(result.is_ok());
    }
}
