# Design — T-046-06-03 closing acceptance run

## Decision summary

Record the two retained containers as non-admitted failed attempts, do not
reinterpret them as closing evidence, do not create duplicate product tickets,
and block Review on an operator-run conforming matrix, seeded variant, and tour
rematch.

The disposition will be operator-owned because the remaining work requires
interactive authentication, real provider tokens, hands-off observation, and
human-controlled measurements.

The block will name the exact action and a read-only verification check.

## Design forces

This ticket is the epic's field gate.

A plausible product implementation is not equivalent to a passing field run.

Both model/provider legs are independently required.

The runbook's exact-prompt boundary is part of the experiment.

The before/after snapshots are part of the experiment.

Independent post-run grading is part of the experiment.

The no-intervention rule is part of the experiment.

The selected model tier is part of the experiment.

The public current surface is part of the experiment.

Substituting an old commit changes the tested system.

The seeded failure and tour rematch are separate acceptance observations.

The ticket text says missing evidence produces a named block.

The workflow says failed field results must remain visible and actionable.

## Option 1 — admit both stopped containers as closing legs

This option would treat the names and exit-zero container state as sufficient.

It has superficial appeal because both agents eventually installed Lisa.

Both eventually reached some form of doctor invocation.

Both used fresh capped containers with no mounts.

Both avoided Rust compiler installation.

However, container exit zero only records the surrounding shell exit.

It does not record doctor or dry-run success.

The Claude run used Sonnet rather than Haiku.

The Codex run needed a second human message.

Both read an old pre-fix README.

Both installed v0.3.0 rather than the fixed release.

Both installed xz after the test began.

Neither has the required measurement snapshots.

Neither has the complete positive exit matrix.

Neither has an independently preserved negative-check output.

Admitting them would erase almost every experimental control.

Decision: reject.

## Option 2 — reconstruct missing measurements

This option would use Docker timestamps and writable-layer sizes.

Container start/finish time could produce an outer duration.

Final layer size could be compared with image size or another container.

Transcript timestamps could estimate agent turn duration.

Those quantities do not match the runbook boundary.

Authentication, setup, later loop use, caches, and agent state contribute to
the final writable layer.

Container lifetime includes work outside instruction-to-success.

Transcript time excludes operator post-check time.

The required exact before/after values never existed.

No reconstruction can recover the independent exit matrix either.

Decision: reject.

## Option 3 — treat the runs as product failures and file new bugs

This option would file bugs for v0.3.0, xz, cargo-named paths, and unmanaged
Zellij.

Those observations look severe in isolation.

They are also identical to the preserved baseline chain.

The retained shell history shows the Claude surface was deliberately fetched
from a fixed pre-fix commit.

The Codex README content is likewise the old pre-fix document.

The run therefore did not exercise the completed fixes.

T-046-03-02 already owns stable-channel skew.

T-046-03-03 already owns no-xz installation.

T-046-02 already owns managed Zellij.

T-046-04 already owns no-source-build directions.

Filing duplicates would imply current-surface regressions not established by
the evidence.

Decision: reject unless a future conforming current-surface run reproduces a
failure.

## Option 4 — change the runbook to match what happened

This option would allow file references instead of embedded install bytes.

It might allow any weak model instead of the named class.

It might accept operator hints after the first turn.

It might replace measurements with final layer size.

That would make the existing runs appear closer to compliant.

It would also destroy comparison with the baseline protocol.

The controls were intentional and are easy to follow as written.

The deviations arose before or around the measured instruction.

No first-contact finding shows the runbook itself is impossible.

Changing the rubric after seeing results is not an honest acceptance method.

Decision: reject.

## Option 5 — perform the reruns autonomously in this agent session

The containers and CLIs exist locally.

In principle another process could start new containers and invoke the CLIs.

The story explicitly assigns operation to John.

Fresh provider authentication requires human account authority.

The experiment tests a low-end agent, not this implementation agent.

The hands-off boundary needs an operator to observe and stop at the correct
point.

The tour run is metered and human-driven for the same reason.

Starting those runs would cross the ticket's authority boundary and contaminate
the evidence.

Decision: reject.

## Option 6 — block without preserving the retained evidence

This option would write only a short reason saying manual testing is missing.

It would be schema-valid.

It would lose useful information about why the current containers do not count.

An operator could accidentally repeat the same old-commit and missing-snapshot
mistakes.

The evidence is safe to summarize without copying credentials.

The summary can name exact container IDs, model mismatch, prompt deviation,
old surface, xz action, and missing checks.

Decision: reject as an incomplete handoff.

## Option 7 — preserve failed evidence and issue an actionable block

This option records the retained runs explicitly as non-admitted.

It maps every failed control to the runbook.

It distinguishes protocol failure from product failure.

It avoids exposing authentication state.

It asks John to run the exact missing experiments.

It links the standing T-047 tour owner instead of duplicating it.

It lets a future attempt verify remedy completion by checking for named evidence
artifacts rather than trusting prose.

It keeps T-046-06-03 open until a clean pass exists.

Decision: choose.

## Evidence artifact design

Create `closing-attempts-2026-07-16.md` in the attempt-private directory.

The document will not be called `closing-results.md`.

That name would imply admitted acceptance evidence.

It will identify each container and its resource isolation.

It will record only sanitized transcript facts.

It will not copy Claude or Codex auth/config files.

It will not include environment dumps.

It will not reproduce provider tokens or account identifiers.

It will include the actual model and prompt boundary.

It will include the old README commit/surface observation.

It will include relevant installer and agent strings.

It will include sudo/apt actions.

It will include the known changed-path summary.

It will mark every unavailable measurement `NOT RECORDED`.

It will explicitly state `NOT ADMITTED` for both legs.

## Finding-routing design

No new source ticket is created from the current evidence.

The old installer chain links to its existing E-046 owners.

Protocol deviations remain the responsibility of this ticket's rerun.

The missing landing probe links to T-047-01-02.

If a conforming current-surface run still installs xz, v0.3.0, or an unmanaged
Zellij, that future evidence must create a new product bug.

The future run must not silently absorb such a failure into this evidence gate.

## Review block design

The disposition is `block`.

The remedy owner is `operator`.

The reason must be non-empty and actionable.

The ask must be one sentence addressed to someone without authorship context.

The ask will require fresh runbook-conforming current-surface runs and placement
of sanitized result artifacts in this attempt directory.

The steps will enumerate primary Claude, primary Codex, seeded 0.40.1, and tour
rematch evidence.

The read-only check will test for the required evidence filenames.

The check will not start containers, authenticate providers, or modify files.

## Verification design

Verify all six phase artifacts are present and nonempty.

Verify the failed-attempt artifact is present and nonempty.

Verify JSON parses and has the required disposition fields.

Verify the reason, ask, owner, steps, and check are coherent.

Run Markdown whitespace checks with `git diff --check` on private paths.

Verify no ticket-owned shared source was modified.

Verify the ordinary Git index has no current-ticket entry.

No Rust test suite is required because no executable source changes.

## Design conclusion

The implementation is an evidence-preserving structured block.

It neither fabricates a pass nor discards the two real failed attempts.

It leaves product ownership intact and gives the operator an exact route to the
next admissible closing run.
