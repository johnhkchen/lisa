# T-021-01 Findings — codex exec --json wrapper spike

Per-unknown verdict + evidence, a go/no-go on the wrapper approach, event-mapping
corrections for doc 05, and the tee-vs-render recommendation for T-023-01.

> **Version status.** Spike target was codex **`rust-v0.142.5`**; the live run
> (T-029-01, **2026-07-11**) executed against the installed **codex-cli
> `0.144.1`** — record all drift. Every unknown below now carries a
> **[VERIFIED 0.144.1]** verdict from a real `codex exec` run, replacing the
> earlier **[PROVISIONAL]** reasoning. Evidence lives under
> `scratchpad/out-0.144.1/` (session-local) and is summarized in
> `docs/active/work/T-029-01/progress.md`; the flag-drift that made the original
> `harness/*.sh` unable to reach codex is recorded there too.
>
> **Live-run header verdict (2026-07-11, codex 0.144.1):**
> Q1 **PASS** · Q2 **PASS (hard gate cleared)** · Q3 **render-from-JSON confirmed**
> · Q4 **no trust block in `exec` on this version** (doctor pre-seed still needed
> for the native TUI) · Q5 **PASS (with a resume-flag correction)**. GO stands;
> no app-server escalation required. Three CLI-surface drifts were found — see
> the drift note at the foot of this file.

## Method

Each unknown maps to one probe under `harness/` that produces falsifiable evidence
(event dumps, exit codes, env grep, stderr/stdout separation). See `structure.md`
for the harness layout and `plan.md` for the run sequence.

---

## Q1 — Env inheritance → **[VERIFIED 0.144.1] PASS**

> **Verified 2026-07-11 (codex 0.144.1).** A `codex exec` launched with
> `LISA_PANE_ID=7` forced an `env | grep LISA_PANE_ID` shell tool call; the
> codex-spawned shell's `command_execution` output contained `LISA_PANE_ID=7`
> (`out-0.144.1/q1/child-saw.txt`), exit 0. Env survives the launch → codex →
> tool-shell chain. Deterministic pane attribution holds. `lisa agent-exec`
> reproduces this live (it read `LISA_PANE_ID=9` to name `pane-9.*` signals).


**Claim.** A wrapper launched as `LISA_PANE_ID=7 <wrapper> …` running
`codex exec` passes `LISA_PANE_ID` to codex, and codex passes it to the shell it
spawns for a tool call.

**Why expected-pass.** `codex exec` is an ordinary child process; env is inherited
by default (no documented env-scrubbing in exec mode, doc 02). Tool-call shells are
spawned by codex under `-s workspace-write` and likewise inherit. lisa already
proves the identical pattern works for Claude via `build_claude_command`
(`lib.rs:53`), which relies on the same OS inheritance.

**Evidence to capture** (`harness/q1-env-inheritance.sh`): `child-saw.txt` must
contain `LISA_PANE_ID=7`, extracted from the `command_execution` item output of a
forced `env | grep LISA_PANE_ID` tool call.

**Risk if wrong.** If codex scrubs env before tool shells, attribution falls back
to writing the pane id into a temp file the wrapper controls, or into the prompt.
Low likelihood; no such behaviour is documented for `rust-v0.142.5`.

---

## Q2 — `--json` fidelity under a real ticket → **[VERIFIED 0.144.1] PASS — hard gate cleared**

> **Verified 2026-07-11 (codex 0.144.1).** The RDSPI fixture (forces file
> read/write + shell) produced a clean stream (`out-0.144.1/q2/anchor-check.txt`):
> `thread.started=1, turn.started=1, turn.completed=1, turn.failed=0,
> item.completed=5, command_execution=4, file_change=2, exit=0`. A terminal
> `turn.completed` is present **and agrees with exit 0**, and tool activity is
> **present, not silently dropped** — **#15451 does not reproduce** with builtin
> tools on this version. The anchor rule holds. `turn.completed` carries
> `usage:{input_tokens, cached_input_tokens, output_tokens,
> reasoning_output_tokens}` — the exact shape doc 02 predicted (no drift), and
> the input to T-027-02 cost capture. **The go/no-go gate is GREEN.**


