# Review: ownership-aware init planning

## Outcome

The init planner now preserves project-owned or unclassifiable static files by
default. Whole-file replacement occurs only when existing bytes exactly match a
known prior Lisa template compiled into the CLI. The reported workflow and hook
clobber class is covered through both planning and real `run_init` execution.

No ticket frontmatter was modified. This `review.md` is the final RDSPI artifact.

## Production changes

### Shared replacement authorization

`crates/lisa-cli/src/init.rs` now contains `plan_owned_template`, the single
planner gate for static whole-file templates. Its outcomes are:

- absent target: create the current template;
- exact current bytes: skip as already up to date;
- exact known-prior bytes: update to the current template;
- unknown readable bytes: skip with
  `preserved: content is not a known Lisa template`;
- read/non-UTF-8 failure: skip with
  `preserved: existing file is unreadable`.

The helper returns `InitAction` only and performs no writes. `run_init` retains
its existing plan-then-execute architecture.

### Complete static-template coverage

The helper now governs every plain-text template target that was previously
eligible for whole-file replacement:

- `docs/knowledge/rdspi-workflow.md`;
- `.lisa/hooks/on-idle.sh`;
- `.lisa/hooks/on-stop.sh`;
- `.lisa/hooks/on-clear.sh`;
- `.lisa/hooks/on-heartbeat.sh`;
- `.lisa/hooks/on-notify.sample`;
- `.lisa/.gitignore`.

This removes the previous error arms where a different file or failed read
silently became `UpdateFile(current_template)`.

### Historical template evidence

`crates/lisa-cli/src/templates.rs` now exposes crate-private known-prior slices
next to current templates. Distinct historical content includes:

- the pre-v0.2.3 RDSPI workflow;
- the v0.3 stop hook before usage capture;
- the v0.3 clear and heartbeat hooks before client-neutral wording;
- the v0.3 one-line Lisa gitignore.

Idle and notification templates have explicit empty prior slices because their
relevant v0.3 bytes already equal current content and take the current no-op
branch.

The larger historical workflow lives at
`crates/lisa-cli/data/legacy/rdspi-workflow-v0.2.md`. Its Git blob hash was
verified against tag v0.2.0 as
`cbe4974f4acbc1348be06219928b67ad22c56cd2`.

## Explicit path policy review

Every path family considered by `plan_init_actions` has a defined policy:

| Path family | Policy | Review result |
|---|---|---|
| active/archive documentation directories | create if absent | unchanged |
| `.lisa/hooks`, `.lisa/signals` directories | create if absent | unchanged |
| `CLAUDE.md`, `AGENTS.md` | preserve if present | unchanged and tested |
| workflow and five hook templates | replace if proven pristine | implemented |
| `.lisa/.gitignore` | replace if proven pristine | implemented pending T-030-02 |
| `.lisa.toml` | format-aware preserving text merge | unchanged and tested |
| Claude settings JSON | format-aware hook merge | unchanged and tested |
| Codex hooks JSON | format-aware hook merge | unchanged and tested |

Fresh initialization still produces eight directories and twelve files. Existing
current templates remain no-ops.

## Test coverage added or revised

### Field regression

`test_init_preserves_project_modified_plain_text_byte_for_byte` constructs a
workflow containing project-only Story Layer/read-the-story guidance. It also
includes:

- a locally extended historical stop hook;
- a locally extended notification sample;
- a Lisa gitignore containing `hooks/ntfy-topic`.

The test asserts preservation actions during planning, confirms planning did not
mutate the workflow, runs real non-dry init, and byte-compares every fixture
afterward.

### Ownership classification

- All seven current plain-text targets are asserted as no-ops.
- Known-prior workflow, stop, clear, heartbeat, and gitignore contents are
  asserted as `UpdateFile` actions.
- A real init run upgrades a known-prior stop hook to current bytes.
- Arbitrary content in each of the five hook targets is asserted as a safety
  skip rather than an update.
- Arbitrary workflow content is asserted as a safety skip.

### Failure behavior

- Invalid UTF-8 workflow and hook bytes deterministically exercise the
  `read_to_string` failure branch and assert no update action.
- Malformed Claude and Codex JSON are asserted as specific skip actions with no
  update fallback.
- Malformed TOML is processed only by the existing preserving textual merge; the
  regression asserts its original project content survives rather than being
  replaced by default config.

### Existing coverage retained

- fresh Rust/Node initialization;
- init/validate round trips;
- create-only context-file preservation;
- current config and stale-version config behavior;
- TOML missing-key upserts with custom values/comments;
- Claude/Codex JSON custom-hook preservation and idempotence;
- malformed validation inputs;
- hook permissions and scaffold content.

## Verification results

- Focused init suite after final test: 65 passed, 0 failed.
- Full `cargo test --workspace`: 626 tests passed before the last test-only
  addition (247 CLI, 145 core, 234 plugin), with no doc-test failures.
- `just check`: passed, including the wasm32-wasip1 plugin check and full tests.
- `cargo fmt --all`: completed successfully.
- `git diff --check`: passed.

## Lint status

Warning-strict clippy is not green on the existing baseline:

- workspace/all-targets reports twelve old `unnecessary_to_owned` findings in
  `lisa-core/src/dag.rs` tests;
- CLI/all-targets reports one old needless borrow around `&format!` in an
  existing config-upsert test in `init.rs`.

Git blame confirms those lines predate this ticket. No clippy diagnostic points
to the new ownership code or tests. The findings were left out of scope to avoid
mixing unrelated cleanup into the safety fix.

## Commits

- `0e67320` — research, design, structure, and plan artifacts.
- `d2d06b3` — historical registry, ownership-aware planner, focused regressions,
  and implementation progress.
- A final scoped verification/handoff commit contains the last malformed-input
  regression plus closing progress/review updates.

## Open concerns and limitations

### Registry maintenance is required

When a static bundled template changes, maintainers must retain the outgoing
distinct bytes in the appropriate legacy slice. Forgetting that step is safe but
conservative: pristine older installs will be preserved rather than upgraded.
It cannot cause an unknown file overwrite.

### Historical coverage begins from available tagged content

The registry covers distinct release content found in local tags relevant to the
current upgrade. An installation from an untagged development build may be
unclassified and therefore preserved. This is the intended safety bias.

### Gitignore follow-up

This ticket makes `.lisa/.gitignore` replacement ownership-aware, so custom rules
are safe and the known v0.3 one-line file can upgrade. T-030-02 still owns the
stronger append-only merge and exact mutation report promised by story S-030.

### Chmod is separate from byte preservation

On Unix, `run_init` continues to apply executable permissions to active hook
paths even when content is skipped. The ticket's byte-loss regression is fully
protected, but projects intentionally keeping an active hook non-executable may
still see its mode made executable. This was existing behavior and is outside
the stated ownership-of-content scope.

### Path type handling

The planner uses the existing `Path::exists` convention. Existing directories or
non-UTF-8 files at template paths are read failures and preserved. Dangling
symlink semantics were not expanded in this ticket and may merit separate
hardening if init must treat symlinks as a distinct ownership category.

## Human review focus

- Confirm the known-prior literals are the intended supported historical set.
- Confirm T-030-02 will replace only the gitignore helper call with append-only
  behavior while retaining preservation of unrelated rules.
- Decide whether active-hook chmod-on-skip should become a later ownership-aware
  permission policy.

## Final assessment

The critical destructive fallback is removed across the complete static init
action set. Project modifications remain the source of truth, ownership is proven
from exact file bytes rather than mutable metadata, known pristine releases can
still upgrade, and fresh/current initialization behavior remains compatible.
No critical issue remains for this ticket.
