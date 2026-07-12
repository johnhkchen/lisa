# Review: T-031-03 provider contract and live regression

## Outcome

T-031-03 now gives Claude, Codex, generated projects, and users one consistent
atomic ticket contract. Agents commit meaningful implementation units only via
`lisa commit-ticket` with exact paths, never use the ordinary index as a staged
handoff, never edit phase/status, and remain on the current ticket after Review.
Lisa alone prepares Done, commits the ticket and all work artifacts, verifies the
completion receipt, releases the provider seat, and unblocks dependents.

The shipped workflow can upgrade clean prior Lisa installations without
overwriting project customizations. A checked-in Cargo integration regression
runs real Lisa CLI transactions in an external temporary Git repository across
five Codex routes and one Claude route, one reused logical seat, dependency
edges, and a foreign staged file. It records commit-tree, index, activity,
provenance, validation, and final-status evidence.

## Contract interpretation

The final completion commit is the one authoritative Done publication. It
introduces Done frontmatter and all six work artifacts; its tree contains the
final ticket source. Source changes themselves are made durable in earlier
exact-path isolated implementation commits.

This preserves the explicit ownership boundary from T-031-01. Collapsing all
implementation diffs into one final commit would require rewriting an interleaved
shared branch and could absorb unrelated ticket commits. The delivered contract
therefore provides one final completion commit closing a chain of isolated
ticket-owned commits, with no ordinary-index handoff at any point.

## Files modified

### `crates/lisa-cli/data/rdspi-workflow.md`

- Replaced generic incremental commit guidance with `lisa commit-ticket`.
- Requires exact repository-relative include paths.
- Forbids ordinary `git add`, broad `git add -A`, ordinary `git commit`, and
  staged handoff.
- Requires all ticket-owned source residue to be durable before Review ends.
- Requires the agent to wait after `review.md`.
- Makes Lisa's completion commit the authority for seat/dependent release.
- Explains fail-closed retry behavior.
- Corrects concurrency wording to isolated alternate-index transactions.

### `docs/knowledge/rdspi-workflow.md`

- Updated to byte-identical current bundled workflow content.
- Gives Lisa's own dogfood project the same generated contract.

### `crates/lisa-cli/src/templates.rs`

- Registers the outgoing v0.4 six-phase workflow as a known Lisa template.
- Retains the older v0.2 workflow migration fixture.

### `crates/lisa-cli/src/init.rs`

- Adds coverage that every known prior workflow upgrades to current content.
- Requires every legacy fixture to remain distinct from current content.
- Existing unknown/customized workflow safety-skip coverage remains intact.

### `crates/lisa-plugin/src/lib.rs`

- Aligns common Claude/Codex initial and reuse prompt text with the isolated
  commit contract.
- Aligns common Review finish-up text with frontmatter/index prohibitions and
  the completion-confirmation wait.
- Resolves the real descriptive ticket filename from the ticket scan.
- Retains `<ticket-id>.md` only as a fallback when no scan result is available.
- Adds phrase-level atomic contract tests.
- Adds descriptive-filename prompt regression coverage.
- Adds focused finish-up contract coverage.

### `README.md`

- Corrects the workflow from five to six phases.
- Adds Review and atomic completion sections.
- Documents exact-path implementation commits and foreign stage preservation.
- Documents commit-confirmed seat/dependent release.
- Documents recovery after a final completion failure.
- Replaces the misleading lock-only concurrency description.

### `docs/knowledge/lisa-loop-setup-guide.md`

- Makes the lifecycle provider-neutral for Claude and Codex.
- Corrects context/workflow paths.
- Removes normal-flow agent/human phase-frontmatter instructions.
- Documents isolated implementation commits.
- Documents Lisa-owned Done completion and recovery.
- Adds `review.md` to the artifact inventory.

## Files created

### `crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.md`

- Exact bytes of the outgoing pre-T-031-03 workflow.
- Allows S-030 ownership classification to upgrade clean installs safely.

### `docs/active/work/T-031-03/harness/run.sh`

- Creates a real temporary repository outside the Lisa checkout.
- Runs `lisa init` and `lisa validate`.
- Stages a foreign file before any ticket transaction.
- Processes five Codex tickets and one Claude ticket through `seat-1`.
- Requires T-CDX-01 completion before T-CDX-05 starts.
- Requires T-CDX-05 completion before the mixed Claude ticket starts.
- Uses real `commit-ticket` for source and `complete-ticket` for Done/artifacts.
- Asserts exact ordinary-index tuple preservation after every transaction.
- Asserts foreign exclusion from every ticket commit.
- Asserts Done first appears in each completion commit.
- Asserts all source/artifact paths exist in completion trees.
- Asserts no loop-owned residue remains.
- Records diagnostic evidence outside the fixture repository.

### `docs/active/work/T-031-03/harness/README.md`

- Documents direct/retained invocation.
- Distinguishes real Git effects from deterministic provider/seat modeling.
- Describes each evidence file and invariant.

### `crates/lisa-cli/tests/atomic_provider_contract.rs`

- Runs the checked-in harness with Cargo's built Lisa executable.
- Makes the regression part of `cargo test --workspace` and `just check`.
- Includes captured stdout/stderr in assertion failures.

