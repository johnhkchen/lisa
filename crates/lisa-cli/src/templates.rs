use lisa_core::context::PURPOSE_PARAGRAPH;
use std::sync::LazyLock;

/// Lisa's workflow document, embedded at compile time
pub static LISA_WORKFLOW: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{PURPOSE_PARAGRAPH}\n\n{}",
        include_str!("../data/lisa-workflow.md")
    )
});

/// Exact outgoing Lisa templates accepted as proof of an unmodified install.
/// Keep only byte-distinct generations; current content is handled separately.
///
/// This list is also the removal warrant for a project's stale
/// `docs/knowledge/rdspi-workflow.md`: 0.5.0 installs the document under a new
/// name, and only bytes on this list may be deleted from the old one. The files
/// keep their original names because that is what they are — the generations
/// Lisa shipped while the document was called `rdspi-workflow.md`.
pub(crate) const LEGACY_WORKFLOWS: &[&str] = &[
    include_str!("../data/legacy/rdspi-workflow-v0.2.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.md"),
    include_str!("../data/legacy/rdspi-workflow-v0.4.4.md"),
];

/// The hooks setup guide, embedded at compile time. Printed by `lisa hooks-guide`.
pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");

/// The `--json` document guide, embedded at compile time. Printed by
/// `lisa json-guide`. The same guide-level treatment `HOOKS_GUIDE` gives the
/// signal contract, for the contract a second reader builds against.
pub const JSON_GUIDE: &str = include_str!("../data/json-guide.md");

/// The compiled WASM plugin, embedded at compile time via build.rs
pub const PLUGIN_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lisa.wasm"));

/// The lease guard every signal hook opens with. Each hook is a standalone
/// script and carries its own copy; this constant is the single text they are
/// all checked against, so a hook cannot quietly drop it.
///
/// A signal belongs to the project the pane was leased from. The hook runs in
/// the agent's working directory, which is a different fact: an agent reading a
/// second repository — ordinary, and encouraged — used to write `pane-N.ack`
/// into whatever tree it was standing in, and `mkdir -p` created
/// `.lisa/signals/` there to hold it. The true project lost a heartbeat; the
/// innocent project gained a fresh signal its own launcher then refused to run
/// against, from a pane numbering it does not share.
///
/// So the hook takes its project from `$LISA_PROJECT`, exported on the pane's
/// launch line beside `$LISA_PANE_ID`, and never from the working directory.
/// Both facts must be present and `$LISA_PROJECT` must already be a Lisa
/// project: a hook that cannot name its lease writes nothing at all, because the
/// alternative — a plausible signal file in a directory Lisa does not manage —
/// is the failure itself, not a degraded form of it. An operator's own session
/// has neither variable and stays silent exactly as before.
#[cfg(test)]
pub(crate) const HOOK_LEASE_GUARD: &str = r#"# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0"#;

/// The on-idle hook script, called by Claude Code's idle_prompt notification.
/// Writes a signal file so the plugin knows which session finished its work.
pub const ON_IDLE_HOOK: &str = r#"#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification.
# Writes a signal file so the plugin knows this session finished its work.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
"#;

pub(crate) const LEGACY_ON_IDLE_HOOKS: &[&str] = &[
    // 0.5.0-rc.3 and earlier. Resolved `.lisa/signals` relative to the agent's
    // working directory, so a session that stepped into another repository
    // signalled there instead (S-061-01).
    r#"#!/bin/sh
# Lisa idle signal hook — called by Claude Code on idle_prompt notification.
# Writes a signal file so the plugin knows this session finished its work.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.idle"
fi
"#,
];

/// The on-stop hook script, called by the native client's Stop event.
/// Fires when Claude or Codex finishes responding (ready for input).
///
/// Beyond writing the `.stopped` signal it forwards the Stop payload (piped on
/// stdin, carrying `transcript_path`) to `lisa capture-usage`, which sums the
/// session's token usage into the provider-specific append-only capture ledger.
/// Stop fires per *turn*, not per tool call, so the heartbeat hook stays trivial.
/// Stops without observable transcript usage append a no-capture marker, while
/// malformed identity or persistence errors remain visible to the operator.
pub const ON_STOP_HOOK: &str = r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Captures session token usage for the provenance ledger (T-027-02) first,
# then writes the stop signal file. Order matters: the stop signal is what
# lets the scheduler act on this pane (advance the ticket, end the session),
# so the capture must already be durable when the signal appears — a session
# ended mid-capture lost 8 of 9 usage records in the 0.4.4-rc.8 field leg.

# An operator's own session has no Lisa pane and no leased project: nothing to
# attribute, so stay silent — and drain stdin, because the caller is writing to
# it. Inside a Lisa-managed pane, capture errors remain loud on purpose (silent
# no-writes were the 2026-07-09 attribution incident).
if [ -z "${LISA_PANE_ID:-}" ] || [ -z "${LISA_PROJECT:-}" ] || [ ! -d "$LISA_PROJECT/.lisa" ]; then
    cat >/dev/null
    exit 0
fi

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in.
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer, naming the leased project so the capture ledger lands beside the
# signals rather than in the tree the agent happens to be reading.
# No-capture markers and capture errors remain visible to operators.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage --cwd "$LISA_PROJECT"

# Signal last: the pane only reads as stopped once its usage is recorded.
# A capture failure still signals (the scheduler must never stall on it);
# its error above stays visible in the pane.
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
"#;

/// Exact prior Stop-hook generations eligible for safe `lisa init` upgrades.
pub(crate) const LEGACY_ON_STOP_HOOKS: &[&str] = &[
    // 0.5.0-rc.3 and earlier: signal directory and capture ledger both resolved
    // against the agent's working directory (S-061-01).
    r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Captures session token usage for the provenance ledger (T-027-02) first,
# then writes the stop signal file. Order matters: the stop signal is what
# lets the scheduler act on this pane (advance the ticket, end the session),
# so the capture must already be durable when the signal appears — a session
# ended mid-capture lost 8 of 9 usage records in the 0.4.4-rc.8 field leg.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

# An operator's own session has no Lisa pane: nothing to attribute, so stay
# silent. Inside a Lisa-managed pane, capture errors remain loud on purpose
# (silent no-writes were the 2026-07-09 attribution incident).
if [ -z "${LISA_PANE_ID:-}" ]; then
    cat >/dev/null
    exit 0
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. No-capture markers and capture errors remain visible to operators.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage

# Signal last: the pane only reads as stopped once its usage is recorded.
# A capture failure still signals (the scheduler must never stall on it);
# its error above stays visible in the pane.
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
"#,
    r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input, and
# captures session token usage for the provenance ledger (T-027-02).

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi

# An operator's own session has no Lisa pane: nothing to attribute, so stay
# silent. Inside a Lisa-managed pane, capture errors remain loud on purpose
# (silent no-writes were the 2026-07-09 attribution incident).
if [ -z "${LISA_PANE_ID:-}" ]; then
    cat >/dev/null
    exit 0
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. No-capture markers and capture errors remain visible to operators.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage
"#,
    r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input, and
# captures session token usage for the provenance ledger (T-027-02).

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. No-capture markers and capture errors remain visible to operators.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage
"#,
    r#"#!/bin/sh
# Lisa stop signal hook — called when the native agent finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input, and
# captures session token usage for the provenance ledger (T-027-02).

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi

# Forward the Stop payload (stdin: includes transcript_path) to the usage
# capturer. Best-effort: never fail the session if lisa is absent.
in=$(cat)
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage 2>/dev/null || true
"#,
    r#"#!/bin/sh
# Lisa stop signal hook — called by Claude Code when it finishes responding.
# Writes a signal file so the plugin knows the pane is ready for input.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.stopped"
fi
"#,
];

/// The on-clear hook script, called by the native client's SessionStart[clear] event.
/// Fires after /clear is processed (context cleared).
pub const ON_CLEAR_HOOK: &str = r#"#!/bin/sh
# Lisa clear signal hook — called after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
"#;

pub(crate) const LEGACY_ON_CLEAR_HOOKS: &[&str] = &[
    // 0.5.0-rc.3 and earlier: relative signal directory (S-061-01).
    r#"#!/bin/sh
# Lisa clear signal hook — called after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#,
    r#"#!/bin/sh
# Lisa clear signal hook — called by Claude Code after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
fi
"#,
];

/// The provider-neutral native process-start hook, called by the native
/// client's SessionStart[startup] event. It publishes the scheduler-owned
/// attempt lease only when the immutable launch identity still matches the
/// pane marker, so a stale predecessor cannot borrow a successor's identity.
///
/// It compares the bytes it is about to publish, not the file it read them
/// from. Reading the marker, then copying it, leaves a window in which the
/// scheduler publishes a successor between the two — and the process would then
/// publish a `.started` naming a generation it does not hold, on the strength of
/// a comparison against different bytes. Copy first, compare the copy, rename
/// the thing compared.
pub const ON_START_HOOK: &str = r#"#!/bin/sh
# Lisa process-start signal hook — called when a native agent process starts.
# Publishes only an exact pane/ticket/attempt-scoped scheduler lease.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

