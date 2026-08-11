#!/bin/sh
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
