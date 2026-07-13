# Design: Gate completion on explicit pass

## Decision goal

Automated Review completion needs one auditable authorization boundary:
lease-validate and publish the current attempt's disposition, parse it with the
core model, and call `request_completion` only for `ReviewDisposition::Pass`.

The design must preserve the existing atomic completion transaction for pass,
keep block and invalid outcomes assigned in Review, expose refusal reasons to
operators, and avoid weakening manual operator completion.

## Option 1: inline matches at each caller

Each named caller could locate `review-disposition.json`, call the parser, and
match `Pass` before invoking `request_completion`.

Advantages:

- minimal control-flow indirection;
- each site visibly contains the explicit pass check;
- no new method signature.

Costs:

- artifact admission, path selection, diagnostics, and exhaustive matching are
  duplicated;
- the idle-signal Review edges could easily diverge or remain ungated;
- future automated completion sites might call the lower-level transaction
  directly and omit the contract;
- duplicate messages make operator behavior inconsistent.

This is viable but does not provide a durable single boundary.

## Option 2: gate inside `request_completion`

`request_completion` could parse a disposition whenever the source is
Artifact, Idle, or Stopped, while exempting Manual and ObservedDone.

Advantages:

- all automated callers are covered centrally;
- existing caller code changes little;
- the transaction function itself cannot be reached automatically without a
  passing document.

Costs:

- `request_completion` is a general commit-transaction primitive, not purely a
  Review-disposition consumer;
- its authority enum does not directly expose a staged artifact path without
  additional matching and publication behavior;
- source classification becomes authorization policy, coupling diagnostic
  provenance to artifact semantics;
- tests that exercise the transaction primitive directly would require review
  artifacts even when testing lease fencing or result handling;
- manual and observed-Done exceptions would become embedded negative policy.

This centralizes too low in the stack and broadens the behavioral blast radius.

## Option 3: add a Review-specific request method

Add a private `request_review_completion` method above `request_completion`.
It accepts the ticket ID, completion source, and optional attempt lease.

The method:

1. admits `review-disposition.json` through the existing lease-aware artifact
   boundary;
2. treats absence or admission failure as a visible safe refusal;
3. parses the admitted canonical file with `parse_review_disposition`;
4. on `Pass`, delegates unchanged to `request_completion` with the same source
   and attempt authority;
5. on `Block`, logs the actionable reason and returns false;
6. on `Invalid`, logs the diagnostic reason and returns false.

All automated Review-to-Done sites call this method. Manual operator completion
continues to call `request_completion` directly.

Advantages:

- one explicit semantic boundary shared by polling, idle, and stopped paths;
- lower-level transaction tests and reconciliation behavior remain unchanged;
- admission and parsing order is uniform;
- `Pass` preserves the existing atomic transaction without duplication;
- block and invalid diagnostics share wording and return behavior.

Costs:

- introduces one small internal layer;
- callers must supply an attempt lease rather than a prebuilt authority;
- the method must distinguish missing admission from parsing failure.

This option best matches the codebase's existing separation between artifact
admission and transaction preparation and is selected.

## Admission-before-parse decision

The helper will call `admit_artifact` before parsing.

This ordering validates the attempt lease before any disposition can authorize
completion. It also copies the exact reviewed disposition into canonical work,
which is the directory included by the later completion transaction.

Parsing only the staged path would validate content but omit disposition
publication unless another mechanism copied it. Parsing canonical state before
admission could accidentally consume stale bytes from an earlier attempt when
the current attempt has no disposition.

Admission results are handled as follows:

- `Ok(true)`: parse the canonical destination;
- `Ok(false)`: refuse as missing required disposition and log the canonical
  filename/ticket context;
- `Err(reason)`: refuse and log the admission/lease failure.

The helper never falls back from a missing current-attempt file to an existing
canonical file. `admit_artifact` already implements the safe legacy-unleased
case, where canonical presence itself returns `Ok(true)`.

## Outcome policy

The core enum is matched exhaustively.

`Pass` is the sole branch that calls `request_completion`.
The call receives the same ticket ID, source, and attempt lease transformed to
`CompletionAuthority::Attempt`. This preserves pending state, command
construction, dependency checks, current-lease checks, and commit-result
publication unchanged.

