#!/usr/bin/env bash
# The incident, reproduced and healed (T-067-01-03). Run via the ignored Cargo
# integration test; LISA_BIN must name the freshly built CLI whose embedded WASM
# is under test.
#
# Everything here is observed through real Zellij. A pane is killed for real —
# focused and closed through `zellij action`, which is byte-for-byte what the
# server sees when a pane's shell exits or an operator closes it, because at the
# Zellij level those are the same event. The arrangement afterwards is read from
# `list-panes --json`: the coding panes are a stack (one member expanded, the
# rest one row of title bar, all the same width) sitting above a dashboard whose
# height must not have moved.
#
# Two kills, because they are different failures:
#
#   1. An idle pane dies. Nothing is in flight in it; the board must simply have
#      four panes again, with both working seats untouched.
#   2. A pane with an agent in it dies. That attempt is over and must be
#      recorded as a lost seat, the *other* attempt must not notice, and the
#      pane that replaces it must be a fresh seat rather than the dead one
#      resumed.
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
RUN_ROOT=$(mktemp -d "$TEST_PARENT/lisa-pane-regeneration.XXXXXX")
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
        echo "--- ledger ---" >&2
        cat "$CURRENT_ROOT/.lisa/provenance.jsonl" >&2 2>/dev/null || true
        echo "--- stub events ---" >&2
        cat "$CURRENT_ROOT/evidence/events.log" >&2 2>/dev/null || true
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

