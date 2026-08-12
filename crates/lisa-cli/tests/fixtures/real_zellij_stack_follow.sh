#!/usr/bin/env bash
# Deterministic, model-free regression for which member of the agent stack is
# expanded (T-062-01-01). Run via the ignored Cargo integration test; LISA_BIN
# must name the freshly built CLI whose embedded WASM is under test.
#
# Everything here is observed through real Zellij: the stack's shape is read
# from `list-panes --json` (a collapsed stack member is one row of title bar,
# the expanded one holds the rest), focus is read from the same manifest, and
# the terminal is resized for real — the loop client runs on a pty this harness
# owns, so a resize is a genuine TIOCSWINSZ plus SIGWINCH rather than a
# simulated relayout.
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
RUN_ROOT=$(mktemp -d "$TEST_PARENT/lisa-stack-follow.XXXXXX")
KEEP_FIXTURES=${KEEP_LISA_ZELLIJ_FIXTURES:-0}
CURRENT_ROOT=
CURRENT_SESSION=
CURRENT_PLUGIN_PANE=
LOOP_PID=

cleanup() {
    local status=$?
    if [[ -n "$CURRENT_SESSION" ]]; then
        "$REAL_ZELLIJ" kill-session "$CURRENT_SESSION" >/dev/null 2>&1 || true
    fi
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
    local message=$1
    echo "FAIL: $message" >&2
    if [[ -n "$CURRENT_ROOT" && -d "$CURRENT_ROOT" ]]; then
        echo "--- panes ---" >&2
        panes_json >&2 2>/dev/null || true
        echo "--- events ---" >&2
        sed -n '1,120p' "$CURRENT_ROOT/evidence/events.log" >&2 2>/dev/null || true
        echo "--- dashboard ---" >&2
        session_action dump-screen --pane-id "$CURRENT_PLUGIN_PANE" >&2 2>/dev/null || true
    fi
    return 1
}

session_action() {
    "$REAL_ZELLIJ" --session "$CURRENT_SESSION" action "$@"
}

