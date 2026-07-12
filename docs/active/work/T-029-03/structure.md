# T-029-03 · Structure — file-level blueprint

The shape of the change. One file touched. No new files, no deletions.

## Files

| File | Action | Why |
|------|--------|-----|
| `crates/lisa-cli/src/agent_exec.rs` | **modify** | Fix `build_codex_argv` resume branch; add null stdin in `run_agent_exec`; add resume-shape unit tests. |

Nothing else. `main.rs` (clap wiring), `loop_cmd.rs` (native TUI), `lisa-core`,
`lisa-plugin` are all untouched. `AgentExecArgs` keeps its current fields.

## Change 1 — `build_codex_argv` (pure; lines ~471–503)

Restructure so the tail flags branch on `args.resume`. Current linear tail
(`--json`, `--skip-git-repo-check`, `-C`, `<cwd>`, sandbox flag) becomes:

```
build_codex_argv(args, resolved_thread):
    argv = []
    if !bypass_sandbox: argv += ["-a", "never"]     # unchanged, top-level, both paths
    argv += ["exec"]

    if resume:
        argv += ["resume", <id> | "--last"]
        argv += ["--json"]                           # resume: stream flag ONLY
        # NO -C, NO --skip-git-repo-check, NO sandbox flag (inherited from session)
    else:
        argv += ["--json", "--skip-git-repo-check", "-C", <cwd>]
        if bypass_sandbox: argv += ["--dangerously-bypass-approvals-and-sandbox"]
        else:              argv += ["-s", "workspace-write"]

    argv += codex_args                               # passthrough, both paths
    argv += [prompt]                                 # positional, both paths
    return argv
```

**Interface unchanged:** same signature `fn build_codex_argv(&AgentExecArgs, Option<&str>) -> Vec<String>`.
Only the internal body branches. Callers (`run_agent_exec`) need no change.

Boundary note: `--json` is duplicated into both arms (rather than hoisted before the
`if`) so each arm reads as a complete, self-documenting argv recipe and the resume arm's
"stream flag only" intent is explicit at the point it matters.

## Change 2 — null stdin in `run_agent_exec` (impure; line ~538)

The `Command` builder currently sets `.stdout(piped())` + `.stderr(inherit())`. Add
`.stdin(Stdio::null())`. `Stdio` is already imported (line 24). Single-line insertion;
no control-flow change; applies to fresh + resume uniformly.

```
Command::new(&args.codex_bin)
    .args(&argv)
    .stdin(Stdio::null())        // <-- new: headless, non-TTY-safe
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
```

## Change 3 — tests (`mod tests`, Layer E)

Add resume-shape assertions. Keep existing tests as-is (they guard the fresh branch and
the resume prefix). New/adjusted tests:

- `argv_resume_omits_cwd_and_sandbox_flags` *(new)* — default mode, `resume=true`,
  `resolved_thread=Some("th_prev")`. Assert the argv **contains** `resume`, `th_prev`,
  `--json`; and **does not contain** `-C`, `--skip-git-repo-check`, `-s`, `workspace-write`,
  `--dangerously-bypass-approvals-and-sandbox`. Assert `-a`/`never` **is** present (default
  mode keeps the top-level approval flag). Assert `prompt` is last.
- `argv_resume_bypass_omits_all_sandbox_and_cwd_flags` *(new)* — `bypass_sandbox=true`,
  `resume=true`. Assert no `-C`, `--skip-git-repo-check`, `-s`, and **no** bypass flag; no
  `-a`/`never` (bypass suppresses it); `resume` + `--json` present; `prompt` last.
- `argv_resume_last_omits_cwd_and_sandbox_flags` *(new, optional)* — `resume=true`,
  `resolved_thread=None` ⇒ `--last`; assert same omissions with `--last` in place of an id.
- `argv_resume_passes_extra_codex_args` *(new, optional)* — resume + `codex_args=["--model","o4"]`;
  assert the `["--model","o4"]` window survives on the resume path too.

Existing tests retained unchanged as the **fresh-branch invariant guard**:
- `argv_default_flags` — full fresh vector (proves fresh path is byte-identical).
- `argv_bypass_sandbox` — fresh bypass path.
- `argv_resume_with_thread`, `argv_resume_falls_back_to_last` — resume prefix + `--last`.
- `argv_passes_extra_codex_args` — fresh passthrough.

All new argv tests are **pure** (no codex spawn) ⇒ no version drift. A module-level comment
on the resume arm cites `codex 0.144.1` and the re-smoke rule so the *reason* the flags are
dropped is discoverable.

## Ordering

1. Edit `build_codex_argv` body (Change 1).
2. Add `.stdin(Stdio::null())` (Change 2).
3. Add unit tests (Change 3).
4. `cargo test -p lisa-cli` (focused) → `cargo test --workspace` → WASM release build → Clippy.
5. Optional live smoke on 0.144.1 (persisted thread) to satisfy AC-2 empirically — documented
   in `progress.md`, not gating CI (needs a real session + codex on PATH).

Steps 1–3 are one atomic commit (the fix + its tests belong together). Step 4 is verification.

## Interfaces & invariants

- **Public surface:** none changes. `run_agent_exec` and `AgentExecArgs` signatures stable.
- **Invariant preserved:** fresh-run argv is byte-for-byte identical (guarded by
  `argv_default_flags` / `argv_bypass_sandbox`).
- **Invariant established:** resume argv carries no `-C`, no `--skip-git-repo-check`, no
  sandbox flag (guarded by the new tests).
- **Purity invariant:** all argv logic stays in `build_codex_argv`; the only impure edit is
  the stdin wiring.
