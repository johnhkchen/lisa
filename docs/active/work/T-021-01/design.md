# T-021-01 Findings — codex exec --json wrapper spike

Per-unknown verdict + evidence, a go/no-go on the wrapper approach, event-mapping
corrections for doc 05, and the tee-vs-render recommendation for T-023-01.

> **Version status.** The spike target is codex **`rust-v0.142.5`**. Codex is
> **not installed on the host this artifact was produced on** (`which codex` →
> not found). The empirical harness (`harness/`) that captures each verdict's
> evidence is complete and syntax-clean, but has not been run against the pinned
> binary. Every verdict below is therefore tagged **[PROVISIONAL]** (reasoned from
> the pinned intel packet + cited `openai/codex` issues) or, where the answer is a
> codebase fact independent of codex, **[CONFIRMED]**. Promotion to authoritative
> is one `harness/run-all.sh` on a host with `rust-v0.142.5`.

## Method

Each unknown maps to one probe under `harness/` that produces falsifiable evidence
(event dumps, exit codes, env grep, stderr/stdout separation). See `structure.md`
for the harness layout and `plan.md` for the run sequence.

---

## Q1 — Env inheritance → **[PROVISIONAL] PASS (expected)**

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

## Q2 — `--json` fidelity under a real ticket → **[PROVISIONAL] PASS with anchor rule**

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

## Q3 — In-pane rendering → **[PROVISIONAL] recommend render-from-JSON**

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

## Q4 — Directory trust headless → **[PROVISIONAL] doctor must pre-seed trust**

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

## Q5 — Follow-up via resume → **[PROVISIONAL] PASS (expected)**

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

**GO on the `codex exec --json` wrapper approach — provisionally, pending one
harness run on `rust-v0.142.5`.** The approach is sound in principle: it needs no
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
