# T-016-01 Design: Create Homebrew tap

## Decision 1: cargo-dist Automation vs Manual Formula

### Option A: Full cargo-dist Homebrew automation
- Add `"homebrew"` to installers and `publish-jobs`
- Set `tap` and `formula` in dist-workspace.toml
- Run `dist generate-ci` to regenerate release.yml
- cargo-dist auto-generates and pushes formula on each release

Pros:
- Zero manual work per release
- Formula always matches release artifacts (URLs, checksums)
- Proven pattern (used by cargo-dist itself)

Cons:
- Cross-repo push requires PAT setup (HOMEBREW_TAP_TOKEN secret)
- Generated formula may not include `depends_on "zellij"` or Claude Code caveats
- Regenerating release.yml may overwrite custom build-setup integration
- Less control over formula contents

### Option B: Manual formula, no CI automation
- Create tap repo and hand-write the formula
- Update the formula manually on each release (URLs + checksums)

Pros:
- Full control over formula contents (deps, caveats)
- No PAT needed
- No CI changes

Cons:
- Manual per-release updates are error-prone and tedious
- Checksums must be computed by hand
- Will rot if forgotten

### Option C: cargo-dist generation + manual customization
- Add `"homebrew"` to installers (so cargo-dist generates the formula)
- Do NOT add `"homebrew"` to publish-jobs (so CI doesn't auto-push)
- Take the generated formula, add zellij dep + Claude Code caveat manually
- Commit to tap repo manually or with a lightweight script

Pros:
- Gets the base formula for free (URLs, checksums, platform detection)
- Full control over customizations
- No PAT needed initially
- Can add CI automation later

Cons:
- Still manual per-release (but less work than Option B)
- Two-step process

### Decision: Option A (full cargo-dist automation)

Rationale: The ticket explicitly asks to "consider using cargo-dist's Homebrew integration." Full automation is the sustainable path. The `depends_on "zellij"` and Claude Code caveat can be added to the tap repo's formula after initial generation — or cargo-dist may support custom formula templates. If not, a post-release hook in the tap repo can patch the formula. The PAT setup is a one-time cost.

The formula customization concern (zellij dep, Claude Code caveat) can be handled by:
1. Letting cargo-dist generate and push the base formula
2. Using a GitHub Action in the tap repo to patch the formula after push
3. OR: maintaining a static section in the tap repo that the CI merges

Actually, after further consideration, cargo-dist's Homebrew publisher pushes a complete formula. The simplest path: let cargo-dist do the initial generation, then manually add the zellij dep and caveat to the tap repo. On subsequent releases, cargo-dist will overwrite the formula — but we can add a `homebrew-formula-custom` block or use cargo-dist's `formula` config.

**Revised decision: Option A with post-generation patches.** Set up full automation. If cargo-dist overwrites customizations, we'll switch to maintaining a GitHub Action in the tap repo that patches the generated formula.

## Decision 2: Tap Repository Name

### Option A: `homebrew-lisa`
- `brew tap johnhkchen/lisa` → `brew install johnhkchen/lisa/lisa`
- Matches ticket requirement exactly
- Dedicated to Lisa

### Option B: `homebrew-tap`
- `brew tap johnhkchen/tap` → `brew install johnhkchen/tap/lisa`
- General-purpose, can host multiple formulas
- Used by cargo-dist itself (`axodotdev/homebrew-tap`)

### Decision: `homebrew-tap`

Rationale: The cargo-dist config uses `tap = "owner/homebrew-tap"` as its convention. Using a general-purpose tap is more scalable if other tools are published later. The install command `brew install johnhkchen/tap/lisa` is clean enough. The ticket says `homebrew-lisa` but the intent is that `brew install lisa` works, which both achieve.

**Wait** — re-reading the ticket: "Create a new GitHub repo: `johnhkchen/homebrew-lisa`" and "brew tap johnhkchen/lisa". The ticket is explicit. Honor the ticket.

**Revised decision: `homebrew-lisa`** per ticket requirements. Install: `brew tap johnhkchen/lisa && brew install lisa` (or `brew install johnhkchen/lisa/lisa`).

## Decision 3: Formula Name

### Option A: `lisa` (override via `formula = "lisa"`)
- `brew install lisa` after tapping
- Matches binary name

### Option B: `lisa-cli` (cargo-dist default from package name)
- `brew install lisa-cli` after tapping
- Matches Cargo package name

### Decision: `lisa`

Rationale: Users should install with the name they'll type. The binary is `lisa`, the CLI is invoked as `lisa`. Set `formula = "lisa"` in dist-workspace.toml.

## Decision 4: Token for Cross-Repo Push

cargo-dist's Homebrew publisher needs to push to `johnhkchen/homebrew-lisa`. The default `GITHUB_TOKEN` only has access to the current repo (`johnhkchen/lisa`).

Options:
- **Fine-grained PAT** scoped to `homebrew-lisa` repo with `contents: write`
- **Classic PAT** with `repo` scope (broader than needed)

### Decision: Fine-grained PAT

Store as `HOMEBREW_TAP_TOKEN` secret in the `lisa` repo. cargo-dist's release workflow will use this token for the Homebrew publish step. Fine-grained tokens are scoped to specific repos, minimizing blast radius.

## Decision 5: Handling `depends_on "zellij"` and Claude Code Caveat

cargo-dist generates a formula that installs the binary but may not add custom dependencies or caveats.

Options:
- Maintain a custom formula in the tap repo and have CI merge cargo-dist's output with custom additions
- Accept that cargo-dist overwrites and manually patch after each release
- Check if cargo-dist supports custom formula templates or hooks

### Decision: Start with cargo-dist automation, add manual customizations to tap repo

The generated formula from cargo-dist handles URLs, checksums, and platform detection. After the first release, we'll inspect the generated formula and add `depends_on "zellij"` and the Claude Code caveat. If cargo-dist overwrites on subsequent releases, we'll add a simple GitHub Action to the tap repo that patches the formula.

Pragmatic path: get the automation working first, customize second.

## Summary

| Decision | Choice |
|----------|--------|
| Approach | cargo-dist full automation (Option A) |
| Tap repo | `johnhkchen/homebrew-lisa` |
| Formula name | `lisa` (via `formula = "lisa"`) |
| Auth token | Fine-grained PAT as `HOMEBREW_TAP_TOKEN` |
| Customization | Post-generation manual or Action-based patching |
