/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-READER */

/**
 * The learning path in a real browser: the column that opens on it, the
 * switch that leaves it, and the two links at the end of every page.
 *
 * Every promise here is about a live document and none of them can be
 * measured anywhere else. That the chosen view is already right at the
 * FIRST frame is the whole point of stamping it before the stylesheet —
 * a unit test would see the choice arrive and never see the flash it
 * exists to prevent. That a pane is hidden rather than absent is a
 * statement about computed style. And that the pager's hrefs are the
 * path's neighbours is a statement about the page a reader lands on.
 *
 * The fixture manual declares two chapters over its two pages, the second
 * an appendix, so this file measures the two ENDS of a path and the step
 * across the seam between its chapters. A page with a neighbour on either
 * side needs a path of three, which the fixture library cannot grow to
 * (see `site/src/fixtures/README.md` on the policy ceiling) — that case is
 * measured over a library written inside `site/src/lib/contents.test.ts`.
 */

import { expect, type Page, test } from "@playwright/test";

const FIRST = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const LAST = "/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/";
const PACKAGE = "/doc/com.example.docs/fixture-manual/0.1.0/";
const RU_LAST =
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/reference/addresses/";

/** The reader's own memory, cleared so one test cannot seed another. */
test.beforeEach(async ({ page }) => {
  await page.goto(LAST);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });
});

/** Open a page and wait until the reader's behaviours are listening. */
async function read(page: Page, at: string): Promise<void> {
  await page.goto(at);
  await expect(page.locator("html")).toHaveAttribute("data-reader", /./);
}

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

test("the column opens on the path, with the appendix unnumbered", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(LAST);

  const path = page.locator("[data-contents] [data-contents-path]");
  await expect(path).toBeVisible();
  await expect(
    page.locator("[data-contents] [data-contents-sections]"),
  ).toBeHidden();

  await expect(path.locator(".contents__heading")).toHaveText([
    "1Reading a page",
    "Tables to look things up in",
  ]);
  await expect(path.locator(".contents__number")).toHaveText(["1"]);
  await expect(path.locator("a")).toHaveText(["Every block once", "Addresses"]);
  await expect(path.locator("a[aria-current='page']")).toHaveText("Addresses");
});

test("the switch says which view is showing, and takes a focus ring", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page, LAST);

  const views = page.locator("[data-contents-views]");
  await expect(views).toHaveAttribute("role", "group");
  await expect(views).toHaveAttribute("aria-label", "Contents view");
  await expect(views.locator("button")).toHaveText(["In order", "By section"]);
  await expect(views.locator("[data-contents-choice='path']")).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(
    views.locator("[data-contents-choice='sections']"),
  ).toHaveAttribute("aria-pressed", "false");

  // A visible ring and not a colour: the buttons are reached by keyboard.
  const outline = await views
    .locator("[data-contents-choice='sections']")
    .evaluate((button) => {
      button.focus();
      const style = window.getComputedStyle(button);
      return `${style.outlineStyle} ${style.outlineWidth}`;
    });
  expect(outline).not.toContain("none");
  expect(outline).not.toBe("solid 0px");
});

test("choosing the folders swaps the panes and the marked button", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page, LAST);

  await page.locator("[data-contents-choice='sections']").click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-contents-view",
    "sections",
  );
  await expect(
    page.locator("[data-contents] [data-contents-sections]"),
  ).toBeVisible();
  await expect(
    page.locator("[data-contents] [data-contents-path]"),
  ).toBeHidden();
  await expect(
    page.locator("[data-contents-choice='sections']"),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("[data-contents-choice='path']")).toHaveAttribute(
    "aria-pressed",
    "false",
  );

  // And back, which is the same choice in the other direction.
  await page.locator("[data-contents-choice='path']").click();
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-contents-view",
    "sections",
  );
  await expect(
    page.locator("[data-contents] [data-contents-path]"),
  ).toBeVisible();
});

/**
 * The choice survives a reload and is already applied at the first frame.
 *
 * This is the test the whole arrangement exists for. The server writes
 * both views and a script ahead of the stylesheet stamps the root, so a
 * reader who asked for the folders must never watch the path appear and
 * go away again — and the only way to see that is to look at the document
 * before it has been painted twice.
 */
