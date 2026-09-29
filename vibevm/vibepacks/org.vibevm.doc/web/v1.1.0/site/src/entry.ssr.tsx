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

import { localeFromPath } from "./landing/i18n.ts";
import Root from "./root.tsx";

/**
 * The document's language, read off the address being rendered.
 *
 * `<html lang>` is a container attribute: it is written before any
 * component runs, so no route can set it and it has to be decided here.
 * The address is the only thing available at this point that knows the
 * answer — and on this site the address is where the language lives
 * anyway (D-06): English at the root, every other language a directory
 * under it.
 *
 * A documentation address answers English here and gets its real
 * language on the article the pipeline renders, which is where a page
 * whose shell and whose text are in different languages needs it.
 *
 * A missing or unparsable URL answers English rather than throwing: a
 * page that renders in the wrong language is a defect, and a page that
 * does not render is an outage.
 */
function documentLanguage(url: unknown): string {
  if (typeof url !== "string") return "en";
  try {
    return localeFromPath(new URL(url).pathname);
  } catch {
    return "en";
  }
}

export default function render(opts: RenderToStreamOptions) {
  const url: unknown = opts.serverData?.["url"];
  return renderToStream(<Root />, {
    ...opts,
    containerAttributes: {
      lang: documentLanguage(url),
      ...opts.containerAttributes,
    },
  });
}
