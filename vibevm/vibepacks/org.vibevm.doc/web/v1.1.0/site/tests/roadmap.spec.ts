/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The roadmap, in both editions: that it is served; that the six stages
 * stand in the owner's order under the marker that says where the project
 * is today; that the two stages with somewhere to go lead there — Zap to
 * its own page in the reader's language, Spec-Driven Linux to its future
 * home — and that the second of those is asserted by its address alone,
 * never fetched (the domain has no records yet); that the page names no
 * date; that it says the same order to a crawler as an `ItemList`; that
 * the header, the footer and the home map all lead to it; that the
 * machine files at the root carry it; that it never scrolls sideways on a
 * phone; and that a reader who asked for stillness gets it.
 *
 * Everything is measured in a browser over the BUILT bytes, like every
 * other run in this directory.
 */

import { expect, type Page, test } from "@playwright/test";

/** The two editions, and what each must say. */
const PAGES = [
  {
    route: "/roadmap/",
    twin: "/ru/roadmap/",
    language: "en",
    heading: "From a preview to an operating system.",
    title: "Roadmap — from a preview to an operating system",
    nav: "Roadmap",
    here: "Developer Preview 1",
    stages: [
      "Developer Preview 2",
      "Zap",
      "The first community packages",
      "IDE plugins",
      "A library and marketplace",
      "Spec-Driven Linux",
    ],
    zap: "/why/zap/",
  },
  {
    route: "/ru/roadmap/",
    twin: "/roadmap/",
    language: "ru",
    heading: "От превью до операционной системы.",
    title: "Дорожная карта — от превью до операционной системы",
    nav: "Роадмап",
    here: "Developer Preview 1",
    stages: [
      "Developer Preview 2",
      "Zap",
      "Первые пакеты от сообщества",
      "Плагины для IDE",
      "Библиотека и маркетплейс",
      "Spec-Driven Linux",
    ],
    zap: "/ru/why/zap/",
  },
] as const;

/** The two landings, and how each names the page. */
const HOMES = [
  { route: "/", nav: "Roadmap", href: "/roadmap/" },
  { route: "/ru/", nav: "Роадмап", href: "/ru/roadmap/" },
] as const;

/** The future home of the sixth stage — asserted as an address, only. */
const LINUX_HOME = "https://specdrivenlinux.org/";

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
 * The order is the content. The marker stands first and is not numbered;
 * the six stages follow it in the owner's order, in an ordered list, each
 * with a name, one sentence and a paragraph.
 */
for (const one of PAGES) {
  test(`${one.route} walks the six stages in the owner's order`, async ({
    page,
  }) => {
    await page.goto(one.route);

    await expect(page.locator("main .rm-here__name")).toHaveText(one.here);

    const list = page.locator("main ol.rm-stages");
    await expect(list).toHaveCount(1);
    await expect(list.locator("> li")).toHaveCount(one.stages.length);
    await expect(list.locator("> li .rm-stage__name")).toHaveText([
      ...one.stages,
    ]);
    await expect(list.locator("> li .rm-stage__n")).toHaveText([
      "01",
      "02",
      "03",
      "04",
      "05",
      "06",
    ]);

    /* Each stage has its three parts, none of them empty. */
    const parts = await list.locator("> li").evaluateAll((items) =>
      items.map((item) => ({
        sub: item.querySelector(".rm-stage__sub")?.textContent?.trim() ?? "",
        body: item.querySelector(".rm-stage__body")?.textContent?.trim() ?? "",
      })),
    );
    for (const part of parts) {
      expect(part.sub.length).toBeGreaterThan(10);
      expect(part.body.length).toBeGreaterThan(40);
    }

    /* The marker precedes the list on the page as well as in the tree. */
    const marker = await page
      .locator("main .rm-here")
      .evaluate((element) => element.getBoundingClientRect().top);
    const first = await list
      .locator("> li")
      .first()
      .evaluate((element) => element.getBoundingClientRect().top);
    expect(marker).toBeLessThan(first);
  });
}

