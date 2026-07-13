#!/bin/sh
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
