#!/bin/sh
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
