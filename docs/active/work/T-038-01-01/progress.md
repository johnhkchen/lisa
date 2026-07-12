# Progress — T-038-01-01 CLI and WASM size baseline

## Outcome

Implementation is complete. The release CLI and embedded-WASM byte counts were
measured after the explicit locked release build sequence, and an immediate
second execution of the identical command produced the same two values.

Baseline:

| Artifact | Canonical path | Run 1 bytes | Run 2 bytes | Result |
|---|---|---:|---:|---|
| Release CLI binary | `target/release/lisa` | 3,013,904 | 3,013,904 | PASS |
| Embedded release WASM | `target/wasm32-wasip1/release/lisa.wasm` | 1,414,183 | 1,414,183 | PASS |

The path-specific values match exactly. The raw `wc -c` total was 4,428,087
bytes on both runs; that sum is retained only as command output and is not a
separate baseline artifact.

## Completion checklist

- [x] Read project, assignment, ticket, story, and RDSPI instructions.
- [x] Mapped the release build and compile-time WASM embedding path.
- [x] Wrote Research, Design, Structure, and Plan privately.
- [x] Captured source, compiler, Cargo, target, and host identity.
- [x] Built the release WASM with the checked-in lockfile.
- [x] Invalidated the WASM input timestamp before the CLI build.
- [x] Built the release CLI with the checked-in lockfile.
- [x] Recorded exact logical file lengths with `wc -c`.
- [x] Repeated the identical build-and-size command.
- [x] Confirmed both path-specific counts are unchanged.
- [x] Confirmed the measured paths have the expected artifact types.
- [x] Confirmed a CLI build-script copy matches the measured WASM.
- [x] Confirmed no ticket-owned source change or staged entry exists.
- [ ] Write final Review artifact.

## Source and environment identity

Measurement source:

- full HEAD: `2f8230d1d36a264522c82112c41adeb63cadf9dd`;
- short description: `2f8230d Complete T-038-03-01`;
- workspace/lisa-cli version: `0.4.0-rc.6`.

Compiler:

```text
rustc 1.99.0-nightly (c4af71034 2026-07-06)
binary: rustc
commit-hash: c4af71034e89a431eeee91125a31ad001379faac
commit-date: 2026-07-06
host: aarch64-apple-darwin
release: 1.99.0-nightly
LLVM version: 22.1.8
```

Cargo:

```text
cargo 1.99.0-nightly (2f0e7011e 2026-07-05)
```

Host:

```text
Darwin Johns-MBP.local 25.5.0 Darwin Kernel Version 25.5.0:
Tue Jun 9 22:28:17 PDT 2026;
root:xnu-12377.121.10~1/RELEASE_ARM64_T8142 arm64
```

The native count is a baseline for this recorded arm64 macOS build environment.
The exact command is reproducible in this environment; different native host
targets, linkers, or compiler versions are not claimed to produce the same CLI
byte count.

## Exact reproduction command

Both measurement executions used this command verbatim from the repository
root:

```bash
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release &&
touch target/wasm32-wasip1/release/lisa.wasm &&
cargo build --locked -p lisa-cli --release &&
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

Why each operation is present:

- the first Cargo call produces the release WASM at its canonical stable path;
- `--locked` uses the checked-in dependency resolution;
- `touch` follows the repository's canonical `build-cli` recipe and ensures the
  CLI build script sees its already-generated file input as changed;
- the second Cargo call builds the native release CLI after that input exists;
- `&&` prevents stale sizes from being printed following a failed build;
- `wc -c` reports logical byte counts for both explicitly named paths.

## Run 1

Exit status: `0`.

Build output summary:

```text
Finished `release` profile [optimized] target(s) in 0.30s
Compiling lisa-cli v0.4.0-rc.6
Finished `release` profile [optimized] target(s) in 7.82s
```

Raw size output:

```text
 3013904 target/release/lisa
 1414183 target/wasm32-wasip1/release/lisa.wasm
 4428087 total
```

## Run 2

Exit status: `0`.

Build output summary:

```text
Finished `release` profile [optimized] target(s) in 0.16s
Compiling lisa-cli v0.4.0-rc.6
Finished `release` profile [optimized] target(s) in 6.96s
```

Raw size output:

```text
 3013904 target/release/lisa
 1414183 target/wasm32-wasip1/release/lisa.wasm
 4428087 total
```

## Repeatability assessment

The CLI comparison is:

`3,013,904 == 3,013,904`

The embedded-WASM comparison is:

`1,414,183 == 1,414,183`

Both comparisons are true. The acceptance clause “re-running the recorded
command yields the same number” is demonstrated for both requested artifacts.

## Artifact identity checks

The measured files were identified as:

```text
target/release/lisa:
Mach-O 64-bit executable arm64

target/wasm32-wasip1/release/lisa.wasm:
WebAssembly (wasm) binary module version 0x1 (MVP)
```

Their SHA-256 checksums after Run 2 were:

```text
21364a09ca9f0b010475856c995069dd093f06c930682857c21abc40e4373449  target/release/lisa
14db37eed0fbde7507bf6da45be5edaf9b17803c6e6ee300875b68b15761c57c  target/wasm32-wasip1/release/lisa.wasm
```

The current CLI build-script output included:

```text
1414183 target/release/build/lisa-cli-b57ea7c670a9b3c9/out/lisa.wasm
```

`/usr/bin/cmp -s` returned success between that copy and
`target/wasm32-wasip1/release/lisa.wasm`. This supports the source inspection:
the stable-path WASM measurement is the exact byte slice copied for CLI
embedding, rather than a proxy transformed by the build script.

## Implementation scope

No Rust source, Cargo manifest, lockfile, test, build script, recipe, or shared
project documentation was changed. Generated outputs under `target/` were
rebuilt as measurement inputs and remain ignored.

No ticket-owned source unit exists, so no `lisa commit-ticket` call was required.
No ordinary `git add`, broad add, or ordinary `git commit` was used.

At the final implementation integrity check:

- the ordinary index contained no staged paths;
- `.lisa/provenance.jsonl` remained modified by Lisa orchestration;
- `docs/active/tickets/T-038-01-01.md` remained modified by Lisa phase handling;
- `docs/active/work/T-038-01-01/` appeared as untracked publication state
  generated by Lisa while attempt-private artifacts were written.

The agent did not write directly to or mutate that shared publication path.

## Deviations and incidental observations

The implementation followed the planned measurement command without deviation.

During a supporting copy-verification command, a shell loop variable was first
named `path`. In zsh, `path` is tied to `PATH`, so commands later in that
read-only check were not found. The check made no file changes. It was rerun
with the variable named `wasm_copy` and absolute `/usr/bin/cmp`; copy comparison
and Git integrity inspection then completed successfully. This did not affect
either recorded measurement run or its outputs.

## Remaining work

Only the Review artifact remains. After it is written, this attempt must stop on
T-038-01-01 and leave phase/status transition, publication, and completion
commit to Lisa.
