# Design — T-046-06-02 baseline-run-record

## Decision summary

Preserve the retained Codex/mini probe as explicitly partial evidence, file the
new xz-free-installer finding, and block Review until John performs both fresh
runbook-conforming baseline legs.

Do not promote reconstructed measurements, predecessor summary prose, the later
Claude tour, or the retained shared-auth container into acceptance evidence.

The design favors an auditable incomplete record over a complete-looking record
whose key measurements never occurred.

## Design forces

The ticket is valuable only if it records what a real low-end agent actually did.

The ticket text makes the human/operator boundary normative.

The runs consume real agent tokens and depend on real authenticated accounts.

The tested agent must receive one prescribed instruction and no expert help.

The operator must capture before/after measurements around that interaction.

The result must include exact strings rather than retrospective paraphrases.

Both provider/model legs are independently required.

The runbook forbids reusing one measured container across legs.

The current session is an implementation agent working on the evidence ticket;
it is not John operating two fresh interactive trials.

## Option 1 — treat the predecessor assertion as sufficient

The predecessor Review says the Codex CLI completed a "full measured baseline
leg" in `cbt-0716-144625`.

One approach would quote that assertion and mark the Codex leg complete.

This is attractive because the retained container and transcript prove that an
install interaction occurred.

It is not sufficient because the assertion does not contain the required
measurements or result fields.

The runbook snapshot files do not exist.

The prompt in the transcript does not match instruction A.

The shell contradicted the agent's PATH success claim after the declaration.

The predecessor itself assigns baseline evidence to this ticket.

Decision: reject as an acceptance disposition.

The predecessor assertion remains useful as a locator for the retained probe.

## Option 2 — reconstruct a complete Codex result

The transcript provides timestamps, command output, model identity, and many
changed paths.

Docker provides current writable-layer size.

A result record could derive wall time from transcript timestamps and label the
current writable-layer size as disk delta.

That would create apparently complete numeric fields.

The numbers would describe different boundaries from the runbook.

The transcript interval excludes operator snapshots and independent acceptance
checks.

The writable layer includes authentication, Codex plugins, and a later Claude
tour.

There is no before-size measurement with which to isolate Lisa's delta.

The prompt deviation also changes the intervention being measured.

Decision: reject.

Derived values may be included only when labeled as non-acceptance context.

## Option 3 — accept the Codex probe and block only Claude

The retained session uses `gpt-5.4-mini`, matching the requested model class.

It shows a real, hands-off agent recovering from several failures.

It preserves valuable verbatim strings.

Accepting it as one leg would reduce the remaining human work.

However, the acceptance criterion requires the runbook template, including wall
time and disk delta.

It also requires artifacts-left-behind and all positive/negative checks at the
prescribed boundary.

The tested prompt referred to a file rather than embedding the install section.

The result therefore cannot be compared cleanly with a later closing leg.

Decision: reject as a baseline leg.

Retain it as a named pre-baseline probe.

## Option 4 — rerun both providers autonomously from this session

The Docker image exists locally.

The retained box contains valid auth state for both providers.

An implementation agent could start containers and invoke provider CLIs.

This would consume live model tokens and mutate external account state.

Reusing the retained auth box violates the fresh-container measurement rule.

Copying its auth directories into fresh boxes violates the runbook explicitly.

Fresh authentication requires human browser/device action.

The ticket calls for John to operate the trial and for this agent to prepare,
verify, and record it.

Running provider sessions here would cross both the authorization and evidence
role boundaries.

Decision: reject.

## Option 5 — ask John to run the protocol before producing any artifacts

The strictest response would stop immediately and request the two manual runs.

That would preserve the evidence boundary.

It would leave the retained Codex probe undiscovered and undocumented.

It would also skip the assignment's instruction to work through all remaining
RDSPI phases in one pass.

The workflow supports a blocked Review disposition after all phases are
documented.

Decision: reject as the overall approach.

Human runs remain the unblock action, but the current evidence is mapped first.

## Option 6 — partial-evidence artifact plus blocked disposition

Create a dedicated artifact that records only facts recoverable from the
retained transcript and Docker metadata.

Label it a pre-baseline probe, not leg B acceptance evidence.

