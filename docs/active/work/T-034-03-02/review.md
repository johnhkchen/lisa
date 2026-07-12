# Review: T-034-03-02 live proof and Claude parity

## Outcome

The deterministic fenced-attempt boundary is proven and remains green, but the
fresh-loop acceptance criterion is not fully satisfied.

The freshly built runtime exposed a reproducible critical defect: the very
first provider command injected during plugin startup is truncated before its
closing quote, leaving zsh at `dquote>` while Lisa already treats the fresh seat
as assigned and owned.

This occurred with Codex first and again with Claude first in a separate control.

No provider process launched in either untouched initial case.

The downstream lease, artifact, completion, and provenance path was exercised
successfully after a documented manual repair of the Codex command.

The dependent Claude ticket then launched without intervention on a later pane
and completed normally, confirming the established Claude adapter and
completion behavior still work outside the startup race.

## Files created

Workflow artifacts:

- `docs/active/work/T-034-03-02/research.md`
- `docs/active/work/T-034-03-02/design.md`
- `docs/active/work/T-034-03-02/structure.md`
- `docs/active/work/T-034-03-02/plan.md`
- `docs/active/work/T-034-03-02/progress.md`
- `docs/active/work/T-034-03-02/review.md`

Evidence:

- `docs/active/work/T-034-03-02/evidence/build-and-tests.md`
- `docs/active/work/T-034-03-02/evidence/live-run.md`

No production or test source file was created, modified, or deleted.

The parent ticket frontmatter was not edited by this agent.

## Build provenance

The proof used source revision:

`0ffe40f67551774964cfaf3e229ba5052cee43ea`

Release WASM and release CLI builds passed.

The exact target and loop-extracted WASM SHA-256 was:

`cfac4d9390a0898682a4d262a1bf3a4b042608cf0db5a1f947643659f5f63ce8`

The fresh installed CLI SHA-256 was:

`c01d0eda63b793725a2d3e6c81888b6cad388bb0f815bf0a7af4bf8677075094`

The generated layout named the temporary absolute CLI in `lisa_bin` and the
content-hashed fresh WASM path.

The old Homebrew Lisa did not drive the proof.

## Deterministic fence proof

The exact prerequisite regression passed:

`split_brain_timeline_fences_old_attempt_and_admits_one_winner`

It executes real scheduler methods for:

- slow/hard-silent predecessor timeout;
- lease revocation;
- pane fencing;
- slot release;
- successor mint and redispatch;
- delivered but unacknowledged replacement prompt;
- resumed predecessor signal rejection;
- private artifact admission;
- stale completion rejection;
- authoritative successor provenance.

It asserts `LeaseRevoked -> PaneFenced -> SlotReleased`, a distinct successor
pane and generation, zero ownership while acknowledgement is missing, no
cross-attempt artifact attribution, and exactly one authoritative Done result.

This is the strongest available proof for the complete adversarial timeline.

## Primary live fixture result

The primary fixture used two matched tickets in one independent Zellij session:

1. Codex initially ready;
2. Claude dependent on the Codex completion receipt.

The new WASM loaded and discovered real terminal panes.

It minted attempt 1 and wrote the expected lease for the Codex ticket.

However, the initial shell command stopped in the middle of the common ticket
prompt and never launched Codex.

The pane title and lease made the scheduler assignment visible even though the
provider process did not exist.

After manually appending the missing suffix and accepting a trust prompt, Codex
completed all six phases through the normal attempt-private artifact path.

Lisa produced completion commit:

`5bc44a697ee5cd8586a8823233999c54bd6ca835`

It also produced exactly one authoritative Codex/OpenAI Done record.

The intervention means this is valid downstream contract evidence but not a
clean assignment success.

## Claude parity result

In the primary run, the dependent Claude ticket was scheduled only after the
Codex completion receipt.

Claude launched on the previously idle pane with the complete generated command,
required no Codex acknowledgement, wrote all six private artifacts, and stopped
after Review.

Lisa admitted the artifacts and produced completion commit:

`fb346aa4f6146836df50f18cd57d0aeb68044d0f`

It also produced exactly one authoritative Claude/Anthropic Done record.

