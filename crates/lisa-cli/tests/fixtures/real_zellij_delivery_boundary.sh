#!/usr/bin/env bash
# Deterministic, model-free regression for Lisa's two real Zellij delivery
# boundaries. Run via the ignored Cargo integration test; LISA_BIN must name the
# freshly built CLI whose embedded WASM is under test.
set -euo pipefail

: "${LISA_BIN:?set LISA_BIN to the freshly built lisa executable}"

for dependency in bash git jq script zellij zsh; do
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
RUN_ROOT=$(mktemp -d "$TEST_PARENT/lisa-real-zellij.XXXXXX")
KEEP_FIXTURES=${KEEP_LISA_ZELLIJ_FIXTURES:-0}
CURRENT_ROOT=
CURRENT_SESSION=
CURRENT_PLUGIN_PANE=
CURRENT_AGENT_PANE=
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
        echo "--- events ---" >&2
        sed -n '1,240p' "$CURRENT_ROOT/evidence/events.log" >&2 2>/dev/null || true
        echo "--- panes ---" >&2
        session_action list-panes --json --all >&2 2>/dev/null || true
        echo "--- dashboard ---" >&2
        dump_pane "$CURRENT_PLUGIN_PANE" >&2 2>/dev/null || true
        echo "--- terminal ---" >&2
        dump_pane "$CURRENT_AGENT_PANE" >&2 2>/dev/null || true
        echo "--- signals ---" >&2
        find "$CURRENT_ROOT/.lisa/signals" -maxdepth 1 -type f -print -exec sh -c 'printf "  "; cat "$1"; printf "\n"' _ {} \; >&2 2>/dev/null || true
        echo "--- launch scripts ---" >&2
        find "$CURRENT_ROOT/.lisa/attempts" -name '.lisa-launch-*.sh' -type f -print -exec sed -n '1,160p' {} \; >&2 2>/dev/null || true
        echo "--- loop client ---" >&2
        sed -n '1,240p' "$CURRENT_ROOT/evidence/loop.log" >&2 2>/dev/null || true
    fi
    return 1
}

session_action() {
    "$REAL_ZELLIJ" --session "$CURRENT_SESSION" action "$@"
}

