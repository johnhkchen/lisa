# Plan: T-031-03 provider contract and live regression

## Goal

Ship one provider-neutral atomic ticket contract across generated workflow text,
Claude/Codex initial/reuse/follow-up prompts, safe init upgrades, a deterministic
six-ticket external-repository regression, and user-facing recovery guidance.

## Step 1: preserve the outgoing workflow template

- Copy the current bundled six-phase workflow bytes into
  `crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.md`.
- Verify the new legacy file is byte-identical to the pre-change bundled file.
- Do this before modifying the current template.

Verification:

- `cmp` succeeds before the current file changes.
- The legacy file contains Review and the old generic incremental commit line.

Atomic unit:

- Legacy ownership fixture plus template registration may land with the workflow
  contract update because neither is safe independently in a release.

## Step 2: update the bundled atomic workflow contract

- Rewrite the Implement commit guidance around `lisa commit-ticket`.
- Require exact repository-relative `--include` paths.
- Forbid ordinary-index `git add`, broad `git add -A`, ordinary `git commit`,
  and staged handoff between commands.
- Require the agent to leave no ticket-owned staged or unstaged residue before
  Review completion.
- Explain that Review ends by waiting for Lisa.
- Explain that Lisa owns Done preparation, final artifact commit, seat release,
  and dependent release.
- Clarify concurrency wording around alternate-index transaction locking.

Verification:

- `rg` finds the isolated command and prohibitions in the bundled file.
- Generic “commit incrementally” no longer stands alone as actionable guidance.
- No instruction tells the agent to edit frontmatter.

## Step 3: synchronize the repository-installed workflow

- Apply the same content to `docs/knowledge/rdspi-workflow.md`.
- Keep the files byte-identical.

Verification:

- `cmp crates/lisa-cli/data/rdspi-workflow.md docs/knowledge/rdspi-workflow.md`.

## Step 4: register ownership-aware upgrade history

- Add the v0.4 legacy include to `LEGACY_RDSPI_WORKFLOWS`.
- Keep the old v0.2 entry.
- Update init tests so every legacy template is exercised.
- Add an explicit current-versus-outgoing distinction assertion.
- Retain the unknown customized-content safety-skip test.

Verification:

- Focused init tests pass.
- Exact outgoing content plans an `UpdateFile`.
- Arbitrary customized content remains a `SafetySkip`.
- New current content remains a `NoOp`.

## Step 5: update the common initial/reuse prompt

- Extend `ticket_prompt` with the isolated implementation commit command.
- State exact include ownership.
- State ordinary-index prohibitions.
- State that ticket-owned changes must not remain staged or unstaged.
- Preserve the existing artifact and frontmatter rules.
- Require waiting for Lisa's completion confirmation before new work.

Verification:

- The prompt still contains real ticket/context/workflow paths.
- Claude and Codex context filenames remain correctly parameterized.
- Prompt wording covers every acceptance-criteria prohibition.

## Step 6: update the common finish-up prompt

- Preserve the required Review handoff fields and exact review path.
- Repeat the phase/status prohibition.
- Forbid independent ordinary-index completion publication.
- Tell the agent to remain on the ticket until Lisa confirms completion.

Verification:

- Both provider adapters still return `FollowUp::TypeIntoPane` with the common
  prompt.
- Existing Codex live-TUI composition tests remain green.

## Step 7: strengthen prompt regressions

- Extend common prompt content tests with safety phrase assertions.
- Add a focused finish-up content test.
- Add provider reuse/follow-up phrase assertions only where needed to make all
  four initial/reuse/follow-up paths explicit.
- Avoid full-string snapshots that would make harmless prose changes costly.

Verification:

- Run prompt and adapter focused tests.
- Confirm both Claude and Codex reuse prompts contain `lisa commit-ticket`.
- Confirm both follow-ups require waiting for Lisa.

## Step 8: scaffold the external-repository harness

- Create `docs/active/work/T-031-03/harness/run.sh` with strict shell options.
- Accept `LISA_BIN`, `--root`, and `--keep`.
- Create separate fixture repository and evidence directories.
- Configure a local Git identity.
- Scaffold the Lisa-required project structure without touching the source repo.
- Create a baseline commit and one foreign staged modification.

Verification:

- The fixture path is outside the Lisa checkout.
- `lisa validate` passes before transaction execution.
- Initial foreign stage tuple is captured.

## Step 9: define six routed fixture tickets

- Add five Codex tickets and one Claude ticket.
- Give T-CDX-05 a dependency on T-CDX-01.
- Give T-MIX-01 a dependency on T-CDX-05.
- Use real frontmatter and descriptive filenames.
- Route all through logical `seat-1` in the activity sequence.

Verification:

- Ticket scan validates all dependencies.
- Exactly five fixture tickets contain `agent: codex`.
- Exactly one contains `agent: claude`.

## Step 10: implement ticket transaction driver

- Before a ticket start, assert every prerequisite completion hash is an
  ancestor of current HEAD.
- Record route/seat/start activity.
- Create a unique source change.
- Call `lisa commit-ticket` with only that source path.
- Record implementation commit hash.
- Create research, design, structure, plan, progress, and review artifacts.
- Record completion pending.
- Call `lisa complete-ticket` with the real ticket and work paths.
- Record completion confirmation and provenance.

Verification after each CLI call:

