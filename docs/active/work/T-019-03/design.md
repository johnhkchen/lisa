# Design — T-019-03 hooks-guide-command

Decisions, with rationale grounded in the research. Each decision lists the chosen
option and what was rejected.

## D1. Content: embedded static doc vs. runtime-composed sections

**Options**
- (A) Embed a static markdown file from `crates/lisa-cli/data/hooks-guide.md` via
  `include_str!`, print it verbatim. Mirrors `RDSPI_WORKFLOW` (`templates.rs:4`).
- (B) Compose `GuideSection`s at runtime like `setup_guide.rs`, interpolating
  project-detected values.

**Decision: (A) embedded static doc.**

**Why.** The hook set is identical across every project — there is nothing
project-specific to interpolate (research: the four `.sh` hooks, the signal contract,
the `on-notify` env vars, and the catch-all command are universal). `setup_guide`
composes sections *because* it embeds a per-project CLAUDE.md and `.lisa.toml`; the
hooks guide has no such variation. (B) would add a section-rendering layer and a
project-detect call for zero benefit and more surface to test. (A) is the minimal,
faithful pattern: a doc that a human can also read directly in the repo, embedded once,
printed once. The acceptance criteria explicitly bless "pure dump … No project path
needed."

## D2. Where the embed source lives

**Options**
- (A) `crates/lisa-cli/data/hooks-guide.md`, embedded with
  `include_str!("../data/hooks-guide.md")` from `templates.rs`.
- (B) `docs/knowledge/hooks-guide.md`, read at runtime.

**Decision: (A) `crates/lisa-cli/data/`.**

**Why.** `docs/knowledge/` is an *init output target*, not reachable at runtime — the
binary is carried between projects as a single file with zero project dependencies
(research; ticket anchor; CLAUDE.md). (B) would break the moment the binary runs
outside this repo. (A) is exactly how `RDSPI_WORKFLOW` is embedded and is the only
option consistent with "carries between projects as a single `.wasm`/binary."

*Optional companion (not chosen as required):* the ticket allows also writing a
human-facing `docs/knowledge/` copy and keeping it in sync. Rejected for this ticket
to avoid a second source of truth that can silently drift — `data/hooks-guide.md` is
both the compiled-in source and the human-readable file (it lives in the repo and is
perfectly readable there). Adding a `docs/knowledge/` duplicate buys nothing and costs
a sync obligation. If a future ticket wants it surfaced under `docs/knowledge/`, `lisa
init` is the right place to *write* it (like it does for `rdspi-workflow.md`), not a
hand-maintained copy.

## D3. The const and where it goes

**Decision.** Add, immediately after `templates.rs:4` (next to `RDSPI_WORKFLOW`):
```rust
/// The hooks setup guide, embedded at compile time. Printed by `lisa hooks-guide`.
pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");
```
**Why.** The ticket names this const verbatim (`HOOKS_GUIDE`) and its location
("next to `templates.rs:4`"). Keeping all compile-time embeds together in `templates.rs`
matches the existing layout (`RDSPI_WORKFLOW`, `PLUGIN_WASM` are both there).

## D4. Handler module shape

**Options**
- (A) New module `hooks_guide.rs` with `pub fn run_hooks_guide() -> Result<(), String>`
  that prints `templates::HOOKS_GUIDE`.
- (B) Inline the print in `main.rs`'s dispatch arm.
- (C) Add a function to `setup_guide.rs`.

**Decision: (A) new `hooks_guide` module.**

**Why.** The ticket specifies "New `hooks_guide` module with `pub fn run_hooks_guide()`".
It mirrors `setup_guide::run_setup_guide` one-to-one, keeps `main.rs` dispatch uniform
(every arm calls a `module::run_*`), and gives tests a clean target. (B) scatters logic
into `main.rs` (which has no tests). (C) conflates two distinct guides. The signature
returns `Result<(), String>` to match the dispatch convention even though a pure dump
cannot fail — uniformity over micro-optimization, and it leaves room for a future
`--path` variant without changing the signature shape.

