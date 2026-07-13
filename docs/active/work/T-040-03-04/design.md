# Design: authorized Codex field report

## Decision summary

Run two independent live Codex fixtures through the exact T-040-03-03 binary.
Retain evidence in this attempt-private directory,
destroy both Zellij sessions, fixture repositories, and ephemeral Codex homes,
then write a forensic field report in `progress.md`.

The cases are intentionally separate:

1. a Codex-authored valid blocking Review with a dependent ticket;
2. a Codex process interrupted before ownership through bounded recovery failure.

No deterministic suite is re-executed.
The report cites the admitted T-040-03-03 rebuild evidence as deterministic proof.

## Option 1: infer field behavior from the current outer Lisa run

The current ticket is itself executing on a Codex seat.
Its lifecycle could be inspected and described.

Advantages:

- no additional metered session;
- existing dashboard and provenance data;
- no harness construction.

Disadvantages:

- the assignment explicitly excludes the outer loop as acceptance evidence;
- it cannot safely manufacture a blocking Review for itself;
- it cannot intentionally fail before ownership because ownership already exists;
- teardown would conflict with the active ticket lease;
- active repository tickets would become test inputs.

Rejected because it violates the stated evidence boundary.

## Option 2: rely only on predecessor deterministic regressions

The two named regressions already pin the required behaviors.

Advantages:

- fast and reproducible;
- no provider cost or timing variability;
- exact internal assertions.

Disadvantages:

- does not launch live Codex;
- does not instantiate the rebuilt WASM in Zellij;
- does not observe provider acknowledgement or pane lifecycle;
- fails the explicit authorized live requirement.

Rejected as incomplete acceptance evidence.
The results remain the deterministic half of the final report.

## Option 3: one fixture containing both cases

A single repository could contain a blocking ticket and a later failure ticket.

Advantages:

- one loop and one evidence root;
- may observe resident-seat reuse;
- lower startup overhead.

Disadvantages:

- a blocking Review retains the only assignment and seat;
- the second ticket cannot start naturally with `max_threads = 1`;
- increasing concurrency entangles timing and pane identification;
- manual reset or ticket mutation would weaken isolation;
- teardown and outcome attribution become harder.

Rejected because independent fixtures produce clearer causal evidence.

## Option 4: two independent disposable fixtures

Each fixture has a new external Git repository,
one named Zellij session,
one ephemeral Codex home,
and a one-thread scheduler.

Advantages:

- cases cannot interfere;
- exact build binding is repeated and inspectable;
- blocking retention cannot starve the pre-ownership case;
- ticket and pane identities stay unambiguous;
- teardown is bounded per case;
- evidence can be evaluated separately.

Disadvantages:

- two provider launches;
- more harness code;
- live timing still needs careful observation.

Selected because it best satisfies isolation and forensic clarity.

## Blocking Review case design

The fixture contains `T-LIVE-BLOCK` and dependent `T-LIVE-DEPENDENT`.
Only the first is initially eligible.
Its assignment requires all six concise RDSPI artifacts,
no product source modifications,
and the exact valid blocking JSON shape with an actionable harness reason.

The expected terminal live state is retained Review.
The harness waits for the attempt-private disposition,
then waits for the published disposition and `review.md`.
It allows a short settling interval so an erroneous completion attempt is observable.

Assertions:

- Codex was launched and a matching acknowledgement exists;
- `owned` was observed;
- the exact block disposition is retained;
- the primary ticket is not Done;
- the dependent is not scheduled or Done;
- no authoritative Done row exists;
- no completion commit exists after baseline;
- the structured reason is present in final dashboard evidence.

If the model emits a pass disposition despite the explicit fixture assignment,
the live case fails rather than editing the artifact.

## Pre-ownership case design

The fixture contains only `T-LIVE-PREOWN`.
The assignment is ordinary and would complete if allowed to own.
The harness continuously discovers the ticket pane and dashboard state.

When a Codex ticket pane exists while no `owned` state has been observed,
the harness closes that pane through Zellij.
It repeats for bounded replacement panes.
The plugin remains alive and owns retry policy.

The harness never changes scheduler timeout values after launch
and never writes a failure record itself.
It waits for a durable assignment-transition row whose state is terminal.
Then it invokes the exact rebuilt CLI's status surface against the physical ledger.

Assertions:

- at least one real Codex pane launched;
- `owned` was never observed;
- pane interruption happened only before ownership;
- a terminal pre-ownership row exists;
- the row identifies ticket, attempt, pane, provider, named state, reason, and time;
- CLI output renders that row;
- no authoritative terminal execution row exists;
- no Done commit exists.

If Codex owns before the harness can interrupt it,
the case is anomalous and blocks rather than being retried invisibly.

## Build binding

The harness requires an explicit absolute `LISA_BIN`.
It refuses to run unless the SHA-256 equals the T-040-03-03 CLI hash.
It records version and size.

After loop launch it copies the generated layout,
extracts the embedded plugin path,
and verifies that plugin SHA-256 equals the T-040-03-03 WASM hash.
This binds host execution to both rebuilt artifacts.

No build command runs in this ticket.
The recorded hashes are acceptance inputs, not recomputed build claims.

## Evidence retention

The durable evidence directory is:

`.lisa/attempts/T-040-03-04/1/work/live-evidence/`.

Each case receives:

- case metadata;
- state and pane-event ledgers;
- dashboard and terminal snapshots;
- final screens and pane JSON;
- generated layout and build identity;
- signal copies;
- ticket and artifact copies;
- provenance and rendered CLI status;
- Git log/tree/status;
- teardown receipt.

The harness script itself is retained beside the evidence.
That makes the observation reproducible without treating it as product source.

## Cleanup design

An EXIT trap always kills the current named Zellij session,
waits for the outer loop process,
removes every fixture root,
and removes every ephemeral Codex home.

Before deletion, targeted evidence is copied out.
After deletion, the harness writes a teardown receipt
that asserts the session name is absent and paths no longer exist.

A teardown failure changes the overall disposition to block.
Successful cases are not retained as hidden live roots.

## Anomaly policy

The harness never patches the rebuilt scheduler or fixture artifacts.
Any of the following blocks Done:

- build hash mismatch;
- no live Codex launch;
- missing or mismatched acknowledgement in the Review case;
- Review case completion despite block;
- pre-ownership case reaching owned;
- missing, duplicate, or malformed failure row;
- fabricated authoritative execution outcome;
- unexplained state transition;
- fixture or Zellij residue;
- inability to render the row through `lisa status`.

The report describes failures as observations and names follow-up action.

## Selected outcome

Implement the two-case attempt-private harness,
execute it once against the exact release artifacts,
evaluate evidence without remediation,
and pass Review only if every live and cleanup assertion succeeds.
