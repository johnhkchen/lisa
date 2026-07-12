# Design — T-038-01-01 CLI and WASM size baseline

## Decision statement

Measure both canonical Cargo release outputs immediately after a locked,
plugin-first release build, using `wc -c` for exact logical byte lengths. Run
the identical command twice and preserve both outputs in `progress.md`, along
with the source/toolchain/host context needed to interpret the native result.

No repository source or shared documentation will be changed. Lisa will publish
the attempt-private RDSPI artifacts after validating the lease.

## Goals

The selected approach must:

- produce the release native CLI size in bytes;
- produce the release embedded-WASM size in bytes;
- prove the CLI build followed the plugin build;
- avoid an empty build-script placeholder;
- name the exact measured paths;
- be safe against stale output after build failure;
- be straightforward for a later release-readiness ticket to rerun;
- demonstrate same-environment repeatability now;
- preserve the measurement without changing the measured product.

## Non-goals

This ticket does not:

- optimize either artifact;
- compare before and after values;
- establish cross-platform binary equivalence;
- measure disk allocation, compressed package size, memory, or timing;
- extract and parse the embedded object section from the native executable;
- change build recipes, source code, profiles, or dependency versions;
- publish artifacts directly to `docs/active/work/T-038-01-01/`.

## Option 1 — Use the canonical `just build-cli` recipe, then `wc -c`

The repository already defines the correct plugin-before-CLI order. A command
could run:

`just build-cli && wc -c <CLI> <WASM>`

### Advantages

- Closely matches the documented local release workflow.
- Reuses the established `touch` that invalidates the CLI build script.
- Concise and easy to type.
- Makes the embedding order visible through the named recipe.

### Limitations

- The recipe does not pass `--locked` to Cargo.
- Reproduction depends on `just` being installed in addition to Cargo.
- A reader must open `justfile` to see the precise build commands.
- Future recipe edits could change what the historical command means.

This is viable, but its indirection weakens a standalone evidence record.

## Option 2 — Record the explicit Cargo build sequence, then `wc -c`

The command spells out all operations:

1. build `lisa-plugin` for `wasm32-wasip1` in release mode with the lockfile;
2. touch the generated WASM to ensure the CLI build script sees it as changed;
3. build `lisa-cli` in release mode with the lockfile;
4. measure both exact output paths with `wc -c`.

### Advantages

- Self-contained; no recipe expansion is required to understand it.
- Mirrors the canonical recipe's material order and invalidation step.
- `--locked` pins dependency resolution to `Cargo.lock`.
- `&&` prevents measurement if any build/invalidation step fails.
- Exact target paths and units are visible in the evidence itself.
- Suitable for direct reuse by the later before/after report.

### Limitations

- Longer than invoking `just`.
- Duplicates the current recipe text in a documentation artifact.
- Still relies on the recorded compiler and host for native reproducibility.

This is the selected approach because the ticket specifically values an exact,
reproducible command over terseness.

## Option 3 — Measure existing target files without rebuilding

This would run only `wc -c` on the two paths.

### Advantages

- Fast and non-mutating even within ignored build output.
- Produces exact logical lengths for whatever files are present.

### Limitations

- Cannot establish which source revision or flags produced the files.
- May measure stale output.
- May measure a CLI built with an earlier WASM or an empty placeholder.
- Fails the acceptance requirement to record the build that produced the size.

Rejected because artifact presence is not provenance.

## Option 4 — Clean the workspace before every release build

This could prepend `cargo clean` or delete the two output trees.

### Advantages

- Forces all relevant crates and build scripts to compile from scratch.
- Removes ambiguity from Cargo freshness decisions.

### Limitations

- Destructive to shared build caches in a concurrent-agent workspace.
- Expensive and unnecessary for a documentation-only measurement.
- Broadens the effect of the ticket beyond the two outputs.
- The canonical recipe's touch already forces the embedding boundary to rerun.
- A clean build does not solve host/compiler variance.

Rejected due to shared-workspace cost and unnecessary breadth.

## Option 5 — Extract the WASM from a runtime `lisa loop` invocation

The CLI writes `PLUGIN_WASM` to a content-addressed file in the temporary
directory. The extracted file could be measured after starting the loop.

### Advantages

- Observes the bytes through the actual runtime extraction path.
- Demonstrates that the compiled CLI can emit the embedded slice.

### Limitations

- Requires Zellij/session lifecycle handling unrelated to size.
- Introduces mutable temp-directory state and cleanup behavior.
- Makes a deterministic file-length measurement operationally complex.
- The source/build code already establishes that no transformation occurs.
- Risks overlapping the separate startup/launch baseline ticket.

Rejected as unnecessary integration scope.

## Option 6 — Parse or carve the WASM bytes from the Mach-O CLI

A binary inspection tool could locate the embedded byte sequence in the native
executable and report its length.

### Advantages

- Directly inspects the final native executable.
- Could independently confirm containment of the plugin bytes.

### Limitations

- Rust's layout does not expose a stable named section contract for this slice.
- Requires platform-specific object tooling or a custom parser.
- Finding a byte sequence is not a better size definition than its source slice.
- Adds fragility without changing the ticket's requested number.

Rejected because `include_bytes!` plus the build-script copy gives a clearer and
portable definition of “embedded-WASM byte count.”

## Exact command shape

The evidence command will be a multiline shell sequence equivalent to:

```bash
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release &&
touch target/wasm32-wasip1/release/lisa.wasm &&
cargo build --locked -p lisa-cli --release &&
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

The ordering of `--locked` relative to other Cargo flags is immaterial, but the
recorded spelling will be used verbatim for both executions.

`wc -c` may also print a total line. Only the two path-specific lines are the
baseline values; the total is a convenience sum and is not a third artifact.

## Repeatability design

The command will be executed once to establish the candidate baseline and a
second time without modification to tracked inputs. The path-specific counts
must match exactly across both executions.

The second build may report Cargo artifacts as fresh. That is acceptable:
Cargo freshness is part of normal command reproduction, while the explicit
touch ensures the CLI build-script/embedding boundary is reconsidered.

If counts differ, the work will stop short of claiming acceptance and inspect:

- source HEAD changes;
- concurrent changes to manifests or lockfile;
- compiler/toolchain drift;
- build-script rerun output;
- target artifact timestamps and types.

## Evidence placement

`progress.md` will be the primary measurement record. It will contain:

- exact command;
- first output;
- second output;
- direct equality assessment;
- artifact meanings;
- source, package, host, Rust, and Cargo context;
- implementation-scope statement and repository-integrity check.

`review.md` will summarize those values, acceptance status, verification, and
limitations for the human handoff. Research, Design, Structure, and Plan retain
the rationale and execution blueprint.

## Decision rationale

The explicit locked build sequence is the smallest approach that closes every
observed ambiguity: it builds the plugin before the CLI, forces Cargo to revisit
the copied input, refuses to measure after failure, uses the checked-in lockfile,
and measures logical lengths at canonical paths. Repeating it demonstrates the
ticket's stated reproducibility property without altering the product whose
size is being baselined.
