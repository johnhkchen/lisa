#!/bin/sh
# Lisa heartbeat signal hook — called after each tool call.
# Writes a signal file so the plugin knows this session is actively working.

SIGNAL_DIR=".lisa/signals"
mkdir -p "$SIGNAL_DIR"

if [ -n "$LISA_PANE_ID" ]; then
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.heartbeat"
fi
