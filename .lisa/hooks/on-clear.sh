#!/bin/sh
# Lisa clear signal hook — called after /clear is processed.
# Writes a signal file so the plugin knows context has been cleared.

# Signals belong to the project this pane was leased from, not to whatever
# directory the agent is standing in. Both facts ride the pane's launch line;
# without them there is no lease to write to, so write nothing rather than
# leave a plausible signal in a repository Lisa does not manage.
[ -n "${LISA_PANE_ID:-}" ] || exit 0
[ -n "${LISA_PROJECT:-}" ] || exit 0
[ -d "$LISA_PROJECT/.lisa" ] || exit 0
SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"
mkdir -p "$SIGNAL_DIR" || exit 0

echo "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SIGNAL_DIR/pane-$LISA_PANE_ID.cleared"
