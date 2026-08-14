# T-069-01-05 — a stale stable formula is corrected, not skipped

## What changed

The publisher now writes `Formula/lisa.rb` on **every** run, from the newest
release that is not a prerelease, instead of skipping the file whenever the
release being published is a prerelease. Refusing to write a candidate into
stable and leaving the candidate that is already there were the same branch;
they are now the same fact stated once — stable's source is the newest stable,
so a prerelease publish can only ever leave stable correct.

**`scripts/publish-tap-formulae.sh`** — takes a fifth argument, the `.rb` of the
newest release that is not a prerelease, or `""` when no such release exists
yet. `""` is the only case that leaves `lisa.rb` alone, and a tap in it has no
stable formula to be stale. Two guards replace the old skip:

- a version with a prerelease component is refused as stable's source, whatever
  the caller claims about the release it is publishing. This is the refusal the
  ticket asks to survive, and it now sits where a caller with the prerelease
  flag wrong cannot argue it away;
- a caller publishing a non-prerelease must hand that same release as stable's
  source, so a stale resolved tag cannot move stable backwards off the release
  being published.

**`.github/workflows/publish-homebrew-tap.yml`** — resolves all three channels
every run: the newest release for `lisa-canary`, the newest non-prerelease for
`lisa`, the promotion pointer for `lisa-nightly`. A non-prerelease publish names
itself as stable rather than waiting for the release list to catch up; a
prerelease publish asks `gh release list --exclude-pre-releases`. The formula is
downloaded once and reused when two channels resolve to the same tag. The run
summary gained a *channel resolved to* column, so a run that failed to converge
says so in its own summary.

**`scripts/verify-nightly-promotion.sh`** — the CI rehearsal (`ci.yml`), and
where the reproduction lives, because it needs no network and runs on every
push. New step 6 seeds a tap with a release candidate under the stable name,
publishes from a prerelease tag, and asserts `lisa.rb` ends on the newest
stable; then publishes again and asserts the tap gained no second commit. New
step 7 asserts the refusal: a candidate handed in as stable's source is rejected
and `lisa.rb` is left where the correction put it.

**`scripts/verify-homebrew-tap.sh`** — the release-time rehearsal gained the
same correction case and refusal against real cargo-dist output, and now asserts
`unchanged Formula/lisa.rb` on a candidate publish over an already-correct tap.

**`scripts/verify-live-tap.sh`** (new) — reads the live tap and the live release
list and says which formula names a release its channel does not mean. Read-only
and safe against the real tap; exit `2` when it cannot look, so a missing `gh`
is not reported as a verdict.

**`docs/knowledge/nightly-promotion.md`** — a *When the tap is already wrong*
section: the one-dispatch republish, how to read the result, that stable is
allowed to move backwards and what that means for a machine holding rc.2
(Homebrew never downgrades — `brew reinstall lisa`), and the two-dispatch repair
for a publisher that predates this fix, with its ordering trap and its downgrade
window written down. The existing republish command carried a literal
`v0.5.0-rc.2`, which is the exact trap the ticket names; it now resolves the
newest release.

**`docs/knowledge/release-checklist.md`** — section 9 now says `lisa.rb` is
written from the newest non-prerelease on a candidate cut too, so the
`$PRIOR_STABLE` assertion is a convergence check rather than an assumption that
nothing touched the file, and points at the repair section when it fails.

## How it is tested

- `scripts/verify-nightly-promotion.sh` — passes locally, exit 0. It runs in CI
  on every push. Steps 6 and 7 are the ticket's reproduction; step 6 fails
  against the old publisher, which leaves the seeded `999`-style candidate under
  `lisa.rb`.
- `scripts/verify-homebrew-tap.sh` — the routing half was run from a mirrored
  layout against a synthetic cargo-dist formula, exit 0, and the log shows the
  correction: `wrote Formula/lisa.rb (0.9.9)` on a publish from a `999.0.0-rc.1`
  tag into a tap seeded with that candidate. Its install half needs real release
  artifacts and a real Homebrew and was not run here; it runs on macOS in
  `release.yml`.
- `shellcheck` clean on all three shell scripts plus the new one; `actionlint`
  clean on the workflow.
- `scripts/verify-live-tap.sh` against the real tap — exit 0, output in the
  correction record.
- No Rust changed, so `cargo test --workspace` was not re-run; nothing in
  `crates/` references these scripts, the workflow, or the two runbooks.

## What still concerns me

**The live tap was corrected by hand, not by this code path.** While this was
being implemented, two dispatches of `publish-homebrew-tap.yml` at 23:25 and
23:26 UTC ran the two-dispatch repair, and the tap reached the exact end state
the ticket asks for. So the end state is recorded and verified —
[live-tap-correction.md](live-tap-correction.md) has the runs, the tap commits,
and the 74-second window where `lisa-canary` read the older version — but the
"by this code path rather than by hand" half of that criterion cannot now be
demonstrated on the live tap: the wrong state it would have corrected is gone,
and re-breaking the tap to prove a point is not a thing to do to a live package
index. This is why the disposition is a note rather than a pass. The confirming
observation is one line, `unchanged Formula/lisa.rb (0.4.4)`, in the next
prerelease publish from `main` after this lands.

**Dispatching a publish is John's to authorize.** The release checklist says
preparing or reviewing it is not authorization to dispatch, publish, or update
the Homebrew tap, so this attempt read the tap and never wrote to it.

**The one behaviour I could not rehearse end to end is the workflow's own
resolution step** — `gh release list --exclude-pre-releases`, the tag dedup, the
summary table. It is shell inside a YAML step, so `actionlint` and the argument
contract of the publisher are the whole of the local coverage; the first real
run is the proof. The publisher fails loudly rather than silently on a bad
hand-off (missing file, prerelease as stable's source, non-prerelease publish
whose stable source is some other release), which is the failure mode I would
rather have there.

**A judgement call worth a second opinion:** on a non-prerelease publish the
workflow names the release being published as stable's source rather than asking
the release list. It avoids a lag window where a just-published stable is not in
the list yet, and it means a deliberate re-dispatch of an older stable tag moves
`lisa` back to that release along with `lisa-canary`. That is consistent — the
dispatch says "publish this release" — but it is a place where "newest stable"
and "the release named" differ, and the publisher's guard enforces the second.
