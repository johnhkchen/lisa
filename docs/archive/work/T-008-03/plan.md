# T-008-03 Plan: Dogfood Idle Signal Phase Transitions

## Step 1: Set Up Hook Infrastructure

1. Merge idle_prompt hook config into `.claude/settings.local.json`
2. Create `.lisa/hooks/on-idle.sh` from template, chmod 755
3. Create `.lisa/signals/` directory
4. Create `.lisa/.gitignore` with `signals/`

**Verify:** `lisa validate` passes, warns about nothing hook-related.

## Step 2: Create Test Tickets

1. Write `docs/active/tickets/T-DOG-01.md` (root spike, phase: ready)
2. Write `docs/active/tickets/T-DOG-02.md` (child spike, depends_on T-DOG-01, phase: ready)

**Verify:** `lisa validate` passes, shows 2 ready tickets in the output.
Then `lisa loop --dry-run` shows correct execution order.

## Step 3: Build Fresh Binary

1. Run `just build-cli` to get WASM + CLI with all T-008-01/02 code
2. Verify binary runs: `./target/release/lisa --version` or similar

## Step 4: Run Lisa Loop

1. Run `lisa loop` (or `cargo run -p lisa-cli -- loop`)
2. Observe in the Zellij dashboard:
   - T-DOG-01 gets scheduled into slot #1
   - Agent starts Research phase
   - Each phase transition fires when idle signal + artifact detected
   - Implement -> Review fires on idle signal alone
   - T-DOG-01 appears in attention banner
   - Press 'd' to mark done
   - T-DOG-02 gets scheduled
   - Same flow
3. Take notes in progress.md throughout

## Step 5: Document Results

1. Update progress.md with:
   - Which transitions fired automatically
   - Which required manual intervention
   - Any issues or bugs found
   - Timing observations
2. Assess against acceptance criteria

## Step 6: Cleanup

1. Remove test tickets: T-DOG-01.md, T-DOG-02.md
2. Remove test work dirs: T-DOG-01/, T-DOG-02/
3. Keep hook infrastructure (useful for future use)

## Testing Strategy

This IS the test. The entire ticket is an integration test run. Success is
measured by observing the behavior described in Step 4 and documenting it
in Step 5.

No unit tests are added — T-008-02 already has comprehensive unit test
coverage for the idle signal logic.
