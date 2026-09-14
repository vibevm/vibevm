/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * The deployment's default theme, put into the script that applies it.
 *
 * The script is read by three different readers — a browser, this test,
 * and the reader `vibe` serves from a machine's own store — so it stays
 * a plain file with a default of its own, and a build replaces that one
 * line (F-48, X-058). What has to be true is that the replacement
 * happened: a silent miss would leave a working page, a green build and
 * a configured setting with no effect, found months later by a reader.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { themeInitScript } from "./theme-init.ts";

const RAW = readFileSync(
  new URL("../../../design/theme-init.js", import.meta.url),
  "utf8",
);

test("the script the design system ships declares its own default", () => {
  assert.match(RAW, /var DEFAULT = "system";/);
  assert.equal(themeInitScript(RAW, "system"), RAW);
});

test("a configured theme replaces the declaration and nothing else", () => {
  const dark = themeInitScript(RAW, "dark");
  assert.match(dark, /var DEFAULT = "dark";/);
  assert.equal(dark.includes('DEFAULT = "system"'), false);
  assert.equal(dark.length, RAW.length + "dark".length - "system".length);
  /* The reader's own choice still wins over the site's default: the
     line that reads storage is untouched, and a stored value is what
     the script stamps. */
  assert.match(dark, /window\.localStorage\.getItem\(KEY\)/);
  assert.match(dark, /stored === "dark" \|\| stored === "light"/);
});

/**
 * A script with no declaration is a refusal rather than an unchanged
 * string. This is the one failure the substitution can have, and it is
 * silent by nature: the page works, the build is green, and the
 * deployment's setting does nothing at all.
 */
test("a script that says nowhere to put the default is refused", () => {
  assert.throws(
    () => themeInitScript("(function () {})();", "dark"),
    /theme-init\.js declares no/,
  );
});
