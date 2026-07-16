//! Zellij runtime selection for the native CLI.
//!
//! Configuration expresses an intent (`managed`, `system`, or an absolute
//! pinned path). This module resolves that intent to the exact executable Lisa
//! will launch and verifies the executable against lisa-core's support policy.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use lisa_core::version::{
    classify_zellij_version_output, ZellijVersion, ZellijVersionVerdict, SUPPORTED_ZELLIJ_RANGE,
};

/// Exact Zellij release used by Lisa's managed-runtime directory contract.
///
/// This is the patch release resolved for the plugin's `zellij-tile = "0.43"`
/// SDK family. T-046-02-02 installs this release at [`managed_zellij_path`].
pub const MANAGED_ZELLIJ_VERSION: ZellijVersion = ZellijVersion::release(0, 43, 1);

/// Runtime intent after `.lisa.toml` defaults have been applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZellijRuntimeRequest {
    Managed,
    System,
    Pinned(PathBuf),
}

/// The selected source of a resolved Zellij executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZellijRuntimeMode {
    Managed,
    System,
    Pinned,
}

impl fmt::Display for ZellijRuntimeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Managed => "managed",
            Self::System => "system",
            Self::Pinned => "pinned",
        })
    }
}

/// The exact compatible Zellij executable selected for this invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedZellijRuntime {
    pub mode: ZellijRuntimeMode,
    pub version: ZellijVersion,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct RuntimeEnvironment {
    path: Option<OsString>,
    xdg_data_home: Option<OsString>,
    home: Option<OsString>,
}

impl RuntimeEnvironment {
    fn from_process() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            xdg_data_home: std::env::var_os("XDG_DATA_HOME"),
            home: std::env::var_os("HOME"),
        }
    }
}

/// Return the version-specific executable path used by managed mode.
fn managed_zellij_path(environment: &RuntimeEnvironment) -> Result<PathBuf, String> {
    let data_home = environment
        .xdg_data_home
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
        .or_else(|| {
            environment
                .home
                .as_deref()
                .map(Path::new)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".local/share"))
        })
        .ok_or_else(|| {
            "Cannot resolve managed Zellij path: set an absolute XDG_DATA_HOME or HOME".to_string()
        })?;

    Ok(data_home
        .join("lisa/runtime")
        .join(format!("zellij-{MANAGED_ZELLIJ_VERSION}"))
        .join(executable_name()))
}

#[cfg(windows)]
fn executable_name() -> &'static str {
    "zellij.exe"
}

#[cfg(not(windows))]
fn executable_name() -> &'static str {
    "zellij"
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn absolute_executable(path: &Path, mode: ZellijRuntimeMode) -> Result<PathBuf, String> {
    path.canonicalize().map_err(|error| {
        format!(
            "Cannot resolve {mode} Zellij executable at {}: {error}",
            path.display()
        )
    })
}

fn find_system_zellij(environment: &RuntimeEnvironment) -> Result<PathBuf, String> {
    let path = environment
        .path
        .as_deref()
        .ok_or_else(|| "Cannot resolve system Zellij: PATH is not set".to_string())?;

    for directory in std::env::split_paths(path) {
        let candidate = directory.join(executable_name());
        if is_executable(&candidate) {
            return absolute_executable(&candidate, ZellijRuntimeMode::System);
        }
    }

    Err("Cannot resolve system Zellij: `zellij` was not found on PATH".to_string())
}