dump_pane() {
    local pane=$1
    [[ -n "$pane" ]] || return 1
    if [[ "$pane" == plugin_* ]]; then
        session_action focus-pane-id "$pane" >/dev/null 2>&1 || true
        session_action dump-screen | tr -d '\r'
    else
        session_action dump-screen --pane-id "$pane" | tr -d '\r'
    fi
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

event_count_is() {
    local kind=$1
    local expected=$2
    local actual=0
    if [[ -f "$CURRENT_ROOT/evidence/events.log" ]]; then
        actual=$(awk -F '\t' -v kind="$kind" '$1 == kind { count++ } END { print count + 0 }' "$CURRENT_ROOT/evidence/events.log")
    fi
    [[ "$actual" == "$expected" ]]
}

event_count_at_least() {
    local kind=$1
    local expected=$2
    local actual=0
    if [[ -f "$CURRENT_ROOT/evidence/events.log" ]]; then
        actual=$(awk -F '\t' -v kind="$kind" '$1 == kind { count++ } END { print count + 0 }' "$CURRENT_ROOT/evidence/events.log")
    fi
    (( actual >= expected ))
}

start_signal_consumed() {
    [[ ! -e "$CURRENT_ROOT/.lisa/signals/pane-0.started" ]]
}

session_is_ready() {
    session_action list-panes --json --all >/dev/null 2>&1
}

discover_panes() {
    local panes
    panes=$(session_action list-panes --json --all) || return 1
    CURRENT_PLUGIN_PANE=$(jq -r \
        '[.[] | select(.is_plugin == true and ((.plugin_url // "") | startswith("file:")))][0] | if . then "plugin_\(.id)" else empty end' \
        <<<"$panes")
    CURRENT_AGENT_PANE=$(jq -r \
        '[.[] | select(.is_plugin == false and ((.title // "") | contains("T-STUB-01")))][0] | if . then "terminal_\(.id)" else empty end' \
        <<<"$panes")
    if [[ -z "$CURRENT_AGENT_PANE" ]]; then
        CURRENT_AGENT_PANE=$(jq -r \
            '[.[] | select(.is_plugin == false)][0] | if . then "terminal_\(.id)" else empty end' \
            <<<"$panes")
    fi
    [[ -n "$CURRENT_PLUGIN_PANE" && -n "$CURRENT_AGENT_PANE" ]]
}

dashboard_has() {
    local state=$1
    local dump
    discover_panes || return 1
    dump=$(dump_pane "$CURRENT_PLUGIN_PANE" 2>/dev/null) || return 1
    printf '%s\n' "$dump" > "$CURRENT_ROOT/evidence/dashboard.txt"
    grep -Fq "$state" <<<"$dump"
}

terminal_has() {
    local text=$1
    local dump
    discover_panes || return 1
    dump=$(dump_pane "$CURRENT_AGENT_PANE" 2>/dev/null) || return 1
    printf '%s\n' "$dump" > "$CURRENT_ROOT/evidence/terminal.txt"
    grep -Fq "$text" <<<"$dump"
}

assert_not_owned() {
    local dump
    discover_panes || fail "cannot refresh panes for non-owned assertion"
    dump=$(dump_pane "$CURRENT_PLUGIN_PANE") || fail "cannot dump dashboard for non-owned assertion"
    printf '%s\n' "$dump" > "$CURRENT_ROOT/evidence/dashboard.txt"
    if grep -Eq '(^|[^[:alnum:]-])owned([^[:alnum:]-]|$)' <<<"$dump"; then
        fail "dashboard published Owned before matching acknowledgement"
    fi
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
: "${LISA_STUB_SCENARIO:?}"
: "${LISA_STUB_EVIDENCE_DIR:?}"
: "${LISA_STUB_START_GATE:?}"

signals=.lisa/signals
events=$LISA_STUB_EVIDENCE_DIR/events.log
mkdir -p "$signals" "$LISA_STUB_EVIDENCE_DIR"
printf 'launch\tpane=%s\tgeneration=%s\n' "$LISA_PANE_ID" "$LISA_ATTEMPT_ID" >> "$events"

publish_start() {
    local marker="$signals/pane-$LISA_PANE_ID.lease"
    local destination="$signals/pane-$LISA_PANE_ID.started"
    local tmp="$destination.tmp.$$"
    cp "$marker" "$tmp"
    mv "$tmp" "$destination"
    printf 'start\tpane=%s\tgeneration=%s\tlease=%s\n' \
        "$LISA_PANE_ID" "$LISA_ATTEMPT_ID" "$(cat "$marker")" >> "$events"
}

publish_ack() {
    local prompt=$1
    local destination="$signals/pane-$LISA_PANE_ID.ack"
    local tmp="$destination.tmp.$$"
    jq -cn --arg prompt "$prompt" \
        '{hook_event_name:"UserPromptSubmit",prompt:$prompt}' > "$tmp"
    mv "$tmp" "$destination"
    printf 'ack\tpane=%s\tgeneration=%s\n' "$LISA_PANE_ID" "$LISA_ATTEMPT_ID" >> "$events"
}

case "$LISA_STUB_SCENARIO" in
    suppress-start)
        while :; do sleep 1; done
        ;;
    dquote)
        if [[ "$LISA_ATTEMPT_ID" == 1 ]]; then
            printf 'dquote\tpane=%s\tgeneration=%s\n' "$LISA_PANE_ID" "$LISA_ATTEMPT_ID" >> "$events"
            (
                sleep 1
                "$LISA_STUB_REAL_ZELLIJ" --session "$LISA_STUB_SESSION" action \
                    write-chars --pane-id "$LISA_PANE_ID" '"unterminated'
                "$LISA_STUB_REAL_ZELLIJ" --session "$LISA_STUB_SESSION" action \
                    write --pane-id "$LISA_PANE_ID" 13
            ) >/dev/null 2>&1 &
            exit 0
        fi
        ;;
    success|suppress-ack)
        ;;
    *)
        echo "unknown stub scenario: $LISA_STUB_SCENARIO" >&2
        exit 2
        ;;
esac

while [[ ! -f "$LISA_STUB_START_GATE" ]]; do sleep 1; done
publish_start

prompt=
while IFS= read -r line; do
    case "$line" in
        "Read and follow the complete assignment at "*)
            prompt=$line
            ;;
        "LISA_ASSIGNMENT "*)
            if [[ -n "$prompt" ]]; then
                prompt=$(printf '%s\n%s' "$prompt" "$line")
            else
                prompt=$line
            fi
            printf 'chat\tpane=%s\tgeneration=%s\tprompt=%s\n' \
                "$LISA_PANE_ID" "$LISA_ATTEMPT_ID" "$(printf '%s' "$prompt" | tr '\n' '|')" >> "$events"
            if [[ "$LISA_STUB_SCENARIO" != suppress-ack ]]; then
                while [[ ! -f "$LISA_STUB_ACK_GATE" ]]; do sleep 1; done
                publish_ack "$prompt"
                while :; do sleep 1; done
            fi
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
if [[ "${1:-}" == "--version" ]]; then
    exec "$LISA_STUB_REAL_ZELLIJ" "$@"
