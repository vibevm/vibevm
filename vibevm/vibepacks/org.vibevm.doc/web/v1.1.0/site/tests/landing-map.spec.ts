/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The map at the foot of the landing, in both languages: that it lists
 * exactly what the header lists, in the header's order, leading where the
 * header leads; that it stands under the owner's content and over the
 * small print; that every plate is one link with a visible ring and one
 * described drawing; that Zap is announced and not installed; that the
 * page never scrolls sideways; and that a reader who asked for stillness
 * gets every drawing finished and still, while a reader who did not gets
 * the drawings.
 *
 * The first of those is the one this file exists for. The map and the
 * header are two renderings of one list (`landing/menu.ts`), and the
 * only way to prove that from outside is to read both off the built page
 * and compare them — a test that imported the list would prove the page
 * consistent with itself and nothing else. All of it in a browser over
 * the BUILT bytes, like every other run in this directory.
 */

import { expect, type Page, test } from "@playwright/test";

/** The two editions, and the words the map says in each. */
const PAGES = [
  {
    route: "/",
    title: "Everything here, on one plane.",
    rows: ["The software", "The argument"],
    zap: "October 2026",
    roadmap: { name: "Roadmap", href: "/roadmap/" },
    legal: {
      name: "Legal",
      href: "/doc/org.vibevm.core/vibevm-docs/latest/legal/applicable-law/",
    },
  },
  {
    route: "/ru/",
    title: "Весь сайт на одной плоскости.",
    rows: ["Софт", "Смысл"],
    zap: "октябрь 2026",
    roadmap: { name: "Роадмап", href: "/ru/roadmap/" },
    legal: {
      name: "Право",
      href: "/doc/ru/org.vibevm.core/vibevm-docs/latest/legal/applicable-law/",
    },
  },
] as const;

/**
 * The map's plates, as the header's rows declare them: ten, in two bands
 * of five and five — the roadmap right after the essay (owner,
 * 2026-09-26) and the law page right after the channels (owner,
 * 2026-09-27). The deploy smoke counts these.
 */
const PLATES = 10;
const BANDS = [5, 5] as const;

const WIDTHS = [1440, 834, 390] as const;

type Destination = {
  readonly label: string;
  readonly href: string;
  readonly rel: string | null;
};

/** The header's destinations, in reading order, as the page renders them. */
async function headerDestinations(page: Page): Promise<Destination[]> {
  return page
    .locator("header .landing-nav__row .landing-nav__link")
    .evaluateAll((links) =>
      links.map((link) => ({
        label: (link.textContent ?? "").trim(),
        href: link.getAttribute("href") ?? "",
        rel: link.getAttribute("rel"),
      })),
    );
}

/** The map's plates, in document order, by the name each one prints. */
async function mapDestinations(page: Page): Promise<Destination[]> {
  return page
    .locator("main .landing-map a.landing-map__plate")
    .evaluateAll((links) =>
      links.map((link) => ({
        label: (
          link.querySelector(".landing-map__name")?.textContent ?? ""
        ).trim(),
        href: link.getAttribute("href") ?? "",
        rel: link.getAttribute("rel"),
      })),
    );
}

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

/**
 * The drawings that have not finished appearing: every piece of a map
 * drawing that has a size and is still at the opacity it starts from.
 */
async function unrevealed(page: Page): Promise<number> {
  return page.evaluate(
    () =>
      [...document.querySelectorAll("main .landing-map svg *")].filter(
        (element) => {
          const box = element.getBoundingClientRect();
          if (box.width < 2 || box.height < 2) return false;
          return Number(getComputedStyle(element).opacity) === 0;
        },
      ).length,
  );
}

for (const one of PAGES) {
  test(`${one.route} maps every header destination, in the header's order`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    const header = await headerDestinations(page);
    const map = await mapDestinations(page);
    expect(header.length).toBe(PLATES);
    expect(map).toEqual(header);

    /* Two rows in the header, two bands on the map, each band as long as
       its row — so the map keeps not only the list but its shape. */
    const rows = await page
      .locator("header .landing-nav__row")
      .evaluateAll((all) =>
        all.map((row) => row.querySelectorAll(".landing-nav__link").length),
      );
    const bands = await page
      .locator("main .landing-map .landing-map__grid")
      .evaluateAll((all) =>
        all.map((grid) => grid.querySelectorAll(".landing-map__plate").length),
      );
    expect(bands).toEqual(rows);
    expect(bands).toEqual([...BANDS]);
    await expect(
      page.locator("main .landing-map .landing-map__row-k"),
    ).toHaveText([...one.rows]);
    await expect(page.locator("main .landing-map h2")).toHaveText(one.title);
  });
}

/**
 * The roadmap's plate: right after the essay's in the argument's band, a
 * band like the essay's and its mirror image — the essay's drawing stands
 * on the right, the roadmap's on the left, so the two long plates read as
 * one Z rather than as a repeat. On a phone both stack their drawing over
 * their words and the mirroring folds away.
 */
