# Field report: authorized Codex Review and pre-ownership observations

## Verdict

**BLOCKING Done.**

The exact rebuilt Lisa CLI/WASM was instantiated in two isolated disposable
repositories and both live Codex behaviors were exercised.

The pre-ownership case is complete and passes every evidence assertion.
It launched a real Codex pane, interrupted it before ownership,
retained exactly one durable `delivery-failed` assignment-transition row,
rendered the same row through the rebuilt CLI,
created no execution outcome or commit,
and removed its fixture, Codex home, and named session.

The blocking Review case produced strong live evidence:
a real Codex session acknowledged the exact generation-1 assignment,
wrote all six artifacts,
and emitted the exact valid blocking disposition.
However, the harness's UI state sampler missed the brief literal `owned` label
and its strict assertion terminated before it copied the fixture's final ticket,
provenance, dependent, and Git snapshots.
The fixture was then removed by the cleanup trap.

That evidence-capture gap prevents this report from independently proving the
full post-stop negative outcome for the live blocking case.
The run is therefore not admitted as a passing field gate.
Per the ticket, it is reported as blocking rather than patched or silently rerun.

## Execution boundary

This report cleanly separates inherited deterministic proof from live observation.
No predecessor test, build, format, Clippy, or workspace gate was re-executed.
No product source was modified.
No active Lisa ticket in this repository was changed to manufacture either case.

Synthetic fixture tickets existed only in external temporary Git repositories:

- `T-LIVE-BLOCK` and `T-LIVE-DEPENDENT`;
- `T-LIVE-PREOWN`.

Each fixture used one scheduler thread and an ephemeral authenticated Codex home.

## Exact rebuild binding

The harness required an explicit absolute `LISA_BIN`:

```text
/Users/johnchen/swe/repos/lisa/target/release/lisa
```

Observed version:

```text
lisa 0.4.0-rc.7
```

CLI SHA-256:

```text
498134e92f43ea5a3d834c5cb22afdf5d6ad180e2543ae543b4ae84588addfe9
```

Release WASM SHA-256:

```text
053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
```

Both equal the identities recorded by T-040-03-03.
The harness refused to run on a mismatch.

Each live fixture copied its generated `.lisa-layout.kdl`,
extracted the instantiated temporary plugin path,
and hashed that actual file.
Both extracted files matched the same expected WASM digest.
Both layouts named the exact absolute rebuilt CLI path.

Host observations:

```text
source HEAD: 3f995394fbba9ec29cb5bf6e1575ca55ef2ee108
Zellij:     0.44.3
Codex CLI:  0.144.1
model:      gpt-5.6-sol (blocking Review acknowledgement)
```

## Deterministic proof — inherited, not rerun

T-040-03-03 rebuilt from settled source revision:

```text
48b9bf80ca59013e7e46f1010c4ac04623762890
```

Its admitted `review.md` and private `rebuild.md` record:

- release WASM and CLI builds passed;
- the build-script copy exactly matched the release WASM;
- formatting passed;
- native and WASM warning-strict Clippy passed;
- 794 workspace tests passed;
- `just check` passed;
- no unexplained rebuild anomaly occurred.

The blocking discriminator was:

```text
test_t039_06_02_blocking_review_never_prepares_done
```

It proves deterministic retention of assignment and lease,
no pending completion,
no Done provenance,
and dependent blocking for a valid blocking disposition.

The pre-ownership discriminator was:

