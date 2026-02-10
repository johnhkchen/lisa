use std::path::Path;

use crate::templates::PLUGIN_WASM;

/// Run the lisa loop: write embedded WASM, generate layout, exec zellij.
///
/// In dry-run mode, scans tickets, builds the DAG, and prints what would happen
/// without writing files or launching zellij.
pub fn run_loop(root: &Path, max_threads: usize, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        // Validate prerequisites (skip in dry-run — user may not have them installed)
        check_binary("zellij", "Install zellij: https://zellij.dev/documentation/installation")?;
        check_binary("claude", "Install Claude Code: https://docs.anthropic.com/en/docs/claude-code")?;
    }

    if !root.join("CLAUDE.md").exists() {
        return Err("No CLAUDE.md found. Run `lisa init` first.".to_string());
    }
    if !root.join("docs/active/tickets").exists() {
        return Err("No docs/active/tickets/ directory. Run `lisa init` first.".to_string());
    }

    if dry_run {
        return run_dry(root, max_threads);
    }

    // Check the WASM plugin is actually embedded (not a dev placeholder)
    if PLUGIN_WASM.is_empty() {
        return Err(
            "WASM plugin not embedded. Build the plugin first:\n  \
             just build && cargo build -p lisa-cli --release"
                .to_string(),
        );
    }

    // Write WASM to a stable temp path
    let wasm_path = std::env::temp_dir().join("lisa-plugin.wasm");
    std::fs::write(&wasm_path, PLUGIN_WASM)
        .map_err(|e| format!("Failed to write WASM plugin to {}: {}", wasm_path.display(), e))?;

    // Generate KDL layout
    let layout = generate_layout(&wasm_path, max_threads);
    let layout_path = root.join(".lisa-layout.kdl");
    std::fs::write(&layout_path, &layout)
        .map_err(|e| format!("Failed to write layout to {}: {}", layout_path.display(), e))?;

    println!("Lisa loop starting...");
    println!("  WASM plugin: {}", wasm_path.display());
    println!("  Layout: {}", layout_path.display());
    println!("  Max threads: {}", max_threads);
    println!();

    // Exec zellij (replaces this process)
    exec_zellij(root, &layout_path)
}

/// Dry-run mode: scan tickets, build DAG, print summary.
fn run_dry(root: &Path, max_threads: usize) -> Result<(), String> {
    let ticket_dir = root.join("docs/active/tickets");
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
    println!("Tickets:  {} total, {} done, {} ready, {} in-progress, {} blocked",
        stats.total_tickets,
        stats.done_tickets,
        stats.ready_tickets,
        stats.in_progress_tickets,
        stats.blocked_tickets,
    );
    println!("Critical path length: {}", stats.critical_path_length);
    println!("Max threads: {}", max_threads);
    println!();

    // Show execution order (topological sort of non-done tickets)
    let topo = dag.topological_sort()
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
                let pending: Vec<_> = deps.iter()
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
    let layout = generate_layout(&wasm_path, max_threads);
    println!();
    println!("Generated layout:");
    println!("{}", layout);

    Ok(())
}

fn check_binary(name: &str, install_hint: &str) -> Result<(), String> {
    match which(name) {
        true => Ok(()),
        false => Err(format!("`{}` not found in PATH. {}", name, install_hint)),
    }
}

fn which(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn generate_layout(wasm_path: &Path, max_threads: usize) -> String {
    format!(
        r#"layout {{
    pane
    pane {{
        plugin location="file://{wasm_path}" {{
            ticket_dir "docs/active/tickets"
            story_dir  "docs/active/stories"
            work_dir   "docs/active/work"
            max_threads "{max_threads}"
        }}
    }}
}}
"#,
        wasm_path = wasm_path.display(),
        max_threads = max_threads,
    )
}

#[cfg(unix)]
fn exec_zellij(root: &Path, layout_path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let err = std::process::Command::new("zellij")
        .arg("--layout")
        .arg(layout_path)
        .current_dir(root)
        .exec();

    // exec() only returns on error
    Err(format!("Failed to exec zellij: {}", err))
}

#[cfg(not(unix))]
fn exec_zellij(root: &Path, layout_path: &Path) -> Result<(), String> {
    let status = std::process::Command::new("zellij")
        .arg("--layout")
        .arg(layout_path)
        .current_dir(root)
        .status()
        .map_err(|e| format!("Failed to run zellij: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("zellij exited with status: {}", status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_layout() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let layout = generate_layout(&wasm_path, 3);

        assert!(layout.contains("file:///tmp/lisa-plugin.wasm"));
        assert!(layout.contains("ticket_dir \"docs/active/tickets\""));
        assert!(layout.contains("story_dir  \"docs/active/stories\""));
        assert!(layout.contains("work_dir   \"docs/active/work\""));
        assert!(layout.contains("max_threads \"3\""));
    }

    #[test]
    fn test_generate_layout_default_threads() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let layout = generate_layout(&wasm_path, 2);
        assert!(layout.contains("max_threads \"2\""));
    }

    #[test]
    fn test_run_loop_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_loop(dir.path(), 2, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CLAUDE.md"));
    }

    #[test]
    fn test_run_loop_missing_tickets_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        let result = run_loop(dir.path(), 2, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tickets"));
    }

    #[test]
    fn test_dry_run_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_loop(dir.path(), 2, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CLAUDE.md"));
    }

    #[test]
    fn test_dry_run_empty_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        let result = run_loop(dir.path(), 2, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_with_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        let tickets_dir = dir.path().join("docs/active/tickets");
        std::fs::create_dir_all(&tickets_dir).unwrap();

        // Create a done root ticket and a ready ticket depending on it
        std::fs::write(tickets_dir.join("T-001.md"), "\
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
").unwrap();

        std::fs::write(tickets_dir.join("T-002.md"), "\
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
").unwrap();

        let result = run_loop(dir.path(), 2, true);
        assert!(result.is_ok());
    }
}
