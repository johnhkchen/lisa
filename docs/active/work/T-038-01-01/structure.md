# Structure — T-038-01-01 CLI and WASM size baseline

## Change topology

This ticket creates RDSPI evidence documents only. It makes no changes to Rust
source, Cargo manifests, the lockfile, build recipes, tests, shared ticket
frontmatter, or previously published work.

All created files initially live under:

`.lisa/attempts/T-038-01-01/1/work/`

Lisa, not this agent, owns admission and publication to:

`docs/active/work/T-038-01-01/`

## Files created

### `research.md`

Purpose: describe the existing release build and embedding chain.

Contents:

- ticket and parent-story scope;
- workspace release outputs;
- `justfile` build ordering;
- `build.rs` copy behavior and empty-placeholder fallback;
- `include_bytes!` embedding boundary;
- runtime extraction relationship;
- byte-measurement semantics;
- source/toolchain/host context;
- existing worktree state and ownership boundaries;
- constraints relevant to reproducibility.

No proposed implementation appears in this artifact beyond its descriptive
research conclusion.

### `design.md`

Purpose: compare measurement strategies and select one.

Contents:

- goals and non-goals;
- canonical `just` recipe option;
- explicit locked Cargo sequence option;
- stale-target-only option;
- clean-build option;
- runtime extraction option;
- native-binary parsing option;
- chosen command shape;
- repeatability and failure handling;
- evidence placement and rationale.

The selected approach is the explicit, fail-fast, locked build sequence followed
by `wc -c`.

### `structure.md`

Purpose: define this ticket's files, their internal responsibilities, and the
artifact/data boundaries before execution.

This file is itself part of that blueprint.

### `plan.md`

Purpose: specify the ordered measurement and verification procedure.

Contents:

- preflight state capture;
- first exact build-and-size execution;
- second identical execution;
- equality verification;
- progress evidence construction;
- repository integrity checks;
- review handoff construction;
- stopping condition.

### `progress.md`

Purpose: serve as the ticket's primary implementation/measurement record.

Planned sections:

1. outcome;
2. source and environment identity;
3. exact reproduction command;
4. first execution output;
5. second execution output;
6. equality assessment;
7. interpretation of each path-specific value;
8. scope and deviations;
9. repository integrity;
10. remaining work.

This is where the acceptance criterion's concrete byte counts will be recorded.

### `review.md`

Purpose: provide the final human-readable handoff.

Planned sections:

- acceptance outcome;
- baseline table;
- exact reproduce command;
- repeatability result;
- files created and files intentionally unchanged;
- test/verification coverage;
- open concerns and limitations;
- repository integrity;
- final assessment.

## Files read but not modified

### Build definition

- `Cargo.toml`
- `Cargo.lock`
- `justfile`
- `crates/lisa-cli/Cargo.toml`
- `crates/lisa-cli/build.rs`
- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-cli/src/loop_cmd.rs`
- `crates/lisa-plugin/Cargo.toml`

These define the release profile, dependency graph, output paths, copy step,
compile-time embedding, and runtime use of the embedded bytes.

### Project and assignment context

- `AGENTS.md`
- `CLAUDE.md`
- `docs/knowledge/rdspi-workflow.md`
- `docs/active/tickets/T-038-01-01.md`
- `docs/active/stories/S-038-01.md`
- `.lisa/attempts/T-038-01-01/1/work/assignment.md`

These define process, ownership, acceptance, and publication constraints.

### Prior work context

- `docs/active/work/T-037-02-01/review.md`
- commit summaries after the predecessor through current HEAD.

These establish the preceding release-candidate context and confirm that the
intervening observed commits are documentation-only.

## Generated build outputs

The implementation command may update ignored files under `target/`. These are
build products, not ticket-owned source changes and are not committed.

### `target/wasm32-wasip1/release/lisa.wasm`

Producer:

`cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release`

Role:

- canonical release plugin output;
- source copied by the CLI build script;
- measured definition of embedded-WASM byte count.

### CLI build-script `OUT_DIR/lisa.wasm`

Producer:

`crates/lisa-cli/build.rs`

Role:

- byte-for-byte copy of the canonical release plugin output;
- input consumed by `include_bytes!`.

Its hashed target path is intentionally not recorded as the public measurement
path because Cargo internals may change the directory hash. The stable source
path supplies the same bytes and is easier to reproduce.

### `target/release/lisa`

Producer:

`cargo build --locked -p lisa-cli --release`

Role:

- canonical native release CLI output;
- contains the copied WASM byte slice;
- measured definition of release CLI-binary byte count.

## Component boundary

The measurement boundary is intentionally at complete artifact files:

```text
lisa-plugin source
  -> Cargo wasm32-wasip1 release build
  -> target/wasm32-wasip1/release/lisa.wasm
  -> lisa-cli build.rs byte-for-byte copy
  -> include_bytes!(OUT_DIR/lisa.wasm)
  -> Cargo native release build
  -> target/release/lisa
```

The ticket records the lengths of the two stable-path artifact nodes. It does
not introduce a new API, module, test harness, script, or machine-readable
schema.

## Command boundary

One shell command chain owns each measurement execution. Every build operation
is connected with `&&`; `wc -c` runs only after both builds and the timestamp
invalidation succeed.

The command has no output redirection into tracked paths. Its stdout is captured
by the agent and transcribed into `progress.md` after both runs.

The size output contains three lines:

- CLI path and bytes;
- WASM path and bytes;
- total of both file lengths.

Only the first two are acceptance values. The total is retained as raw command
output but has no independent semantic role.

## Ownership and commit boundary

There are no ticket-owned source units to commit. Therefore:

- no ordinary Git index command is used;
- no ordinary `git commit` is used;
- no `lisa commit-ticket` source transaction is needed;
- the RDSPI artifacts remain private for Lisa's completion process.

The existing modified ticket and provenance files are Lisa-owned orchestration
state. They remain untouched.

## Verification boundary

The acceptance test is observational rather than a code test:

1. exact build-and-size command exits zero;
2. both outputs have the expected file types;
3. both path-specific byte counts are positive;
4. identical second execution exits zero;
5. second path-specific byte counts equal the first;
6. no ticket-owned source file is modified, staged, or untracked.

Full workspace tests, Clippy, and formatting checks are outside this
measurement-only slice because no source changes are made. Release compilation
of both measured products is itself part of the evidence.

## Execution ordering

1. Complete Research, Design, Structure, and Plan artifacts.
2. Capture pre-measurement HEAD and toolchain identity.
3. Execute the selected command once.
4. Execute the exact same command again.
5. Compare path-specific values.
6. Inspect file types and repository status.
7. Write `progress.md` with raw evidence and interpretation.
8. Write `review.md` with the acceptance handoff.
9. Stop on this ticket for Lisa's completion gate.
