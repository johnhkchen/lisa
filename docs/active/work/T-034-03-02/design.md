# Design: T-034-03-02 live proof and Claude parity

## Goal

Produce reviewable field evidence that the committed split-brain boundary and
the current provider contract survive a fresh build and a real isolated loop.

The validation must not depend on scheduler code already loaded by the parent
Lisa session.

No production behavior is to be changed.

## Evidence model

The ticket has two different proof obligations.

The adversarial obligation requires exact control over attempt generations,
time, stale signals, private artifacts, and provenance.

The runtime obligation requires the newly built embedded WASM to load under
Zellij and drive real Claude and Codex processes through assignment and
completion.

A single observation surface cannot satisfy both with equal rigor.

The design therefore composes deterministic regression evidence and fresh-loop
field evidence from the same source revision.

## Option 1: rerun only the native regression

Build the repository and execute
`split_brain_timeline_fences_old_attempt_and_admits_one_winner`.

### Benefits

- Exact and deterministic split-brain timeline.
- Strong assertions for every lease boundary.
- Fast and repeatable.

### Costs

- Does not instantiate the embedded WASM.
- Zellij pane closure remains stubbed.
- Does not exercise real Claude hooks or provider completion.
- Does not satisfy the temporary fresh-loop language in the ticket.

### Decision

Rejected as the complete solution, retained as the first proof layer.

## Option 2: observe the parent loop

Use the currently running repository session as proof that Lisa schedules this
ticket.

### Benefits

- No extra fixture or model invocation.
- Existing pane and artifact activity is visible.

### Costs

- The parent plugin was loaded before the prerequisite code landed.
- It cannot hot-reload the new WASM.
- Repository activity is mixed with unrelated tickets.
- The ticket explicitly forbids this substitution.

### Decision

Rejected.

## Option 3: reproduce every adversarial event manually in Zellij

Launch Codex, suspend or kill its process, wait for timeout, manipulate prompt
delivery, resume the predecessor, and inspect the result.

### Benefits

- Exercises actual Zellij close-pane behavior.
- Closely resembles the original field failure.

### Costs

- Terminal process timing is nondeterministic.
- A closed pane normally tears down the predecessor process, making a later
  resume impossible by design.
- Reliably forcing a replacement prompt miss requires invasive terminal
  interference.
- The live run cannot directly inspect private lease maps or distinguish all
  stale signal rejection branches.
- Repeating this for Claude would test a transport state Claude does not have.

### Decision

Rejected as the sole proof and as the definition of provider parity.

An opportunistic hard-silence observation may be retained if safe, but it is not
allowed to weaken or replace the committed regression.

## Option 4: fresh build plus composed isolated harness

Use one immutable source revision for three connected checks:

1. build release WASM and CLI and record their hashes;
2. run the committed deterministic split-brain regression;
3. run matched minimal Codex and Claude tickets in a new temporary Git project
   under the freshly built CLI and embedded WASM.

### Benefits

- Executes the exact committed adversarial regression.
- Exercises the real WASM/Zellij/provider boundary.
- Records provider assignment and completion using durable repository evidence.
- Avoids parent-loop hot reload.
- Keeps Claude semantics unchanged rather than imposing Codex ack behavior.
- Can preserve evidence after terminating the temporary Zellij session.

### Costs

- Provider turns are live and can take variable time.
- Runtime evidence cannot expose every private scheduler assertion.
- Requires careful cleanup and explicit provenance capture.

### Decision

Chosen.

It matches the honest boundary established by T-034-03-01 and closes the named
live gap without introducing a second scheduler implementation.

## Fresh build design

Record the starting Git commit.

Build in dependency order:

1. `lisa-plugin` release WASM for `wasm32-wasip1`;
2. `lisa-cli` release binary, whose build script embeds that WASM.

Copy the release CLI to a ticket-specific temporary installation directory.

Use that absolute copy for every fixture command.

Record SHA-256 hashes for:

- the built WASM target;
- the installed fixture CLI;
- the extracted content-hashed WASM written by `lisa loop`.