This confirms unchanged Claude assignment/completion semantics for a later
fresh-pane assignment in an already-running plugin.

The second control placed Claude in the identical initially-ready position.

That command also stopped mid-prompt at `dquote>` and Claude did not launch.

Therefore the broader claim that Claude behavior is unchanged across the full
fresh-start harness is not met.

The failure is provider-neutral at the observed boundary.

## Acceptance mapping

### Isolated temporary project

Met.

Two disposable repositories and two independent Zellij sessions were used.

Parent Zellij environment variables were removed before launch.

### Freshly built and installed Lisa CLI/WASM

Met.

Hashes prove that the loop-extracted WASM equals the release target.

### Execute the committed split-brain harness

Met.

The exact named regression passed 1/1.

### Prove fenced boundary end-to-end

Partially met.

The deterministic production-method regression proves every lease and stale
attempt boundary.

The live WASM loaded, minted leases, admitted artifacts, and completed real Git
transactions, but the live run did not safely recreate a timed-out predecessor
and resume it after actual Zellij pane closure.

The first-assignment failure also prevents an intervention-free end-to-end run.

### Record unchanged Claude assignment/completion behavior

Partially met, with a critical exception.

Later Claude assignment and completion are unchanged and successful.

Initially-ready Claude assignment is broken in the same way as Codex.

### No parent-loop hot reload

Met.

Both isolated sessions used fresh processes and were terminated after capture.

## Test coverage

Passed:

```text
cargo test -p lisa-plugin \
  split_brain_timeline_fences_old_attempt_and_admits_one_winner
cargo test -p lisa-plugin
cargo fmt --all -- --check
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- docs/active/work/T-034-03-02
```

Results:

- focused split-brain regression: 1 passed;
- full plugin suite: 273 passed;
- formatting: passed;
- WASM check: passed;
- ticket work whitespace: passed.

## Coverage gap revealed

Existing native tests verify the generated command string and scheduler intent.

They do not exercise delivery of the long command into a newly created Zellij
shell at plugin startup.

They therefore cannot detect a partial host write or early pane-readiness race.

A Zellij integration test should assert that the first assigned pane actually
runs the provider process and receives the complete closing quote/prompt before
the scheduler records ownership.

## Critical issue

Severity: critical for unattended fresh loops.

Observed invariant violation:

`fresh seat is Owned` did not imply `provider process accepted or even started`.

Consequences:

- a ticket can appear assigned while no agent is running;
- no provider heartbeat or completion arrives;
- recovery waits for the much longer hard-silence/session timeout boundary;
- the same issue affects Claude and Codex when either is the first assignment;
- a user can mistake a titled pane and lease file for active ownership.

The exact cause is not proven here.

Evidence is consistent with early plugin-to-shell input loss or pane-readiness
timing, rather than provider-specific parsing, because both initial provider
commands failed and a later Claude command succeeded.

The trust-pregrant path also deserves follow-up because Codex used a
`/private/var/...` canonical path while the fixture was created under
`/var/...`.

## Recommended follow-up

Create a dedicated implementation ticket that:

1. reproduces first-assignment input delivery under real Zellij;
2. makes long command delivery atomic or explicitly acknowledged;
3. does not mark a fresh seat Owned before provider-start evidence;
4. adds a bounded recovery path for a provider that never starts;
5. canonicalizes macOS fixture paths before Codex trust pregrant;
6. reruns this exact Claude-first and Codex-first harness without intervention.

## Repository integrity

No parent source path changed.

No ordinary-index ticket-owned entry exists.

No `lisa commit-ticket` source transaction was required because this ticket
produced validation evidence only.

Unrelated pre-existing modified and untracked parent paths remain untouched.

Both temporary Zellij sessions were terminated.

## Final assessment

The lease-fencing implementation remains regression-proven, and the real
artifact/completion/provenance paths work for both providers once a provider is
running.

The live proof did its job by finding a critical fresh-start boundary failure.

T-034-03-02 should not be treated as a clean acceptance pass until the initial
assignment defect is fixed and the identical isolated harness succeeds for both
Claude and Codex without manual command repair or trust intervention.
