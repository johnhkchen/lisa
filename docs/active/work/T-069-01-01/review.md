# T-069-01-01 — brew install lisa-nightly is the whole arrangement

`johnhkchen/homebrew-lisa` now carries three formulae instead of one, and which
one you install is which channel the machine is on. `lisa` stops meaning "newest
of anything": a release candidate reaches `lisa-canary` and never `lisa`.

## What changed

| file | what |
| --- | --- |
| `scripts/publish-tap-formulae.sh` | new. Derives all three formulae from the one `.rb` cargo-dist writes, and owns the routing rule. |
| `scripts/verify-homebrew-tap.sh` | new. Rehearses the routing, then installs each formula with a real Homebrew in a throwaway prefix. |
| `.github/workflows/release.yml` | new `verify-homebrew-tap` job on macOS; `publish-homebrew-formula` rewritten to generate the three and gated on the rehearsal. |
| `README.md` | a `### macOS` section: the three formulae, `brew trust`, changing channel, rollback, and what changes for a box already on `lisa`. |
| `docs/knowledge/release-checklist.md` | channel baseline, workflow-gate grep, Homebrew convergence and the cut record now read three formulae rather than one. |

Commits: `b979a67`, `e5f7599`, `4981a71`, `c6b816f`.

### The rule, in one place

`publish-tap-formulae.sh` is where the three channels are decided, so a script
can rehearse it instead of it living in YAML:

- **canary** ← this release, every time.
- **stable** ← this release, only when it is not a prerelease.
- **nightly** ← the release the promotion pointer names, resolved by the caller.
  A release publish never chooses it.

Nothing is rewritten when the contents would not change, so the tap's history
stays readable during an incident.

`lisa-nightly` follows `packaging/apt/nightly-tag.txt` — the same pointer
`T-069-01-02` gave the apt suites. One promotion, both package managers, and
`T-069-01-03` writes one file rather than two that can disagree. While it says
`stable`, nightly carries the newest non-prerelease release, which is the honest
answer before anything has been promoted.

### What happens to a box already on `lisa`

Nothing is uninstalled and no version is taken away. Two things do change, and
both are in the README:

1. `lisa` stops taking release candidates. A machine that used to move every few
   days now sits still until the next real release. To keep following
   candidates: `brew uninstall lisa && brew install johnhkchen/lisa/lisa-canary`.
2. **`brew trust johnhkchen/lisa` is now needed once per machine** — see the
   first concern below.

The tap's `lisa.rb` currently carries `0.5.0-rc.2`, published under the old
rule. This ticket does not roll that back — there is nothing to roll it back
*to* without republishing an older release's formula — so between now and the
next stable release, `brew upgrade` on a box still below `0.5.0-rc.2` can move
it there. The next stable cut rewrites `lisa.rb` and the residue is gone.

### Decisions the ticket asked to make in review

**`keg_only`: no.** `conflicts_with` already guarantees one Lisa per machine,
and `keg_only` would take `lisa` off `PATH` — the opposite of what a machine
that runs Lisa wants. All three formulae install the same `bin/lisa`, so
whichever channel is installed, `lisa` is just `lisa`.

## How it is tested

`scripts/verify-homebrew-tap.sh` covers the channel the way
`verify-shell-installer.sh` and `verify-apt-repository.sh` cover theirs. It
checks two different things:

- **Routing.** A prerelease into a stable tap moves `lisa-canary` only; `lisa`
  stays where it was, `lisa-nightly` reports `unchanged` and is not rewritten. A
  prerelease into an empty tap leaves no `lisa.rb` at all rather than seeding
  stable from a candidate. Every formula carries `conflicts_with` and the
  generated-file header.
- **Install.** A real Homebrew, cloned into a throwaway prefix with an isolated
  `HOME`, installs each of the three from a local copy of this build's archive:
  the version it lands on, that `bin/lisa` runs, and that `brew` refuses each of
  the other two by name while it is installed. Six refusals, all asserted on
  Homebrew's own "conflicting formulae are installed" message.

Runs:

- macOS 15 / arm64, against the live `lisa.rb` from the tap plus a local
  archive: **pass, ~20s.**
- Debian-family Linux (`ubuntu:24.04` container, amd64): **pass.**
- Negative test: with `conflicts_with` removed from the generator, the script
  fails with *"brew refused lisa-nightly alongside lisa without naming the
  conflict"* rather than passing quietly.
- The release workflow's publish loop was simulated locally against a fake tap
  with stand-ins for `gh release list` / `gh release download`: a prerelease
  commits `lisa-canary` and `lisa-nightly` separately and skips `lisa`; running
  it a second time produces no commit at all.
- `shellcheck` clean on both new scripts. `release.yml` parses as YAML.
- `brew style --fix` was run on generated output and is idempotent, and it
  preserves the class name, the channel `desc` and the `conflicts_with` line.

## Concerns

**1. `brew trust johnhkchen/lisa` is a new one-time step, and this ticket
introduced the need for it.** Current Homebrew refuses to load a formula from a
non-official tap unless it was named on the command line or the tap is trusted,
and `conflicts_with` makes brew load the two siblings. So with the conflict
declared, `brew install johnhkchen/lisa/lisa` fails on an untrusted machine with
*"Refusing to load formula johnhkchen/lisa/lisa-nightly from untrusted tap"* —
and so does `brew upgrade` on a machine that already has `lisa`. This was found
by the rehearsal, not by reading, and the rehearsal asserts it so the README
line stays honest; if Homebrew ever stops requiring it, the script prints a
notice instead of failing a release. It is the price of the conflict the ticket
asked for, and Homebrew is making tap trust mandatory regardless, but it is a
real change for every existing brew box and worth knowing before the mini is
set up.

**2. The `verify-homebrew-tap` job runs on `macos-15`, a runner this pipeline
has not used before.** Homebrew's readers are on macOS and a Linux runner drags
in Homebrew's own glibc and gcc bottles — on Debian bookworm that is ~600 MB and
twelve minutes, and the install then exited non-zero for reasons I did not
chase, which is exactly the wrong thing to be debugging during a release. If the
`macos-15` label is unavailable the release fails at that job, before anything
reaches the tap.

**3. The release workflow itself can only be exercised by a real release.** The
generator, the routing and the tap install are all covered by a script that runs
today; the job that calls them is covered only by the local simulation described
above and by `release.yml` parsing. The first real cut is the first end-to-end
run, and the checklist's section 9 now reads all three formulae.

**4. The promotion pointer lives at `packaging/apt/nightly-tag.txt`.** It is
fleet-wide, not apt-specific, and now has two readers. `T-069-01-03` may want to
move it to `packaging/nightly-tag.txt`; both readers have to move together.

**5. Verifying the tap on a developer's Mac writes into a throwaway `HOME`.**
An earlier run of this script, before that isolation, added two entries to the
real `~/.homebrew/trust.json` on this machine — `johnhkchen/lisa/lisa-canary`
and `johnhkchen/lisa/lisa-nightly`. They are redundant, since the tap itself is
already trusted there, and they name formulae this ticket creates. Left in
place; noted because it happened.