/**
 * The two stages with somewhere to go. Zap's link is to its own page in
 * the reader's language; the sixth stage's link is to the owner's domain,
 * which is asserted as an address and never followed — there is nothing
 * behind it to fetch yet, and a test that fetched it would fail for the
 * wrong reason.
 */
for (const one of PAGES) {
  test(`${one.route} leads from Zap to its page and names the sixth stage's home`, async ({
    page,
  }) => {
    await page.goto(one.route);

    const zap = page.locator("main .rm-stage--zap .rm-stage__home a");
    await expect(zap).toHaveCount(1);
    await expect(zap).toHaveAttribute("href", one.zap);

    const linux = page.locator("main .rm-stage--linux .rm-stage__home a");
    await expect(linux).toHaveCount(1);
    await expect(linux).toHaveAttribute("href", LINUX_HOME);
    await expect(linux).toHaveAttribute("rel", "noopener");
    await expect(linux).toHaveText("specdrivenlinux.org");

    /* No other stage leads anywhere. */
    await expect(page.locator("main .rm-stages .rm-stage__home")).toHaveCount(
      2,
    );

    await zap.click();
    await page.waitForURL(`**${one.zap}`);
    expect(new URL(page.url()).pathname).toBe(one.zap);
  });
}

/**
 * The page keeps its own rule: no dates. Not a year anywhere in the
 * page's content, and nothing that tells a reader to install a stage.
 */
for (const one of PAGES) {
  test(`${one.route} names no date and installs nothing`, async ({ page }) => {
    await page.goto(one.route);
    const text = await page.locator("main").innerText();
    expect(text).not.toMatch(/\b(19|20)\d\d\b/);
    expect(text).not.toMatch(/vibe install/);
  });
}

/**
 * What each edition tells a crawler: its own address, its twin, and a
 * `WebPage` whose main entity is the ordered list of stages — the same
 * names in the same order the page prints, with the two addresses the
 * page links to.
 */
