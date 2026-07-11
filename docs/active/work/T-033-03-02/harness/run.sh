#!/usr/bin/env bash
# T-033-03-02 consecutive native reuse proof and report generator.
set -euo pipefail

REPORT=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --report)
      [ "$#" -ge 2 ] || { echo "--report requires a path" >&2; exit 2; }
      REPORT="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
CARGO_BIN="${CARGO:-cargo}"
command -v "$CARGO_BIN" >/dev/null 2>&1 || {
  echo "cargo executable not found: $CARGO_BIN" >&2
  exit 2
}

RAW="$(mktemp "${TMPDIR:-/tmp}/lisa-t0330302.raw.XXXXXX")"
RECORDS="$(mktemp "${TMPDIR:-/tmp}/lisa-t0330302.records.XXXXXX")"
cleanup() {
  rm -f "$RAW" "$RECORDS"
}
trap cleanup EXIT

cd "$ROOT"
set +e
"$CARGO_BIN" test -p lisa-plugin consecutive_reused_panes \
  -- --nocapture --test-threads=1 >"$RAW" 2>&1
status=$?
set -e
if [ "$status" -ne 0 ]; then
  echo "focused consecutive-reuse test failed" >&2
  cat "$RAW" >&2
  exit "$status"
fi

awk '
  index($0, "T0330302|") {
    print substr($0, index($0, "T0330302|"))
  }
' "$RAW" > "$RECORDS"

fail() {
  echo "T-033-03-02 harness validation failed: $1" >&2
  echo "normalized evidence:" >&2
  cat "$RECORDS" >&2
  exit 1
}

count_matching() {
  pattern="$1"
  grep -c "$pattern" "$RECORDS" || true
}

[ "$(count_matching '^T0330302|assignment|provider=codex|')" -eq 10 ] || \
  fail "expected exactly 10 Codex assignment rows"
[ "$(count_matching '^T0330302|assignment|provider=claude|')" -eq 10 ] || \
  fail "expected exactly 10 Claude control rows"
[ "$(count_matching 'provider=codex|.*|outcome=ack-then-owned|')" -eq 9 ] || \
  fail "expected exactly 9 ack-then-owned outcomes"
[ "$(count_matching 'provider=codex|.*|outcome=timeout-then-fallback|')" -eq 1 ] || \
  fail "expected exactly 1 timeout-then-fallback outcome"
[ "$(count_matching 'provider=claude|.*|outcome=clear-then-owned-unchanged|')" -eq 10 ] || \
  fail "Claude control transition changed"
[ "$(count_matching 'silent_stall=false$')" -eq 20 ] || \
  fail "every assignment must explicitly report no silent stall"
[ "$(count_matching 'silent_stall=true')" -eq 0 ] || \
  fail "silent stall observed"
[ "$(count_matching 'outcome=timeout-then-fallback|fallback_launches=1|')" -eq 1 ] || \
  fail "forced timeout must perform exactly one fallback launch"
[ "$(count_matching 'final=owned|silent_stall=false$')" -eq 20 ] || \
  fail "every assignment must terminate owned and non-stalled"

codex_panes="$(awk -F'|' '
  /provider=codex/ {
    for (i = 1; i <= NF; i++) if ($i ~ /^pane=/) print substr($i, 6)
  }
' "$RECORDS" | sort -u | tr '\n' ' ' | sed 's/ $//')"
claude_panes="$(awk -F'|' '
  /provider=claude/ {
    for (i = 1; i <= NF; i++) if ($i ~ /^pane=/) print substr($i, 6)
  }
' "$RECORDS" | sort -u | tr '\n' ' ' | sed 's/ $//')"
[ "$codex_panes" = "10 11" ] || fail "expected Codex panes 10 and 11, got: $codex_panes"
[ "$claude_panes" = "20 21" ] || fail "expected Claude panes 20 and 21, got: $claude_panes"

expected_summary='T0330302|summary|codex=10|ack_then_owned=9|timeout_then_fallback=1|claude=10|silent_stalls=0'
[ "$(count_matching "^${expected_summary}$")" -eq 1 ] || fail "summary row mismatch"
[ "$(wc -l < "$RECORDS" | tr -d ' ')" -eq 21 ] || fail "unexpected extra evidence rows"

