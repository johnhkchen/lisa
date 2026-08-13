# A host with no GUI, no WezTerm, and no terminal at all (T-066-01-02).
#
# This is the sixth machine on the operator's desk: the Codespace the desk can
# read a board on and start nothing. Reproducing it by unsetting a variable on
# a Mac proves nothing, so the board runs here instead — in a container entered
# with `docker exec` and no `-t`, which has no controlling terminal in exactly
# the way `ssh -T host` has none.
#
# Built with the repository root as context: `docker build -f
# docker/headless-board/Dockerfile .` (or `just headless-leg`).

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN rustup target add wasm32-wasip1
RUN cargo build -p lisa-plugin --target wasm32-wasip1 --release
# The dashboard the run will not draw still has to be inside the binary: a
# placeholder plugin is refused by the loop long before the terminal question.
RUN LISA_REQUIRE_EMBEDDED_WASM=1 cargo build -p lisa-cli --release

FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      git \
      jq \
      procps \
 && rm -rf /var/lib/apt/lists/*

# The fixture's own invariant: nothing here can draw a window. If a later base
# image starts shipping a terminal emulator, this build fails rather than
# quietly proving something easier than what the ticket asked.
RUN for gui in wezterm alacritty kitty xterm Xorg; do \
      if command -v "$gui" >/dev/null 2>&1; then \
        echo "fixture invariant failed: $gui must be absent" >&2; \
        exit 1; \
      fi; \
    done \
 && test -z "${DISPLAY:-}" \
 && test -z "${WAYLAND_DISPLAY:-}"

COPY --from=builder /src/target/release/lisa /usr/local/bin/lisa
COPY docker/headless-board/leg /headless/leg
COPY docker/headless-board/agent-stub /headless/agent-stub
RUN chmod 0755 /headless/leg /headless/agent-stub

# `operator` is already a group on Debian, so the account is named for the
# thing it does rather than for the person.
RUN useradd --create-home --shell /bin/bash runner
USER runner
WORKDIR /home/runner

# Lisa's managed Zellij, fetched at build time so the leg measures the run and
# not the download. Best-effort: the loop fetches it itself otherwise.
RUN mkdir -p /home/runner/demo \
 && lisa init --no-history --path /home/runner/demo > /dev/null \
 && (cd /home/runner/demo && lisa doctor > /home/runner/doctor.txt 2>&1 || true)

CMD ["/headless/leg"]
