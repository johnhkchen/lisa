use std::path::Path;

use crate::config::ResolvedConfig;
use crate::templates::PLUGIN_WASM;

/// Run the lisa loop: write embedded WASM, generate layout, exec zellij.
///
/// In dry-run mode, scans tickets, builds the DAG, and prints what would happen
/// without writing files or launching zellij.
pub fn run_loop(root: &Path, config: &ResolvedConfig, dry_run: bool) -> Result<(), String> {
    // Validate project structure first (cheap, no external deps)
    if !root.join("CLAUDE.md").exists() {
        return Err("No CLAUDE.md found. Run `lisa init` first.".to_string());
    }
    if !root.join(&config.ticket_dir).exists() {
        return Err(format!(
            "No {}/ directory. Run `lisa init` first.",
            config.ticket_dir
        ));
    }

    if dry_run {
        return run_dry(root, config);
    }

    // Validate binary prerequisites (skip in dry-run — user may not have them installed)
    crate::doctor::check_required_deps().map_err(|missing| {
        format!(
            "Missing required dependencies: {}\n\nRun `lisa doctor` for details and install instructions.",
            missing.join(", ")
        )
    })?;

    // Check the WASM plugin is actually embedded (not a dev placeholder)
    if PLUGIN_WASM.is_empty() {
        return Err("WASM plugin not embedded in this binary.\n\n  \
             If installed via `cargo install`, the WASM plugin is not included.\n  \
             Build from source for full functionality:\n    \
             git clone https://github.com/johnhkchen/lisa && cd lisa && just release"
            .to_string());
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

    // Generate KDL layout
    let layout = generate_layout(&wasm_path, config);
    let layout_path = root.join(".lisa-layout.kdl");
    std::fs::write(&layout_path, &layout)
        .map_err(|e| format!("Failed to write layout to {}: {}", layout_path.display(), e))?;

    println!("Lisa loop starting...");
    println!("  WASM plugin: {}", wasm_path.display());
    println!("  Layout: {}", layout_path.display());
    println!(
        "  Max threads: {} (panes: {})",
        config.max_threads,
        config.max_threads * 2
    );
    println!();

    // Exec zellij (replaces this process)
    exec_zellij(root, &layout_path)
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
    let layout = generate_layout(&wasm_path, config);
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

fn generate_layout(wasm_path: &Path, config: &ResolvedConfig) -> String {
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
        pane size="30%" {{
            plugin location="file://{wasm_path}" {{
                ticket_dir "{ticket_dir}"
                story_dir  "{story_dir}"
                work_dir   "{work_dir}"
                max_threads "{max_threads}"
                auto_advance "{auto_advance}"
                review_timeout_secs "{review_timeout_secs}"
                session_timeout_secs "{session_timeout_secs}"
                wind_down_secs "{wind_down_secs}"
            }}
        }}
    }}
}}
"#,
        agent_panes = agent_panes,
        wasm_path = wasm_path.display(),
        ticket_dir = config.ticket_dir,
        story_dir = config.story_dir,
        work_dir = config.work_dir,
        max_threads = config.max_threads,
        auto_advance = config.auto_advance,
        review_timeout_secs = config.review_timeout_secs,
        session_timeout_secs = config.session_timeout_secs,
        wind_down_secs = config.wind_down_secs,
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

    fn default_config() -> ResolvedConfig {
        ResolvedConfig::default()
    }

    #[test]
    fn test_generate_layout() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let mut config = default_config();
        config.max_threads = 3;
        let layout = generate_layout(&wasm_path, &config);

        assert!(layout.contains("file:///tmp/lisa-plugin.wasm"));
        assert!(layout.contains("ticket_dir \"docs/active/tickets\""));
        assert!(layout.contains("story_dir  \"docs/active/stories\""));
        assert!(layout.contains("work_dir   \"docs/active/work\""));
        assert!(layout.contains("max_threads \"3\""));
        assert!(layout.contains("auto_advance \"false\""));
        assert!(layout.contains("review_timeout_secs \"600\""));
        assert!(layout.contains("session_timeout_secs \"3600\""));
        assert!(
            layout.contains("compact-bar"),
            "layout should include status bar"
        );
        // max_threads=3 should produce 6 pane lines (2x)
        let pane_count = layout.matches("            pane").count();
        assert_eq!(pane_count, 6, "Expected 6 panes (2 * max_threads=3)");
    }

    #[test]
    fn test_generate_layout_default_threads() {
        let wasm_path = PathBuf::from("/tmp/lisa-plugin.wasm");
        let config = default_config();
        let layout = generate_layout(&wasm_path, &config);
        assert!(layout.contains("max_threads \"2\""));
        // max_threads=2 should produce 4 pane lines (2x)
        let pane_count = layout.matches("            pane").count();
        assert_eq!(pane_count, 4, "Expected 4 panes (2 * max_threads=2)");
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
        let layout = generate_layout(&wasm_path, &config);
        assert!(layout.contains("ticket_dir \"custom/tickets\""));
        assert!(layout.contains("story_dir  \"custom/stories\""));
        assert!(layout.contains("work_dir   \"custom/work\""));
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
    fn test_run_loop_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CLAUDE.md"));
    }

    #[test]
    fn test_run_loop_missing_tickets_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tickets"));
    }

    #[test]
    fn test_dry_run_missing_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CLAUDE.md"));
    }

    #[test]
    fn test_dry_run_empty_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        std::fs::create_dir_all(dir.path().join("docs/active/tickets")).unwrap();
        let config = default_config();
        let result = run_loop(dir.path(), &config, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_with_tickets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
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
        let result = run_loop(dir.path(), &config, true);
        assert!(result.is_ok());
    }
}
