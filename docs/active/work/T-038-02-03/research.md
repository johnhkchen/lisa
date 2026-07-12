# T-038-02-03 Research: Final Test and WASM Verification

## Ticket boundary

- Ticket `T-038-02-03` is a high-priority task in story `S-038-02`.
- Its current phase at assignment time is `research`.
- It depends on `T-038-02-02`, which is complete at repository `HEAD`.
- The ticket asks for confirmation and recording, not a feature or repair.
- The sole acceptance criterion names two required outcomes:
  - `cargo test --workspace` passes.
  - The WASM check passes, expressed as either the explicit Cargo build or the
    repository's `just check` workflow.
- Both results must be obtained on the formatting- and Clippy-clean tree.
- The ticket advances the project principles that repository state must be
  truthful and that observed field evidence becomes durable regression evidence.

## Repository state

- The repository root is `/Users/johnchen/swe/repos/lisa`.
- The active branch is `main`.
- `HEAD` at the start of Research is `8cc053c` (`Complete T-038-02-02`).
- The immediately preceding commits completed the formatting and Clippy tickets:
  - `763f2a4` completed `T-038-02-01`.
  - `8cc053c` completed `T-038-02-02`.
- The ordinary Git index is empty at Research time.
- Two working-tree modifications exist and are owned by Lisa's orchestration:
  - `.lisa/provenance.jsonl` contains completion records for prior tickets.
  - `docs/active/tickets/T-038-02-03.md` changes the phase from `ready` to
    `research`.
- No ticket-owned source modification or untracked source file is present.
- The assignment requires those Lisa-owned modifications to remain untouched.

## Workspace layout

- The root `Cargo.toml` defines a Cargo workspace using resolver version 2.
- Workspace membership is `crates/*`.
- The workspace currently contains three product packages:
  - `lisa-core`, shared types, ticket parsing, and DAG behavior.
  - `lisa-plugin`, the Zellij WASM plugin and scheduler/UI behavior.
  - `lisa-cli`, the command-line application and embedded plugin delivery path.
- The workspace uses Rust edition 2021.
- The shared version is `0.4.0-rc.6`.
- The plugin's deployed compilation target is `wasm32-wasip1`.
- Native workspace tests cover the host-testable behavior of all members.
- The WASM check covers the target-specific compilation boundary that native
  tests cannot exercise.

## Developer command surface

- The root `Justfile` is the documented local command surface.
- Its default recipe is `check`.
- The `test` recipe runs exactly:

```text
cargo test --workspace
```

- The `check-wasm` recipe runs exactly:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

- The composite `check` recipe runs, in order:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

- Therefore `just check` is a direct repository-defined representation of both
  acceptance gates, with the WASM compilation check first and tests second.
- The `build-dev` recipe performs a non-release WASM build.
- The `build` recipe performs the release WASM build:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

- The ticket wording explicitly accepts a WASM build or `just check`.
- `cargo check` validates type checking and target compatibility without emitting
  the final release artifact.
- `cargo build` additionally exercises code generation and linking.

## Formatting and lint baseline

- `T-038-02-01` is the formatting predecessor.
- `T-038-02-02` is the zero-warning Clippy predecessor.
- The predecessor review records no source changes were required for Clippy.
- It records a passing formatting command:

```text
cargo fmt --all -- --check
```

- It records warning-strict native workspace Clippy:

```text
cargo clippy --workspace -- -D warnings
```

- It records warning-strict target-specific plugin Clippy:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

- Both Clippy invocations exited zero with no warnings.
- The predecessor also recorded 723 passing tests, zero failed tests, and one
  ignored environment-dependent integration test.
- It recorded a successful ordinary WASM `cargo check`.
- Those predecessor results establish the claimed tightened baseline, but this
  ticket still requires a fresh confirmation on its own starting tree.

## Continuous integration boundary

- `.github/workflows/ci.yml` installs the stable Rust toolchain.
- CI installs the `wasm32-wasip1` target and the `rustfmt` and `clippy`
  components.
- CI checks formatting with `cargo fmt --all -- --check`.
- CI runs warning-strict Clippy separately for core, CLI, and WASM plugin.
- CI runs `cargo test --workspace`.
- CI checks the WASM plugin with:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

- The required test and WASM check commands therefore match the CI contract.
- `.github/workflows/release.yml` goes beyond check mode and builds the release
  WASM plugin before producing distribution artifacts.
- The release build command is:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

- Passing `just check` demonstrates the normal pre-merge gate.
- Passing the release build demonstrates the deployable artifact boundary.

## Test topology observed from predecessor evidence

- The CLI unit-test binary previously reported 274 passing tests.
- `atomic_provider_contract` previously reported 1 passing integration test.
- `help_surface` previously reported 3 passing integration tests.
- `real_zellij_delivery_boundary` contains one ignored test.
- Its source annotation states that real Zellij, zsh, script, jq, and the WASM
  target are required.
- `lisa-core` previously reported 155 passing unit tests.
- `lisa-plugin` previously reported 290 passing unit tests.
- Doc-test targets exist but currently define no tests.
- The aggregate predecessor count is 723 passed, 0 failed, 1 ignored.
- A fresh command may report different counts if the tree changed, so evidence
  for this ticket must use this attempt's actual output rather than copying the
  predecessor total.

## Artifact and transaction constraints

- Phase artifacts belong only in
  `.lisa/attempts/T-038-02-03/1/work/` during this attempt.
- Lisa, not the agent, publishes admitted artifacts to
  `docs/active/work/T-038-02-03/`.
- The ticket's phase and status frontmatter are Lisa-controlled.
- Source edits, if any become necessary, must be committed through
  `lisa commit-ticket`.
- Each invocation must include exact repository-relative ticket-owned paths.
- Ordinary `git add`, broad staging, and ordinary `git commit` are prohibited.
- A verification-only result with no source changes has no meaningful source
  unit to commit.
- Test/build output under ignored Cargo target directories is generated state,
  not ticket-owned source.
- Before Review, the ordinary index and ticket-owned working tree must be clean.

## Assumptions and constraints

- The installed Rust toolchain must include `wasm32-wasip1`; predecessor success
  strongly indicates it does.
- Cargo may reuse cached dependencies and compilation units.
- Cached success still evaluates Cargo's dependency graph and exit status for the
  current source inputs.
- Running commands sequentially avoids Cargo lock coordination messages and makes
  the evidence easier to attribute.
- Formatting and Clippy should be rechecked because “on the fmt+clippy-clean tree”
  is part of the acceptance context, even though the primary criterion names only
  tests and WASM compilation.
- Read-only verification should not manufacture a source diff merely to create a
  ticket commit.
- If a required check fails, diagnostics must be investigated before any edit is
  considered.
- Any resulting fix would need to remain strictly within this ticket's acceptance
  boundary and isolated commit rules.

## Research conclusion

- The repository already defines the exact required combined gate as `just check`.
- CI independently confirms the same test and WASM-check boundary.
- The release workflow adds a stronger WASM build boundary that matches the
  ticket's alternate wording.
- The tightened baseline is present at `HEAD` via the completed formatting and
  Clippy predecessor tickets.
- No source change is currently indicated.
- The remaining work is to select a proportionate command sequence, execute it,
  preserve exact result evidence, and verify transaction hygiene.