if [ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ]; then
    case "$LISA_ATTEMPT_ID" in
        *[!0-9]*) exit 0 ;;
    esac
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")

    # Publish only bytes that were compared: take the copy first, then judge it.
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.started.tmp.$$"
    cp "$marker" "$tmp" 2>/dev/null || { rm -f "$tmp"; exit 0; }
    if [ "$(cat "$tmp")" = "$expected" ]; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.started"
    else
        rm -f "$tmp"
    fi
fi
"#;

pub(crate) const LEGACY_ON_START_HOOKS: &[&str] = &[
    // 0.5.0-rc.3. Correct identity test, relative signal directory: a process
    // that had stepped into another repository read that tree's marker and
    // published its `.started` there (S-061-01).
    r#"#!/bin/sh
# Lisa process-start signal hook — called when a native agent process starts.
# Publishes only an exact pane/ticket/attempt-scoped scheduler lease.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ] && [ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ]; then
    case "$LISA_ATTEMPT_ID" in
        *[!0-9]*) exit 0 ;;
    esac
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")

    # Publish only bytes that were compared: take the copy first, then judge it.
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.started.tmp.$$"
    cp "$marker" "$tmp" 2>/dev/null || { rm -f "$tmp"; exit 0; }
    if [ "$(cat "$tmp")" = "$expected" ]; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.started"
    else
        rm -f "$tmp"
    fi
fi
"#,
    // 0.5.0-rc.2. Same identity test, but it compared the marker file and then
    // copied it again, so a successor published between the two reads could be
    // announced by a process that does not hold it.
    r#"#!/bin/sh
# Lisa process-start signal hook — called when a native agent process starts.
# Publishes only an exact pane/ticket/attempt-scoped scheduler lease.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ] && [ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ]; then
    case "$LISA_ATTEMPT_ID" in
        *[!0-9]*) exit 0 ;;
    esac
    marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
    expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")
    actual=$(cat "$marker" 2>/dev/null) || exit 0
    [ "$actual" = "$expected" ] || exit 0

    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.started.tmp.$$"
    if cp "$marker" "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.started"
    else
        rm -f "$tmp"
    fi
fi
"#,
];

/// The heartbeat hook script, called by the native client's PostToolUse event.
/// Fires after every tool call, and writes two files because it has two
/// separable things to say.
///
/// `pane-<id>.alive` says *a process ran a tool call in this pane*. It names
/// nobody, so there is nothing in it to forge, and it is written before any
/// identity check — the exit policy holds a pane open while a provider is still
/// emitting hooks, and during a recycle a still-resident predecessor stops
/// matching the marker the moment the successor's lease is published.
///
/// `pane-<id>.heartbeat` says *this attempt is making progress*, and the
/// scheduler acts on it: it moves activity clocks and clears the
/// `AskUserQuestion` guard. That claim needs proof, so it is published only when
/// the caller's own immutable launch identity byte-matches the pane marker,
/// exactly as `ON_START_HOOK` does. A process that cannot name itself as the
/// current attempt gets residency and nothing more.
pub const ON_HEARTBEAT_HOOK: &str = r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Residency is unconditional; authority must name itself.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

# "A process is in this pane." Asserts nothing about who, so it needs no
# identity and survives a recycle that has already published the successor's
# lease into the marker this hook used to copy blindly.
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.alive"

# "This attempt is making progress." Published only against the immutable launch
# identity, so a resident predecessor cannot borrow a successor's lease.
[ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ] || exit 0
case "$LISA_ATTEMPT_ID" in
    *[!0-9]*) exit 0 ;;
esac

marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")

# Publish only bytes that were compared: take the copy first, then judge it.
tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat.tmp.$$"
cp "$marker" "$tmp" 2>/dev/null || { rm -f "$tmp"; exit 0; }
if [ "$(cat "$tmp")" = "$expected" ]; then
    mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
else
    rm -f "$tmp"
fi
"#;

pub(crate) const LEGACY_ON_HEARTBEAT_HOOKS: &[&str] = &[
    // 0.5.0-rc.3. Both signals correct in kind, both written relative to the
    // agent's working directory: an agent reading a second repository sent its
    // liveness there and its own project went quiet (S-061-01).
    r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Residency is unconditional; authority must name itself.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

[ -n "$LISA_PANE_ID" ] || exit 0

# "A process is in this pane." Asserts nothing about who, so it needs no
# identity and survives a recycle that has already published the successor's
# lease into the marker this hook used to copy blindly.
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.alive"

# "This attempt is making progress." Published only against the immutable launch
# identity, so a resident predecessor cannot borrow a successor's lease.
[ -n "$LISA_TICKET_ID" ] && [ -n "$LISA_ATTEMPT_ID" ] || exit 0
case "$LISA_ATTEMPT_ID" in
    *[!0-9]*) exit 0 ;;
esac

marker="$SIGNAL_DIR/pane-$LISA_PANE_ID.lease"
expected=$(printf '{"ticket_id":"%s","attempt_id":%s}' "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID")

# Publish only bytes that were compared: take the copy first, then judge it.
tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat.tmp.$$"
cp "$marker" "$tmp" 2>/dev/null || { rm -f "$tmp"; exit 0; }
if [ "$(cat "$tmp")" = "$expected" ]; then
    mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
else
    rm -f "$tmp"
fi
"#,
    // 0.5.0-rc.2. Copies the marker on the strength of `$LISA_PANE_ID` alone, so
    // any process that can run a tool call in the pane publishes a signal
    // carrying whatever attempt the marker names. Listed here so `lisa init`
    // upgrades every board that already has it.
    r#"#!/bin/sh
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
"#,
    r#"#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
"#,
    r#"#!/bin/sh
# Lisa heartbeat signal hook — called by Claude Code after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
"#,
];

/// The native provider `UserPromptSubmit` hook. It preserves the complete JSON
/// payload for the plugin's ticket/generation detector and publishes it with an
/// atomic rename so the polling scheduler never observes a partial document.
pub const ON_ACK_HOOK: &str = r#"#!/bin/sh
# Lisa assignment acknowledgment hook — called before a provider submits a user prompt.
# Writes the raw lifecycle payload for ticket/generation matching in the plugin.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so drain the payload and write
# nothing rather than leave a plausible signal in a repository Lisa does not
# manage.
if [ -z "${LISA_PANE_ID:-}" ] || [ -z "${LISA_PROJECT:-}" ] || [ ! -d "$LISA_PROJECT/.lisa" ]; then
    cat >/dev/null
    exit 0
fi
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
if cat > "$tmp"; then
    mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
else
    rm -f "$tmp"
fi
"#;

pub(crate) const LEGACY_ON_ACK_HOOKS: &[&str] = &[
    // 0.4.0-rc.6, back when the hook was Codex-only and said so. Byte-identical
    // to its successor apart from that first comment line, and listed for the
    // same reason: a board still carrying it would otherwise keep the relative
    // signal directory forever, and this is the hook the field report caught in
    // the act (S-061-01).
    r#"#!/bin/sh
# Lisa Codex acknowledgment hook — called before Codex submits a user prompt.
# Writes the raw lifecycle payload for ticket/generation matching in the plugin.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
    if cat > "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
    else
        rm -f "$tmp"
    fi
fi
"#,
    // 0.5.0-rc.3 and earlier: relative signal directory. This is the hook the
    // field report caught in the act — `pane-1.ack` in a repository whose pane
    // numbering it did not share (S-061-01).
    r#"#!/bin/sh
# Lisa assignment acknowledgment hook — called before a provider submits a user prompt.
# Writes the raw lifecycle payload for ticket/generation matching in the plugin.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
    if cat > "$tmp"; then
        mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
    else
        rm -f "$tmp"
    fi
fi
"#,
];

/// Gitignore content for `.lisa/` runtime state. Signal files and per-provider
/// usage/session artifacts are machine-owned and must never enter the project DAG.
///
/// `scheduler.alive` is the running scheduler's own stamp (T-060-01-01). It says
/// when a scheduler was last here, which is a fact about this machine at this
/// moment and never about the project, so it belongs in history even less than
/// the signal files do.
pub const LISA_GITIGNORE: &str =
    "signals/\nattempts/\nclaude/\ncodex/\nrun-events.jsonl\nrun-baseline.json\nscheduler.alive\n";

