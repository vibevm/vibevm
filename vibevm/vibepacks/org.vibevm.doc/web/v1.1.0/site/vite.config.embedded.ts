/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * The embedded build: the documentation alone, at base `/doc/`.
 *
 * Three differences from the static build, and no fourth:
 *
 *  1. The route directory is `src/routes/doc`, so the documentation
 *     routes sit at the root of this tree and the landing routes are
 *     not in it at all — the exclusion is where the files are, not a
 *     condition inside them.
 *  2. The base is `/doc/`, which is where `vibe doc serve` mounts what
 *     this build produces. The two builds arrive at the same public
 *     address from opposite directions, which is why every link on the
 *     site is written through one function instead of by hand.
 *  3. The library is the package's own fixture pair and never the trees
 *     a deployment rendered. This build produces ONE route template that
 *     `vibe doc serve` dresses every page of every package in, and the
 *     shell reads the library it is actually serving from
 *     `<base>manifest.json` while the page is being read — so
 *     prerendering a deployment's ninety-nine addresses here would be
 *     ninety-eight templates thrown away, and a shell whose weight
 *     depended on what happened to be in the environment when it was
 *     embedded.
 *
 * The island is a placeholder in BOTH builds now, so it is no longer one
 * of the differences: what changes is who fills the hole — the build
 * driver over the pages it generated, or the server per request.
 *
 * Everything else — the components, the routes, the styles, the entry —
 * is the same source. That is the whole claim of «one shell, two
 * adapters», and the file is short because the claim is true.
 */

import { qwikVite } from "@qwik.dev/core/optimizer";
import { qwikRouter } from "@qwik.dev/router/vite";
import { defineConfig, type UserConfig } from "vite";

import { fixtureLibrary } from "../tools/library-source.mjs";

export default defineConfig((): UserConfig => {
  return {
    base: "/doc/",
    define: {
      __VIBE_DOC_MANIFESTS__: JSON.stringify(fixtureLibrary()),
      __VIBE_LOCAL_READER__: "true",
    },
    plugins: [
      qwikRouter({ routesDir: "src/routes/doc", trailingSlash: true }),
      qwikVite({
        client: { outDir: "dist-embedded" },
        ssr: { outDir: "server-embedded" },
      }),
    ],
  };
});