wait_until() {
    local timeout=$1
    local description=$2
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

panes_json() {
    session_action list-panes --json --all
}

session_is_ready() {
    panes_json >/dev/null 2>&1
}

# The lisa dashboard pane, as `plugin_<id>`.
find_plugin_pane() {
    local panes
    panes=$(panes_json) || return 1
    CURRENT_PLUGIN_PANE=$(jq -r \
        '[.[] | select(.is_plugin == true and ((.plugin_url // "") | startswith("file:")))][0]
         | if . then "plugin_\(.id)" else empty end' <<<"$panes")
    [[ -n "$CURRENT_PLUGIN_PANE" ]]
}

# The four coding panes exist as soon as the layout is applied.
agent_pane_count_is() {
    local expected=$1
    local actual
    actual=$(panes_json | jq '[.[] | select(.is_plugin == false)] | length') || return 1
    [[ "$actual" == "$expected" ]]
}

# The expanded stack member, as a bare terminal id. A collapsed member is one
# row; exactly one member is taller than that.
expanded_pane() {
    panes_json | jq -r '[.[] | select(.is_plugin == false and .pane_rows > 1)]
        | if length == 1 then .[0].id else "none" end'
}

# Whatever holds focus, as `plugin_<id>` or `terminal_<id>`.
focused_pane() {
    panes_json | jq -r '[.[] | select(.is_focused == true)][0]
        | if . == null then "none"
          elif .is_plugin then "plugin_\(.id)" else "terminal_\(.id)" end'
}

# The zellij terminal id of the pane working on a ticket, read off the
# scheduler-owned pane name.
pane_for_ticket() {
    local ticket=$1
    panes_json | jq -r --arg ticket "$ticket" \
        '[.[] | select(.is_plugin == false and ((.title // "") | contains($ticket)))][0]
         | if . then .id else "none" end'
}

# A coding pane that has never held a ticket.
idle_pane() {
    panes_json | jq -r '[.[] | select(.is_plugin == false and ((.title // "") | contains("idle")))][0]
        | if . then .id else "none" end'
}

# The last such pane, the one an operator walking down to the dashboard leaves
# expanded behind them.
last_idle_pane() {
    panes_json | jq -r '[.[] | select(.is_plugin == false and ((.title // "") | contains("idle")))]
        | sort_by(.pane_y) | last | if . then .id else "none" end'
}

expanded_is() {
    [[ "$(expanded_pane)" == "$1" ]]
}

focused_is() {
    [[ "$(focused_pane)" == "$1" ]]
}

keys_delivered() {
    [[ ! -s "$CURRENT_ROOT/evidence/keys.in" ]]
}

press_key() {
    printf '%s' "$1" > "$CURRENT_ROOT/evidence/keys.in"
    wait_until 10 "the keystroke to reach the terminal" keys_delivered
    sleep 0.4
}

# Walk the operator's focus one pane at a time until it lands on $1, the way
# they really do it: Alt+j / Alt+k, on the client that is actually attached.
focus_walk_to() {
    local target=$1
    local direction=${2:-down}
    local key=$'\033j'
    [[ "$direction" == up ]] && key=$'\033k'
    local step
    for step in 1 2 3 4 5 6 7 8; do
        focused_is "$target" && return 0
        press_key "$key"
    done
    focused_is "$target" \
        || fail "walking $direction never reached $target (stopped at $(focused_pane))"
}

# Lisa's slot number for a ticket, as its provider announced it at launch.
lisa_pane_for_ticket() {
    local ticket=$1
    awk -F '\t' -v ticket="ticket=$ticket" \
        '$1 == "launch" { for (i = 2; i <= NF; i++) if ($i == ticket) hit = 1 }
         $1 == "launch" && hit { for (i = 2; i <= NF; i++) if ($i ~ /^pane=/) { sub(/^pane=/, "", $i); print $i; exit } }' \
        "$CURRENT_ROOT/evidence/events.log"
}

event_count() {
    local kind=$1
    local events="$CURRENT_ROOT/evidence/events.log"
    [[ -f "$events" ]] || { printf '0\n'; return; }
    awk -F '\t' -v kind="$kind" '$1 == kind { count++ } END { print count + 0 }' "$events"
}

event_count_at_least() {
    (( $(event_count "$1") >= $2 ))
}

# Ask the provider stub in lisa slot $1 for one tool call, and wait until the
# plugin has consumed the heartbeat it publishes.
beat() {
    local lisa_pane=$1
    local before
    before=$(event_count beat)
    touch "$CURRENT_ROOT/evidence/beat-$lisa_pane"
    wait_until 20 "heartbeat from lisa pane $lisa_pane" event_count_at_least beat $(( before + 1 ))
    wait_until 20 "heartbeat consumption by the plugin" \
        test ! -e "$CURRENT_ROOT/.lisa/signals/pane-$lisa_pane.heartbeat"
}

# Nothing may move for this long. Long enough for several plugin polls.
settle() {
    sleep 6
}

write_stub_provider() {
    local root=$1
    cat > "$root/bin/claude" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
    echo "claude stub 1.0.0"
    exit 0
fi

: "${LISA_PANE_ID:?}"
: "${LISA_TICKET_ID:?}"
: "${LISA_ATTEMPT_ID:?}"
: "${LISA_STUB_EVIDENCE_DIR:?}"

signals=.lisa/signals
events=$LISA_STUB_EVIDENCE_DIR/events.log
mkdir -p "$signals" "$LISA_STUB_EVIDENCE_DIR"
printf 'launch\tpane=%s\tticket=%s\tgeneration=%s\n' \
    "$LISA_PANE_ID" "$LISA_TICKET_ID" "$LISA_ATTEMPT_ID" >> "$events"

publish() {
    # Exactly what the native hooks do: copy the scheduler-owned lease marker
    # into the signal, atomically.
    local suffix=$1
    local marker="$signals/pane-$LISA_PANE_ID.lease"
    local destination="$signals/pane-$LISA_PANE_ID.$suffix"
    local tmp="$destination.tmp.$$"
    cp "$marker" "$tmp"
    mv "$tmp" "$destination"
}

publish started
printf 'start\tpane=%s\tticket=%s\n' "$LISA_PANE_ID" "$LISA_TICKET_ID" >> "$events"

# One tool call per request from the harness, so "most recently active" is
# something the test states rather than races for.
(
    gate=$LISA_STUB_EVIDENCE_DIR/beat-$LISA_PANE_ID
    while :; do
        if [[ -f "$gate" ]]; then
            rm -f "$gate"
            publish heartbeat
            printf 'beat\tpane=%s\tticket=%s\n' "$LISA_PANE_ID" "$LISA_TICKET_ID" >> "$events"
        fi
        sleep 0.2
    done
) &

prompt=
while IFS= read -r line; do
    case "$line" in
        "Read and follow the complete assignment at "*)
            prompt=$line
            ;;
        "LISA_ASSIGNMENT "*)
            if [[ -n "$prompt" ]]; then
                prompt=$(printf '%s\n%s' "$prompt" "$line")
            fi
            destination=$signals/pane-$LISA_PANE_ID.ack
            tmp=$destination.tmp.$$
            jq -cn --arg prompt "$prompt" \
                '{hook_event_name:"UserPromptSubmit",prompt:$prompt}' > "$tmp"
            mv "$tmp" "$destination"
            printf 'ack\tpane=%s\tticket=%s\n' "$LISA_PANE_ID" "$LISA_TICKET_ID" >> "$events"
            prompt=
            ;;
    esac
done
STUB
    chmod +x "$root/bin/claude"
}