for (const one of PAGES) {
  test(`${one.route} stands the roadmap's plate under the essay's, mirrored`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    const story = page.locator(
      "main .landing-map .landing-map__band:nth-child(3) .landing-map__plate",
    );
    await expect(story.nth(0)).toHaveClass(/\blanding-map__plate--vision\b/);
    await expect(story.nth(1)).toHaveClass(/\blanding-map__plate--roadmap\b/);

    const plate = story.nth(1);
    await expect(plate).toHaveClass(/\blanding-map__plate--band\b/);
    await expect(plate).toHaveAttribute("href", one.roadmap.href);
    await expect(plate.locator(".landing-map__name")).toHaveText(
      one.roadmap.name,
    );
    await expect(plate.locator(".landing-map__body")).not.toContainText(
      /\b(19|20)\d\d\b/,
    );

    const sides = async (which: number) =>
      story.nth(which).evaluate((element) => {
        const art = element.querySelector(".landing-map__art");
        const text = element.querySelector(".landing-map__text");
        if (art === null || text === null) return null;
        return art.getBoundingClientRect().left <
          text.getBoundingClientRect().left
          ? "art-left"
          : "text-left";
      });
    expect(await sides(0)).toBe("text-left");
    expect(await sides(1)).toBe("art-left");

    const essayBottom = await story
      .nth(0)
      .evaluate((element) => element.getBoundingClientRect().bottom);
    const roadmapTop = await plate.evaluate(
      (element) => element.getBoundingClientRect().top,
    );
    expect(roadmapTop).toBeGreaterThan(essayBottom);

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(one.route);
    const stacked = await plate.evaluate((element) => {
      const art = element.querySelector(".landing-map__art");
      const text = element.querySelector(".landing-map__text");
      if (art === null || text === null) return false;
      return (
        art.getBoundingClientRect().bottom <=
        text.getBoundingClientRect().top + 1
      );
    });
    expect(stacked).toBe(true);
  });
}

/**
 * The law page's plate: last in the software's band, closing its line the
 * way the essay's opens the band below — one plate across the whole row,
 * with its drawing beside the words and a drawing of its own to carry.
 *
 * Its ADDRESS is what is held here and not the page behind it: the page is
 * written on another branch and lands after this, and the link lint reads
 * a `/doc/` address into a package this build does not carry as a citation
 * rather than a miss. What a test can say now is that the plate leads to
 * the right coordinate, in the reader's own edition.
 */
for (const one of PAGES) {
  test(`${one.route} closes the software's band with the law page`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    const tools = page.locator(
      "main .landing-map .landing-map__band:nth-child(2) .landing-map__plate",
    );
    await expect(tools).toHaveCount(5);
    const plate = tools.nth(4);
    await expect(plate).toHaveClass(/\blanding-map__plate--legal\b/);
    await expect(plate).toHaveClass(/\blanding-map__plate--band\b/);
    await expect(plate).toHaveAttribute("href", one.legal.href);
    await expect(plate.locator(".landing-map__name")).toHaveText(
      one.legal.name,
    );

    /* The balance, described once on the frame and drawn inside it. */
    const art = plate.locator('[role="img"][aria-label]');
    await expect(art).toHaveCount(1);
    await expect(art.locator("svg.lm-legal")).toHaveCount(1);
    await expect(art.locator("svg.lm-legal .lm-legal__beam")).toHaveCount(1);

    /* Level, centred on the fulcrum, the two weights at equal distance —
       the three things the drawing's own description claims, asked of the
       drawing rather than looked at. A middle survives the reveal: the
       pop scales each shape about its centre, so these are the same
       numbers before and after it. */
    const balance = await plate.locator("svg.lm-legal").evaluate((svg) => {
      const ends = (selector: string): readonly [number, number] => {
        const found = svg.querySelector(selector);
        return [
          Number(found?.getAttribute("y1") ?? Number.NaN),
          Number(found?.getAttribute("y2") ?? Number.NaN),
        ];
      };
      const middle = (selector: string): number => {
        const found = svg.querySelector(selector);
        if (found === null) return Number.NaN;
        const box = found.getBoundingClientRect();
        return box.x + box.width / 2;
      };
      const [from, to] = ends(".lm-legal__beam");
      return {
        from,
        to,
        beam: middle(".lm-legal__beam"),
        fulcrum: middle(".lm-legal__fulcrum"),
        weight: middle(".lm-legal__weight"),
        pan: middle(".lm-legal__pan"),
      };
    });
    expect(balance.from, "the beam is level").toBe(balance.to);
    expect(
      Math.abs(balance.beam - balance.fulcrum),
      "the beam is centred on the fulcrum",
    ).toBeLessThan(1);
    expect(
      Math.abs(
        balance.fulcrum - balance.weight - (balance.pan - balance.fulcrum),
      ),
      "the two weights hang at equal distance",
    ).toBeLessThan(1);
  });
}

