# Lisa + Vend Turbo Mode — deadline-shaped clearing and execution

**Status:** working thesis and field protocol  
**First field test:** Battle Bots Hack Night, 2026-07-15  
**Scope:** a proposed opt-in operating profile for vend + lisa; not a change to normal mode

## The problem in one sentence

The ordinary vend → lisa system is good at turning understood intent into trustworthy,
long-running autonomous throughput; it does not yet have a gear for turning a fuzzy idea into one
convincing live experience before a fixed deadline a few hours away.

That distinction matters. The normal loop asks, “What is the right body of work, and how do we
finish it safely without a human in the loop?” A sprint asks, “What is the shortest honest path to
the core moment, what can happen in parallel, and what must be cut now so the core moment is live
with rehearsal time left?”

Turbo mode should be a different optimization profile over the same two engines:

- **vend still clears intent into allocatable work**, but clears against a deadline and a demo
  outcome rather than completeness;
- **lisa still executes a dependency graph into atomic commits**, but schedules the critical path,
  protects integration capacity, and reacts to shrinking time rather than merely draining every
  ready ticket;
- **normal mode remains unchanged.** Turbo is explicitly selected, measured, and disposable after
  the timebox.

## Why this surfaced now

Demo Runway compressed months of reusable know-how into a strong template: exhibit contracts,
live state, replay, rendering, model-player seams, tests, deployment machinery, and operational
checks. The event-specific Drone Arena remains weak by comparison because it represents the work
that must fit into one event session. That is not evidence that the template failed. It exposes the
next reusable layer: the machinery for rapidly breaking an unstructured idea across cooperating
agents and continuously converging their output into a live product.

The Battle Bots event is a useful natural experiment because the rule is unusually clean:

- the reusable chassis and operating know-how may exist beforehand;
- the event exhibit must be built during the event;
- the public session is roughly four hours, including kickoff, integration, rehearsal, and demo;
- success is observable: a stranger can watch one bot battle another before the deadline.

We should prepare the method and measurement now, not the event implementation.

## Field evidence from the runway build

The final E-014/E-015/E-016 runway slice is the nearest baseline. From
`../boilerplate-demo/.lisa/provenance.jsonl`:

| Signal | Observed baseline |
|---|---:|
| Tickets in the three epics | 16 |
| Authoritative automatic completions recorded | 14 |
| Tickets manually recovered after assignment failure | 2 |
| Recorded agent wall time | 13,342 s (3 h 42 m) |
| Median recorded ticket time | 963 s (16 m 3 s) |
| Fastest / slowest recorded ticket | 10 m 2 s / 23 m 46 s |
| First recorded start → last recorded finish | 10 h 33 m 40 s |

The 10.6-hour span includes a stopped loop and a long gap, so it is not a clean performance
benchmark. It is still the operational truth: excellent aggregate throughput did not guarantee a
continuous path to the user-visible result. Two prompt-delivery failures required intervention,
and the completed graph produced the reusable substrate while the actual battle remained unbuilt.

The lesson is not “make every ticket larger” or “open more panes.” It is:

> Ticket throughput is a supporting measure. Turbo’s governing measure is time to a rehearsable
> core moment.

## The turbo contract

Turbo begins with a **mission manifold**: a small, machine-readable sprint brief that turns the
large idea space into the few dimensions needed to break work up rationally. “Manifold” is useful
here because it does not pretend the idea is already a backlog. It defines the surface on which
agents can make locally independent decisions while still converging on one product.

### Mission manifold

The intake should require answers to these fields and no more:

| Field | Question it settles |
|---|---|
| `deadline` | When must the product be rehearsable, not merely code-complete? |
| `core_moment` | What should a cold viewer see happen in one sentence? |
| `proof` | What observable fact makes that moment honest? |
| `starting_substrate` | What known-good machinery may be reused unchanged? |
| `unknowns` | Which discoveries could invalidate the approach? |
| `hard_constraints` | Policy, safety, provider, device, venue, and deployment boundaries |
| `demo_path` | The shortest click/command sequence from start to core moment |
| `fallback_ladder` | Which progressively simpler versions remain honest and demoable? |
| `integration_owner` | Which lane owns the continuously runnable product? |
| `reserve` | How much of the timebox is protected for integration and rehearsal? |