find_plugin_pane() {
    local panes
    panes=$(panes_json) || return 1
    CURRENT_PLUGIN_PANE=$(jq -r \
        '[.[] | select(.is_plugin == true and ((.plugin_url // "") | startswith("file:")))][0]
         | if . then "plugin_\(.id)" else empty end' <<<"$panes")
    [[ -n "$CURRENT_PLUGIN_PANE" ]]
}

# The coding panes, newest layout order first, as bare terminal ids.
coding_panes() {
    panes_json | jq -r '[.[] | select(.is_plugin == false)] | sort_by(.pane_y) | .[].id' | tr '\n' ' '
}

coding_pane_count_is() {
    local expected=$1
    local actual
    actual=$(panes_json | jq '[.[] | select(.is_plugin == false)] | length') || return 1
    [[ "$actual" == "$expected" ]]
}

# The dashboard's height and top edge, which the 30% half of the split fixes.
# Healing must not move it: a replacement pane that landed beside the stack
# would take its space, which is the shape this ticket calls "a new bug wearing
# the fix's clothes".
dashboard_geometry() {
    panes_json | jq -r --arg pane "${CURRENT_PLUGIN_PANE#plugin_}" \
        '[.[] | select(.is_plugin == true and (.id|tostring) == $pane)][0]
         | "\(.pane_y)x\(.pane_rows)"'
}

# Whether the coding panes really are one stack: same left edge, same width,
# exactly one member taller than its title bar.
panes_are_one_stack() {
    local expected=$1
    panes_json | jq -e --argjson expected "$expected" '
        [.[] | select(.is_plugin == false)] as $coding
        | ($coding | length) == $expected
          and ([$coding[].pane_x] | unique | length) == 1
          and ([$coding[].pane_columns] | unique | length) == 1
          and ([$coding[] | select(.pane_rows > 1)] | length) == 1
    ' >/dev/null
}

pane_title() {
    panes_json | jq -r --argjson id "$1" \
        '[.[] | select(.is_plugin == false and .id == $id)][0] | .title // "none"'
}

# The zellij terminal id of the pane working a ticket, read off the
# scheduler-owned pane name.
pane_for_ticket() {
    local ticket=$1
    panes_json | jq -r --arg ticket "$ticket" \
        '[.[] | select(.is_plugin == false and ((.title // "") | contains($ticket)))][0]
         | if . then .id else "none" end'
}

idle_panes() {
    panes_json | jq -r '[.[] | select(.is_plugin == false and ((.title // "") | contains("idle")))]
        | sort_by(.pane_y) | .[].id' | tr '\n' ' '
}

# Lisa's slot number for a ticket, as its provider announced it at launch.
lisa_pane_for_ticket() {
    local ticket=$1
    awk -F '\t' -v ticket="ticket=$ticket" \
        '$1 == "launch" { hit = 0; for (i = 2; i <= NF; i++) if ($i == ticket) hit = 1 }
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
# plugin has consumed the heartbeat it publishes. A seat that still answers this
# is a seat that kept its lease: the hook only publishes when its own launch
# identity byte-matches `pane-<id>.lease`.
beat() {
    local lisa_pane=$1
    local before
    before=$(event_count beat)
    touch "$CURRENT_ROOT/evidence/beat-$lisa_pane"
    wait_until 20 "heartbeat from lisa pane $lisa_pane" event_count_at_least beat $(( before + 1 ))
    wait_until 20 "heartbeat consumption by the plugin" \
        test ! -e "$CURRENT_ROOT/.lisa/signals/pane-$lisa_pane.heartbeat"
}

# Terminal rows in the ledger for one ticket, by outcome.
ledger_outcomes() {
    local ticket=$1
    local ledger="$CURRENT_ROOT/.lisa/provenance.jsonl"
    [[ -f "$ledger" ]] || return 0
    jq -r --arg ticket "$ticket" \
        'select(.record_type == null or .record_type == "execution")
         | select(.ticket_id == $ticket) | .outcome // empty' \
        "$ledger" 2>/dev/null | sort | tr '\n' ' '
}

seat_lost_rows() {
    local ticket=$1
    local ledger="$CURRENT_ROOT/.lisa/provenance.jsonl"
    [[ -f "$ledger" ]] || { printf '0\n'; return; }
    jq -s --arg ticket "$ticket" \
        '[.[] | select(.ticket_id == $ticket and .outcome == "seat-lost")] | length' \
        "$ledger" 2>/dev/null || printf '0\n'
}

seat_lost_rows_at_least() {
    (( $(seat_lost_rows "$1") >= $2 ))
}

# Kill a pane the way the world kills one. Zellij cannot tell a shell that
# exited from an operator closing the pane from a crashed terminal emulator —
# they arrive at the server as the same event — so closing it is the faithful
# reproduction, not a shortcut around one.
kill_pane() {
    local target=$1
    session_action focus-pane-id "terminal_$target" >/dev/null 2>&1 || true
    sleep 0.5
    session_action close-pane >/dev/null 2>&1 || true
    wait_until 20 "pane $target to be gone" pane_is_gone "$target"
}

pane_is_gone() {
    local target=$1
    ! panes_json | jq -e --argjson id "$1" \
        '[.[] | select(.is_plugin == false and .id == $id)] | length > 0' >/dev/null
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

# One tool call per request from the harness, so "this seat still works" is
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

write_pty_runner() {
    local root=$1
    cat > "$root/pty_run.py" <<'PTY'
"""Run a command on a real pty, so the loop client has a terminal to live on."""
import fcntl, os, struct, sys, termios

out_log = sys.argv[1]
cmd = sys.argv[sys.argv.index("--") + 1:]

pid, fd = os.forkpty()
if pid == 0:
    os.environ["TERM"] = "xterm-256color"
    os.execvp(cmd[0], cmd)

fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))

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

# $1 scenario name, $2... ticket ids
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
title: pane-regeneration
status: active
---

# Pane regeneration
STORY
    local ticket
    for ticket in "$@"; do
        cat > "$root/docs/active/tickets/$ticket.md" <<TICKET
---
id: $ticket
story: S-STUB
title: pane-regeneration-seat
type: task
status: open
priority: high
agent: claude
phase: implement
depends_on: []
---

## Context

Deterministic local-provider fixture. The provider must not execute this ticket.

## Acceptance Criteria

- [ ] The harness kills a pane and watches the board put it back.
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
    (
        cd "$root"
        python3 pty_run.py evidence/loop.log -- "$runner"
    ) >/dev/null 2>&1 &
    LOOP_PID=$!
    wait_until 30 "named Zellij session $session" session_is_ready
    wait_until 20 "dashboard pane discovery" find_plugin_pane
    wait_until 20 "the four coding panes" coding_pane_count_is 4
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

# The layout is the source of truth for how many panes a run should have, and it
# has to tell the plugin the same number it created.
assert_layout_declares_what_it_creates() {
    local created declared
    created=$(awk '/stacked=true/, /^        }/' "$CURRENT_ROOT/.lisa-layout.kdl" \
        | grep -c '^ *pane$' || true)
    declared=$(sed -n 's/.*agent_panes "\([0-9]*\)".*/\1/p' "$CURRENT_ROOT/.lisa-layout.kdl")
    [[ "$created" == 4 ]] \
        || fail "expected 2 x max_threads = 4 coding panes in the layout, found $created"
    [[ "$declared" == "$created" ]] \
        || fail "the layout created $created panes and declared $declared"
    echo "  layout: created $created coding panes and told the plugin so"
}

run_regeneration() {
    echo "scenario pane-regeneration"
    CURRENT_ROOT=$(create_fixture regeneration T-STUB-01 T-STUB-02)
    CURRENT_SESSION="lisa-t067-heal-$$"
    start_loop "$CURRENT_ROOT" "$CURRENT_SESSION"
    assert_layout_declares_what_it_creates
    wait_until 60 "both seats to take a ticket" event_count_at_least ack 2

    local first second first_slot second_slot dashboard_before
    first=$(pane_for_ticket T-STUB-01)
    second=$(pane_for_ticket T-STUB-02)
    [[ "$first" != none && "$second" != none ]] \
        || fail "the two tickets did not land on named panes"
    first_slot=$(lisa_pane_for_ticket T-STUB-01)
    second_slot=$(lisa_pane_for_ticket T-STUB-02)
    dashboard_before=$(dashboard_geometry)
    echo "  working: T-STUB-01 on terminal_$first, T-STUB-02 on terminal_$second"
    echo "  panes:   $(coding_panes)| dashboard $dashboard_before"

    # --- 1. An idle pane dies ----------------------------------------------
    local spare
    spare=$(idle_panes | awk '{ print $1 }')
    [[ -n "$spare" && "$spare" != none ]] || fail "no idle coding pane to kill"
    echo "  killing idle terminal_$spare"
    # `kill_pane` already waits for the pane to be gone; there is deliberately no
    # assertion that the board is *observed* at three panes. The loop heals on
    # the same `PaneUpdate` that reports the death, so the short board can be
    # over before anything outside Zellij can look at it — which is the point.
    kill_pane "$spare"

    wait_until 30 "the board to have four panes again" coding_pane_count_is 4
    panes_are_one_stack 4 \
        || fail "after healing the coding panes are not one stack: $(panes_json)"
    [[ "$(dashboard_geometry)" == "$dashboard_before" ]] \
        || fail "healing moved the dashboard from $dashboard_before to $(dashboard_geometry)"
    echo "  healed:  $(coding_panes)| dashboard $(dashboard_geometry)"

    # Nothing in flight was disturbed: both agents still hold their panes,
    # their names, and — the part only the lease can prove — their seats.
    [[ "$(pane_for_ticket T-STUB-01)" == "$first" ]] \
        || fail "T-STUB-01 moved from terminal_$first to terminal_$(pane_for_ticket T-STUB-01)"
    [[ "$(pane_for_ticket T-STUB-02)" == "$second" ]] \
        || fail "T-STUB-02 moved from terminal_$second to terminal_$(pane_for_ticket T-STUB-02)"
    beat "$first_slot"
    beat "$second_slot"
    [[ "$(seat_lost_rows T-STUB-01)" == 0 && "$(seat_lost_rows T-STUB-02)" == 0 ]] \
        || fail "an idle pane's death was recorded against a live attempt"
    echo "  in flight: both seats kept their panes and their leases"

    # --- 2. Asked, and already fine ----------------------------------------
    local answer
    answer=$(cd "$CURRENT_ROOT" && "$LISA_BIN" heal-panes --asked-by rail --timeout-secs 30 --json)
    jq -e '.answered == true and .answer == "already-fine" and .present == 4' >/dev/null <<<"$answer" \
        || fail "asking a whole board did not answer already-fine: $answer"
    coding_pane_count_is 4 || fail "asking a whole board created a pane"
    echo "  asked:   $(jq -r '.answer' <<<"$answer") ($(jq -r '.present' <<<"$answer") of $(jq -r '.declared' <<<"$answer") panes)"

    # --- 3. A pane with an agent in it dies ---------------------------------
    local before_kill
    before_kill=$(coding_panes)
    echo "  killing working terminal_$second (T-STUB-02)"
    kill_pane "$second"
    wait_until 30 "the board to have four panes again" coding_pane_count_is 4
    panes_are_one_stack 4 \
        || fail "after healing a working pane the coding panes are not one stack"
    [[ "$(dashboard_geometry)" == "$dashboard_before" ]] \
        || fail "healing a working pane moved the dashboard to $(dashboard_geometry)"

    # The attempt that died with the pane is over, and said so.
    wait_until 30 "the lost seat to be recorded" seat_lost_rows_at_least T-STUB-02 1
    echo "  recorded: T-STUB-02 outcomes [$(ledger_outcomes T-STUB-02)]"

    # The other attempt did not notice. This is the whole reason to heal rather
    # than restart the loop.
    [[ "$(pane_for_ticket T-STUB-01)" == "$first" ]] \
        || fail "the surviving attempt moved to terminal_$(pane_for_ticket T-STUB-01)"
    beat "$first_slot"
    [[ "$(seat_lost_rows T-STUB-01)" == 0 ]] \
        || fail "the surviving attempt was recorded as lost"
    echo "  in flight: T-STUB-01 still on terminal_$first, still beating"

    # And the pane that replaced it is a fresh seat, not the dead one resumed:
    # an id nothing on this board has used, carrying no ticket. Found by
    # difference against the ids that were there before the kill, so a spare that
    # happened to be idle already cannot pass for the replacement.
    local healed
    healed=$(comm -13 \
        <(printf '%s\n' $before_kill | sort) \
        <(coding_panes | tr ' ' '\n' | grep -v '^$' | sort) | head -1)
    [[ -n "$healed" ]] \
        || fail "no pane id appeared that was not already there: was [$before_kill], now [$(coding_panes)]"
    [[ "$healed" != "$second" ]] || fail "the replacement reused the dead pane's id"
    [[ "$(pane_title "$healed")" == *idle* ]] \
        || fail "the replacement pane is named $(pane_title "$healed"), not an idle seat"
    echo "  fresh:   terminal_$healed is an idle seat ($(pane_title "$healed")), new since [$before_kill]"

    settle
    coding_pane_count_is 4 || fail "the board did not stay at four panes"
    panes_are_one_stack 4 || fail "the stack came apart after settling"
    echo "  settled: four panes, one stack, six seconds later"
    stop_loop
}

run_regeneration

echo "real-zellij-pane-regeneration: PASS"
