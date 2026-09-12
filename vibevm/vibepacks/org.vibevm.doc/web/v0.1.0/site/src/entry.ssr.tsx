/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-QWIK */

/**
 * The server entry — what renders a page when there is no browser yet.
 *
 * Both adapters go through here: the static build calls it once per page
 * at build time and writes the result to disk, and the embedded build
 * calls it to produce the route template `vibe` fills in. There is
 * deliberately no third path — an SSR server would be a fourth way for a
 * page to be produced, and the site has no need of one (`##STACK-NODE-SERVER-ONLY`).
 */

import {
  renderToStream,
  type RenderToStreamOptions,
} from "@qwik.dev/core/server";

import Root from "./root.tsx";

export default function render(opts: RenderToStreamOptions) {
  return renderToStream(<Root />, {
    ...opts,
    // The language of the shell, not of the documentation: a page's own
    // language is a segment of its address and is set on the article the
    // pipeline renders.
    containerAttributes: { lang: "en", ...opts.containerAttributes },
  });
}
