#!/bin/sh
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