fn inspect_zellij(path: &Path, mode: ZellijRuntimeMode) -> Result<ZellijVersion, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("Cannot run {mode} Zellij at {}: {error}", path.display()))?;

    if !output.status.success() {
        return Err(format!(
            "Cannot use {mode} Zellij at {}: `--version` exited with {}",
            path.display(),
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match classify_zellij_version_output(&stdout) {
        ZellijVersionVerdict::InRange(version) => Ok(version),
        ZellijVersionVerdict::BelowFloor(version) => Err(format!(
            "Unsupported {mode} Zellij {version} at {}; Lisa requires {}. Set `[runtime] zellij = \"managed\"` to use Lisa's managed runtime.",
            path.display(),
            SUPPORTED_ZELLIJ_RANGE,
        )),
        ZellijVersionVerdict::Unparseable => Err(format!(
            "Unsupported {mode} Zellij at {}: unparseable `--version` output {:?}; Lisa requires {}. Set `[runtime] zellij = \"managed\"` to use Lisa's managed runtime.",
            path.display(),
            stdout.trim(),
            SUPPORTED_ZELLIJ_RANGE,
        )),
    }
}

/// Resolve and validate the exact Zellij executable Lisa should use.
pub fn resolve_zellij_runtime(
    request: &ZellijRuntimeRequest,
) -> Result<ResolvedZellijRuntime, String> {
    resolve_zellij_runtime_in(request, &RuntimeEnvironment::from_process())
}

fn resolve_zellij_runtime_in(
    request: &ZellijRuntimeRequest,
    environment: &RuntimeEnvironment,
) -> Result<ResolvedZellijRuntime, String> {
    let (mode, path) = match request {
        ZellijRuntimeRequest::Managed => (
            ZellijRuntimeMode::Managed,
            managed_zellij_path(environment)?,
        ),
        ZellijRuntimeRequest::System => {
            (ZellijRuntimeMode::System, find_system_zellij(environment)?)
        }
        ZellijRuntimeRequest::Pinned(path) => (ZellijRuntimeMode::Pinned, path.clone()),
    };
    let path = absolute_executable(&path, mode)?;
    let version = inspect_zellij(&path, mode)?;

    Ok(ResolvedZellijRuntime {
        mode,
        version,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn write_stub(path: &Path, version_output: &str, exit_code: i32) {
        use std::os::unix::fs::PermissionsExt;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\nexit {}\n",
                version_output, exit_code
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    fn environment(
        home: &Path,
        xdg_data_home: Option<&Path>,
        path: Option<&Path>,
    ) -> RuntimeEnvironment {
        RuntimeEnvironment {
            path: path.map(|value| value.as_os_str().to_os_string()),
            xdg_data_home: xdg_data_home.map(|value| value.as_os_str().to_os_string()),
            home: Some(home.as_os_str().to_os_string()),
        }
    }

    #[test]
    fn managed_path_prefers_absolute_xdg_data_home() {
        let env = environment(
            Path::new("/home/lisa"),
            Some(Path::new("/var/lib/user-data")),
            None,
        );
        assert_eq!(
            managed_zellij_path(&env).unwrap(),
            Path::new("/var/lib/user-data/lisa/runtime/zellij-0.43.1/zellij")
        );
    }

    #[test]
    fn managed_path_falls_back_to_home_for_relative_or_missing_xdg() {
        for xdg in [None, Some(Path::new("relative/data"))] {
            let env = environment(Path::new("/home/lisa"), xdg, None);
            assert_eq!(
                managed_zellij_path(&env).unwrap(),
                Path::new("/home/lisa/.local/share/lisa/runtime/zellij-0.43.1/zellij")
            );
        }
    }

    #[test]
    fn managed_path_requires_an_absolute_data_root() {
        let env = RuntimeEnvironment {
            xdg_data_home: Some(OsString::from("relative/data")),
            home: Some(OsString::from("relative/home")),
            ..RuntimeEnvironment::default()
        };
        let error = managed_zellij_path(&env).unwrap_err();
        assert!(error.contains("XDG_DATA_HOME"));
        assert!(error.contains("HOME"));
    }

    #[cfg(unix)]
    #[test]
    fn system_mode_uses_first_path_entry_and_returns_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        write_stub(&first.join("zellij"), "zellij 0.43.7", 0);
        write_stub(&second.join("zellij"), "zellij 0.44.2", 0);
        let joined = std::env::join_paths([&first, &second]).unwrap();
        let env = RuntimeEnvironment {
            path: Some(joined),
            ..RuntimeEnvironment::default()
        };

        let resolved = resolve_zellij_runtime_in(&ZellijRuntimeRequest::System, &env).unwrap();
        assert_eq!(resolved.mode, ZellijRuntimeMode::System);
        assert_eq!(resolved.version.to_string(), "0.43.7");
        assert_eq!(resolved.path, first.join("zellij").canonicalize().unwrap());
        assert!(resolved.path.is_absolute());
    }

    #[cfg(unix)]
    #[test]
    fn pinned_mode_wins_over_system_path() {
        let dir = tempfile::tempdir().unwrap();
        let pinned = dir.path().join("pinned-zellij");
        let system_dir = dir.path().join("system");
        write_stub(&pinned, "zellij 0.44.1", 0);
        write_stub(&system_dir.join("zellij"), "zellij 0.43.1", 0);
        let env = environment(dir.path(), None, Some(&system_dir));

        let resolved =
            resolve_zellij_runtime_in(&ZellijRuntimeRequest::Pinned(pinned.clone()), &env).unwrap();
        assert_eq!(resolved.mode, ZellijRuntimeMode::Pinned);
        assert_eq!(resolved.version.to_string(), "0.44.1");
        assert_eq!(resolved.path, pinned.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn managed_mode_uses_versioned_path_instead_of_system_path() {
        let dir = tempfile::tempdir().unwrap();
        let data_home = dir.path().join("data");
        let managed = data_home.join("lisa/runtime/zellij-0.43.1/zellij");
        let system_dir = dir.path().join("system");
        write_stub(&managed, "zellij 0.43.1", 0);
        write_stub(&system_dir.join("zellij"), "zellij 0.44.0", 0);
        let env = environment(dir.path(), Some(&data_home), Some(&system_dir));

        let resolved = resolve_zellij_runtime_in(&ZellijRuntimeRequest::Managed, &env).unwrap();
        assert_eq!(resolved.mode, ZellijRuntimeMode::Managed);
        assert_eq!(resolved.version.to_string(), "0.43.1");
        assert_eq!(resolved.path, managed.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn below_floor_system_binary_is_rejected_with_named_remedy() {
        let dir = tempfile::tempdir().unwrap();
        write_stub(&dir.path().join("zellij"), "zellij 0.40.1", 0);
        let env = environment(dir.path(), None, Some(dir.path()));

        let error = resolve_zellij_runtime_in(&ZellijRuntimeRequest::System, &env).unwrap_err();
        assert!(error.contains("system"));
        assert!(error.contains("0.40.1"));
        assert!(error.contains(">= 0.43.0"));
        assert!(error.contains("managed"));
    }

    #[cfg(unix)]
    #[test]
    fn supported_system_minor_and_patch_versions_pass() {
        for output in ["zellij 0.43.9", "zellij 0.44.0"] {
            let dir = tempfile::tempdir().unwrap();
            write_stub(&dir.path().join("zellij"), output, 0);
            let env = environment(dir.path(), None, Some(dir.path()));
            assert!(resolve_zellij_runtime_in(&ZellijRuntimeRequest::System, &env).is_ok());
        }
    }

    #[cfg(unix)]
    #[test]
    fn unparseable_and_failing_binaries_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let zellij = dir.path().join("zellij");
        let env = environment(dir.path(), None, Some(dir.path()));

        write_stub(&zellij, "mystery version", 0);
        let unparseable =
            resolve_zellij_runtime_in(&ZellijRuntimeRequest::System, &env).unwrap_err();
        assert!(unparseable.contains("unparseable"));
        assert!(unparseable.contains("mystery version"));

        write_stub(&zellij, "zellij 0.44.0", 7);
        let failed = resolve_zellij_runtime_in(&ZellijRuntimeRequest::System, &env).unwrap_err();
        assert!(failed.contains("exited with"));
    }
}