for (const one of PAGES) {
  test(`${one.route} keeps the map under the content and the small print last`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);

    /* The order of the page's own content, read off the document: the
       hero, the three cards, the map, the four notes of small print.
       Nothing the owner placed moved, and the map came after all of it.
       The ground behind the page is decorative, says so, and is not
       content. */
    const order = await page.evaluate(() =>
      [...document.querySelectorAll("main > *")]
        .filter((element) => element.getAttribute("aria-hidden") !== "true")
        .filter((element) => element.getBoundingClientRect().height > 0)
        .map((element) => element.className.split(/\s+/)[0]),
    );
    expect(order).toEqual([
      "hero",
      "capability-row",
      "landing-map",
      /* Proper-name provenance, then one paragraph per namesake; all four
         are still the last of the page's content. */
      "landing-disambiguation",
      "landing-disambiguation",
      "landing-disambiguation",
      "landing-disambiguation",
    ]);

    const cards = await page
      .locator("main .capability-row")
      .evaluate((element) => element.getBoundingClientRect().bottom);
    const map = await page
      .locator("main .landing-map")
      .evaluate((element) => element.getBoundingClientRect().top);
    expect(map).toBeGreaterThan(cards);
  });
}

for (const one of PAGES) {
  test(`${one.route} names each plate once and hides its drawing`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const plates = page.locator("main .landing-map a.landing-map__plate");
    const count = await plates.count();
    expect(count).toBeGreaterThan(0);

    /* One described picture per plate, and no svg announced on its own. */
    await expect(
      page.locator('main .landing-map [role="img"][aria-label]'),
    ).toHaveCount(count);
    const exposed = await page.evaluate(
      () =>
        [...document.querySelectorAll("main .landing-map svg")].filter(
          (svg) => svg.getAttribute("aria-hidden") !== "true",
        ).length,
    );
    expect(exposed).toBe(0);

    /* The link's name is the destination's name: `aria-labelledby` points
       at the heading inside it, and that heading exists. */
    const broken = await plates.evaluateAll(
      (links) =>
        links.filter((link) => {
          const id = link.getAttribute("aria-labelledby") ?? "";
          const named = document.getElementById(id);
          return named === null || !link.contains(named);
        }).length,
    );
    expect(broken).toBe(0);

    /* The whole plate is the link, and the only one in it. */
    await expect(page.locator("main .landing-map a")).toHaveCount(count);
  });
}

for (const one of PAGES) {
  test(`${one.route} gives a plate a visible focus ring`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const plate = page
      .locator("main .landing-map a.landing-map__plate")
      .first();
    await plate.focus();
    await expect(plate).toBeFocused();
    const outline = await plate.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

/* Zap is announced for October 2026 and is not working software (owner,
   2026-09-25): its plate says when, and nothing on the map tells a
   reader to install it. */
for (const one of PAGES) {
  test(`${one.route} announces Zap on the map and does not install it`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const zap = page.locator("main .landing-map .landing-map__plate--why-zap");
    await expect(zap).toHaveCount(1);
    await expect(zap).toContainText(one.zap);
    const text = await page.locator("main .landing-map").innerText();
    expect(text).not.toMatch(/vibe install|zap-quicklens|install Zap/i);
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
 * A reader who asked for stillness gets it: nothing on the page is
 * animating, and every drawing on the map is already whole — before the
 * reader has scrolled anywhere near it, because a drawing that appears on
 * scrolling is motion too.
 */
for (const one of PAGES) {
  test(`${one.route} stands still for a reader who asked`, async ({
    browser,
  }) => {
    const context = await browser.newContext({ reducedMotion: "reduce" });
    const page = await context.newPage();
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);

    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    expect(await unrevealed(page)).toBe(0);

    await context.close();
  });
}

/**
 * And a reader who did not ask gets the drawings — as they arrive. The
 * reveal is driven by the plate's own entry into the window where the
 * browser can do that (this one can), so with the map below the fold the
 * drawings are still waiting, and scrolling to the foot of the map, which
 * is what a reader does to see it, brings every plate through its entry
 * and leaves every drawing whole. The first assertion is what proves the
 * reveal is scroll-driven and not a load-time animation that happened to
 * finish: it pins the design, not the fallback.
 */
for (const one of PAGES) {
  test(`${one.route} draws the map for a reader who scrolls to it`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);
    expect(
      await page.evaluate(() => document.getAnimations().length),
    ).toBeGreaterThan(0);
    await page.waitForTimeout(1500);
    expect(await unrevealed(page)).toBeGreaterThan(0);

    await page.evaluate(() =>
      window.scrollTo(0, document.documentElement.scrollHeight),
    );
    await expect.poll(() => unrevealed(page), { timeout: 4000 }).toBe(0);
  });
}
