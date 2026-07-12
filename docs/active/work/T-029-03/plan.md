# T-029-03 · Plan — ordered, verifiable steps

Sequenced execution of the Structure blueprint. One production file (`agent_exec.rs`).
Steps 1–3 form one atomic commit; step 4 verifies; step 5 optionally smokes live.

## Step 1 — Branch the resume tail in `build_codex_argv`

**Edit:** `crates/lisa-cli/src/agent_exec.rs`, lines ~488–498.

Replace the unconditional tail (`--json`, `--skip-git-repo-check`, `-C`, `<cwd>`, sandbox
flag) with a branch on `args.resume`:
- **resume arm:** push `--json` only (cwd/git/sandbox inherited from the persisted session).
- **fresh arm:** push `--json`, `--skip-git-repo-check`, `-C`, `<cwd>`, then the sandbox flag
  (`--dangerously-bypass-approvals-and-sandbox` when bypass, else `-s workspace-write`) —
  identical to today.

The `resume`/`--last` positional block (lines 480–486) and the leading `-a never` (474–477)
stay exactly as they are. `codex_args` + `prompt` tail (500–502) stays after the branch.

Add a short comment on the resume arm citing **codex 0.144.1** (`resume` rejects `-C`/`-s`;
cwd + sandbox + git context inherited) and the re-smoke-after-`codex update` rule.

**Verify:** `cargo build -p lisa-cli` compiles.

## Step 2 — Null the child's stdin

**Edit:** same file, `run_agent_exec`, the `Command::new(...)` builder (~line 538).

Insert `.stdin(Stdio::null())` before `.stdout(Stdio::piped())`. `Stdio` already imported.

**Verify:** compiles; no clippy warning about the added call.

## Step 3 — Add resume-shape unit tests

**Edit:** same file, `mod tests`, after `argv_passes_extra_codex_args`.

Add (pure, no spawn):
1. `argv_resume_omits_cwd_and_sandbox_flags` — default mode, thread `th_prev`:
   - present: `resume`, `th_prev`, `--json`, `-a`, `never`; prompt last.
   - absent: `-C`, `--skip-git-repo-check`, `-s`, `workspace-write`,
     `--dangerously-bypass-approvals-and-sandbox`.
2. `argv_resume_bypass_omits_all_sandbox_and_cwd_flags` — `bypass_sandbox=true`, resume:
   - present: `resume`, `--json`; prompt last.
   - absent: `-C`, `--skip-git-repo-check`, `-s`, bypass flag, `-a`/`never`.
3. `argv_resume_last_omits_cwd_and_sandbox_flags` — resume, `resolved_thread=None` ⇒ `--last`;
   same omissions, `--last` present.
4. `argv_resume_passes_extra_codex_args` — resume + `["--model","o4"]`; assert window survives.

Helper: reuse `base_args()`; set `.resume = true` (and `.bypass_sandbox` where needed).

Assertion style: `assert!(!argv.contains(&"-C".to_string()))` etc.; `argv.last()` for prompt;
`argv.windows(2).any(...)` for adjacency where it matters.

**Verify:** `cargo test -p lisa-cli agent_exec` (or `cargo test -p lisa-cli`) — all green,
including the untouched `argv_default_flags` / `argv_bypass_sandbox` (fresh-branch guard).

## Step 4 — Full verification gate

Run in order; each must pass:
1. `cargo test -p lisa-cli` — focused CLI suite (argv + writers + translator).
2. `cargo test --workspace` — whole suite (lisa-core, lisa-plugin, lisa-cli).
3. `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — WASM release build.
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean.
   (`just check` covers WASM check + tests; run the explicit commands for the release build
   + clippy the AC names.)

**Commit** after step 4 passes: `fix: omit cwd/sandbox flags on codex exec resume argv`
(steps 1–3 + green gate in one commit). Body: cite T-029-03, the 0.144.1 exit-2 repro, and
the stdin-null hardening.

## Step 5 — Live smoke (optional; empirical AC-2), documented not gating

On a machine with codex 0.144.1 + a persisted thread:
1. Fresh: `LISA_PANE_ID=99 LISA_TICKET_ID=T-SMOKE lisa agent-exec "say ONE"` → expect
   `pane-99.stopped`, a `.thread` artifact, exit 0.
2. Resume: `LISA_PANE_ID=99 LISA_TICKET_ID=T-SMOKE lisa agent-exec --resume "say TWO"` →
   expect **exit 0** (was exit 2 before the fix), fresh `pane-99.*` signals, thread re-entered.
3. Confirm no `Reading additional input from stdin…` hang (stdin-null working).

Record outcome (pass/fail + codex version) in `progress.md`. If codex isn't on PATH or no
persisted thread exists, note it as deferred — the pure unit tests + help-probe evidence
already cover the argv shape; the live run is confirmation, not a CI gate.

## Testing strategy summary

| Concern | Coverage | Type |
|---------|----------|------|
| Resume omits `-C`/`-s`/`--skip-git-repo-check` (default) | `argv_resume_omits_cwd_and_sandbox_flags` | unit, pure |
| Resume omits all sandbox + cwd (bypass) | `argv_resume_bypass_omits_all_sandbox_and_cwd_flags` | unit, pure |
| Resume `--last` fallback shape | `argv_resume_last_omits_cwd_and_sandbox_flags` | unit, pure |
| Resume passthrough | `argv_resume_passes_extra_codex_args` | unit, pure |
| Fresh branch unchanged | `argv_default_flags`, `argv_bypass_sandbox` (existing) | unit, pure |
| Real resume exits 0 on 0.144.1 | Step 5 live smoke | manual, version-pinned |
| stdin-null (no hang) | Step 5 observation | manual |

## Rollback

Single-commit change to one file. Revert the commit to restore prior behaviour. No schema,
no artifact-format, no cross-crate coupling — rollback is clean.
