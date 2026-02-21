# T-017-06: Verify release artifacts and install paths

## Platform

- macOS 15 (Darwin 25.3.0), arm64 (Apple Silicon M2)
- Release: v0.2.0

## Results

### 1. Direct binary download — PASS

Downloaded `lisa-cli-aarch64-apple-darwin.tar.xz` from the GitHub Release.

| Check | Result |
|---|---|
| Extract tarball | OK — contains `lisa` binary (2.3 MB), LICENSE, README.md |
| SHA256 checksum | OK — `c1a94fb0099ba94216618668f08b63691eb8ea1ac0321d613dcd155b8c6ea732` matches |
| `lisa --help` | OK — shows all subcommands |
| `lisa --version` | OK — prints `lisa 0.2.0` |
| `lisa doctor` | OK — zellij, claude, wasm target all detected |
| `lisa init --dry-run` | OK — plans all expected files without writing |

### 2. Shell installer — PASS

Ran the installer script from the release:

```
sh lisa-cli-installer.sh
```

Installed to `$CARGO_HOME/bin/lisa`.

| Check | Result |
|---|---|
| Install without errors | OK |
| `lisa --help` | OK |
| `lisa --version` | OK — prints `lisa 0.2.0` |
| `lisa doctor` | OK |

### 3. Cargo install --path — PASS

```
cargo install --path crates/lisa-cli
```

| Check | Result |
|---|---|
| Build and install | OK — compiled in ~6s |
| `lisa --version` | OK — prints `lisa 0.2.0` |
| `lisa doctor` | OK |

### 4. Homebrew — PASS

Tap: `johnhkchen/lisa` (repo: `johnhkchen/homebrew-lisa`)

```
brew tap johnhkchen/lisa
brew install johnhkchen/lisa/lisa
```

Formula was auto-published by the cargo-dist release workflow via `HOMEBREW_TAP_TOKEN`. Installed as a prebuilt bottle (0 seconds build time).

| Check | Result |
|---|---|
| `brew tap` | OK — tapped 1 formula |
| `brew install` | OK — installed to `/opt/homebrew/Cellar/lisa/0.2.0` (2.3 MB) |
| `lisa --help` | OK — shows all subcommands |
| `lisa --version` | OK — prints `lisa 0.2.0` |
| `lisa doctor` | OK — zellij, claude, wasm target all detected |
| `lisa init --dry-run` | OK — plans all expected files without writing |

## Release asset inventory

All expected assets present in the v0.2.0 release:

- `lisa-cli-aarch64-apple-darwin.tar.xz` (663 KB)
- `lisa-cli-aarch64-unknown-linux-gnu.tar.xz` (674 KB)
- `lisa-cli-x86_64-apple-darwin.tar.xz` (705 KB)
- `lisa-cli-x86_64-unknown-linux-gnu.tar.xz` (723 KB)
- `lisa-cli-installer.sh` (51 KB)
- `lisa.rb` (Homebrew formula)
- SHA256 checksums for all archives
- `dist-manifest.json`
- `source.tar.gz`

## Install command for users

```bash
# Homebrew (macOS)
brew tap johnhkchen/lisa
brew install lisa

# Shell installer (any platform)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/download/v0.2.0/lisa-cli-installer.sh | sh

# Or direct download (macOS arm64)
curl -LO https://github.com/johnhkchen/lisa/releases/download/v0.2.0/lisa-cli-aarch64-apple-darwin.tar.xz
tar xf lisa-cli-aarch64-apple-darwin.tar.xz
./lisa-cli-aarch64-apple-darwin/lisa --version
```
