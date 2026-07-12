# T-029-03 · Progress — implementation log

Single production file touched: `crates/lisa-cli/src/agent_exec.rs`. Plan followed
as written; two small deviations noted below.

## Completed

- **Step 1 — resume-branch argv (done).** `build_codex_argv` now branches the tail on
  `args.resume`. Resume arm emits `resume <id|--last>` + `--json` only. Fresh arm emits
  `--json --skip-git-repo-check -C <cwd>` + sandbox flag, byte-identical to before. Comment
  on the resume arm cites codex 0.144.1 + the re-smoke rule.
- **Step 2 — null stdin (done).** Added `.stdin(Stdio::null())` to the `Command` builder in
  `run_agent_exec`, with a comment explaining the non-TTY-pipe hang it prevents.
- **Step 3 — resume-shape tests (done).** Added four pure unit tests + a `resume_forbidden_flags()`
  helper:
  - `argv_resume_omits_cwd_and_sandbox_flags` (default)
  - `argv_resume_bypass_omits_all_sandbox_and_cwd_flags` (bypass; also asserts no `-a never`)
  - `argv_resume_last_omits_cwd_and_sandbox_flags` (`--last` fallback)
  - `argv_resume_passes_extra_codex_args` (passthrough survives on resume)
- **Step 4 — verification gate (done, one caveat):**
  - `cargo test -p lisa-cli` → **255 passed, 0 failed** (26 in `agent_exec`, incl. all 8 argv tests).
  - `cargo test --workspace` → **255 + 145 + 234 = all passed, 0 failed**.
  - `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → **Finished, clean**.
  - `cargo clippy --workspace --all-targets -- -D warnings` → **fails on 15 PRE-EXISTING**
    findings in `crates/lisa-core/src/dag.rs` + `init.rs` (`unnecessary use of to_string`),
    all in unrelated test code. Proven pre-existing: stashing my `agent_exec.rs` change and
    re-running clippy still reports the same 15 errors. **`agent_exec.rs` itself is
    clippy-clean** (0 findings). See Deviation 2.

## Deviations from plan

1. **Design open question resolved by a live help-probe, not a full resume run.** I ran
   `codex exec resume --help` on codex 0.144.1 (on PATH here). It proved `-C`/`--cd` and
   `-s`/`--sandbox` are absent (⇒ rejected) while `--json`, `--skip-git-repo-check`, and
   `--dangerously-bypass-approvals-and-sandbox` are present (⇒ accepted). This sharpened the
   ticket's claim: 0.144.1 rejects only `-C`/`-s` on resume, not `--skip-git-repo-check` —
   clap aborts on the first unexpected arg (`-C`), so the T-021-01 Q5 probe never reached the
   git flag. The fix still drops `--skip-git-repo-check` on resume (AC-1 + it is redundant and
   a future-drift liability), per Design Option B.
2. **Clippy gate is red on pre-existing, out-of-scope debt.** The 15 failures live in `dag.rs`
   / `init.rs`, not the file this ticket changes. Fixing them would widen the diff into an
   unrelated subsystem, against the ticket's "do not widen scope" guardrail. Left untouched
   and flagged in `review.md` as the one open concern a human must adjudicate. My change adds
   zero new clippy findings.

## Not done (documented, non-gating)

- **Step 5 — live resume smoke** (`agent-exec --resume` end-to-end exit-0 on a persisted
  thread). Deferred: requires a real persisted codex session + authenticated codex turn
  (network + tokens) that this environment isn't set up to run non-interactively. The argv
  shape — the actual bug — is fully covered by the pure unit tests and the live `--help`
  flag-surface probe. Re-run the two-turn smoke in Plan Step 5 after the next `codex update`.

## Commit status

Change is complete and green (except the pre-existing clippy debt above) but **not
committed** — no commit was requested, and Lisa drives commit serialization on this workflow.
Ready to commit as a single atomic change: `fix: omit cwd/sandbox flags on codex exec resume argv`.