```text
rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

It proves deterministic production timeout behavior,
physical schema-v3 ledger persistence,
and CLI reconstruction from the same row.

These are regression facts, not claims about the live provider runs below.

## Live observation A — blocking Review

### Fixture identity

```text
ticket:     T-LIVE-BLOCK
dependent:  T-LIVE-DEPENDENT
attempt:    1
session:    l40b-64236
fixture:    external disposable temporary Git repository
```

The fixture ticket required all six RDSPI artifacts,
no product source change,
and this exact disposition:

```json
{"disposition":"block","reason":"field harness intentionally requires operator clearance"}
```

### Live provider evidence

The retained `started.json` binds the live start to:

```json
{"ticket_id":"T-LIVE-BLOCK","attempt_id":1}
```

The retained `lease.json` carries the same ticket and attempt.

The retained `ack.json` is a real Codex `UserPromptSubmit` hook payload.
It contains:

- a Codex session ID;
- a Codex turn ID;
- model `gpt-5.6-sol`;
- the canonical fixture working directory;
- the exact `LISA_ASSIGNMENT` prompt;
- ticket `T-LIVE-BLOCK`;
- generation `1`.

This is stronger ownership-boundary evidence than terminal text alone:
the rebuilt scheduler only promotes the matching acknowledged generation.

The live terminal snapshots show Codex reading `CLAUDE.md`,
the private `assignment.md`, the RDSPI workflow, and the fixture ticket.
Codex wrote:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

The retained Review confirms the intentional block.
The retained structured disposition matches the required exact object.
The terminal snapshot shows Codex verifying every file and printing the JSON.

### What the live evidence proves

- the exact rebuilt CLI instantiated the exact rebuilt WASM;
- a real Codex process launched in the isolated fixture;
- attempt 1 received and acknowledged generation 1;
- Codex completed all six phases;
- the structured disposition was valid and blocking;
- the provider did not rewrite the requested block to pass;
- the session and fixture were disposable and external to this repository.

### Evidence-capture failure

The harness sampled the full Zellij screen by focusing the plugin pane.
It copied the matching acknowledgement,
but did not retain a sample containing the short-lived literal `owned` label.
Its assertion required that UI string even though the matching acknowledgement
and completed phase work already establish the ownership boundary.

That assertion ran before common final capture.
It terminated the case before copying:

- final `T-LIVE-BLOCK` frontmatter;
- final dependent frontmatter;
- final `.lisa/provenance.jsonl`;
- final Git log/tree/status;
- final published artifact listing.

The cleanup trap correctly removed the fixture and Codex home,
so those missing negative-outcome records cannot be reconstructed afterward.

The report therefore does **not** claim live proof that:

- the ticket remained non-Done after the provider stopped;
- authoritative Done provenance was absent;
- the dependent remained unscheduled;
- the baseline remained the only fixture commit.

Those behaviors remain deterministically proven by the named regression,
but deterministic proof cannot substitute for the requested live observation.

No second metered blocking run was launched merely to replace the missing capture.
That honors the assignment's no-redundant-reexecution boundary.

## Live observation B — pre-ownership failure

### Fixture identity

```text
ticket:    T-LIVE-PREOWN
attempt:   1
session:   l40p-84882
pane:      terminal_0 / pane_id 0
baseline:  ef8237c08c2675814262b566a38cf306d01de875
```

The ticket was an ordinary artifact-only Codex ticket.
It did not encode a provider failure.
The hostile input came from the external harness closing the live Codex pane
before any `owned` observation.

### Transition chronology

The harness recorded:

```text
2026-07-13T02:50:26.3NZ  discovered-live-codex-pane  terminal_0
2026-07-13T02:50:26.3NZ  closed-before-ownership    terminal_0
```

No `owned` event appears in the state ledger.
No artifact was written by the ticket.
The ticket remained `status: open`, `phase: research`.

The rebuilt scheduler applied its own bounded policy.
The harness did not write the ledger row,
change retry counts,
or fabricate a provider outcome.

### Durable assignment-transition row

After 40 seconds, the physical fixture ledger contained exactly one matching row:

```json
{"schema_version":3,"record_type":"assignment-transition","ticket_id":"T-LIVE-PREOWN","attempt_lease":{"ticket_id":"T-LIVE-PREOWN","attempt_id":1},"pane_id":0,"provider":"openai","state":"delivery-failed","reason":"provider did not acknowledge the bounded chat assignment","started_at":1783911025,"ended_at":1783911065,"wall_clock_secs":40}
```

The harness asserted:

- schema version 3;
- record type `assignment-transition`;
- exact ticket and attempt identity;
- pane ID 0;
- provider `openai`;
- terminal state `delivery-failed`;
- nonempty reason;
- monotonic timestamps;
- wall clock equal to `ended_at - started_at`;
- exactly one matching assignment row;
- no row carrying an execution `outcome` for the ticket.

All assertions passed.

### CLI reconstruction

The pinned rebuilt CLI rendered the physical ledger as:

```text
Pre-ownership failures for T-LIVE-PREOWN (1):
Attempt 1 (pane 0)
  state: delivery-failed
  reason: provider did not acknowledge the bounded chat assignment
  provider: openai
  started_at: 1783911025
  ended_at: 1783911065
  wall_clock_secs: 40
