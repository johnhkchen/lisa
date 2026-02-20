---
id: T-013-02
title: Add dependency checks to lisa loop
type: task
phase: done
status: done
priority: high
story: S-013
created: 2026-02-20
depends_on:
  - T-013-01
---

# T-013-02: Add dependency checks to `lisa loop`

## Objective

`lisa loop` should refuse to start if required runtime dependencies are missing. Rather than failing cryptically when Zellij or Claude Code isn't found, it should check upfront and point the user to `lisa doctor`.

## Requirements

### Pre-flight check

Before `lisa loop` does any work (writing WASM, generating layout, exec'ing zellij), run the same dependency checks from `lisa doctor`.

### Behavior on failure

If any required dependency is missing:

```
Error: Missing required dependencies.

Run `lisa doctor` for details and install instructions.
```

Exit with a non-zero code. Do not proceed to launch Zellij.

### Behavior on success

Print nothing extra — just proceed as normal. Don't add noise to the happy path.

### Implementation

- Reuse the check functions from `doctor.rs` in `loop_cmd.rs`
- Extract shared checking logic into a function like `check_required_deps() -> Result<(), Vec<MissingDep>>`
- Keep `lisa doctor` as the verbose diagnostic tool; `lisa loop` just gates on the boolean result

## Acceptance Criteria

- [ ] `lisa loop` checks for `zellij` and `claude` before launching
- [ ] Clear error message pointing to `lisa doctor` on failure
- [ ] Normal operation when deps are present (no extra output)
- [ ] Shared check logic between `doctor` and `loop` (no duplication)
- [ ] Tests cover the gating behavior
