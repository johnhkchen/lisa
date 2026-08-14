# Promoting a release into nightly

`nightly` is the channel for machines that run real work with nothing at stake:
it takes release candidates, but only after a release has been out for a day
without being replaced. This is how that day is counted, who counts it, and what
to do when the count stops.

## Where the decision is made

Once, centrally, by `.github/workflows/promote-nightly.yml`. It runs hourly and
does four things:

1. reads the release list as it stands right now;
2. asks `lisa promote-nightly`, which applies the nightly rule in
   `crates/lisa-cli/src/channel.rs`;
3. writes `packaging/apt/nightly-tag.txt` when — and only when — the answer
   changed;
4. republishes the tap and the apt archive from that pointer.

Both package managers read the same one-line file, so `brew install lisa-nightly`
and a `nightly` apt sources line cannot disagree about which release has soaked.

**A run with nothing to promote stops after step 2.** No commit, no rewritten
formula, no re-signed suite. A tap whose history is full of no-op commits is one
nobody will read during an incident.

## The rule, and what "superseded" means

- **The newest release is the only candidate.** Everything below it has been
  superseded — whether or not the release above it has soaked, and however long
  it has been sitting there.
- **A candidate becomes nightly's once it is 24 hours old.**
- So two releases inside one window promote *nothing*: the older one was
  superseded the moment the newer one was tagged, and the newer one waits out its
  own window. A candidate that a hotfix replaces twenty minutes later never
  reaches a machine.
- A release that was yanked, deleted, turned back into a draft, or never finished
  uploading its artifacts is not in the list the promotion reads, so it cannot be
  promoted. If the pointer already named it, the promotion retires the pointer
  back to `stable` rather than leaving the publish to fail closed on a release
  that is not there.

## The soak window is one number

`DEFAULT_SOAK_HOURS` in `crates/lisa-cli/src/channel.rs`. The promotion does not
carry a second copy of it — it calls the same `resolve()` a machine calls, so
there is one window and one superseded rule for the whole fleet.

**Client-side soak is not retired; it is scoped.** Which mechanism applies
follows from how Lisa got onto the box:

| how Lisa got there | who waits out the soak |
| --- | --- |
| `brew install lisa-nightly` | the promotion. `brew upgrade` installs what the formula says |
| a `nightly` apt sources line | the promotion. `apt-get upgrade` installs what the suite says |
| the one-command install, or a source build | the box, from the release list, using the same window |

This is one rule in two places, not two rules. A package manager has no clock and
no release list; it installs the version it is pointed at, so somebody has to
have done the waiting before it looks. A curl-installed box has no publisher
pointing at anything, so it does the waiting itself. The two can differ for at
most one promotion cycle — an hour — and only in the direction of the
curl-installed box being slightly quicker.

The consequence worth knowing: `soak_hours` in the per-user config still changes
the wait on a curl-installed box, and changes nothing on a packaged one. A
packaged box that wants a different wait wants a different channel.

## Reading where nightly stands, without ssh

```bash
# which release nightly carries, and since when
gh api -H "Accept: application/vnd.github.raw" \
  repos/johnhkchen/lisa/contents/packaging/apt/nightly-tag.txt
git log -1 --format='%cI %s' -- packaging/apt/nightly-tag.txt

# what the last promotion run decided, no-ops included
gh run list --repo johnhkchen/lisa --workflow promote-nightly.yml --limit 5
```

Every run writes a summary saying what it decided and why, so a run that changed
nothing still says *why* it changed nothing. `stable` in the pointer means
nothing has been promoted yet and `nightly` is carrying what `stable` carries.

On a box, `lisa nightly status` and `lisa doctor` still answer for that box.

## When it needs a person

**Ask what would happen, and change nothing:**

```bash
gh workflow run promote-nightly.yml --repo johnhkchen/lisa --field dry-run=true
```

**Promote now instead of at seventeen past:** run the same workflow without
`--field dry-run=true`.

**Republish without a promotion** — the pointer is right but the tap or the
archive does not match it, say after a failed deploy:

```bash
newest=$(gh release list --repo johnhkchen/lisa --exclude-drafts --limit 1 \
  --json tagName --jq '.[0].tagName')

gh workflow run publish-apt-repository.yml --repo johnhkchen/lisa
gh workflow run publish-homebrew-tap.yml --repo johnhkchen/lisa \
  --field release-tag="$newest"
```

`release-tag` is what `lisa-canary` will carry, so it has to be the newest
release **at the moment you run this**, prerelease or not. Resolve it rather than
copying a tag out of a plan: a literal that was newest when the plan was written
moves `lisa-canary` backwards when the plan is finally run.

**Yank a release from nightly by hand.** Deleting or un-publishing the release is
the real act — the next promotion run reads the list, sees it gone, and retires
the pointer. Editing `packaging/apt/nightly-tag.txt` on `main` works too and
takes effect on the next publish.

## When the tap is already wrong

