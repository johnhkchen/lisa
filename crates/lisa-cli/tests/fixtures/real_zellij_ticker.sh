#!/usr/bin/env bash
# What the plugin pane's title actually says, read out of real Zellij
# (T-062-01-03). Run via the ignored Cargo integration test; LISA_BIN must name
# the freshly built CLI whose embedded WASM is under test.
#
# Nothing here is inferred from lisa's own state: the title is read back from
# `zellij action list-panes --json --all`, which is the same string the operator
# sees on the title row. The three moments the ticket asks for are driven in
# order on one running loop -- idle, one seat working, and just after a ticket
# finished -- and the row is read at each.
set -euo pipefail

: "${LISA_BIN:?set LISA_BIN to the freshly built lisa executable}"

for dependency in bash git jq python3 zellij zsh; do
    command -v "$dependency" >/dev/null 2>&1 || {
        echo "missing required command: $dependency" >&2
        exit 1
    }
done
[[ -x "$LISA_BIN" ]] || {
    echo "LISA_BIN is not executable: $LISA_BIN" >&2
    exit 1
}

LISA_BIN=$(cd "$(dirname "$LISA_BIN")" && pwd -P)/$(basename "$LISA_BIN")
REAL_ZELLIJ=$(command -v zellij)
TEST_PARENT=${LISA_ZELLIJ_TEST_ROOT:-${TMPDIR:-/tmp}}
RUN_ROOT=$(mktemp -d "$TEST_PARENT/lisa-ticker.XXXXXX")
KEEP_FIXTURES=${KEEP_LISA_ZELLIJ_FIXTURES:-0}
SESSION="lisa-ticker-$$"
# The project directory's name, which is what the row must lead with.
PROJECT=steer
ROOT="$RUN_ROOT/$PROJECT"
LOOP_PID=

cleanup() {
    local status=$?
    "$REAL_ZELLIJ" kill-session "$SESSION" >/dev/null 2>&1 || true
    if [[ -n "$LOOP_PID" ]]; then
        kill "$LOOP_PID" >/dev/null 2>&1 || true
        wait "$LOOP_PID" >/dev/null 2>&1 || true
    fi
    if [[ "$KEEP_FIXTURES" == 1 && $status -ne 0 ]]; then
        echo "retained failed fixtures at $RUN_ROOT" >&2
    else
        rm -rf "$RUN_ROOT"
    fi
    exit "$status"
}
trap cleanup EXIT INT TERM

fail() {
    echo "FAIL: $1" >&2
    echo "--- pane titles ---" >&2
    panes_json 2>/dev/null | jq -r '.[] | "\(.id) plugin=\(.is_plugin) \(.title)"' >&2 || true
    echo "--- loop log tail ---" >&2
    tail -n 40 "$ROOT/evidence/loop.log" >&2 2>/dev/null || true
    return 1
}

panes_json() {
    "$REAL_ZELLIJ" --session "$SESSION" action list-panes --json --all 2>/dev/null
}

session_is_ready() {
    panes_json >/dev/null 2>&1
}

