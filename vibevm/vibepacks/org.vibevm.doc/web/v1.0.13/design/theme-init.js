/* theme-init.js — the first thing the document runs, and the only script
   that must run before the stylesheet.
   ---------------------------------------------------------------------
   Two of the reading settings are decided here, and they are here
   together because they are the two a stylesheet acts on: what a reader
   sees at the first frame must already be theirs, or the page is telling
   them their setting did not take. Everything else a reader may change —
   the text size, the column, the block numbers — is applied by
   `reader/settings.ts` after the document is parsed, because none of it
   can flash.

   The theme has three states, not two: `dark` and `light` are the
   reader's explicit choice and are stamped on the root element;
   `system` is the default and stamps NOTHING, because the absence of the
   attribute is what lets `@media (prefers-color-scheme)` in `tokens.css`
   decide (PROP-057 `##READER-SETTINGS`).

   The contents view follows the same shape with two states: `sections`
   is the reader's explicit choice and is stamped, and the learning path
   is the default and stamps nothing, because a documentation that
   declared a path opens on it (`##NAV-CHAPTERS-READER`) and the absence
   of the attribute is what the column's stylesheet reads as «the path».
   A documentation with no path ignores the attribute entirely.

   It is inlined into `<head>` ahead of every stylesheet link. A reader
   who chose light while the system prefers dark must never see a dark
   page repaint to light — the flash is not a cosmetic complaint, it is
   the page telling the reader their setting did not take.

   Storage can throw rather than return null — a browser set to block
   site data raises on the property access itself — so the read is
   wrapped and a failure means the site's default, the same as never
   having chosen. Nothing here reads the network, and nothing here needs
   the DOM to be parsed: at this point `document.documentElement` is the
   only element that exists, and it is the one being written.

   The site's default is the line marked below, and a build replaces it
   from `[site].default_theme` in the deployment's configuration (F-48,
   X-058). The file is valid on its own with the value it carries here,
   which is what lets it stay a plain script that a browser, a test and a
   build all read the same way. */

(function () {
  var KEY = "vibe-doc:theme";
  var DEFAULT = "system";
  var stored = null;
  try {
    stored = window.localStorage.getItem(KEY);
  } catch (e) {
    stored = null;
  }
  var theme = stored === "dark" || stored === "light" ? stored : DEFAULT;
  if (theme === "dark" || theme === "light") {
    document.documentElement.setAttribute("data-theme", theme);
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
})();

(function () {
  var KEY = "vibe-doc:contents";
  var stored = null;
  try {
    stored = window.localStorage.getItem(KEY);
  } catch (e) {
    stored = null;
  }
  if (stored === "sections") {
    document.documentElement.setAttribute("data-contents-view", "sections");
  } else {
    document.documentElement.removeAttribute("data-contents-view");
  }
})();