The form must accept provisional answers. Turbo starts amid ambiguity; forcing false precision is
slower and less honest than naming an unknown. The first work wave should retire the highest-risk
unknowns while keeping a runnable shell alive.

### Output: a mission graph, not a miniature roadmap

Vend should compile the manifold into ordinary lisa-compatible tickets plus one turbo manifest.
The graph should normally contain four to seven **work packets**, not a comprehensive product
backlog:

1. **Walking skeleton** — the smallest end-to-end path that reaches the intended runtime.
2. **Core contract** — the minimum shared state/actions/data contract every lane needs.
3. **Independent capability lanes** — usually two or three, each with explicit file ownership and
   a runnable seam.
4. **Continuous integration lane** — owns the live demo path from the first wave onward; it is not
   a final merge ticket that starts after everyone else finishes.
5. **Proof and presentation** — cold-viewer legibility, one headline check, and the demo script.

Everything not necessary for the selected fallback level remains a signal, not a ticket. Turbo
must be able to say “not in this session” without treating that as failed decomposition.

### Work-packet shape

A turbo packet is larger than the median ordinary ticket but has harder boundaries. It should fit
roughly 30–50 minutes of agent wall time and declare:

- the single observable contribution it makes to the core moment;
- owned paths and contracts it may change;
- inputs it may assume and outputs it must expose;
- its local executable check;
- a time budget and a cut line;
- what can be omitted while preserving a useful result;
- the commit that constitutes handoff to the integration lane.

Larger scope reduces per-ticket setup and RDSPI artifact overhead. Explicit ownership and cut
lines prevent “larger” from becoming “unbounded.” If a packet cannot state a useful partial result,
it is too coupled for turbo execution.

### Compressed reasoning, not absent reasoning

Normal RDSPI produces six durable phase artifacts because overnight work values crash recovery,
deep review, and asynchronous trust. Turbo should retain the reasoning disciplines while reducing
the artifact tax:

1. **Map** — research the actual seam and name the dangerous unknown.
2. **Flight plan** — combine design, structure, and plan into one bounded handoff describing owned
   files, contracts, check, and cut line.
3. **Build and prove** — implement, run the packet check, record the result, and commit atomically.

This is a hypothesis for the experiment, not permission to skip tests, ownership, review, or
atomic completion. The compressed artifact should be recoverable enough for another agent to take
over after a failed pane.

## Scheduling doctrine

### Do not maximize pane count; maximize useful parallelism

Four agents editing one integration surface are one blocked lane disguised as four busy panes.
Turbo should calculate useful parallelism from file ownership and contracts, then reserve capacity
for convergence.

For a four-slot event run, the default hypothesis is:

- **three maker slots** for disjoint packets;
- **one integration slot** that keeps the product runnable, absorbs completed contracts, and owns
  the demo path.

The integration slot may take a maker packet only when the walking skeleton is green and no handoff
is pending. Raising concurrency beyond four is an experiment to justify with reduced makespan, not
a turbo-mode article of faith.

### Schedule the critical path, not every ready node equally

Lisa’s ordinary greedy DAG scheduling is appropriate when all admitted work should eventually
finish. Turbo needs deadline-aware priority among ready packets:

1. walking-skeleton blockers;
2. core-moment blockers;
3. integration handoffs;
4. proof/rehearsal blockers;
5. enhancements in fallback order.

When time shrinks, lower fallback levels should become unschedulable automatically or via one human
scope-cut gesture. Starting another enhancement while the live path is broken is a scheduling
defect even if a pane is free.

### Continuous convergence

Every maker packet should hand off a committed, locally verified contract. The integration lane
pulls those commits into the runnable path as they land. This produces frequent integration pulses
instead of one late “put it all together” cliff.

A useful default pulse is every 20 minutes or every completed packet, whichever comes first:

