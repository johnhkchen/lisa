#!/usr/bin/env bash
# Live, metered reproduction of the Codex Review-recovery delivery boundary.
# This launches authenticated Codex twice. See
# docs/knowledge/live-codex-review-boundary.md before running it.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd "$SCRIPT_DIR/../../../.." && pwd -P)
REAL_ZELLIJ=$(command -v zellij || true)
EVIDENCE_DIR=${EVIDENCE_DIR:-${TMPDIR:-/tmp}/lisa-live-codex-review-$(date -u +%Y%m%dT%H%M%SZ)}
LEGACY_LISA_BIN=${LEGACY_LISA_BIN:-$(command -v lisa || true)}
CURRENT_LISA_BIN=${CURRENT_LISA_BIN:-}
SKIP_BUILD=${SKIP_BUILD:-0}
PREPARE_ONLY=${PREPARE_ONLY:-0}
KEEP_FIELD_FIXTURES=${KEEP_FIELD_FIXTURES:-1}
FIELD_TIMEOUT_SECS=${FIELD_TIMEOUT_SECS:-1200}
LEGACY_DELAY_SECS=${LEGACY_DELAY_SECS:-35}
CURRENT_CLAIM_DELAY_SECS=${CURRENT_CLAIM_DELAY_SECS:-1}
FIXTURE_PARENT=${LISA_FIELD_FIXTURE_PARENT:-${TMPDIR:-/tmp}}
TICKET_ID=T-FIELD-REVIEW

CURRENT_CASE=
CURRENT_KIND=
CURRENT_ROOT=
CURRENT_SESSION=
CURRENT_PLUGIN_PANE=
CURRENT_AGENT_PANE=
CURRENT_CODEX_HOME=
LOOP_PID=
SAMPLER_PID=

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

fail() {
    local message=$1
    echo "FAIL: $message" >&2
    if [[ -n "$CURRENT_CASE" && -d "$CURRENT_CASE" ]]; then
        echo "evidence: $CURRENT_CASE" >&2
        sed -n '1,200p' "$CURRENT_CASE/state-events.tsv" >&2 2>/dev/null || true
        sed -n '1,200p' "$CURRENT_CASE/signal-events.tsv" >&2 2>/dev/null || true
        sed -n '1,160p' "$CURRENT_CASE/dashboard-final.txt" >&2 2>/dev/null || true
        sed -n '1,160p' "$CURRENT_CASE/terminal-final.txt" >&2 2>/dev/null || true
        tail -160 "$CURRENT_CASE/process-snapshots.txt" >&2 2>/dev/null || true
        tail -200 "$CURRENT_CASE/loop.log" >&2 2>/dev/null || true
    fi
    return 1
}

cleanup() {
    local status=$?
    stop_case
    if [[ -f "$EVIDENCE_DIR/codex-homes.txt" ]]; then
        while IFS= read -r home; do
            [[ -n "$home" ]] && rm -rf "$home"
        done < "$EVIDENCE_DIR/codex-homes.txt"
    fi
    if [[ "$KEEP_FIELD_FIXTURES" != 1 && $status -eq 0 && -f "$EVIDENCE_DIR/fixture-roots.txt" ]]; then
        while IFS= read -r root; do
            [[ -n "$root" ]] && rm -rf "$root"
        done < "$EVIDENCE_DIR/fixture-roots.txt"
    fi
    if (( status != 0 )); then
        echo "live Codex Review harness failed; retained evidence at $EVIDENCE_DIR" >&2
    fi
}
trap cleanup EXIT INT TERM

for dependency in bash cargo codex git jq just ps script sed shasum zellij zsh; do
    command -v "$dependency" >/dev/null 2>&1 || fail "missing required command: $dependency"
