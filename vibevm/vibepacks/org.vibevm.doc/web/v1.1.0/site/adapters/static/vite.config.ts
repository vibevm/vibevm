/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-QWIK */

/**
 * The static adapter over the site build: every route prerendered to
 * `<route>/index.html`, with the island already in it.
 *
 * The integration is called `ssg`, not `static` — the CLI's old name for
 * it fails with «Invalid integration», while the old import path still
 * works as an alias. Naming the current one here keeps the file from
 * being the place someone learns that the hard way.
 */

import { ssgAdapter } from "@qwik.dev/router/adapters/ssg/vite";
import { extendConfig } from "@qwik.dev/router/vite";

import baseConfig from "../../vite.config.ts";

export default extendConfig(baseConfig, () => {
  return {
    build: {
      ssr: true,
      rolldownOptions: { input: ["@qwik-router-config"] },
    },
    plugins: [ssgAdapter({ origin: "https://vibevm.org" })],
  };
});
