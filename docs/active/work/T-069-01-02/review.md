# T-069-01-02 — the apt sources line is the channel

The apt archive now carries three suites — `stable`, `nightly`, `canary` — over
one shared pool, signed by the one archive key. Putting a Linux box on a channel
is the word in `/etc/apt/sources.list.d/lisa.list` and `apt-get update`.

## What changed

**`scripts/build-apt-repository.sh`** — takes an input root holding one
directory per suite instead of one flat pile of `.deb` files. It pools every
input once (deduped by name/version/architecture with a byte-conflict check, as
before), indexes the whole pool once per architecture, then cuts each suite's
`Packages` out of that index by suite membership. Consequences:

- The pool is shared and never pruned, so `apt-get install lisa=<version>` stays
  a real rollback on any channel.
- A candidate's bytes can sit in the pool while `stable`'s index does not list
  it. Suite membership, not the pool, decides what a box sees.
- Each suite gets its own `Release` (`Suite`/`Codename` = the suite name),
  `InRelease` and `Release.gpg`, all signed with the same fingerprint and all
  verified against the same public-only keyring before the site is moved into
  place.
- Two new build-time invariants: each suite must carry both packages for both
  architectures, and each suite index must list *exactly* its own members —
  a count mismatch fails the build rather than shipping a leaky suite.

**`.github/workflows/release.yml`** — the `publish-apt-repository` job lost its
`!prerelease` gate and now builds three suite inputs:

- `stable` — every non-draft, non-prerelease release with the complete
  four-asset set. Unchanged meaning.
- `canary` — every non-draft release with that set, prereleases included.
- `nightly` — everything `stable` has, plus whichever release
  `packaging/apt/nightly-tag.txt` names.

Tags are downloaded once into a store and hardlinked into each suite, so a
release in three suites costs one download.

**`packaging/apt/nightly-tag.txt`** (new) — one line, currently `stable`,
meaning nothing has been promoted yet. It exists so a release publish rebuilds
`nightly` from a pointer rather than from a rule of its own; without it, the
next release would silently overwrite whatever `T-069-01-03`'s promotion had
just written. A tag named there must be a real release with the complete asset
set or the publish fails closed.

**`scripts/verify-apt-repository.sh`** — repacks the real packages into three
generations (old `0.0.0-1`, current, candidate `9.9.9~rc1-1`) and gives the
candidate to `canary` only. It then checks, in order: three suites published,
one key import verifies all three, the candidate is in the pool but in no index
except canary's, a box on `stable` upgrades to current and can neither see nor
install the candidate, exact-version rollback and forward again, `nightly`
offers current and no candidate, a one-word edit moves the box to `canary` and
`apt-get upgrade` takes the candidate, and coming back down to `stable` needs
`--allow-downgrades` — `apt-get upgrade` alone leaves the box on the candidate.
The existing network-isolated `lisa doctor` checks are unchanged.

**Docs.** `README.md` names the three channels in a table, shows the sources
line with the channel as a variable, and states both directions of a channel
move including the downgrade. `packaging/apt/README.md` describes the three-suite
shape, states the `lisa-runtime-zellij` answer and why, documents the nightly
pointer, and corrects the claim that prerelease publishing never uses the
production key. `docs/knowledge/mac-mini-nightly.md` gains the apt rollback next
to the Homebrew one, with the downgrade note. `docs/knowledge/release-checklist.md`
had four statements my change falsifies (a skipped apt publish is no longer
correct on a prerelease cut) and gains a channel-separation smoke test.

## The `lisa-runtime-zellij` answer

**It is published to all three suites, in lockstep with `lisa`.** Its version is
`${LISA_VERSION}` and it pins the Zellij that release was built against, and
`lisa` only *recommends* it, without a version. A single stable runtime shared by
all three would therefore let apt pair a canary `lisa` with a stale runtime and
say nothing. The build script enforces it: a suite missing the runtime for
either architecture fails the build. Stated in `packaging/apt/README.md`.

## How it is tested

Docker is available on this machine but the real Debian packages are not — they
need a Linux x86_64 build host. So the verifier was run end to end against
fabricated packages with the same names, architectures and control fields, and a
stub `lisa` that answers `init` and `doctor`. Every assertion in the script ran.

- `scripts/verify-apt-repository.sh <fixture>` — **exit 0**, ending on the
  three-suite summary line. Run twice: once during development and once against
  the committed scripts.
- **Negative control**: a mutated copy that also puts the candidate into
  `stable`'s input — **exit 1**, `the stable index lists candidate 9.9.9~rc1-1`,
  failing at the build-output inspection before the client is even involved. The
  criterion test is not vacuous.
- **Fail-closed**: a missing `nightly` directory and a `nightly` carrying `lisa`
  without the runtime both refuse to build, with the suite named in the message.
- `shellcheck` clean on both scripts; `actionlint .github/workflows/release.yml`
  reports nothing in the changed step (its six findings are all pre-existing,
  outside lines 341–424).
- `cargo test --workspace` — **exit 0**, 695 passed. No Rust changed.

CI runs the same verifier against the real four `.deb` assets in
`build-global-artifacts`, which is where the `lisa doctor` leg gets real bytes.

## Concerns

1. **The verifier has not run against real Lisa packages.** The stub can't be
   wrong about apt behaviour — apt does not care what is inside the `.deb` — but
   the `doctor` leg is only meaningfully exercised in CI. That leg is unchanged
   from before this ticket.

2. **Two of my commits are not mine.** A concurrent operator-side commit,
   `59b8962 chore: clear the four --all-targets clippy lints the cut gate trips
   on`, swept my in-progress edits to both scripts into itself before I could
   run `lisa commit-ticket`. The content on the branch is correct and complete;
   the attribution is not. It also swept in a scratch file I had put inside the
   repo tree, which I removed in a follow-up commit. The lesson is mine: scratch
   files do not belong in the worktree while other threads are sweeping it.

3. **`T-069-01-01` is editing `release.yml` concurrently.** Both tickets declare
   `depends_on: []` and both touch that file — a missing DAG edge. I changed only
   the `publish-apt-repository` job and left `publish-homebrew-formula` alone, so
   the two edits should not overlap, but whoever lands second should read the
   whole job list rather than trusting that.

4. **The nightly pointer is a decision `T-069-01-03` inherits.** I introduced
   `packaging/apt/nightly-tag.txt` because the alternative — deriving `nightly`
   at publish time — makes a release quietly undo a promotion. But a promotion
   only reaches the served site when the publish job next runs, so `T-069-01-03`
   needs a way to trigger that job outside a release. `release.yml` has a
   `workflow_dispatch` that requires an existing tag, which would work; a
   dedicated trigger would be cleaner. I did not build one — it is that ticket's
   call, not mine.

5. **Every existing apt box is on `stable` and stays there.** Unlike the Homebrew
   half of this story, nothing changes for a machine that has already installed
   from this repo: its sources line already says `stable`, and `stable` still
   means non-prerelease. The behaviour change is that `canary` and `nightly` now
   exist to move to.
