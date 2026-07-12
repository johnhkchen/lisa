#!/usr/bin/env bash
# Live, metered validation of the first-assignment boundary for installed Codex
# and Claude clients. Each provider is first in an independent freshly built
# Lisa/Zellij loop. See docs/knowledge/fresh-loop-live-startup.md.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd "$SCRIPT_DIR/../../../.." && pwd -P)
REAL_ZELLIJ=$(command -v zellij || true)
EVIDENCE_DIR=${EVIDENCE_DIR:-${TMPDIR:-/tmp}/lisa-live-startup-evidence-$(date -u +%Y%m%dT%H%M%SZ)}
SKIP_BUILD=${SKIP_BUILD:-0}
SKIP_DETERMINISTIC_PREFLIGHT=${SKIP_DETERMINISTIC_PREFLIGHT:-0}
PREPARE_ONLY=${PREPARE_ONLY:-0}
KEEP_LIVE_FIXTURES=${KEEP_LIVE_FIXTURES:-1}
LIVE_STARTUP_TIMEOUT_SECS=${LIVE_STARTUP_TIMEOUT_SECS:-1200}
LISA_BIN=${LISA_BIN:-}

CURRENT_CASE=
CURRENT_ROOT=
CURRENT_SESSION=
CURRENT_PLUGIN_PANE=
CURRENT_AGENT_PANE=
LOOP_PID=
SAMPLER_PID=

fail() {
    local message=$1
    echo "FAIL: $message" >&2
    if [[ -n "$CURRENT_CASE" && -d "$CURRENT_CASE" ]]; then
        echo "evidence: $CURRENT_CASE" >&2
        sed -n '1,220p' "$CURRENT_CASE/state-events.tsv" >&2 2>/dev/null || true
        sed -n '1,220p' "$CURRENT_CASE/dashboard-final.txt" >&2 2>/dev/null || true
        sed -n '1,220p' "$CURRENT_CASE/terminal-final.txt" >&2 2>/dev/null || true
        sed -n '1,220p' "$CURRENT_CASE/loop.log" >&2 2>/dev/null || true
    fi
    return 1
}

stop_sampler() {
    if [[ -n "$SAMPLER_PID" ]]; then
        kill "$SAMPLER_PID" >/dev/null 2>&1 || true
        wait "$SAMPLER_PID" >/dev/null 2>&1 || true
        SAMPLER_PID=
    fi
}

stop_case() {
    stop_sampler
    if [[ -n "$CURRENT_SESSION" && -n "$REAL_ZELLIJ" ]]; then
        "$REAL_ZELLIJ" kill-session "$CURRENT_SESSION" >/dev/null 2>&1 || true
    fi
    if [[ -n "$LOOP_PID" ]]; then
        kill "$LOOP_PID" >/dev/null 2>&1 || true
        wait "$LOOP_PID" >/dev/null 2>&1 || true
        LOOP_PID=
    fi
    CURRENT_SESSION=
    CURRENT_PLUGIN_PANE=
    CURRENT_AGENT_PANE=
}

cleanup() {
    local status=$?
    stop_case
    if [[ "$KEEP_LIVE_FIXTURES" != 1 && -d "$EVIDENCE_DIR/fixtures" ]]; then
        rm -rf "$EVIDENCE_DIR/fixtures"
    fi
    if (( status != 0 )); then
        echo "live startup harness failed; retained evidence at $EVIDENCE_DIR" >&2
    fi
}
trap cleanup EXIT INT TERM

for dependency in bash cargo git jq just rustc script shasum zellij zsh codex claude; do
    command -v "$dependency" >/dev/null 2>&1 || fail "missing required command: $dependency"