**On `--path`:** the acceptance criteria make it optional ("No project path needed …
an optional `--path` … is acceptable but not required"). Decision: **no `--path`** —
the guide is identical regardless of cwd, so a path arg would be inert and misleading
(it would imply the output is contextualized when it is not). `run_hooks_guide()` takes
no arguments. This is the honest, minimal surface. The `Version` arm (`main.rs:93-95`)
is precedent for an argument-free command.

## D5. clap variant naming

**Decision.** Add to `Commands`:
```rust
/// Output the hooks setup guide for agents configuring Claude Code hooks
HooksGuide,
```
**Why.** Clap derives `HooksGuide` → `hooks-guide` automatically (research; same
mechanism that maps `SetupGuide` → `setup-guide`). A unit-less variant (no fields)
matches D4's no-`--path` decision and the `Version` precedent. The doc comment becomes
the `--help` text.

## D6. Dispatch arm

**Decision.** Add, mirroring the `SetupGuide`/`Version` arms:
```rust
Commands::HooksGuide => {
    if let Err(e) = hooks_guide::run_hooks_guide() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```
No `resolve_path` call (no path). `mod hooks_guide;` added to the `main.rs:1-8` block.

**Why.** Uniform with every other arm's error handling (`Error: {e}` + `exit(1)`).
Even though the handler can't currently fail, keeping the `if let Err` shape costs
nothing and matches the others, so the file reads consistently.

## D7. Document content scope (what the guide must say)

Grounded in the acceptance criteria, the doc covers, in order:

1. **What hooks are & the signal flow.** One paragraph: hooks are shell scripts Claude
   Code runs on lifecycle events; they write signal files into `.lisa/signals/`; the
   WASM plugin reads and **deletes** them. Signals flow shell → plugin only.
2. **The four lifecycle hooks** — a table: script, Claude Code event, signal file, role.
   Plus the note that the plugin reads+deletes and that heartbeat is the liveness
   primitive.
3. **The `on-notify` user hook** — the `on-notify <event> [detail]` contract, the full
   env-var list grouped by event (all / complete / attention), `complete` vs
   `attention` semantics, the two fire paths (plugin + catch-all Notification), and the
   `test -x` opt-in model. Includes the copy-paste enable step
   (`cp on-notify.sample on-notify && chmod +x on-notify`) and a ntfy.sh dispatch
   example. Explicit line: **lisa never depends on ntfy or any transport.**
4. **How `lisa init` scaffolds all of this** — the files it writes and that re-running
   is safe/idempotent.
5. **Manual setup** — for a project not `lisa init`'d: the `.lisa/hooks/` layout, the
   `.lisa/signals/` + `.gitignore`, and the full `.claude/settings.local.json` with all
   five bindings (including the exact catch-all command string).
6. **Verify** — `lisa validate` confirms the hook set; what it checks.

**Why this scope/order.** It matches the acceptance-criteria bullet list exactly and
follows the agent's mental path: understand the mechanism → see the automatic path →
fall back to manual → verify. Tables and fenced commands keep it agent-actionable
(style: `lisa-loop-setup-guide.md`).

## D8. Keeping the doc honest about the contract

**Risk.** The env-var list and the catch-all command are duplicated from code
(`lib.rs`, `templates.rs`). Prose can drift from code silently.

**Mitigation (decision).** (a) Write the env-var names and the catch-all command by
copying them verbatim from the current code, citing the source file in the doc so a
future editor knows where truth lives. (b) Add a test that pins the load-bearing
markers so an accidental deletion is caught: the printed guide must be non-empty and
contain `on-notify`, `LISA_EVENT`, `complete`, `attention`, and `cp on-notify.sample`.
This is the same "assert contains" strategy `setup_guide.rs` tests use. We do **not**
attempt a structural equality check against `templates.rs`/`lib.rs` strings — that
would couple a doc to exact code formatting and is brittle; the marker test is the
right altitude.

## D9. Tests

**Decision.** In `hooks_guide.rs`:
- `test_run_hooks_guide_ok` — `run_hooks_guide()` returns `Ok(())`.
- `test_hooks_guide_non_empty` — `templates::HOOKS_GUIDE` is non-empty.
- `test_hooks_guide_contains_contract_markers` — contains `on-notify` and `LISA_EVENT`
  (the ticket's required markers), plus `complete`, `attention`, the four hook
  filenames, and the `cp on-notify.sample on-notify` enable step.
In `templates.rs`:
- `test_hooks_guide_embedded` — `HOOKS_GUIDE.contains("on-notify")` &&
  `.contains("LISA_EVENT")`, mirroring `test_rdspi_workflow_embedded`.

**Why.** Directly satisfies the acceptance criterion ("a test asserting the command's
output is non-empty and contains the `on-notify` contract marker … `on-notify` and
`LISA_EVENT`"). Splitting embed-presence (templates.rs) from handler-behavior
(hooks_guide.rs) matches how `RDSPI_WORKFLOW` is tested in `templates.rs` while
handlers are tested in their own modules.

## D10. Out of scope / non-goals

- No `--path`, no project detection, no per-project rendering (D1/D4).
- No `docs/knowledge/` duplicate (D2).
- No change to `lisa init`, `templates.rs` hook constants, or the plugin — those landed
  in T-019-01/T-019-02; this ticket only *documents* them and adds the dump command.
- No new runtime deps.
