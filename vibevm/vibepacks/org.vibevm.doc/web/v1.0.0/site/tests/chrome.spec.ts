/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The header's composition, measured rather than looked at.
 *
 * The bar used to be one wrapping list, and what a wrapping list does at
 * a width nobody measured is not a design decision — it is a report. The
 * report it filed was the theme switch, alone, on a line of its own
 * under the brand, outside a header one row tall. Every assertion below
 * exists because a screenshot at one width would not have caught that
 * and did not: the fault was invisible at the width the page was built
 * at and total at the width it was opened at.
 *
 * So what is checked is geometry over the BUILT bytes, at the widths the
 * site is actually opened at and in both languages: which row each entry
 * stands on, that the rows are the ones the composition declares, that
 * the two rows stand centred on one axis in the bar's middle rather than
 * clumped against the brand, that no two things in the bar overlap, that
 * the document never goes wide, and that the control which fell off has
 * a row it shares with something else. A row is read off the boxes — the vertical middles the browser
 * actually computed — because that is the only statement of «which line
 * is this on» that a CSS change cannot quietly make true while the page
 * says otherwise.
 *
 * One page load answers every question about one width, and the widths
 * are walked inside a test rather than across a hundred of them. That is
 * not only economy: this suite runs one worker against one small static
 * server, and a hundred extra loads early in the run was enough to push
 * an already timing-sensitive assertion in `language.spec.ts` over its
 * timeout about a third of the time. A test that reports a fault in
 * another file is reporting its own.
 */

import { expect, type Page, test } from "@playwright/test";

/** Where the bar is asked the question, and in which language. */
const PAGES = [
  { route: "/", locale: "en", label: "the English landing" },
  { route: "/ru/", locale: "ru", label: "the Russian landing" },
  { route: "/why/zap/", locale: "en", label: "an English Why page" },
  { route: "/ru/why/zap/", locale: "ru", label: "a Russian Why page" },
] as const;

/**
 * The widths the composition is answerable at: a wide desktop, a laptop,
 * a small laptop, a tablet held upright, and a phone. All but the last
 * get the two-row bar, and 834 is deliberately on the desktop side of
 * that line.
 */
const DESKTOP = [1920, 1440, 1280, 1024, 834] as const;
const PHONE = 390;

/** What each row of the desktop bar carries, left to right. */
const ROWS = {
  en: [
    ["Documentation", "GitHub", "GitVerse", "News & support", "search"],
    ["Vision", "Why VibeVM", "Why Zap", "AI-Native Language", "lang", "theme"],
  ],
  ru: [
    ["Документация", "GitHub", "GitVerse", "Новости и поддержка", "search"],
    [
      "Видение",
      "Почему VibeVM",
      "Почему Zap",
      "AI-Native Языки",
      "lang",
      "theme",
    ],
  ],
} as const;

/** A row's destinations, without the control that stands beside it. */
const CONTROLS = ["search", "lang", "theme"];

function destinations(row: readonly string[]): string[] {
  return row.filter((name) => !CONTROLS.includes(name));
}

type Box = {
  readonly name: string;
  readonly x: number;
  readonly right: number;
  readonly y: number;
  readonly bottom: number;
  readonly middle: number;
};

/**
 * Every visible thing in the header, as the browser placed it.
 *
 * Named by what it is rather than by its class, so an assertion below
 * reads as the sentence it is making. Anything the stylesheet has taken
 * out of the flow at this width is left out: a control that is not shown
 * is not on a row, and asking which row it is on would invent one.
 */
async function headerBoxes(page: Page): Promise<Box[]> {
  return page.evaluate(() => {
    const header = document.querySelector("header.docs-header");
    if (header === null) return [];
    const found: Box[] = [];
    const take = (name: string, element: Element | null): void => {
      if (element === null) return;
      const rect = element.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) return;
      found.push({
        name,
        x: Math.round(rect.x),
        right: Math.round(rect.right),
        y: Math.round(rect.y),
        bottom: Math.round(rect.bottom),
        middle: rect.y + rect.height / 2,
      });
    };
    take("brand", header.querySelector(".docs-header__brand"));
    for (const link of header.querySelectorAll(".landing-nav__link")) {
      take((link.textContent ?? "").trim(), link);
    }
    take("search", header.querySelector(".search-box"));
    take("lang", header.querySelector("[data-site-language]"));
    take("theme", header.querySelector("[data-theme-switch]"));
    return found;
  }) as Promise<Box[]>;
}

/**
 * The boxes gathered into the lines a reader sees, top to bottom and
 * then left to right.
 *
 * Eight pixels of tolerance because a row holds a 13px word beside a
 * 28px pill and the two share a centre rather than an edge; anything
 * that disagrees by more than that is on another line.
 */
function rows(boxes: readonly Box[]): Box[][] {
  const lines: Box[][] = [];
  for (const box of [...boxes].sort((a, b) => a.middle - b.middle)) {
    const last = lines[lines.length - 1];
    if (last !== undefined && Math.abs(last[0].middle - box.middle) <= 8) {
      last.push(box);
    } else {
      lines.push([box]);
    }
  }
  for (const line of lines) line.sort((a, b) => a.x - b.x);
  return lines;
}

