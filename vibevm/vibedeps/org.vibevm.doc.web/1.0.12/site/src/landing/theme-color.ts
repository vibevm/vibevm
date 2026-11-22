/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import palette from "@vibe-docs/design/palette.css?raw";

/**
 * The one colour the site states outside CSS — and it still comes from
 * the palette.
 *
 * `<meta name="theme-color">` paints the browser's own chrome around the
 * page, and a meta tag cannot hold a `var()`: the value has to be a
 * literal in the HTML. R-27 forbids writing one in a component, and it
 * is right to — a second copy of a colour is a copy that will drift. So
 * the value is read out of `palette.css` at build time, from the same
 * declaration the stylesheet reads. The literal exists once, in the file
 * whose job is to hold literals.
 *
 * The regex is deliberately narrow: a name, a colon, a value, a
 * semicolon. If the palette ever stops declaring the name this asks for,
 * the build fails here with the name in the message rather than shipping
 * a page whose browser chrome is the string `undefined`.
 */
function paletteValue(name: string): string {
  const pattern = new RegExp(`${name}\\s*:\\s*([^;]+);`);
  const found = pattern.exec(palette);
  const value = found?.[1];
  if (value === undefined) {
    throw new Error(
      `palette.css declares no ${name}: <meta name="theme-color"> has nothing to say`,
    );
  }
  return value.trim();
}

/**
 * The browser-chrome colour, as the Astro landing set it: the warm
 * near-black the dark theme is built on.
 *
 * One value and not two. A site with two themes could name a colour per
 * `prefers-color-scheme`, and one day it probably should — but the port
 * moves the landing one to one (D-28, F-74), and choosing what the light
 * theme paints the address bar is a question for the design review, not
 * for the move.
 */
export const THEME_COLOR = paletteValue("--ink");
