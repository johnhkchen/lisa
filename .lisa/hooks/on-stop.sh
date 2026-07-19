#!/bin/sh
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
