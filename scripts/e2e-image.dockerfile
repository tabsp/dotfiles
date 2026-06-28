FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      bash \
      build-essential \
      ca-certificates \
      curl \
      file \
      fontconfig \
      git \
      procps \
      python3 \
      sudo \
      tar \
      unzip \
      xz-utils \
      expect \
 && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash tester \
 && printf 'tester ALL=(ALL) NOPASSWD:ALL\n' >/etc/sudoers.d/tester \
 && chmod 0440 /etc/sudoers.d/tester
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable \
 && chmod -R a+rw /opt/rustup /opt/cargo
ENV PATH="/home/tester/.local/bin:/opt/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
WORKDIR /work