/// The on-notify hook SAMPLE, scaffolded as `.lisa/hooks/on-notify.sample`.
/// User-owned attention/completion notification hook. It is deliberately a
/// non-executable `.sample` so the `test -x` guards stay inert until the user
/// opts in (`cp on-notify.sample on-notify && chmod +x on-notify`). lisa never
/// names a notification service outside the commented example below.
pub const ON_NOTIFY_HOOK: &str = r#"#!/bin/sh
# Lisa notify hook (SAMPLE) — copy to on-notify and `chmod +x` to enable.
#
# Contract:  on-notify <event> [detail]      ($1 mirrors $LISA_EVENT)
#
# Environment (all events):
#   LISA_EVENT    complete | attention
#   LISA_PROJECT  absolute project root (identifies which loop; you may `cd` to it)
# complete:
#   LISA_TICKETS_DONE   number of tickets completed
#   LISA_DURATION_SECS  loop duration in seconds
# attention:
#   LISA_REASON      question | permission | idle-without-artifact
#   LISA_PANE_ID     the originating pane
#   LISA_TICKET_ID   ticket the agent is working on, when known
#   LISA_QUESTION_HEADER  short label of the question (question reason only)
#
# Payload on STDIN: for the question/permission reasons, the full Claude Code
# hook JSON is piped to this script's stdin, so you can extract anything (e.g.
# every question + its options) with sed/jq:  payload=$(cat)
#
# Example dispatch (uncomment and customise):
# case "$1" in
#   complete)  msg="lisa [$LISA_PROJECT] done: $LISA_TICKETS_DONE tickets in ${LISA_DURATION_SECS}s" ;;
#   attention) msg="lisa [$LISA_PROJECT] ${LISA_TICKET_ID:-?} needs you (${LISA_REASON}): $2" ;;
# esac
# curl -s -d "$msg" ntfy.sh/your-topic-here

exit 0
"#;

pub(crate) const LEGACY_ON_NOTIFY_HOOKS: &[&str] = &[];

/// Command for the catch-all (matcher-less) `Notification` hook that fires the
/// user-owned `on-notify` hook for permission/attention payloads. POSIX `sh`
/// only (no jq, no bashisms). It reads the payload from stdin once, skips
/// `idle_prompt` payloads (already handled by on-idle.sh + the plugin), records
/// the fact that an interactive gate fired, and then invokes the user hook when
/// the operator opted in. Payload text is never retained in the run ledger.
///
/// Like the hook scripts, it addresses the project it belongs to rather than the
/// directory the agent is standing in: `$LISA_PROJECT` when a Lisa pane exported
/// one, `$PWD` for an operator's own session, which is what `$PWD` always meant
/// here. Nothing is created outside an existing `.lisa/`, so a session reading an
/// unrelated repository no longer leaves a ledger — or runs that project's
/// `on-notify` — behind it.
const NOTIFY_ATTENTION_COMMAND: &str = r#"in=$(cat); case "$in" in *idle_prompt*) : ;; *) proj="${LISA_PROJECT:-$PWD}"; if [ -d "$proj/.lisa" ]; then printf '%s\n' '{"event":"manual-intervention","kind":"permission"}' >> "$proj/.lisa/run-events.jsonl"; fi; if test -x "$proj/.lisa/hooks/on-notify"; then printf '%s' "$in" | LISA_EVENT=attention LISA_REASON=permission LISA_PROJECT="$proj" "$proj/.lisa/hooks/on-notify" attention "$in"; fi ;; esac"#;

/// Command for the `PreToolUse[AskUserQuestion]` hook. POSIX `sh` only (no jq,
/// no bashisms). It (1) **unconditionally** writes `pane-$LISA_PANE_ID.awaiting`
/// so the plugin can suppress injection while the agent is blocked on a question
/// (consumed in T-020-03; harmless unread file until then), and (2) best-effort
/// records a payload-free question event, extracts the first question text, and
/// fires the opt-in `on-notify attention` with `LISA_REASON=question`. Only the
/// notify dispatch is `test -x`-gated — signal and event writes must work even
/// when the user never enabled `on-notify`. A question
/// containing an escaped `\"` truncates the greedy-free `[^"]*` capture; that
/// degrades to the generic detail, never a hard failure (design Q3).
///
/// The `.awaiting` signal is a scheduler input like every other signal, so it
/// obeys the hook scripts' rule exactly: it is written to `$LISA_PROJECT`, the
/// project the pane was leased from, and not written at all when the pane cannot
/// name one. The ledger and the opt-in notify dispatch keep the `$PWD` meaning
/// an operator's own session has always had.
const NOTIFY_QUESTION_COMMAND: &str = r#"proj="${LISA_PROJECT:-$PWD}"; if [ -n "$LISA_PANE_ID" ] && [ -n "$LISA_PROJECT" ] && [ -d "$LISA_PROJECT/.lisa" ]; then mkdir -p "$LISA_PROJECT/.lisa/signals"; date -u +%Y-%m-%dT%H:%M:%SZ > "$LISA_PROJECT/.lisa/signals/pane-$LISA_PANE_ID.awaiting"; fi; if [ -d "$proj/.lisa" ]; then printf '%s\n' '{"event":"manual-intervention","kind":"question"}' >> "$proj/.lisa/run-events.jsonl"; fi; in=$(cat); q=$(printf '%s' "$in" | sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p'); [ -z "$q" ] && q="agent is asking a question"; hdr=$(printf '%s' "$in" | sed -n 's/.*"header":[ ]*"\([^"]*\)".*/\1/p'); if test -x "$proj/.lisa/hooks/on-notify"; then printf '%s' "$in" | LISA_EVENT=attention LISA_REASON=question LISA_PROJECT="$proj" LISA_QUESTION_HEADER="$hdr" "$proj/.lisa/hooks/on-notify" attention "$q"; fi"#;

/// Generate .claude/settings.local.json with Stop, SessionStart, UserPromptSubmit, Notification
/// (idle_prompt + catch-all attention), PostToolUse heartbeat, and
/// PreToolUse[AskUserQuestion] hooks.
/// Hook commands use `test -x` guards so they succeed silently if the scripts
/// haven't been created yet (e.g. settings.local.json exists before `lisa init`).
pub fn settings_local_json() -> String {
    merge_hooks(r#"{"hooks":{}}"#).expect("empty Lisa settings template is valid JSON")
}

/// Generate `.codex/hooks.json` for the native interactive Codex adapter.
///
/// Codex has no `idle_prompt` or `AskUserQuestion` hook equivalent, so the TUI
/// installs only lifecycle signals it can state truthfully: prompt submission,
/// tool progress, turn completion, and `/clear` completion. Shared scripts
/// attribute events through `LISA_PANE_ID`; the ack script preserves raw JSON.
pub fn codex_hooks_json() -> String {
    r#"{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh"
          }
        ]
      },
      {
        "matcher": "clear",
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
          }
        ]
      }
    ]
  }
}
"#
    .to_string()
}

/// Ensure a single hook entry exists in the hooks object with the correct command.
/// For hooks with a matcher (SessionStart, Notification), deduplication checks the matcher value.
/// For hooks without a matcher (Stop), deduplication checks the command path.
/// If the hook exists but uses an old bare-path command, it is upgraded in place.
fn ensure_hook(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event_type: &str,
    matcher: Option<&str>,
    command: &str,
) {
    let entries = hooks_obj
        .entry(event_type)
        .or_insert_with(|| serde_json::json!([]));
    let arr = match entries.as_array_mut() {
        Some(a) => a,
        None => return,
    };

    // Extract the script path from the command for dedup matching.
    // Commands may be bare paths (".lisa/hooks/on-stop.sh") or guarded
    // ("test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh").
    // Match on the script filename to handle both forms.
    let script_path = [
        ".lisa/hooks/on-notify",
        ".lisa/hooks/on-stop.sh",
        ".lisa/hooks/on-start.sh",
        ".lisa/hooks/on-clear.sh",
        ".lisa/hooks/on-idle.sh",
        ".lisa/hooks/on-heartbeat.sh",
        ".lisa/hooks/on-ack.sh",
    ]
    .into_iter()
    .find(|path| command.contains(path))
    .unwrap_or_else(|| command.rsplit("&& ").next().unwrap_or(command).trim());

    // Find the matching entry index (if any)
    let found_idx = match matcher {
        Some(m) => arr
            .iter()
            .position(|entry| entry.get("matcher").and_then(|v| v.as_str()) == Some(m)),
        None => arr.iter().position(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|hooks| {
                    hooks.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .is_some_and(|c| c.contains(script_path))
                    })
                })
        }),
    };

    match found_idx {
        Some(idx) => {
            // Entry exists — upgrade Lisa's old bare-path command in place. A
            // user hook may share the same matcher; keep it and append Lisa's
            // command instead of treating the matcher alone as a duplicate.
            if let Some(hooks_arr) = arr[idx].get_mut("hooks").and_then(|h| h.as_array_mut()) {
                let mut lisa_hook_found = false;
                for hook in hooks_arr.iter_mut() {
                    if let Some(cmd_val) = hook.get_mut("command") {
                        if let Some(existing) = cmd_val.as_str() {
                            if existing.contains(script_path) {
                                lisa_hook_found = true;
                                if existing != command {
                                    *cmd_val = serde_json::json!(command);
                                }
                            }
                        }
                    }
                }
                if !lisa_hook_found {
                    hooks_arr.push(serde_json::json!({
                        "type": "command",
                        "command": command
                    }));
                }
            }
        }
        None => {
            // Entry doesn't exist — create it
            let mut entry = serde_json::Map::new();
            if let Some(m) = matcher {
                entry.insert("matcher".to_string(), serde_json::json!(m));
            }
            entry.insert(
                "hooks".to_string(),
                serde_json::json!([{
                    "type": "command",
                    "command": command
                }]),
            );
            arr.push(serde_json::Value::Object(entry));
        }
    }
}

