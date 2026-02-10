# T-008-03 Design: Dogfood Idle Signal Phase Transitions

## Approach Decision

**Option A: Test on lisa itself with existing S-008 tickets**
- Pros: Real project, no setup, meaningful
- Cons: T-008-01/02 are already done, only T-008-03 is open. T-008-03 is this
  ticket — circular. No tickets left to observe going through RDSPI phases.

**Option B: Create a small scratch test project**
- Pros: Clean environment, controlled, can observe full RDSPI flow
- Cons: Extra setup, not a "real" workload

**Option C: Create temporary test tickets within lisa repo**
- Pros: Real repo with existing infrastructure, can control what the agents do
- Cons: Creates noise in the ticket directory, must clean up

**Chosen: Option C** — Create 2 minimal test tickets (one root, one dependent)
directly in this repo's `docs/active/tickets/`, run `lisa loop`, observe the
full Research -> Review flow, then remove the test tickets. This gives us a
real environment with real hook infrastructure and real artifact paths.

## Rejected Options

- **Option A**: No observable work — the only non-done ticket is this one.
- **Option B**: Adds setup complexity and we lose the "dogfood on itself" aspect.

## Test Design

### Prerequisites
1. Merge idle_prompt hook config into existing `.claude/settings.local.json`
2. Create `.lisa/hooks/on-idle.sh` (from template) and make executable
3. Create `.lisa/signals/` directory
4. Create `.lisa/.gitignore`
5. Build fresh WASM + CLI with `just build-cli`

### Test Tickets
Create two simple tickets with a dependency:

**T-DOG-01** (root, no deps):
- Title: "dogfood-test-root"
- Type: spike
- Phase: ready
- Task: Document the color scheme used in ui.rs (trivial research + design +
  structure + plan + implement)

**T-DOG-02** (depends on T-DOG-01):
- Title: "dogfood-test-child"
- Type: spike
- Phase: ready (won't start until T-DOG-01 is done)
- Task: Add a comment to ui.rs documenting the color constants (trivial)

### Observation Plan
1. Run `lisa loop` with `max_threads: 1` (sequential, easier to observe)
2. Watch for:
   - T-DOG-01 gets scheduled, agent starts Research phase
   - Agent writes research.md -> idle signal fires -> plugin advances to Design
   - Agent writes design.md -> idle signal fires -> plugin advances to Structure
   - Continue through Plan and Implement
   - Agent finishes Implement -> idle signal fires -> plugin advances to Review
   - T-DOG-01 appears in attention banner, thread parks
   - Manually press 'd' to mark done
   - T-DOG-02 becomes ready, gets scheduled
   - Same flow repeats
3. Document results in progress.md

### Success Criteria
- At least T-DOG-01 completes Research through Review without manual phase
  intervention (all transitions via idle signal + artifact or idle signal alone)
- Implement -> Review specifically fires via idle signal (not manual)
- No regressions in existing phase detection
- Attention banner correctly shows parked review tickets

### Cleanup
- Remove T-DOG-01.md and T-DOG-02.md from tickets/
- Remove docs/active/work/T-DOG-01/ and T-DOG-02/ work dirs
- Revert any changes to .claude/settings.local.json if desired

## Risk Mitigation

- **WASM out of date**: Run `just build-cli` before `lisa loop`
- **settings.local.json merge**: Manually add hooks key alongside existing
  permissions key (both are top-level)
- **Test tickets interfere with real work**: Use T-DOG- prefix to distinguish,
  clean up immediately after
- **Agent does unexpected work**: Test tickets are trivial spikes with tight
  scope — worst case the agent writes some docs
