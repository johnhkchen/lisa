//! Zellij runtime selection for the native CLI.
//!
//! Configuration expresses an intent (`managed`, `system`, or an absolute
//! pinned path). This module resolves that intent to the exact executable Lisa
//! will launch and verifies the executable against lisa-core's support policy.

use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

use flate2::read::GzDecoder;
use lisa_core::version::{
    classify_zellij_version_output, ZellijVersion, ZellijVersionVerdict, SUPPORTED_ZELLIJ_RANGE,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Exact Zellij release used by Lisa's managed-runtime directory contract.
///
/// This is the patch release resolved for the plugin's `zellij-tile = "0.43"`
/// SDK family. Managed mode installs this release at [`managed_zellij_path`].
pub const MANAGED_ZELLIJ_VERSION: ZellijVersion = ZellijVersion::release(0, 43, 1);

/// Companion-package Zellij installed outside PATH by native system packages.
const PACKAGED_ZELLIJ_PATH: &str = "/usr/libexec/lisa/zellij";

/// Release-pinned archive checksums for Lisa's managed Zellij runtime.
///
/// T-046-02-02 consumes this compile-time manifest while fetching and atomically
/// installing the managed runtime. The checksums cover the downloaded archives,
/// so verification completes before any archive entry is unpacked.
pub const MANAGED_RUNTIME_SHA256_MANIFEST: &str =
    include_str!("../data/managed-runtime-sha256.json");

#[derive(Debug, Deserialize)]
struct ManagedRuntimeManifest {
    version: String,
    artifacts: Vec<ManagedRuntimeArtifact>,
}

#[derive(Debug, Deserialize)]
struct ManagedRuntimeArtifact {
    target: String,
    url: String,
    sha256: String,
}

static MANAGED_RUNTIME_MANIFEST: LazyLock<ManagedRuntimeManifest> = LazyLock::new(|| {
    let manifest: ManagedRuntimeManifest = serde_json::from_str(MANAGED_RUNTIME_SHA256_MANIFEST)
        .expect("managed-runtime sha256 manifest must be valid JSON");
    assert_eq!(
        manifest.version,
        MANAGED_ZELLIJ_VERSION.to_string(),
        "managed-runtime manifest version must match MANAGED_ZELLIJ_VERSION"
    );
    manifest
});

const INSTALL_TEMP_ATTEMPTS: u64 = 32;
static INSTALL_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
struct ManagedRelease<'a> {
    url: &'a str,
    sha256: &'a str,
}

fn managed_release() -> Result<ManagedRelease<'static>, String> {
    managed_release_for(std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
        format!(
            "Managed Zellij is not available for {} {}; use `[runtime] zellij = \"system\"` or an absolute pinned path",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })
}

/// True when Lisa's managed-runtime matrix covers this host.
pub fn managed_runtime_supported() -> bool {
    managed_release_for(std::env::consts::OS, std::env::consts::ARCH).is_some()
}

/// The runtime request an unconfigured project gets: managed where the
/// matrix covers the host, otherwise the system Zellij on PATH — an
/// unconfigured default must never resolve to a mode that cannot work here.
pub fn default_runtime_request() -> ZellijRuntimeRequest {
    if managed_runtime_supported() {
        ZellijRuntimeRequest::Managed
    } else {
        ZellijRuntimeRequest::System
    }
}

fn managed_release_for(os: &str, architecture: &str) -> Option<ManagedRelease<'static>> {
    let target = match (os, architecture) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        _ => return None,
    };
    let manifest: &'static ManagedRuntimeManifest = &MANAGED_RUNTIME_MANIFEST;
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.target == target)
        .map(|artifact| ManagedRelease {
            url: &artifact.url,
            sha256: &artifact.sha256,
        })
}

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
    Packaged,
    Managed,
    System,
    Pinned,
}

impl fmt::Display for ZellijRuntimeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Packaged => "packaged",
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
    packaged_zellij: PathBuf,
}

