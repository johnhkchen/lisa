# Release-candidate cut record — v0.5.0-rc.3

Field report for the checklist in [release-checklist.md](release-checklist.md).
Every value below is from live evidence gathered during or after the cut; none
are assumed.

```text
release: v0.5.0-rc.3
prepared_at: 2026-08-14
prepared_by: Claude — preparation; John authorized the publishing push explicitly
cut_at: 2026-08-14T23:06:18Z
operator: johnhkchen (authorized in session; the push was the publishing action)
release_commit: ce41dcd27727c970b5410a29e0633c70243c190d  ("Complete T-069-01-03")
version_bump_commit: a62fe51  ("chore: release 0.5.0-rc.3") — not the tip, see below
release_run_url: https://github.com/johnhkchen/lisa/actions/runs/31848668968
tag_api_prerelease: true
latest_api_tag: v0.4.4              # confirmed — latest does not move for a prerelease
ancestry_gates: e045 / musl / seal / workflow / delivery / channel — all ancestors of ce41dcd
asset_audit: 18 assets — 4 target tarballs + 4 .sha256, installer, lisa.rb, dist-manifest.json, sha256.sum, source.tar.gz(+.sha256), 4 .deb
aarch64_musl_job: success
x86_64_musl_job: success
readme_installer_path: n/a for a prerelease — releases/latest resolves to v0.4.4
homebrew_lisa: 0.4.4                # after the corrective dispatches below
homebrew_lisa_nightly: 0.4.4
homebrew_lisa_canary: 0.5.0-rc.3    # the acceptance value for this cut
apt_stable: 0.4.4-1
apt_nightly: 0.4.4-1
apt_canary: 0.5.0~rc.3-1            # the acceptance value for this cut
channel_skew: deliberate
```

## Why this cut exists

`S-068-01` blocked on it. `lisa upgrade`, `doctor`'s channel-drift row and
`lisa nightly` all existed only on `main`, so **no published release could put a
machine on a channel at all** — the Mac mini could not be made *level with
nightly* except by hand-copying an unreleased binary, which is the failure that
story exists to end. rc.3 is the first release that carries the arrangement.

It also carries all of `S-069-01`: three Homebrew formulae, three apt suites, the
soak promotion, and channel derived from the installing package.

## Three things about this cut that were not true of earlier ones

**The version bump is not the tip.** `a62fe51` set the workspace to 0.5.0-rc.3;
a Lisa loop then committed 26 more commits on top while `S-069-01` was worked.
`auto-release.yml` reads the workspace version from the tip, so the tag landed on
`ce41dcd`. This is fine and arguably better — rc.3 carries `S-069-01` whole
rather than a slice — but it is not the shape the checklist describes, and the
next cut should either quiesce the loop first or expect the same.

**Three commits were mixed.** The bump and two follow-ups were made with
`git commit -am` / `git add -A` in a tree a Lisa loop was live in, so they swept
in the loop's uncommitted work — 234 lines of `build-apt-repository.sh` into the
version bump, among others. The loop's own next commit is titled *"remove a
verification scratch file that a concurrent commit swept in"*. **Do not use
`-a` or `add -A` for a cut in a working tree.** Stage by explicit path.

**The publish path was new.** This was the first live run of
`publish-apt-repository.yml` and `publish-homebrew-tap.yml` as reusable
workflows, and of `promote-nightly.yml`'s hourly schedule. All green on first
exercise.

## What the cut did not fix, and the repair that followed

`publish-tap-formulae.sh` *skips* the stable formula on a prerelease rather than
correcting it, so `Formula/lisa.rb` was left holding `0.5.0-rc.2` — a prerelease
under the stable name, inherited from the single-formula era. `dists/stable` was
correct at `0.4.4` the whole time, so Homebrew and apt disagreed about what
stable meant, and Homebrew was the wrong one.

Found by the `screen-design` desk while certifying the Mac mini. Repaired the
same day by two hand-dispatches of `publish-homebrew-tap.yml`, authorized by John
and run by that desk:

```
run 31850304597   release-tag v0.4.4        -> lisa 0.4.4 / lisa-nightly 0.4.4 / lisa-canary 0.4.4
run 31850378787   release-tag v0.5.0-rc.3   -> lisa 0.4.4 / lisa-nightly 0.4.4 / lisa-canary 0.5.0-rc.3
```

The order matters and one dispatch could not reach it: the first alone drags
canary backwards, the second alone leaves stable holding a prerelease. For about
four minutes between them all three formulae read `0.4.4` — a real downgrade
window, with nobody in it.

`T-069-01-05` made the publisher correct a stale stable formula in one run, so
this repair is a documented fallback rather than the procedure.

## Two defects this cut surfaced

- **`T-069-01-05`** — a stale stable formula was skipped forever rather than
  corrected. Fixed.
- **`T-069-01-06`** — `doctor`'s two-lisa check derived the packaged copy from
  the *running* binary, so it fired only when the Homebrew copy was already what
  ran, and reported `one lisa … OK` on a box with four. The dangerous case — a
  keg nobody executes, where `brew upgrade` moves a binary that never answers —
  was the one it called healthy. Fixed.

Both were found by measuring machines rather than reading tickets, and both came
from the two desks re-checking each other's measurements rather than accepting
them. That is the method worth keeping.
