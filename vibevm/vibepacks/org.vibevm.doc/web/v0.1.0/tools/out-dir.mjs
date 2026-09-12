#!/usr/bin/env node
// Where the static build writes, decided once for the driver and the
// Vite configuration that have to agree about it.
//
// `dist` unless `VIBE_SITE_DIST` names somewhere else. The knob exists
// because a build renders ONE library set — the trees `VIBE_DOC_OUT`
// names — and a second set is a second site: the end-to-end run builds
// the package's fixture library into `dist` and a two-library set beside
// it, and a build that could only ever write one directory would have to
// overwrite the first to measure the second.
//
// It is read here and in `site/vite.config.ts`, and it travels to the
// Vite child through the environment it already inherits. The embedded
// build has no such knob: it produces one route template rather than a
// library, so there is no second one to build.

/** The environment name that moves the static build's output. */
export const OUT_DIR_ENV = "VIBE_SITE_DIST";

/** The directory the static build writes its pages and files into. */
export function staticOutDir(env = process.env) {
  const named = (env[OUT_DIR_ENV] ?? "").trim();
  return named.length === 0 ? "dist" : named;
}

/**
 * Where the adapter's server bundle goes — an intermediate of the build
 * and not part of the output, so it follows the output's name rather
 * than colliding with the default one.
 */
export function staticServerDir(env = process.env) {
  const out = staticOutDir(env);
  return out === "dist" ? "server" : `${out}-server`;
}