`Block { reason }` logs a warning containing the ticket ID and exact reason.
It does not alter the thread, slot, lease, ticket file, DAG, or pending map.
The return value is false.

`Invalid { reason }` logs an error containing the ticket ID and parser reason.
It likewise leaves all scheduler ownership and phase state unchanged and
returns false.

Missing and admission errors are invalid evidence even though they do not reach
the parser. They use the same refusal behavior and visible diagnostic class.

## Logging decision

Use `ActivityEvent::Warning` for a valid explicit block.
The agent has supplied trustworthy evidence, but the operator must act on it.

Use `ActivityEvent::Error` for invalid, missing, unreadable, or inadmissible
evidence. These represent a broken Review contract or authority problem rather
than an intentional reviewed outcome.

Both messages begin with a stable phrase such as “Completion blocked” or
“Completion refused” and include the ticket ID. Tests inspect meaningful
substrings instead of pinning full parser error text.

No new alert vector or UI model is added. The existing activity stream is
already rendered in the dashboard and is used by other completion refusals.

## Automated caller coverage

The selected design changes all automated Review-to-Done calls, not only the
two line references in the ticket:

- `check_artifact_advances` Review next-phase branch;
- `check_idle_signals` Implement-to-Review catch-up branch;
- `check_idle_signals` direct Review next-phase branch;
- `auto_complete_review` stopped-session branch.

This is a direct application of the stated invariant “at both review->Done
sites” to the actual current code, where idle compatibility paths provide two
additional entries into the same semantic edge.

The following remain direct lower-level calls:

- manual operator completion;
- reconciliation of an externally observed Done ticket;
- focused transaction tests.

These are not automated agent Review approval.

## Regression test design

Add a table-driven scheduler test that exercises block, pass, and invalid
documents against the artifact polling consumer.

Each case creates a two-ticket DAG:

- `T-REVIEW` is assigned to a running Review thread;
- `T-DEPENDENT` depends on `T-REVIEW` and remains ready-phase/open.

The fixture installs a current attempt, writes `review.md`, and writes the case
disposition into the private attempt directory.

For block, assert:

- no pending completion exists;
- Review thread remains present/running and assigned to its slot;
- ticket file and DAG remain Review/not Done;
- dependent remains blocked;
- activity contains the exact actionable reason.

For pass, assert:

- one pending completion exists with Artifact source and current authority;
- Review thread and assignment remain until transaction result;
- ticket file remains Review, demonstrating existing atomic behavior;
- disposition is admitted to canonical work.

For invalid, assert:

- no pending completion exists;
- ticket and assignment remain Review;
- dependent remains blocked;
- activity contains a visible refusal diagnostic.

Add or update a stopped-session test so `auto_complete_review` also proves
block refusal and pass admission. Existing positive tests must write explicit
pass dispositions to retain their original transaction assertions.

## Rejected extensions

A persistent `review_block_alerts` state collection was considered but rejected
because it requires UI-state mapping, dismissal semantics, and lifecycle
cleanup beyond this ticket. Activity logging meets operator visibility using an
existing rendered surface.

Moving parsing into `lisa-plugin` was rejected because the core typed parser is
already the dependency-provided single source of truth.

Changing `request_completion` to accept `ReviewDisposition` was rejected
because manual and reconciliation sources do not derive authority from an
agent-authored Review artifact.

Deleting or renaming a blocked disposition after reading was rejected. The
artifact is evidence and should remain available for operator inspection and a
later corrected attempt.

## Compatibility and risk

Passing automated reviews gain one required artifact, already mandated by the
workflow contract. Their transaction behavior after authorization is unchanged.

Older agents that write only `review.md` now fail closed and remain visible in
Review. This is the intended compatibility break: absence cannot mean pass.

Repeated polling can repeat a block/refusal activity entry. The log is bounded,
and repeated visibility is preferable to silent failure. Debouncing would need
state keyed by disposition contents and is not required for correctness.

The principal implementation risk is failing to update an existing positive
fixture, which will manifest as an expected test failure and be corrected by
adding the explicit pass document rather than weakening the gate.