/** Any two boxes in the bar that share pixels. */
function overlaps(boxes: readonly Box[]): string[] {
  const hit: string[] = [];
  for (let i = 0; i < boxes.length; i += 1) {
    for (let j = i + 1; j < boxes.length; j += 1) {
      const a = boxes[i];
      const b = boxes[j];
      const across = Math.min(a.right, b.right) - Math.max(a.x, b.x);
      const down = Math.min(a.bottom, b.bottom) - Math.max(a.y, b.y);
      if (across > 1 && down > 1) hit.push(`${a.name} × ${b.name}`);
    }
  }
  return hit;
}

/** Whether the document is wider than the window holding it. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

/**
 * The whole promise of the bar, at every width, in one pass.
 *
 * The desktop widths must produce the two rows the composition declares,
 * in the owner's order — the software, where its source is kept and where
 * it is spoken about, then the essay and the three arguments it is the
 * worldview of — with the brand centred against both rather than standing
 * on either, and the cluster of the two standing in the bar's middle,
 * leaning toward the brand. The phone must produce a different
 * composition rather than the same one squeezed. And at every width, in
 * both languages, nothing may overlap anything, the page may not go wide,
 * and the theme switch may not be alone on its line.
 */
for (const one of PAGES) {
  test(`${one.label} composes its header at every width`, async ({ page }) => {
    for (const width of [...DESKTOP, PHONE]) {
      await page.setViewportSize({
        width,
        height: width === PHONE ? 844 : 900,
      });
      await page.goto(one.route);
      await page.evaluate(() => document.fonts.ready);

      const boxes = await headerBoxes(page);
      const lines = rows(boxes);
      const named = lines.map((line) => line.map((box) => box.name));
      const at = `${one.label} at ${width}px`;

      expect(overlaps(boxes), `nothing overlaps — ${at}`).toEqual([]);
      expect(await scrolls(page), `no sideways scroll — ${at}`).toBe(false);

      /* The control that fell off has company, and it is the company the
         rest of the site puts it in. This is the defect itself, written
         as a question a build can answer. */
      const themeLine = named.find((line) => line.includes("theme"));
      expect(themeLine, `the theme switch is somewhere — ${at}`).toBeDefined();
      expect(themeLine, `the theme switch is not alone — ${at}`).not.toEqual([
        "theme",
      ]);
      expect(
        themeLine,
        `the theme switch keeps the language — ${at}`,
      ).toContain("lang");

      if (width === PHONE) {
        /* The compact bar: the preferences move up to the brand's line —
           the corner a thumb reaches first — the two nav rows take the
           width under them, and the field steps out, as it always did,
           because the manual's own header carries the same one a tap
           away. Each row of four becomes two columns of two, which is a
           shape: neither row fits across one phone line in either
           language, and one composition for both is the point. What is
           pinned is that shape AND the order inside it — a grid fills
           row-major, so the two lines of a row read as the row does. */
        expect(named[0], `brand keeps the preferences — ${at}`).toEqual([
          "brand",
          "lang",
          "theme",
        ]);
        expect(named, `five lines and no more — ${at}`).toHaveLength(5);
        for (const line of named.slice(1)) {
          expect(line, `two columns — ${at}`).toHaveLength(2);
        }
        expect(
          [...named[1], ...named[2]],
          `the software row, in order — ${at}`,
        ).toEqual(destinations(ROWS[one.locale][0]));
        expect(
          [...named[3], ...named[4]],
          `the argument row, in order — ${at}`,
        ).toEqual(destinations(ROWS[one.locale][1]));
        await expect(page.locator(".search-box")).toBeHidden();
        continue;
      }

      /* Three bands: the first nav row, the brand centred against both,
         and the second nav row. The brand is a line of its own in this
         reading precisely because its middle falls between the two. */
      expect(named, `three bands — ${at}`).toHaveLength(3);
      expect(named[0], `the first row — ${at}`).toEqual([
        ...ROWS[one.locale][0],
      ]);
      expect(named[1], `the brand between them — ${at}`).toEqual(["brand"]);
      expect(named[2], `the second row — ${at}`).toEqual([
        ...ROWS[one.locale][1],
      ]);

      /* «Visually stable» is this: the bar grew a second row underneath
         the mark and the mark did not move up to make room. */
      const brand = boxes.find((box) => box.name === "brand")!;
      const search = boxes.find((box) => box.name === "search")!;
      const theme = boxes.find((box) => box.name === "theme")!;
      const between = (search.middle + theme.middle) / 2;
      expect(
        Math.abs(brand.middle - between),
        `the brand is centred — ${at}`,
      ).toBeLessThanOrEqual(3);

      /* The centred cluster. The grid stands the two rows in the middle
         of the bar rather than against the brand — where they used to
         clump, with every spare pixel pooled into one void before the
         field — and leans them a little toward the brand, because the
         right flank is the heavier one and an arithmetic centre reads
         as pushed into it. What is asserted is the rule and not the
         pixel: one axis for both rows, a lean that is real but bounded,
         and clear air after the brand where the clump used to be. */
      const links = ROWS[one.locale].map((row) => destinations(row));
      const span = (line: readonly Box[], names: readonly string[]) => {
        const own = line.filter((box) => names.includes(box.name));
        return { x: own[0].x, right: own[own.length - 1].right };
      };
      const tools = span(lines[0], links[0]);
      const story = span(lines[2], links[1]);
      expect(
        Math.abs((tools.x + tools.right) / 2 - (story.x + story.right) / 2),
        `the rows share one axis — ${at}`,
      ).toBeLessThanOrEqual(8);

      const cluster =
        (Math.min(tools.x, story.x) + Math.max(tools.right, story.right)) / 2;
      expect(
        width / 2 - cluster,
        `the cluster leans toward the brand — ${at}`,
      ).toBeGreaterThanOrEqual(4);
      expect(
        width / 2 - cluster,
        `…and only leans — ${at}`,
      ).toBeLessThanOrEqual(64);
      expect(
        Math.min(tools.x, story.x) - brand.right,
        `air after the brand — ${at}`,
      ).toBeGreaterThanOrEqual(40);
    }
  });
}

