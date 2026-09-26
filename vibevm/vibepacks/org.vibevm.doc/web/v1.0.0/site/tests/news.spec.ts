/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The channels page, in both languages: that it is served, that it says
 * who it is to a crawler, that the header's first row ends with it
 * wherever a reader stands and leads to the edition they are reading,
 * that the page marks itself current, that the two letters in the corner
 * keep a reader on it — and that the five cards lead to the five
 * addresses the owner named, each one a whole target with a ring around
 * it.
 *
 * The addresses are written out here rather than imported from the copy
 * table, and that is the point of this file: a test that read the same
 * constant the page renders would prove the page consistent with itself
 * and nothing else. A channel address is the one value on this site where
 * a typo is invisible — every card would still look right, and a reader
 * would land somewhere that is not the project — so it is asserted
 * against what the owner wrote, by hand, once.
 *
 * All of it in a browser over the BUILT bytes, like every other run in
 * this directory.
 */

import { expect, type Page, test } from "@playwright/test";

/** The two editions, and what each must say it is. */
const PAGES = [
  {
    route: "/news-and-support/",
    twin: "/ru/news-and-support/",
    language: "en",
    heading: "News & support",
    title: "News & support — VibeVM",
    nav: "News & support",
    groups: ["News", "Support", "Conversation"],
  },
  {
    route: "/ru/news-and-support/",
    twin: "/news-and-support/",
    language: "ru",
    heading: "Новости и поддержка",
    title: "Новости и поддержка — VibeVM",
    nav: "Новости и поддержка",
    groups: ["Новости", "Поддержка", "Разговоры"],
  },
] as const;

/**
 * The five destinations, in the order the page prints them, with the
 * address each card must show and the address it must lead to.
 *
 * Both, because they are two different promises: a card that led to the
 * right place while printing another is a lie a reader cannot see, and a
 * card that printed the right place while leading elsewhere is worse.
 */
const CHANNELS = [
  { href: "https://t.me/vibevm", shown: "t.me/vibevm" },
  { href: "https://x.com/1red2black", shown: "x.com/1red2black" },
  { href: "https://t.me/vibevm_chat", shown: "t.me/vibevm_chat" },
  { href: "https://www.reddit.com/r/vibevm/", shown: "reddit.com/r/vibevm" },
  { href: "https://t.me/chat_1red2black", shown: "t.me/chat_1red2black" },
] as const;

/** Where the header's first row is asked for its last entry. */
const ELSEWHERE = [
  { route: "/", locale: "en" },
  { route: "/ru/", locale: "ru" },
  { route: "/why/zap/", locale: "en" },
  { route: "/ru/why/zap/", locale: "ru" },
  { route: "/vision/", locale: "en" },
  { route: "/ru/vision/", locale: "ru" },
] as const;

const ENTRY = {
  en: { label: "News & support", href: "/news-and-support/" },
  ru: { label: "Новости и поддержка", href: "/ru/news-and-support/" },
} as const;

const WIDTHS = [1440, 834, 390] as const;

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

for (const one of PAGES) {
  test(`${one.route} is served and opens with its own heading`, async ({
    page,
  }) => {
    const response = await page.goto(one.route);
    expect(response?.status()).toBe(200);

    const heading = page.locator("h1");
    await expect(heading).toHaveCount(1);
    await expect(heading).toHaveText(one.heading);

    await expect(page).toHaveTitle(one.title);
    expect(await page.getAttribute("html", "lang")).toBe(one.language);

    /* The three groups, in the owner's order, as the page's only h2s. */
    await expect(page.locator("main h2")).toHaveText([...one.groups]);
  });
}

/**
 * What the page tells a crawler about itself: the same set of tags the
 * Why pages carry, with this address in them.
 */
for (const one of PAGES) {
  test(`${one.route} declares its own address`, async ({ page }) => {
    await page.goto(one.route);

    const canonical = await page.getAttribute("link[rel=canonical]", "href");
    expect(canonical).toBe(`https://vibevm.org${one.route}`);

    const alternates = await page
      .locator("link[rel=alternate][hreflang]")
      .evaluateAll((links) =>
        links
          .map(
            (link) =>
              `${link.getAttribute("hreflang")} ${link.getAttribute("href")}`,
          )
          .sort(),
      );
    const english = one.language === "en" ? one.route : one.twin;
    const russian = one.language === "ru" ? one.route : one.twin;
    expect(alternates).toEqual([
      `en https://vibevm.org${english}`,
      `ru https://vibevm.org${russian}`,
      `x-default https://vibevm.org${english}`,
    ]);

    const ogUrl = await page.getAttribute('meta[property="og:url"]', "content");
    expect(ogUrl).toBe(`https://vibevm.org${one.route}`);
    const description = await page.getAttribute(
      'meta[name="description"]',
      "content",
    );
    expect(description).toBeTruthy();

    const graph = await page
      .locator('script[type="application/ld+json"]')
      .first()
      .textContent();
    const parsed = JSON.parse(graph ?? "{}");
    expect(parsed["@type"]).toBe("WebPage");
    expect(parsed.url).toBe(`https://vibevm.org${one.route}`);
    expect(parsed.inLanguage).toBe(one.language);
    expect(parsed.isPartOf?.name).toBe("VibeVM");
  });
}

