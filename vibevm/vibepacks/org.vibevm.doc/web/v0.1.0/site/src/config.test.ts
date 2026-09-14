/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ANALYTICS */

/**
 * What a build does with what it was told, and with what it was not.
 *
 * Every value here identifies a deployment rather than the source, so
 * every one of them has an answer for «nothing was said» — and the
 * answers are not alike. An absent origin is a default, because a page
 * has to claim to live somewhere. An absent website id is an
 * instruction: publish no tag at all, because a tag with an empty
 * attribute loads a script that reports to nobody. An absent analytics
 * host is the domain itself, which is what first-party means. An absent
 * theme is `dark`, the site's own (F-48, the review of 2026-09-14) —
 * which is a decision and therefore has to be pinned somewhere a change
 * to it is visible.
 *
 * They are asserted over a literal environment rather than the ambient
 * one, which is why `siteConfig` takes its input: a test that had to set
 * a process-wide variable would be a test that could not run beside
 * another one.
 */

import assert from "node:assert/strict";
import test from "node:test";

import { ENV_NAMES, siteConfig } from "./config.ts";

test("a build told nothing publishes no analytics tag and claims one domain", () => {
  const config = siteConfig({});
  assert.equal(config.origin, "https://vibevm.org");
  assert.equal(config.umamiWebsiteId, "");
  assert.equal(config.umamiHostUrl, "");
  assert.equal(config.indexNowKey, "");
  assert.equal(config.defaultTheme, "dark");
  assert.match(config.lastmod, /^\d{4}-\d{2}-\d{2}$/);
});

/**
 * The two values of the analytics tag travel separately because they
 * answer different questions — which property, and where it lives — and
 * a deployment may name the second without the first only to discover
 * that naming it changed nothing: no id, no tag, wherever it would have
 * reported to.
 */
test("the analytics property and the host it reports to arrive separately", () => {
  const config = siteConfig({
    [ENV_NAMES.umamiWebsiteId]: " abc-123 ",
    [ENV_NAMES.umamiHostUrl]: "https://vibevm.org/",
  });
  assert.equal(config.umamiWebsiteId, "abc-123");
  assert.equal(config.umamiHostUrl, "https://vibevm.org");
});

/**
 * A trailing slash on either address is removed once, here, rather than
 * by every caller that concatenates a path onto it: the alternative is a
 * `canonical` with a double slash in it, which a crawler reads as a
 * different address than the one every link on the page points at.
 */
test("an address keeps no trailing slash whichever way it was written", () => {
  const config = siteConfig({
    [ENV_NAMES.origin]: "https://example.test///",
    [ENV_NAMES.umamiHostUrl]: "https://metrics.example.test//",
  });
  assert.equal(config.origin, "https://example.test");
  assert.equal(config.umamiHostUrl, "https://metrics.example.test");
});

test("each of the three themes arrives as itself", () => {
  for (const theme of ["system", "light", "dark"]) {
    assert.equal(
      siteConfig({ [ENV_NAMES.defaultTheme]: theme }).defaultTheme,
      theme,
    );
  }
});

/**
 * A word this build does not know is the site's own default rather than
 * a refusal. The configuration is read and refused where the file and
 * its line are still in hand; by the time a value has travelled through
 * an environment variable there is nothing useful left to say about it,
 * and stopping a deploy over which of two correct palettes a first-time
 * reader sees would be the wrong trade.
 *
 * `system` stays reachable — by asking for it. What it is no longer is
 * the answer to a question nobody asked.
 */
test("a theme nobody defined is the site's own", () => {
  assert.equal(
    siteConfig({ [ENV_NAMES.defaultTheme]: "sepia" }).defaultTheme,
    "dark",
  );
  assert.equal(
    siteConfig({ [ENV_NAMES.defaultTheme]: "  " }).defaultTheme,
    "dark",
  );
  assert.equal(
    siteConfig({ [ENV_NAMES.defaultTheme]: 7 }).defaultTheme,
    "dark",
  );
  assert.equal(
    siteConfig({ [ENV_NAMES.defaultTheme]: "system" }).defaultTheme,
    "system",
  );
});
