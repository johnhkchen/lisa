# Lisa development tasks

# Default: run checks
default: check

# Build the WASM plugin
build:
    cargo build -p lisa-plugin --target wasm32-wasip1 --release

# Build for development (faster, no optimizations)
build-dev:
    cargo build -p lisa-plugin --target wasm32-wasip1

# Build the CLI (builds WASM plugin first so it gets embedded)
build-cli: build
    touch target/wasm32-wasip1/release/lisa.wasm
    cargo build -p lisa-cli --release

# Tag and push a release (cargo-dist builds + publishes to Homebrew)
release: check fmt-check
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "lisa-cli") | .version')
    tag="v${version}"
    if git rev-parse "$tag" >/dev/null 2>&1; then
        echo "Error: tag $tag already exists. Bump version in Cargo.toml first."
        exit 1
    fi
    echo "Releasing $tag..."
    git tag "$tag"
    git push origin main "$tag"
    echo ""
    echo "Release $tag pushed. cargo-dist CI will build and publish to Homebrew."
    echo "  Monitor: gh run list -R johnhkchen/lisa --limit 2"
    echo "  Install: brew update && brew upgrade lisa"

# Run all tests (native target)
test:
    cargo test --workspace

# Run a specific test
test-one NAME:
    cargo test --workspace {{NAME}}

# Type check without building (fast feedback)
check:
    cargo check -p lisa-plugin --target wasm32-wasip1
    cargo test --workspace

# Type check only (no tests)
check-wasm:
    cargo check -p lisa-plugin --target wasm32-wasip1

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and re-check
watch:
    cargo watch -x 'check -p lisa-plugin --target wasm32-wasip1' -x 'test --workspace'

# Build WASM + install CLI to CARGO_HOME/bin (replaces cargo install)
install: build
    touch target/wasm32-wasip1/release/lisa.wasm
    cargo install --path crates/lisa-cli --force

# Copy built plugin WASM to a target project
install-wasm PATH:
    cargo build -p lisa-plugin --target wasm32-wasip1 --release
    cp target/wasm32-wasip1/release/lisa.wasm {{PATH}}

# Show the DAG from example tickets (dry run)
parse-tickets:
    cargo test --workspace test_dependency_chain -- --nocapture

# Lint
lint:
    cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
    cargo clippy -p lisa-core -- -D warnings
    cargo clippy -p lisa-cli -- -D warnings

# Format
fmt:
    cargo fmt --all

# Format check (CI)
fmt-check:
    cargo fmt --all -- --check

# Initialize a project for lisa-loop (dry run)
init-dry-run PATH:
    cargo run -p lisa-cli -- init --dry-run --path {{PATH}}

# Initialize a project for lisa-loop
init PATH:
    cargo run -p lisa-cli -- init --path {{PATH}}

# Validate project setup
validate PATH=".":
    cargo run -p lisa-cli -- validate --path {{PATH}}

# Enter the Chromebook-test fixture (E-046): builds the image, verifies the
# fixture invariants in a disposable container, then opens an interactive
# capped shell as `tester`. The container is KEPT after exit — it is evidence
# until its run record is complete; remove it with `docker rm <name>`.
# Optional LEG labels the container: `just emulate-debian claude-a`.
emulate-debian LEG="":
    #!/usr/bin/env bash
    set -euo pipefail
    docker build -t lisa-chromebook-test docker/chromebook-test/
    docker image inspect lisa-chromebook-test \
        --format 'image id={{{{.Id}} architecture={{{{.Architecture}}'
    echo ">>> preflight: fixture invariants (disposable container, no tokens spent)"
    docker run --rm --memory=4g --cpus=2 lisa-chromebook-test bash -c '
        set -eu
        test "$(id -un)" = tester
        test "$HOME" = /home/tester
        sudo -n true
        command -v claude >/dev/null
        command -v codex >/dev/null
        for b in git rustc cargo rustup xz gcc cc g++ make; do
            if command -v "$b" >/dev/null 2>&1; then
                echo "fixture invariant failed: $b present" >&2
                exit 1
            fi
        done
        test ! -e "$HOME/.claude"
        test ! -e "$HOME/.codex"
        echo "preflight OK: node=$(node --version) claude=$(claude --version) codex=$(codex --version)"
    '
    name="cbt-$(date +%m%d-%H%M%S)"
    if [ -n "{{LEG}}" ]; then name="$name-{{LEG}}"; fi
    echo ""
    echo ">>> entering fixture: $name   (memory=4g, cpus=2, kept after exit)"
    echo ">>> record this container name in the run record"
    echo ">>> runbook: docs/knowledge/chromebook-install-test.md"
    echo ">>> auth inside:  claude auth login   |   codex login --device-auth"
    exec docker run -it --memory=4g --cpus=2 --name "$name" lisa-chromebook-test bash

