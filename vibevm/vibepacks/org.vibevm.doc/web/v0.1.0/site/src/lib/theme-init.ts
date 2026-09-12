/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * The theme script this build inlines, with the deployment's default in
 * it.
 *
 * `theme-init.js` is a plain script and has to stay one: it runs before
 * the first stylesheet is parsed, it is read by a browser, by a test and
 * by the reader `vibe` serves from a machine's own store, and a file
 * that only made sense after a bundler had been through it would be
 * three different things to those three readers. So it carries a
 * default of its own and this replaces that one line — the site's
 * configuration decides what a first-time reader sees (F-48, X-058), and
 * the configuration reaches the bundle through the environment like the
 * origin and the analytics id do.
 *
 * The substitution is asserted rather than attempted. A silent miss
 * would be the worst of the three outcomes: the page would work, the
 * build would be green, and the deployment's setting would simply have
 * no effect — noticed by a reader, months later, as «the site ignores
 * what it says it does».
 */

import type { DefaultTheme } from "../config.ts";

/** The line the script declares its own default on. */
const DECLARATION = /^(\s*var DEFAULT = )"(system|light|dark)";$/m;

/**
 * `raw` with its default replaced by `theme`.
 *
 * The script is unchanged when the two agree, which is the ordinary
 * case: the campaign's answer to «which theme» is the reader's own
 * system setting until the design review says otherwise.
 */
export function themeInitScript(raw: string, theme: DefaultTheme): string {
  const found = raw.match(DECLARATION);
  if (found === null) {
    throw new Error(
      'theme-init.js declares no `var DEFAULT = "…";` line: the site\'s ' +
        "default theme cannot be put into a script that does not say where " +
        "it goes",
    );
  }
  return raw.replace(DECLARATION, `$1"${theme}";`);
}