### RDSPI artifacts

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`

No file was deleted. This session did not manually edit ticket phase/status.

## End-to-end regression behavior

The harness creates these routed transactions:

1. T-CDX-01 — Codex, seat-1, independent.
2. T-CDX-02 — Codex, seat-1, reused.
3. T-CDX-03 — Codex, seat-1, reused.
4. T-CDX-04 — Codex, seat-1, reused.
5. T-CDX-05 — Codex, seat-1, waits for T-CDX-01 completion hash.
6. T-MIX-01 — Claude, seat-1, waits for T-CDX-05 completion hash.

Each ticket produces one isolated implementation commit and one final completion
commit. Before a dependent start row is recorded, the prerequisite hash must
exist as a commit and be an ancestor of current HEAD. Every completion verifies
its parent was not Done and its committed ticket is Done.

The retained implementation run produced six implementation and six completion
hashes. It recorded five Codex starts plus one Claude start, all on `seat-1`.
T-CDX-01 confirmation preceded T-CDX-05 start; T-CDX-05 confirmation preceded
T-MIX-01 start.

The foreign stage tuple remained exactly:

```text
100644 e4199730783d26c93fe57610bd955b2bf3cf7248 0 foreign.txt
```

Final fixture status contained only `M  foreign.txt`; all source, ticket, and
artifact paths were clean. Evidence for the retained run is outside the source
tree at the path recorded in `progress.md`.

## Acceptance criteria assessment

### Consistent provider contract

Met. Common prompt functions cover Claude/Codex initial, reuse, and finish-up
delivery. The bundled workflow states the same rules.

### Isolated agent-side Git operations

Met. Generated instructions require `lisa commit-ticket` with exact includes and
explicitly forbid ordinary-index and broad staging behavior.

### Customized workflow protection

Met. The outgoing exact bytes are a known legacy template; unknown content
continues to produce a safety skip.

### Five Codex tickets, reuse, dependency, foreign stage

Met by the checked-in deterministic external-repository harness. Five Codex
routes use one logical seat; dependency start is commit-gated; the same foreign
stage tuple survives all twelve ticket transactions.

### Done/artifact/code durability and no residue

Met. Each Done blob first appears in its completion commit, all six artifacts
exist there, final source exists in the tree, scoped status is clean, and only
the foreign staged entry remains.

### Mixed provider

Met. A Claude-routed dependent executes the same exact transaction/assertion
driver after the five Codex routes.

### Diagnostic evidence

Met. The harness records tree listings, ticket blobs, hashes, index snapshots,
activity rows, provenance rows, init/validate output, and final status.

### User-facing atomicity and recovery

Met in README and the setup guide.

### Required verification

Met; details follow.

## Test coverage

### Focused

- Known workflow generation upgrades: passed.
- Unknown customized workflow preservation: passed.
- Three ticket prompt/path tests: passed.
- Finish-up contract test: passed.
- Atomic provider-contract integration: passed.
- Direct retained harness execution: passed.

### Full

- `cargo fmt --all -- --check`: passed.
- `lisa validate`: passed; 16 tickets, 1 ready, valid DAG.
- Plugin Clippy with `-D warnings`: passed.
- Workspace suite: 268 CLI unit, 1 new integration, 147 core, and 238 plugin
  tests passed; doc tests passed.
- WASM release build: passed.
- `just check`: passed, including WASM check and full workspace tests.

## Commits

- `2cb0689` — atomic workflow/provider contract, upgrade history, prompt path.
- `9367154` — six-ticket external-repository harness and Cargo integration.
- `9edc92b` — user atomicity and recovery documentation.

All implementation commits used `lisa commit-ticket` with exact include paths.

## Open concerns and limitations

### Provider UI execution is deterministic, not paid/live

The checked-in harness executes real Git and Lisa processes but models provider
route and seat events; it does not launch Claude/Codex TUIs or Zellij. This is
intentional so CI needs no credentials, network, terminal host, or nondeterministic
model behavior. T-031-02 plugin tests cover pending Codex seat retention,
successful release, dependency gating, and mixed provider routing in scheduler
state. A future manual release qualification may still run the same scenario in
interactive Zellij for UI/hook confidence.

### Implementation commits remain separate

The final completion commit does not repeat source diffs already committed by
`lisa commit-ticket`; it contains their final tree. This is necessary on an
interleaved shared branch and should remain explicit in future documentation.

### Reusable lock ignore

The transaction intentionally leaves `.lisa-commit.lock` as a reusable inode.
The harness adds it to the fixture baseline `.gitignore`, matching this
repository's convention. Projects should keep that runtime lock ignored.

## Critical issues

None found. No test, validation, Clippy, WASM, residue, index-preservation, or
dependency-order failure remains.

## Reviewer focus

1. Whether the final-commit interpretation matches desired shared-branch history.
2. Prompt wording around exact `--include` ownership and the terminal wait.
3. `ticket_prompt` real-path discovery and fallback behavior.
4. Legacy workflow ownership registration and customization preservation.
5. Harness separation between real transaction effects and modeled provider seat.
