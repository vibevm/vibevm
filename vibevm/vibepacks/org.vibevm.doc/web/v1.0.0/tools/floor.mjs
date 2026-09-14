#!/usr/bin/env node
// `pnpm floor:discipline` — the seven-step floor of the AI-Native
// TypeScript discipline (prettier -> tsc -> tests -> eslint -> conform ->
// specmap -> test-gate), run over this package.
//
// The floor itself is a compiled binary that ships with the discipline
// stack, not an npm package, so the one thing this script does is find
// it. Three places are tried, in the order of how specific the answer is:
// an explicit TYPESCRIPT_AI_NATIVE, the PATH, and finally the built slot
// of a host repository this package happens to be checked out inside.
// The last one is a convenience for working in the vibevm tree and never
// a dependency: nothing in `package.json` names a path outside this
// directory, so the package still builds where no such tree exists.
//
// Absent tool is a failure with a recipe, never a skip — a floor that
// quietly runs six of seven steps reports a number with no denominator.

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXE = process.platform === "win32" ? "typescript-ai-native.exe" : "typescript-ai-native";
const SLOT = "vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang";

/** The built binary inside a host checkout, newest version slot first. */
function fromHostCheckout() {
  let dir = PACKAGE_ROOT;
  for (;;) {
    const slot = join(dir, SLOT);
    if (existsSync(slot)) {
      const versions = readdirSync(slot).sort().reverse();
      for (const version of versions) {
        for (const profile of ["release", "debug"]) {
          const candidate = join(slot, version, "target", profile, EXE);
          if (existsSync(candidate)) return candidate;
        }
      }
    }
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

/** The binary under whatever name PATH resolves, proven by `--help`. */
function fromPath() {
  const probe = spawnSync(EXE, ["--help"], { stdio: "ignore", shell: false });
  return probe.status === 0 ? EXE : null;
}

const binary = process.env["TYPESCRIPT_AI_NATIVE"] ?? fromPath() ?? fromHostCheckout();

if (binary === null) {
  process.stderr.write(
    [
      "floor: the discipline binary `typescript-ai-native` was not found.",
      "",
      "It is a compiled tool of the stack package, not an npm dependency.",
      "Any one of these makes this script work:",
      `  1. set TYPESCRIPT_AI_NATIVE to the binary's full path;`,
      `  2. put ${EXE} on PATH;`,
      `  3. inside a vibevm checkout, build it once:`,
      `       vibe bin build typescript-ai-native --assume-yes`,
      "",
      "The floor is not run and nothing is judged green.",
      "",
    ].join("\n"),
  );
  process.exit(1);
}

const run = spawnSync(binary, ["floor", "--path", PACKAGE_ROOT, ...process.argv.slice(2)], {
  stdio: "inherit",
  shell: false,
});

process.exit(run.status ?? 1);
