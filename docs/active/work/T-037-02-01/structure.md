# Structure — T-037-02-01 fresh-loop-live-provider-parity-rerun

The blueprint: exact file-level changes. No new files, no deletions. Two ticket-owned files
modified; scheduler untouched.

## Files touched

### 1. `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` (modified)

Only `verify_state_order` and the one receipt string it writes change. Everything else
(build, fixtures, sampler, launch/completion verifiers, `run_case`, tail) stays as-is.

**Current `verify_state_order` (lib lines ~463–481):**

```bash
verify_state_order() {
    local previous=0 state line
    for state in starting ready-for-assignment delivering owned; do
        line=$(awk … state-events.tsv)
        [[ -n "$line" ]] || fail "dashboard never exposed $state"
        (( line > previous )) || fail "state $state was observed out of order"
        previous=$line
    done
    [[ -f "$CURRENT_CASE/started.json" ]] || fail "sampler did not retain process-start evidence"
    [[ -f "$CURRENT_CASE/ack.json" ]]     || fail "sampler did not retain prompt acknowledgement evidence"
    jq -e … ack.json  # matching ticket marker
    if grep -Eiq 'dquote>|…trust…' snapshots; then fail …; fi
    printf 'ordered_states=starting -> ready-for-assignment -> delivering -> owned\nmatching_ack=PASS\n' > state-contract.txt
}
```

**New shape (provider-aware).** Signature already receives `$1=provider`, `$2=ticket_id`
(called as `verify_state_order "$provider" "$ticket_id"`). Internal organization:

- Local `provider=$1 ticket_id=$2`.
- Choose the required ordered sequence by provider:
  - `codex`  → `starting delivering owned`
  - `claude` → `starting ready-for-assignment delivering owned`
- Run the existing monotonic-order loop over that provider-specific list (unchanged logic,
  variable list).
- **Codex-only positive check:** assert `ready-for-assignment` was *never* seen —
  `if state_was_seen ready-for-assignment; then fail "grace-mode Codex must not claim ready-for-assignment"; fi`
  (reuses the existing `state_was_seen` helper, lines 396–400).
- **`started.json` requirement becomes Claude-only.** Claude keeps
  `[[ -f started.json ]] || fail …`. Codex does not require it (A2). Record which applies in the
  receipt.
- **Shared tail, unchanged:** `ack.json` presence + `jq` matching-ticket marker; the
  forbidden-screen `grep` (`dquote>`, `startup-failed`, `delivery-failed`, `recovery-failed`,
  trust wording).
- **Receipt line becomes provider-specific:**
  - codex  → `ordered_states=starting -> delivering -> owned` and
    `ready_for_assignment_absent=PASS`
  - claude → `ordered_states=starting -> ready-for-assignment -> delivering -> owned`
  - both   → `matching_ack=PASS`

No other function changes. `run_case` already calls `verify_state_order "$provider" "$ticket_id"`
so no call-site edit is needed.

### 2. `docs/knowledge/fresh-loop-live-startup.md` (modified)

Three prose regions updated to describe two provider paths; no structural reorg.

- **Purpose list (lines ~14–18):** item 2 currently reads "becomes `ready-for-assignment`, not
  `owned`." Reword to note the provider split: Claude reports process start and becomes
  `ready-for-assignment`; Codex has no truthful pre-prompt readiness hook, so after a bounded
  named startup grace it moves `starting → delivering` directly, never claiming
  `ready-for-assignment`, and neither becomes `owned` before the matching acknowledgement.
- **"Expected state order" (lines ~155–169):** replace the single shared block with two:
  - Claude: `starting / ready-for-assignment / delivering / owned`.
  - Codex: `starting / delivering / owned`, with an explicit sentence that `ready-for-assignment`
    must **never** appear because grace paces the first prompt; the grace lives inside `starting`.
  Keep the `.started`/`.ack` sampler note but scope the pre-prompt `.started` readiness gate to
  Claude; state that Codex's ownership evidence is the matching `.ack` alone.
- **One-line cross-reference:** note that this provider-aware order is the E-037 contract landed
  in S-037-01 (grace-mode Codex vs SessionStart-mode Claude), so the two controls are not
  symmetric by design.

## Ordering of changes

1. Edit the harness `verify_state_order` + receipt.
2. Edit the runbook prose to match.
3. `bash -n` + `shellcheck` the harness.
4. Commit both files together via `lisa commit-ticket` (one cohesive unit — the assertion and its
   documentation move together).
5. Free preflight (`PREPARE_ONLY=1`).
6. Metered live run (`both`), monitored.

## Interfaces / contracts held invariant

- Harness env-var surface, evidence layout, exit receipts (`PREPARED`, `PASS`) unchanged.
- `run_case` flow, `create_fixture` shape, signal capture, completion verification unchanged.
- No Rust source, no `.lisa.toml` schema, no scheduler behavior touched.
- Ordinary git index never used for ticket work; both files committed with exact `--include`.

## Risk notes

- If the live Codex control regressed (e.g., grace never fires, or it *does* fake a ready claim),
  the new Codex assertions fail **closed** with a precise message — the intended behavior.
- If `started.json` happens to appear for Codex post-prompt, the Claude-only requirement means it
  is simply not asserted for Codex; harmless.
- The runbook edits are documentation only and cannot affect the pass/fail logic.
