# T-029-03 · Research — agent-exec resume argv drift

Descriptive map of the code, the flag surface, and the constraints. No solutions here.

## The bug in one sentence

`build_codex_argv` unconditionally appends `-C <cwd>`, `--skip-git-repo-check`, and
sandbox flags (`-s workspace-write` or the bypass flag) to *every* invocation,
including the `codex exec resume` subcommand, which on codex-cli **0.144.1** rejects
all three — so `lisa agent-exec --resume` exits 2 and emits no signals.

## Where the code lives

- `crates/lisa-cli/src/agent_exec.rs` — the entire `agent-exec` subcommand.
  - **Layer A** (`SignalKind`, `StreamEffect`, `Outcome`): pure signal vocabulary.
  - **Layer B** (`Translator`): the tested event-stream translator + anchor rule.
  - **Layer C** (`SignalWriter`, `persist_run_artifacts`, `read_thread_id`): file writers.
  - **Layer D** (`AgentExecArgs`, `build_codex_argv`, `run_agent_exec`): the command shell.
    - `build_codex_argv` (lines 471–503) is **pure and unit-tested** — the primary fix site.
    - `run_agent_exec` (lines 510–614) spawns the child (line 538) and streams stdout.
  - **Layer E** (`#[cfg(test)] mod tests`): observe/finalize, fixtures, writers, argv builder.
- `crates/lisa-cli/src/main.rs` — clap wiring. The `AgentExec` subcommand (lines ~85–168)
  parses `prompt`, `resume`, `codex_bin`, `cwd`, `bypass_sandbox`, `codex_args`, `signal_dir`
  into `AgentExecArgs` and calls `run_agent_exec`. No argv logic here; it all flows through
  `build_codex_argv`. Fix does not touch `main.rs`.

## `build_codex_argv` — current shape

Positional order it emits today (fresh run, default sandbox):

```
-a never  exec  [resume <id|--last>]  --json  --skip-git-repo-check  -C <cwd>
  (-s workspace-write | --dangerously-bypass-approvals-and-sandbox)  [codex_args…]  <prompt>
```

Key structural facts:
- `-a never` is emitted **only** when `!bypass_sandbox`, and leads the argv (top-level
  option; 0.144.x rejects it *after* the `exec` subcommand — this was T-029's prior drift fix).
- The `resume` block (lines 480–486) inserts `resume` + (`<thread_id>` | `--last`) right
  after `exec`.
- The three offending flags (`--skip-git-repo-check`, `-C <cwd>`, and the sandbox flag)
  are appended **unconditionally** at lines 488–498 — they do not branch on `args.resume`.
- `codex_args` (passthrough) and `prompt` always come last.

## The flag surface (from the intel packet + live 0.144.1 run)

`docs/knowledge/codex-client/02-codex-capabilities.md` documents the **fresh** `codex exec`
flag set (`--json`, `-s/--sandbox`, `-C/--cd`, `--skip-git-repo-check`, `--ephemeral`,
`--dangerously-bypass-approvals-and-sandbox`/`--yolo`, etc.) — all valid on the fresh path.

`codex exec resume` is a **distinct subcommand with a reduced flag set**. Verified live in
`docs/active/work/T-021-01/design.md` Q5 (2026-07-11, codex 0.144.1):

> `codex exec resume` on 0.144.1 has a **reduced flag set** — it rejects `-C`, `-s`, and
> `--skip-git-repo-check` (session cwd/sandbox are inherited). The first resume attempt with
> the old harness flags failed exit 2; dropping `-s`/`-C` made it pass.

The ticket reproduces the exact error: `error: unexpected argument '-C' found`, exit 2.
Resume **inherits** cwd, sandbox policy, and (being inside the persisted session's repo)
the git-repo context from the original session — hence these flags are not merely rejected
but semantically redundant.

- `codex` **0.144.1** is confirmed on PATH in this environment (`codex --version`).
- `--json` **is** accepted on resume (Q5 ran `codex exec resume <id> --json …` successfully
  once the three flags were removed) — so `--json` stays on both branches.
- Open question for Design: does `resume` accept `--dangerously-bypass-approvals-and-sandbox`?
  Q5 only exercised the default (sandboxed) path. The safe reading is that resume inherits
  the sandbox from the session, so the bypass flag is also redundant on resume. To be pinned
  by `codex exec resume --help` during Design/Implement.

## The second, latent defect — stdin

`run_agent_exec` (line 538) spawns the child with `.stdout(piped())` and `.stderr(inherit())`
but **does not set `.stdin(...)`**, so the child inherits lisa's stdin. Under headless
automation the pane's stdin is a non-TTY pipe; `codex exec` can block on
`Reading additional input from stdin…`. The ticket asks for `Stdio::null()` on stdin for
the headless path. This is independent of the argv fix but lives two lines away.

## Existing tests (the safety net + the gap)

`mod tests` already covers argv shape:
- `argv_default_flags` — asserts the **full fresh-run** positional vector (will still pass; fresh path unchanged).
- `argv_bypass_sandbox` — fresh run, bypass flag present, no `never`.
- `argv_resume_with_thread` — asserts only `argv[..5] == ["-a","never","exec","resume","th_prev"]`.
- `argv_resume_falls_back_to_last` — asserts a `["resume","--last"]` window exists.
- `argv_passes_extra_codex_args` — passthrough survives.

**Gap:** no test asserts that the resume argv **omits** `-C`, `--skip-git-repo-check`, and
the sandbox flag. `argv_resume_with_thread` only checks a prefix, so it passes today *and*
would pass after the fix — it does not pin the bug. AC requires new resume-shape tests for
both `--bypass-sandbox` and default modes.

## Constraints & boundaries

- **Blast radius is bounded** (ticket + `agent-exec --help`): `lisa loop` drives the native
  Codex TUI, not `agent-exec`. This bug breaks only the diagnostic/headless `agent-exec
  --resume` path. Do **not** widen into the native-TUI adapter (`loop_cmd.rs`).
- **Version volatility:** the codex CLI surface drifts across releases (0.142.5 → 0.144.0 →
  0.144.1 each moved flags). Any test asserting live codex behaviour must be version-pinned
  and re-smoked after `codex update` (ticket Notes; codex-client version-drift sections).
- **`build_codex_argv` must stay pure** — that is what makes the argv unit-testable without
  spawning codex. The fix belongs inside it (branch on `args.resume`), not in `run_agent_exec`.
- **Fresh-run branch must not change** — AC is explicit. `argv_default_flags` /
  `argv_bypass_sandbox` are the regression guard for that invariant.
- The workspace suite, focused CLI tests, the WASM release build, and Clippy must all pass.

## Assumptions to validate in Design

1. Resume inherits sandbox policy → the bypass flag is also redundant/rejected on resume
   (Q5 only proved `-s` is rejected; the bypass flag's acceptance on resume is unconfirmed).
2. `--json` is the only stream flag resume needs (confirmed by Q5).
3. `codex_args` passthrough and `<prompt>` positioning are identical on both branches.
