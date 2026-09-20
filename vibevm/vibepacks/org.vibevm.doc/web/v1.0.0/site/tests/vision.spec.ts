/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The essay, in both editions: that it is served, that it says it is an
 * Article to a crawler, that the header names it and marks it current,
 * that the language switch keeps a reader on it, that a reader who
 * asked for stillness gets it — and that the doorway on the AI-Native
 * hero actually leads here.
 *
 * Everything is measured in a browser over the BUILT bytes, like every
 * other run in this directory: these are questions about a live
 * document, and a component rendered in isolation cannot answer them.
 */

import { expect, type Page, test } from "@playwright/test";

/** The two editions, and what each must say it is. */
const PAGES = [
  {
    route: "/vision/",
    twin: "/ru/vision/",
    language: "en",
    heading: "The Big Vision",
    title: "The Big Vision — an essay on the new technological order",
    nav: "Vision",
  },
  {
    route: "/ru/vision/",
    twin: "/vision/",
    language: "ru",
    heading: "Большой Вижен",
    title: "Большой Вижен — эссе о новом технологическом укладе",
    nav: "Видение",
  },
] as const;

const WIDTHS = [1440, 834, 390] as const;

async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

for (const one of PAGES) {
  test(`${one.route} is served and opens with its own headline`, async ({
    page,
  }) => {
    const response = await page.goto(one.route);
    expect(response?.status()).toBe(200);

    const heading = page.locator("h1");
    await expect(heading).toHaveCount(1);
    const text = (await heading.innerText()).replace(/\s+/g, " ").trim();
    expect(text).toBe(one.heading);

    await expect(page).toHaveTitle(one.title);
    expect(await page.getAttribute("html", "lang")).toBe(one.language);
  });
}

/**
 * What each edition tells a crawler about itself.
 *
 * The one departure from the Why pages is deliberate and asserted: the
 * structured data is an `Article` — an authored essay with a headline
 * and an author — rather than the generic `WebPage`. The author is the
 * person the domain's footer already names on every page.
 */
for (const one of PAGES) {
  test(`${one.route} declares itself an Article at its own address`, async ({
    page,
  }) => {
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
    expect(parsed["@type"]).toBe("Article");
    expect(parsed.headline).toBe(one.heading);
    expect(parsed.url).toBe(`https://vibevm.org${one.route}`);
    expect(parsed.mainEntityOfPage).toBe(`https://vibevm.org${one.route}`);
    expect(parsed.inLanguage).toBe(one.language);
    expect(parsed.author?.name).toBe("Oleg Chirukhin");
    expect(parsed.isPartOf?.name).toBe("VibeVM");
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

for (const one of PAGES) {
  test(`${one.route} stops its decoration for a reader who asked`, async ({
    browser,
  }) => {
    const context = await browser.newContext({ reducedMotion: "reduce" });
    const page = await context.newPage();
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);

    const running = await page.evaluate(() => document.getAnimations().length);
    expect(running).toBe(0);

    /* Every drawing's entry animation must have left its subject
       visible: the essay has no element that IS its motion. */
    const invisible = await page.evaluate(
      () =>
        [...document.querySelectorAll("main svg *")].filter((element) => {
          const box = element.getBoundingClientRect();
          if (box.width < 2 || box.height < 2) return false;
          return Number(getComputedStyle(element).opacity) === 0;
        }).length,
    );
    expect(invisible).toBe(0);

    await context.close();
  });
}

for (const one of PAGES) {
  test(`${one.route} animates when motion is allowed`, async ({ page }) => {
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);
    const count = await page.evaluate(() => document.getAnimations().length);
    expect(count).toBeGreaterThan(0);
  });
}

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
  });
}

for (const one of PAGES) {
  test(`${one.route} labels every section with an id that exists`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const broken = await page.evaluate(() =>
      [...document.querySelectorAll("main [aria-labelledby]")]
        .map((element) => element.getAttribute("aria-labelledby") ?? "")
        .filter((id) => document.getElementById(id) === null),
    );
    expect(broken).toEqual([]);
  });
}

for (const one of PAGES) {
  test(`${one.route} gives its way out a visible focus ring`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const action = page.locator("main a").first();
    await action.focus();
    await expect(action).toBeFocused();
    const outline = await action.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

for (const one of PAGES) {
  test(`${one.route} describes each drawing once`, async ({ page }) => {
    await page.goto(one.route);
    const figures = page.locator('main [role="img"][aria-label]');
    expect(await figures.count()).toBeGreaterThanOrEqual(4);
    const exposed = await page.evaluate(
      () =>
        [...document.querySelectorAll("main svg")].filter(
          (svg) => svg.getAttribute("aria-hidden") !== "true",
        ).length,
    );
    expect(exposed).toBe(0);
  });
}

for (const one of PAGES) {
  test(`${one.route} ships no trace of the previous framework`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const html = await page.content();
    expect(html).not.toContain("_astro");
    expect(html).not.toContain("astro-island");
    expect(html).not.toContain("data-astro");
  });
}

/**
 * The doorway: the AI-Native hero names the essay and leads to it, in
 * each language — an entry into a worldview, not another install
 * button, which is why what is asserted is the link and its address
 * rather than any button furniture.
 */
const DOORWAYS = [
  { from: "/why/ai-native/", to: "/vision/", head: "The Big Vision →" },
  { from: "/ru/why/ai-native/", to: "/ru/vision/", head: "Большой Вижен →" },
] as const;

for (const one of DOORWAYS) {
  test(`${one.from} carries the doorway into the essay`, async ({ page }) => {
    await page.goto(one.from);
    const doorway = page.locator(".an-vision-entry");
    await expect(doorway).toHaveCount(1);
    await expect(doorway).toHaveAttribute("href", one.to);
    await expect(doorway.locator(".an-vision-entry__head")).toHaveText(
      one.head,
    );

    await doorway.click();
    await page.waitForURL(`**${one.to}`);
    expect(new URL(page.url()).pathname).toBe(one.to);
  });
}
