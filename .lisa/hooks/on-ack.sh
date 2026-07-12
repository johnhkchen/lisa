#!/bin/sh
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
