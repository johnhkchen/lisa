# Design: T-013-01 — `lisa doctor` subcommand

## Options considered

### Option A: Reuse `which()` + add version capture inline
Extend the existing `which()` in `loop_cmd.rs`, add version parsing there. Simple but couples doctor logic to loop_cmd. Not testable without real binaries.

### Option B: Trait-based command runner (over-engineered)
Define a `CommandRunner` trait with `fn run(&self, cmd, args) -> Result<Output>`. Production impl uses `std::process::Command`, test impl returns canned data. Adds abstraction for 3 checks.

### Option C: Struct-based check list with closure runner (chosen)
Define a `DependencyCheck` struct with fields for name, required, and a check function. The check function is a closure `Fn() -> CheckResult`. For testing, build checks with mock closures. For production, build checks that shell out. This is extensible (just push more checks) without abstract traits.

## Decision: Option C

**Rationale**: The ticket explicitly asks for "extensible (vec of check structs/closures)". Option C matches this directly. It avoids trait overhead while keeping tests deterministic. The closure boundary is clean — production code builds closures that call `Command::new()`, test code builds closures that return fixed results.

## Rejected

- **Option A**: Not testable without real binaries, not extensible.
- **Option B**: Over-engineered for 3 checks. A trait + impl pair adds files/complexity for no benefit over closures.

## Key design decisions

1. **No `--path` argument**: `doctor` checks system binaries (PATH), not project structure. Unlike `validate`, it doesn't need a project root.

2. **Check result type**: `CheckResult` enum with `Found { version: String }`, `NotFound { install_hint: String }`, and `Skipped { reason: String }` (for optional checks like WASM target when rustup is missing).

3. **Output formatting**: Two-column aligned output. Name column left-padded, version/status right-aligned. Install hints indented below failed checks.

4. **Exit code**: Iterate results, count required failures. Exit 0 if zero, exit 1 otherwise. The `Skipped` variant doesn't count as failure.

5. **Version extraction**: Use `Command::new(binary).arg("--version").output()` and parse first line of stdout. Trim whitespace. If the command succeeds but output is unexpected, still report "found" with raw output.

6. **Separation of concerns**:
   - `build_checks()` — constructs the Vec of checks with real command closures
   - `run_checks(checks)` — executes closures, returns results
   - `format_results(results)` — builds output string
   - `run_doctor()` — orchestrates all three, prints, returns exit success/failure
