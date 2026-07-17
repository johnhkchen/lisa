# Plan — T-049-01-01 seal types and resolution

## Goal

Land typed completion-seal intent, pure resolution, one-time native environment
probing, config validation, and pinned runtime transport without implementing
journal completion mechanics or changing Tier 1 transaction semantics.

## Step 1 — Add core seal vocabulary

Modify `crates/lisa-core/src/completion.rs`.

Add `CompletionSeal` and `CompletionSealMode` with stable lowercase parse/display
representations.

Add typed commit-support and unavailability vocabulary.

Add immutable `ResolvedCompletionSeal` with getters only.

Add typed explicit-commit resolution error.

Implement the total pure resolution matrix.

Add unit tests for every mode/support combination.

Verify with:

`cargo test -p lisa-core completion::tests`

Success criteria:

- auto selects commit only when support is available;
- auto retains a typed journal-fallback reason;
- explicit commit cannot produce journal;
- explicit journal always produces journal;
- invalid parse text lists `auto`, `commit`, and `journal`.

## Step 2 — Transport the pinned seal into plugin configuration

Modify `crates/lisa-core/src/types.rs`.

Add `completion_seal` to `PluginConfig`.

Use commit as the legacy/default runtime value.

Parse `completion_seal` leniently from the KDL map.

Ensure `auto` and malformed values do not become runtime states.

Add tests for commit, journal, missing, auto, and invalid map values.

Run formatting and the full `lisa-core` suite.

Success criteria:

- valid pinned tiers round-trip;
- a legacy layout remains commit-sealed;
- an untrusted/malformed map cannot weaken the completion guard.

## Step 3 — Commit core domain/transport unit

Inspect `git diff` for the two core files.

Confirm no ordinary-index staging exists for either path.

Run:

`lisa commit-ticket --ticket-id T-049-01-01 --message "Add completion seal domain types" --include crates/lisa-core/src/completion.rs --include crates/lisa-core/src/types.rs`

Verify the commit result and clean status for those exact paths.

## Step 4 — Parse `[guards].completion`

Modify `crates/lisa-cli/src/config.rs`.

Add raw `GuardsConfig` input and resolved `completion_mode` intent.

Default unconfigured intent to auto.

Add known-section/key handling.

Reject unknown completion values semantically with actionable accepted values.

Add the inert generated configuration example.

Add tests for parsing, resolution defaults, each valid value, invalid value,
unknown key warnings, and generated config.

Verify with focused config tests.

Success criteria:

- missing config resolves auto;
- every documented value parses;
- unknown values fail validation rather than warn or silently default;
- unknown keys continue following repository warning convention.

## Step 5 — Implement the one-shot native resolver

Create `crates/lisa-cli/src/completion_seal.rs`.

Register it in `crates/lisa-cli/src/main.rs`.

Add the immutable per-run wrapper carrying core resolution and discovered Git
root.

Implement a `FnOnce`-based probe seam.

Short-circuit explicit journal before invoking the probe.

Implement read-only repository, email, HEAD, and Git-directory probes.

Map probe outcomes to core support reasons.

Format explicit commit errors with named preflight text and the exact two-line
identity remedy.

Add unit tests with invocation counters and typed fake outcomes.

Add focused real-probe tests only if they remain hermetic under parallel test
execution; otherwise keep system-process variability out of the unit matrix.

Success criteria:

- auto and explicit commit invoke the injected probe exactly once;
- explicit journal invokes it zero times;
- repository, identity, and transaction failures preserve their categories;
- missing-identity explicit commit contains both remedy commands.

## Step 6 — Commit CLI config/resolver unit

Format the touched CLI files.

Run focused CLI config and completion-seal tests.

Inspect the exact diff.

Run:

`lisa commit-ticket --ticket-id T-049-01-01 --message "Resolve completion seal configuration" --include crates/lisa-cli/src/config.rs --include crates/lisa-cli/src/completion_seal.rs --include crates/lisa-cli/src/main.rs`

Verify those paths are clean afterward.

## Step 7 — Make Git dependency conditional

Modify `crates/lisa-cli/src/doctor.rs`.

Parameterize shared required dependency construction with `require_git`.

Keep doctor behavior unchanged by passing true from doctor.

Allow loop to pass false for a pinned journal run.

Update existing dependency-list tests.

Add a test that provider-only dependencies exclude Git and retain the selected
agent.

Success criteria:

- commit-tier loop checks still require Git;
- journal-tier loop does not fail merely because Git is missing;
- doctor still reports Git as before.

## Step 8 — Pin and transport at loop startup

Modify `crates/lisa-cli/src/loop_cmd.rs`.

Call the resolver once on the non-dry-run branch.

Use the same result for dependency selection, repository-root transport, and
layout tier transport.

Remove the later unconditional Git-root re-discovery.

For repo-less journal mode, canonicalize the project root for the legacy path
slot.

Add the `completion_seal` KDL key.

Update layout helpers and tests to pass a seal.

Keep dry-run free of environment preflight and choose explicit mode or legacy
commit for its illustrative layout.

Add tests for both rendered tiers.

Success criteria:

- a real run has one local pinned result;
- no downstream loop code calls the probe again;
- generated layout contains only commit or journal, never auto;
- explicit journal is rendered journal even in a Git-capable environment;
- existing layout keys remain intact.

## Step 9 — Commit loop integration unit

Format all changed Rust source.

Run focused doctor and loop tests.

Inspect the exact diff and current index state.

Run:

`lisa commit-ticket --ticket-id T-049-01-01 --message "Pin completion seal at loop startup" --include crates/lisa-cli/src/doctor.rs --include crates/lisa-cli/src/loop_cmd.rs`

Verify the two paths are clean afterward.

## Step 10 — Full verification

Run:

`cargo fmt --all -- --check`

`cargo test -p lisa-core`

`cargo test -p lisa-cli`

`cargo test --workspace`

Use `just check` only if the repository's installed WASM target is available and
the command adds coverage beyond the workspace tests without requiring a source
build for installation.

Inspect `git status --short`.

Distinguish pre-existing Lisa metadata/ticket changes from ticket-owned source.

Confirm no ticket-owned source is staged, modified, or untracked.

Confirm no ordinary `git add` or `git commit` was used.

## Step 11 — Progress artifact

Write `progress.md` in the private attempt work directory.

Record completed steps, commit IDs/messages, test commands/results, and any
deviation from this plan.

If implementation uncovers a necessary design change, record it before taking
the changed course.

## Step 12 — Review

Review committed diffs and tests against every acceptance criterion.

Check the exact config error and identity remedy strings.

Check that explicit journal has no Git probe path.

Check that legacy plugin defaults preserve commit behavior.

Write `review.md` with files changed, behavior, coverage, known limitations,
and handoff notes.

Write `review-disposition.json` exactly as a passing disposition only if all
required implementation is committed and verified.

Remain on T-049-01-01 after both artifacts are present.
