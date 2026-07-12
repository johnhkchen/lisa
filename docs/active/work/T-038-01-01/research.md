# Research — T-038-01-01 CLI and WASM size baseline

## Ticket boundary

The ticket asks for a reproducible pre-pass size baseline for two release
artifacts:

1. the native Lisa CLI binary; and
2. the WebAssembly plugin embedded in that CLI.

The acceptance criterion requires the byte count for each artifact and the
exact build and size command that produced those counts. The parent story,
`S-038-01`, classifies this as a measurement-only slice. It explicitly limits
the work to per-ticket work artifacts and excludes source changes.

This attempt begins with the ticket in `phase: research`. Lisa owns ticket
phase/status transitions and publication from the private attempt directory.

## Workspace and release outputs

The workspace contains three crates:

- `lisa-core`, the shared model and DAG logic;
- `lisa-plugin`, the Zellij WebAssembly plugin;
- `lisa-cli`, the native `lisa` command.

The workspace release profile is declared in the root `Cargo.toml`:

- `opt-level = "s"`;
- `lto = true`.

The native release CLI output is:

`target/release/lisa`

The release WASM output is:

`target/wasm32-wasip1/release/lisa.wasm`

The latter name follows the `lisa-plugin` library output name rather than the
package's hyphenated spelling.

## Build ordering and embedding

The root `justfile` defines `build-cli` as a dependency on `build`. The `build`
recipe runs:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`

After that completes, `build-cli` touches the WASM output and runs:

`cargo build -p lisa-cli --release`

The touch is part of the established build path because the CLI build script
must be rerun when the already-built WASM changes.

`crates/lisa-cli/build.rs` locates the workspace root and reads:

`target/wasm32-wasip1/release/lisa.wasm`

It copies that file to `OUT_DIR/lisa.wasm` and emits a
`cargo:rerun-if-changed` directive for the source path. If the release WASM is
absent, the build script writes an empty placeholder instead. Therefore build
ordering is material: building the CLI alone is not sufficient evidence that
the intended plugin bytes were embedded.

`crates/lisa-cli/src/templates.rs` declares `PLUGIN_WASM` with `include_bytes!`
against the build script's `OUT_DIR/lisa.wasm`. The CLI therefore contains the
copied plugin byte slice at compile time.

At runtime, `crates/lisa-cli/src/loop_cmd.rs` writes `PLUGIN_WASM` to a
content-hash-named temporary `.wasm` file. No compression or transformation is
applied at that boundary. The release WASM output is consequently the direct,
measurable source of the embedded byte slice.

## Measurement semantics

The requested unit is bytes, not filesystem allocation blocks or a
human-formatted size. A suitable size command must report logical file length.
`wc -c` does so and is available in the current macOS environment.

The two paths must be named explicitly in the recorded command. This prevents
accidentally measuring a debug artifact, a copied installation, a temporary
runtime extraction, or the `lisa-plugin` directory rather than its output.

The build and measurement need to be one command sequence with fail-fast
semantics. Otherwise a failed build could leave stale target artifacts that
still produce plausible byte counts.

Cargo's `--locked` flag makes dependency resolution honor the checked-in
`Cargo.lock`. It does not by itself make native output portable across host
architectures or compiler versions, but it narrows the build inputs within the
recorded environment.

## Reproducibility inputs

The source state observed during Research is:

- repository HEAD: `2f8230d` (`Complete T-038-03-01`);
- package version: `0.4.0-rc.6`;
- host target: `aarch64-apple-darwin`;
- Rust compiler: `rustc 1.99.0-nightly (c4af71034 2026-07-06)`;
- Cargo: `cargo 1.99.0-nightly (2f0e7011e 2026-07-05)`;
- release WASM target: `wasm32-wasip1`;
- operating system/kernel: Darwin 25.5.0 on arm64.

Native executable bytes can vary across host targets, linkers, compiler
versions, dependency graphs, build flags, and source revisions. WASM bytes can
vary across compiler versions, dependency graphs, flags, and source revisions.
The baseline must therefore be read as a snapshot of the listed build inputs,
not as a cross-platform universal constant.

The immediately preceding commits for the other S-038 baseline tickets and
T-038-03-01 contain only ticket/work documentation. Inspection of their commit
stats shows no Rust, manifest, or build-script changes. Thus they do not alter
the release artifact inputs relative to the source pass that this story is
measuring, although the exact HEAD is still recorded.

## Existing repository state

At Research time, the ordinary worktree already contains Lisa-owned changes:

- `.lisa/provenance.jsonl` is modified;
- `docs/active/tickets/T-038-01-01.md` is modified.

Those files are outside this ticket's implementation ownership. The assignment
explicitly forbids manual phase/status updates, so neither is to be edited or
included in a ticket source commit.

Existing target artifacts are present and recognized as:

- a Mach-O 64-bit arm64 executable for the CLI;
- a WebAssembly MVP module for the plugin.

Their presence is not sufficient baseline evidence because their producing
command and freshness are not established by existence alone. The recorded
measurement must begin with the release builds.

## Relevant files and boundaries

- `Cargo.toml`: workspace members, package version, release profile.
- `Cargo.lock`: locked dependency graph used by a `--locked` build.
- `justfile`: canonical plugin-before-CLI release build ordering.
- `crates/lisa-plugin/Cargo.toml`: plugin crate definition.
- `crates/lisa-cli/Cargo.toml`: CLI binary definition and dependencies.
- `crates/lisa-cli/build.rs`: copies the release WASM into `OUT_DIR`.
- `crates/lisa-cli/src/templates.rs`: compile-time `include_bytes!` boundary.
- `crates/lisa-cli/src/loop_cmd.rs`: runtime emission of the embedded bytes.
- `target/wasm32-wasip1/release/lisa.wasm`: measured embedded-WASM source.
- `target/release/lisa`: measured native release CLI.
- `.lisa/attempts/T-038-01-01/1/work/`: private phase-artifact destination.

## Constraints surfaced

- No production, test, manifest, workflow, or shared documentation source is in
  scope for modification.
- Phase artifacts must be written only under the attempt-private work path.
- Ticket phase and status must not be edited by the agent.
- Any ticket-owned source change would require `lisa commit-ticket`, but the
  stated story scope anticipates no such source change.
- The build sequence must prebuild the WASM before compiling the CLI.
- The measurement must use byte length, not human-readable disk usage.
- A second execution is necessary to demonstrate the criterion that rerunning
  the recorded command yields the same numbers.
- Final review must distinguish exact within-environment reproduction from
  unsupported cross-platform bit-for-bit claims.

## Research conclusion

The artifact relationship is direct and observable: the release plugin build
produces `lisa.wasm`; the CLI build script copies that exact file; Rust embeds
the copied bytes into the native executable; and the runtime later writes the
same slice back out. The ticket can be satisfied without source changes by
recording a fail-fast, locked, plugin-first release build followed by exact
`wc -c` measurements, then rerunning that identical sequence and comparing the
two results.
