# T-TEST-03 Research: Test Coverage Analysis

## Source: T-TEST-02 Build System Summary

T-TEST-02 documented the build system, workspace structure, and build tooling. This research focuses on test coverage: what is tested, how well, and where gaps exist.

## Test Execution Environment

All tests run on the **native** target via `cargo test --workspace`. The WASM target (`wasm32-wasip1`) is not used for testing because tests avoid Zellij APIs. The sole test dependency across all crates is `tempfile = "3"` for creating temporary directories.

There are **no integration tests** (no `tests/` directories). All tests are in-module `#[cfg(test)]` unit tests.

## Aggregate Test Counts

| Crate | Tests | Source Lines | Ratio |
|-------|-------|-------------|-------|
| **lisa-cli** | 127 | ~4,525 | 1 test per ~36 lines |
| **lisa-core** | 78 | ~3,135 | 1 test per ~40 lines |
| **lisa-plugin** | 131 | ~7,248 | 1 test per ~55 lines |
| **Total** | **336** | **~14,940** | 1 test per ~44 lines |

Note: The project MEMORY.md cites 88 tests (Sprint 7). Current count is **336** — substantial growth since then.

## Per-Module Breakdown

### lisa-core (78 tests)

| Module | Tests | Lines | Coverage Quality |
|--------|-------|-------|-----------------|
| `dag.rs` | 30 | 1,066 | **Excellent** — cycles, topo sort, critical path, execution waves, diamond/chain patterns |
| `types.rs` | 28 | 901 | **Excellent** — Phase transitions, Thread lifecycle, HealthStatus, serde roundtrips |
| `ticket.rs` | 14 | 854 | **Good** — Parsing, frontmatter extraction, scanning, field updates |
| `diagnostics.rs` | 6 | 314 | **Good** — Covers clean loads, errors, cycles, missing deps |
| `lib.rs` | 0 | 4 | N/A (re-export file) |

### lisa-plugin (131 tests)

| Module | Tests | Lines | Coverage Quality |
|--------|-------|-------|-----------------|
| `lib.rs` | 85 | 5,279 | **Good** — Scheduler logic, thread spawning, phase advancement, idle signals, pause/resume, health evaluation, slot management, transition signals, review timeouts |
| `ui.rs` | 46 | 1,969 | **Excellent** — Dashboard rendering, DAG visualization, status lines, attention banners, compact mode, scroll, slots display |

### lisa-cli (127 tests)

| Module | Tests | Lines | Coverage Quality |
|--------|-------|-------|-----------------|
| `config.rs` | 22 | 411 | **Excellent** — Parsing, validation, resolution, overrides, warnings |
| `init.rs` | 31 | 1,690 | **Good** — Plan actions, roundtrip init+validate, diagnostics, overwrite protection, hook scaffolding |
| `doctor.rs` | 14 | 433 | **Excellent** — Dependency checks, display formatting, skip/found/missing states |
| `detect.rs` | 6 | 305 | **Good** — All project types + priority order + layout scan |
| `setup_guide.rs` | 9 | 444 | **Good** — RDSPI content, project types, step numbering |
| `templates.rs` | 13 | 479 | **Good** — Embedded content verification, hook scripts, settings JSON, merge logic |
| `loop_cmd.rs` | 7 | 355 | **Adequate** — Layout generation, dry-run mode |
| `status.rs` | 5 | 260 | **Adequate** — Ticket scanning, dependency chains, error paths |
| `main.rs` | 0 | 148 | **Untested** — CLI entry point, command routing |
| `build.rs` | 0 | 28 | **Untested** — Build script, file copy |

## Files Without Tests

1. **`main.rs`** (148 lines) — Clap CLI entry point with command routing and path resolution. Not easily unit-tested because it calls `std::process::exit` and delegates to other modules. The real logic is in the modules it calls, which are well-tested.

2. **`build.rs`** (28 lines) — Cargo build script that copies WASM binary. Has a single purpose with minimal branching. Would require a test harness for build scripts, which Cargo does not natively support.

3. **`lib.rs`** (4 lines) — Module re-export file with no logic.

## What the Tests Cover Well

- **DAG computation**: Cycle detection, topological sort, diamond/chain dependency patterns, concurrent scheduling, execution waves, critical path calculation. This is the algorithmic heart of lisa and it's thoroughly tested.
- **Ticket parsing**: YAML frontmatter extraction, field validation, type/phase/status enum parsing, directory scanning, and frontmatter update operations.
- **Configuration**: Full resolution chain (defaults → config file → CLI overrides), validation with warnings, and edge cases.
- **UI rendering**: Dashboard layout, status indicators, phase progress, attention banners, compact mode, scroll behavior.
- **Plugin lifecycle**: Thread state machines, phase transitions, idle/stop/clear signals, health monitoring (stuck/failed detection), slot management with cooldowns, pause propagation, review timeouts.
- **Init scaffolding**: Plan-then-execute pattern, never-overwrite guarantee, hook creation, roundtrip init-then-validate.

## What the Tests Don't Cover

1. **No integration tests**: All tests are unit tests. There's no end-to-end test that runs `lisa init` then `lisa validate` then `lisa loop` as a CLI binary.

2. **No WASM target testing**: Tests run natively only. WASI-specific code paths (filesystem mounting, `/host` prefix handling) are tested via mocking or conditional compilation, not in the actual WASM runtime.

3. **No Zellij API interaction testing**: The plugin's `load()`, `update()`, and `render()` methods that call Zellij APIs are not tested. Only the internal logic that doesn't require Zellij is tested.

4. **Sparse loop_cmd coverage**: 7 tests for 355 lines. The KDL layout generation is tested but actual Zellij launch logic is not (it calls `exec`).

5. **No fuzz testing or property-based testing**: All tests are example-based.

6. **No concurrent/parallel test scenarios**: The scheduler's commit lock and thread management are tested sequentially, not under concurrent conditions.

## Observations

- Test quality is high overall. The project follows a pattern of testing internal logic thoroughly while excluding external API boundaries (Zellij, filesystem, process execution).
- `tempfile` is used consistently for filesystem tests, providing good isolation.
- The test-to-code ratio is healthy (1:44 lines average), with core algorithmic modules having denser coverage.
- The project has grown from 88 to 336 tests, indicating active test development alongside feature work.