impl RuntimeEnvironment {
    fn from_process() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            xdg_data_home: std::env::var_os("XDG_DATA_HOME"),
            home: std::env::var_os("HOME"),
            packaged_zellij: PathBuf::from(PACKAGED_ZELLIJ_PATH),
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

fn acquisition_error(
    category: &str,
    detail: impl fmt::Display,
    release: ManagedRelease<'_>,
) -> String {
    format!(
        "{category}: {detail}; URL: {}; expected sha256: {}",
        release.url, release.sha256
    )
}

struct TempInstall {
    path: PathBuf,
    published: bool,
}

impl TempInstall {
    fn create(runtime_root: &Path, version_dir_name: &str) -> io::Result<Self> {
        std::fs::create_dir_all(runtime_root)?;

        for _ in 0..INSTALL_TEMP_ATTEMPTS {
            let sequence = INSTALL_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = runtime_root.join(format!(
                ".{version_dir_name}.install-{}-{sequence}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        published: false,
                    })
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "could not allocate a temporary directory after {INSTALL_TEMP_ATTEMPTS} attempts"
            ),
        ))
    }

    fn disarm(&mut self) {
        self.published = true;
    }
}

impl Drop for TempInstall {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn download_archive(release: ManagedRelease<'_>, destination: &Path) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .timeout_write(Duration::from_secs(10))
        .redirects(5)
        .build();
    let response = agent
        .get(release.url)
        .set("User-Agent", concat!("lisa/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| acquisition_error("Managed Zellij download failed", error, release))?;

    let file = File::create(destination).map_err(|error| {
        acquisition_error(
            "Managed Zellij download failed",
            format!("cannot create {}: {error}", destination.display()),
            release,
        )
    })?;
    let mut writer = BufWriter::new(file);
    io::copy(&mut response.into_reader(), &mut writer).map_err(|error| {
        acquisition_error(
            "Managed Zellij download failed",
            format!("response body was incomplete or could not be stored: {error}"),
            release,
        )
    })?;
    writer.flush().map_err(|error| {
        acquisition_error(
            "Managed Zellij download failed",
            format!("cannot flush {}: {error}", destination.display()),
            release,
        )
    })?;
    writer.get_ref().sync_all().map_err(|error| {
        acquisition_error(
            "Managed Zellij download failed",
            format!("cannot sync {}: {error}", destination.display()),
            release,
        )
    })
}

fn sha256_reader(mut reader: impl Read) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sha256_file(path: &Path) -> io::Result<String> {
    sha256_reader(BufReader::new(File::open(path)?))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn extract_zellij(
    archive_path: &Path,
    executable_path: &Path,
    release: ManagedRelease<'_>,
) -> Result<(), String> {
    let operation = || -> io::Result<()> {
        let file = File::open(archive_path)?;
        let decoder = GzDecoder::new(BufReader::new(file));
        let mut archive = tar::Archive::new(decoder);
        let mut found = false;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?;
            if entry_path.as_ref() != Path::new(executable_name())
                || !entry.header().entry_type().is_file()
                || found
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("archive contains unexpected entry {}", entry_path.display()),
                ));
            }

            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(executable_path)?;
            let mut writer = BufWriter::new(file);
            io::copy(&mut entry, &mut writer)?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            found = true;
        }

        if !found {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("archive does not contain {}", executable_name()),
            ));
        }

        make_executable(executable_path)
    };

    operation().map_err(|error| {
        acquisition_error(
            "Managed Zellij install failed",
            format!("cannot unpack verified archive: {error}"),
            release,
        )
    })
}

