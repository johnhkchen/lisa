use std::fs;
use std::path::Path;

/// Detected project type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Go,
    Python,
    Unknown,
}

/// Information about a detected project
#[derive(Debug, Clone)]
pub struct DetectedProject {
    pub project_type: ProjectType,
    pub name: String,
    pub build_command: String,
    pub test_command: String,
    pub lint_command: String,
    pub source_layout: String,
}

/// Detect the project type by checking for marker files in priority order.
pub fn detect_project(root: &Path) -> DetectedProject {
    if root.join("Cargo.toml").exists() {
        detect_rust(root)
    } else if root.join("package.json").exists() {
        detect_node(root)
    } else if root.join("go.mod").exists() {
        detect_go(root)
    } else if root.join("pyproject.toml").exists() {
        detect_python(root)
    } else {
        DetectedProject {
            project_type: ProjectType::Unknown,
            name: dir_name(root),
            build_command: String::new(),
            test_command: String::new(),
            lint_command: String::new(),
            source_layout: String::new(),
        }
    }
}

fn detect_rust(root: &Path) -> DetectedProject {
    let name = parse_cargo_name(root).unwrap_or_else(|| dir_name(root));
    let source_layout = scan_source_layout(root, "src", &["rs"]);

    DetectedProject {
        project_type: ProjectType::Rust,
        name,
        build_command: "cargo build".to_string(),
        test_command: "cargo test".to_string(),
        lint_command: "cargo clippy".to_string(),
        source_layout,
    }
}

fn detect_node(root: &Path) -> DetectedProject {
    let name = parse_package_json_name(root).unwrap_or_else(|| dir_name(root));
    let source_layout = scan_source_layout(root, "src", &["ts", "tsx", "js", "jsx"]);

    DetectedProject {
        project_type: ProjectType::Node,
        name,
        build_command: "npm run build".to_string(),
        test_command: "npm test".to_string(),
        lint_command: "npm run lint".to_string(),
        source_layout,
    }
}

fn detect_go(root: &Path) -> DetectedProject {
    let name = parse_go_mod_name(root).unwrap_or_else(|| dir_name(root));
    let source_layout = scan_source_layout(root, ".", &["go"]);

    DetectedProject {
        project_type: ProjectType::Go,
        name,
        build_command: "go build ./...".to_string(),
        test_command: "go test ./...".to_string(),
        lint_command: "golangci-lint run".to_string(),
        source_layout,
    }
}

fn detect_python(root: &Path) -> DetectedProject {
    let name = parse_pyproject_name(root).unwrap_or_else(|| dir_name(root));
    let source_layout = scan_source_layout(root, "src", &["py"]);

    DetectedProject {
        project_type: ProjectType::Python,
        name,
        build_command: "pip install -e .".to_string(),
        test_command: "pytest".to_string(),
        lint_command: "ruff check".to_string(),
        source_layout,
    }
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string()
}

fn parse_cargo_name(root: &Path) -> Option<String> {
    let content = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let name = val.trim().trim_matches('"').trim_matches('\'');
                return Some(name.to_string());
            }
        }
    }
    None
}

fn parse_package_json_name(root: &Path) -> Option<String> {
    let content = fs::read_to_string(root.join("package.json")).ok()?;
    // Simple JSON name extraction without pulling in a JSON parser
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"name\"") {
            if let Some(val) = trimmed.split(':').nth(1) {
                let name = val.trim().trim_matches(',').trim().trim_matches('"');
                return Some(name.to_string());
            }
        }
    }
    None
}

fn parse_go_mod_name(root: &Path) -> Option<String> {
    let content = fs::read_to_string(root.join("go.mod")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            let module_path = trimmed.strip_prefix("module ")?.trim();
            // Use the last path component as the name
            let name = module_path.rsplit('/').next().unwrap_or(module_path);
            return Some(name.to_string());
        }
    }
    None
}

fn parse_pyproject_name(root: &Path) -> Option<String> {
    let content = fs::read_to_string(root.join("pyproject.toml")).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let name = val.trim().trim_matches('"').trim_matches('\'');
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Scan source directory to build a layout string for CLAUDE.md
fn scan_source_layout(root: &Path, src_dir: &str, extensions: &[&str]) -> String {
    let src_path = root.join(src_dir);
    if !src_path.exists() {
        return String::new();
    }

    let mut entries = Vec::new();
    if let Ok(dir) = fs::read_dir(&src_path) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                entries.push(format!("  {}/", name));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext) {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    entries.push(format!("  {}", name));
                }
            }
        }
    }

    entries.sort();
    if entries.is_empty() {
        String::new()
    } else {
        format!(
            "{}:\n{}",
            if src_dir == "." { "." } else { src_dir },
            entries.join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_rust_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-rust-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Rust);
        assert_eq!(project.name, "my-rust-app");
        assert_eq!(project.build_command, "cargo build");
        assert_eq!(project.test_command, "cargo test");
    }

    #[test]
    fn test_detect_node_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"my-node-app\",\n  \"version\": \"1.0.0\"\n}\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Node);
        assert_eq!(project.name, "my-node-app");
        assert_eq!(project.build_command, "npm run build");
    }

    #[test]
    fn test_detect_go_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/my-go-app\n\ngo 1.21\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Go);
        assert_eq!(project.name, "my-go-app");
        assert_eq!(project.build_command, "go build ./...");
    }

    #[test]
    fn test_detect_python_project() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"my-python-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Python);
        assert_eq!(project.name, "my-python-app");
        assert_eq!(project.test_command, "pytest");
    }

    #[test]
    fn test_detect_unknown_project() {
        let dir = tempfile::tempdir().unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Unknown);
    }

    #[test]
    fn test_priority_order() {
        // If both Cargo.toml and package.json exist, Rust wins
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"dual\"\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"dual\"\n}\n",
        )
        .unwrap();

        let project = detect_project(dir.path());
        assert_eq!(project.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_source_layout_scan() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src.join("lib.rs"), "").unwrap();
        fs::create_dir(src.join("utils")).unwrap();

        let layout = scan_source_layout(dir.path(), "src", &["rs"]);
        assert!(layout.contains("main.rs"));
        assert!(layout.contains("lib.rs"));
        assert!(layout.contains("utils/"));
    }
}