- Exit status is zero and stdout is a Git hash.
- Foreign ordinary stage tuple remains identical.
- Foreign path is absent from the commit diff.
- Scoped ticket-owned status is clean at completion.

## Step 11: add completion content assertions

- Require Done phase/status in the completion commit ticket blob.
- Require non-Done parent ticket blob.
- Require all six artifacts in the completion commit.
- Require final source in the completion commit tree.
- Require completion to descend from its implementation commit.
- Capture per-ticket tree evidence.

Verification:

- A deliberate local assertion failure produces actionable ticket/hash output.
- Evidence includes sufficient paths and hashes to replay Git inspection.

## Step 12: add aggregate scheduling/provider assertions

- Require five Codex starts on one seat.
- Require one Claude start through the same driver.
- Require T-CDX-01 confirmed before T-CDX-05 starts.
- Require T-CDX-05 confirmed before T-MIX-01 starts.
- Require exactly one provenance row per ticket.
- Require no loop-owned staged, unstaged, or untracked paths.
- Require foreign stage tuple byte equality at the end.

Verification:

- Activity/provenance counts and ordering checks pass.
- Final status evidence shows only the expected foreign staged entry.

## Step 13: document and integrate the harness

- Write `harness/README.md` with scope, invocation, evidence, and limitations.
- Add `crates/lisa-cli/tests/atomic_provider_contract.rs`.
- Resolve `CARGO_BIN_EXE_lisa` and pass it as `LISA_BIN`.
- Run the shell harness and include captured streams in test failures.

Verification:

- The focused integration test executes the real built binary.
- No Codex/Claude binary, credentials, network, or Zellij is required.
- `cargo test --workspace` automatically includes the harness.

## Step 14: update README

- Correct workflow count to six and add Review.
- Replace generic incremental commit wording with the isolated command contract.
- Explain ordinary index preservation and final completion confirmation.
- Explain dependent/seat gating.
- Add commit-failure recovery guidance.
- Correct concurrency text that implies locking alone makes ordinary Git safe.

Verification:

- README contains the atomic guarantee and recovery behavior.
- README no longer tells agents to use generic incremental commits.

## Step 15: update setup/workflow guide

- Correct Agent Lifecycle to artifact-driven transitions.
- Correct implementation Git instructions.
- Correct completion ownership.
- Add Review to artifact tree.
- Add atomic completion/recovery subsection.
- Preserve unrelated setup material.

Verification:

- No lifecycle step tells the agent to set phase or Done.
- The guide explains how foreign staged changes behave.
- The guide explains retry after completion failure.

## Step 16: focused formatting and tests

Run:

- `cargo fmt --all -- --check` after formatting;
- focused Lisa CLI init/template tests;
- focused plugin prompt/adapter tests;
- focused atomic provider contract integration test;
- direct harness run with retained evidence once for inspection;
- `git diff --check` on ticket-owned paths.

Fix ticket-caused failures before broad verification.

## Step 17: required broad verification

Run:

- `cargo run -q -p lisa-cli -- validate`;
- `cargo test --workspace`;
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`;
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`;
- `just check`.

Record exact pass counts or command outcomes in `progress.md` and `review.md`.

## Step 18: commit discipline

This working tree contains unrelated user changes. Never use broad staging.

Meaningful implementation units:

1. workflow/prompt contract plus ownership tests;
2. external-repository harness and integration test;
3. user documentation;
4. final RDSPI progress/review artifacts.

Use `lisa commit-ticket` with exact paths if committing during the pass. Do not
use the ordinary index and do not include unrelated worktree files.

## Step 19: finalize handoff

- Update `progress.md` throughout implementation.
- Record deviations before taking a materially different path.
- Write `review.md` with file inventory, behavior, test coverage, and concerns.
- Do not change the ticket's phase or status fields.
- Stop after `review.md`; Lisa owns completion and subsequent scheduling.

## Failure handling

- If legacy bytes were not captured before current-template edits, recover them
  from the committed parent rather than reconstructing by memory.
- If the harness reveals foreign index drift, treat it as critical and stop the
  transaction sequence; never weaken the assertion.
- If `lisa validate` rejects Review-phase fixtures for having no ready ticket,
  keep one independent ready sentinel or validate before changing fixture phase.
- If shell portability differs, restrict the harness to documented Bash and use
  Git plumbing rather than platform-specific utilities.
- If full tests fail due to unrelated dirty changes, isolate and report the
  baseline only after focused ticket tests pass.

## Done checklist

- [ ] Bundled workflow has isolated atomic contract.
- [ ] Installed workflow matches bundled bytes.
- [ ] Clean outgoing workflow upgrades safely.
- [ ] Customized workflow remains protected.
- [ ] Claude/Codex initial and reuse prompts align.
- [ ] Claude/Codex finish-up prompts align.
- [ ] Five-Codex reused-seat harness passes.
- [ ] Dependency commit gate is asserted.
- [ ] Mixed-provider invariant is asserted.
- [ ] Foreign staged entry survives unchanged.
- [ ] Commit-tree/index/activity/provenance evidence is recorded.
- [ ] README and setup guide explain guarantee and recovery.
- [ ] `lisa validate` passes.
- [ ] Focused and workspace tests pass.
- [ ] WASM release build passes.
- [ ] Plugin Clippy passes.
- [ ] Review handoff is complete.
