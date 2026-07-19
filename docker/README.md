# Cross-compiling to aarch64 (Raspberry Pi 4)

```sh
cargo install cross --git https://github.com/cross-rs/cross
cross build --release --target aarch64-unknown-linux-gnu -p avio
```

Run from the repo root (where `Cross.toml` and the workspace `Cargo.toml`
live). First run rebuilds the custom image and installs ~1-2GB of arm64
`-dev` packages via `docker/cross-aarch64-unknown-linux-gnu.Dockerfile` -
expect it to be slow once, then cached.

## What this does and doesn't give you

`cross build` only compiles the Rust workspace to
`target/aarch64-unknown-linux-gnu/release/avio` (the tauri binary). It does
**not**:

- Run the frontend build (`bun run build`) or bundle a `.deb`/AppImage the
  way `cargo tauri build` / `tauri-action` does in CI - `linuxdeploy` and the
  bundler's other host tooling aren't part of this image.
- Build `avio-compositor` (a separate meson/C project, not a Cargo crate) -
  it needs its own aarch64 build, same as CI presumably handles it
  separately from the Rust build.
- Bundle the GStreamer runtime assets under `assets/gstreamer/linux-arm64/`.

If you need a real installable Pi bundle rather than a binary to `scp` over
for a quick test, building on `ubuntu-24.04-arm` (as `.github/workflows/build.yml`
already does) or building natively on the Pi itself avoids piecing all of
this back together by hand.

## If a package fails to install in the Dockerfile

The arm64 package list mirrors `.github/apt-packages-linux.txt`'s dev/link
packages - if that file changes (new native dependency added), update the
`apt-get install ... :arm64` list in `cross-aarch64-unknown-linux-gnu.Dockerfile`
to match.
