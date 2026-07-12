# T-029-03 · Design — resume-branch argv + headless stdin

Decide how `build_codex_argv` should shape the `resume` branch, and how the child's
stdin should be wired. Grounded in Research + a live `codex exec resume --help` probe.

## Live flag-surface finding (resolves Research open question)

`codex exec resume --help` on **0.144.1** (probed 2026-07-11) lists these and only these
relevant flags: `--json`, `--skip-git-repo-check`, `--dangerously-bypass-approvals-and-sandbox`,
`--ephemeral`, `-c/--config`, `-m/--model`, `-o/--output-last-message`, `--output-schema`,
`--last`, `--all`, `-i/--image`, `--ignore-user-config`, `--ignore-rules`.

**Absent (⇒ rejected):** `-C`/`--cd`, `-s`/`--sandbox`.
**Present (⇒ accepted):** `--json`, `--skip-git-repo-check`, `--dangerously-bypass-approvals-and-sandbox`.

So the ticket's assertion that resume rejects `--skip-git-repo-check` is **imprecise**:
clap aborts on the *first* unexpected arg (`-C`), so the Q5 probe never reached the
git flag. The only flags 0.144.1 *actually* rejects on resume are `-C` and `-s`.

This matters for choosing how aggressive the fix should be.

## What must be true

- **AC-1:** resume branch omits `-C`, `--skip-git-repo-check`, and the sandbox flag
  (`-s`/`--sandbox` *or* the bypass flag). Fresh branch unchanged.
- **AC-2:** `agent-exec --resume` on 0.144.1 exits 0, re-enters the thread, emits signals.
- **AC-3:** headless child stdin = null.
- **AC-4:** unit tests pin the resume argv shape (no cwd/sandbox flags) for bypass + default.

Note AC-1 is stricter than 0.144.1 *requires* (it demands dropping `--skip-git-repo-check`
too, which 0.144.1 tolerates). The design must honour the AC, not the minimum.

## Options

### Option A — Minimal fix: drop only what 0.144.1 rejects (`-C`, `-s`)

Branch the argv so resume skips `-C <cwd>` and the sandbox flag, but keep
`--skip-git-repo-check`. Smallest diff that makes 0.144.1 pass.

- **Pro:** minimal change; matches the observed live-reject set exactly.
- **Con:** violates AC-1, which explicitly lists `--skip-git-repo-check` among the omissions.
  Also keeps a flag that is semantically meaningless on resume (git context is inherited
  from the persisted session) and is a future-drift liability — a later codex could tighten
  resume's parser to reject it, silently reintroducing the exit-2 bug.
- **Rejected:** fails an explicit acceptance criterion.

### Option B — AC-faithful: resume emits only stream flag + passthrough + prompt (CHOSEN)

On the resume branch, after `exec resume <id|--last>`, emit **only** `--json`, then the
`codex_args` passthrough, then the prompt. Drop `-C`, `--skip-git-repo-check`, and **all**
sandbox flags (both `-s workspace-write` and `--dangerously-bypass-approvals-and-sandbox`).
Resume inherits cwd, sandbox policy, approval policy, and git context from the persisted
session. Fresh branch keeps its exact current shape.

- **Pro:** satisfies AC-1 literally; drops every flag that is redundant-on-resume, so the
  argv can't drift into a reject as codex tightens the resume parser; `--json` is confirmed
  accepted (Q5 + help); smallest surface that is also the most future-proof.
- **Pro:** keeps `-a never` (top-level, emitted before `exec`) on both paths — it is a
  global option, accepted regardless of subcommand, and preserves the autonomous
  no-approval posture on the resumed turn (the ticket's repro shows `-a never exec resume`
  parsed fine).
- **Con:** if a session were somehow resumed into a *different* desired sandbox, resume
  can't override it — but that is not a use case (`agent-exec --resume` continues the same
  ticket's thread in place), and `-c sandbox_mode=…` remains available via `codex_args` if
  ever needed.
- **Chosen** — it is the only option that meets AC-1 and is the most drift-resilient.

### Option C — Keep the bypass flag on resume, drop only `-s`/`-C`/`--skip-git-repo-check`

A middle path: on resume still pass `--dangerously-bypass-approvals-and-sandbox` when
`bypass_sandbox`, since resume *accepts* it (help confirms), but drop the other three.

- **Pro:** preserves an explicit "no sandbox" intent on the resumed turn even though it's
  inherited.
- **Con:** AC-1 says omit "`-s`/`--sandbox` (or the bypass flag)" — i.e. the sandbox flag in
  *either* spelling. Passing the bypass flag on resume is redundant (sandbox is inherited)
  and adds a second code path (bypass-on-resume) that the tests must cover for no behavioural
  gain. Simpler to drop all sandbox flags on resume uniformly.
- **Rejected:** more surface, no benefit, borderline against AC-1's parenthetical.

## Chosen shapes (Option B)

Fresh run (default) — **unchanged**:
```
-a never  exec  --json  --skip-git-repo-check  -C <cwd>  -s workspace-write  [codex_args…]  <prompt>
```
Fresh run (bypass) — **unchanged**:
```
exec  --json  --skip-git-repo-check  -C <cwd>  --dangerously-bypass-approvals-and-sandbox  [codex_args…]  <prompt>
```
Resume (default) — **new**:
```
-a never  exec  resume  <id|--last>  --json  [codex_args…]  <prompt>
```
Resume (bypass) — **new** (note: no `-a never` because `bypass_sandbox` suppresses it, and
no sandbox flag because resume inherits it):
```
exec  resume  <id|--last>  --json  [codex_args…]  <prompt>
```

Rationale for `--json` staying on both: it is the event-stream contract the whole
`Translator` depends on, and resume accepts it (Q5 ran it live, help lists it).

## Stdin decision

Add `.stdin(Stdio::null())` to the `Command` builder in `run_agent_exec` (currently only
`.stdout`/`.stderr` are set). `codex exec` reads a prompt from stdin only when the positional
prompt is `-`; lisa always passes an explicit prompt positional, so null stdin is safe and
prevents `Reading additional input from stdin…` hangs behind a non-TTY pipe. `Stdio` is
already imported. Applies to both fresh and resume runs — headless `agent-exec` never needs
an interactive stdin.

## Purity & test strategy

- Keep the branch **inside** `build_codex_argv` (stays pure ⇒ unit-testable without spawning).
  The stdin change is the only edit to the impure `run_agent_exec`.
- New tests assert the resume argv **omits** `-C`, `--skip-git-repo-check`, and any sandbox
  flag, for both default and `--bypass-sandbox`. Existing `argv_default_flags` /
  `argv_bypass_sandbox` stay green ⇒ they are the fresh-branch regression guard (AC-1).
- **Version-pin:** any test/comment asserting live codex behaviour is annotated with
  `codex 0.144.1` and the re-smoke-after-update note (ticket Notes). Unit tests over
  `build_codex_argv` are pure and don't invoke codex, so they don't drift — but the comment
  documenting *why* the flags are dropped cites the version.

## Out of scope (guardrails)

- Native-TUI adapter (`loop_cmd.rs`) — unaffected; `lisa loop` doesn't use `agent-exec`.
- Event translator, signal writers, provenance — untouched.
- No change to `main.rs` clap wiring; `AgentExecArgs` fields are sufficient.