# The dashboard pane's title, exactly as Zellij holds it. Selected by its `file:`
# location, because the tab also carries Zellij's own bundled plugins.
ticker_title() {
    panes_json | jq -r '
        [.[] | select(.is_plugin == true and ((.plugin_url // "") | startswith("file:")))][0]
        | .title // empty' 2>/dev/null
}

plugin_pane_exists() {
    [[ -n "$(ticker_title)" ]]
}

title_matches() {
    [[ "$(ticker_title)" =~ $1 ]]
}

wait_until() {
    local timeout=$1 description=$2
    shift 2
    local deadline=$(( $(date +%s) + timeout ))
    while (( $(date +%s) <= deadline )); do
        if "$@"; then
            return 0
        fi
        sleep 0.25
    done
    fail "timed out after ${timeout}s waiting for $description"
}

# Every observation is also a check that the build detail is gone for good.
observe() {
    local moment=$1 pattern=$2
    wait_until 40 "the $moment row to read /$pattern/" title_matches "$pattern"
    local title
    title=$(ticker_title)
    case "$title" in
        *.wasm*|file:*|*/var/folders/*)
            fail "the $moment row still carries a build detail: $title"
            ;;
    esac
    printf 'ticker[%s]: %s\n' "$moment" "$title"
}

# A ticket file, written whole so a rewrite is one atomic-enough replacement.
write_ticket() {
    local id=$1 status=$2 phase=$3
    cat > "$ROOT/docs/active/tickets/$id.md" <<TICKET
---
id: $id
story: S-TICK
title: ticker-fixture-seat
type: task
status: $status
priority: high
agent: claude
phase: $phase
depends_on: []
---

## Context

Deterministic local-provider fixture. The provider must not execute this ticket.

## Acceptance Criteria

- [ ] The harness reads the plugin pane's title out of Zellij.
TICKET
}

# A provider that starts, acknowledges its assignment, and then does nothing.
# The seat only has to *be* occupied for the row to say so.
write_stub_provider() {
    cat > "$ROOT/bin/claude" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
    echo "claude stub 1.0.0"
    exit 0
fi

: "${LISA_PANE_ID:?}"
signals=.lisa/signals
mkdir -p "$signals"

publish() {
    # Exactly what the native hooks do: copy the scheduler-owned lease marker
    # into the signal, atomically.
    local destination="$signals/pane-$LISA_PANE_ID.$1"
    local tmp="$destination.tmp.$$"
    cp "$signals/pane-$LISA_PANE_ID.lease" "$tmp"
    mv "$tmp" "$destination"
}

publish started

prompt=
while IFS= read -r line; do
    case "$line" in
        "Read and follow the complete assignment at "*)
            prompt=$line
            ;;
        "LISA_ASSIGNMENT "*)
            if [[ -n "$prompt" ]]; then
                prompt=$(printf '%s\n%s' "$prompt" "$line")
                destination=$signals/pane-$LISA_PANE_ID.ack
                tmp=$destination.tmp.$$
                jq -cn --arg prompt "$prompt" \
                    '{hook_event_name:"UserPromptSubmit",prompt:$prompt}' > "$tmp"
                mv "$tmp" "$destination"
                prompt=
            fi
            ;;
    esac
done
STUB
    chmod +x "$ROOT/bin/claude"
}

# Keep one addressable session name whatever the CLI picks for itself.
write_zellij_wrapper() {
    cat > "$ROOT/bin/zellij" <<'WRAPPER'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" || "${1:-}" == "list-sessions" ]]; then
    exec "$LISA_STUB_REAL_ZELLIJ" "$@"
fi
if [[ "${1:-}" == "--session" && "${3:-}" == "--new-session-with-layout" && $# == 4 ]]; then
    exec "$LISA_STUB_REAL_ZELLIJ" --session "$LISA_STUB_SESSION" \
        --new-session-with-layout "$4"
fi
echo "unexpected zellij invocation: $*" >&2
exit 2
WRAPPER
    chmod +x "$ROOT/bin/zellij"
}

write_pty_runner() {
    cat > "$ROOT/pty_run.py" <<'PTY'
"""Run a command on a real pty, because zellij refuses to start without one."""
import os, pty, sys

log = open(sys.argv[1], "wb", buffering=0)
cmd = sys.argv[sys.argv.index("--") + 1:]

pid, fd = os.forkpty()
if pid == 0:
    os.environ["TERM"] = "xterm-256color"
    os.environ["COLUMNS"] = "140"
    os.environ["LINES"] = "50"
    os.execvp(cmd[0], cmd)

while True:
    try:
        data = os.read(fd, 65536)
    except OSError:
        break
    if not data:
        break
    log.write(data)
PTY
}

create_fixture() {
    mkdir -p "$ROOT/bin" "$ROOT/evidence" "$ROOT/home"
    printf '%s\n' '{"bypassPermissionsModeAccepted": true}' > "$ROOT/home/.claude.json"
    "$LISA_BIN" init --path "$ROOT" --no-history >/dev/null
    mkdir -p "$ROOT/docs/active/tickets" "$ROOT/docs/active/stories"

    local project_version
    project_version=$("$LISA_BIN" --version | awk '{ print $2 }')
    [[ -n "$project_version" ]] || fail "could not read the version of $LISA_BIN"
    cat > "$ROOT/.lisa.toml" <<TOML
version = "$project_version"

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
review_timeout_secs = 600
session_timeout_secs = 0
wind_down_secs = 1
assignment_ack_timeout_secs = 60

[agent]
client = "claude"
TOML
    cat > "$ROOT/docs/active/stories/S-TICK.md" <<'STORY'
---
id: S-TICK
title: ticker
status: active
---

# Ticker
STORY

    # One finished, one held back for the working moment, one held back for
    # good so the board never drains and the loop never terminates.
    write_ticket T-TICK-01 done done
    write_ticket T-TICK-02 blocked implement
    write_ticket T-TICK-03 blocked implement

    write_stub_provider
    write_zellij_wrapper
    write_pty_runner
    (
        cd "$ROOT"
        git init -q
        git config user.name "Lisa Ticker Test"
        git config user.email "lisa-ticker-test@example.invalid"
        git add .
        git commit -qm "fixture baseline"
    )
}

start_loop() {
    local runner="$ROOT/run-loop.sh"
    cat > "$runner" <<RUNNER
#!/usr/bin/env bash
exec env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME \\
  HOME=$(printf '%q' "$ROOT/home") \\
  ZDOTDIR=$(printf '%q' "$ROOT/home") \\
  PATH=$(printf '%q' "$ROOT/bin:$PATH") \\
  LISA_STUB_REAL_ZELLIJ=$(printf '%q' "$REAL_ZELLIJ") \\
  LISA_STUB_SESSION=$(printf '%q' "$SESSION") \\
  $(printf '%q' "$LISA_BIN") loop --path $(printf '%q' "$ROOT") --client claude
RUNNER
    chmod +x "$runner"
    (
        cd "$ROOT"
        python3 pty_run.py evidence/loop.log -- "$runner"
    ) >/dev/null 2>&1 &
    LOOP_PID=$!
    wait_until 30 "the Zellij session" session_is_ready
    wait_until 20 "the dashboard pane" plugin_pane_exists
}

create_fixture
start_loop

# The layout's own floor, before the plugin has said anything: a pane that
# would otherwise be titled with the wasm path it was loaded from.
grep -Fq 'name="lisa · starting"' "$ROOT/.lisa-layout.kdl" \
    || fail "the generated layout no longer names the dashboard pane"

# 1. Idle. Two of three tickets are held back and one is done, so there is
#    nothing to schedule -- and the row says which nothing this is.
observe idle '^steer · idle · 1/3 done$'

# 2. One seat working. Releasing T-TICK-02 gives it a seat and a provider.
write_ticket T-TICK-02 open implement
observe working '^steer · 1/4 working · T-TICK-02 [0-9<][0-9hm<]*m?$'

# 3. Just after a ticket finished. The transition to Done is what the row
#    reports, whoever wrote it -- here, the same frontmatter edit an operator
#    makes by hand.
write_ticket T-TICK-02 done done
observe finished '^steer · .*done T-TICK-02'

# ...and it gives the place back once the news is stale. DONE_WINDOW_SECS is 90.
sleep 95
observe settled '^steer · idle · 2/3 done$'

echo "real-zellij-ticker: PASS"
