# T-057-01-05 — Research

What exists today, where, and how it connects. Descriptive only.

## 1. The document and how it reaches a project

- **Source of truth on disk:** `crates/lisa-cli/data/rdspi-workflow.md` (144 lines).
- **Embedded:** `crates/lisa-cli/src/templates.rs:4–10` — `RDSPI_WORKFLOW` is a `LazyLock<String>`
  that prefixes `PURPOSE_PARAGRAPH` (from `lisa-core/src/context.rs:8`) to the `include_str!` of
  that data file. Rendered length: 146 lines.
- **Checked into this repository:** `docs/knowledge/rdspi-workflow.md`, pinned byte-for-byte
  against the rendered template by `templates.rs:653–657`. This repo is itself a Lisa project, so
  the file is both Lisa's output and Lisa's input.
- **Installed:** `init.rs:362–368` calls `plan_owned_template(root/"docs/knowledge/rdspi-workflow.md",
  RDSPI_WORKFLOW, LEGACY_RDSPI_WORKFLOWS)`.
- **Required by validate:** `init.rs:1160–1168` — absence is an *error* diagnostic, not a warning.
- **Named in the assignment prompt:** already migrated. `crates/lisa-plugin/src/lib.rs` (T-057-01-04)
  asserts the prompt contains `docs/knowledge/lisa-workflow.md` and does **not** contain `rdspi`
  (`lib.rs:~13505`). **The plugin already points at a file that does not exist yet.** That is the
  most load-bearing fact in this research: this ticket is not proposing the new path, it is
  making the path the shipped scheduler already names real.

## 2. The ownership rule that must survive

`plan_owned_template` (`init.rs:257–283`) is four arms:

```rust
!path.exists()                            => CreateFile
Ok(existing) if existing == current       => NoOp
Ok(existing) if known_prior.contains(..)  => UpdateFile     // an unmodified Lisa template
Ok(_)                                     => SafetySkip     // the operator's file now
Err(_)                                    => SafetySkip     // unreadable
```

`LEGACY_RDSPI_WORKFLOWS` (`templates.rs:14–17`) holds two byte-exact prior generations:
`data/legacy/rdspi-workflow-v0.2.md` (103 lines) and `-v0.4.md` (111 lines). Note what those
files hold: the *rendered* text, purpose paragraph included, because the comparison is against
`RDSPI_WORKFLOW.as_str()`, not the raw data file. A new legacy entry must be captured the same
way.

The executor (`init.rs:956–987`) has five arms and no delete arm. `InitAction` (`init.rs:230–236`)
has five variants and no removal variant. Adding a removal is therefore a change to three places:
the enum, its `Display` (`init.rs:238–252`), and the executor.

`FileMutation`/`FileMutationKind` (used at `init.rs:972–984`) records what a run touched; a
removal is a mutation an honest run should report.

## 3. What the outgoing document contains

146 rendered lines. Two halves:

**Dead half — goes.**
- `~7–37` — Research/Design/Structure/Plan chapters, each demanding a ~200-line artifact.
- `~88–101` — "Phase Rules" 1–5: all six phases always run, ~200 lines each, phase transitions,
  high-leverage phases, and Rule 5 ("Artifacts are insurance").

**Live half — stays, verbatim in meaning.**
- `~41–47` Implement's commit discipline (`lisa commit-ticket`, exact `--include`, never the
  ordinary index, journal-sealed projects skip commits).
- `~49–59` Review: `review.md` + the three disposition shapes (pass / block / note), the
  `remedy_owner` vocabulary, `check`/`steps` guidance.
- `~61–69` The five clauses of the check-execution contract (where it runs, what it sees, writes,
  how long, exit codes) — **pinned by test**, see §4.
- `~71–76` How to write an `ask` for a bystander, with the counter-example paragraph.
- `~77–79` `lisa check-disposition`, and the seal-gated wait-for-Lisa boundary.
- `~100` Rule 6, completion is seal-gated.
- `~104–138` Ticket format and frontmatter field list — the `phase:` line still enumerates eight
  values including the four retired ones.
- `~142–147` Concurrency and DAG rules.

## 4. The tests that pin the document

All in `templates.rs`, all reading `RDSPI_WORKFLOW`:

| Test | What it pins |
|---|---|
| `test_rdspi_workflow_embedded` | Heading `RDSPI Workflow`; the six phase words; **byte equality with `docs/knowledge/rdspi-workflow.md`** |
| `test_review_disposition_contract_is_injected` | 13 exact substrings: the three JSON shapes, the `remedy_owner` sentence, the `ask` guidance, the field counter-example, `lisa check-disposition <ticket-id>` |
| `the_documented_check_contract_matches_the_code_that_enforces_it` | The five clauses, with the budget numbers *formatted from* `DEFAULT_CHECK_BUDGET_SECS` (5) and `MAX_CHECK_BUDGET_SECS` (1800) rather than typed |
| `test_injected_context_is_purpose_first_and_copy_is_single_sourced` | The purpose paragraph appears exactly once, before any mechanism word (`dag`, `phase`, `scheduling`, `zellij`); and exactly one template source in the tree contains it — the list at `templates.rs:785–790` names `../data/rdspi-workflow.md` by path |

The middle two are the S-056-01 pins. They constrain *content*, not path, so they survive a
rename untouched as long as the sentences do. The first and last name the path and must move.