# Test the current release candidate inside an emulated OS. The tag derives
# from the workspace version, so this always targets "the rc we just made".
# Defaults: debian:bookworm, claude. Extra flags go to /cbt/prepare.
#   just test-rc                                  # bookworm + claude, install leg
#   just test-rc ubuntu:24.04 codex               # other OS flavor / agent
#   just test-rc debian:bookworm claude --no-git  # journal-seal loop on the rc
# Inside the container: authenticate the agent CLI, then run /tmp/go —
# it chains prepare --release <tag>, run, and grade. Afterwards on the
# host: just cbt-collect <container-name>.
test-rc OS="debian:bookworm" AGENT="claude" *FLAGS="":
    #!/usr/bin/env bash
    set -euo pipefail
    tag="v$(cargo metadata --no-deps --format-version 1 |
        jq -r '.packages[] | select(.name == "lisa-cli") | .version')"
    echo ">>> candidate: $tag on {{OS}} driving {{AGENT}}"
    if ! curl -sfLI -o /dev/null \
        "https://github.com/johnhkchen/lisa/releases/download/$tag/lisa-cli-installer.sh"; then
        echo "FAIL: no published release for $tag — cut it first, or bump/check the workspace version" >&2
        exit 1
    fi
    slug=$(printf '%s' "{{OS}}" | tr ':/.' '---')
    image="lisa-chromebook-test:$slug"
    docker build --build-arg "BASE_IMAGE={{OS}}" -t "$image" docker/chromebook-test/
    echo ">>> preflight: fixture invariants (disposable container, no tokens spent)"
    docker run --rm --memory=4g --cpus=2 "$image" bash -c '
        set -eu
        test "$(id -un)" = tester
        sudo -n true
        command -v claude >/dev/null
        command -v codex >/dev/null
        for b in git rustc cargo rustup xz gcc cc g++ make; do
            if command -v "$b" >/dev/null 2>&1; then
                echo "fixture invariant failed: $b present" >&2
                exit 1
            fi
        done
        test ! -e "$HOME/.claude"
        test ! -e "$HOME/.codex"
        echo "preflight OK: node=$(node --version) claude=$(claude --version) codex=$(codex --version)"
    '
    name="cbt-$(date +%m%d-%H%M%S)-rc-$slug"
    docker run -d --memory=4g --cpus=2 --name "$name" "$image" sleep infinity > /dev/null
    docker exec "$name" sh -c "printf '%s\n' '#!/bin/sh' 'set -eu' \
        '/cbt/prepare --release $tag {{FLAGS}}' \
        '/cbt/run {{AGENT}}' \
        '/cbt/grade' > /tmp/go && chmod +x /tmp/go"
    echo ""
    echo ">>> leg container: $name   (kept after exit — record this name)"
    echo ">>> inside: 1) authenticate: claude auth login  |  codex login --device-auth"
    echo ">>>         2) /tmp/go   (= prepare --release $tag {{FLAGS}} + run + grade)"
    echo ">>> afterwards on the host: just cbt-collect $name"
    exec docker exec -it "$name" bash

# Re-run the acceptance grade inside an existing leg container with the
# repo's current grader (preserves the leg; only the grading logic updates).
cbt-regrade CONTAINER:
    docker start {{CONTAINER}} > /dev/null
    docker cp docker/chromebook-test/bin/grade {{CONTAINER}}:/cbt/grade
    docker exec {{CONTAINER}} /cbt/grade

# Pull a finished Chromebook-test leg's evidence off a container into the
# closing-run ticket's work dir: run record, leg metadata, instruction,
# tour page if present, and a docker-diff summary.
cbt-collect CONTAINER:
    #!/usr/bin/env bash
    set -euo pipefail
    dest="docs/active/work/T-046-06-03/{{CONTAINER}}"
    mkdir -p "$dest"
    for f in run-record.md leg-meta instruction.txt install-section.md; do
        docker cp "{{CONTAINER}}:/tmp/$f" "$dest/" 2>/dev/null || true
    done
    if docker exec "{{CONTAINER}}" test -f /home/tester/no-git-demo/.lisa/completion-journal.jsonl; then
        mkdir -p "$dest/no-git-demo/.lisa" \
            "$dest/no-git-demo/docs/active/tickets" \
            "$dest/no-git-demo/docs/active/work"
        docker cp "{{CONTAINER}}:/home/tester/no-git-demo/.lisa/completion-journal.jsonl" \
            "$dest/no-git-demo/.lisa/completion-journal.jsonl"
        docker cp "{{CONTAINER}}:/home/tester/no-git-demo/docs/active/tickets/T-NOGIT-001.md" \
            "$dest/no-git-demo/docs/active/tickets/T-NOGIT-001.md"
        docker cp "{{CONTAINER}}:/home/tester/no-git-demo/docs/active/work/T-NOGIT-001" \
            "$dest/no-git-demo/docs/active/work/"
    fi
    docker cp "{{CONTAINER}}:/home/tester/lisa-tour.html" "$dest/" 2>/dev/null || true
    docker diff "{{CONTAINER}}" 2>/dev/null | head -100 > "$dest/docker-diff.txt" || true
    echo "collected into $dest:"
    ls -la "$dest"

# S-020 interactive-gate dry run (T-020-05): set up an instrumented throwaway
# project, run lisa loop against it, then print the block/resume evidence on exit.
gate-test DEST="/tmp/lisa-gate-dryrun":
    #!/usr/bin/env bash
    set -euo pipefail
    bash docs/active/work/T-020-05/setup-gate-harness.sh {{DEST}}
    echo ">>> launching lisa loop — call AskUserQuestion, watch for [AWAITING], answer it, then quit (q)"
    cd {{DEST}}
    {{justfile_directory()}}/target/release/lisa loop || true
    echo ""
    echo "===================== GATE EVIDENCE ====================="
    echo "--- on-notify.log (expect: EVENT=attention LISA_REASON=question) ---"
    cat .lisa/on-notify.log 2>/dev/null || echo "(empty)"
    echo "--- trace.log (expect a 'heartbeat pane=N' line AFTER you answered) ---"
    cat .lisa/trace.log 2>/dev/null || echo "(empty)"
    echo "========================================================"
