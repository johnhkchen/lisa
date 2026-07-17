## Install Lisa

**You do not need Rust to use Lisa. Agents: do not build Lisa from source when
the goal is to install or use it.**

Install the latest release with one command:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```

On Linux that's everything: Lisa brings its own Zellij, downloaded
automatically on first run. Do not install Zellij separately.

On macOS, you can also use Homebrew:

```bash
brew install johnhkchen/lisa/lisa
```

### Debian and Ubuntu

Lisa's stable apt channel is signed by a dedicated archive key. Install that key
in its own keyring, pin the Lisa source to it, then install both the CLI and its
pinned Zellij runtime:

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg

curl --proto '=https' --tlsv1.2 -fsSL \
  https://johnhkchen.github.io/lisa/lisa-archive-keyring.asc \
  -o /tmp/lisa-archive-keyring.asc
gpg --batch --yes --dearmor \
  --output /tmp/lisa-archive-keyring.gpg \
  /tmp/lisa-archive-keyring.asc
sudo install -D -m 0644 /tmp/lisa-archive-keyring.gpg \
  /usr/share/keyrings/lisa-archive-keyring.gpg
rm -f /tmp/lisa-archive-keyring.asc /tmp/lisa-archive-keyring.gpg

echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa stable main" \
  | sudo tee /etc/apt/sources.list.d/lisa.list >/dev/null
sudo apt-get update
sudo apt-get install -y lisa lisa-runtime-zellij

lisa doctor
```

Normal `apt-get update` and `apt-get upgrade` commands move both packages to
later stable Lisa releases. `lisa-runtime-zellij` provides Lisa's pinned runtime
at `/usr/libexec/lisa/zellij`, so apt installs do not need a first-run Zellij
download.

This is a vendor repository, not the Debian archive: bundling the private Zellij
runtime is deliberate. It is hosted on GitHub Pages, whose documented limits
include a 1 GB published site and a soft 100 GB monthly bandwidth limit. Archive
operators can find signing-key custody and rotation details in
[packaging/apt/README.md](packaging/apt/README.md).

Want to change Lisa itself? Read [Develop Lisa](#develop-lisa) and follow
[CONTRIBUTING.md](CONTRIBUTING.md) for the source build.