test("the chosen view is already on the page at the first frame", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page, LAST);
  await page.locator("[data-contents-choice='sections']").click();

  await page.addInitScript(() => {
    const record = window as unknown as {
      __viewAtFirstFrame?: string | null;
      __pathAtFirstFrame?: string | null;
    };
    record.__viewAtFirstFrame = "not-measured";
    requestAnimationFrame(() => {
      record.__viewAtFirstFrame =
        document.documentElement.getAttribute("data-contents-view");
      const pane = document.querySelector("[data-contents-path]");
      record.__pathAtFirstFrame =
        pane === null ? null : window.getComputedStyle(pane).display;
    });
  });

  await page.reload();
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const record = window as unknown as {
            __viewAtFirstFrame?: string | null;
            __pathAtFirstFrame?: string | null;
          };
          return [record.__viewAtFirstFrame, record.__pathAtFirstFrame];
        }),
      { timeout: 4000 },
    )
    .toEqual(["sections", "none"]);

  // And it is still the folders once the reader has loaded, rather than
  // the stamp being undone by the settings behaviour a moment later.
  await expect(page.locator("html")).toHaveAttribute("data-reader", /./);
  await expect(
    page.locator("[data-contents] [data-contents-sections]"),
  ).toBeVisible();
});

test("the path ends each page with the neighbours it has, and no others", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto(FIRST);
  await expect(page.locator("[data-pager-previous]")).toHaveCount(0);
  await expect(page.locator("[data-pager-next]")).toHaveAttribute("href", LAST);

  await page.goto(LAST);
  await expect(page.locator("[data-pager-previous]")).toHaveAttribute(
    "href",
    FIRST,
  );
  await expect(page.locator("[data-pager-next]")).toHaveCount(0);
});

test("the pager names the chapter the path crosses into", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  /* Into the appendix, which is named and carries no number — here as in
     the column. */
  await page.goto(FIRST);
  await expect(page.locator("[data-pager-next] .pager__chapter")).toHaveText(
    "Tables to look things up in",
  );
  await expect(page.locator("[data-pager-next] .pager__title")).toHaveText(
    "Addresses",
  );
  await expect(
    page.locator("[data-pager-next] .pager__chapter-number"),
  ).toHaveCount(0);

  /* And out of it, into a numbered chapter, whose caption carries the
     number. */
  await page.goto(LAST);
  await expect(
    page.locator("[data-pager-previous] .pager__chapter"),
  ).toHaveText("Chapter 1 · Reading a page");
  await expect(page.locator("[data-pager-previous] .pager__title")).toHaveText(
    "Every block once",
  );
});

test("the pager stands after everything the page itself carries", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page, FIRST);

  const rules = await page
    .locator("[data-page-rules]")
    .evaluate((block) => block.getBoundingClientRect().bottom);
  const pager = await page
    .locator("[data-pager]")
    .evaluate((block) => block.getBoundingClientRect().top);
  expect(pager).toBeGreaterThan(rules);
  /* Inside the reading column and not beside it: it is the end of the
     text, not a panel about the page. */
  await expect(page.locator(".prose [data-pager]")).toHaveCount(1);
});

test("a package's own page opens the documentation at the path's first page", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PACKAGE);

  await expect(page.locator(".doc-package__start")).toHaveText("Start here");
  await expect(page.locator(".doc-package__start")).toHaveAttribute(
    "href",
    FIRST,
  );
  /* The first card of the «Documentation» shelf is the same door. */
  await expect(
    page.locator(".shelf", { hasText: "Documentation" }).locator("a").first(),
  ).toHaveAttribute("href", FIRST);

  const shelf = page.locator(".shelf", { hasText: "Pages" });
  await expect(shelf.locator(".shelf__caption")).toHaveText(
    "In the order of the learning path",
  );
  await expect(shelf.locator(".doc-chapter")).toHaveText([
    "1Reading a page",
    "Tables to look things up in",
  ]);
  /* Each title carries the standing badge beside the name, so each is
     matched at its start: what is asserted is which pages stand under
     which chapter, and in which order. */
  await expect(shelf.locator(".card__title")).toHaveText([
    /^Every block once/,
    /^Addresses/,
  ]);
});