fn ensure_managed_zellij(
    executable_path: &Path,
    release: ManagedRelease<'_>,
) -> Result<(), String> {
    if is_executable(executable_path) {
        return Ok(());
    }

    let version_dir = executable_path.parent().ok_or_else(|| {
        acquisition_error(
            "Managed Zellij install failed",
            format!("invalid executable path {}", executable_path.display()),
            release,
        )
    })?;
    if version_dir.exists() {
        return Err(acquisition_error(
            "Managed Zellij cache is invalid",
            format!(
                "{} exists without an executable {}",
                version_dir.display(),
                executable_name()
            ),
            release,
        ));
    }

    let runtime_root = version_dir.parent().ok_or_else(|| {
        acquisition_error(
            "Managed Zellij install failed",
            format!("invalid runtime directory {}", version_dir.display()),
            release,
        )
    })?;
    let version_dir_name = version_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            acquisition_error(
                "Managed Zellij install failed",
                format!("invalid runtime directory {}", version_dir.display()),
                release,
            )
        })?;
    let mut temporary = TempInstall::create(runtime_root, version_dir_name).map_err(|error| {
        acquisition_error(
            "Managed Zellij install failed",
            format!("cannot create a temporary install directory: {error}"),
            release,
        )
    })?;

    let archive_path = temporary.path.join("download.tar.gz");
    download_archive(release, &archive_path)?;
    let actual_sha256 = sha256_file(&archive_path).map_err(|error| {
        acquisition_error(
            "Managed Zellij checksum failed",
            format!("cannot hash {}: {error}", archive_path.display()),
            release,
        )
    })?;
    if actual_sha256 != release.sha256 {
        return Err(acquisition_error(
            "Managed Zellij checksum mismatch",
            format!("actual sha256: {actual_sha256}"),
            release,
        ));
    }

    let temporary_executable = temporary.path.join(executable_name());
    extract_zellij(&archive_path, &temporary_executable, release)?;
    std::fs::remove_file(&archive_path).map_err(|error| {
        acquisition_error(
            "Managed Zellij install failed",
            format!("cannot remove verified archive before publication: {error}"),
            release,
        )
    })?;

    match std::fs::rename(&temporary.path, version_dir) {
        Ok(()) => {
            temporary.disarm();
            Ok(())
        }
        Err(_) if is_executable(executable_path) => Ok(()),
        Err(error) => Err(acquisition_error(
            "Managed Zellij install failed",
            format!(
                "cannot atomically publish {}: {error}",
                version_dir.display()
            ),
            release,
        )),
    }
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

// ETXTBSY (os error 26 on Linux and macOS): exec of a freshly written binary
// can transiently fail while a concurrently forked process still holds a copy
// of the writer's file descriptor — the fd table is cloned at fork and
// CLOEXEC only clears at that process's own exec. The window is milliseconds;
// retry bounded rather than surfacing a scary one-off failure on first run.
fn run_version_probe(path: &Path) -> io::Result<std::process::Output> {
    let mut delay = Duration::from_millis(10);
    for _ in 0..6 {
        match Command::new(path).arg("--version").output() {
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(delay);
                delay = delay.saturating_mul(2);
            }
            other => return other,
        }
    }
    Command::new(path).arg("--version").output()
}

