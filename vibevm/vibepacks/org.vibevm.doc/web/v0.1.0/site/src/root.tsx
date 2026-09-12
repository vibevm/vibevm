/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

import { component$ } from "@qwik.dev/core";
import { QwikRouterProvider, RouterOutlet } from "@qwik.dev/router";
import themeInit from "@vibe-docs/design/theme-init.js?raw";

import "./global.css";

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
 */
export default component$(() => {
  return (
    <QwikRouterProvider>
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <script dangerouslySetInnerHTML={themeInit} />
      </head>
      <body>
        <RouterOutlet />
      </body>
    </QwikRouterProvider>
  );
});