Record every deviation that prevents admission.

Record exact strings from the transcript within the available evidence.

Map known findings to existing E-046 tickets.

File any uncovered finding as a new ticket.

Produce all RDSPI phase artifacts and end Review with a block reason naming the
two exact required human runs.

Decision: choose.

This uses the available evidence fully while honoring the ticket's explicit
fail-closed rule.

## Finding-disposition design

Channel skew is already owned by T-046-03-02.

The move from Rust-shaped `~/.cargo/bin` to `~/.local/bin` and its PATH wording
is already owned by T-046-03-01.

Managed Zellij acquisition is already owned by T-046-02-01 and T-046-02-02.

Compiler-steering documentation and remedy strings are already owned by
T-046-04-01 and T-046-04-02.

The observed installer failure when `xz` is absent is not explicitly owned by
those acceptance criteria.

The fixture intentionally excludes xz, so installing xz as setup would weaken
the acceptance instrument.

The current cargo-dist installer consumes `.tar.xz` archives.

Static-musl changes address runtime linkage, not extraction-tool availability.

Design decision: file T-046-03-03 under the static-linux-artifacts story.

The ticket will require the README shell installer to succeed in the no-xz
fixture without installing xz or a toolchain.

It will preserve the one-command user interface rather than prescribing the
underlying archive or decompressor mechanism.

## Artifact design

`research.md` maps the existing fixture, runbook, retained evidence, gaps, and
ticket relationships.

`design.md` records why the partial-evidence/block approach is selected.

`structure.md` defines the exact private artifacts and the one new ticket file.

`plan.md` sequences extraction, finding filing, validation, and Review.

`baseline-probe.md` is the evidence-specific implementation artifact.

`progress.md` records execution and source transaction status.

`review.md` provides the reviewer handoff.

`review-disposition.json` contains the exact block shape required by Lisa.

The assignment, launch helper, and phase artifacts remain private to this
attempt until Lisa decides what to admit.

## Evidence labeling rules

Use "probe" for the retained Codex interaction.

Do not call it "leg B complete," "baseline complete," or "measured baseline."

Use "derived transcript interval" for the approximately 234.7-second duration.

Do not call that number the runbook wall clock.

Use "final writable-layer size" for Docker's 117,821,440-byte value.

Do not call that value disk delta.

Quote transcript text verbatim and identify whether it came from README,
installer, doctor, agent, or shell.

Do not copy credential files, auth output, environment dumps, or complete
provider state into the repository.

## Pass/block rule

A pass requires two runbook-template records, one for each primary model leg.

Each must have exact model id, separate fresh container identity, prompt
conformance, wall time, disk delta, independent positive exits, independent
negative checks, artifacts, exact strings, and ticket links.

The absence of either leg is sufficient to block.

Missing wall or disk measurements cannot be waived retrospectively.

The current state is missing both an admissible Codex leg and any Claude leg.

The final disposition must therefore be block.

## Unblock contract

John builds or identifies the fixture image and runs the host preflight.

John starts a fresh capped container for Claude/Haiku.

John authenticates Claude inside that container without a host mount.

John follows instruction A and all snapshot/acceptance steps.

John records or supplies the complete result-template fields and sanitized
transcript excerpts.

John repeats the same process in a separate fresh capped container for
Codex/mini.

The evidence recorder then verifies each field against the retained containers
or supplied sanitized output.

Only after both records exist can Review change to pass.

## Trade-offs

Blocking leaves T-046-06-03 and epic closure waiting.

That delay is intentional because the baseline loses its value once rewritten
documentation or a stable release replaces the pre-fix surface.

The partial probe still preserves unique field evidence that might otherwise be
lost.

Filing the xz gap prevents the closing run from rediscovering an already
observed prerequisite failure without ownership.

No product source is changed by this evidence ticket.

The only shared source unit is the newly filed finding ticket.

## Design conclusion

The chosen design produces an honest, reviewable blocked handoff.

It preserves real transcript evidence, refuses invented measurement values,
routes known failures to their existing owners, and gives the uncovered xz
failure its own owner.

The terminal block reason will name the missing human-operated Claude/Haiku and
Codex/mini runbook records as the actionable condition.