/// Merge all Lisa hooks (Stop, SessionStart[clear], Notification[idle_prompt],
/// PostToolUse heartbeat, the catch-all Notification[attention] binding, and the
/// PreToolUse[AskUserQuestion] question binding) into an existing
/// settings.local.json. Returns the updated JSON string, or an error if the JSON
/// is malformed.
pub fn merge_hooks(existing_json: &str) -> Result<String, String> {
    let mut root: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| format!("invalid JSON in settings.local.json: {}", e))?;

    let obj = root
        .as_object_mut()
        .ok_or("settings.local.json root is not an object")?;

    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or("settings.local.json 'hooks' is not an object")?;

    ensure_hook(
        hooks_obj,
        "Stop",
        None,
        "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("startup"),
        "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("clear"),
        "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh",
    );
    ensure_hook(
        hooks_obj,
        "Notification",
        Some("idle_prompt"),
        "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh",
    );
    ensure_hook(
        hooks_obj,
        "PostToolUse",
        None,
        "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh",
    );
    ensure_hook(
        hooks_obj,
        "UserPromptSubmit",
        None,
        "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh",
    );
    // Catch-all (matcher-less) Notification entry for permission/attention payloads.
    // Distinct from the idle_prompt entry above: ensure_hook dedups a matcher-less
    // entry by its command substring, which references on-notify (not on-idle.sh),
    // so the two coexist and re-runs stay idempotent.
    ensure_hook(hooks_obj, "Notification", None, NOTIFY_ATTENTION_COMMAND);
    // PreToolUse[AskUserQuestion]: fires the on-notify attention path with
    // LISA_REASON=question and writes the pane-<id>.awaiting signal. It carries a
    // matcher, so ensure_hook dedups by matcher value (idempotent, coexists with
    // the matcher-less PostToolUse heartbeat — a different event key entirely).
    ensure_hook(
        hooks_obj,
        "PreToolUse",
        Some("AskUserQuestion"),
        NOTIFY_QUESTION_COMMAND,
    );

    serde_json::to_string_pretty(&root).map_err(|e| format!("failed to serialize JSON: {}", e))
}

