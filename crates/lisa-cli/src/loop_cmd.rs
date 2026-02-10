use std::path::Path;

use crate::templates::PLUGIN_WASM;

/// Run the lisa loop: write embedded WASM, generate layout, exec zellij.
pub fn run_loop(root: &Path, max_threads: usize) -> Result<(), String> {
    // Validate prerequisites
    check_binary("zellij", "Install zellij: https://zellij.dev/documentation/installation")?;
    check_binary("claude", "Install Claude Code: https://docs.anthropic.com/en/docs/claude-code")?;

    if !root.join("CLAUDE.md").exists() {
        return Err("No CLAUDE.md found. Run `lisa init` first.".to_string());
    }
    if !root.join("docs/active/tickets").exists() {
        return Err("No docs/active/tickets/ directory. Run `lisa init` first.".to_string());
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
        let result = run_loop(dir.path(), 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CLAUDE.md"));
    }

    #[test]
    fn test_run_loop_missing_tickets_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        let result = run_loop(dir.path(), 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tickets"));
    }
}
