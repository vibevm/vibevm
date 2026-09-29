/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * Every mention of a person leads to the person.
 *
 * «Every mention of Oleg Chirukhin must lead to his site, oleg.guru»
 * (owner, 2026-09-26). The site names him in several places and they are
 * not one component: the footer of every page, a bridge's maintainer on
 * the catalogue, a card on the channels page, the author in the essay's
 * structured data. So this walks the pages and asks the question of each
 * mention it finds rather than of the places somebody remembered.
 *
 * What is passed over is what a reader does not read as prose: a `<pre>`
 * or a `<code>` where the name is a VALUE somebody may copy into their own
 * configuration, and the machine surfaces in `<script>`. A link inside a
 * data example would be a link inside somebody's file.
 */

import { expect, test } from "@playwright/test";

import { CREATOR_SITE } from "../src/lib/people.ts";

/** The name in both spellings, as every page of the site may print it. */
const NAMES = ["Oleg Chirukhin", "Олег Чирухин"] as const;

/**
 * Where the name is looked for: the front door in both languages, the
 * channels page in both, the catalogue, a page of the manual in both, the
 * essay and a Why page. Between them they cover every layout the site
 * has a footer in.
 */
const PAGES = [
  "/",
  "/ru/",
  "/news-and-support/",
  "/ru/news-and-support/",
  "/doc/",
  "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
  "/vision/",
  "/why/ai-native/",
] as const;

for (const at of PAGES) {
  test(`${at} leads from his name to his site`, async ({ page }) => {
    await page.goto(at);
    await expect(page.locator("html")).toHaveAttribute("data-site-lang", /./);

    const mentions = await page.evaluate((names) => {
      const walker = document.createTreeWalker(
        document.body,
        NodeFilter.SHOW_TEXT,
      );
      const found: { text: string; href: string | null; where: string }[] = [];
      for (
        let node = walker.nextNode();
        node !== null;
        node = walker.nextNode()
      ) {
        const text = node.textContent ?? "";
        if (!names.some((name) => text.includes(name))) continue;
        const owner = node.parentElement;
        if (owner === null) continue;
        /* Code, data and the machine surfaces are not prose. */
        if (owner.closest("pre, code, script, style, template") !== null) {
          continue;
        }
        const link = owner.closest("a");
        found.push({
          text: text.trim(),
          href: link === null ? null : link.getAttribute("href"),
          where: owner.className,
        });
      }
      return found;
    }, NAMES);

    /* At least the footer's signature is on every one of these pages. */
    expect(mentions.length).toBeGreaterThan(0);
    for (const mention of mentions) {
      expect(mention.href, `${mention.where}: ${mention.text}`).toBe(
        CREATOR_SITE,
      );
    }
  });
}

/**
 * The footer's own signature, in both languages: the year stays text and
 * the name is the link — «© 2026 » leads nowhere and never did.
 */
const FOOTERS = [
  { route: "/", name: "Oleg Chirukhin" },
  { route: "/ru/", name: "Олег Чирухин" },
] as const;

for (const one of FOOTERS) {
  test(`${one.route} signs itself with a name that leads to him`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const line = page.locator("footer .footer__copyright");
    await expect(line).toContainText("© 2026");
    await expect(line).toContainText(one.name);

    const link = line.locator("a.footer__person");
    await expect(link).toHaveText(one.name);
    await expect(link).toHaveAttribute("href", CREATOR_SITE);
  });
}

/**
 * And on a page of the manual, whose footer is prerendered in English and
 * moved into the reader's own language in their browser: the name moves
 * with it and keeps leading to the same place.
 */
test("the manual's footer keeps the link in either language", async ({
  page,
}) => {
  const at = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
  await page.goto(at);
  const link = page.locator("footer a.footer__person");
  await expect(link).toHaveText("Oleg Chirukhin");
  await expect(link).toHaveAttribute("href", CREATOR_SITE);

  await page
    .locator('[data-site-language] [data-site-lang-choice="ru"]')
    .click();
  await expect(link).toHaveText("Олег Чирухин");
  await expect(link).toHaveAttribute("href", CREATOR_SITE);
});

/**
 * The channels page carries him twice now, and neither card lies about
 * where it goes: the card IS the link, so his name could not lead to him
 * while it led to an account on X.
 */
const CREATOR_CARDS = [
  {
    route: "/news-and-support/",
    name: "Oleg Chirukhin",
    platform: "Website",
    handle: "@1red2black",
    posts: "Posts by the creator of VibeVM.",
  },
  {
    route: "/ru/news-and-support/",
    name: "Олег Чирухин",
    platform: "Сайт",
    handle: "@1red2black",
    posts: "Записи создателя VibeVM.",
  },
] as const;

for (const one of CREATOR_CARDS) {
  test(`${one.route} gives his site a card of its own`, async ({ page }) => {
    await page.goto(one.route);
    const site = page.locator("main .ns-card", { hasText: one.name });
    await expect(site).toHaveCount(1);
    await expect(site).toHaveAttribute("href", CREATOR_SITE);
    await expect(site.locator(".ns-card__platform")).toHaveText(one.platform);
    await expect(site.locator(".ns-card__at")).toHaveText("oleg.guru");

    const posts = page.locator("main .ns-card", { hasText: one.handle });
    await expect(posts).toHaveCount(1);
    await expect(posts).toHaveAttribute("href", "https://x.com/1red2black");
    await expect(posts).toContainText(one.posts);
  });
}

/** And the essay's author is a person a graph can resolve. */
test("the essay's structured data gives the author his address", async ({
  page,
}) => {
  await page.goto("/vision/");
  const graph = await page
    .locator('script[type="application/ld+json"]')
    .first()
    .textContent();
  const parsed = JSON.parse(graph ?? "{}") as {
    author?: { name?: string; url?: string };
  };
  expect(parsed.author?.name).toBe("Oleg Chirukhin");
  expect(parsed.author?.url).toBe(CREATOR_SITE);
});
