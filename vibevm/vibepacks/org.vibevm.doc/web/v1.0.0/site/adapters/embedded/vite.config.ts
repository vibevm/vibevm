/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-ONE-BASE */

/**
 * The embedded adapter: the same static generator over the documentation
 * routes only, producing the route template `vibe` fills in.
 *
 * It is the SSG adapter and not something else on purpose. What the
 * embedded shell needs is a page with everything around the island
 * already rendered — the head, the styles, the chunks, the layout — and
 * a marked hole where the island goes. That is exactly a prerendered
 * page whose island happens to be a comment, so the server's work is a
 * string replacement rather than a render, and the bytes around the
 * island are provably the same ones the public site serves.
 */

import { ssgAdapter } from "@qwik.dev/router/adapters/ssg/vite";
import { extendConfig } from "@qwik.dev/router/vite";

import baseConfig from "../../vite.config.embedded.ts";

export default extendConfig(baseConfig, () => {
  return {
    build: {
      ssr: true,
      rolldownOptions: { input: ["@qwik-router-config"] },
    },
    // The origin is a local one: the addresses in an embedded page point
    // at the reader's own machine, and a public origin baked into a
    // canonical link there would send them to the public site instead.
    plugins: [ssgAdapter({ origin: "http://127.0.0.1" })],
  };
});
