/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

import { component$ } from "@qwik.dev/core";
import {
  DocumentHeadTags,
  QwikRouterProvider,
  RouterOutlet,
} from "@qwik.dev/router";
import themeInit from "@vibe-docs/design/theme-init.js?raw";

import { SITE } from "./config.ts";
import { themeInitScript } from "./lib/theme-init.ts";

import "./global.css";

/**
 * The theme script with this deployment's default in it, composed once
 * at module scope: the value is a build-time constant and composing it
 * per render would hand the framework a new string on every page.
 */
const THEME_INIT = themeInitScript(themeInit, SITE.defaultTheme);

/**
 * The document itself — the only place the site writes `<head>`.
 *
 * The theme script is inlined rather than linked, and the order matters
 * more than the byte count: it runs before the stylesheet is parsed, so
 * a reader who chose light while the system prefers dark never sees the
 * page repaint. A linked script could be delayed by the network; this
 * one cannot be. The cost is that a Content-Security-Policy for this
 * site has to carry the script's hash — which is the deployment's
 * business, and cheaper than a flash on every page load.
 *
 * `DocumentHeadTags` renders what each route declared in its `head`
 * export: the title, the description, `canonical`, the `hreflang` set,
 * Open Graph, the Twitter card, the structured data and the font
 * preloads. Without it a route's `head` is a value nobody reads, and a
 * page can be perfectly right to a reader and invisible to a crawler
 * (`##SEO-STRUCTURED-DATA`). It sits after the two constants above so
 * that the charset is settled before anything carrying a URL is parsed.
 */
export default component$(() => {
  return (
    <QwikRouterProvider>
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <script dangerouslySetInnerHTML={THEME_INIT} />
        <DocumentHeadTags />
      </head>
      <body>
        <RouterOutlet />
      </body>
    </QwikRouterProvider>
  );
});
