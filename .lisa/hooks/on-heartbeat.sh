#!/bin/sh
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