```

The harness compared the rendered state and reason with the JSONL row.
Both matched.

### Git and outcome evidence

The fixture baseline and final HEAD were identical:

```text
ef8237c08c2675814262b566a38cf306d01de875
```

The Git log contained only the disposable baseline commit.
The ticket remained open in Research.
There was no Done commit.
The mixed ledger contained no execution outcome for this ticket.

This is a truthful pre-ownership failure,
not a fabricated failed model execution.

## Teardown

Both fixture repositories and both ephemeral Codex homes are absent.
Both live Zellij server processes were killed.

The harness initially used `zellij kill-session` and checked immediately.
Later audit found the blocking session name retained as an exited,
resurrectable Zellij metadata entry.
The pre-ownership receipt had the same immediate-check limitation.

Both named entries were explicitly deleted with `zellij delete-session --force`.
Final `zellij list-sessions --short` contains zero `l40*` entries.
No fixture root or ephemeral Codex home remains.

The initial teardown receipt was therefore optimistic about metadata deletion,
although the live processes and fixture data were already removed.
The final cleanup state satisfies physical teardown.

## Harness-only setup retries

Before the admitted observations, three harness setup defects were found:

1. a too-long Zellij session name was rejected before a Codex process started;
2. optional signal absence returned nonzero under strict shell mode before assignment;
3. macOS Bash rejected an associative array before pane interruption.

Each failure occurred before the corresponding provider behavior under test.
They were corrected only in the private harness.
No Lisa product source or fixture outcome was patched.

The later blocking UI assertion is different:
it occurred after a real metered Codex turn and caused a material evidence gap.
That gap is not dismissed as setup noise.

## Repository ownership and hygiene

No product source file was created, modified, staged, or committed by this ticket.
No `lisa commit-ticket` source unit was necessary.
No ordinary `git add` or `git commit` was run in this repository.

Current non-attempt status consists of Lisa-managed lifecycle files,
Lisa-published phase copies,
and pre-existing plugin-relative fixture residue.
Those paths were not used to manufacture either live case.

## Acceptance assessment

The following requirements are satisfied:

- isolated external disposable harnesses;
- exact T-040-03-03 CLI/WASM binding;
- real live Codex blocking disposition generation;
- real live Codex pre-ownership failure;
- retained lease/start/ack evidence for the blocking case;
- retained structured blocking disposition and Review;
- retained transition and pane ledger evidence;
- exactly one durable pre-ownership row;
- CLI reconstruction from the physical ledger;
- no pre-ownership execution outcome or commit;
- physical teardown of sessions, fixtures, and Codex homes;
- deterministic/live evidence separation;
- no active-ticket mutation to manufacture cases;
- no product patching in response to observations.

The following requirement is not fully evidenced:

- live post-stop proof that the blocking Review produced no Done commit,
  no authoritative provenance, and no dependent scheduling.

Because that missing evidence is material to the ticket's core claim,
the final disposition must block.

## Action required to unblock

Authorize a narrowly scoped replacement blocking-Review observation
whose harness captures ticket, dependent, provenance, and Git snapshots
before any sampled-UI assertion and deletes, rather than only kills,
the named Zellij session.

The replacement must remain bound to the exact recorded rebuild
or a newly authorized rebuild if the source has changed.
It must not alter scheduler behavior while gathering evidence.
