# Stable cut record — v0.5.0

Field report for the checklist in [release-checklist.md](release-checklist.md).
Every value below is from live evidence gathered during or after the cut; none
are assumed.

```text
release: v0.5.0
prepared_at: 2026-08-15
prepared_by: Claude — preparation and, on explicit authorization, publication
cut_at: 2026-08-15T00:35:39Z
operator: johnhkchen ("cut it", said in the lisa session; the push was the publishing action)
release_commit: d4a94aa647c8cfecaf51cb472be1e69949ea074f  ("chore: release 0.5.0")
release_run_url: https://github.com/johnhkchen/lisa/actions/runs/31853582077
tag_api_prerelease: false
latest_api_tag: v0.5.0              # moved off v0.4.4 for the first time since 2026-07-19
ancestry_gates: e045 / musl / seal / workflow / delivery / channel — all ancestors of d4a94aa
dist_plan: app_version 0.5.0, announcement_is_prerelease false, 10 artifacts, no gnu targets, both musl matrix jobs
asset_audit: 18 assets — 4 target tarballs + 4 .sha256, installer, lisa.rb, dist-manifest.json, sha256.sum, source.tar.gz(+.sha256), 4 .deb
publish_apt_repository: success
publish_homebrew_formula: success
announce: success
homebrew_lisa: 0.5.0
homebrew_lisa_nightly: 0.5.0
homebrew_lisa_canary: 0.5.0
apt_stable: 0.5.0-1
apt_nightly: 0.5.0-1
apt_canary: 0.5.0-1                 # present in the pool alongside every prior candidate
channel_skew: eliminated            # this is the stable cut rc.2's record named as its resolution
mac_mini_took_it_untouched: PENDING # expected 04:37 local, 2026-08-15 — see below
```

## What this release is

The first stable carrying channels. `lisa upgrade`, `doctor`'s channel row,
`lisa nightly`, three Homebrew formulae, three apt suites, the soak promotion,
and channel derived from the installing package — all of `S-068-01` and all of
`S-069-01`.

Before it, a machine's Lisa version was an accident of which installer last
touched it. After it, `brew install lisa-nightly` or one word in an apt sources
line is the whole act of choosing what a box tracks.

## All three channels read 0.5.0, and that is correct

`lisa`, `lisa-nightly` and `lisa-canary` all name `0.5.0`; so do `dists/stable`
and `dists/nightly`. **This is not a bug and it is worth expecting.** Nightly
follows the newest *soaked* release through `packaging/apt/nightly-tag.txt`, and
after a stable cut that is this release. The three diverge again the moment a
release candidate ships: canary takes it, nightly waits out the soak, stable
never takes it at all.

## A false defect this cut nearly recorded

The post-cut check first read `apt_canary` as `0.5.0~rc.3-1`, which would have
meant canary had not taken the release. It had — `0.5.0-1` is in the pool. The
`awk` used to read the index printed the *last stanza in the file* rather than
the newest version, which is the same mistake the checklist already warns about
one paragraph earlier:

> *"The earlier single-stanza form stopped at the first `Version:` and reported
> the oldest version in the repository as the baseline."*

Print every version in the stanza and read them, or ask a specific question
(`grep -qx "0.5.0-1"`). A partial read of a correct index is indistinguishable
from a correct read of a broken one.

## The thing this release exists to prove has not happened yet

`mac_mini_took_it_untouched: PENDING`. The Mac mini is on Homebrew `lisa`, its
pin is off, and `dev.b28.lisa-stay-current` runs at 04:37 local. If the
arrangement works, that timer moves it `0.4.4 -> 0.5.0` with nobody touching the
machine, and the `screen-design` desk ticks the last box on
`the-mac-mini-joins-a-channel.md` with the timer's own log line as evidence.

Until that happens this is a plan that has never run. Fill this value in from the
machine, not from the tap.

## A checklist gap this cut found, unfixed

**Authorizing a release is not modelled as work anywhere.** It is a step in this
document. Checklists are read by whoever is already doing the thing; boards are
read by whoever is deciding what to do next. So an agent holding a prepared cut
has no way to reach the operator except a pane the operator may not be looking
at — which is exactly what happened here, twice, and was resolved both times by
another desk relaying the question by hand.

Lisa already has the right shape for this and does not use it for releases: a
blocked ticket with `remedy_owner: operator` reaches the board, and the board is
read. `T-068-01-03` reached John that way the same day, from an agent that was
not even alive.

Recorded rather than fixed. The fix is either a release cut being a ticket, or
park-and-ask growing a form for questions with no ticket attached — and the first
is better, because the second keeps the question outside the thing that
remembers. *A pane is where an agent thinks; a board is where a desk remembers.*
