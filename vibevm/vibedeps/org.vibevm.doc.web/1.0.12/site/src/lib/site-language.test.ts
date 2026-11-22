/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The site's language, apart from the documentation's.
 *
 * What is worth a test here is not the table — a wrong translation is a
 * reading, not a failure — but the three rules around it: that the table
 * reads both ways and agrees with itself, that a string it does not know
 * is left alone rather than blanked, and that a browser asking for a
 * language nobody here speaks gets the one the interface is written in.
 */

import assert from "node:assert/strict";
import test from "node:test";

import {
  RUSSIAN_CHROME,
  SITE_LANGUAGES,
  chromeIn,
  isSiteLanguage,
  preferredSiteLanguage,
} from "./site-language.ts";

test("the two languages are the two the site is published in", () => {
  assert.deepEqual([...SITE_LANGUAGES], ["en", "ru"]);
  assert.equal(isSiteLanguage("ru"), true);
  assert.equal(isSiteLanguage("de"), false);
  assert.equal(isSiteLanguage(undefined), false);
});

test("every row reads both ways, so a switch back is not a reload", () => {
  for (const [english, russian] of Object.entries(RUSSIAN_CHROME)) {
    assert.equal(chromeIn(english, "ru"), russian, english);
    assert.equal(chromeIn(russian, "en"), english, russian);
  }
});

/**
 * Two English strings sharing one Russian one would make the way back
 * ambiguous, and the reverse table would silently keep whichever came
 * last — so the round trip above would pass for one of them and fail for
 * the other. This says the table cannot get into that state at all.
 */
test("no two rows share a translation", () => {
  const russian = Object.values(RUSSIAN_CHROME);
  assert.equal(new Set(russian).size, russian.length);
});

test("a string the chrome does not carry is left alone", () => {
  // The documentation's own words: a page title, an abstract, the text.
  assert.equal(chromeIn("Every block once", "ru"), null);
  assert.equal(chromeIn("", "ru"), null);
  // Already in the language asked for is also «nothing to do».
  assert.equal(chromeIn("Оглавление", "ru"), null);
});

test("the browser's first known preference decides, in its own order", () => {
  assert.equal(preferredSiteLanguage(["ru-RU", "en-US"]), "ru");
  assert.equal(preferredSiteLanguage(["en-GB", "ru"]), "en");
  // A language the site does not carry is passed over, not approximated.
  assert.equal(preferredSiteLanguage(["de-DE", "ru"]), "ru");
  // And nothing known at all is the language the interface is written in.
  assert.equal(preferredSiteLanguage(["de", "fr"]), "en");
  assert.equal(preferredSiteLanguage([]), "en");
});
