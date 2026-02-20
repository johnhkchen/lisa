# T-016-01 Plan: Create Homebrew tap

## Step 1: Create `homebrew-lisa` repository on GitHub

Using `gh repo create`:
```bash
gh repo create johnhkchen/homebrew-lisa --public --description "Homebrew tap for Lisa" --clone=false
```

Then initialize with a README and Formula directory:
```bash
gh api repos/johnhkchen/homebrew-lisa/contents/README.md \
  -X PUT -f message="Initial commit" \
  -f content="$(echo '# homebrew-lisa\n\nHomebrew tap for [Lisa](https://github.com/johnhkchen/lisa).\n\n## Install\n\n```bash\nbrew install johnhkchen/lisa/lisa\n```' | base64)"
```

Or clone, add files, push.

**Verification:** `gh repo view johnhkchen/homebrew-lisa` succeeds.

## Step 2: Update `dist-workspace.toml`

Add Homebrew-related config:
```toml
installers = ["shell", "homebrew"]
tap = "johnhkchen/homebrew-lisa"
formula = "lisa"
publish-jobs = ["homebrew"]
```

**Verification:** `dist plan` succeeds and lists a Homebrew installer in its output.

## Step 3: Regenerate release workflow

```bash
dist generate-ci
```

This updates `.github/workflows/release.yml` to include the Homebrew publish job.

**Verification:**
- release.yml contains a `publish-homebrew` job (or similar)
- The job references `HOMEBREW_TAP_TOKEN`
- WASM build-setup steps are still present in build-local-artifacts

## Step 4: Create PAT and add as repo secret

Manual steps (cannot be automated by agent):
1. Go to GitHub Settings → Developer Settings → Fine-grained tokens
2. Create token scoped to `johnhkchen/homebrew-lisa` with `contents: write`
3. Add as secret `HOMEBREW_TAP_TOKEN` on `johnhkchen/lisa` repo

**Verification:** `gh secret list` shows `HOMEBREW_TAP_TOKEN`.

## Step 5: Update README.md

Add Homebrew install option to the Install section:
```markdown
### Homebrew (macOS)

\`\`\`bash
brew install johnhkchen/lisa/lisa
\`\`\`
```

Place between "Shell installer" and "From crates.io" sections.

**Verification:** README renders correctly with the new section.

## Step 6: Run tests

```bash
cargo test --workspace
```

No code changes, but confirm nothing broke.

**Verification:** All tests pass.

## Step 7: Commit changes

Commit the following files:
- `dist-workspace.toml`
- `.github/workflows/release.yml`
- `README.md`

Message: "Add Homebrew tap support via cargo-dist (T-016-01)"

## Step 8: Tag and release (deferred)

This step triggers the actual release. May be done separately:
```bash
git tag v0.1.7  # or whatever the next version is
git push origin v0.1.7
```

The release workflow will:
1. Build all platform binaries
2. Create GitHub Release with artifacts
3. Generate and push Homebrew formula to `homebrew-lisa` repo

## Step 9: Post-release formula customization

After the first release pushes the generated formula:
1. Clone `homebrew-lisa`
2. Edit `Formula/lisa.rb` to add:
   ```ruby
   depends_on "zellij"

   def caveats
     <<~EOS
       Lisa requires Claude Code to be installed separately.
       See: https://docs.anthropic.com/en/docs/claude-code
     EOS
   end
   ```
3. Commit and push

**Verification:** `brew install johnhkchen/lisa/lisa` shows the caveat after install.

## Step 10: End-to-end verification

```bash
brew tap johnhkchen/lisa
brew install lisa
lisa --help
```

**Verification:** All three commands succeed. `lisa --help` prints usage.

## Testing Strategy

- **Unit tests:** None needed (no code changes)
- **Integration:** `dist plan` validates the cargo-dist config
- **Manual:** Brew install after first release
- **Regression:** `cargo test --workspace` confirms no breakage

## Steps Executable by Agent

Steps 1-3, 5-7 can be executed by the agent.
Step 4 requires manual GitHub UI interaction (PAT creation).
Steps 8-10 require a release to exist and manual verification.

## Commit Plan

Single commit for steps 2, 3, 5:
- `dist-workspace.toml` (Homebrew config)
- `.github/workflows/release.yml` (regenerated)
- `README.md` (install instructions)