done
[[ "$SKIP_BUILD" =~ ^[01]$ ]] || fail "SKIP_BUILD must be 0 or 1"
[[ "$SKIP_DETERMINISTIC_PREFLIGHT" =~ ^[01]$ ]] || fail "SKIP_DETERMINISTIC_PREFLIGHT must be 0 or 1"
[[ "$PREPARE_ONLY" =~ ^[01]$ ]] || fail "PREPARE_ONLY must be 0 or 1"
[[ "$KEEP_LIVE_FIXTURES" =~ ^[01]$ ]] || fail "KEEP_LIVE_FIXTURES must be 0 or 1"
[[ "$LIVE_STARTUP_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] || fail "LIVE_STARTUP_TIMEOUT_SECS must be positive"
if [[ "$SKIP_BUILD" == 1 && -z "$LISA_BIN" ]]; then
    fail "SKIP_BUILD=1 requires an explicit LISA_BIN"
fi

mkdir -p "$EVIDENCE_DIR/build" "$EVIDENCE_DIR/fixtures"
EVIDENCE_DIR=$(cd "$EVIDENCE_DIR" && pwd -P)

record_versions() {
    {
        printf 'timestamp_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        printf 'source_head=%s\n' "$(git -C "$REPO_ROOT" rev-parse HEAD)"
        printf 'source_root=%s\n' "$REPO_ROOT"
        printf 'rustc=%s\n' "$(rustc --version)"
        printf 'cargo=%s\n' "$(cargo --version)"
        printf 'zellij=%s\n' "$(zellij --version)"
        printf 'codex=%s\n' "$(codex --version)"
        printf 'claude=%s\n' "$(claude --version)"
    } > "$EVIDENCE_DIR/build/versions.txt"
}

build_fresh_lisa() {
    if [[ "$SKIP_BUILD" != 1 ]]; then
        (cd "$REPO_ROOT" && just build-cli) 2>&1 | tee "$EVIDENCE_DIR/build/build.log"
        LISA_BIN="$REPO_ROOT/target/release/lisa"
    fi
    [[ -x "$LISA_BIN" ]] || fail "LISA_BIN is not executable: $LISA_BIN"
    LISA_BIN=$(cd "$(dirname "$LISA_BIN")" && pwd -P)/$(basename "$LISA_BIN")
    local target_wasm="$REPO_ROOT/target/wasm32-wasip1/release/lisa.wasm"
    [[ -f "$target_wasm" ]] || fail "release WASM is missing: $target_wasm"
    {
        printf 'lisa_bin=%s\n' "$LISA_BIN"
        printf 'lisa_version=%s\n' "$("$LISA_BIN" --version)"
        printf 'lisa_sha256=%s\n' "$(shasum -a 256 "$LISA_BIN" | awk '{print $1}')"
        printf 'target_wasm=%s\n' "$target_wasm"
        printf 'target_wasm_sha256=%s\n' "$(shasum -a 256 "$target_wasm" | awk '{print $1}')"
    } > "$EVIDENCE_DIR/build/artifacts.txt"
}

run_deterministic_preflight() {
    if [[ "$SKIP_DETERMINISTIC_PREFLIGHT" == 1 ]]; then
        printf 'SKIPPED by SKIP_DETERMINISTIC_PREFLIGHT=1\n' > "$EVIDENCE_DIR/build/deterministic-preflight.txt"
        return
    fi
    (cd "$REPO_ROOT" && cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture) \
        2>&1 | tee "$EVIDENCE_DIR/build/deterministic-preflight.txt"
    # The Rust integration wrapper independently asserts the inner shell
    # harness's `real-zellij-delivery-boundary: PASS` receipt. Cargo captures
    # that stdout on success, so the outer live harness observes the Rust test's
    # stable `... ok` line instead.
    grep -Fq 'test real_zellij_delivery_boundary ... ok' "$EVIDENCE_DIR/build/deterministic-preflight.txt" \
        || fail "deterministic real-Zellij preflight lacked its passing test receipt"
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

session_action() {
    "$REAL_ZELLIJ" --session "$CURRENT_SESSION" action "$@"
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
        '[.[] | select(.is_plugin == false and ((.title // "") | contains("T-LIVE-")))][0] | if . then "terminal_\(.id)" else empty end' \
        <<<"$panes")
    [[ -n "$CURRENT_PLUGIN_PANE" && -n "$CURRENT_AGENT_PANE" ]]
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

write_zellij_wrapper() {
    local root=$1
    mkdir -p "$root/bin"
    cat > "$root/bin/zellij" <<'WRAPPER'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
    exec "$LISA_LIVE_REAL_ZELLIJ" "$@"
fi
if [[ "${1:-}" == "--layout" && $# == 2 ]]; then
    exec "$LISA_LIVE_REAL_ZELLIJ" --session "$LISA_LIVE_SESSION" \
        --new-session-with-layout "$2"
fi
echo "unexpected zellij invocation: $*" >&2
exit 2
WRAPPER
    chmod +x "$root/bin/zellij"
}

create_fixture() {
    local provider=$1
    local ticket_id=$2
    local root="$EVIDENCE_DIR/fixtures/$provider-first"
    rm -rf "$root"
    mkdir -p "$root"
    "$LISA_BIN" init --path "$root" > "$EVIDENCE_DIR/$provider-first/init.log"
    mkdir -p "$root/docs/active/tickets" "$root/docs/active/stories"
    cat > "$root/.lisa.toml" <<'TOML'
version = "0.4.0"

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 1
auto_advance = true
review_timeout_secs = 30
session_timeout_secs = 1800
wind_down_secs = 5
assignment_ack_timeout_secs = 30

[agent]
client = "claude"
TOML
    cat > "$root/docs/active/stories/S-LIVE.md" <<'STORY'
---
id: S-LIVE
title: live-first-assignment-proof
status: active
---

# Live first-assignment proof
STORY
    cat > "$root/docs/active/tickets/$ticket_id.md" <<TICKET
---
id: $ticket_id
story: S-LIVE
title: ${provider}-first-assignment-proof
type: task
status: open
priority: high
agent: $provider
phase: research
depends_on: []
---

## Context

This is an isolated live harness ticket. Produce concise RDSPI phase artifacts only.
Do not modify product source. Continue through Review without asking questions, then stop
and let Lisa publish completion.

## Acceptance Criteria

- [ ] All six concise phase artifacts identify $provider as the assigned provider and
      confirm no product source change was required.
TICKET
    write_zellij_wrapper "$root"
    (
        cd "$root"
        git init -q
        git config user.name "Lisa Live Startup Harness"
        git config user.email "lisa-live-startup@example.invalid"
        git add .
        git commit -qm "live startup fixture baseline"
    )
    "$LISA_BIN" validate --path "$root" > "$EVIDENCE_DIR/$provider-first/validate.log"
    printf '%s\n' "$root"
}

start_loop() {
    local provider=$1
    local root=$2
    local runner="$CURRENT_CASE/run-loop.sh"
    cat > "$runner" <<RUNNER
#!/usr/bin/env bash
stty rows 50 cols 140 2>/dev/null || true
exec env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME \\
  PATH=$(printf '%q' "$root/bin:$PATH") \\
  LISA_LIVE_REAL_ZELLIJ=$(printf '%q' "$REAL_ZELLIJ") \\
  LISA_LIVE_SESSION=$(printf '%q' "$CURRENT_SESSION") \\
  $(printf '%q' "$LISA_BIN") loop --path $(printf '%q' "$root") --client $(printf '%q' "$provider")
RUNNER
    chmod +x "$runner"
    if script --version 2>&1 | grep -qi util-linux; then
        script -q -c "$runner" "$CURRENT_CASE/loop.log" >/dev/null 2>&1 &
    else
        script -q "$CURRENT_CASE/loop.log" "$runner" >/dev/null 2>&1 &
    fi
    LOOP_PID=$!
    wait_until 40 "named Zellij session $CURRENT_SESSION" session_is_ready
    wait_until 30 "Lisa plugin and ticket pane" discover_panes
}

copy_signal_once() {
    local suffix=$1
    local destination=$2
    [[ -f "$destination" ]] && return 0
    local source
    source=$(find "$CURRENT_ROOT/.lisa/signals" -maxdepth 1 -name "pane-*.$suffix" -type f -print -quit 2>/dev/null || true)
    if [[ -n "$source" ]]; then
        cp "$source" "$destination"
        printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)" "$suffix-signal" >> "$CURRENT_CASE/signal-events.tsv"
    fi
}

record_state_once() {
    local state=$1
    local dashboard=$2
    if grep -Fq "$state" <<<"$dashboard" && ! awk -F '\t' -v value="$state" '$2 == value { found=1 } END { exit !found }' "$CURRENT_CASE/state-events.tsv"; then
        printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)" "$state" >> "$CURRENT_CASE/state-events.tsv"
    fi
}

sample_once() {
    discover_panes || return 0
    local dashboard terminal timestamp
    dashboard=$(dump_pane "$CURRENT_PLUGIN_PANE" 2>/dev/null || true)
    terminal=$(dump_pane "$CURRENT_AGENT_PANE" 2>/dev/null || true)
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)
    {
        printf '\n===== %s =====\n' "$timestamp"
        printf '%s\n' "$dashboard"
    } >> "$CURRENT_CASE/dashboard-snapshots.txt"
    {
        printf '\n===== %s =====\n' "$timestamp"
        printf '%s\n' "$terminal"
    } >> "$CURRENT_CASE/terminal-snapshots.txt"
    record_state_once starting "$dashboard"
    record_state_once ready-for-assignment "$dashboard"
    record_state_once delivering "$dashboard"
    record_state_once owned "$dashboard"
    copy_signal_once lease "$CURRENT_CASE/lease.json"
    copy_signal_once started "$CURRENT_CASE/started.json"
    copy_signal_once ack "$CURRENT_CASE/ack.json"
}

start_sampler() {
    : > "$CURRENT_CASE/state-events.tsv"
    : > "$CURRENT_CASE/signal-events.tsv"
    : > "$CURRENT_CASE/dashboard-snapshots.txt"
    : > "$CURRENT_CASE/terminal-snapshots.txt"
    (
        while :; do
            sample_once
            sleep 0.25
        done
    ) &
    SAMPLER_PID=$!
}

ticket_is_done() {
    local ticket=$1
    grep -Fq 'status: done' "$ticket" && grep -Fq 'phase: done' "$ticket"
}

verify_build_identity() {
    local layout="$CURRENT_ROOT/.lisa-layout.kdl"
    [[ -f "$layout" ]] || fail "generated layout is missing"
    cp "$layout" "$CURRENT_CASE/layout.kdl"
    local extracted_wasm target_hash extracted_hash
    extracted_wasm=$(sed -n 's/.*plugin location="file:\/\/\([^\"]*lisa-plugin-[^\"]*\.wasm\)".*/\1/p' "$layout" | head -1)
    [[ -f "$extracted_wasm" ]] || fail "layout-extracted WASM is missing: $extracted_wasm"
    target_hash=$(awk -F= '$1 == "target_wasm_sha256" { print $2 }' "$EVIDENCE_DIR/build/artifacts.txt")
    extracted_hash=$(shasum -a 256 "$extracted_wasm" | awk '{print $1}')
    [[ "$target_hash" == "$extracted_hash" ]] || fail "embedded/extracted WASM hash mismatch"
    {
        printf 'layout_lisa_bin=%s\n' "$(sed -n 's/.*lisa_bin "\([^"]*\)".*/\1/p' "$layout")"
        printf 'extracted_wasm=%s\n' "$extracted_wasm"
        printf 'extracted_wasm_sha256=%s\n' "$extracted_hash"
        printf 'target_wasm_sha256=%s\n' "$target_hash"
        printf 'hash_match=PASS\n'
    } > "$CURRENT_CASE/build-identity.txt"
    grep -Fq "lisa_bin \"$LISA_BIN\"" "$layout" || fail "layout did not name the fresh Lisa executable"
}

verify_codex_trust() {
    local canonical_root config header
    canonical_root=$(cd "$CURRENT_ROOT" && pwd -P)
    if [[ -n "${CODEX_HOME:-}" ]]; then
        config="$CODEX_HOME/config.toml"
    else
        config="$HOME/.codex/config.toml"
    fi
    header="[projects.\"$canonical_root\"]"
    [[ -f "$config" ]] || fail "Codex config was not found at $config"
    awk -v header="$header" '
        $0 == header { print; getline; print; found=1 }
        END { exit !found }
    ' "$config" > "$CURRENT_CASE/codex-trust.txt" || fail "canonical Codex trust table is absent"
    grep -Fq 'trust_level = "trusted"' "$CURRENT_CASE/codex-trust.txt" \
        || fail "canonical Codex trust table is not trusted"
    printf 'canonical_root=%s\nconfig=%s\n' "$canonical_root" "$config" \
        >> "$CURRENT_CASE/codex-trust.txt"
}

verify_launch_contract() {
    local provider=$1
    local ticket_id=$2
    local scripts=() assignments=()
    while IFS= read -r path; do scripts+=("$path"); done < <(find "$CURRENT_ROOT/.lisa/attempts/$ticket_id" -name '.lisa-launch-*.sh' -type f | sort)
    while IFS= read -r path; do assignments+=("$path"); done < <(find "$CURRENT_ROOT/.lisa/attempts/$ticket_id" -name assignment.md -type f | sort)
    (( ${#scripts[@]} == 1 )) || fail "expected one launch script for $ticket_id, found ${#scripts[@]}"
    (( ${#assignments[@]} == 1 )) || fail "expected one assignment file for $ticket_id, found ${#assignments[@]}"
    grep -Fq "LISA_TICKET_ID='$ticket_id'" "${scripts[0]}" || grep -Fq "LISA_TICKET_ID=$ticket_id" "${scripts[0]}" \
        || fail "launch script lacks ticket lifecycle identity"
    grep -Fq "$provider" "${scripts[0]}" || fail "launch script lacks bare $provider command"
    if grep -Eq 'LISA_ASSIGNMENT|assignment\.md|Read and follow the complete assignment|This is an isolated live harness ticket' "${scripts[0]}"; then
        fail "launch script contains assignment payload or reference"
    fi
    grep -Fq "$ticket_id" "${assignments[0]}" || fail "assignment file lacks ticket identity"
    grep -Fq 'work through ALL remaining phases' "${assignments[0]}" || fail "assignment file lacks full workflow instruction"
    {
        printf 'launch_script=%s\n' "${scripts[0]#$CURRENT_ROOT/}"
        printf 'assignment_file=%s\n' "${assignments[0]#$CURRENT_ROOT/}"
        printf 'provider=%s\n' "$provider"
        printf 'inline_assignment_absent=PASS\n'
        printf 'separate_assignment_present=PASS\n'
    } > "$CURRENT_CASE/launch-contract.txt"
}

verify_state_order() {
    local previous=0 state line
    for state in starting ready-for-assignment delivering owned; do
        line=$(awk -F '\t' -v value="$state" '$2 == value { print NR; exit }' "$CURRENT_CASE/state-events.tsv")
        [[ -n "$line" ]] || fail "dashboard never exposed $state"
        (( line > previous )) || fail "state $state was observed out of order"
        previous=$line
    done
    [[ -f "$CURRENT_CASE/started.json" ]] || fail "sampler did not retain process-start evidence"
    [[ -f "$CURRENT_CASE/ack.json" ]] || fail "sampler did not retain prompt acknowledgement evidence"
    jq -e --arg ticket "$2" '.prompt | contains("LISA_ASSIGNMENT") and contains($ticket)' "$CURRENT_CASE/ack.json" >/dev/null \
        || fail "acknowledgement did not carry the matching ticket marker"
    if grep -Eiq 'dquote>|startup-failed|delivery-failed|recovery-failed|do you trust the contents|trust this folder' \
        "$CURRENT_CASE/dashboard-snapshots.txt" "$CURRENT_CASE/terminal-snapshots.txt"; then
        fail "forbidden repair, trust, or failure state appeared"
    fi
    printf 'ordered_states=starting -> ready-for-assignment -> delivering -> owned\nmatching_ack=PASS\n' \
        > "$CURRENT_CASE/state-contract.txt"
}

verify_completion() {
    local provider=$1
    local ticket_id=$2
    local work="$CURRENT_ROOT/docs/active/work/$ticket_id"
    for artifact in research design structure plan progress review; do
        [[ -s "$work/$artifact.md" ]] || fail "published $artifact.md is missing for $ticket_id"
    done
    jq -e --arg ticket "$ticket_id" --arg method "$provider" \
        'select(.ticket_id == $ticket and .outcome == "done" and .authoritative == true and .actual.method == $method)' \
        "$CURRENT_ROOT/.lisa/provenance.jsonl" >/dev/null \
        || fail "authoritative $provider Done provenance is missing for $ticket_id"
    git -C "$CURRENT_ROOT" status --short > "$CURRENT_CASE/fixture-status.txt"
    [[ ! -s "$CURRENT_CASE/fixture-status.txt" ]] || fail "completed fixture repository is not clean"
    git -C "$CURRENT_ROOT" log --oneline --decorate -10 > "$CURRENT_CASE/git-log.txt"
    cp "$CURRENT_ROOT/docs/active/tickets/$ticket_id.md" "$CURRENT_CASE/ticket-final.md"
    cp "$CURRENT_ROOT/.lisa/provenance.jsonl" "$CURRENT_CASE/provenance.jsonl"
    mkdir -p "$CURRENT_CASE/published-work"
    cp "$work"/*.md "$CURRENT_CASE/published-work/"
}

capture_final_screens() {
    discover_panes || return 0
    dump_pane "$CURRENT_PLUGIN_PANE" > "$CURRENT_CASE/dashboard-final.txt" 2>/dev/null || true
    dump_pane "$CURRENT_AGENT_PANE" > "$CURRENT_CASE/terminal-final.txt" 2>/dev/null || true
    session_action list-panes --json --all > "$CURRENT_CASE/panes-final.json" 2>/dev/null || true
}

run_case() {
    local provider=$1
    local ticket_id
    if [[ "$provider" == codex ]]; then
        ticket_id=T-LIVE-CODEX
    else
        ticket_id=T-LIVE-CLAUDE
    fi
    CURRENT_CASE="$EVIDENCE_DIR/$provider-first"
    mkdir -p "$CURRENT_CASE"
    CURRENT_ROOT=$(create_fixture "$provider" "$ticket_id")
    CURRENT_ROOT=$(cd "$CURRENT_ROOT" && pwd -P)
    CURRENT_SESSION="lisa-live-$provider-$$"
    printf 'provider=%s\nticket_id=%s\ncanonical_root=%s\nsession=%s\n' \
        "$provider" "$ticket_id" "$CURRENT_ROOT" "$CURRENT_SESSION" > "$CURRENT_CASE/case.txt"
    echo "$provider-first: launching $CURRENT_SESSION"
    start_loop "$provider" "$CURRENT_ROOT"
    start_sampler
    wait_until 60 "attempt launch files for $ticket_id" test -d "$CURRENT_ROOT/.lisa/attempts/$ticket_id"
    verify_build_identity
    if [[ "$provider" == codex ]]; then
        verify_codex_trust
    else
        printf 'not-applicable: Claude does not use Codex project trust\n' > "$CURRENT_CASE/codex-trust.txt"
    fi
    wait_until "$LIVE_STARTUP_TIMEOUT_SECS" "$ticket_id completion" ticket_is_done "$CURRENT_ROOT/docs/active/tickets/$ticket_id.md"
    sleep 1
    capture_final_screens
    stop_sampler
    verify_launch_contract "$provider" "$ticket_id"
    verify_state_order "$provider" "$ticket_id"
    verify_completion "$provider" "$ticket_id"
    stop_case
    printf '%s-first: PASS\n' "$provider" | tee "$CURRENT_CASE/result.txt"
}

record_versions
build_fresh_lisa
run_deterministic_preflight

if [[ "$PREPARE_ONLY" == 1 ]]; then
    echo "fresh-loop-live-startup: PREPARED"
    exit 0
fi

run_case codex
run_case claude

echo "fresh-loop-live-startup: PASS"