write_zellij_wrapper() {
    local root=$1
    cat > "$root/bin/zellij" <<'WRAPPER'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" || "${1:-}" == "list-sessions" ]]; then
    exec "$LISA_STUB_REAL_ZELLIJ" "$@"
fi
if [[ "${1:-}" == "--session" && "${3:-}" == "--new-session-with-layout" && $# == 4 ]]; then
    # Start under the harness's own name so one session stays addressable.
    exec "$LISA_STUB_REAL_ZELLIJ" --session "$LISA_STUB_SESSION" \
        --new-session-with-layout "$4"
fi
echo "unexpected zellij invocation: $*" >&2
exit 2
WRAPPER
    chmod +x "$root/bin/zellij"
}

# A pty this harness owns, so the terminal can be resized for real.
write_pty_runner() {
    local root=$1
    cat > "$root/pty_run.py" <<'PTY'
"""Run a command on a real pty this harness can resize and type into.

usage: python3 pty_run.py <size-file> <keys-file> <out-log> -- cmd args...

The size file holds "ROWS COLS". It is polled, and on change the pty is resized
with TIOCSWINSZ and the child is sent SIGWINCH -- exactly what a terminal
emulator does when its window is dragged.

The keys file is drained into the pty as raw bytes, so the harness moves focus
the way the operator does: with keystrokes zellij binds, on the one client that
is really attached. (`zellij action focus-pane-id` cannot be used for this: the
CLI attaches as a *second* client, so it moves that client's focus and leaves
the operator's where it was.)
"""
import fcntl, os, signal, struct, sys, termios, threading, time

size_file, keys_file, out_log = sys.argv[1], sys.argv[2], sys.argv[3]
cmd = sys.argv[sys.argv.index("--") + 1:]


def read_size():
    try:
        rows, cols = open(size_file).read().split()
        return int(rows), int(cols)
    except Exception:
        return 50, 140


rows, cols = read_size()
pid, fd = os.forkpty()
if pid == 0:
    os.environ["TERM"] = "xterm-256color"
    os.execvp(cmd[0], cmd)

fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def watch_size():
    global rows, cols
    while True:
        time.sleep(0.3)
        wanted = read_size()
        if wanted != (rows, cols):
            rows, cols = wanted
            fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            os.kill(pid, signal.SIGWINCH)


def watch_keys():
    while True:
        time.sleep(0.15)
        try:
            if os.path.getsize(keys_file) > 0:
                with open(keys_file, "rb") as handle:
                    data = handle.read()
                open(keys_file, "wb").close()
                os.write(fd, data)
        except Exception:
            pass


threading.Thread(target=watch_size, daemon=True).start()
threading.Thread(target=watch_keys, daemon=True).start()

with open(out_log, "wb", buffering=0) as log:
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

# $1 scenario name, $2... ticket specs as "id:status"
create_fixture() {
    local scenario=$1
    shift
    local root="$RUN_ROOT/$scenario"
    mkdir -p "$root/bin" "$root/evidence" "$root/home"
    printf '%s\n' '{"bypassPermissionsModeAccepted": true}' > "$root/home/.claude.json"
    "$LISA_BIN" init --path "$root" --no-history >/dev/null
    mkdir -p "$root/docs/active/tickets" "$root/docs/active/stories"
    local project_version
    project_version=$("$LISA_BIN" --version | awk '{ print $2 }')
    [[ -n "$project_version" ]] || fail "could not read the version of $LISA_BIN"
    cat > "$root/.lisa.toml" <<TOML
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
    cat > "$root/docs/active/stories/S-STUB.md" <<'STORY'
---
id: S-STUB
title: stack-follow
status: active
---

# Stack follow
STORY
    local spec
    for spec in "$@"; do
        local ticket=${spec%%:*}
        local status=${spec##*:}
        cat > "$root/docs/active/tickets/$ticket.md" <<TICKET
---
id: $ticket
story: S-STUB
title: stack-follow-seat
type: task
status: $status
priority: high
agent: claude
phase: implement
depends_on: []
---

## Context

Deterministic local-provider fixture. The provider must not execute this ticket.

## Acceptance Criteria

- [ ] The harness observes which stack member is expanded.
TICKET
    done
    write_stub_provider "$root"
    write_zellij_wrapper "$root"
    write_pty_runner "$root"
    (
        cd "$root"
        git init -q
        git config user.name "Lisa Zellij Test"
        git config user.email "lisa-zellij-test@example.invalid"
        git add .
        git commit -qm "fixture baseline"
    )
    printf '%s\n' "$root"
}

start_loop() {
    local root=$1
    local session=$2
    local runner="$root/run-loop.sh"
    cat > "$runner" <<RUNNER
#!/usr/bin/env bash
exec env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME \\
  HOME=$(printf '%q' "$root/home") \\
  ZDOTDIR=$(printf '%q' "$root/home") \\
  PATH=$(printf '%q' "$root/bin:$PATH") \\
  LISA_STUB_EVIDENCE_DIR=$(printf '%q' "$root/evidence") \\
  LISA_STUB_REAL_ZELLIJ=$(printf '%q' "$REAL_ZELLIJ") \\
  LISA_STUB_SESSION=$(printf '%q' "$session") \\
  $(printf '%q' "$LISA_BIN") loop --path $(printf '%q' "$root") --client claude
RUNNER
    chmod +x "$runner"
    printf '%s\n' "50 140" > "$root/evidence/size"
    : > "$root/evidence/keys.in"
    (
        cd "$root"
        python3 pty_run.py evidence/size evidence/keys.in evidence/loop.log -- "$runner"
    ) >/dev/null 2>&1 &
    LOOP_PID=$!
    wait_until 30 "named Zellij session $session" session_is_ready
    wait_until 20 "dashboard pane discovery" find_plugin_pane
    wait_until 20 "the four coding panes" agent_pane_count_is 4
}

resize_terminal() {
    printf '%s\n' "$1" > "$CURRENT_ROOT/evidence/size"
}

stop_loop() {
    "$REAL_ZELLIJ" kill-session "$CURRENT_SESSION" >/dev/null 2>&1 || true
    if [[ -n "$LOOP_PID" ]]; then
        kill "$LOOP_PID" >/dev/null 2>&1 || true
        wait "$LOOP_PID" >/dev/null 2>&1 || true
    fi
    LOOP_PID=
    CURRENT_SESSION=
    CURRENT_PLUGIN_PANE=
}

assert_layout_has_no_expanded_attribute() {
    grep -Fq 'stacked=true' "$CURRENT_ROOT/.lisa-layout.kdl" \
        || fail "the generated layout no longer stacks the coding panes"
    if grep -Fq 'expanded' "$CURRENT_ROOT/.lisa-layout.kdl"; then
        fail "the generated layout carries an expanded attribute"
    fi
    local pane_count
    pane_count=$(awk '/stacked=true/, /^        }/' "$CURRENT_ROOT/.lisa-layout.kdl" \
        | grep -c '^ *pane$' || true)
    [[ "$pane_count" == 4 ]] \
        || fail "expected 2 x max_threads = 4 coding panes, found $pane_count"
}

# Nothing is schedulable, so no seat holds a lease: there is nothing to promote
# and lisa must leave the stack exactly as the operator left it.
run_idle_board() {
    echo "scenario idle-board"
    CURRENT_ROOT=$(create_fixture idle-board T-STUB-01:blocked)
    CURRENT_SESSION="lisa-t062-idle-$$"
    start_loop "$CURRENT_ROOT" "$CURRENT_SESSION"
    assert_layout_has_no_expanded_attribute

    # The operator walks down through the stack and stops at the dashboard.
    focus_walk_to "$CURRENT_PLUGIN_PANE"
    local before
    before=$(expanded_pane)
    echo "  idle board: expanded=terminal_$before focus=$CURRENT_PLUGIN_PANE"

    settle
    [[ "$(expanded_pane)" == "$before" ]] \
        || fail "an idle board moved the expansion from terminal_$before to terminal_$(expanded_pane)"
    focused_is "$CURRENT_PLUGIN_PANE" \
        || fail "an idle board moved focus to $(focused_pane)"
    echo "  idle board: unchanged six seconds later"
    stop_loop
}

run_follow() {
    echo "scenario follow"
    CURRENT_ROOT=$(create_fixture follow T-STUB-01:open T-STUB-02:open)
    CURRENT_SESSION="lisa-t062-follow-$$"
    start_loop "$CURRENT_ROOT" "$CURRENT_SESSION"
    assert_layout_has_no_expanded_attribute
    wait_until 60 "both seats to take a ticket" event_count_at_least ack 2

    local first second first_slot second_slot
    first=$(pane_for_ticket T-STUB-01)
    second=$(pane_for_ticket T-STUB-02)
    [[ "$first" != none && "$second" != none ]] \
        || fail "the two tickets did not land on named panes"
    first_slot=$(lisa_pane_for_ticket T-STUB-01)
    second_slot=$(lisa_pane_for_ticket T-STUB-02)
    echo "  T-STUB-01 on terminal_$first (lisa slot $first_slot), T-STUB-02 on terminal_$second (lisa slot $second_slot)"

    # --- The reported state, reproduced ------------------------------------
    # Both seats are working; T-STUB-01 spoke last.
    beat "$second_slot"
    beat "$first_slot"
    # The operator walks down towards the dashboard and pauses on a spare pane,
    # which zellij expands because focusing a stacked member expands it.
    local spare
    spare=$(idle_pane)
    [[ "$spare" != none ]] || fail "no idle coding pane to walk through"
    focus_walk_to "terminal_$spare"
    wait_until 10 "the spare pane to expand under focus" expanded_is "$spare"

    # --- With focus inside the stack, nothing happens -----------------------
    # This is the non-event: T-STUB-02 wakes up while the operator is reading
    # the spare pane, and the stack does not move under them.
    beat "$second_slot"
    settle
    [[ "$(expanded_pane)" == "$spare" ]] \
        || fail "focus inside the stack still moved the expansion to terminal_$(expanded_pane)"
    [[ "$(focused_pane)" == "terminal_$spare" ]] \
        || fail "focus inside the stack was moved to $(focused_pane)"
    echo "  focus inside the stack: expanded stayed terminal_$spare through a heartbeat on terminal_$second"

    # The operator carries on to the last member of the stack and steps out to
    # the dashboard. The state they leave behind is the reported one: an idle
    # pane holding the whole 70%. Nothing in zellij ever moves it back — the
    # idle-board scenario above is that same expansion, still there six seconds
    # later — so what happens next is lisa or nothing.
    local last_spare
    last_spare=$(last_idle_pane)
    focus_walk_to "terminal_$last_spare"
    wait_until 10 "the last spare pane to expand under focus" expanded_is "$last_spare"
    echo "  reproduced: expanded=terminal_$last_spare (idle, holds no lease), operator now steps out"
    focus_walk_to "$CURRENT_PLUGIN_PANE"

    # --- Fixed --------------------------------------------------------------
    wait_until 20 "the working pane to take the stack" expanded_is "$second"
    [[ "$(focused_pane)" == "$CURRENT_PLUGIN_PANE" ]] \
        || fail "promotion moved focus to $(focused_pane)"
    echo "  fixed: expanded=terminal_$second (T-STUB-02, newest heartbeat) focus=$CURRENT_PLUGIN_PANE"

    # --- It follows the newest heartbeat, and only ever a leased pane -------
    beat "$first_slot"
    wait_until 20 "the newest heartbeat to take the stack" expanded_is "$first"
    [[ "$(focused_pane)" == "$CURRENT_PLUGIN_PANE" ]] \
        || fail "following the newest heartbeat moved focus to $(focused_pane)"
    beat "$second_slot"
    wait_until 20 "the stack to follow back" expanded_is "$second"
    [[ "$(focused_pane)" == "$CURRENT_PLUGIN_PANE" ]] \
        || fail "following the newest heartbeat moved focus to $(focused_pane)"
    echo "  follows the newest heartbeat, both directions, focus untouched"

    # An idle spare is never the answer while a seat is working: the promotion
    # target is a leased pane on every observation above, and nothing here ever
    # returns to terminal_$spare.
    [[ "$(expanded_pane)" != "$spare" ]] \
        || fail "an idle pane displaced a working one"

    # --- A resize changes nothing ------------------------------------------
    # Zellij's relayout drifts the expansion back to the stack's last-focused
    # member; the operator must not see the idle pane again, and must not be
    # moved.
    resize_terminal "40 110"
    settle
    [[ "$(focused_pane)" == "$CURRENT_PLUGIN_PANE" ]] \
        || fail "a resize moved focus to $(focused_pane)"
    expanded_is "$second" \
        || fail "after a resize the stack shows terminal_$(expanded_pane), not the working terminal_$second"
    resize_terminal "50 140"
    settle
    [[ "$(focused_pane)" == "$CURRENT_PLUGIN_PANE" ]] \
        || fail "a resize back moved focus to $(focused_pane)"
    expanded_is "$second" \
        || fail "after resizing back the stack shows terminal_$(expanded_pane), not the working terminal_$second"
    echo "  resize: expanded=terminal_$second focus=$CURRENT_PLUGIN_PANE, both ways"

    assert_layout_has_no_expanded_attribute
    stop_loop
}

run_idle_board
run_follow

echo "real-zellij-stack-follow: PASS"