- can the walking skeleton still run?
- what is the highest honest fallback currently demoable?
- which unknown now threatens the deadline most?
- should one queued packet be cut or reshaped?

This is not human step approval. The agents and checks run autonomously; the human only settles a
genuine product fork or authorizes a scope cut.

## What each tool owns

### Vend turbo profile

Vend owns the transformation from fuzzy mission to deadline-shaped graph:

- capture and validate the mission manifold;
- identify the walking skeleton and core contract;
- produce a fallback ladder before elaborating features;
- generate four to seven packets with ownership, budgets, cut lines, and a critical-path rank;
- refuse plans whose nominal work consumes the rehearsal reserve;
- retain omitted ideas as signals rather than speculative inventory;
- record the planned envelope and decomposition decisions for comparison with actuals.

This should be an opt-in profile of the existing clearing function, not a refurbishment of Vend’s
normal value/allocation gates. A plausible eventual gesture is:

```text
vend chain "<mission>" --profile turbo --deadline 4h --reserve 45m
```

The exact CLI is not yet a decision. The durable contract is the manifold and mission graph.

### Lisa turbo profile

Lisa owns deadline-aware execution of that graph:

- reserve or preferentially maintain an integration seat;
- rank ready work by critical-path and fallback level;
- enforce packet time budgets and surface a cut-line prompt before a generic timeout;
- make pane replacement cheap and assignment acknowledgement immediate;
- expose “highest demoable fallback” alongside ticket counts;
- stop admitting enhancements when the reserve boundary is reached;
- preserve atomic ticket completion and exact path ownership;
- journal packet, handoff, integration, and intervention timing for the experiment.

A plausible eventual gesture is:

```text
lisa loop --profile turbo --deadline "2026-07-15T20:15:00-07:00"
```

Again, the flag spelling is provisional. Turbo must not silently change ordinary loop semantics.

## Battle Bots field protocol

The event should test the method without importing a prebuilt battle implementation.

### Before the event

- preserve the generic runway as the declared starting substrate;
- prepare an empty mission-manifold form and timer;
- verify the local toolchain, provider access, and deployment credentials independently of the
  event idea;
- rehearse turbo once on a neutral disposable exhibit, then discard that exhibit;
- record the hypotheses and thresholds below before seeing the kickoff constraints.

### At kickoff

In the first 15 minutes, fill the manifold from the revealed constraints. The likely core moment is
provisional until then. Vend should produce no more than seven packets and should protect at least
45 minutes at the end for integration, cold-read rehearsal, and deployment recovery.

One plausible graph shape—illustrative, not prebuilt—is:

```text
walking skeleton ─► minimal arena contract ─┬─► rules + deterministic match ─┐
                                           ├─► bot doctrine adapters ───────┼─► live integration
                                           └─► spectator rendering ────────┘        │
                                                                                     ▼
                                                                              proof + rehearsal
```

The first demoable fallback should not require a live model call. A deterministic match between two
authored strategies, streamed and replayable, is honest. Live agent-in-the-loop play, richer rules,
and tournament aggregation are higher fallback levels and may be cut.

### Stop conditions

Turbo should make these scope decisions explicit:

- **T−90 minutes:** no new architectural seam; use the strongest runnable fallback.
- **T−60 minutes:** stop feature admission; integrate, deploy, and rehearse.
- **T−30 minutes:** freeze code except for defects that block the rehearsed path.
- **Two failed integrations of the same capability:** cut or replace it with its fallback rather
  than starting a third speculative repair.
- **Provider instability:** switch to already-authored deterministic strategies; do not let live
  calls own the demo’s critical path.

## Measurements

The experiment should answer whether turbo improves compressed outcome delivery, not whether it
keeps agents busier.

### Primary measures

| Measure | Definition | Initial target |
|---|---|---:|
| Time to walking skeleton | Kickoff → first end-to-end runnable path | ≤ 45 min |
| Time to core moment | Kickoff → first honest bot-vs-bot spectacle | ≤ 150 min |
| Time to rehearsable demo | Kickoff → cold viewer can follow scripted path | ≤ 180 min |
| Protected reserve delivered | Rehearsable demo → presentation deadline | ≥ 45 min |
| Highest fallback shipped | Highest predeclared level that survives rehearsal | Record, do not game |