if [ -n "$REPORT" ]; then
  case "$REPORT" in
    /*) report_path="$REPORT" ;;
    *) report_path="$PWD/$REPORT" ;;
  esac
  mkdir -p "$(dirname "$report_path")"
  generated_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  revision="$(git rev-parse --short HEAD)"
  rust_version="$(rustc --version)"
  cargo_version="$($CARGO_BIN --version)"

  {
    echo "# T-033-03-02 consecutive reuse run report"
    echo
    echo "**Verdict: PASS.** Ten consecutive Codex reassignments across two reused panes resolved to exactly one allowed outcome each, including one forced lost acknowledgment, with zero silent stalls. Ten equivalent Claude reassignments preserved the existing clear-handshake and immediate-ownership behavior."
    echo
    echo "## Run metadata"
    echo
    echo "| Field | Value |"
    echo "|---|---|"
    echo "| Generated (UTC) | \`$generated_at\` |"
    echo "| Git revision | \`$revision\` |"
    echo "| Rust | \`$rust_version\` |"
    echo "| Cargo | \`$cargo_version\` |"
    echo "| Command | \`docs/active/work/T-033-03-02/harness/run.sh --report docs/active/work/T-033-03-02/run-report.md\` |"
    echo
    echo "## Proof boundary"
    echo
    echo "This is a deterministic native live-style harness. It drives Lisa's real scheduler, adapter reset, assignment-generation, acknowledgment, injected deadline, recovery launch, completion, release, and DAG-recompute paths. It does not launch Zellij or installed Codex/Claude clients, consume tokens, or prove host keystroke and hook-file delivery."
    echo
    echo "## Codex consecutive reassignments"
    echo
    echo "| Seq | Ticket | Pane | Generation | Outcome | Fallback launches | Final | Silent stall |"
    echo "|---:|---|---:|---:|---|---:|---|---|"
    awk -F'|' '
      /T0330302\|assignment\|provider=codex/ {
        for (i = 1; i <= NF; i++) {
          split($i, pair, "="); values[pair[1]] = pair[2]
        }
        printf "| %s | %s | %s | %s | %s | %s | %s | %s |\n", \
          values["sequence"], values["ticket"], values["pane"], \
          values["generation"], values["outcome"], values["fallback_launches"], \
          values["final"], values["silent_stall"]
        delete values
      }
    ' "$RECORDS"
    echo
    echo "## Claude unchanged control"
    echo
    echo "| Seq | Ticket | Pane | Generation | Outcome | Fallback launches | Final | Silent stall |"
    echo "|---:|---|---:|---|---|---:|---|---|"
    awk -F'|' '
      /T0330302\|assignment\|provider=claude/ {
        for (i = 1; i <= NF; i++) {
          split($i, pair, "="); values[pair[1]] = pair[2]
        }
        printf "| %s | %s | %s | %s | %s | %s | %s | %s |\n", \
          values["sequence"], values["ticket"], values["pane"], \
          values["generation"], values["outcome"], values["fallback_launches"], \
          values["final"], values["silent_stall"]
        delete values
      }
    ' "$RECORDS"
    echo
    echo "## Summary"
    echo
    echo "| Measure | Observed | Required |"
    echo "|---|---:|---:|"
    echo "| Codex consecutive reassignments | 10 | at least 10 |"
    echo "| Codex panes reused | 2 (10, 11) | reused panes |"
    echo "| ack-then-owned | 9 | allowed outcome |"
    echo "| timeout-then-fallback | 1 | one forced lost-ack case |"
    echo "| Fresh fallback launches in fault row | 1 | exactly 1 |"
    echo "| Claude control reassignments | 10 | equivalent control |"
    echo "| Claude panes reused | 2 (20, 21) | reused panes |"
    echo "| Silent stalls | 0 | 0 |"
    echo
    echo "The fault row is T-CODEX-06. Its original generation 6 times out, recovery allocates a fenced generation, exactly one fresh launch occurs, and the recovery acknowledgment reaches owned. The subsequent observed original generation is 8, demonstrating that the recovery generation consumed its own identity."
  } > "$report_path"
  echo "report: $report_path"
fi

echo "PASS: 10 Codex reuses (9 ack, 1 fallback), 10 Claude controls, 0 silent stalls"
