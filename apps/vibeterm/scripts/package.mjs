#!/usr/bin/env node
/**
 * vibeterm packaging — produce a relocatable, self-contained directory that the
 * VVM version manager diff-copies into an instance next to the `vibe` binary.
 *
 * `npm install` alone is NOT enough: npm 11 blocks native/postinstall scripts,
 * so Electron's binary download and node-pty's prebuild staging do not run.
 * So this does the full dance, in order:
 *
 *   1. npm install                                 — deps (idempotent)
 *   2. npm rebuild node-pty --foreground-scripts   — node-pty prebuild + ConPTY DLLs
 *   3. node node_modules/electron/install.js       — Electron's binary (its postinstall)
 *   4. npx @electron/packager … --dir --asar=false — the relocatable dir
 *
 * NO @electron/rebuild step. node-pty is built on node-addon-api (N-API), so its
 * shipped prebuilds are ABI-stable across Node/Electron versions: the same .node
 * loads in system Node 24 (modules=137) and Electron 32's Node 20 (modules=128)
 * and ConPTY spawns correctly under Electron (verified). node-pty's own install
 * script (`scripts/prebuild.js || node-gyp rebuild`) takes the prebuild branch and
 * never invokes node-gyp, so deps/winpty/src/winpty.gyp — whose `<!(cmd /c "cd
 * shared && GetCommitHash.bat")` assumes cwd=deps/winpty/src and fails under
 * node-gyp's real cwd — is never reached. Forcing @electron/rebuild would re-enter
 * node-gyp, hit that broken winpty.gyp, and fail; it is both unnecessary and broken
 * for this N-API addon, so it is omitted.
 *
 * Output: `<out>/vibeterm-<plat>-<arch>/` — `electron(.exe)` at the root,
 * `resources/app/{main.cjs,renderer.js,index.html,lib/,package.json,
 * node_modules/}` inside, plus the Chromium runtime + `LICENSES.chromium.html`.
 * The Rust caller (NpmPackager) resolves that single child.
 *
 * Packaging is per-OS: it runs on the target host (node-pty's native addon and
 * Electron's runtime are OS/arch-specific). Usage:
 *
 *     node scripts/package.mjs --out <abs-dir>
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const appDir = resolve(here, '..');

const outIdx = process.argv.indexOf('--out');
const out =
  outIdx >= 0 && process.argv[outIdx + 1]
    ? resolve(process.argv[outIdx + 1])
    : resolve(appDir, '..', 'build', 'vibeterm-dist');
if (!existsSync(out)) mkdirSync(out, { recursive: true });

const PLATFORM = process.platform;
const ARCH = process.arch;
// Windows: npm/npx are .cmd shims, so spawn through a shell everywhere for uniformity.
const SHELL = PLATFORM === 'win32';

function run(label, cmd, args) {
  process.stderr.write(`[package] ${label}: ${cmd} ${args.join(' ')}\n`);
  const res = spawnSync(cmd, args, { stdio: 'inherit', shell: SHELL, cwd: appDir });
  if (res.status !== 0) {
    process.stderr.write(`[package] FAILED: ${label} (exit ${res.status})\n`);
    process.exit(res.status ?? 1);
  }
}

run('npm install', 'npm', ['install']);
run('node-pty prebuild', 'npm', ['rebuild', 'node-pty', '--foreground-scripts']);
run('electron binary', 'node', ['node_modules/electron/install.js']);

// electron-packager: a plain DIRECTORY (no installer), no asar (the unpacked
// tree is transparent and diffable by VVM). @electron/packager v20 ships asar
// OFF by default, so no flag is passed (passing `--asar=false` warns). `--prune`
// drops devDeps from the packaged node_modules; `--overwrite` replaces a prior
// build.
run('electron-packager', 'npx', [
  '@electron/packager',
  appDir,
  'vibeterm',
  '--dir',
  `--platform=${PLATFORM}`,
  `--arch=${ARCH}`,
  `--out=${out}`,
  '--overwrite',
  '--prune=true',
]);

const children = readdirSync(out).filter((n) => n.startsWith('vibeterm-'));
if (children.length !== 1) {
  process.stderr.write(
    `[package] expected exactly one 'vibeterm-*' child under ${out}, found ${children.length}: ${children.join(', ')}\n`,
  );
  process.exit(1);
}
// The single line of stdout the Rust caller reads to locate the produced dir.
process.stdout.write(join(out, children[0]) + '\n');