done
[[ "$SKIP_BUILD" =~ ^[01]$ ]] || fail "SKIP_BUILD must be 0 or 1"
[[ "$PREPARE_ONLY" =~ ^[01]$ ]] || fail "PREPARE_ONLY must be 0 or 1"
[[ "$KEEP_FIELD_FIXTURES" =~ ^[01]$ ]] || fail "KEEP_FIELD_FIXTURES must be 0 or 1"
[[ "$FIELD_TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] || fail "FIELD_TIMEOUT_SECS must be positive"
[[ "$LEGACY_DELAY_SECS" =~ ^[1-9][0-9]*$ ]] || fail "LEGACY_DELAY_SECS must be positive"
[[ "$CURRENT_CLAIM_DELAY_SECS" =~ ^[1-9][0-9]*$ ]] || fail "CURRENT_CLAIM_DELAY_SECS must be positive"
[[ -x "$LEGACY_LISA_BIN" ]] || fail "LEGACY_LISA_BIN is not executable: $LEGACY_LISA_BIN"
if [[ "$SKIP_BUILD" == 1 && -z "$CURRENT_LISA_BIN" ]]; then
    fail "SKIP_BUILD=1 requires CURRENT_LISA_BIN"
fi

SOURCE_CODEX_HOME=${CODEX_HOME:-$HOME/.codex}
[[ -f "$SOURCE_CODEX_HOME/auth.json" ]] \
    || fail "Codex authentication file is missing from $SOURCE_CODEX_HOME"

mkdir -p "$EVIDENCE_DIR" "$FIXTURE_PARENT"
EVIDENCE_DIR=$(cd "$EVIDENCE_DIR" && pwd -P)
FIXTURE_PARENT=$(cd "$FIXTURE_PARENT" && pwd -P)
: > "$EVIDENCE_DIR/fixture-roots.txt"
: > "$EVIDENCE_DIR/codex-homes.txt"

canonical_executable() {
    local executable=$1
    local directory
    directory=$(cd "$(dirname "$executable")" && pwd -P)
    printf '%s/%s\n' "$directory" "$(basename "$executable")"
}

record_versions() {
    {
        printf 'timestamp_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        printf 'source_head=%s\n' "$(git -C "$REPO_ROOT" rev-parse HEAD)"
        printf 'source_root=%s\n' "$REPO_ROOT"
        printf 'cargo=%s\n' "$(cargo --version)"
        printf 'codex=%s\n' "$(codex --version)"
        printf 'zellij=%s\n' "$(zellij --version)"
    } > "$EVIDENCE_DIR/versions.txt"
}

build_current_lisa() {
    LEGACY_LISA_BIN=$(canonical_executable "$LEGACY_LISA_BIN")
    if [[ "$SKIP_BUILD" != 1 ]]; then
        (cd "$REPO_ROOT" && just build-cli) 2>&1 | tee "$EVIDENCE_DIR/build.log"
        CURRENT_LISA_BIN="$REPO_ROOT/target/release/lisa"
    fi
    [[ -x "$CURRENT_LISA_BIN" ]] || fail "CURRENT_LISA_BIN is not executable: $CURRENT_LISA_BIN"
    CURRENT_LISA_BIN=$(canonical_executable "$CURRENT_LISA_BIN")

    local legacy_hash current_hash target_wasm
    legacy_hash=$(shasum -a 256 "$LEGACY_LISA_BIN" | awk '{print $1}')
    current_hash=$(shasum -a 256 "$CURRENT_LISA_BIN" | awk '{print $1}')
    [[ "$legacy_hash" != "$current_hash" ]] \
        || fail "legacy and current Lisa binaries have the same SHA-256"
    target_wasm="$REPO_ROOT/target/wasm32-wasip1/release/lisa.wasm"
    [[ -f "$target_wasm" ]] || fail "current release WASM is missing: $target_wasm"
    {
        printf 'legacy_lisa_bin=%s\n' "$LEGACY_LISA_BIN"
        printf 'legacy_lisa_version=%s\n' "$("$LEGACY_LISA_BIN" --version)"
        printf 'legacy_lisa_sha256=%s\n' "$legacy_hash"
        printf 'current_lisa_bin=%s\n' "$CURRENT_LISA_BIN"
        printf 'current_lisa_version=%s\n' "$("$CURRENT_LISA_BIN" --version)"
        printf 'current_lisa_sha256=%s\n' "$current_hash"
        printf 'target_wasm=%s\n' "$target_wasm"
        printf 'target_wasm_sha256=%s\n' "$(shasum -a 256 "$target_wasm" | awk '{print $1}')"
        printf 'binary_hashes_differ=PASS\n'
    } > "$EVIDENCE_DIR/binary-identity.txt"
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
    panes=$(session_action list-panes --json --all 2>/dev/null) || return 1
    jq -e 'type == "array"' <<<"$panes" >/dev/null 2>&1 || return 1
    CURRENT_PLUGIN_PANE=$(jq -r \
        '[.[] | select(.is_plugin == true and ((.plugin_url // "") | startswith("file:")))][0] | if . then "plugin_\(.id)" else empty end' \
        <<<"$panes")
    CURRENT_AGENT_PANE=$(jq -r --arg ticket "$TICKET_ID" \
        '[.[] | select(.is_plugin == false and ((.title // "") | contains($ticket)))][0] | if . then "terminal_\(.id)" else empty end' \
        <<<"$panes")
    if [[ -z "$CURRENT_AGENT_PANE" ]]; then
        CURRENT_AGENT_PANE=$(jq -r \
            '[.[] | select(.is_plugin == false)][0] | if . then "terminal_\(.id)" else empty end' \
            <<<"$panes")
    fi
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
    exec "$LISA_FIELD_REAL_ZELLIJ" "$@"
fi
if [[ "${1:-}" == "--layout" && $# == 2 ]]; then
    exec "$LISA_FIELD_REAL_ZELLIJ" --session "$LISA_FIELD_SESSION" \
        --new-session-with-layout "$2"
fi
echo "unexpected zellij invocation: $*" >&2
exit 2
WRAPPER
    chmod +x "$root/bin/zellij"
}

write_agents_protocol() {
    local root=$1
    cat > "$root/AGENTS.md" <<'AGENTS'
# Live Codex Review boundary fixture

This is a metered field fixture. Follow the ticket and Lisa's generated assignment exactly.

Your FIRST tool action must be one shell command that runs the following protocol before you read
canonical Review work or create any file. Do not split it into multiple tool calls:

```sh
set -e; work=".lisa/attempts/$LISA_TICKET_ID/$LISA_ATTEMPT_ID/work"; assignment=$(find "$work" -maxdepth 1 -type f -name "assignment-$LISA_ATTEMPT_ID-*.md" -print -quit 2>/dev/null || true); if [ -n "$assignment" ]; then nonce=${assignment##*-}; nonce=${nonce%.md}; while [ ! -f "$work/.field-claim-gate" ]; do sleep 0.25; done; sleep "$LISA_FIELD_CURRENT_CLAIM_DELAY_SECS"; "$LISA_BIN" claim --path . --ticket-id "$LISA_TICKET_ID" --attempt-id "$LISA_ATTEMPT_ID" --nonce "$nonce"; else sleep "$LISA_FIELD_LEGACY_DELAY_SECS"; fi
```

The sleep is intentional harness timing. After the first command returns, inspect the existing
Review evidence, write only the current attempt's `review.md` and `review-disposition.json`, then
remain on this ticket. Do not modify product source, ticket frontmatter, or canonical work directly.
AGENTS
}

create_fixture() {
    local kind=$1
    local lisa_bin=$2
    local root
    root=$(mktemp -d "$FIXTURE_PARENT/lisa-field-$kind.XXXXXX")
    root=$(cd "$root" && pwd -P)
    printf '%s\n' "$root" >> "$EVIDENCE_DIR/fixture-roots.txt"
    "$lisa_bin" init --path "$root" > "$EVIDENCE_DIR/$kind-init.log"
    mkdir -p "$root/docs/active/tickets" "$root/docs/active/stories" \
        "$root/docs/active/work/$TICKET_ID"
    cat > "$root/.lisa.toml" <<'TOML'
version = "0.4.0"

[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 1
auto_advance = true
review_timeout_secs = 300
session_timeout_secs = 1800
wind_down_secs = 5
assignment_ack_timeout_secs = 8

[agent]
client = "codex"
TOML
    cat > "$root/docs/active/stories/S-FIELD.md" <<'STORY'
---
id: S-FIELD
title: live-review-recovery-field-fixture
status: active
---

# Live Review recovery field fixture
STORY
    cat > "$root/docs/active/tickets/$TICKET_ID.md" <<'TICKET'
---
id: T-FIELD-REVIEW
story: S-FIELD
title: recover-existing-review
type: task
status: open
priority: high
agent: codex
phase: review
depends_on: []
---

## Context

This ticket already reached Review in a prior session. Inspect its existing canonical review,
then publish a current-attempt Review disposition. Do not change product source.

## Acceptance Criteria

- [x] Prior work required no product source changes.
TICKET
    cat > "$root/docs/active/work/$TICKET_ID/review.md" <<'REVIEW'
# Prior Review evidence

The earlier attempt completed an artifact-only field note. It changed no product source and its
focused checks passed. Recovery should admit a fresh attempt-private Review and disposition only.
REVIEW
    write_agents_protocol "$root"
    write_zellij_wrapper "$root"
    (
        cd "$root"
        git init -q
        git config user.name "Lisa Live Codex Review Harness"
        git config user.email "lisa-live-review@example.invalid"
        git add .
        git commit -qm "live Review fixture baseline"
    )
    "$lisa_bin" validate --path "$root" > "$EVIDENCE_DIR/$kind-validate.log"
    printf '%s\n' "$root"
}

prepare_codex_home() {
    local kind=$1
    CURRENT_CODEX_HOME=$(mktemp -d "$FIXTURE_PARENT/lisa-field-codex-home-$kind.XXXXXX")
    CURRENT_CODEX_HOME=$(cd "$CURRENT_CODEX_HOME" && pwd -P)
    printf '%s\n' "$CURRENT_CODEX_HOME" >> "$EVIDENCE_DIR/codex-homes.txt"
    ln -s "$SOURCE_CODEX_HOME/auth.json" "$CURRENT_CODEX_HOME/auth.json"
    cat > "$CURRENT_CODEX_HOME/config.toml" <<'CODEX_CONFIG'
[features]
hooks = false
CODEX_CONFIG
    {
        printf 'codex_home=%s\n' "$CURRENT_CODEX_HOME"
        printf 'auth_source=%s\n' "$SOURCE_CODEX_HOME/auth.json"
        printf 'features_hooks=false\n'
        printf 'hooks_json_absent=%s\n' "$([[ ! -e "$CURRENT_CODEX_HOME/hooks.json" ]] && echo PASS || echo FAIL)"
    } > "$CURRENT_CASE/codex-runtime.txt"
}

start_loop() {
    local lisa_bin=$1
    local runner="$CURRENT_CASE/run-loop.sh"
    cat > "$runner" <<RUNNER
#!/usr/bin/env bash
stty rows 50 cols 140 2>/dev/null || true
exec env -u ZELLIJ -u ZELLIJ_PANE_ID -u ZELLIJ_SESSION_NAME \\
  PATH=$(printf '%q' "$CURRENT_ROOT/bin:$PATH") \\
  CODEX_HOME=$(printf '%q' "$CURRENT_CODEX_HOME") \\
  LISA_FIELD_REAL_ZELLIJ=$(printf '%q' "$REAL_ZELLIJ") \\
  LISA_FIELD_SESSION=$(printf '%q' "$CURRENT_SESSION") \\
  LISA_FIELD_LEGACY_DELAY_SECS=$(printf '%q' "$LEGACY_DELAY_SECS") \\
  LISA_FIELD_CURRENT_CLAIM_DELAY_SECS=$(printf '%q' "$CURRENT_CLAIM_DELAY_SECS") \\
  $(printf '%q' "$lisa_bin") loop --path $(printf '%q' "$CURRENT_ROOT") --client codex
RUNNER
    chmod +x "$runner"
    if script --version 2>&1 | grep -qi util-linux; then
        script -q -c "$runner" "$CURRENT_CASE/loop.log" >/dev/null 2>&1 &
    else
        script -q "$CURRENT_CASE/loop.log" "$runner" >/dev/null 2>&1 &
    fi
    LOOP_PID=$!
    wait_until 40 "named Zellij session $CURRENT_SESSION" session_is_ready
    wait_until 30 "Lisa plugin and Codex pane" discover_panes
}

record_state_once() {
    local state=$1
    local dashboard=$2
    local seen="$CURRENT_CASE/.state-$state"
    if grep -Fqi "$state" <<<"$dashboard" && [[ ! -e "$seen" ]]; then
        : > "$seen"
        printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$state" \
            >> "$CURRENT_CASE/state-events.tsv"
        if [[ "$CURRENT_KIND" == current && "$state" == delivered-awaiting-claim ]]; then
            date -u +%Y-%m-%dT%H:%M:%SZ \
                > "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID/1/work/.field-claim-gate"
        fi
    fi
}

sample_signals() {
    local signal_dir="$CURRENT_ROOT/.lisa/signals"
    [[ -d "$signal_dir" ]] || return 0
    local source base digest size key sequence destination
    while IFS= read -r source; do
        base=$(basename "$source")
        digest=$(shasum -a 256 "$source" 2>/dev/null | awk '{print $1}') || continue
        [[ -n "$digest" ]] || continue
        key="$base:$digest"
        grep -Fqx "$key" "$CURRENT_CASE/.seen-signals" 2>/dev/null && continue
        printf '%s\n' "$key" >> "$CURRENT_CASE/.seen-signals"
        sequence=$(wc -l < "$CURRENT_CASE/.seen-signals" | tr -d ' ')
        destination="$CURRENT_CASE/captured-signals/$(printf '%04d' "$sequence")-$base"
        cp "$source" "$destination" 2>/dev/null || continue
        size=$(wc -c < "$destination" | tr -d ' ')
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$sequence" "$base" "$size" \
            "$digest" "${destination#"$CURRENT_CASE"/}" >> "$CURRENT_CASE/signal-events.tsv"
    done < <(find "$signal_dir" -maxdepth 1 -type f -print 2>/dev/null | sort)
}

sample_once() {
    discover_panes || return 0
    local dashboard terminal timestamp
    dashboard=$(dump_pane "$CURRENT_PLUGIN_PANE" 2>/dev/null || true)
    terminal=$(dump_pane "$CURRENT_AGENT_PANE" 2>/dev/null || true)
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    {
        printf '\n===== %s =====\n' "$timestamp"
        printf '%s\n' "$dashboard"
    } >> "$CURRENT_CASE/dashboard-snapshots.txt"
    {
        printf '\n===== %s =====\n' "$timestamp"
        printf '%s\n' "$terminal"
    } >> "$CURRENT_CASE/terminal-snapshots.txt"
    record_state_once starting "$dashboard"
    record_state_once delivering "$dashboard"
    record_state_once delivered-awaiting-claim "$dashboard"
    record_state_once owned "$dashboard"
    record_state_once claim-timed-out "$dashboard"
    if grep -Fq "FAILED $TICKET_ID" <<<"$dashboard"; then
        record_state_once "FAILED $TICKET_ID" "$dashboard"
    fi
    {
        printf '\n===== %s =====\n' "$timestamp"
        # shellcheck disable=SC2009 # Retain full command lines as field evidence.
        ps -axo pid=,ppid=,command= \
            | grep -E "($CURRENT_ROOT|launch-codex.*assignment-|[ /]codex .*assignment-)" \
            | grep -v grep || true
    } >> "$CURRENT_CASE/process-snapshots.txt"
    sample_signals
}

start_sampler() {
    mkdir -p "$CURRENT_CASE/captured-signals"
    : > "$CURRENT_CASE/state-events.tsv"
    : > "$CURRENT_CASE/signal-events.tsv"
    : > "$CURRENT_CASE/dashboard-snapshots.txt"
    : > "$CURRENT_CASE/terminal-snapshots.txt"
    : > "$CURRENT_CASE/process-snapshots.txt"
    : > "$CURRENT_CASE/.seen-signals"
    (
        while :; do
            sample_once
            sleep 0.1
        done
    ) &
    SAMPLER_PID=$!
}

state_was_seen() {
    local state=$1
    awk -F '\t' -v state="$state" '$2 == state { found=1 } END { exit !found }' \
        "$CURRENT_CASE/state-events.tsv"
}

claim_was_captured() {
    awk -F '\t' '$3 ~ /\.claim$/ { found=1 } END { exit !found }' \
        "$CURRENT_CASE/signal-events.tsv"
}

ticket_is_done() {
    local ticket="$CURRENT_ROOT/docs/active/tickets/$TICKET_ID.md"
    grep -Fq 'status: done' "$ticket" && grep -Fq 'phase: done' "$ticket"
}

capture_final_screens() {
    discover_panes || return 0
    dump_pane "$CURRENT_PLUGIN_PANE" > "$CURRENT_CASE/dashboard-final.txt" 2>/dev/null || true
    dump_pane "$CURRENT_AGENT_PANE" > "$CURRENT_CASE/terminal-final.txt" 2>/dev/null || true
    session_action list-panes --json --all > "$CURRENT_CASE/panes-final.json" 2>/dev/null || true
}

capture_fixture_evidence() {
    capture_final_screens
    git -C "$CURRENT_ROOT" status --short > "$CURRENT_CASE/fixture-status.txt"
    git -C "$CURRENT_ROOT" log --oneline --decorate -10 > "$CURRENT_CASE/git-log.txt"
    cp "$CURRENT_ROOT/docs/active/tickets/$TICKET_ID.md" "$CURRENT_CASE/ticket-final.md"
    cp "$CURRENT_ROOT/.lisa-layout.kdl" "$CURRENT_CASE/layout.kdl" 2>/dev/null || true
    cp "$CURRENT_ROOT/.lisa/provenance.jsonl" "$CURRENT_CASE/provenance.jsonl" 2>/dev/null || true
    cp "$CURRENT_ROOT/.lisa/completion-journal.jsonl" "$CURRENT_CASE/completion-journal.jsonl" \
        2>/dev/null || true
    mkdir -p "$CURRENT_CASE/attempt-snapshot" "$CURRENT_CASE/work-snapshot"
    cp -R "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID/." "$CURRENT_CASE/attempt-snapshot/" \
        2>/dev/null || true
    cp -R "$CURRENT_ROOT/docs/active/work/$TICKET_ID/." "$CURRENT_CASE/work-snapshot/" \
        2>/dev/null || true
    local extracted_wasm
    extracted_wasm=$(sed -n 's/.*plugin location="file:\/\/\([^\"]*lisa-plugin-[^\"]*\.wasm\)".*/\1/p' \
        "$CURRENT_ROOT/.lisa-layout.kdl" 2>/dev/null | head -1)
    {
        printf 'kind=%s\n' "$CURRENT_KIND"
        printf 'fixture_root=%s\n' "$CURRENT_ROOT"
        printf 'session=%s\n' "$CURRENT_SESSION"
        printf 'extracted_wasm=%s\n' "$extracted_wasm"
        if [[ -f "$extracted_wasm" ]]; then
            printf 'extracted_wasm_sha256=%s\n' "$(shasum -a 256 "$extracted_wasm" | awk '{print $1}')"
        fi
    } > "$CURRENT_CASE/case-identity.txt"
}

prepare_case() {
    local kind=$1
    local lisa_bin=$2
    CURRENT_KIND=$kind
    CURRENT_CASE="$EVIDENCE_DIR/$kind"
    mkdir -p "$CURRENT_CASE"
    CURRENT_ROOT=$(create_fixture "$kind" "$lisa_bin")
    CURRENT_ROOT=$(cd "$CURRENT_ROOT" && pwd -P)
    CURRENT_SESSION="lisa-field-$kind-$$"
    prepare_codex_home "$kind"
    printf 'kind=%s\nticket_id=%s\nfixture_root=%s\nsession=%s\nlisa_bin=%s\n' \
        "$kind" "$TICKET_ID" "$CURRENT_ROOT" "$CURRENT_SESSION" "$lisa_bin" \
        > "$CURRENT_CASE/case.txt"
}

run_legacy_case() {
    prepare_case legacy "$LEGACY_LISA_BIN"
    echo "legacy: launching $CURRENT_SESSION"
    start_loop "$LEGACY_LISA_BIN"
    start_sampler
    wait_until 90 "legacy false delivery failure" state_was_seen "FAILED $TICKET_ID"
    stop_sampler
    capture_fixture_evidence
    local launch_script
    launch_script=$(find "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID" -name '.lisa-launch-*.sh' \
        -type f -print -quit)
    [[ -f "$launch_script" ]] || fail "legacy case did not publish a launch script"
    grep -Fq ' codex --dangerously-bypass-approvals-and-sandbox' "$launch_script" \
        || fail "legacy launch script did not use direct Codex launch"
    if grep -Fq 'launch-codex' "$launch_script"; then
        fail "legacy launch unexpectedly used the current native launcher"
    fi
    if claim_was_captured; then
        fail "legacy case unexpectedly published an assignment claim"
    fi
    if find "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID" -name review.md -type f -print -quit \
        | grep -q .; then
        fail "legacy attempt wrote Review output before false failure was captured"
    fi
    printf 'legacy-false-delivery-failure: OBSERVED\n' | tee "$CURRENT_CASE/result.txt"
    stop_case
}

run_current_case() {
    prepare_case current "$CURRENT_LISA_BIN"
    echo "current: launching $CURRENT_SESSION"
    start_loop "$CURRENT_LISA_BIN"
    start_sampler
    wait_until 90 "current delivered-awaiting-claim state" \
        state_was_seen delivered-awaiting-claim
    wait_until 30 "current exact claim signal capture" claim_was_captured
    wait_until 30 "current claim-owned seat" state_was_seen owned
    wait_until "$FIELD_TIMEOUT_SECS" "current Review ticket completion" ticket_is_done
    sleep 1
    stop_sampler
    capture_fixture_evidence
    if grep -Eiq 'delivery-failed|claim-timed-out|FAILED T-FIELD-REVIEW' \
        "$CURRENT_CASE/dashboard-snapshots.txt"; then
        fail "current path exposed a delivery or claim failure"
    fi
    local launch_scripts=() assignments=()
    while IFS= read -r path; do launch_scripts+=("$path"); done < <(
        find "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID/1/work" -name '.lisa-launch-*.sh' -type f | sort
    )
    while IFS= read -r path; do assignments+=("$path"); done < <(
        find "$CURRENT_ROOT/.lisa/attempts/$TICKET_ID/1/work" -name 'assignment-1-*.md' -type f | sort
    )
    (( ${#launch_scripts[@]} == 1 )) \
        || fail "current case expected one attempt-one launch script, found ${#launch_scripts[@]}"
    (( ${#assignments[@]} == 1 )) \
        || fail "current case expected one nonce assignment, found ${#assignments[@]}"
    grep -Fq ' launch-codex' "${launch_scripts[0]}" \
        || fail "current launch did not invoke launch-codex"
    local pane_assignment_path=${assignments[0]#"$CURRENT_ROOT"/}
    grep -Fq "$pane_assignment_path" "${launch_scripts[0]}" \
        || fail "current launch did not carry the exact assignment path"
    jq -e --arg ticket "$TICKET_ID" \
        'select(.ticket_id == $ticket and .attempt_id == 1 and (.nonce | type == "number"))' \
        "$CURRENT_CASE"/captured-signals/*-pane-*.claim >/dev/null \
        || fail "captured claim did not name the exact ticket and attempt"
    printf 'current-claim-delivery: OBSERVED\n' | tee "$CURRENT_CASE/result.txt"
    stop_case
}

record_versions
build_current_lisa
bash -n "$SCRIPT_DIR/live_codex_review_boundary.sh"

prepare_case legacy "$LEGACY_LISA_BIN"
stop_case
prepare_case current "$CURRENT_LISA_BIN"
stop_case

if [[ "$PREPARE_ONLY" == 1 ]]; then
    echo "live-codex-review-boundary: PREPARED"
    exit 0
fi

run_legacy_case
run_current_case

echo "live-codex-review-boundary: PASS"
