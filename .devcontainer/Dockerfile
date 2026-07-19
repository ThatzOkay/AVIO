# x86_64 native build+run environment for testing the real cage + avio-compositor + avio stack
# locally, against a vkms virtual DRM device instead of the physical Pi. Debian trixie to match
# the Pi's actual OS (see project memory: Pi runs "Debian GNU/Linux 13 (trixie)").
FROM debian:trixie

ENV DEBIAN_FRONTEND=noninteractive

# Build tools + the same dev/link packages as .github/apt-packages-linux.txt (the CI native-arm64
# build's package list - native here since we're genuinely x86_64, no cross/multiarch needed),
# plus meson (>=1.4, same reason as gstreamer-assets.yml/build-avio-compositor-sidecar.ts: apt's
# meson is usually older than avio-compositor's wlroots-0.20 subproject needs) and cage itself.
RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    build-essential curl wget file pkg-config ca-certificates git unzip \
    libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
    protobuf-compiler libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
    gstreamer1.0-tools gstreamer1.0-pulseaudio gstreamer1.0-libav \
    librtlsdr-dev libudev-dev libasound2-dev libfftw3-dev \
    ninja-build bison patchelf \
    libegl-dev libgles-dev libgbm-dev libcairo2-dev libxkbcommon-dev libpixman-1-dev \
    libffi-dev libexpat1-dev xdg-utils \
    python3 python3-dev python3-pip python3-venv \
    cage grim wlr-randr \
    mesa-vulkan-drivers mesa-utils mesa-va-drivers \
    pulseaudio \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --break-system-packages --upgrade 'meson>=1.4'

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -fsSL https://bun.com/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

WORKDIR /work
