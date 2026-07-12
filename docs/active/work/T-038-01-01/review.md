# Review — T-038-01-01 CLI and WASM size baseline

## Outcome

Acceptance is met. This ticket records a reproducible pre-pass release-size
baseline for the native Lisa CLI binary and the release WASM embedded into it.
The exact locked build-and-size command completed successfully twice, and both
path-specific byte counts were identical across the two executions.

## Baseline

| Artifact | Exact path | Byte count |
|---|---|---:|
| Release CLI binary | `target/release/lisa` | **3,013,904 bytes** |
| Embedded release WASM | `target/wasm32-wasip1/release/lisa.wasm` | **1,414,183 bytes** |

Both are logical file lengths reported by `wc -c`, not filesystem allocation
or human-rounded sizes.

The command also printed a 4,428,087-byte total. That sum is not a separate
product or acceptance value; it is simply the two file lengths added by `wc`.

## Exact reproduce command

Run from the repository root:

```bash
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release &&
touch target/wasm32-wasip1/release/lisa.wasm &&
cargo build --locked -p lisa-cli --release &&
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

Expected path-specific output in the recorded environment and source state:

```text
 3013904 target/release/lisa
 1414183 target/wasm32-wasip1/release/lisa.wasm
 4428087 total
```

## Reproduction evidence

Run 1 exited zero and produced:

- CLI: 3,013,904 bytes;
- WASM: 1,414,183 bytes.

Run 2 repeated the exact command verbatim, exited zero, and produced:

- CLI: 3,013,904 bytes;
- WASM: 1,414,183 bytes.

Therefore:

- CLI Run 1 equals CLI Run 2;
- WASM Run 1 equals WASM Run 2;
- the ticket's explicit rerun criterion passes for both artifacts.

## Recorded build identity

The baseline was captured against:

- Git HEAD `2f8230d1d36a264522c82112c41adeb63cadf9dd`;
- Lisa version `0.4.0-rc.6`;
- `rustc 1.99.0-nightly (c4af71034 2026-07-06)`;
- `cargo 1.99.0-nightly (2f0e7011e 2026-07-05)`;
- native host `aarch64-apple-darwin`;
- Darwin 25.5.0 on arm64;
- WASM target `wasm32-wasip1`;
- workspace release profile `opt-level = "s"`, `lto = true`;
- dependency resolution from the checked-in `Cargo.lock` via `--locked`.

This context is material for interpreting the native executable size. The
record does not claim that another OS, architecture, linker, compiler revision,
or source revision will produce a 3,013,904-byte native executable.

## Why this is the embedded-WASM count

The repository's established release chain is:

1. Cargo builds `lisa-plugin` to
   `target/wasm32-wasip1/release/lisa.wasm`;
2. `crates/lisa-cli/build.rs` copies that file byte-for-byte into its `OUT_DIR`;
3. `crates/lisa-cli/src/templates.rs` embeds the copy with `include_bytes!`;
4. the native CLI is compiled after the WASM exists and its timestamp is
   invalidated by the same touch used in the canonical `just build-cli` recipe.

The measured stable-path WASM was additionally compared with the current
build-script copy using `/usr/bin/cmp -s`; the comparison succeeded. Both files
were 1,414,183 bytes. No compression or transformation exists at the embedding
boundary.

The post-build SHA-256 values were:

```text
21364a09ca9f0b010475856c995069dd093f06c930682857c21abc40e4373449  target/release/lisa
14db37eed0fbde7507bf6da45be5edaf9b17803c6e6ee300875b68b15761c57c  target/wasm32-wasip1/release/lisa.wasm
```

Checksums are supporting artifact identity, not replacements for the requested
byte counts.

## Files created

The attempt created these private phase artifacts:

- `.lisa/attempts/T-038-01-01/1/work/research.md`;
- `.lisa/attempts/T-038-01-01/1/work/design.md`;
- `.lisa/attempts/T-038-01-01/1/work/structure.md`;
- `.lisa/attempts/T-038-01-01/1/work/plan.md`;
- `.lisa/attempts/T-038-01-01/1/work/progress.md`;
- `.lisa/attempts/T-038-01-01/1/work/review.md`.

No production, test, manifest, lockfile, build-script, recipe, or shared source
file was created, modified, or deleted by this ticket. Build outputs under
`target/` were refreshed and remain ignored generated artifacts.

## Verification coverage

### Release compilation

Both measurement runs successfully built:

- `lisa-plugin` with `--target wasm32-wasip1 --release --locked`;
- `lisa-cli` natively with `--release --locked`.

This verifies that both measured outputs are valid products of the recorded
release command rather than leftover files measured without provenance.

### Size repeatability

The identical command was executed twice. Both requested values and the raw
total matched exactly. This directly tests the sole acceptance criterion.

### Artifact identity

`file` identified:

- the CLI as a Mach-O 64-bit arm64 executable;
- the plugin as a WebAssembly MVP module.

The build-script copy compared byte-for-byte equal to the stable measured WASM.

### Repository integrity

Read-only Git inspection found no ordinary-index entries and no ticket-owned
source delta. The modified provenance and ticket files are Lisa-owned
orchestration state. Lisa also exposed the shared work directory as untracked
publication state while ingesting attempt-private artifacts; the agent did not
write there directly.

No ordinary `git add`, `git commit`, or broad index operation was used. No
`lisa commit-ticket` source transaction was necessary because there is no
ticket-owned source change.

## Test coverage assessment

No unit or integration test was added or needed: the ticket changes no behavior
and asks for direct release artifact observations. The proportionate test is
the exact producing command plus immediate rerun, supported by file-type and
byte-copy checks.

Formatting, Clippy, and workspace unit tests were not run. They cannot validate
the requested byte counts and there is no source delta for them to assess. Both
release compilation paths did execute successfully twice.

## Open concerns and limitations

- Native binary size is platform/toolchain-specific. Rerun comparison should
  use the recorded environment when evaluating exact equality.
- A future source, dependency, compiler, release-profile, or linker change is
  expected to move one or both values; that is the purpose of this before
  baseline, not a failure of it.
- `touch` forces the CLI embedding boundary to rebuild and follows the current
  canonical recipe. If the build pipeline later changes, the later after-report
  should record its then-current exact command rather than silently assuming
  this sequence.
- The WASM count represents the uncompressed embedded byte slice. It is not a
  package archive size, runtime memory measurement, or host-process footprint.
- The intervening S-038 baseline/comparison commits observed before measurement
  were documentation-only. The exact measured HEAD is nevertheless recorded so
  later comparisons can distinguish source state explicitly.

There are no critical issues, TODOs, or unmet acceptance items for this ticket.

## Final assessment

The pre-pass baseline is now locked with exact byte counts, explicit build
provenance, an exact reproduction command, and a successful immediate rerun:
3,013,904 bytes for the arm64 macOS release CLI and 1,414,183 bytes for the
embedded release WASM. The product source was left untouched. This attempt is
ready for Lisa's lease verification, artifact publication, and completion
commit, and must remain on T-038-01-01 until that gate completes.