test("a reader in Russian gets the chapters in Russian and the rest from the source", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page, RU_LAST);
  await page.locator('.docs-header [data-site-lang-choice="ru"]').click();

  /* The adaptation named the first chapter and left the appendix alone,
     so the appendix keeps the source's words — which is the rule, not a
     gap in the fixture. */
  await expect(
    page.locator("[data-contents] [data-contents-path] .contents__heading"),
  ).toHaveText(["1Как читать страницу", "Tables to look things up in"]);
  await expect(page.locator("[data-contents-views]")).toHaveAttribute(
    "aria-label",
    "Вид оглавления",
  );
  await expect(page.locator("[data-contents-views] button")).toHaveText([
    "По порядку",
    "По разделам",
  ]);
  await expect(page.locator("[data-pager-previous] .pager__word")).toHaveText(
    "Назад",
  );
  await expect(
    page.locator("[data-pager-previous] .pager__chapter"),
  ).toHaveText("Глава 1 · Как читать страницу");

  await page.goto(`${RU_LAST.replace("reference/addresses/", "")}`);
  await expect(page.locator(".doc-package__start")).toHaveText("Начать отсюда");
});

test("nothing the path adds scrolls sideways at any width", async ({
  page,
}) => {
  for (const width of [390, 834, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    for (const at of [FIRST, LAST, PACKAGE]) {
      await page.goto(at);
      /* The package page carries no column, so what is waited for is the
         view itself: the switch and the pager are inside it either way. */
      await expect(page.locator(".doc-view")).toBeVisible();
      expect(await scrolls(page), `${at} at ${width}px`).toBe(false);
    }
  }
});

test("a phone gets the switch inside the block it opens", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await read(page, LAST);

  const column = page.locator("[data-contents]");
  await expect(column.locator("[data-contents-views]")).toBeHidden();
  await column.locator("summary").click();
  await expect(column.locator("[data-contents-views]")).toBeVisible();

  // And it still switches from there, where a phone reader will meet it.
  await column.locator("[data-contents-choice='sections']").click();
  await expect(column.locator("[data-contents-sections]")).toBeVisible();
  await expect(column.locator("[data-contents-path]")).toBeHidden();
});

/**
 * The arrow moves two pixels towards where the link leads, and does not
 * move at all for a reader who asked for stillness.
 *
 * The base stylesheet takes every transition's DURATION away under
 * `prefers-reduced-motion`, which turns a movement into a jump rather
 * than into no movement — so the pager refuses the transform itself, and
 * that is what is measured here.
 */
test("the pager's arrow steps on hover, and stands still under reduced motion", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });

  /* The step is a transition, so the value is polled until it settles:
     the frame after the pointer arrives still carries the one before. */
  const arrow = (): Promise<string> =>
    page
      .locator("[data-pager-next] .pager__arrow")
      .evaluate((one) => window.getComputedStyle(one).transform);
  const border = (): Promise<string> =>
    page
      .locator("[data-pager-next]")
      .evaluate((one) => window.getComputedStyle(one).borderTopColor);
  const hoverTheWayOn = async (): Promise<string> => {
    await page.goto(FIRST);
    const resting = await border();
    await page.locator("[data-pager-next]").hover();
    return resting;
  };

  await page.emulateMedia({ reducedMotion: "no-preference" });
  await hoverTheWayOn();
  await expect.poll(arrow, { timeout: 4000 }).toBe("matrix(1, 0, 0, 1, 2, 0)");

  await page.emulateMedia({ reducedMotion: "reduce" });
  const resting = await hoverTheWayOn();
  await expect.poll(arrow, { timeout: 4000 }).toBe("none");
  /* And the hover did land: the card's own line answers it either way, so
     the stillness above is the transform being refused rather than the
     pointer having missed. */
  await expect.poll(border, { timeout: 4000 }).not.toBe(resting);
});

/**
 * Every rule of the switch is keyed to one class on the column's nav, so
 * that a documentation which declared no path is untouched by all of
 * them. This says the class is there when a path is; the other half —
 * that a documentation without one shows a single view and no pager — is
 * measured on the fixture pair, which declares none, in
 * `libraries.spec.ts`.
 */
test("the switch is offered where a path was declared", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(LAST);
  await expect(
    page.locator("[data-contents] .contents__nav--path"),
  ).toHaveCount(1);
  await expect(page.locator("[data-contents-views]")).toHaveCount(1);
});
