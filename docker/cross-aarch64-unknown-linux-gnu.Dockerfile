# Extends cross-rs's own aarch64-unknown-linux-gnu image (Ubuntu 24.04-based),
# which already exports the multiarch pkg-config wiring every `pkg-config`-sys
# crate here (webkit2gtk, gstreamer, rtl-sdr, alsa, fftw3, ...) needs:
#   PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig/
#   PKG_CONFIG_ALLOW_CROSS=1
#   CROSS_SYSROOT=/usr/aarch64-linux-gnu
# We only need to add the arm64 :dev libraries themselves via apt multiarch -
# see https://wiki.debian.org/Multiarch/HOWTO. Without this, linking fails
# with "cannot find -lwebkit2gtk-4.1" etc. (the exact wall hit in
# https://github.com/tauri-apps/tauri/issues/12475, which never got this far).
FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

# Host-arch (x86_64) build tools that run *during* the build, not linked into
# the target binary - protoc (aa-proto's build.rs), ninja/bison (meson deps,
# harmless here), patchelf (tauri-bundler, harmless here).
RUN apt-get update && apt-get install --assume-yes \
    protobuf-compiler \
    ninja-build \
    bison \
    patchelf \
    python3

# Target-arch (arm64) libraries that actually get linked into the binary.
# This list mirrors the dev/link packages in .github/apt-packages-linux.txt
# (the native CI arm64 build's package list) - keep the two in sync if that
# file changes. libbz2-dev is the one addition: welle-io-sys's build.rs links
# it directly (`cargo:rustc-link-lib=bz2`) but it isn't in the CI list because
# Ubuntu's runner images ship it preinstalled already.
RUN dpkg --add-architecture arm64 && \
    { \
        echo "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports noble main universe"; \
        echo "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports noble-updates main universe"; \
        echo "deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports noble-security main universe"; \
    } > /etc/apt/sources.list.d/arm64-ports.list && \
    sed -i -E 's/^deb (\[?[^]]*\]? ?)?(http:\/\/(archive|security)\.ubuntu\.com)/deb [arch=amd64] \2/' \
        /etc/apt/sources.list 2>/dev/null || true && \
    if [ -f /etc/apt/sources.list.d/ubuntu.sources ]; then \
        sed -i '/^Types:/a Architectures: amd64' /etc/apt/sources.list.d/ubuntu.sources; \
    fi && \
    apt-get update && apt-get install --assume-yes \
        libssl-dev:arm64 \
        libwebkit2gtk-4.1-dev:arm64 \
        libayatana-appindicator3-dev:arm64 \
        librsvg2-dev:arm64 \
        libxdo-dev:arm64 \
        libgstreamer1.0-dev:arm64 \
        libgstreamer-plugins-base1.0-dev:arm64 \
        librtlsdr-dev:arm64 \
        libudev-dev:arm64 \
        libasound2-dev:arm64 \
        libfftw3-dev:arm64 \
        libegl-dev:arm64 \
        libgles-dev:arm64 \
        libgbm-dev:arm64 \
        libcairo2-dev:arm64 \
        libxkbcommon-dev:arm64 \
        libpixman-1-dev:arm64 \
        libffi-dev:arm64 \
        libexpat1-dev:arm64 \
        libbz2-dev:arm64