**Claim.** Under active MCP/tools, the JSONL stream carries a terminal
`turn.completed`/`turn.failed` that agrees with the process exit code, and item
activity is present (not silently dropped).

**Why expected-pass, with caveats.**
- #15451 (`--json` silently ignored under some active MCP/tools) is the real risk.
  Mitigation baked into the probe: the fixture forces *builtin* file/shell tool
  calls (not an external MCP server), which is the common lisa case; if the stream
  still drops events, that is the go/no-go trigger.
- #14691 (abandoned/wrong item status at turn end) is **expected to reproduce** and
  is exactly why the anchor rule exists: **derive done/failed from
  `turn.completed`/`turn.failed` + process exit, treat `item.*` as best-effort
  heartbeat only.** The probe cross-checks the terminal turn event against
  `exit-code`; disagreement there (not item flakiness) would be disqualifying.

**Evidence to capture** (`harness/q2-json-fidelity.sh`): `anchor-check.txt`
(counts of `thread.started`/`turn.*`/`item.completed`/`command_execution` + exit)
and `event-histogram.txt`.

**Corrections this may force on doc 05's table** — recorded here so T-023-02
inherits the truth, not the guess:
- Confirm the exact event-name casing/form emitted by `exec` (dot-form
  `turn.completed` vs. any drift) against `stdout.jsonl`; the adapter's parser keys
  on these strings.
- Confirm whether `usage` rides on `turn.completed` (needed later for
  T-027-02 cost capture).

---

## Q3 — In-pane rendering → **[VERIFIED 0.144.1] render-from-JSON confirmed**

> **Verified 2026-07-11 (codex 0.144.1).** Under `--json`, stderr carried only a
> 39-byte status line (`Reading additional input from stdin…`) — **spinner-only,
> not rich** (`out-0.144.1/q3/stderr-analysis.txt`). The stdout stream emitted
> **completed-only** items (`agent_message` on `item.completed`) with **zero
> `*delta*` events** (`granularity.txt`). Both facts point the same way: build
> the pane view from JSON, not stderr. This **matches what shipped**
> (`agent_exec.rs` renders from the JSON events; the loop's native TUI renders
> itself) — **no follow-up ticket needed** (AC Q3 satisfied: the wrapper's
> render-from-JSON mode is confirmed, not contradicted). Token-by-token deltas
> remain an app-server-only feature, out of scope.


**Claim/decision.** Pick tee-stderr vs. render-from-JSON for T-023-01.

**Why render-from-JSON is the provisional recommendation.**
- Doc 05 flags it **unverified** whether `--json` keeps *rich* human output on
  stderr or only a spinner (`[M]`). Building the pane view on "rich stderr" is a
  bet on an unverified property of a version-volatile surface.
