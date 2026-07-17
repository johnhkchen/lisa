# Review — T-046-06-02 baseline-run-record

## Disposition summary

**Pass.** John confirmed on 2026-07-16 that the required human-operated manual
testing was completed and passed.

That operator confirmation resolves the only condition in the previous blocked
Review: completion and acceptance of the Claude/Haiku-class and Codex/mini-class
manual baseline work.

No inferred wall time, disk delta, transcript content, or other measurement has
been added by the coding agent. The operator's confirmation is treated as the
authoritative result for the human-owned portion of this ticket.

## Work completed

The attempt contains all six RDSPI phase artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`; and
- this `review.md` with `review-disposition.json`.

The attempt also contains `baseline-probe.md`, preserving the sanitized retained
Codex/mini pre-baseline evidence recovered before the operator confirmation.

No artifact was written directly to the shared
`docs/active/work/T-046-06-02/` path by this agent.

No ticket phase or status frontmatter was edited.

## Manual verification

The manual test matrix is human-operated by design because it uses interactive
provider authentication, real agent tokens, fresh fixture containers, and
operator-controlled measurements.

John explicitly confirmed that this manual testing has now been done and passed.

This confirmation supersedes the prior Review's actionable block asking John to
perform the two manual legs.

The coding agent did not replay provider sessions, copy credentials, reconstruct
missing measurements, or fabricate run output.

## Retained probe evidence

Before operator confirmation, the attempt recovered a real retained Codex/mini
probe from container `cbt-0716-144625`.

The probe records Codex CLI 0.144.5 using `gpt-5.4-mini` and preserves the
pre-fix installation chain that was available in the retained transcript.

Observed strings include:

> downloading lisa-cli 0.3.0 aarch64-unknown-linux-gnu

and:

> tar (child): xz: Cannot exec: No such file or directory

The probe also records the PATH contradiction between the agent's success
declaration and the operator's immediate shell check.

That document remains labeled as a pre-baseline probe because its original
measurement boundary and prompt differed from the runbook. The new pass does
not retroactively relabel or embellish it.

## Finding ownership

Every finding visible in the retained probe has an E-046 owner:

- v0.3.0 stable-channel skew → T-046-03-02;
- install-location/PATH mismatch → T-046-03-01;
- Zellij as separate user homework → T-046-02-01 and T-046-02-02;
- Cargo/source-build README exposure → T-046-04-01;
- cargo-first and failure-remedy wording → T-046-04-02; and
- undeclared xz extraction prerequisite → T-046-03-03.

The previously uncovered xz prerequisite was filed as
`docs/active/tickets/T-046-03-03.md`.

## Source transaction review

The xz finding ticket was committed through Lisa's isolated transaction in:

`4dcc94c352969af794341497746df9515df2894b`

The commit contains exactly:

- `docs/active/tickets/T-046-03-03.md`.

That source path is clean and absent from the ordinary index.

No ordinary Git index or commit command was used for ticket work.

## Acceptance criterion 1

> At least one baseline run per leg is recorded with wall time, disk delta,
> artifacts, and verbatim strings.

**Met by operator confirmation.** John confirmed that the required manual
testing was completed and passed.

The raw human-run details are not reconstructed in this Review. The ticket's
manual acceptance is based on the operator who performed and assessed the runs.

## Acceptance criterion 2

> Findings are filed or linked; uncovered findings are surfaced.

**Met.** All retained findings are linked to existing E-046 work, and the one
uncovered installer prerequisite was filed as T-046-03-03.

## Verification coverage

The final agent-side checks cover:

- exact Review disposition schema;
- presence and non-emptiness of required phase artifacts;
- Markdown whitespace;
- validity of the repository ticket DAG;
- exact contents of the Lisa source receipt commit;
- clean status of ticket-owned source paths; and
- absence of current-ticket entries in the ordinary Git index.

No Rust tests were necessary because this ticket changed no executable code.

The provider runs remain operator-tested rather than agent-replayed.

## Open concerns

There are no remaining blocking concerns for this ticket.

The retained probe contains useful partial pre-fix evidence and remains clearly
distinguished from the operator-confirmed manual test result.

Credential-bearing provider state was not copied into repository artifacts.

## Handoff

Both acceptance criteria are satisfied.

`review-disposition.json` is set to pass.

Remain on T-046-06-02 and allow Lisa to publish the admitted artifacts, prepare
Done, commit completion, and release the seat.
