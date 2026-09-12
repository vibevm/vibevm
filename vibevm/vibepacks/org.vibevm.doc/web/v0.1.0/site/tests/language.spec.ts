/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

/**
 * A language a page has not reached yet, and what a reader gets instead.
 *
 * The promise is that an adaptation never 404s and never silently pulls
 * a reader back into the source language. Both halves are measured here
 * over the built site, because both are about what is actually on disk:
 * a page that exists at an address the adaptation does not carry, and
 * links inside it that were rendered from the source's document.
 */

import { expect, test } from "@playwright/test";

const ADAPTED =
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const MISSING =
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/reference/addresses/";
const SOURCE =
  "/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/";

/**
 * The reader's memory is cleared from a PAGE and never from the door.
 *
 * The door takes its language decision once per session, and taking it
 * is itself a thing it remembers — so clearing the session on the door
 * races the decision it has just started making, and the next test finds
 * the choice already used up.
 */
test.beforeEach(async ({ page }) => {
  await page.goto(SOURCE);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
    document.cookie = "lang=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/";
  });
});

test("a page the adaptation does not carry is served, not refused", async ({
  page,
}) => {
  const response = await page.goto(MISSING);
  expect(response?.status()).toBe(200);
  await expect(page.locator("[data-island]")).toBeVisible();
});

test("the fallback page says it is one, in both languages, once", async ({
  page,
}) => {
  await page.goto(MISSING);
  const notice = page.locator("[data-fallback-notice]");
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("не переведена");
  await expect(notice).toContainText("source language");

  await page.locator("[data-fallback-dismiss]").click();
  await expect(notice).toBeHidden();

  // Once per session: the next fallback page in the same session is silent.
  await page.goto(MISSING);
  await expect(page.locator("[data-fallback-notice]")).toBeHidden();
});

test("the fallback page points a crawler at the source and asks to be skipped", async ({
  page,
}) => {
  await page.goto(MISSING);
  await expect(page.locator("link[rel='canonical']")).toHaveAttribute(
    "href",
    SOURCE,
  );
  await expect(page.locator("meta[name='robots']")).toHaveAttribute(
    "content",
    "noindex",
  );

  // The adaptation's own page claims neither.
  await page.goto(ADAPTED);
  await expect(page.locator("meta[name='robots']")).toHaveCount(0);
});

test("links inside a fallback page keep the reader in their language", async ({
  page,
}) => {
  await page.goto(MISSING);
  const other = page.locator(".docs-nav a").first();
  await expect(other).toHaveAttribute("href", new RegExp("^/doc/ru/"));
});

test("an address with a language is remembered for the door", async ({
  page,
}) => {
  await page.goto(ADAPTED);
  await expect
    .poll(async () => page.evaluate(() => document.cookie), { timeout: 4000 })
    .toContain("lang=ru");

  await page.goto("/doc/");
  await expect(page).toHaveURL(new RegExp("/doc/ru/$"));
});

test("the door honours an explicit choice over the remembered one", async ({
  page,
}) => {
  await page.goto(ADAPTED);
  await expect
    .poll(async () => page.evaluate(() => document.cookie), { timeout: 4000 })
    .toContain("lang=ru");

  await page.locator("[data-language-selector] summary").click();
  await page.locator("[data-language-selector] a[hreflang='en']").click();
  await expect(page).toHaveURL(
    new RegExp("/doc/com\\.example\\.docs/fixture-manual/"),
  );

  await page.goto("/doc/");
  await expect(page).toHaveURL(new RegExp("/doc/$"));
});

/**
 * The documentation's sitemap is its own file, an index by package and
 * language (`##SEO-SITEMAP`), and it lists a page at the one address
 * that is canonical — `latest`, which every numbered address points its
 * `rel=canonical` at (`##SITE-CANONICAL-LATEST`).
 *
 * What is measured here has not changed: a page an adaptation does not
 * carry is `noindex`, and a sitemap that offered it anyway would spend a
 * crawler's fetch to be turned away.
 */
test("the fallback is not offered to a crawler in the sitemap", async ({
  request,
}) => {
  const index = await (await request.get("/doc/sitemap.xml")).text();
  const part = "/doc/sitemap/com.example.docs/fixture-manual/ru.xml";
  expect(index).toContain(`<loc>https://vibevm.org${part}</loc>`);

  const russian = await (await request.get(part)).text();
  const latest = (address: string) => address.replace("/0.1.0/", "/latest/");
  expect(russian).toContain(`<loc>https://vibevm.org${latest(ADAPTED)}</loc>`);
  expect(russian).not.toContain(
    `<loc>https://vibevm.org${latest(MISSING)}</loc>`,
  );

  // The source it points at is of course still offered.
  const english = await (
    await request.get("/doc/sitemap/com.example.docs/fixture-manual/en.xml")
  ).text();
  expect(english).toContain(`<loc>https://vibevm.org${latest(SOURCE)}</loc>`);

  // And no version-numbered address stands in any of them.
  expect(russian).not.toContain("/0.1.0/");
  expect(english).not.toContain("/0.1.0/");
});

test("each language has a catalogue of its own", async ({ page }) => {
  await page.goto("/doc/ru/");
  await expect(page.locator(".shelf__items .card").first()).toBeVisible();
  await expect(page.locator("[data-language-selector] summary")).toContainText(
    "ru",
  );
});