fn inspect_zellij(path: &Path, mode: ZellijRuntimeMode) -> Result<ZellijVersion, String> {
    let output = run_version_probe(path)
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
        ZellijRuntimeRequest::Managed => {
            if is_executable(&environment.packaged_zellij) {
                (
                    ZellijRuntimeMode::Packaged,
                    environment.packaged_zellij.clone(),
                )
            } else {
                let path = managed_zellij_path(environment)?;
                if !is_executable(&path) {
                    ensure_managed_zellij(&path, managed_release()?)?;
                }
                (ZellijRuntimeMode::Managed, path)
            }
        }
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
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Arc;
    use std::thread::JoinHandle;
    use std::time::Instant;

    enum FixtureResponse {
        Complete(Vec<u8>),
        Interrupted(Vec<u8>),
    }

    struct FixtureServer {
        url: String,
        requests: Arc<AtomicUsize>,
        thread: Option<JoinHandle<()>>,
    }

    impl FixtureServer {
        fn start(response: FixtureResponse) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(AtomicUsize::new(0));
            let thread_requests = Arc::clone(&requests);
            let thread = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            thread_requests.fetch_add(1, AtomicOrdering::SeqCst);
                            read_request_headers(&mut stream);
                            match response {
                                FixtureResponse::Complete(body) => {
                                    write_response(&mut stream, body.len(), &body)
                                }
                                FixtureResponse::Interrupted(body) => {
                                    let advertised = body.len() + 1024;
                                    write_response(&mut stream, advertised, &body)
                                }
                            }
                            break;
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                url: format!("http://{address}/zellij.tar.gz"),
                requests,
                thread: Some(thread),
            }
        }

        fn request_count(&self) -> usize {
            self.requests.load(AtomicOrdering::SeqCst)
        }

        fn join(&mut self) {
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
        }
    }

    impl Drop for FixtureServer {
        fn drop(&mut self) {
            self.join();
        }
    }

    fn read_request_headers(stream: &mut TcpStream) {
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 512];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => request.extend_from_slice(&buffer[..read]),
            }
        }
    }

    fn write_response(stream: &mut TcpStream, content_length: usize, body: &[u8]) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {content_length}\r\nContent-Type: application/gzip\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
    }

    #[cfg(unix)]
    fn fixture_archive() -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let script = b"#!/bin/sh\nprintf '%s\\n' 'zellij 0.43.1'\n";
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, executable_name(), &script[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn fixture_sha256(bytes: &[u8]) -> String {
        sha256_reader(bytes).unwrap()
    }

    fn managed_fixture_path(root: &Path) -> PathBuf {
        root.join("runtime/zellij-0.43.1/zellij")
    }

    fn assert_no_install_temporary_directories(executable_path: &Path) {
        let runtime_root = executable_path.parent().unwrap().parent().unwrap();
        if !runtime_root.exists() {
            return;
        }
        let leftovers: Vec<_> = std::fs::read_dir(runtime_root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().contains(".install-"))
            .collect();
        assert!(leftovers.is_empty(), "temporary directories: {leftovers:?}");
    }

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
            packaged_zellij: home.join("missing-packaged-zellij"),
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
    fn managed_mode_prefers_packaged_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let packaged = dir.path().join("usr/libexec/lisa/zellij");
        let data_home = dir.path().join("data");
        let managed = data_home.join("lisa/runtime/zellij-0.43.1/zellij");
        let system_dir = dir.path().join("system");
        write_stub(&packaged, "zellij 0.44.2", 0);
        write_stub(&managed, "zellij 0.43.1", 0);
        write_stub(&system_dir.join("zellij"), "zellij 0.43.7", 0);
        let mut env = environment(dir.path(), Some(&data_home), Some(&system_dir));
        env.packaged_zellij = packaged.clone();

        let resolved = resolve_zellij_runtime_in(&ZellijRuntimeRequest::Managed, &env).unwrap();

        assert_eq!(resolved.mode, ZellijRuntimeMode::Packaged);
        assert_eq!(resolved.version.to_string(), "0.44.2");
        assert_eq!(resolved.path, packaged.canonicalize().unwrap());
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

    #[test]
    fn production_release_metadata_is_pinned_to_managed_version() {
        let manifest: serde_json::Value =
            serde_json::from_str(MANAGED_RUNTIME_SHA256_MANIFEST).unwrap();
        assert_eq!(
            manifest["version"].as_str().unwrap(),
            MANAGED_ZELLIJ_VERSION.to_string()
        );

        let artifacts = manifest["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 4);
        let expected_targets = std::collections::BTreeSet::from([
            "aarch64-apple-darwin",
            "aarch64-unknown-linux-musl",
            "x86_64-apple-darwin",
            "x86_64-unknown-linux-musl",
        ]);
        let mut observed_targets = std::collections::BTreeSet::new();

        for artifact in artifacts {
            let object = artifact.as_object().unwrap();
            assert_eq!(object.len(), 4);
            let target = object["target"].as_str().unwrap();
            assert!(observed_targets.insert(target), "duplicate target {target}");

            let archive = object["archive"].as_str().unwrap();
            let url = object["url"].as_str().unwrap();
            let sha256 = object["sha256"].as_str().unwrap();
            assert!(archive.starts_with("zellij-no-web-"));
            assert!(archive.contains(target));
            assert!(archive.ends_with(".tar.gz"));
            assert_eq!(
                url,
                format!(
                    "https://github.com/zellij-org/zellij/releases/download/v{}/{archive}",
                    MANAGED_ZELLIJ_VERSION
                )
            );
            assert_eq!(sha256.len(), 64);
            assert!(sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        }

        assert_eq!(observed_targets, expected_targets);

        for (architecture, target) in [
            ("x86_64", "x86_64-unknown-linux-musl"),
            ("aarch64", "aarch64-unknown-linux-musl"),
        ] {
            let release = managed_release_for("linux", architecture).unwrap();
            let artifact = artifacts
                .iter()
                .find(|artifact| artifact["target"] == target)
                .unwrap();
            assert_eq!(release.url, artifact["url"].as_str().unwrap());
            assert_eq!(release.sha256, artifact["sha256"].as_str().unwrap());
        }
    }

    #[cfg(unix)]
    #[test]
    fn successful_fetch_verify_and_atomic_store() {
        let root = tempfile::tempdir().unwrap();
        let executable = managed_fixture_path(root.path());
        let archive = fixture_archive();
        let sha256 = fixture_sha256(&archive);
        let mut server = FixtureServer::start(FixtureResponse::Complete(archive));
        let url = server.url.clone();
        let release = ManagedRelease {
            url: &url,
            sha256: &sha256,
        };

        ensure_managed_zellij(&executable, release).unwrap();
        server.join();

        assert_eq!(server.request_count(), 1);
        assert!(is_executable(&executable));
        assert_eq!(
            inspect_zellij(&executable, ZellijRuntimeMode::Managed).unwrap(),
            MANAGED_ZELLIJ_VERSION
        );
        assert!(!executable
            .parent()
            .unwrap()
            .join("download.tar.gz")
            .exists());
        assert_no_install_temporary_directories(&executable);
    }

    #[cfg(unix)]
    #[test]
    fn checksum_mismatch_is_named_and_leaves_no_partial_install() {
        let root = tempfile::tempdir().unwrap();
        let executable = managed_fixture_path(root.path());
        let archive = fixture_archive();
        let actual_sha256 = fixture_sha256(&archive);
        let expected_sha256 = "0000000000000000000000000000000000000000000000000000000000000000";
        let mut server = FixtureServer::start(FixtureResponse::Complete(archive));
        let release = ManagedRelease {
            url: &server.url,
            sha256: expected_sha256,
        };

        let error = ensure_managed_zellij(&executable, release).unwrap_err();
        server.join();

        assert!(error.contains("Managed Zellij checksum mismatch"));
        assert!(error.contains(&server.url));
        assert!(error.contains(expected_sha256));
        assert!(error.contains(&actual_sha256));
        assert_eq!(server.request_count(), 1);
        assert!(!executable.parent().unwrap().exists());
        assert_no_install_temporary_directories(&executable);
    }

    #[cfg(unix)]
    #[test]
    fn interrupted_download_leaves_no_torn_runtime_directory() {
        let root = tempfile::tempdir().unwrap();
        let executable = managed_fixture_path(root.path());
        let archive = fixture_archive();
        let sha256 = fixture_sha256(&archive);
        let prefix = archive[..archive.len() / 2].to_vec();
        let mut server = FixtureServer::start(FixtureResponse::Interrupted(prefix));
        let release = ManagedRelease {
            url: &server.url,
            sha256: &sha256,
        };

        let error = ensure_managed_zellij(&executable, release).unwrap_err();
        server.join();

        assert!(error.contains("Managed Zellij"));
        assert!(error.contains(&server.url));
        assert!(error.contains(&sha256));
        assert_eq!(server.request_count(), 1);
        assert!(!executable.parent().unwrap().exists());
        assert_no_install_temporary_directories(&executable);
    }

    #[cfg(unix)]
    #[test]
    fn offline_without_cache_is_one_bounded_named_error() {
        let root = tempfile::tempdir().unwrap();
        let executable = managed_fixture_path(root.path());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{address}/zellij.tar.gz");
        let expected_sha256 = "1111111111111111111111111111111111111111111111111111111111111111";
        let release = ManagedRelease {
            url: &url,
            sha256: expected_sha256,
        };
        let started = Instant::now();

        let error = ensure_managed_zellij(&executable, release).unwrap_err();

        assert!(error.contains("Managed Zellij download failed"));
        assert!(error.contains(&url));
        assert!(error.contains(expected_sha256));
        // Bounded-failure guarantee: a retry loop or hang would blow far past
        // this; the bound is generous because the full suite runs in parallel
        // and a loaded host stretched the old 2s limit into a flake.
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(!executable.parent().unwrap().exists());
        assert_no_install_temporary_directories(&executable);
    }

    #[cfg(unix)]
    #[test]
    fn second_managed_resolution_performs_zero_network_calls() {
        let root = tempfile::tempdir().unwrap();
        let executable = managed_fixture_path(root.path());
        let archive = fixture_archive();
        let sha256 = fixture_sha256(&archive);
        let mut server = FixtureServer::start(FixtureResponse::Complete(archive));
        let url = server.url.clone();
        let release = ManagedRelease {
            url: &url,
            sha256: &sha256,
        };

        ensure_managed_zellij(&executable, release).unwrap();
        server.join();
        ensure_managed_zellij(&executable, release).unwrap();

        assert_eq!(server.request_count(), 1);
        assert_eq!(
            inspect_zellij(&executable, ZellijRuntimeMode::Managed).unwrap(),
            MANAGED_ZELLIJ_VERSION
        );
        assert_no_install_temporary_directories(&executable);
    }
}