/// Merge the native Codex lifecycle hooks into an existing `.codex/hooks.json`
/// without disturbing user-owned hook groups.
pub fn merge_codex_hooks(existing_json: &str) -> Result<String, String> {
    let mut root: serde_json::Value = serde_json::from_str(existing_json)
        .map_err(|e| format!("invalid JSON in hooks.json: {}", e))?;
    let obj = root
        .as_object_mut()
        .ok_or("hooks.json root is not an object")?;
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or("hooks.json 'hooks' is not an object")?;

    ensure_hook(
        hooks_obj,
        "Stop",
        None,
        "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("startup"),
        "test -x .lisa/hooks/on-start.sh && .lisa/hooks/on-start.sh",
    );
    ensure_hook(
        hooks_obj,
        "SessionStart",
        Some("clear"),
        "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh",
    );
    ensure_hook(
        hooks_obj,
        "PostToolUse",
        Some(".*"),
        "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh",
    );
    ensure_hook(
        hooks_obj,
        "UserPromptSubmit",
        None,
        "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh",
    );

    serde_json::to_string_pretty(&root).map_err(|e| format!("failed to serialize JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lisa_core::context::ROLE_CONTRACT;
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_workflow_document_embedded() {
        assert!(LISA_WORKFLOW.contains("How a ticket moves"));
        assert!(LISA_WORKFLOW.contains("Implement"));
        assert!(LISA_WORKFLOW.contains("Review"));
        assert_eq!(
            LISA_WORKFLOW.as_str(),
            include_str!("../../../docs/knowledge/lisa-workflow.md"),
            "checked-in project context must match the rendered template"
        );
    }

    /// The document describes the board Lisa actually runs: four states, one
    /// artifact pair, no phase that exists to be written about.
    ///
    /// The four retired phases were most of the outgoing document's bulk, and a
    /// replacement that quietly refilled it would satisfy every other assertion
    /// here while undoing the point — hence the length bound, measured against
    /// the 146-line document this one replaces.
    #[test]
    fn the_workflow_document_describes_the_board_lisa_actually_runs() {
        let document = LISA_WORKFLOW.as_str();

        // Four states, named in order, and the retired four gone entirely.
        let phase_line = "`phase`: `ready` | `implement` | `review` | `done`";
        assert!(
            document.contains(phase_line),
            "phase list must be the four live states"
        );
        for retired in ["research.md", "design.md", "structure.md", "plan.md"] {
            assert!(
                !document.contains(retired),
                "{retired} is not an artifact any more"
            );
        }

        // One artifact pair, and no per-phase writing duty besides it.
        assert!(document.contains("Implement produces no document. The commits are the record."));
        assert!(document.contains("`review.md` and `review-disposition.json`"));

        // Lisa still detects the artifact and advances the ticket itself.
        assert!(document.contains(
            "Lisa detects `review.md` and advances the ticket's `phase` field in the YAML frontmatter automatically."
        ));

        // What replaced the artifacts-are-insurance promise, stated where the
        // person it affects reads it.
        assert!(document.contains("Nothing in your work directory\nis a resume point."));
        assert!(
            document.contains("every\n  `lisa commit-ticket` you ran is already on the branch.")
        );
        assert!(document.contains("The ticket restarts from the beginning."));

        assert!(
            document.lines().count() < 146,
            "the document must be shorter than the 146-line original, got {}",
            document.lines().count()
        );
    }

    #[test]
    fn test_review_disposition_contract_is_injected() {
        assert!(LISA_WORKFLOW.contains("review-disposition.json"));
        assert!(LISA_WORKFLOW.contains(r#"{"disposition":"pass","reason":null}"#));
        assert!(LISA_WORKFLOW.contains(
            r#"{"disposition":"note","reason":null,"criterion_quote":"<exact disputed criterion>","evidence_citation":"<repository-relative evidence path>","summary":"<plain one-sentence summary>"}"#
        ));
        assert!(LISA_WORKFLOW.contains(
            r#"{"disposition":"block","reason":"<non-empty actionable reason>","remedy_owner":"<agent|operator|world>","ask":"<one-sentence action>","steps":["<optional exact step>"],"check":"<read-only verification command>","check_timeout_secs":<optional seconds the check needs>}"#
        ));
        assert!(LISA_WORKFLOW
            .contains("A pass with a reason, or a block without a non-empty reason, is invalid."));
        assert!(LISA_WORKFLOW.contains("Choose `remedy_owner` honestly"));
        assert!(LISA_WORKFLOW.contains(
            "`agent` when another coding attempt can perform the remedy, `operator` when a person must act, and `world` when external reality must change"
        ));
        assert!(
            LISA_WORKFLOW.contains("Supply a `check` whenever the remedy is externally observable")
        );
        assert!(LISA_WORKFLOW.contains(
            "Write the `ask` as one sentence addressed to a person who didn't do the work, naming the action rather than the subsystem."
        ));
        assert!(LISA_WORKFLOW.contains("no stable Pages artifact has been deployed"));
        assert!(LISA_WORKFLOW.contains(
            "Lisa needs the release published; run: just release. Lisa will notice on its own once it's live."
        ));
        assert!(LISA_WORKFLOW.contains(
            "Write for a bystander: say plainly what they should do. Keep subsystem names, measurements, and other jargon in `reason` or `steps`, not the `ask`."
        ));
        assert!(LISA_WORKFLOW.contains(
            "The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass."
        ));
        assert!(LISA_WORKFLOW.contains("lisa check-disposition <ticket-id>"));
        assert!(LISA_WORKFLOW.contains("Correct every reported issue before finishing Review."));
    }

    /// The check contract a reviewer writes against, asserted against the code
    /// that enforces it.
    ///
    /// Every number here is formatted from the constant rather than copied, so
    /// the document cannot quietly drift from the runtime. That drift is the
    /// failure this ticket exists to prevent: the constraints were real, nobody
    /// had written them down, and the person who found them was the one who
    /// could not fix them.
    #[test]
    fn the_documented_check_contract_matches_the_code_that_enforces_it() {
        use lisa_core::disposition::{DEFAULT_CHECK_BUDGET_SECS, MAX_CHECK_BUDGET_SECS};

        // Where it runs and what it sees — the T-056-01-02 root cause, stated.
        assert!(LISA_WORKFLOW.contains(
            "**Where it runs:** the project root, the same directory you are working in."
        ));
        assert!(LISA_WORKFLOW.contains(
            "**What it sees:** every file that is really there — build output, fetched dependencies, and anything else `.gitignore` hides from git."
        ));

        // Whether a check may write, decided and stated.
        assert!(LISA_WORKFLOW.contains(
            "**Writes:** a check must only look. Lisa runs it in the live project and cannot stop it writing"
        ));
        assert!(LISA_WORKFLOW.contains("`npm run build && npm run verify` is not a check"));

        // The budget, in the document, equal to the constants in force.
        assert!(
            LISA_WORKFLOW.contains(&format!(
                "**How long:** {DEFAULT_CHECK_BUDGET_SECS} seconds."
            )),
            "the documented default budget must equal DEFAULT_CHECK_BUDGET_SECS"
        );
        assert!(
            LISA_WORKFLOW.contains(&format!(
                "declares `\"check_timeout_secs\": <seconds>`, up to {MAX_CHECK_BUDGET_SECS} ({} minutes)",
                MAX_CHECK_BUDGET_SECS / 60
            )),
            "the documented cap must equal MAX_CHECK_BUDGET_SECS"
        );
        assert!(LISA_WORKFLOW.contains("Lisa stops the check and says how long it waited."));

        // The exit codes the runner distinguishes.
        assert!(LISA_WORKFLOW
            .contains("`2`, `126`, `127`, or death by a signal mean the check could not look"));

        // And that recording a check now runs it.
        assert!(LISA_WORKFLOW.contains(
            "`lisa check-disposition` runs your recorded check under exactly this contract and refuses one that can never pass"
        ));
    }

    #[test]
    fn test_hooks_guide_embedded() {
        assert!(HOOKS_GUIDE.contains("on-notify"));
        assert!(HOOKS_GUIDE.contains("LISA_EVENT"));
    }

    #[test]
    fn test_agent_contract_names_both_roles_and_both_prohibitions() {
        let contract = ROLE_CONTRACT;

        // The two-role fork must name both roles and both prohibitions.
        assert!(contract.contains("Working a ticket for Lisa?"));
        assert!(contract.contains("Helping set the project up?"));
        assert!(contract.contains("Do not implement tickets yourself"));
        assert!(contract.contains("do not run `lisa loop`"));
        assert!(contract.contains("their own terminal pane or window"));
    }

    #[test]
    fn test_injected_context_is_purpose_first_and_copy_is_single_sourced() {
        for (name, context) in [("workflow document", LISA_WORKFLOW.as_str())] {
            assert_eq!(
                context.matches(PURPOSE_PARAGRAPH).count(),
                1,
                "{name} should contain one canonical purpose paragraph"
            );
            let lower = context.to_lowercase();
            let purpose_position = lower.find(&PURPOSE_PARAGRAPH.to_lowercase()).unwrap();
            for mechanism in ["dag", "phase", "scheduling", "zellij"] {
                if let Some(position) = lower.find(mechanism) {
                    assert!(
                        purpose_position < position,
                        "{name}: purpose must precede {mechanism}"
                    );
                }
            }
        }

        let template_sources = [
            include_str!("../../lisa-core/src/context.rs"),
            include_str!("../../lisa-plugin/src/lib.rs"),
            include_str!("templates.rs"),
            include_str!("../data/lisa-workflow.md"),
        ];
        assert_eq!(
            template_sources
                .iter()
                .map(|source| source.matches(PURPOSE_PARAGRAPH).count())
                .sum::<usize>(),
            1,
            "canonical prose must have exactly one template source"
        );
    }

    /// Every shipped signal hook resolves its directory from the lease, and none
    /// of them can reach `mkdir` without one. A hook that regains a relative
    /// `SIGNAL_DIR` is the S-061-01 field failure exactly: it succeeds, and
    /// leaves a plausible signal in a repository nobody pointed Lisa at.
    #[test]
    fn every_signal_hook_writes_where_its_lease_is() {
        for (name, hook) in [
            ("on-idle.sh", ON_IDLE_HOOK),
            ("on-stop.sh", ON_STOP_HOOK),
            ("on-clear.sh", ON_CLEAR_HOOK),
            ("on-start.sh", ON_START_HOOK),
            ("on-heartbeat.sh", ON_HEARTBEAT_HOOK),
            ("on-ack.sh", ON_ACK_HOOK),
        ] {
            assert!(
                hook.contains(r#"SIGNAL_DIR="$LISA_PROJECT/.lisa/signals""#),
                "{name} must take its signal directory from the leased project"
            );
            assert!(
                !hook.contains(r#"SIGNAL_DIR=".lisa/signals""#),
                "{name} must not resolve signals against the agent's working directory"
            );
            // Written `[ -d … ]` where the hook proceeds on success and
            // `[ ! -d … ]` where it bails, but the same fact either way.
            assert!(
                hook.contains(r#"-d "$LISA_PROJECT/.lisa" ]"#),
                "{name} must refuse to create a signal tree outside a Lisa project"
            );
            let guard = hook
                .find(r#"-d "$LISA_PROJECT/.lisa" ]"#)
                .expect("guard present");
            assert!(
                guard < hook.find("mkdir -p").expect("mkdir present"),
                "{name} must prove its lease before it creates anything"
            );
        }
        // The two hooks whose event arrives with a payload drain stdin on the
        // silent path, so an unattributable session never breaks its caller.
        for (name, hook) in [("on-stop.sh", ON_STOP_HOOK), ("on-ack.sh", ON_ACK_HOOK)] {
            assert!(
                hook.contains("cat >/dev/null"),
                "{name} must drain its payload when it stays silent"
            );
        }
    }

    /// The lease guard is copied into each standalone script; this is the one
    /// text they are all measured against.
    #[test]
    fn the_lease_guard_is_one_text() {
        for (name, hook) in [
            ("on-idle.sh", ON_IDLE_HOOK),
            ("on-clear.sh", ON_CLEAR_HOOK),
            ("on-start.sh", ON_START_HOOK),
            ("on-heartbeat.sh", ON_HEARTBEAT_HOOK),
        ] {
            assert!(
                hook.contains(HOOK_LEASE_GUARD),
                "{name} must carry the lease guard verbatim"
            );
        }
    }

    #[test]
    fn test_on_idle_hook_content() {
        assert!(ON_IDLE_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_IDLE_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_IDLE_HOOK.contains(".lisa/signals"));
        assert!(ON_IDLE_HOOK.contains(".idle"));
    }

    #[test]
    fn test_on_stop_hook_content() {
        assert!(ON_STOP_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_STOP_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_STOP_HOOK.contains(".lisa/signals"));
        assert!(ON_STOP_HOOK.contains(".stopped"));
    }

    #[test]
    fn test_on_clear_hook_content() {
        assert!(ON_CLEAR_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_CLEAR_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_CLEAR_HOOK.contains(".lisa/signals"));
        assert!(ON_CLEAR_HOOK.contains(".cleared"));
    }

    #[test]
    fn test_on_start_hook_content() {
        assert!(ON_START_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_START_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_START_HOOK.contains("LISA_TICKET_ID"));
        assert!(ON_START_HOOK.contains("LISA_ATTEMPT_ID"));
        assert!(ON_START_HOOK.contains(".lease"));
        assert!(ON_START_HOOK.contains(".started"));
        assert!(ON_START_HOOK.contains("mv \"$tmp\""));
    }

    fn run_start_hook(marker: Option<&str>, ticket_id: &str, attempt_id: &str) -> bool {
        let root = tempfile::tempdir().unwrap();
        let signals = root.path().join(".lisa/signals");
        fs::create_dir_all(&signals).unwrap();
        let script = root.path().join("on-start.sh");
        fs::write(&script, ON_START_HOOK).unwrap();
        if let Some(body) = marker {
            fs::write(signals.join("pane-7.lease"), body).unwrap();
        }
        // Run from somewhere else entirely: the pane's project is the one it was
        // leased from, never the directory the agent is standing in.
        let elsewhere = tempfile::tempdir().unwrap();
        let status = Command::new("/bin/sh")
            .arg(&script)
            .current_dir(elsewhere.path())
            .env("LISA_PROJECT", root.path())
            .env("LISA_PANE_ID", "7")
            .env("LISA_TICKET_ID", ticket_id)
            .env("LISA_ATTEMPT_ID", attempt_id)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(
            !elsewhere.path().join(".lisa").exists(),
            "the hook must not create a signal directory where it happens to run"
        );
        let started = signals.join("pane-7.started");
        if started.exists() {
            assert_eq!(fs::read_to_string(&started).unwrap(), marker.unwrap());
            true
        } else {
            assert!(fs::read_dir(&signals)
                .unwrap()
                .flatten()
                .all(|entry| !entry.file_name().to_string_lossy().contains("started.tmp")));
            false
        }
    }

    #[test]
    fn test_start_hook_fixture_accepts_only_matching_attempt() {
        let matching = r#"{"ticket_id":"T-035-01-01","attempt_id":2}"#;
        assert!(run_start_hook(Some(matching), "T-035-01-01", "2"));
        assert!(!run_start_hook(Some(matching), "T-035-01-01", "1"));
        assert!(!run_start_hook(Some(matching), "T-OTHER", "2"));
        assert!(!run_start_hook(None, "T-035-01-01", "2"));
        assert!(!run_start_hook(Some(matching), "T-035-01-01", "bad"));
    }

    #[test]
    fn test_no_provider_start_produces_no_signal() {
        let root = tempfile::tempdir().unwrap();
        let signals = root.path().join(".lisa/signals");
        fs::create_dir_all(&signals).unwrap();
        fs::write(
            signals.join("pane-7.lease"),
            r#"{"ticket_id":"T-035-01-01","attempt_id":2}"#,
        )
        .unwrap();
        assert!(!signals.join("pane-7.started").exists());
    }

    #[test]
    fn test_on_heartbeat_hook_content() {
        assert!(ON_HEARTBEAT_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_HEARTBEAT_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_HEARTBEAT_HOOK.contains("LISA_TICKET_ID"));
        assert!(ON_HEARTBEAT_HOOK.contains("LISA_ATTEMPT_ID"));
        assert!(ON_HEARTBEAT_HOOK.contains(".lisa/signals"));
        assert!(ON_HEARTBEAT_HOOK.contains(".alive"));
        assert!(ON_HEARTBEAT_HOOK.contains(".heartbeat"));
        assert!(ON_HEARTBEAT_HOOK.contains(".lease"));
        assert!(ON_HEARTBEAT_HOOK.contains("mv \"$tmp\""));
        // Residency is written before the first thing that can exit early, or a
        // caller who cannot name itself would produce no evidence at all.
        assert!(
            ON_HEARTBEAT_HOOK.find(".alive").unwrap()
                < ON_HEARTBEAT_HOOK.find("LISA_TICKET_ID").unwrap(),
            "the presence-only write must precede every identity check"
        );
    }

    /// Drive the real script. Returns `(alive, heartbeat)` — what a process with
    /// this launch identity actually published into a pane whose marker names
    /// `marker`.
    fn run_heartbeat_hook(
        marker: Option<&str>,
        ticket_id: &str,
        attempt_id: &str,
    ) -> (bool, Option<String>) {
        let root = tempfile::tempdir().unwrap();
        let signals = root.path().join(".lisa/signals");
        fs::create_dir_all(&signals).unwrap();
        let script = root.path().join("on-heartbeat.sh");
        fs::write(&script, ON_HEARTBEAT_HOOK).unwrap();
        if let Some(body) = marker {
            fs::write(signals.join("pane-7.lease"), body).unwrap();
        }
        // Run from somewhere else entirely: the pane's project is the one it was
        // leased from, never the directory the agent is standing in.
        let elsewhere = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sh");
        command
            .arg(&script)
            .current_dir(elsewhere.path())
            .env("LISA_PROJECT", root.path())
            .env("LISA_PANE_ID", "7");
        if !ticket_id.is_empty() {
            command.env("LISA_TICKET_ID", ticket_id);
        }
        if !attempt_id.is_empty() {
            command.env("LISA_ATTEMPT_ID", attempt_id);
        }
        let status = command.status().unwrap();
        assert!(status.success());
        assert!(
            !elsewhere.path().join(".lisa").exists(),
            "the hook must not create a signal directory where it happens to run"
        );
        assert!(
            fs::read_dir(&signals).unwrap().flatten().all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .contains("heartbeat.tmp")),
            "no temporary file may survive the hook"
        );
        (
            signals.join("pane-7.alive").exists(),
            fs::read_to_string(signals.join("pane-7.heartbeat")).ok(),
        )
    }

    /// The forgery this hook used to permit, driven against the script itself.
    /// Only the process that can name the attempt in the marker publishes a
    /// heartbeat; everyone else in the pane publishes residency and stops there.
    #[test]
    fn test_heartbeat_hook_publishes_progress_only_for_the_attempt_it_names() {
        let marker = r#"{"ticket_id":"T-058-01-01","attempt_id":4}"#;

        // The attempt the marker names.
        assert_eq!(
            run_heartbeat_hook(Some(marker), "T-058-01-01", "4"),
            (true, Some(marker.to_string()))
        );

        // A resident predecessor, after the successor's marker was published.
        assert_eq!(
            run_heartbeat_hook(Some(marker), "T-058-01-01", "3"),
            (true, None)
        );
        // A process working on another ticket entirely.
        assert_eq!(
            run_heartbeat_hook(Some(marker), "T-OTHER", "4"),
            (true, None)
        );
        // A process with no launch identity at all — an operator's own session
        // inside a Lisa pane, or any process that inherited only the pane id.
        assert_eq!(run_heartbeat_hook(Some(marker), "", ""), (true, None));
        assert_eq!(
            run_heartbeat_hook(Some(marker), "T-058-01-01", ""),
            (true, None)
        );
        // A non-numeric attempt id cannot be spliced into the expected JSON.
        assert_eq!(
            run_heartbeat_hook(Some(marker), "T-058-01-01", "4 or bust"),
            (true, None)
        );
        // No marker: nothing to prove identity against, residency still true.
        assert_eq!(run_heartbeat_hook(None, "T-058-01-01", "4"), (true, None));
    }

    #[test]
    fn test_on_ack_hook_preserves_payload_atomically() {
        assert!(ON_ACK_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_ACK_HOOK.contains("LISA_PANE_ID"));
        assert!(ON_ACK_HOOK.contains("cat > \"$tmp\""));
        assert!(ON_ACK_HOOK.contains("mv \"$tmp\""));
        assert!(ON_ACK_HOOK.contains("pane-$LISA_PANE_ID.ack"));
    }

    #[test]
    fn test_on_notify_hook_content() {
        assert!(ON_NOTIFY_HOOK.starts_with("#!/bin/sh"));
        assert!(ON_NOTIFY_HOOK.contains("on-notify"));
        assert!(ON_NOTIFY_HOOK.contains("LISA_EVENT"));
        assert!(ON_NOTIFY_HOOK.contains("complete"));
        assert!(ON_NOTIFY_HOOK.contains("attention"));
        assert!(ON_NOTIFY_HOOK.contains("LISA_REASON"));
        // ntfy may only appear as a commented example — never active.
        for line in ON_NOTIFY_HOOK.lines() {
            if line.contains("ntfy") {
                assert!(
                    line.trim_start().starts_with('#'),
                    "ntfy must only appear in comments: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_settings_local_json() {
        let json = settings_local_json();
        // Lifecycle and interaction hook types present.
        assert!(json.contains("\"Stop\""));
        assert!(json.contains("\"SessionStart\""));
        assert!(json.contains("\"Notification\""));
        assert!(json.contains("\"PostToolUse\""));
        assert!(json.contains("\"UserPromptSubmit\""));
        // Hook commands
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-start.sh"));
        assert!(json.contains("on-idle.sh"));
        assert!(json.contains("on-heartbeat.sh"));
        assert!(json.contains("on-ack.sh"));
        // Matchers
        assert!(json.contains("\"clear\""));
        assert!(json.contains("idle_prompt"));
        assert!(json.contains(r#""type": "command""#));
        // Catch-all attention Notification binding (alongside idle_prompt).
        assert!(json.contains("on-notify"));
        assert!(json.contains(".lisa/run-events.jsonl"));
        // The generated JSON must embed the exact catch-all command and parse cleanly.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
            "test -x .lisa/hooks/on-ack.sh && .lisa/hooks/on-ack.sh"
        );
        assert_eq!(parsed["hooks"]["SessionStart"][0]["matcher"], "startup");
        assert_eq!(parsed["hooks"]["SessionStart"][1]["matcher"], "clear");
        let notifications = parsed["hooks"]["Notification"].as_array().unwrap();
        assert_eq!(notifications.len(), 2, "idle_prompt + catch-all attention");
        let cmd = notifications[1]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(cmd, NOTIFY_ATTENTION_COMMAND);
        // PreToolUse[AskUserQuestion] question binding present and in sync with the const.
        assert!(json.contains("AskUserQuestion"));
        let pretool = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pretool.len(), 1, "single AskUserQuestion entry");
        assert_eq!(
            pretool[0]["matcher"].as_str().unwrap(),
            "AskUserQuestion",
            "the entry carries the AskUserQuestion matcher"
        );
        let qcmd = pretool[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(
            qcmd, NOTIFY_QUESTION_COMMAND,
            "embedded JSON command must match the const (no drift)"
        );
    }

    #[test]
    fn test_codex_hooks_json_contains_native_tui_signals() {
        let json = codex_hooks_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["hooks"]["Stop"].is_array());
        assert!(parsed["hooks"]["PostToolUse"].is_array());
        assert!(parsed["hooks"]["UserPromptSubmit"].is_array());
        assert_eq!(parsed["hooks"]["PostToolUse"][0]["matcher"], ".*");
        assert_eq!(parsed["hooks"]["SessionStart"][0]["matcher"], "startup");
        assert_eq!(parsed["hooks"]["SessionStart"][1]["matcher"], "clear");
        assert!(json.contains("on-stop.sh"));
        assert!(json.contains("on-start.sh"));
        assert!(json.contains("on-clear.sh"));
        assert!(json.contains("on-heartbeat.sh"));
        assert!(json.contains("on-ack.sh"));
        assert!(!json.contains("idle_prompt"));
        assert!(!json.contains("AskUserQuestion"));
    }

    #[test]
    fn test_merge_codex_hooks_preserves_user_hooks_and_is_idempotent() {
        let input = r#"{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"./mine.sh"}]}],"PostToolUse":[{"matcher":".*","hooks":[{"type":"command","command":"./my-heartbeat.sh"}]}]}}"#;
        let merged = merge_codex_hooks(input).unwrap();
        assert!(merged.contains("./mine.sh"));
        assert!(merged.contains("./my-heartbeat.sh"));
        assert!(merged.contains("on-stop.sh"));
        assert!(merged.contains("on-start.sh"));
        assert!(merged.contains("on-clear.sh"));
        assert!(merged.contains("on-heartbeat.sh"));
        assert!(merged.contains("on-ack.sh"));

        let again = merge_codex_hooks(&merged).unwrap();
        assert_eq!(again.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(again.matches("test -x .lisa/hooks/on-start.sh").count(), 1);
        assert_eq!(again.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(
            again.matches("test -x .lisa/hooks/on-heartbeat.sh").count(),
            1
        );
        assert_eq!(again.matches("test -x .lisa/hooks/on-ack.sh").count(), 1);
    }

    #[test]
    fn test_lisa_gitignore_content() {
        assert!(LISA_GITIGNORE.contains("signals/"));
        assert!(LISA_GITIGNORE.contains("claude/"));
        assert!(LISA_GITIGNORE.contains("codex/"));
        assert!(LISA_GITIGNORE.contains("run-events.jsonl"));
        assert!(LISA_GITIGNORE.contains("run-baseline.json"));
        assert!(LISA_GITIGNORE.contains(lisa_core::liveness::SCHEDULER_ALIVE_NAME));
    }

    /// Lisa generates no agent context file for a project. That document states
    /// the project's own standing intentions, and it is not Lisa's to author.
    #[test]
    fn test_no_context_file_generator_survives() {
        let source = include_str!("templates.rs");
        // Built at runtime so this assertion is not its own counter-example.
        for stem in ["claude", "agents"] {
            let definition = format!("fn generate_{stem}_md");
            assert!(
                !source.contains(&definition),
                "{definition} came back — Lisa does not author a project's context file"
            );
        }
    }

    #[test]
    fn test_merge_hooks_empty_object() {
        let result = merge_hooks("{}").unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("\"Notification\""));
        assert!(result.contains("idle_prompt"));
        assert!(result.contains("on-idle.sh"));
        assert!(result.contains("\"PostToolUse\""));
        assert!(result.contains("on-heartbeat.sh"));
        assert!(result.contains("\"UserPromptSubmit\""));
        assert!(result.contains("on-ack.sh"));
        // Catch-all attention binding added too.
        assert!(result.contains("on-notify"));
        // PreToolUse[AskUserQuestion] question binding added.
        assert!(result.contains("\"PreToolUse\""));
        assert!(result.contains("AskUserQuestion"));
        assert_eq!(count_question_commands(&result), 1);
        let again = merge_hooks(&result).unwrap();
        assert_eq!(again.matches("test -x .lisa/hooks/on-ack.sh").count(), 1);
    }

    #[test]
    fn test_merge_hooks_adds_attention_to_existing_idle() {
        // Settings that already has the idle_prompt Notification hook.
        let input = r#"{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}"#;
        let result = merge_hooks(input).unwrap();
        // Both entries present: idle_prompt preserved (not duplicated) + catch-all added.
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
        assert!(result.contains("on-notify"));
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed["hooks"]["Notification"].as_array().unwrap().len(),
            2,
            "idle_prompt entry and catch-all attention entry coexist"
        );
        assert_eq!(count_attention_commands(&result), 1);
        // Idempotent: re-merging does not collapse or duplicate either entry.
        let again = merge_hooks(&result).unwrap();
        assert_eq!(again.matches("\"idle_prompt\"").count(), 1);
        assert_eq!(count_attention_commands(&again), 1);
        let reparsed: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(
            reparsed["hooks"]["Notification"].as_array().unwrap().len(),
            2
        );
    }

    /// Count how many Notification hook commands exactly equal the catch-all
    /// attention command. Parses JSON so escaped quotes don't break a substring
    /// match (the command embeds `"` characters).
    fn count_attention_commands(json: &str) -> usize {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let Some(entries) = v["hooks"]["Notification"].as_array() else {
            return 0;
        };
        entries
            .iter()
            .filter_map(|e| e["hooks"].as_array())
            .flatten()
            .filter_map(|h| h["command"].as_str())
            .filter(|c| *c == NOTIFY_ATTENTION_COMMAND)
            .count()
    }

    /// Count how many PreToolUse hook commands exactly equal the question command.
    /// Parses JSON so the escaped quotes/backslashes in the command don't break a
    /// substring match.
    fn count_question_commands(json: &str) -> usize {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let Some(entries) = v["hooks"]["PreToolUse"].as_array() else {
            return 0;
        };
        entries
            .iter()
            .filter_map(|e| e["hooks"].as_array())
            .flatten()
            .filter_map(|h| h["command"].as_str())
            .filter(|c| *c == NOTIFY_QUESTION_COMMAND)
            .count()
    }

    #[test]
    fn test_merge_hooks_adds_pretooluse_question() {
        // Start from settings that already has the five legacy bindings but no PreToolUse.
        let input = r#"{
  "hooks": {
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-heartbeat.sh && .lisa/hooks/on-heartbeat.sh" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh" }] }],
    "Notification": [{ "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": "test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh" }] }]
  }
}"#;
        let result = merge_hooks(input).unwrap();
        // The question binding is added exactly once, with the right matcher.
        assert_eq!(count_question_commands(&result), 1);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let pretool = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pretool.len(), 1);
        assert_eq!(pretool[0]["matcher"].as_str().unwrap(), "AskUserQuestion");
        // The pre-existing five bindings survive untouched.
        assert!(result.contains("on-heartbeat.sh"));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("idle_prompt"));
        assert_eq!(count_attention_commands(&result), 1);
        // Idempotent: re-merging neither duplicates nor drops the question entry.
        let again = merge_hooks(&result).unwrap();
        assert_eq!(count_question_commands(&again), 1);
        let reparsed: serde_json::Value = serde_json::from_str(&again).unwrap();
        assert_eq!(reparsed["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }

    /// Replicate the hook's POSIX `sed` extraction so the contract is tested end to
    /// end against a real payload, not a reimplementation. `sed` is unix-only.
    #[cfg(unix)]
    fn extract_question_via_sed(payload: &str) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("sed")
            .args(["-n", r#"s/.*"question":[ ]*"\([^"]*\)".*/\1/p"#])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn sed");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    #[cfg(unix)]
    #[test]
    fn test_notify_question_command_extracts_question() {
        // The const must literally embed the documented sed expression.
        assert!(
            NOTIFY_QUESTION_COMMAND.contains(r#"sed -n 's/.*"question":[ ]*"\([^"]*\)".*/\1/p'"#)
        );
        // It writes the awaiting signal into the leased project — not the
        // directory the agent walked to — and only test-x-gates the notify.
        assert!(NOTIFY_QUESTION_COMMAND
            .contains(r#""$LISA_PROJECT/.lisa/signals/pane-$LISA_PANE_ID.awaiting""#));
        assert!(NOTIFY_QUESTION_COMMAND.contains("LISA_REASON=question"));
        assert!(NOTIFY_QUESTION_COMMAND.contains(r#"test -x "$proj/.lisa/hooks/on-notify""#));

        // (i) Happy path: the real captured single-line payload shape.
        let payload = r#"{"tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Which approach should I use to build the feature?","header":"Approach","options":[]}]}}"#;
        assert_eq!(
            extract_question_via_sed(payload),
            "Which approach should I use to build the feature?"
        );

        // (ii) Escaped-quote variant degrades gracefully (truncates, never panics).
        let escaped = r#"{"questions":[{"question":"He said \"hi\" to me","header":"X"}]}"#;
        let got = extract_question_via_sed(escaped);
        // The greedy-free [^"]* stops at the embedded quote; result is a (possibly
        // empty/partial) string, and the hook's `[ -z "$q" ]` fallback covers empties.
        assert!(
            !got.contains("to me"),
            "extraction stops at the escaped quote"
        );

        // (iii) No question key at all -> empty extraction -> hook falls back to generic.
        let none = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert_eq!(extract_question_via_sed(none), "");
    }

    #[cfg(unix)]
    #[test]
    fn interaction_hooks_retain_payload_free_gate_facts() {
        use std::io::Write;
        use std::process::Stdio;

        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".lisa")).unwrap();
        // Both commands run from a repository the agent stepped into, and must
        // land their facts in the leased project instead.
        let elsewhere = tempfile::tempdir().unwrap();
        let question_payload = r#"{"tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"secret question text","header":"Choice"}]}}"#;
        let mut question = Command::new("/bin/sh")
            .args(["-c", NOTIFY_QUESTION_COMMAND])
            .current_dir(elsewhere.path())
            .env("LISA_PROJECT", root.path())
            .env("LISA_PANE_ID", "7")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        question
            .stdin
            .take()
            .unwrap()
            .write_all(question_payload.as_bytes())
            .unwrap();
        assert!(question.wait().unwrap().success());

        let mut permission = Command::new("/bin/sh")
            .args(["-c", NOTIFY_ATTENTION_COMMAND])
            .current_dir(elsewhere.path())
            .env("LISA_PROJECT", root.path())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        permission
            .stdin
            .take()
            .unwrap()
            .write_all(b"permission payload is also secret")
            .unwrap();
        assert!(permission.wait().unwrap().success());

        let events = fs::read_to_string(root.path().join(".lisa/run-events.jsonl")).unwrap();
        assert_eq!(events.lines().count(), 2);
        assert!(events.contains(r#"{"event":"manual-intervention","kind":"question"}"#));
        assert!(events.contains(r#"{"event":"manual-intervention","kind":"permission"}"#));
        assert!(!events.contains("secret"));
        assert!(root.path().join(".lisa/signals/pane-7.awaiting").exists());
        assert!(
            !elsewhere.path().join(".lisa").exists(),
            "neither command may leave anything in the repository it ran in"
        );
    }

    /// An operator's own session has no pane and no leased project, and the two
    /// interactive commands keep meaning what they always meant there: `$PWD`,
    /// the project the operator is actually working in. What is gone is the
    /// unconditional `mkdir` — nothing is created outside an existing `.lisa/`.
    #[cfg(unix)]
    #[test]
    fn an_operator_session_records_in_its_own_project_and_nowhere_else() {
        use std::io::Write;
        use std::process::Stdio;

        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join(".lisa")).unwrap();
        let mut permission = Command::new("/bin/sh")
            .args(["-c", NOTIFY_ATTENTION_COMMAND])
            .current_dir(project.path())
            .env_remove("LISA_PROJECT")
            .env_remove("LISA_PANE_ID")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        permission
            .stdin
            .take()
            .unwrap()
            .write_all(b"a permission prompt")
            .unwrap();
        assert!(permission.wait().unwrap().success());
        let events = fs::read_to_string(project.path().join(".lisa/run-events.jsonl")).unwrap();
        assert!(events.contains(r#""kind":"permission""#));

        // The same session standing in a directory that is not a Lisa project
        // writes nothing at all, and creates nothing to write into.
        let stranger = tempfile::tempdir().unwrap();
        let mut outside = Command::new("/bin/sh")
            .args(["-c", NOTIFY_ATTENTION_COMMAND])
            .current_dir(stranger.path())
            .env_remove("LISA_PROJECT")
            .env_remove("LISA_PANE_ID")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        outside
            .stdin
            .take()
            .unwrap()
            .write_all(b"a permission prompt")
            .unwrap();
        assert!(outside.wait().unwrap().success());
        assert!(!stranger.path().join(".lisa").exists());
    }

    #[cfg(unix)]
    #[test]
    fn idle_notification_does_not_record_a_permission_gate() {
        use std::io::Write;
        use std::process::Stdio;

        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sh")
            .args(["-c", NOTIFY_ATTENTION_COMMAND])
            .current_dir(root.path())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        command
            .stdin
            .take()
            .unwrap()
            .write_all(b"idle_prompt")
            .unwrap();
        assert!(command.wait().unwrap().success());
        assert!(!root.path().join(".lisa/run-events.jsonl").exists());
    }

    #[test]
    fn test_merge_hooks_with_existing_idle_only() {
        // Start with only idle_prompt — should add Stop + SessionStart
        let input = r#"{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("\"Stop\""));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("\"SessionStart\""));
        assert!(result.contains("on-clear.sh"));
        // idle_prompt should not be duplicated
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
    }

    #[test]
    fn test_merge_hooks_already_complete() {
        let input = settings_local_json();
        let result = merge_hooks(&input).unwrap();
        // No duplicate hook entries (each command string contains the script name twice
        // due to the test -x guard, so count the full command instead)
        assert_eq!(result.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(result.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
        // The catch-all attention command appears exactly once.
        assert_eq!(count_attention_commands(&result), 1);
    }

    #[test]
    fn test_merge_hooks_upgrades_bare_path_commands() {
        // Old-style settings with bare-path hook commands
        let input = r#"{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": ".lisa/hooks/on-stop.sh" }] }],
    "SessionStart": [{ "matcher": "clear", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-clear.sh" }] }],
    "Notification": [{ "matcher": "idle_prompt", "hooks": [{ "type": "command", "command": ".lisa/hooks/on-idle.sh" }] }]
  }
}"#;
        let result = merge_hooks(input).unwrap();
        // Should upgrade to guarded commands
        assert!(result.contains("test -x .lisa/hooks/on-stop.sh && .lisa/hooks/on-stop.sh"));
        assert!(result.contains("test -x .lisa/hooks/on-clear.sh && .lisa/hooks/on-clear.sh"));
        assert!(result.contains("test -x .lisa/hooks/on-idle.sh && .lisa/hooks/on-idle.sh"));
        // No duplicates — each hook entry appears once
        assert_eq!(result.matches("test -x .lisa/hooks/on-stop.sh").count(), 1);
        assert_eq!(result.matches("test -x .lisa/hooks/on-clear.sh").count(), 1);
        assert_eq!(result.matches("\"idle_prompt\"").count(), 1);
    }

    #[test]
    fn test_merge_hooks_preserves_permissions() {
        let input = r#"{"permissions":{"allow":["Bash(cargo test:*)"]}}"#;
        let result = merge_hooks(input).unwrap();
        assert!(result.contains("cargo test"));
        assert!(result.contains("on-stop.sh"));
        assert!(result.contains("on-clear.sh"));
        assert!(result.contains("idle_prompt"));
    }

    #[test]
    fn test_merge_hooks_invalid_json() {
        let result = merge_hooks("not json");
        assert!(result.is_err());
    }

    #[test]
    fn stop_hook_writes_stopped_and_keeps_capture_outcomes_visible() {
        // T-027-02: the Stop hook keeps writing the `.stopped` signal and now
        // forwards its stdin payload to `lisa capture-usage`.
        assert!(ON_STOP_HOOK.contains("pane-$LISA_PANE_ID.stopped"));
        assert!(ON_STOP_HOOK.contains("capture-usage"));
        // Outside a Lisa-managed pane the hook exits silently before the
        // capturer runs — the operator's own sessions must never see an error.
        assert!(
            ON_STOP_HOOK.contains(r#"if [ -z "${LISA_PANE_ID:-}" ] || [ -z "${LISA_PROJECT:-}" ]"#)
        );
        assert!(
            ON_STOP_HOOK.find(r#"[ -z "${LISA_PANE_ID:-}" ]"#).unwrap()
                < ON_STOP_HOOK.find("capture-usage").unwrap(),
            "the no-pane guard must precede the capture-usage forward"
        );
        // The capture ledger belongs beside the signals, in the leased project.
        assert!(ON_STOP_HOOK.contains(r#"capture-usage --cwd "$LISA_PROJECT""#));
        // Capture before signal: the stop signal is the scheduler's cue to act
        // on the pane (advance the ticket, end the session), so the capture
        // must be durable before the signal appears — a session ended
        // mid-capture lost 8 of 9 usage records in the 0.4.4-rc.8 field leg.
        assert!(
            ON_STOP_HOOK.find("capture-usage").unwrap()
                < ON_STOP_HOOK.find("pane-$LISA_PANE_ID.stopped").unwrap(),
            "the capture must complete before the stopped signal is written"
        );
        assert!(ON_STOP_HOOK.contains("${LISA_BIN:-lisa}"));
        // Reads stdin once (the Stop payload carries transcript_path).
        assert!(ON_STOP_HOOK.contains("in=$(cat)"));
        assert!(!ON_STOP_HOOK.contains("2>/dev/null"));
        assert!(!ON_STOP_HOOK.contains("|| true"));

        let live_hook = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.lisa/hooks/on-stop.sh"),
        )
        .unwrap();
        assert_eq!(live_hook, ON_STOP_HOOK);
    }

    #[test]
    fn heartbeat_hook_stays_trivial() {
        // The ticket's constraint: PostToolUse capture must not grow. The
        // heartbeat hook must not read stdin or invoke lisa.
        assert!(!ON_HEARTBEAT_HOOK.contains("capture-usage"));
        assert!(!ON_HEARTBEAT_HOOK.contains("$(cat)"));
        assert!(ON_HEARTBEAT_HOOK.contains("pane-$LISA_PANE_ID.heartbeat"));
        assert!(ON_HEARTBEAT_HOOK.contains("pane-$LISA_PANE_ID.lease"));
    }
}
