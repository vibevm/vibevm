/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * What a theme IS, in one place: three states, where the choice is kept,
 * and what changing it does to the document.
 *
 * It is its own module because two pages ask for it and only one of them
 * has a reader. A documentation page runs the whole reading-settings
 * behaviour and the theme is one of its four settings; the landing runs
 * nothing at all and still carries the switch in its header. Two copies
 * of «what a theme change means» would be two chances for the landing
 * and the manual to disagree about a reader's own choice — which is kept
 * under one key, read by `theme-init.js` before either of them runs.
 *
 * Three states and the third one is not a spare. `dark` and `light` are
 * the reader's explicit choice and are stamped on the root element;
 * `system` stamps NOTHING, because the absence of the attribute is what
 * hands the decision to `prefers-color-scheme` in `tokens.css`. A
 * two-state toggle would take the default away and give a reader no way
 * back to it.
 *
 * The site's default is what a reader who has chosen nothing gets, and
 * it arrives from the deployment's configuration rather than from here
 * (F-48, X-058) — the same value `theme-init.js` was built with, so the
 * page the reader sees at the first frame and the button this marks
 * current are answering out of one source.
 */

import { SITE } from "../config.ts";
import { all } from "./dom.ts";
import { readLocal, writeLocal } from "./storage.ts";

export type Theme = "dark" | "light" | "system";

/** Whether a value is one of the three — asked, never asserted. */
export function isTheme(value: unknown): value is Theme {
  return value === "dark" || value === "light" || value === "system";
}

/**
 * The theme this reader is on: their own choice if they made one, and
 * the site's default otherwise.
 *
 * A stored value comes from a previous version of this page as often as
 * from this one, so it is checked rather than trusted; anything else is
 * the same state as never having chosen.
 */
export function storedTheme(): Theme {
  const stored = readLocal("theme");
  return isTheme(stored) ? stored : SITE.defaultTheme;
}

/** Remember the choice, where `theme-init.js` will read it next time. */
export function keepTheme(theme: Theme): void {
  writeLocal("theme", theme);
}

/**
 * Put a theme on the document, and on every control that offers one.
 *
 * The controls are found by their data attribute rather than by a
 * reference, because a page may carry two — the switch in the header and
 * the row in the reading panel — and a reader who changes the theme in
 * one must not be left looking at the other still claiming the old one.
 */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);

  for (const button of all("[data-theme-choice]")) {
    const current = button.dataset["themeChoice"] === theme;
    button.classList.toggle("is-current", current);
    button.setAttribute("aria-pressed", current ? "true" : "false");
  }
}

/**
 * The switch on a page that has no reading settings behind it.
 *
 * The landing offers the theme and nothing else — there is no column to
 * widen and no block numbers to hide on a page that is not a document —
 * so it starts this rather than the reader. A documentation page starts
 * `startSettings`, which drives the same controls through the same three
 * functions above.
 */
export function startThemeSwitch(): () => void {
  applyTheme(storedTheme());

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const choice = target.closest("[data-theme-choice]");
    if (!(choice instanceof HTMLElement)) return;
    const theme = choice.dataset["themeChoice"];
    if (!isTheme(theme)) return;
    applyTheme(theme);
    keepTheme(theme);
  };

  document.addEventListener("click", onClick);
  return () => document.removeEventListener("click", onClick);
}
