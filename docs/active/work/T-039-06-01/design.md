# Design: T-039-06-01

## Objective

Produce a release CLI from the completed post-refactor source tree whose embedded
plugin bytes come from a successful release `wasm32-wasip1` build, then establish
that formatting, native and WASM Clippy, native tests, and the canonical WASM
check are all green.

The output is evidence rather than a feature change.
The design must avoid manufacturing source churn merely to create a commit.
It must also avoid disturbing Lisa-owned lifecycle files or unrelated work.

## Option 1: use only `just build-cli` and `just check`

This is the shortest repository-native path.
`just build-cli` builds the plugin, touches it, and builds the CLI.
`just check` runs a WASM check and workspace tests.

Advantages:

- follows documented repository recipes;
- preserves the correct plugin-before-CLI ordering;
- includes the touch that retriggers the CLI build script;
- minimizes command count.

Limitations:

- the ticket explicitly names both Cargo release build commands;
- `just check` does not run formatting;
- `just check` does not run Clippy;
- it cannot alone satisfy the full acceptance wording.

This option is insufficient as the complete verification strategy.

## Option 2: run only the literal acceptance commands

This option runs the two named release builds, a format check, native Clippy,
WASM Clippy, and workspace tests.

Advantages:

- maps directly to every acceptance clause;
- produces straightforward command/result evidence;
- avoids relying on recipe interpretation.

Limitations:

- two adjacent Cargo build commands do not explicitly reproduce the Justfile's touch;
- an already-fresh CLI may not rerun its build script when plugin bytes are unchanged;
- it omits the repository's canonical `just check` unless added separately.

This option is close, but the embedding freshness signal should be explicit.

## Option 3: clean package build outputs before rebuilding

This option removes plugin and CLI Cargo artifacts, then runs all gates.

Advantages:

- forces compilation of the two products;
- prevents reuse of their previous release build outputs;
- makes “rebuild” literal at the artifact level.

Limitations:

- the repository can be shared by concurrent Lisa seats;
- Cargo's target directory is shared across those seats;
- cleaning can slow or invalidate unrelated concurrent work;
- cleanup is not required by the ticket or the repository recipe;
- package cleanup can remove more target state than this ticket owns.

The disruption is not justified when the documented build workflow already
contains an embedding freshness mechanism.

## Option 4: isolated Cargo target directory

This option sets a ticket-specific `CARGO_TARGET_DIR`, builds the plugin there,
and attempts to build the CLI there.

Advantages:

- guarantees clean, isolated artifacts;
- cannot interfere with the shared default target directory;
- offers strong provenance for a from-scratch build.

Limitations:

- `crates/lisa-cli/build.rs` hardcodes the workspace-relative `target/` path;
- it does not derive the plugin location from `CARGO_TARGET_DIR`;
- a CLI built in an alternate target directory still reads the default target's WASM;
- changing `build.rs` is outside this verification-only ticket;
- the result would not test the repository's supported embedding path.

This option is incompatible with the current build-script boundary.

## Option 5: repository workflow plus explicit acceptance gates

This option combines the strongest parts of Options 1 and 2:

1. run the exact release plugin build command;
2. verify the resulting WASM exists and is non-empty;
3. record its byte count and SHA-256 digest;
4. touch the WASM, matching `just build-cli`;
5. run the exact release CLI build command;
6. record the CLI byte count and SHA-256 digest;
7. run formatting check;
8. run broad native Clippy with warnings denied;
9. run production-target WASM Clippy with warnings denied;
10. run workspace tests;
11. run `just check` for the canonical WASM check and repeated tests;
12. inspect repository cleanliness and ticket-owned paths.

Advantages:

- matches the named acceptance commands;
- matches the supported plugin-first embedding workflow;
- keeps generated outputs in the path expected by `build.rs`;
- does not clean shared build state;
- makes the exact output identities recordable;
- covers stricter Clippy scope than the minimal Justfile lint recipe;
- includes the canonical repository gate.

Limitations:

- compilation may reuse valid Cargo intermediates;
- touching the WASM changes its timestamp but not its bytes;
- hashes identify artifacts but do not prove runtime loading by Zellij.

The limitations are honest and consistent with the ticket boundary.
Runtime/live-seat observation belongs to the following ticket.

## Decision

Choose Option 5.

The decisive codebase fact is that the CLI build script reads only the default
workspace `target/wasm32-wasip1/release/lisa.wasm` path. The repository's own
`build-cli` recipe handles freshness by building that file first and touching it
before the CLI build. Reproducing that ordering tests the supported release path.

The verification commands will be run separately rather than hidden in one shell
pipeline. Separate invocations preserve an unambiguous pass/fail result for each
acceptance clause. If one fails, subsequent diagnosis can name the exact gate.

## Build evidence design

The first required build is:

`cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

After it succeeds, verify:

- `target/wasm32-wasip1/release/lisa.wasm` exists;
- the file is non-empty;
- its logical byte length is recorded;
- its SHA-256 digest is recorded.

Then touch that exact file and run:

`cargo build -p lisa-cli --release`.

The touch is not counted as a separate acceptance gate.
It is the repository-defined invalidation step for `cargo:rerun-if-changed`.
The CLI build's success proves the build script copied a present WASM into its
`OUT_DIR` and Rust compiled `include_bytes!` against that copy.

Afterward record the CLI artifact's byte length and digest.
Artifact sizes are informational and no size threshold applies.

## Gate design

Formatting gate:

`cargo fmt --all -- --check`.

Native warning-strict lint gate:

`cargo clippy --workspace --all-targets --all-features -- -D warnings`.

WASM warning-strict lint gate:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

Native behavior gate:

`cargo test --workspace`.

Canonical repository gate:

`just check`.

The canonical gate repeats native tests, but that repetition is useful because it
also couples them with the ordinary WASM `cargo check` used by contributors.

## Failure policy

If a command fails because of a deterministic code defect, record the failure.
Do not silently modify structural code in this closing slice.
The story says newly discovered defects become separate tickets or epics.

If a command fails for a clearly transient environmental reason, inspect the
cause and retry only when the retry does not mask a source problem.
Record both the first result and the retry in `progress.md`.

If any gate remains red or an anomaly remains unexplained, Review must identify
it as blocking acceptance.

## Source and commit policy

The expected source-change set is empty.
Generated `target/` artifacts are not committed.
Private phase artifacts are not committed directly by the agent.
Lisa owns their final publication and completion commit.

If an unexpected necessary ticket-owned source edit arises, it must first be
documented as a plan deviation. It must then be committed only through:

`lisa commit-ticket --ticket-id T-039-06-01 --message ... --include <exact-path>`.

No ordinary index command will be used.
The pre-existing ticket and provenance changes will remain untouched.

## Completion boundary

Implementation completes when both release products exist, every gate is green,
and `progress.md` contains exact commands and outcomes.
Review then summarizes the no-source-change result, evidence, coverage, and any
open concerns.

After `review.md` is written, work stops on this ticket.
The next field-report ticket is deliberately not started.
