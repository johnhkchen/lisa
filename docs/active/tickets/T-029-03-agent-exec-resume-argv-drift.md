---
id: T-029-03
story: S-029
title: agent-exec-resume-argv-drift
type: bug
status: open
priority: medium
phase: done
depends_on: []
---

## Context

Found during the T-029-01 live run (codex-cli **0.144.1**, 2026-07-11). The
`resume` branch of `build_codex_argv` (`crates/lisa-cli/src/agent_exec.rs`,
~lines 480–500) appends `-C <cwd>`, `--skip-git-repo-check`, and
`-s workspace-write` on every run. On codex ≥ 0.144.1 the `codex exec resume`
subcommand has a **reduced flag set** and **rejects all three** (cwd and sandbox
are inherited from the resumed session):

```
$ codex -a never exec resume <id> --json --skip-git-repo-check -C <dir> -s workspace-write "…"
error: unexpected argument '-C' found
exit 2
```

So `lisa agent-exec --resume` exits 2 and produces no signals on this version.

**Blast radius is bounded:** `lisa loop` drives the **native Codex TUI**, not
`agent-exec` (`agent-exec --help`: "`lisa loop` uses the native Codex TUI; this
remains available for diagnostics and headless automation"). The review-timeout
finish-up in the live loop types into the resident TUI, so this bug does **not**
break the loop — it breaks the diagnostic/headless `agent-exec --resume` path
and anything that shells out to it. The fresh-run `agent-exec` path is fine (it
puts `-a never` in the top-level position, which 0.144.1 still accepts).

Related drift documented in `docs/knowledge/codex-client/02-codex-capabilities.md`
(0.144.1 write-back) and `docs/active/work/T-021-01/design.md` (Q5 verdict + drift
note). A second, latent issue in the same file: the spawned child does not set
`.stdin(...)` (`agent_exec.rs:538`), and `codex exec` blocks reading stdin behind
a non-TTY pipe — headless `agent-exec` should pass `Stdio::null()` for stdin.

## Acceptance Criteria

- On the `resume` branch, `build_codex_argv` omits `-C`, `--skip-git-repo-check`,
  and `-s`/`--sandbox` (or the bypass flag) — resume inherits session cwd/sandbox.
  The fresh-run branch is unchanged.
- `lisa agent-exec --resume "<nudge>"` against a persisted thread on codex 0.144.1
  exits 0, re-enters the thread, and emits the expected `pane-<id>.*` signals.
- Headless `agent-exec` sets the child's stdin to null so `codex exec` cannot
  hang on `Reading additional input from stdin…` behind a non-TTY pipe.
- `build_codex_argv` unit tests cover the resume argv shape (no cwd/sandbox
  flags) for both `--bypass-sandbox` and default modes.
- Focused plugin/CLI tests, the workspace suite, the WASM release build, and
  Clippy all pass.

## Notes

- Do **not** widen scope into the native-TUI adapter; that path is unaffected.
- Version-pin any test that asserts codex CLI behaviour — this surface drifts
  (see the codex-client version-drift sections). Re-smoke after `codex update`.
