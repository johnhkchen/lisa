FROM debian:bookworm

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG DEBIAN_FRONTEND=noninteractive

# Crostini-like floor: transport, process inspection, and passwordless sudo.
# Node and npm come from NodeSource below because bookworm's Node 18 is older
# than the current Claude Code floor (Node 22), and Debian's separate npm
# package pulls in a large development-oriented dependency closure.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      procps \
      sudo \
 && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

RUN npm install --global --no-audit --no-fund \
      @anthropic-ai/claude-code \
      @openai/codex \
 && node --version \
 && npm --version \
 && claude --version \
 && codex --version

# Absence is an invariant of this fixture, not an optimization. Fail the build
# if package drift introduces a source checkout or compiler path by accident.
RUN for binary in git rustc cargo rustup xz gcc cc g++ make; do \
      if command -v "$binary" >/dev/null 2>&1; then \
        echo "fixture invariant failed: $binary must be absent" >&2; \
        exit 1; \
      fi; \
    done

# Crostini's default user has passwordless sudo; mirror that without making
# the measured agent sessions run as root.
RUN useradd --create-home --shell /bin/bash tester \
 && printf '%s\n' 'tester ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/tester \
 && chmod 0440 /etc/sudoers.d/tester \
 && visudo --check --file=/etc/sudoers.d/tester

USER tester
WORKDIR /home/tester

RUN test "$(id -un)" = tester \
 && test "$HOME" = /home/tester \
 && sudo -n true \
 && command -v claude \
 && command -v codex \
 && test ! -e "$HOME/.claude" \
 && test ! -e "$HOME/.codex"
