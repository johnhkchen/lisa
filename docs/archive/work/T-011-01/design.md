# T-011-01 Design: Build and Install Validation

## Decision Context

This is a validation chore — the goal is to verify the existing build-from-source workflow, not to change it. The "design" here is about how to document findings and what (if any) improvements to recommend.

## Approach: Validate and Document

**Chosen approach:** Execute the documented build steps exactly as written, record results, and document any issues found. No code changes unless the build itself is broken.

### Rationale

The ticket explicitly says "no code changes expected, just validation." The research phase confirmed that the build succeeds and all 332 tests pass. The dead-code warnings and documentation gaps noted in research are real but minor — they belong in separate tickets if they need fixing.

## Issues Found (from Research)

### Non-blocking

1. **3 dead-code warnings in ui.rs** — `pane_id` fields on `ActiveThread`, `ParkedThread`, `SlotInfo`. Cosmetic, would be a separate cleanup ticket.

2. **README doesn't include `just` install command** — Says "just command runner" is required but doesn't show how to install it. A docs improvement, not a build blocker.

3. **Test count diverged from memory** — Memory says 88 tests (Sprint 7), actual count is 332. Memory is stale, not an issue with the build.

### None Found

- Build completes successfully
- All tests pass
- Binary is on PATH and functional
- Version output works via both `--version` and `version` subcommand
- WASM plugin is properly embedded

## Rejected Alternatives

1. **Automate the validation in CI** — Overkill for this ticket. CI already builds and tests on every PR via the release workflow. This ticket is about manual verification on a real device.

2. **Fix the dead-code warnings in this ticket** — Out of scope. The ticket is validation, not cleanup.

## Deliverable

A `progress.md` documenting the validation results, satisfying all acceptance criteria checkboxes.
