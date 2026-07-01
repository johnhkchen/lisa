# T-021-01 Progress — spike harness

The "implementation" of a spike is the harness plus the written verdicts. No
`crates/` code changed. Status of each planned step below.

## Done

- **Step 1 — shared scaffold** (`harness/00-common.sh`): version guard
  `require_codex` (hard-fails if codex absent, warns on version ≠ `rust-v0.142.5`,
  writes `codex-version.txt`), `probe_out` evidence-dir helper, `OUT_DIR`/
  `SANDBOX_HOME`, logging. `bash -n` clean.
- **Step 2 — Q1** (`harness/q1-env-inheritance.sh`): exports `LISA_PANE_ID=7`,
  forces `env | grep LISA_PANE_ID` tool call, extracts child's view to
  `child-saw.txt`.
- **Step 3 — Q2 + fixture** (`harness/q2-json-fidelity.sh`,
  `harness/fixtures/rdspi-ticket-prompt.txt`): runs a real read+write+shell ticket
  through `codex exec --json`, builds `event-histogram.txt` and the
  `anchor-check.txt` cross-check of terminal turn event vs. exit code.
- **Step 4 — Q3** (`harness/q3-inpane-rendering.sh`): stdout→file, stderr→separate
  file; `stderr-analysis.txt` (rich vs spinner) + `granularity.txt` (delta vs
  completed).
- **Step 5 — Q4** (`harness/q4-directory-trust.sh`): three cases A/B/C, each with a
  fresh `CODEX_HOME`; B writes a seeded `trust_level = "trusted"` config.
- **Step 6 — Q5** (`harness/q5-resume-followup.sh`): plant codeword → capture
  `thread_id` → recall via `codex exec resume`.
- **Step 7 — driver + README** (`harness/run-all.sh`, `harness/README.md`):
  graceful no-op when codex is absent.
- **Phase docs**: `research.md`, `design.md` (findings), `structure.md`, `plan.md`.

### Verification performed on this host

- `bash -n` on all six scripts + `00-common.sh` → **all clean** (no syntax errors).
- `bash run-all.sh` with codex absent → prints the install notice, exits 1, no
  probe side effects. **Confirmed graceful no-op.**
- `chmod +x` applied to all `q*.sh` and `run-all.sh`.

## Blocked (the one deviation from a "normal" ticket)

- **Step 8 — empirical run on `rust-v0.142.5`**, and **Step 9 — transcribe
  verdicts to authoritative.** Blocked: **codex is not installed on this host**
  (`which codex` → not found, `~/.codex` absent). A spike is empirical by
  definition, so this is a genuine environment gap, not a shortcut.

## Deviation from plan (documented per RDSPI rule)

The plan assumed a host with codex. It does not have one. Rather than fabricate
event streams, the harness was built to be **runnable and self-verifying**, and the
verdicts in `design.md` are explicitly tagged `[PROVISIONAL]` with the reasoning
that supports each and the exact evidence file that would confirm it. Promotion to
authoritative is a single `bash run-all.sh` on a pinned host — no code changes
needed. This keeps the spike honest (no invented evidence) while still delivering a
go/no-go the epic can act on.

## Files touched

Created (all under `docs/active/work/T-021-01/`):
- `harness/00-common.sh`, `harness/q1-env-inheritance.sh`,
  `harness/q2-json-fidelity.sh`, `harness/q3-inpane-rendering.sh`,
  `harness/q4-directory-trust.sh`, `harness/q5-resume-followup.sh`,
  `harness/run-all.sh`, `harness/README.md`,
  `harness/fixtures/rdspi-ticket-prompt.txt`
- `research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`

Modified/deleted under `crates/`: **none.**
