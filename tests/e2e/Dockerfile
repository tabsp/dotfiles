# Dockerfile for dotman e2e tests.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# hadolint ignore=DL3008
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    build-essential curl git fish tmux neovim \
    python3 python3-yaml ca-certificates sudo unzip \
    && rm -rf /var/lib/apt/lists/*

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:$PATH"

# Create non-root user with passwordless sudo for install-branches scenario
RUN useradd -m -s /bin/bash dotman-test \
    && echo 'dotman-test ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/dotman-test

WORKDIR /app
COPY . .
RUN cargo build --release \
    && chmod +x /app/tests/e2e/*.sh /app/tests/e2e/scenarios/*.sh

USER dotman-test
ENV HOME=/home/dotman-test
ENTRYPOINT ["/app/tests/e2e/runner.sh"]
