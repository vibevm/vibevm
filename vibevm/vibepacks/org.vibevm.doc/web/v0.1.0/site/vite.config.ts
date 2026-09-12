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
 *
 * The one thing this configuration decides is WHICH DOCUMENTATION the
 * build renders: the manifests of the trees `VIBE_DOC_OUT` names, or the
 * package's own fixture pair when it names none. A bundler cannot import
 * a file whose path only the deployment knows, so the manifests arrive
 * as a substituted value — the same channel the card addresses take.
 */

import { qwikVite } from "@qwik.dev/core/optimizer";
import { qwikRouter } from "@qwik.dev/router/vite";
import { defineConfig, type UserConfig } from "vite";

import { buildLibrary } from "../tools/library-source.mjs";

export default defineConfig((): UserConfig => {
  return {
    base: "/",
    define: {
      __VIBE_DOC_MANIFESTS__: JSON.stringify(buildLibrary()),
      __VIBE_LOCAL_READER__: "false",
    },
    plugins: [
      qwikRouter({ trailingSlash: true }),
      qwikVite({ client: { outDir: "dist" }, ssr: { outDir: "server" } }),
    ],
  };
});
