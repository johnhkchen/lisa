# Design — T-057-03-01, release-0-5-0-rc-1

## The shape of the deliverable

This ticket's output is not a code change. It is **a prepared release plus a
correctly-refused handoff**. Two halves, and the second is the harder one.

### Half one: preparation (done, re-verified)

Everything an agent can do without crossing the authorization line:

1. Version at `0.5.0-rc.1` across the workspace, proven by `cargo metadata` and
   by a local build printing it.
2. The version comparison asserted, not reasoned about. A prerelease suffix is
   exactly the string that breaks a naive compare, and S-057-02's entire upgrade
   path reads through `version_is_stale`.
3. Checklist re-parameterized, gate appended, no gate deleted.
4. Cut record written for someone who ran 0.4.4 yesterday.
5. Live baseline captured; `just check` green.

### Half two: the refusal

The design decision that matters: **the block is the deliverable, not a
failure.** A pass here would be a lie — it would report success at a boundary
the agent cannot cross. The ticket says so (`_Advances: P2 — the ticket's own
state stays honest_`), and the disposition schema has exactly the shape for it.

## Designing the `check`

The check has an unusual burden: it verifies a reality that does not exist when
it is written, from a machine that cannot make it exist. Three constraints from
the workflow contract shape it.

**Read-only.** Lisa runs the check in the live project alongside other threads.
A `brew install` or `brew upgrade` would be a remedy performing itself and would
mutate a shared machine. One `gh api` GET against the tap's formula file is the
whole check. It writes nothing anywhere.

**Sized for a round trip, not for a build.** The ticket is explicit: the
operator runs the check *after* CI finishes, so it verifies a settled fact. A
check that tried to wait out a cargo-dist build would reach for the 1800-second
ceiling and be refused. Measured: 0.31s. Budget: 20s — one network round trip
with slack, and nothing more.

**Three exit paths, all meaningful.** Under the contract, `2` means "could not
look" (inconclusive) and other non-zero means "looked and said no" (a verdict).
Those must not be confused, or an operator standing at a network failure reads
it as their release having failed. Hence the explicit empty-content guard:
unreachable tap → `exit 2`; tap reachable and serving the wrong version →
`exit 1`; tap serving `0.5.0-rc.1` → `exit 0`.

**Subject choice: the tap formula, not `brew info`.** `brew info` reads a local
Homebrew state that is stale until `brew update` runs — and `brew update` writes.
The formula file in `johnhkchen/homebrew-lisa` is the published fact itself, and
reading it is a GET.

## Designing the `ask` and `steps`

The workflow document splits these deliberately, and there is a real authoring
trap in the split.

`validate_block_ask` (`crates/lisa-core/src/parking.rs`) looks for an action word
only in the first sentence, and ends that sentence at the first `.` — so an ask
that opens with a version number is truncated at `Publish 0.` and rejected as
having no action. The design consequence is not a workaround: it is that
**version arithmetic belongs in `reason` and `steps`, and the `ask` is one plain
sentence naming a command.** Which is what the workflow document asked for
anyway.

`steps` carry both halves of the operator's job, in order:

1. Confirm the line being published.
2. `just release` — one route, never both routes for one commit.
3. Watch CI; know that a skipped `publish-apt-repository` is correct here.
4. **Update this machine.** Named as the step that changes `lisa --version`,
   with the reason `brew upgrade` will not do it and the tagged installer URL
   that will.
5. Confirm it.
6. The story's Homebrew acceptance, separately, with the shadowing called out so
   it does not read as a failure.

Step 4 is the one the ticket says matters, and it is written so that a person
who skips the prose still gets the right command.