`disposition.rs:26` and `check_run.rs:7` are doc comments citing
`docs/knowledge/rdspi-workflow.md`; the second is the file the check contract describes.

## 5. The system the document must now describe

- **`Phase`** (`lisa-core/src/types.rs:121–143`): `Ready`, `Implement`, `Review`, `Done`.
  `Implement` carries `#[serde(alias = "research"|"design"|"structure"|"plan")]` so a 0.4 board and
  a fail-closed journal replay both still load. Only `implement` is ever written back.
- **`Phase::next`** (`:158–165`): `Ready → Implement → Review → Done`.
- **`Phase::artifact_filename`** (`:172–177`): `Review => review.md`; everything else `None`.
  `Implement` deliberately has no artifact — `progress.md` was never a phase edge.
- **The plugin** no longer publishes `progress.md` (commit `c4772bd`) and reads the phase edge
  from one place.
- **Resume:** an attempt's private `.lisa/attempts/<id>/<n>/work/` is not a resume surface. In a
  commit-sealed project a dead session's finished work is on the branch, because
  `lisa commit-ticket` committed it. In a journal-sealed project there are no commits, so nothing
  survives a mid-ticket death — the ticket restarts from the beginning. S-057-01 names this an
  accepted regression for this cut and asks for it to be stated where an operator will read it.

## 6. Every surface that names the old thing

`git ls-files | xargs grep -ril rdspi`, excluding `docs/archive/**`, `docs/active/work/**`, and the
board's own ticket/story/epic files (history, not description):

**Must change — describes the live system**

| File | What it says |
|---|---|
| `crates/lisa-cli/data/rdspi-workflow.md` | the document itself |
| `docs/knowledge/rdspi-workflow.md` | this project's installed copy |
| `crates/lisa-cli/src/templates.rs` | `RDSPI_WORKFLOW`, `LEGACY_RDSPI_WORKFLOWS`, 4 tests, the single-source path list |
| `crates/lisa-cli/src/init.rs` | install path, validate check, ~40 test call sites |
| `crates/lisa-cli/src/setup_guide.rs` | scaffold table row, "you need not mention the RDSPI workflow", validate checklist, `research = 300` example, 2 tests |
| `crates/lisa-cli/src/check_run.rs:7` | doc comment citing the path |
| `crates/lisa-core/src/disposition.rs:26` | doc comment citing the path |
| `crates/lisa-core/src/types.rs:339,441` | "RDSPI workflow" in two doc comments |
| `crates/lisa-core/src/ticket.rs:4` | module doc |
| `crates/lisa-core/src/context.rs:5,18` | `ROLE_CONTRACT` tells an agent to take a ticket "through its RDSPI phases" |
| `crates/lisa-plugin/src/adapter.rs:354` | doc comment |
| `README.md:76,258,286–293,356,363` | six-phase list, layout tree, `lisa init` description |
| `CONTRIBUTING.md:86–91` | "five phases", link to the old path |
| `CLAUDE.md:15,54,68` / `AGENTS.md:11` | this repo's own context files |
| `docs/ROADMAP.md:15,38,40,169` | historical entries naming the document |
| `docs/PROMPT_CODEX.md:18,56` | onboarding prompt naming the path |
| `aur/PKGBUILD:7` | package description |
| `crates/lisa-cli/tests/fixtures/live_provider_startup.sh:268` | "Produce concise RDSPI phase artifacts only" |
| `docker/chromebook-test/bin/prepare:104` | "create the required RDSPI phase artifacts" |
| `docker/chromebook-test/board/{tickets/T-004.md,stories/S-001.md}` | use "RDSPI" as an example of jargon to avoid |
| `crates/lisa-plugin/tests/fixtures/codex_ack/*.json` | two synthetic ack payloads whose `prompt` field says "complete the RDSPI workflow" |

**Explicitly out of scope**
- `docs/archive/**` — history.
- `docs/active/work/**`, `docs/active/tickets/**`, `docs/active/stories/**`, `docs/active/epic*/**`
  — the record of past work; ~300 files.
- `docs/knowledge/` field notes and runbooks (`codex-client/*`, `chromebook-install-test.md`,
  `vend-workflow.md`, `fresh-loop-live-startup.md`, `turbo-mode-field-experiment.md`,
  `landing-probes/*`) — S-057-01 says this ticket "does not touch `docs/knowledge/` files that
  are field notes or runbooks".

## 7. Constraints and assumptions

1. `lisa commit-ticket` stages with `git add -A -- <paths>` (`commit_transaction.rs:855`), so a
   deletion is committable through the isolated transaction by naming the removed path in
   `--include`.
2. The fixture board under `docker/chromebook-test/board/` keeps its retired `phase:` values on
   purpose — it exercises T-057-01-01's forward mapping. Only the two jargon mentions move.
3. `config.rs:524–531` accepts six phase names in `[scheduling.phase_timeouts]`. That is a
   forward-compatibility allowlist parallel to the serde aliases, not a description of a
   workflow; the resolved key still maps to `Phase::Implement`.
4. The document must end up shorter than 146 rendered lines, and its live half is ~85 lines of
   prose that must not lose meaning. The budget for new writing is small by construction.
5. `just check` = `cargo check -p lisa-plugin --target wasm32-wasip1`, `cargo fmt --check`,
   `cargo clippy`, `cargo test --workspace`.