for (const one of PAGES) {
  test(`${one.route} declares its stages to a crawler, in order`, async ({
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

    const description = await page.getAttribute(
      'meta[name="description"]',
      "content",
    );
    expect(description).toBeTruthy();
    expect(description).not.toMatch(/\b(19|20)\d\d\b/);

    const graph = await page
      .locator('script[type="application/ld+json"]')
      .first()
      .textContent();
    const parsed = JSON.parse(graph ?? "{}");
    expect(parsed["@type"]).toBe("WebPage");
    expect(parsed.url).toBe(`https://vibevm.org${one.route}`);
    expect(parsed.inLanguage).toBe(one.language);
    expect(parsed.isPartOf?.name).toBe("Anarchic VibeVM");

    const list = parsed.mainEntity;
    expect(list?.["@type"]).toBe("ItemList");
    expect(list.itemListOrder).toBe(
      "https://schema.org/ItemListOrderAscending",
    );
    expect(list.numberOfItems).toBe(one.stages.length);
    const items = list.itemListElement as {
      "@type": string;
      position: number;
      name: string;
      description: string;
      url?: string;
    }[];
    expect(items.map((item) => item.name)).toEqual([...one.stages]);
    expect(items.map((item) => item.position)).toEqual([1, 2, 3, 4, 5, 6]);
    for (const item of items) {
      expect(item["@type"]).toBe("ListItem");
      expect(item.description.length).toBeGreaterThan(10);
    }
    expect(items[1].url).toBe(`https://vibevm.org${one.zap}`);
    expect(items[5].url).toBe(LINUX_HOME);
    expect(items.filter((item) => item.url !== undefined)).toHaveLength(2);
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

/**
 * The three ways in from the landing that are this page's own: the
 * header's second row, right after the essay; the footer, right after
 * the essay; and the home map, whose plate stands right under the
 * essay's and leads to the same address. (The hero badge is the third
 * the owner asked for and is measured where it is written.)
 */
for (const one of HOMES) {
  test(`${one.route} leads to the roadmap from the header, the footer and the map`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    const story = page.locator(
      "header .landing-nav__row--story .landing-nav__link",
    );
    const labels = await story.evaluateAll((links) =>
      links.map((link) => (link.textContent ?? "").trim()),
    );
    expect(labels[1]).toBe(one.nav);
    await expect(story.nth(1)).toHaveAttribute("href", one.href);

    const footer = page.locator(`footer a[href="${one.href}"]`);
    await expect(footer).toHaveCount(1);
    await expect(footer).toHaveText(one.nav);
    const footerOrder = await page
      .locator(".landing-footer__links a")
      .evaluateAll((links) => links.map((link) => link.getAttribute("href")));
    const essay = footerOrder.findIndex((at) => at?.endsWith("vision/"));
    expect(footerOrder[essay + 1]).toBe(one.href);

    const plate = page.locator(
      "main .landing-map a.landing-map__plate--roadmap",
    );
    await expect(plate).toHaveCount(1);
    await expect(plate).toHaveAttribute("href", one.href);
    await expect(plate.locator(".landing-map__name")).toHaveText(one.nav);
    await expect(plate.locator('[role="img"][aria-label]')).toHaveCount(1);

    await plate.click();
    await page.waitForURL(`**${one.href}`);
    expect(new URL(page.url()).pathname).toBe(one.href);
  });
}

/**
 * The machine files at the root know the page: the short index names it
 * under «## Project» right after the essay, the full text carries both
 * editions, and the sitemap lists both addresses.
 */
test("the root machine files carry the roadmap", async ({ request }) => {
  const llms = await (await request.get("/llms.txt")).text();
  const lines = llms.split("\n");
  const entry = lines.findIndex((line) =>
    line.startsWith("- [Roadmap](https://vibevm.org/roadmap/): "),
  );
  expect(entry).toBeGreaterThan(0);
  expect(lines[entry - 1]).toMatch(/^- \[The Big Vision\]\(/);
  expect(lines[entry]).toContain("Developer Preview 2");
  expect(lines[entry]).toContain("Spec-Driven Linux");
  expect(lines[entry]).toContain("specdrivenlinux.org");

  const full = await (await request.get("/llms-full.txt")).text();
  expect(full).toContain("## URL: https://vibevm.org/roadmap/");
  expect(full).toContain("## URL: https://vibevm.org/ru/roadmap/");
  expect(full).toContain("Spec-Driven Linux");

  const sitemap = await (await request.get("/sitemap.xml")).text();
  expect(sitemap).toContain("<loc>https://vibevm.org/roadmap/</loc>");
  expect(sitemap).toContain("<loc>https://vibevm.org/ru/roadmap/</loc>");
});

for (const one of PAGES) {
  for (const width of WIDTHS) {
    test(`${one.route} does not scroll sideways at ${width}px`, async ({
      page,
    }) => {
      await page.setViewportSize({
        width,
        height: width === 390 ? 844 : 900,
      });
      await page.goto(one.route);
      await page.evaluate(() => document.fonts.ready);
      expect(await scrolls(page)).toBe(false);

      /* And the page is not clipping an overflow it merely hides: every
         stage's words stand inside the window. */
      const outside = await page.locator("main .rm-stage__text").evaluateAll(
        (all) =>
          all.filter((element) => {
            const box = element.getBoundingClientRect();
            return box.right > window.innerWidth + 1 || box.left < -1;
          }).length,
      );
      expect(outside).toBe(0);
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
       visible — before the reader has scrolled anywhere, because a
       drawing that appears on scrolling is motion too. */
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
  test(`${one.route} gives its first link a visible focus ring`, async ({
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

/**
 * The drawings are described once each: the hero, the marker and the six
 * emblems carry their words on the figure, and no `svg` is announced on
 * its own.
 */
for (const one of PAGES) {
  test(`${one.route} describes each drawing once`, async ({ page }) => {
    await page.goto(one.route);
    const figures = page.locator('main [role="img"][aria-label]');
    expect(await figures.count()).toBe(8);
    const exposed = await page.evaluate(
      () =>
        [...document.querySelectorAll("main svg")].filter(
          (svg) => svg.getAttribute("aria-hidden") !== "true",
        ).length,
    );
    expect(exposed).toBe(0);
  });
}
