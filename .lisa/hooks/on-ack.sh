#!/bin/sh
# Lisa assignment acknowledgment hook — called before a provider submits a user prompt.
# Writes the raw lifecycle payload for ticket/generation matching in the plugin.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so drain the payload and write
# nothing rather than leave a plausible signal in a repository Lisa does not
# manage.
if [ -z "${LISA_PANE_ID:-}" ] || [ -z "${LISA_PROJECT:-}" ] || [ ! -d "$LISA_PROJECT/.lisa" ]; then
    cat >/dev/null
    exit 0
fi
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

tmp="$SIGNAL_DIR/pane-$LISA_PANE_ID.ack.tmp.$$"
if cat > "$tmp"; then
    mv "$tmp" "$SIGNAL_DIR/pane-$LISA_PANE_ID.ack"
else
    rm -f "$tmp"
fi