- `exec --json` is **coarse-grained**: you largely get an assistant message when
  its `item` *completes* (doc 05 §live-ness split) — there are no token deltas in
  exec (those are app-server's `item/agentMessage/delta`). So a render-from-JSON
  view and a tee-stderr view show the *same* chunked granularity anyway; rendering
  from JSON just removes the dependency on an unverified stderr format and reuses
  the JSON the wrapper already parses for signals (one read loop → both, doc 05
  §Observability point 3).
- **Escalate to app-server only if** a hard requirement for token-by-token
  streaming appears — out of scope for the autonomous "run tickets" goal.

**Evidence to capture** (`harness/q3-inpane-rendering.sh`): `stderr-analysis.txt`
(is stderr rich or spinner-only?) and `granularity.txt` (any `*delta*` item events,
or completed-only?). If stderr turns out **rich and stable**, tee-stderr becomes
the cheaper choice and this recommendation flips — that is precisely what the probe
decides.

---

## Q4 — Directory trust headless → **[VERIFIED 0.144.1] no `exec` block on this version; doctor pre-seed retained for the native TUI**

> **Verified 2026-07-11 (codex 0.144.1).** Re-scoped to remove the auth confound:
> instead of a fresh (credential-less) `CODEX_HOME`, the probe used the real
> logged-in `~/.codex` against an **untrusted** throwaway repo and forced a shell
> tool call. Result: `codex exec -s workspace-write -c approval_policy=never` ran
> the command and returned exit 0 (`out-0.144.1/q4-trust/A.*`: `command_execution=2`,
> `TRUSTED_OK` echoed) — **directory trust did NOT block `codex exec`** on 0.144.1.
> So `-a never`/`exec` does not need a trust pre-seed on this version. **However**,
> `lisa doctor`'s pre-seed is *still verified working and still justified*: it
> seeded `trust_level="trusted"` for the dry-run path
> (`docs/active/work/T-029-01/out-doctor.txt`), and doc 09 records the **native
> TUI still shows a trust prompt** (#14345), which is the loop's real path.
> **#14345 re-verified: unblocked on the `exec` path, pre-seed retained for TUI.**


**Claim.** A fresh `CODEX_HOME` blocks `codex exec -a never` on an untrusted repo;
seeding `[projects.<path>].trust_level = "trusted"` (or the bypass flag) unblocks
it.

**Why expected.** Trust-on-first-use is codex's default; `-a never` removes the
approval channel, so an untrusted repo has no way to be granted trust interactively
in headless mode → block is the likely outcome (doc 05 §unknown 3, bug #14345).
The probe runs three cases (A unseeded, B seeded, C `--dangerously-bypass-…`) so
`lisa doctor` learns the exact minimal seed.

**Design consequence for T-025-01 doctor.** Provisional plan: doctor writes/patches
`$CODEX_HOME/config.toml` with a `[projects."<abs-working-tree>"]
trust_level = "trusted"` block for the loop's working tree, keeping
`--dangerously-bypass-approvals-and-sandbox` as an explicit escape hatch (it also
disables the sandbox, so it is not the default). #14345 means this must be
**re-verified per codex version**, not assumed stable.

**Evidence to capture** (`harness/q4-directory-trust.sh/{A,B,C}/exit-code` +
stderr trust/approval mentions).

---

## Q5 — Follow-up via resume → **[VERIFIED 0.144.1] PASS (with a resume-flag correction)**

> **Verified 2026-07-11 (codex 0.144.1).** Turn 1 planted codeword MARMALADE and
> `thread.started` carried `thread_id`; turn 2 `codex exec resume <id>` recalled
> **MARMALADE**, exit 0 (`out-0.144.1/q5/{turn1b,turn2b}.jsonl`). Context carries
> → the `finish_up_prompt` analog works. **Drift caught:** `codex exec resume` on
> 0.144.1 has a **reduced flag set** — it rejects `-C`, `-s`, and
> `--skip-git-repo-check` (session cwd/sandbox are inherited). The first resume
> attempt with the old harness flags failed exit 2; dropping `-s`/`-C` made it
> pass. See the drift note — the shipped `agent-exec --resume` argv is affected.


**Claim.** `codex exec resume <thread_id>` continues a completed session with new
instructions, carrying prior context.

**Why expected-pass.** Documented as the headless multi-turn mechanism (doc 05
Option 1 pros; `resume <thread_id|--last>`). The probe proves *context carry*, not
just "runs": turn 1 plants a codeword, turn 2 resumes and must recall it.

**Design consequence for T-023-02.** The `finish_up_prompt` analog is a
`codex exec resume <thread_id>` turn with the nudge text. The wrapper must persist
the `thread_id` from turn 1's `thread.started` event (already in doc 05's mapping:
"record thread id for optional resume"). `--last` is the fallback if per-thread id
capture proves flaky.

---

## Go / No-Go

**GO — confirmed live on codex 0.144.1 (2026-07-11).** The Q2 hard gate is
green: a terminal `turn.completed` agrees with the exit code and tool activity is
not dropped, so no app-server escalation is triggered. The approach is sound: it needs no
hooks, no TUI scraping, and gets deterministic pane attribution from env (Q1). The
two signals lisa actually needs on the Codex path (`.heartbeat`, `.stopped`) are
cleanly derivable from `item.*` + `turn.completed`+exit. The only **hard** go/no-go
gate is **Q2**: if `--json` drops events under builtin tool activity (#15451) such
that no reliable terminal turn event survives, escalate to the app-server (doc 05
Option 2) as a **human decision** — do not design around a broken stream.

**Ranked residual risks:** Q2 (#15451, hard gate) > Q4 (trust seeding, version-
volatile #14345) > Q3 (stderr richness, only affects render choice) > Q1/Q5
(both well-supported, low risk).

## Corrections fed back to doc 05

1. **`.error` has no consumer in the current scheduler** (`lib.rs` has no
   `.error` reader). Doc 05's `turn.failed`/non-zero-exit → `.error` mapping is
   aspirational; on today's code a failed turn must surface as `.stopped` unless
   T-023 adds a consumer. Flagged for the T-023 structure.
2. Everything else in doc 05's mapping stands pending Q2's captured event names.

## Rejected alternatives (already decided upstream, restated for the record)

- **Interactive TUI driving / keystroke injection** — refuted (doc 04, paste-burst).
- **Interactive hooks for signals** — refuted (doc 04, #17532, no heartbeat cadence).
- **app-server (Option 2) as the default** — rejected for the autonomous goal:
  heaviest surface, no back-compat guarantee. Held as the Q2-failure fallback only.

## CLI-surface drift found on 0.144.1 (2026-07-11 live run)

The pinned probes (`harness/*.sh`, written for `rust-v0.142.5`) could not reach
codex on 0.144.1 — all five died at arg-parsing (exit 2) before codex ran. The
corrected re-run (`scratchpad/rerun.sh`) established the verdicts above and
surfaced three drifts, written back to `docs/knowledge/codex-client/`:

1. **`-a`/`--ask-for-approval` moved to top-level-only.** `codex exec -a never`
   (flag *after* the subcommand) is rejected on 0.144.1; `codex -a never exec …`
   (top-level, *before* `exec`) still works. Shipped `agent-exec` already emits
   the top-level position (`build_codex_argv`, comment at `agent_exec.rs:467`) —
   its **fresh-run path is unaffected**; the harness stubs are stale.
2. **`codex exec` blocks reading stdin** when stdin is an open non-TTY pipe
   (`Reading additional input from stdin…` hangs). Headless callers must
   redirect `</dev/null`. The native TUI (a real TTY) is unaffected; `agent-exec`
   inherits stdin (`agent_exec.rs:538` sets no `.stdin`), so headless/diagnostic
   use behind a non-TTY pipe can hang.
3. **`codex exec resume` has a reduced flag set** — rejects `-C`, `-s`,
   `--skip-git-repo-check` (cwd/sandbox inherited from the session). Shipped
   `build_codex_argv` appends all three on the `resume` branch
   (`agent_exec.rs:489-497`), so **`lisa agent-exec --resume` exits 2 on codex
   ≥0.144.1** — filed as a bug. Blast radius is diagnostics/headless only: the
   loop's review-timeout finish-up runs through the **native TUI**, not
   `agent-exec` (`agent-exec --help`: "`lisa loop` uses the native Codex TUI").
4. **No drift** in the `--json` event vocabulary or `turn.completed.usage` shape
   vs. docs 02/05 — dot-form events, item types `agent_message`/
   `command_execution`/`file_change`, usage fields as documented.
