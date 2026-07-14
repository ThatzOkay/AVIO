#!/usr/bin/env bun
// Builds the nested wlroots compositor (avio-compositor) via meson/ninja and bundles it with
// its non-system shared libs into src-tauri/compositor/, so tauri's `bundle.resources` can drop
// it into the AppImage (see tauri.conf.json's before*Command hooks). Ported from LIVI's
// scripts/compositor/build-linux.sh. The pinned 0.20 wlroots subproject is always forced (it
// carries the AVIO patches under subprojects/packagefiles/), never the system one.
import {
  chmodSync,
  existsSync,
  mkdirSync,
  realpathSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import path from "node:path";

if (process.platform !== "linux") {
  console.log("[avio-compositor] builds on Linux only; skipping");
  process.exit(0);
}

const repoRoot = path.dirname(import.meta.dir);
const srcDir = path.join(repoRoot, "src-tauri", "crates", "avio-compositor");
const buildDir = path.join(srcDir, "build");
const outDir = path.join(repoRoot, "src-tauri", "compositor");
const bin = path.join(buildDir, "avio-compositor");

for (const tool of ["meson", "ninja", "pkg-config", "patchelf"]) {
  if (!commandExists(tool)) {
    console.error(`[avio-compositor] missing build tool: ${tool}`);
    process.exit(1);
  }
}

// AVIO carries a wlroots patch (subprojects/packagefiles/wlroots-avio.patch) that exposes
// host-output control for app-kiosk + our own decorations. force-fallback-for builds the
// pinned 0.20 subproject even when a system wlroots-0.20 exists, so the patch always applies.
console.log(
  "→ Forcing pinned wlroots-0.20 subproject (carries the AVIO output-control patch)",
);
const mesonArgs = ["--buildtype=release", "--wrap-mode=default"];
let fallback = "wlroots-0.20";

if (!pkgConfigExists("wlroots-0.20")) {
  // No system wlroots: the whole stack is built from wraps. Force wayland from the wrap too, so
  // the bundled wayland-scanner matches the (newer) wayland-protocols.
  fallback += ",wayland,wayland-protocols";
  mesonArgs.push(
    "-Dwayland:documentation=false",
    "-Dwayland:tests=false",
    "-Dwayland:dtd_validation=false",
    "-Dlibxkbcommon:enable-docs=false",
    "-Dlibxkbcommon:enable-tools=false",
    "-Dlibxkbcommon:enable-xkbregistry=false",
  );
}
mesonArgs.push(`--force-fallback-for=${fallback}`);

// Only run `meson setup` once. `--reconfigure` on an existing build dir hits a meson quirk
// where project options for a subproject not part of the cached reconfigure state (here,
// libxkbcommon: satisfied by the system package, not wrapped) fail as "Unknown option" even
// though the identical flags work on a fresh setup. ninja's generated build.ninja already
// auto-regenerates the meson config internally if meson.build/wraps change, so plain
// `ninja -C build` covers incremental rebuilds without ever needing --reconfigure.
console.log("→ Configuring avio-compositor");
if (!existsSync(path.join(buildDir, "build.ninja"))) {
  run("meson", ["setup", buildDir, srcDir, ...mesonArgs], repoRoot);
}

console.log("→ Compiling");
run("ninja", ["-C", buildDir], repoRoot);

if (!existsSync(bin)) {
  console.error(`[avio-compositor] build did not produce ${bin}`);
  process.exit(1);
}

// Host-provided libs (GPU driver + base graphics + libc): never bundled
const SYSTEM_EXCLUDED = [
  /\/lib64\/ld-linux-.*/,
  /\/lib\/ld-linux-.*/,
  /\/lib\/x86_64-linux-gnu\/ld-linux-.*/,
  /\/lib\/aarch64-linux-gnu\/ld-linux-.*/,
  /\/libc\.so\..*/,
  /\/libm\.so\..*/,
  /\/libmvec\.so\..*/,
  /\/libpthread\.so\..*/,
  /\/libdl\.so\..*/,
  /\/librt\.so\..*/,
  /\/libgcc_s\.so\..*/,
  /\/libstdc\+\+\.so\..*/,
  /\/libglib-2\.0\.so\..*/,
  /\/libgobject-2\.0\.so\..*/,
  /\/libgio-2\.0\.so\..*/,
  /\/libgmodule-2\.0\.so\..*/,
  /\/libdbus-1\.so\..*/,
  /\/libsystemd\.so\..*/,
  /\/libX.*\.so\..*/,
  /\/libxcb.*\.so\..*/,
  /\/libdrm\.so\..*/,
  /\/libgbm\.so\..*/,
  /\/libGL\.so\..*/,
  /\/libEGL\.so\..*/,
  /\/libGLESv2\.so\..*/,
  /\/libGLdispatch\.so\..*/,
  /\/libOpenGL\.so\..*/,
  /\/libglapi\.so\..*/,
  /\/libgallium.*\.so\..*/,
  /\/libvulkan\.so\..*/,
  /\/libudev\.so\..*/,
  /\/libgudev-1\.0\.so\..*/,
];
const isSystemExcluded = (p: string): boolean =>
  SYSTEM_EXCLUDED.some((re) => re.test(p));

const seenLibs = new Set<string>();
const pendingLibs: string[] = [];

function scanDeps(file: string): string[] {
  const out = execSyncText("ldd", [file]);
  const deps: string[] = [];
  for (const line of out.split("\n")) {
    const arrowMatch = /=>\s+(\/\S+)/.exec(line);
    const bareMatch = /^\s*(\/\S+)/.exec(line);
    const dep = arrowMatch?.[1] ?? bareMatch?.[1];
    if (dep) deps.push(dep);
  }
  return [...new Set(deps)].sort();
}

function queueDep(dep: string): void {
  if (!dep || !existsSync(dep)) return;
  const real = realpathSync(dep);
  if (isSystemExcluded(real)) return;
  if (seenLibs.has(dep)) return;
  seenLibs.add(dep);
  pendingLibs.push(dep);
}

console.log(`→ Bundling into ${outDir}`);
rmSync(outDir, { recursive: true, force: true });
mkdirSync(path.join(outDir, "bin"), { recursive: true });
mkdirSync(path.join(outDir, "lib"), { recursive: true });
const deployedBin = path.join(outDir, "bin", "avio-compositor");
copyFilePreservingMode(bin, deployedBin);

// meson's build-tree RUNPATH ($ORIGIN/subprojects/wlroots, ...) only resolves inside build/;
// repoint it at the bundled lib/ dir now that the binary lives at bundle/bin/. Without this,
// tools that inspect the ELF directly (e.g. linuxdeploy's dependency scan for the AppImage)
// report libwlroots-0.20.so as unresolvable even though the launcher's LD_LIBRARY_PATH would
// have found it at actual runtime.
run("patchelf", ["--set-rpath", "$ORIGIN/../lib", deployedBin], repoRoot);

for (const dep of scanDeps(bin)) queueDep(dep);

let idx = 0;
while (idx < pendingLibs.length) {
  const lib = pendingLibs[idx];
  idx += 1;
  const linkName = path.basename(lib);
  const realName = realpathSync(lib);
  const realBase = path.basename(realName);
  const destReal = path.join(outDir, "lib", realBase);
  if (!existsSync(destReal)) copyFilePreservingMode(realName, destReal);
  const destLink = path.join(outDir, "lib", linkName);
  if (linkName !== realBase && !existsSync(destLink))
    symlinkSync(realBase, destLink);
  for (const dep of scanDeps(realName)) queueDep(dep);
}

// Launcher: bundled libs first, then exec the compositor. avio spawns this (see
// compositor_bootstrap.rs).
const launcher = path.join(outDir, "avio-compositor");
await Bun.write(
  launcher,
  `#!/usr/bin/env bash\nhere="$(cd "$(dirname "\${BASH_SOURCE[0]}")" && pwd)"\nexport LD_LIBRARY_PATH="$here/lib\${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\nexec "$here/bin/avio-compositor" "$@"\n`,
);
chmodSync(launcher, 0o755);

console.log(`→ avio-compositor bundle at: ${outDir}`);

function run(cmd: string, args: string[], cwd: string): void {
  const result = Bun.spawnSync([cmd, ...args], {
    cwd,
    stdout: "inherit",
    stderr: "inherit",
  });
  if (!result.success) {
    process.exit(result.exitCode ?? 1);
  }
}

function execSyncText(cmd: string, args: string[]): string {
  const result = Bun.spawnSync([cmd, ...args]);
  return result.stdout.toString();
}

function commandExists(cmd: string): boolean {
  return Bun.spawnSync(["sh", "-c", `command -v ${cmd}`]).success;
}

function pkgConfigExists(pkg: string): boolean {
  return Bun.spawnSync(["pkg-config", "--exists", pkg]).success;
}

function copyFilePreservingMode(src: string, dest: string): void {
  Bun.spawnSync(["cp", "-p", src, dest]);
}