A formula naming a release its channel does not mean — `lisa` on a release
candidate, say. One dispatch fixes it: the *Republish without a promotion* one
above, naming the newest release of any kind. Every publish resolves all
three channels from scratch: the newest release for `lisa-canary`, the newest
release that is *not* a prerelease for `lisa`, the promotion pointer for
`lisa-nightly`. Whatever kind of release is newest, the run leaves the whole tap
correct, and a formula that was already right is not rewritten or recommitted.

Read the result. From a Lisa checkout, `scripts/verify-live-tap.sh` asks the
live tap and the live release list and says which formula is wrong — it only
looks, so it is safe against the real tap:

```bash
scripts/verify-live-tap.sh
```

Or by hand, the same loop the release checklist uses:

```bash
for formula in lisa lisa-nightly lisa-canary; do
  printf '%s: ' "$formula"
  gh api "repos/johnhkchen/homebrew-lisa/contents/Formula/$formula.rb" \
    --jq .content | tr -d '\n' | base64 --decode |
    awk -F'"' '/^  version "/ { print $2; exit }'
done
```

**Stable is allowed to move backwards, and this is what that looks like.** The
correction on 2026-08-14 took `lisa` from `0.5.0-rc.2` down to `0.4.4`, because
`0.4.4` is the newest release that is not a prerelease and has been since
2026-07-19. `lisa` means *the newest stable*, not *the highest version anyone has
seen*; a tap that refuses to come down from a candidate is a tap that lies about
which line it is on. Two consequences worth saying out loud:

- A machine already holding `0.5.0-rc.2` does not come back on `brew upgrade`.
  Homebrew never downgrades, so it needs `brew reinstall lisa`. Anyone who ran
  `brew upgrade lisa` during the single-formula era is in this position.
- A machine on `0.4.4` was right all along and stops being told otherwise:
  `brew outdated` no longer reports `0.4.4 < 0.5.0-rc.2`. A `brew pin lisa` held
  only to stop an absent-minded upgrade from taking the box onto a candidate can
  come off.

### The two-dispatch repair, for a tap the fixed publisher cannot reach

Before `T-069-01-05` a prerelease cut skipped `Formula/lisa.rb` entirely, so a
stale stable formula stayed stale until somebody cut a release that was not a
prerelease. If you are working against a publisher from before that fix — an old
`ref`, or a tap being repaired from a tagged tree — the repair is two dispatches
in this order, and the order is the whole trick:

```bash
newest_stable=$(gh release list --repo johnhkchen/lisa \
  --exclude-drafts --exclude-pre-releases --limit 1 --json tagName --jq '.[0].tagName')
newest=$(gh release list --repo johnhkchen/lisa \
  --exclude-drafts --limit 1 --json tagName --jq '.[0].tagName')

# 1. Publish the newest stable as if it were the release: this is the only way
#    the old publisher writes Formula/lisa.rb. It moves lisa-canary too.
gh workflow run publish-homebrew-tap.yml --repo johnhkchen/lisa \
  --field release-tag="$newest_stable"

# 2. Wait for that run to finish, then put lisa-canary back on the newest
#    release of any kind.
gh workflow run publish-homebrew-tap.yml --repo johnhkchen/lisa \
  --field release-tag="$newest"
```

Three things this gets wrong if it is run carelessly:

- **Step 2 must name the newest release at the time you run it**, resolved, not a
  literal. A rehearsal of this repair written before `v0.5.0-rc.3` existed named
  `v0.5.0-rc.2` in step 2, and would have moved `lisa-canary` backwards a release.
- **Between the two runs `lisa-canary` reads the older version.** A
  `brew upgrade lisa-canary` inside that window is a downgrade for whoever runs
  it. Keep the gap short, and prefer the one-run dispatch above whenever the
  publisher on `main` is available to it.
- **Step 1 is not optional and cannot be reordered.** Run in the other order,
  the stable write lands last and leaves `lisa-canary` on the stable release.

`lisa-nightly` is untouched by either dispatch: it follows
`packaging/apt/nightly-tag.txt` on `main`, not the tag you name.

## The failure mode to watch for

**A promotion that quietly stops looks exactly like a healthy one from a
machine's point of view.** `brew upgrade` on a `lisa-nightly` frozen three weeks
ago succeeds and says nothing is out of date, because from the formula's side
nothing is.

Three things push back on it, and none of them is complete on its own:

- Every run that promotes late says so as a workflow warning, so the Actions tab
  shows the schedule slipping rather than only its result.
- Every run that finds the newest release half-published warns too, because that
  is the one state that can hold `nightly` still indefinitely without anything
  failing.
- GitHub disables scheduled workflows after 60 days without repository activity,
  and emails the owner when it does.

What none of them catches is the schedule simply not firing on a repository that
is otherwise busy. The cheapest check is the one above: read the date on
`packaging/apt/nightly-tag.txt` and compare it to the newest release. If it is
more than a couple of days behind, the promotion is not running.

## What promotes a release to stable

A person, by dropping `-rc` from the workspace version and merging. That is a
deliberate act and it stays one — this job promotes into `nightly` and nothing
else.
