#!/usr/bin/env bun
// Builds the gst-video crate's `gst-host` binary and drops it into src-tauri/binaries/ named
// with the Rust target triple, matching tauri's `bundle.externalBin` convention (see
// tauri.conf.json) so the packaged app can find it next to the main executable.
// Run before both `tauri dev` and `tauri build` (see tauri.conf.json's before*Command hooks).
import { existsSync, mkdirSync } from 'node:fs'
import path from 'node:path'

const repoRoot = path.dirname(import.meta.dir)
const srcTauri = path.join(repoRoot, 'src-tauri')

const rustInfo = execSyncText('rustc', ['-vV'])
const targetTriple = /host:\s*(\S+)/.exec(rustInfo)?.[1]
if (!targetTriple) {
  console.error('[gst-host sidecar] could not determine the rustc host triple from `rustc -vV`')
  process.exit(1)
}

const debug = process.env.TAURI_ENV_DEBUG === 'true'
const profile = debug ? 'debug' : 'release'
const cargoArgs = ['build', '-p', 'gst-video', '--bin', 'gst-host', ...(debug ? [] : ['--release'])]

run('cargo', cargoArgs, repoRoot)

const ext = targetTriple.includes('windows') ? '.exe' : ''
const built = path.join(repoRoot, 'target', profile, `gst-host${ext}`)
if (!existsSync(built)) {
  console.error(`[gst-host sidecar] expected build output missing: ${built}`)
  process.exit(1)
}

const binariesDir = path.join(srcTauri, 'binaries')
mkdirSync(binariesDir, { recursive: true })
const dest = path.join(binariesDir, `gst-host-${targetTriple}${ext}`)
await Bun.write(dest, Bun.file(built))

console.log(
  `[gst-host sidecar] built ${profile} binary for ${targetTriple} -> ${path.relative(repoRoot, dest)}`
)

function run(cmd: string, args: string[], cwd: string): void {
  const result = Bun.spawnSync([cmd, ...args], { cwd, stdout: 'inherit', stderr: 'inherit' })
  if (!result.success) {
    process.exit(result.exitCode ?? 1)
  }
}

function execSyncText(cmd: string, args: string[]): string {
  const result = Bun.spawnSync([cmd, ...args])
  return result.stdout.toString()
}
