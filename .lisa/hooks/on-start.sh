#!/bin/sh
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
