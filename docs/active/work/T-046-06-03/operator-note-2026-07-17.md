# Operator note — manual resolution, 2026-07-17

## What happened

The 2026-07-17 morning loop's codex reviewer blocked this ticket with a legacy-format
disposition (bare `reason`, no structured ask). Verbatim:

> The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB
> gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant
> bypassed the old binary through managed mode instead of recording the required recovery
> through Lisa's error strings; John must either provide conforming reruns or explicitly
> amend both acceptance requirements before Review can pass.

The block was then **orphaned**: the session ended before the park policy (which requires a
live Running thread holding the current lease) could fire, so the ticket sat at
`phase: review, status: open` — invisible to Waiting-on-you, yet still schedulable, and the
next loop re-seated a reviewer to re-derive a verdict already on disk. This incident is the
field evidence for S-049-05 (parked-means-parked).

## The decision

The reviewer was right that the documents disagreed, and offered the remedy itself:
*"explicitly amend both acceptance requirements."* The operator (John, executed via Claude)
took that remedy:

1. **Disk bound**: the ~200 MiB figure predated calibration. The real closing legs measured
   186 MiB (claude) and 225 MiB (codex); the runbook bound was recalibrated to 300 MiB with
   recorded composition on 2026-07-16 (commit b303ccc), *before* this review ran. The
   ticket/story text now cites the calibrated bound instead of the pre-evidence guess.
2. **Old-Zellij variant**: the platform-aware managed default means the seeded 0.40.1 binary
   is never consulted — the recorded variant (cbt-0716-211533-variant-oldzellij) shows the
   hazard designed out of reachability, a strictly stronger outcome than string-guided
   recovery. The criterion now accepts either outcome, with error strings remaining the
   fallback contract for pinned system mode.

No run evidence was altered. Both amended sentences cite their evidence. The review
disposition is set to pass by the operator on the amended criteria; the prior block text is
preserved above and in git history.
