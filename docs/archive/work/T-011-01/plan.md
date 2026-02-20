# T-011-01 Plan: Build and Install Validation

## Steps

### Step 1: Run `just install`

Execute `just install` and capture full output including any warnings or errors.

**Verify:** Exit code 0, binary written to `$CARGO_HOME/bin/lisa`.

### Step 2: Verify binary on PATH

Run `which lisa`, `lisa --help`, `lisa --version`, and `lisa version`.

**Verify:** All commands succeed. `--help` shows expected subcommands. Version matches workspace version (0.1.6).

### Step 3: Run test suite

Execute `cargo test --workspace` and capture results.

**Verify:** All tests pass (0 failures). Note total test count.

### Step 4: Write progress.md

Document results against the ticket's acceptance criteria checklist. Note:
- Whether `just install` succeeded on first try
- Any missing dependencies or unclear error messages
- Whether all tests passed
- Approximate build time
- Any README friction

### Step 5: Mark ticket done

Update ticket frontmatter: `phase: done`, `status: done`.

## Testing Strategy

This ticket IS the test. The validation steps themselves are the verification. No unit tests or integration tests to write — we're running the existing test suite as part of validation.

## Commit Strategy

Single commit with all RDSPI artifacts and the updated ticket frontmatter.