Compare the target WASM and extracted WASM hashes.

Equality demonstrates the launched loop used the just-built plugin bytes.

## Deterministic harness design

Run the exact named regression from the current checkout after the release
build.

Capture complete command output and exit status in the evidence directory.

Also run the full plugin suite after the live proof to detect interaction or
baseline regressions.

The named test remains the authoritative proof for:

- fence before reschedule;
- stale signal rejection;
- no duplicate ownership;
- no cross-attempt artifact attribution;
- one authoritative Done result.

## Temporary project design

Create the fixture under a new `mktemp` directory outside the parent repository.

Initialize it with the fresh Lisa binary.

Add minimal repository instructions that tell both providers to:

- follow the generated RDSPI workflow;
- make no product source changes;
- write six short artifacts;
- stop after review so Lisa performs completion.

Create two matched tickets with identical acceptance text:

- a Codex-routed ticket;
- a Claude-routed ticket depending on the Codex ticket.

The dependency and `max_threads = 1` serialize the proof and make state easier
to attribute.

Using distinct providers in one loop exercises per-ticket routing and the same
plugin instance.

The Claude ticket follows Codex on the same logical harness without changing its
native ownership contract.

Commit the complete fixture baseline before launch.

## Runtime launch design

Invoke the fresh absolute CLI with:

`loop --path <fixture> --max-threads 1 --client codex`

The default selects Codex for any ticket without a route, while explicit ticket
frontmatter preserves the Claude control.

Launch in a uniquely named Zellij session with a real PTY, then detach the client
without terminating the server.

Monitor from outside the session using filesystem state, Git history, Zellij
pane listings, and screen dumps.

Do not interact with the parent Zellij session.

## Success criteria

### Build provenance

- release build succeeds;
- fresh CLI reports the expected version;
- generated layout names the fresh CLI path;
- target and extracted WASM hashes match;
- the extracted filename contains a new content hash.

### Split-brain boundary

- the exact committed regression passes;
- lifecycle order is covered by its assertions;
- the regression leaves one authoritative attempt-2 Done row.

### Codex live path

- the ticket is assigned to a real Codex pane;
- all six canonical artifacts appear;
- Lisa changes the committed ticket to Done only through completion;
- a completion commit includes the ticket and work artifacts;
- provenance records a Codex/OpenAI Done outcome.

### Claude parity path

- the matched dependent ticket is assigned after Codex completion;
- a real Claude pane completes all six artifacts;
- Lisa performs the same commit-gated Done publication;
- provenance records a Claude/Anthropic Done outcome;
- no Codex acknowledgement requirement is introduced into Claude behavior.

### Isolation

- fixture Git root and Zellij session are unique;
- parent ticket frontmatter is unchanged;
- no parent source change is made;
- the temporary session is terminated after evidence capture.

## Failure handling

If a provider blocks on authentication or capacity, preserve the screen and
filesystem evidence and state the limitation in Review.

If the loop launches the wrong binary or mismatched WASM, stop and rebuild;
that run is invalid.

If an agent edits fixture source beyond the requested artifacts, inspect but do
not treat those changes as parity evidence.

If completion stalls after review, inspect hook signals, pending transaction
state, ticket frontmatter, and pane output before intervening.

Do not manually mark fixture tickets Done; that would erase the acceptance
boundary being tested.

## Rejected scope

This ticket will not:

- change scheduler or adapter code;
- add acknowledgements to Claude;
- mutate the parent loop;
- use the old Homebrew Lisa;
- treat a process launch alone as completion proof;
- delete timeout provenance to manufacture a single total row;
- publish the parent ticket as Done.

## Design conclusion

The chosen proof is a layered contract test from one fresh revision.

The committed native regression supplies exact adversarial state assertions;
the isolated mixed-provider loop supplies the real WASM, Zellij, hook,
assignment, artifact, transaction, and provenance evidence.

Together they prove the fenced safety boundary while preserving Claude's
intentionally different but unchanged assignment semantics.
