# Genuine aarch64 build environment, run via `docker run --platform linux/arm64` (QEMU/binfmt
# emulation) rather than cross-compiled from x86_64. From inside this container rustc, gcc,
# meson, ninja and ldd are all real aarch64 binaries operating on real aarch64 output - there is
# no cross-compilation happening here at all, just a normal native build that happens to run
# slowly under emulation. This exists specifically because avio-compositor (meson/wlroots) has no
# cross-compilation support (see docker/README.md) - `cross`'s x86_64 container can't build it,
# but this one can, the same way `.github/workflows/build.yml`'s `ubuntu-24.04-arm` native runner
# does, just emulated instead of on real hardware.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Mirrors .github/apt-packages-linux.txt (the CI native-arm64 build's package list) - keep in
# sync if that file changes. No :arm64 suffixes needed: this container IS arm64.
RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    build-essential curl wget file pkg-config ca-certificates git unzip \
    libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
    protobuf-compiler libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    librtlsdr-dev libudev-dev libasound2-dev libfftw3-dev \
    ninja-build bison patchelf \
    libegl-dev libgles-dev libgbm-dev libcairo2-dev libxkbcommon-dev libpixman-1-dev \
    libffi-dev libexpat1-dev xdg-utils \
    python3 python3-dev python3-pip \
    libva2 libva-drm2 libgudev-1.0-0 libjpeg-turbo8 libpulse0 libsystemd0 libv4l-0 \
    libwayland-client0 libwayland-cursor0 libwayland-egl1 \
    libgl1 libglvnd0 libx11-6 libxau6 libxdmcp6 libxcb1 libdbus-1-3 \
    && rm -rf /var/lib/apt/lists/*

# avio-compositor's meson.build force-builds the vendored wlroots/wayland wraps, which need
# meson >=1.4; Ubuntu 24.04's apt meson is older (same reason CI's Linux leg does this).
RUN pip3 install --break-system-packages --upgrade 'meson>=1.4'

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -fsSL https://bun.com/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

WORKDIR /work