### Diagnostic measures

- agent-active seconds divided by available slot-seconds (**slot utilization**);
- agent-active seconds that produced a used handoff (**useful utilization**);
- critical-path idle seconds while a runnable blocker existed;
- time from maker commit to successful integration (**handoff latency**);
- time spent repairing cross-ticket conflicts (**integration tax**);
- packet starts, completions, cuts, retries, and abandoned output;
- assignment failures and minutes lost before replacement;
- human interventions, separated into genuine product forks, scope cuts, and orchestration recovery;
- phase/artifact time versus implementation/proof time;
- first-pass integration rate and headline-check pass rate;
- deployment/rehearsal time and remaining reserve.

Throughput remains useful, but only beside these measures. A run that completes ten packets and has
no rehearsable core moment is red. A run that deliberately cuts three packets and ships a clear,
honest spectacle with an hour to rehearse is green.

## Hypotheses to test

1. **Deadline-shaped decomposition beats ordinary decomposition** for a four-hour outcome because
   it admits only core-moment and fallback work.
2. **Four to seven 30–50 minute packets beat many 10–20 minute tickets** by reducing setup and
   artifact overhead without creating unbounded sessions.
3. **Three makers plus one continuous integrator beat four undifferentiated makers** by reducing
   handoff latency and the final integration cliff.
4. **A predeclared fallback ladder reduces thrash** because scope cuts become planned transitions,
   not admissions of failure.
5. **Compressed RDSPI preserves enough recoverability and review** while buying meaningful wall
   time; if rework or takeover failures rise, the compression went too far.
6. **Deadline-aware admission matters more than raw concurrency** once all disjoint lanes are busy.

## Failure modes to watch

- **Big-ticket romanticism:** fewer tickets become vague mini-epics and agents wander.
- **Concurrency theater:** panes are occupied but contend on shared files or wait on one contract.
- **Integration captain as janitor:** the reserved lane only resolves conflicts instead of owning a
  continuously runnable product.
- **Premature framework work:** agents improve the runway rather than build the event’s core moment.
- **Demo polish without proof:** the spectacle runs but cannot show that both bots used the declared
  rules and information.
- **Proof without spectacle:** the system is fair and tested but illegible to a five-minute audience.
- **Fallback shame:** the team keeps repairing an aspirational level after a simpler honest demo is
  already available.
- **Policy contamination:** event-specific implementation is rehearsed or carried in beforehand;
  only the generic method and chassis should precede kickoff.

## Smallest productization path after the experiment

Do not build a general turbo subsystem from this document alone. After the event:

1. reconstruct the actual timeline from Lisa provenance, completion journal, commits, and a short
   human decision log;
2. compare planned packets, cuts, handoffs, and fallback level with what happened;
3. identify which parts required repeated human prompt craft;
4. encode only those repeated parts into the mission-manifold schema and Vend profile;
5. add Lisa scheduling behavior only where the trace shows ordinary greedy scheduling lost time;
6. run a second, different two-to-four-hour project before calling the profile reusable.

The product opportunity is not “hackathon mode” as a novelty. It is a reusable way to convert a
fixed deadline, a fuzzy outcome, and a pool of agent capacity into a continuously converging
vertical slice. Hack nights are simply the cleanest pressure test.

## Decision summary

- Build turbo as an **opt-in profile**, never a weakening of normal mode.
- Optimize for **time to rehearsable core moment**, not tickets completed.
- Let Vend create a **mission manifold, fallback ladder, and four-to-seven-packet mission graph**.
- Let Lisa execute it with **critical-path priority, continuous integration capacity, deadline-aware
  admission, and reliable replacement**.
- Start with **three makers plus one integrator** at four-way concurrency.
- Compress RDSPI’s artifacts while retaining mapping, design ownership, proof, and atomic commits.
- Use Battle Bots as a **pre-registered natural experiment**, then productize only the mechanisms
  the trace proves valuable.