/**
 * The header's first row ends with this page, wherever a reader stands,
 * and leads to the edition they are reading.
 *
 * Asked on the landing, on a Why page and on the essay, in both
 * languages: the chrome is one component over every landing address, so
 * the entry is either last in that row everywhere or somewhere else on
 * one of them.
 */
for (const one of ELSEWHERE) {
  test(`${one.route} ends its first header row with the channels page`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    /* The first row is the first row: read off the document's order, so
       that «last in the first row» cannot be satisfied by an entry that
       is last in the second one. */
    const rows = page.locator("header .landing-nav__row");
    await expect(rows).toHaveCount(2);
    const first = rows.nth(0);
    await expect(first).toHaveClass(/\blanding-nav__row--tools\b/);

    /* Last, and in the language the page is written in: the label and the
       address are both the reader's edition's, which is what the entry
       being locale-aware means. */
    const entries = first.locator(".landing-nav__link");
    await expect(entries).toHaveCount(4);
    const last = entries.nth(3);
    await expect(last).toHaveText(ENTRY[one.locale].label);
    await expect(last).toHaveAttribute("href", ENTRY[one.locale].href);
  });
}

for (const one of PAGES) {
  test(`${one.route} marks itself current in the header`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const current = page.locator(
      'header .landing-nav__link[aria-current="page"]',
    );
    await expect(current).toHaveCount(1);
    await expect(current).toHaveText(one.nav);
    await expect(current).toHaveAttribute("href", one.route);
  });
}

for (const one of PAGES) {
  test(`${one.route} offers its twin, not the front door`, async ({ page }) => {
    await page.goto(one.route);
    const other = one.language === "en" ? "ru" : "en";
    const link = page.locator(
      `[data-site-language] [data-site-lang-choice="${other}"]`,
    );
    await expect(link).toHaveAttribute("href", one.twin);

    await link.click();
    await page.waitForURL(`**${one.twin}`);
    expect(new URL(page.url()).pathname).toBe(one.twin);
  });
}

/**
 * The five cards: the owner's five addresses, in his order, each leading
 * off the domain and saying so, and each printing where it goes.
 */
for (const one of PAGES) {
  test(`${one.route} carries the five channels the owner named`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const cards = page.locator("main .ns-card");
    await expect(cards).toHaveCount(CHANNELS.length);

    expect(
      await cards.evaluateAll((links) =>
        links.map((link) => link.getAttribute("href")),
      ),
    ).toEqual(CHANNELS.map((channel) => channel.href));

    expect(
      await cards.evaluateAll((links) =>
        links.map((link) => link.getAttribute("rel")),
      ),
    ).toEqual(CHANNELS.map(() => "noopener"));

    await expect(cards.locator(".ns-card__at")).toHaveText(
      CHANNELS.map((channel) => channel.shown),
    );

    /* The whole card is the link, and it is the only one: a second
       target inside it would give a reader two stops for one place and a
       ring that is not where they clicked. */
    await expect(page.locator("main a")).toHaveCount(CHANNELS.length);
  });
}

for (const one of PAGES) {
  test(`${one.route} gives a card a visible focus ring`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const card = page.locator("main .ns-card").first();
    await card.focus();
    await expect(card).toBeFocused();
    const outline = await card.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

for (const one of PAGES) {
  for (const width of WIDTHS) {
    test(`${one.route} does not scroll sideways at ${width}px`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(one.route);
      await page.evaluate(() => document.fonts.ready);
      expect(await scrolls(page)).toBe(false);
    });
  }
}

/**
 * The landmarks, the heading order, and every section announced by a name
 * that is on the page.
 */
for (const one of PAGES) {
  test(`${one.route} has one main and an unbroken heading order`, async ({
    page,
  }) => {
    await page.goto(one.route);
    await expect(page.locator("main")).toHaveCount(1);
    await expect(page.locator("header")).toHaveCount(1);
    await expect(page.locator("footer")).toHaveCount(1);

    const levels = await page
      .locator("main h1, main h2, main h3, main h4")
      .evaluateAll((headings) =>
        headings.map((heading) => Number(heading.tagName.slice(1))),
      );
    expect(levels[0]).toBe(1);
    expect(levels.filter((level) => level === 1)).toHaveLength(1);
    for (let i = 1; i < levels.length; i += 1) {
      expect(levels[i] - levels[i - 1]).toBeLessThanOrEqual(1);
    }

    const broken = await page.evaluate(() =>
      [...document.querySelectorAll("main [aria-labelledby]")]
        .map((element) => element.getAttribute("aria-labelledby") ?? "")
        .filter((id) => document.getElementById(id) === null),
    );
    expect(broken).toEqual([]);
  });
}
