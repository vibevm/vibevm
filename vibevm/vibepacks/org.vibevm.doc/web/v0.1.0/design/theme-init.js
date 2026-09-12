/* theme-init.js — the first thing the document runs, and the only script
   that must run before the stylesheet.
   ---------------------------------------------------------------------
   The theme has three states, not two: `dark` and `light` are the
   reader's explicit choice and are stamped on the root element;
   `system` is the default and stamps NOTHING, because the absence of the
   attribute is what lets `@media (prefers-color-scheme)` in `tokens.css`
   decide (PROP-057 `##READER-SETTINGS`).

   It is inlined into `<head>` ahead of every stylesheet link. A reader
   who chose light while the system prefers dark must never see a dark
   page repaint to light — the flash is not a cosmetic complaint, it is
   the page telling the reader their setting did not take.

   Storage can throw rather than return null — a browser set to block
   site data raises on the property access itself — so the read is
   wrapped and a failure means «system», the same as never having
   chosen. Nothing here reads the network, and nothing here needs the
   DOM to be parsed: at this point `document.documentElement` is the only
   element that exists, and it is the one being written. */

(function () {
  var KEY = "vibe-doc:theme";
  var stored = null;
  try {
    stored = window.localStorage.getItem(KEY);
  } catch (e) {
    stored = null;
  }
  if (stored === "dark" || stored === "light") {
    document.documentElement.setAttribute("data-theme", stored);
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
})();
