#!/bin/sh
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
