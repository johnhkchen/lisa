//! Typed sibling-temporary publication for scheduler-owned files.
//!
//! This module centralizes only the mechanism shared by the plugin's atomic
//! publication sites: resolve a temporary beside its destination, write a
//! complete payload, and replace the destination. Payload serialization,
//! directory creation, attempt authority, provenance append, and Git ref
//! transactions remain with their respective callers.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// How one publication site names its same-directory temporary.
pub(crate) enum TemporaryName {
    /// `{prefix}{wall-clock nanosecond nonce}`.
    Nonce { prefix: String },
    /// `{prefix}{attempt_id}-{wall-clock nanosecond nonce}`.
    AttemptNonce { prefix: String, attempt_id: u64 },
    /// One deterministic sibling filename.
    Exact { file_name: String },
}

impl TemporaryName {
    fn resolve(self) -> String {
        match self {
            Self::Nonce { prefix } => format!("{prefix}{}", publication_nonce()),
            Self::AttemptNonce { prefix, attempt_id } => {
                format!("{prefix}{attempt_id}-{}", publication_nonce())
            }
            Self::Exact { file_name } => file_name,
        }
    }
}

/// Destination plus the typed naming policy for its sibling temporary.
pub(crate) struct PublicationPath {
    pub(crate) destination: PathBuf,
    pub(crate) temporary_name: TemporaryName,
}

struct ResolvedPublicationPath {
    destination: PathBuf,
    temporary: PathBuf,
}

impl PublicationPath {
    fn resolve(self) -> ResolvedPublicationPath {
        let temporary = self
            .destination
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(self.temporary_name.resolve());
        ResolvedPublicationPath {
            destination: self.destination,
            temporary,
        }
    }
}

/// Site-specific operator labels for the two fallible publication operations.
pub(crate) struct PublicationErrors {
    pub(crate) write: &'static str,
    pub(crate) publish: &'static str,
}

/// A complete Rust-side sibling-temp publication request.
pub(crate) struct RustPublication<'a> {
    pub(crate) path: PublicationPath,
    pub(crate) body: &'a [u8],
    pub(crate) errors: PublicationErrors,
}

impl RustPublication<'_> {
    /// Write complete bytes and atomically replace the destination by rename.
    pub(crate) fn publish(self) -> Result<PathBuf, String> {
        let resolved = self.path.resolve();
        std::fs::write(&resolved.temporary, self.body).map_err(|error| {
            format!(
                "{} {}: {error}",
                self.errors.write,
                resolved.temporary.display()
            )
        })?;
        if let Err(error) = std::fs::rename(&resolved.temporary, &resolved.destination) {
            let _ = std::fs::remove_file(&resolved.temporary);
            return Err(format!(
                "{} {}: {error}",
                self.errors.publish,
                resolved.destination.display()
            ));
        }
        Ok(resolved.destination)
    }
}

/// A shell-side sibling-temp publication rendered for later pane execution.
pub(crate) struct ShellPublication<'a> {
    pub(crate) path: PublicationPath,
    pub(crate) body: &'a str,
}

impl ShellPublication<'_> {
    /// Render the existing shell collision contract without host-side I/O.
    pub(crate) fn command(self) -> String {
        let resolved = self.path.resolve();
        format!(
            "command printf '%s' {} > {} && command mv {} {}",
            shell_quote(self.body),
            shell_quote(&resolved.temporary.to_string_lossy()),
            shell_quote(&resolved.temporary.to_string_lossy()),
            shell_quote(&resolved.destination.to_string_lossy()),
        )
    }
}

/// Encode one arbitrary UTF-8 value as one POSIX shell argument.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn publication_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