/**
 * The manual's header is untouched.
 *
 * The two halves of the site share one component and the landing is the
 * half that gained a row; a change that had reached the other one would
 * show up here as a documentation page whose bar stopped being a bar.
 *
 * The door and a page, and deliberately not a package's own page: that
 * address carries a shelf whose cards are revealed by an island, and
 * loading it from this file made the timing-sensitive assertion in
 * `language.spec.ts` fail about half the time. The bar is the same bar
 * at all three addresses, so the cheapest true statement is the one that
 * does not lean on that shelf.
 */
test("the manual keeps its single-row header", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  for (const at of [
    "/doc/",
    "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
  ]) {
    await page.goto(at);
    await page.evaluate(() => document.fonts.ready);

    const lines = rows(await headerBoxes(page));
    expect(
      lines.map((line) => line.map((box) => box.name)),
      at,
    ).toEqual([["brand", "search", "lang", "theme"]]);

    const height = await page.evaluate(
      () =>
        document.querySelector("header.docs-header")?.getBoundingClientRect()
          .height ?? 0,
    );
    expect(Math.round(height), at).toBe(65);
  }
});

/**
 * The keyboard walks the bar by kind: every destination, then the field,
 * then the two preferences.
 *
 * Which is NOT the order the eye takes, and the difference is deliberate
 * and worth stating. The field is lifted into the first row because that
 * row is the documentation and this field searches it, so a reader
 * tabbing through the bar reaches it after the eighth link rather than
 * after the fourth. Geometry and sequence can both be had only by putting
 * each control inside the row it stands on — and that is the same move
 * that would push the theme switch to the foot of a five-line bar on a
 * phone instead of the corner beside the brand, which is the one thing
 * this whole composition exists to prevent.
 *
 * So the bar keeps the grouping the manual's header already has — the
 * places, then the field, then the settings — and what is pinned here is
 * that grouping, including the part of it a reader carries between the
 * two halves of the site: search, then language, then theme, in that
 * order, always. A change that moved the theme switch in front of the
 * field would break a habit rather than a layout, which is exactly the
 * kind of break no screenshot reports.
 *
 * The current entry and its focus ring ride along: both survived a
 * rewrite of the bar's layout, and both are the kind of thing that
 * survives one silently right up until it does not.
 */
test("the bar is tabbed by kind, and marks where the reader stands", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/why/zap/");
  await page.evaluate(() => document.fonts.ready);

  const order = await page.evaluate(() => {
    const header = document.querySelector("header.docs-header");
    if (header === null) return [];
    return [
      ...header.querySelectorAll<HTMLElement>("a[href], button, input"),
    ].map((element) => (element.textContent ?? "").trim() || element.tagName);
  });

  /* The brand, then the eight destinations in the two rows' own order —
     the first row's four, then the second row's four. */
  expect(order.slice(0, 9)).toEqual([
    "VibeVM",
    "Documentation",
    "GitHub",
    "GitVerse",
    "News & support",
    "Vision",
    "Why VibeVM",
    "Why Zap",
    "AI-Native Language",
  ]);
  /* Then the field, the two letters and the three themes — the tail the
     documentation's header ends with, in the same sequence. */
  expect(order.slice(9)).toEqual([
    "INPUT",
    "EN",
    "RU",
    "BUTTON",
    "BUTTON",
    "BUTTON",
  ]);

  const current = page.locator(
    'header .landing-nav__link[aria-current="page"]',
  );
  await expect(current).toHaveCount(1);
  await expect(current).toHaveText("Why Zap");

  await current.focus();
  await expect(current).toBeFocused();
  const outline = await current.evaluate(
    (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
  );
  expect(outline).not.toBe("0px");
});
