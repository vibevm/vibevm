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
 *  3. The island is a placeholder. The server renders the real one per
 *     request out of the machine store and puts it where the marker is.
 *
 * Everything else — the components, the routes, the styles, the entry —
 * is the same source. That is the whole claim of «one shell, two
 * adapters», and the file is short because the claim is true.
 */

import { qwikVite } from "@qwik.dev/core/optimizer";
import { qwikRouter } from "@qwik.dev/router/vite";
import { defineConfig, type UserConfig } from "vite";

import { ISLAND_PLACEHOLDER } from "./src/lib/island-placeholder.ts";

export default defineConfig((): UserConfig => {
  return {
    base: "/doc/",
    define: { __VIBE_ISLAND_HTML__: JSON.stringify(ISLAND_PLACEHOLDER) },
    plugins: [
      qwikRouter({ routesDir: "src/routes/doc", trailingSlash: true }),
      qwikVite({ client: { outDir: "dist-embedded" }, ssr: { outDir: "server-embedded" } }),
    ],
  };
});
