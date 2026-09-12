/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * The static build: the whole site at base `/`.
 *
 * One base for one application. The landing at `/`, the Russian landing
 * at `/ru/` and the documentation under `/doc/` are route DIRECTORIES,
 * not three mount points — Qwik's base is a prefix over the whole route
 * tree, and using it to separate sections would put the prefix in front
 * of every one of them.
 *
 * `trailingSlash` stays at its default of `true` and is written down
 * anyway. The generator produces `<route>/index.html`; with the flag
 * turned off it produced one page out of seven and still exited 0. A
 * default whose other value silently breaks the build is worth one line
 * of config and a sentence saying so.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { qwikVite } from "@qwik.dev/core/optimizer";
import { qwikRouter } from "@qwik.dev/router/vite";
import { defineConfig, type UserConfig } from "vite";

const HERE = dirname(fileURLToPath(import.meta.url));

/**
 * The page this build shows. Today it is the pipeline's own golden, so
 * the shell can be built and measured before a documentation package is
 * on the machine; the shape does not change when a real one arrives.
 */
const ISLAND = readFileSync(join(HERE, "src", "fixtures", "island.html"), "utf8");

export default defineConfig((): UserConfig => {
  return {
    base: "/",
    define: { __VIBE_ISLAND_HTML__: JSON.stringify(ISLAND) },
    plugins: [
      qwikRouter({ trailingSlash: true }),
      qwikVite({ client: { outDir: "dist" }, ssr: { outDir: "server" } }),
    ],
  };
});