fi
if [[ "${1:-}" == "--layout" && $# == 2 ]]; then
    exec "$LISA_STUB_REAL_ZELLIJ" --session "$LISA_STUB_SESSION" \
        --new-session-with-layout "$2"
fi
echo "unexpected zellij invocation: $*" >&2
exit 2
WRAPPER
    chmod +x "$root/bin/zellij"
}

create_fixture() {
    local scenario=$1
    local root="$RUN_ROOT/$scenario"
    mkdir -p "$root/bin" "$root/evidence" "$root/home"
    "$LISA_BIN" init --path "$root" >/dev/null
    mkdir -p "$root/docs/active/tickets" "$root/docs/active/stories"
    cat > "$root/.lisa.toml" <<'TOML'
version = "0.4.0"

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 1
auto_advance = false
review_timeout_secs = 300
session_timeout_secs = 0
wind_down_secs = 1
assignment_ack_timeout_secs = 5

[agent]
client = "claude"
TOML
    cat > "$root/docs/active/stories/S-STUB.md" <<'STORY'
---
id: S-STUB
title: stub-provider-boundary
status: active
---

# Stub provider boundary
STORY
    cat > "$root/docs/active/tickets/T-STUB-01.md" <<'TICKET'
---
id: T-STUB-01
story: S-STUB
title: delivery-boundary
type: task
status: open
priority: high
agent: claude
phase: research
depends_on: []
---

## Context

Deterministic local-provider fixture. The provider must not execute this ticket.

## Acceptance Criteria

- [ ] The harness observes delivery boundaries.
TICKET
    write_stub_provider "$root"
    write_zellij_wrapper "$root"
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
    local scenario=$1
    local root=$2
    local session=$3
    local runner="$root/run-loop.sh"
    cat > "$runner" <<RUNNER
#!/usr/bin/env bash
stty rows 50 cols 140 2>/dev/null || true
exec env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME \\
  HOME=$(printf '%q' "$root/home") \\
  ZDOTDIR=$(printf '%q' "$root/home") \\
  PATH=$(printf '%q' "$root/bin:$PATH") \\
  LISA_STUB_SCENARIO=$(printf '%q' "$scenario") \\
  LISA_STUB_EVIDENCE_DIR=$(printf '%q' "$root/evidence") \\
  LISA_STUB_START_GATE=$(printf '%q' "$root/evidence/start-gate") \\
  LISA_STUB_ACK_GATE=$(printf '%q' "$root/evidence/ack-gate") \\
  LISA_STUB_REAL_ZELLIJ=$(printf '%q' "$REAL_ZELLIJ") \\
  LISA_STUB_SESSION=$(printf '%q' "$session") \\
  $(printf '%q' "$LISA_BIN") loop --path $(printf '%q' "$root") --client claude
RUNNER
    chmod +x "$runner"
    if script --version 2>&1 | grep -qi util-linux; then
        script -q -c "$runner" "$root/evidence/loop.log" >/dev/null 2>&1 &
    else
        script -q "$root/evidence/loop.log" "$runner" >/dev/null 2>&1 &
    fi
    LOOP_PID=$!
    wait_until 30 "named Zellij session $session" session_is_ready
    wait_until 20 "agent and plugin pane discovery" discover_panes
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
    CURRENT_AGENT_PANE=
}

assert_launch_contract() {
    local scripts=()
    while IFS= read -r path; do scripts+=("$path"); done < <(find "$CURRENT_ROOT/.lisa/attempts" -name '.lisa-launch-*.sh' -type f | sort)
    (( ${#scripts[@]} >= 1 )) || fail "no attempt-private launch script was created"
    local script_path
    for script_path in "${scripts[@]}"; do
        grep -Fq 'claude --dangerously-skip-permissions' "$script_path" || fail "launch script does not contain bare stub provider command"
        if grep -Eq 'Read the ticket|LISA_ASSIGNMENT|complete assignment' "$script_path"; then
            fail "launch script contains assignment prose"
        fi
    done
}

assert_chat_contract() {
    grep -Fq 'Read and follow the complete assignment at .lisa/attempts/T-STUB-01/' \
        "$CURRENT_ROOT/evidence/events.log" || fail "chat did not reference the attempt-private assignment"
    grep -Fq 'LISA_ASSIGNMENT {"ticket_id":"T-STUB-01","generation":' \
        "$CURRENT_ROOT/evidence/events.log" || fail "chat did not carry an attempt marker"
}

assert_two_same_pane_launches() {
    local rows panes generations
    rows=$(awk -F '\t' '$1 == "launch" { print }' "$CURRENT_ROOT/evidence/events.log")
    [[ $(wc -l <<<"$rows" | tr -d ' ') == 2 ]] || fail "expected exactly two launch attempts"
    panes=$(sed -E 's/.*pane=([0-9]+).*/\1/' <<<"$rows" | sort -u)
    [[ $(wc -l <<<"$panes" | tr -d ' ') == 1 ]] || fail "recovery changed physical pane"
    generations=$(sed -E 's/.*generation=([0-9]+).*/\1/' <<<"$rows")
    local first second
    first=$(sed -n '1p' <<<"$generations")
    second=$(sed -n '2p' <<<"$generations")
    (( second > first )) || fail "recovery generation did not increase"
}

run_success() {
    echo "scenario success"
    CURRENT_ROOT=$(create_fixture success)
    CURRENT_SESSION="lisa-t035-success-$$"
    start_loop success "$CURRENT_ROOT" "$CURRENT_SESSION"
    wait_until 15 "stub launch" event_count_is launch 1
    touch "$CURRENT_ROOT/evidence/start-gate"
    wait_until 10 "stub start publication" event_count_is start 1
    wait_until 30 "exact process-start signal consumption" start_signal_consumed
    assert_not_owned
    wait_until 20 "bounded chat receipt" event_count_is chat 1
    wait_until 15 "Delivering" dashboard_has delivering
    assert_not_owned
    touch "$CURRENT_ROOT/evidence/ack-gate"
    wait_until 20 "matching ack publication" event_count_is ack 1
    wait_until 20 "Owned after ack" dashboard_has owned
    assert_launch_contract
    assert_chat_contract
    event_count_is launch 1 || fail "success scenario launched more than once"
    stop_loop
}

run_suppress_start() {
    echo "scenario suppress-start"
    CURRENT_ROOT=$(create_fixture suppress-start)
    CURRENT_SESSION="lisa-t035-no-start-$$"
    start_loop suppress-start "$CURRENT_ROOT" "$CURRENT_SESSION"
    wait_until 75 "bounded replacement startup failure" dashboard_has 'FAILED T-STUB-01'
    assert_not_owned
    event_count_is launch 2 || fail "suppressed start did not stop after one same-pane relaunch"
    assert_two_same_pane_launches
    event_count_is chat 0 || fail "chat was delivered without process-start evidence"
    sleep 7
    event_count_is launch 2 || fail "suppressed start relaunched without bound"
    stop_loop
}

run_suppress_ack() {
    echo "scenario suppress-ack"
    CURRENT_ROOT=$(create_fixture suppress-ack)
    CURRENT_SESSION="lisa-t035-no-ack-$$"
    start_loop suppress-ack "$CURRENT_ROOT" "$CURRENT_SESSION"
    wait_until 15 "stub launch" event_count_is launch 1
    touch "$CURRENT_ROOT/evidence/start-gate"
    wait_until 10 "stub start publication" event_count_is start 1
    wait_until 30 "exact process-start signal consumption" start_signal_consumed
    assert_not_owned
    wait_until 20 "initial chat receipt" event_count_at_least chat 1
    wait_until 15 "Delivering" dashboard_has delivering
    assert_not_owned
    wait_until 60 "bounded delivery failure" dashboard_has 'FAILED T-STUB-01'
    assert_not_owned
    event_count_is launch 1 || fail "missing chat ack restarted the provider"
    event_count_is chat 2 || fail "missing chat ack did not perform exactly one retry"
    sleep 7
    event_count_is chat 2 || fail "chat delivery retried without bound"
    assert_chat_contract
    stop_loop
}

run_dquote() {
    echo "scenario dquote"
    CURRENT_ROOT=$(create_fixture dquote)
    CURRENT_SESSION="lisa-t035-dquote-$$"
    start_loop dquote "$CURRENT_ROOT" "$CURRENT_SESSION"
    wait_until 20 "real zsh dquote continuation prompt" terminal_has 'dquote>'
    assert_not_owned
    wait_until 40 "same-pane replacement launch" event_count_is launch 2
    touch "$CURRENT_ROOT/evidence/start-gate"
    wait_until 10 "replacement start publication" event_count_is start 1
    wait_until 45 "replacement process-start signal consumption" start_signal_consumed
    assert_not_owned
    wait_until 20 "replacement bounded chat" event_count_is chat 1
    wait_until 15 "replacement Delivering" dashboard_has delivering
    assert_not_owned
    touch "$CURRENT_ROOT/evidence/ack-gate"
    wait_until 20 "replacement matching ack" event_count_is ack 1
    wait_until 20 "replacement Owned" dashboard_has owned
    assert_two_same_pane_launches
    event_count_is dquote 1 || fail "dquote fault was not injected exactly once"
    event_count_is start 1 || fail "only the replacement may publish process start"
    sleep 7
    event_count_is launch 2 || fail "dquote recovery relaunched more than once"
    assert_launch_contract
    assert_chat_contract
    stop_loop
}

run_success
run_suppress_start
run_suppress_ack
run_dquote

echo "real-zellij-delivery-boundary: PASS"
