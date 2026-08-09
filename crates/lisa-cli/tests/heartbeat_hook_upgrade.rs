//! The upgrade path for the heartbeat hook, end to end.
//!
//! A hook only reaches a project through `lisa init`, and `lisa init` replaces
//! an existing one only when its bytes match a generation Lisa shipped. So the
//! interesting question is not what `LEGACY_ON_HEARTBEAT_HOOKS` contains — it is
//! whether a board written by 0.5.0-rc.2 actually ends up with the hardened hook
//! when an operator runs `lisa init`, and whether the script that lands there
//! refuses to publish progress for an attempt its caller cannot name.
//!
//! Both halves are driven, not read: the fixture is the literal text rc.2 wrote,
//! and the assertions run the resulting file through `/bin/sh`.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Exactly what 0.5.0-rc.2 wrote into `.lisa/hooks/on-heartbeat.sh`. Kept here
/// as a literal so this test fails if that generation is ever dropped from the
/// upgrade list, instead of quietly following the list wherever it goes.
const RC2_ON_HEARTBEAT_HOOK: &str = r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Copies the scheduler-owned attempt lease into an atomic liveness signal.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat.tmp.$$"
    if [ -r "$marker" ] && cp "$marker" "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
    else
        rm -f "$tmp"
    fi
fi
"#;

const MARKER: &str = r#"{"ticket_id":"T-UPGRADE","attempt_id":4}"#;

/// Run the project's installed hook as a process with this launch identity, and
/// report what it published: `(residency, progress)`.
fn run_installed_hook(root: &Path, ticket_id: &str, attempt_id: &str) -> (bool, Option<String>) {
    let signals = root.join(".lisa/signals");
    fs::create_dir_all(&signals).unwrap();
    for name in ["pane-7.alive", "pane-7.heartbeat"] {
        let _ = fs::remove_file(signals.join(name));
    }
    fs::write(signals.join("pane-7.lease"), MARKER).unwrap();

    let status = Command::new("/bin/sh")
        .arg(root.join(".lisa/hooks/on-heartbeat.sh"))
        .current_dir(root)
        .env("LISA_PANE_ID", "7")
        .env("LISA_TICKET_ID", ticket_id)
        .env("LISA_ATTEMPT_ID", attempt_id)
        .status()
        .unwrap();
    assert!(status.success(), "a hook must never fail its caller's turn");

    (
        signals.join("pane-7.alive").exists(),
        fs::read_to_string(signals.join("pane-7.heartbeat")).ok(),
    )
}

#[test]
fn an_rc2_project_is_upgraded_in_place_and_stops_publishing_forgeable_progress() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("board");
    fs::create_dir_all(root.join(".lisa/hooks")).unwrap();
    fs::write(
        root.join(".lisa/hooks/on-heartbeat.sh"),
        RC2_ON_HEARTBEAT_HOOK,
    )
    .unwrap();

    // Before: the rc.2 hook publishes progress for whatever the marker names,
    // to any process that can run a tool call in the pane.
    assert_eq!(
        run_installed_hook(&root, "T-UPGRADE", "3"),
        (false, Some(MARKER.to_string())),
        "the rc.2 hook publishes a predecessor's forged heartbeat"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["init", "--no-history", "--path"])
        .arg(&root)
        .env("HOME", temp.path().join("home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "lisa init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let upgraded = fs::read_to_string(root.join(".lisa/hooks/on-heartbeat.sh")).unwrap();
    assert_ne!(
        upgraded, RC2_ON_HEARTBEAT_HOOK,
        "an existing rc.2 hook must be replaced, not left in place"
    );

    // After: the same predecessor gets residency and nothing else, while the
    // attempt the marker names still publishes progress exactly as before.
    assert_eq!(
        run_installed_hook(&root, "T-UPGRADE", "3"),
        (true, None),
        "a process that cannot name the current attempt must publish no progress"
    );
    assert_eq!(
        run_installed_hook(&root, "T-UPGRADE", "4"),
        (true, Some(MARKER.to_string())),
        "the attempt the marker names still heartbeats"
    );

    // And init is idempotent: a second run leaves the hardened hook alone.
    let again = Command::new(env!("CARGO_BIN_EXE_lisa"))
        .args(["init", "--no-history", "--path"])
        .arg(&root)
        .env("HOME", temp.path().join("home"))
        .output()
        .unwrap();
    assert!(again.status.success());
    assert_eq!(
        fs::read_to_string(root.join(".lisa/hooks/on-heartbeat.sh")).unwrap(),
        upgraded
    );
}
