# T-TEST-03 Design: Test Coverage Summary Approach

## Problem

The ticket asks for a summary of test coverage: how many tests exist, which crates have tests, and what areas lack coverage. This is an analysis task, not a code change. The deliverable is documentation.

## Approach Options

### Option A: Static count from cargo test output

Run `cargo test --workspace`, parse the summary lines, and report totals per crate. Simple, accurate, and directly verifiable.

**Pros**: Exact numbers, no tooling dependencies, reproducible.
**Cons**: Only counts test functions — says nothing about what code paths are exercised.

### Option B: Line-level coverage via cargo-tarpaulin or llvm-cov

Run instrumented test builds to get line/branch coverage percentages.

**Pros**: Precise coverage percentages per file.
**Cons**: Requires installing additional tools (cargo-tarpaulin doesn't support macOS well; llvm-cov needs nightly). Overkill for a summary ticket in a test chain.

### Option C: Manual module-level analysis

Read each source file, catalog its test module, and assess coverage quality based on what the tests exercise vs. the module's responsibility.

**Pros**: Identifies semantic coverage gaps (untested behaviors) not just line counts.
**Cons**: More effort, subjective.

## Decision: Option A + C combined

Use `cargo test` output for exact counts (Option A) and manual module analysis (Option C) for qualitative assessment. Skip instrumented coverage (Option B) because:

1. This is a test ticket for validating lisa loop, not a production coverage mandate.
2. The manual analysis from Research already identifies the meaningful gaps.
3. cargo-tarpaulin has poor macOS support and llvm-cov requires nightly — neither is worth installing for a one-time summary.

## Output Format

The final summary (in progress.md) will contain:

1. **Aggregate numbers**: Total tests, per-crate breakdown, test-to-code ratio.
2. **Module-level breakdown**: Tests per module with qualitative assessment.
3. **Gap analysis**: What's untested and why it matters (or doesn't).
4. **Comparison with project memory**: Note the growth from 88 (Sprint 7) to 336 tests.

## Rejected Alternatives

- **Option B alone**: Instrumented coverage is the right tool for ongoing CI enforcement, but wrong for a one-off summary in a test dependency chain.
- **Generating coverage badges or artifacts**: Out of scope — no CI integration changes requested.
