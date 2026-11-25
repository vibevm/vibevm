/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS */

/**
 * The reader's behaviours, in a real browser over the built site.
 *
 * Each test states one promise the norm makes to a reader, and each one
 * would pass silently in a unit test that never opened a page: a block
 * «in the middle of the window», a page that does NOT scroll itself, a
 * theme that is already right on the first frame. None of those is a
 * pure function; all of them are what a reader actually experiences.
 */

import { expect, type Page, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const RU_PAGE =
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/";

/**
 * Open a page and wait until its behaviours are listening.
 *
 * A documentation page looks interactive from the first frame and is
 * inert until the reader's chunk has loaded: the gear, the pills and the
 * block numbers are markup with a listener added afterwards, and a click
 * that arrives in between goes nowhere and is not retried. The page says
 * which of the two states it is in, and every test that clicks a control
 * asks first.
 */
async function read(page: Page, at: string = PAGE): Promise<void> {
  await page.goto(at);
  await expect(page.locator("html")).toHaveAttribute("data-reader", /./);
}

/** The reader's own memory, cleared so one test cannot seed another. */
test.beforeEach(async ({ page }) => {
  await page.goto(PAGE);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });
});

test("opening a block number puts the block in the middle of the window", async ({
  page,
}) => {
  await page.goto(`${PAGE}#p07`);
  const block = page.locator("[data-p='7']");
  await expect(block).toBeVisible();

  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const element = document.querySelector("[data-p='7']");
          if (element === null) return 1;
          const box = element.getBoundingClientRect();
          const middle = box.top + box.height / 2;
          return Math.abs(middle - window.innerHeight / 2) / window.innerHeight;
        }),
      { timeout: 4000 },
    )
    .toBeLessThan(0.15);
});

test("a block number copies the whole address and ticks", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await read(page);
  const anchor = page.locator("a.p-anchor#p03");
  await anchor.click();

  await expect(page).toHaveURL(new RegExp("#p03$"));
  await expect(anchor).toHaveClass(/copied/);
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied).toContain("/guide/every-block/#p03");
});

test("a reload offers the reading place and does not take it", async ({
  page,
}) => {
  await read(page);
  await page.evaluate(() => window.scrollTo(0, 2200));
  // The position is saved at most once a second, by design.
  await page.waitForTimeout(1400);

  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-reader", /./);
  await expect(page.locator("[data-return]")).toBeVisible();
  expect(await page.evaluate(() => window.scrollY)).toBe(0);

  await page.locator("[data-return]").click();
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 4000 })
    .toBeGreaterThan(200);
});

test("changing the language keeps the fragment", async ({ page }) => {
  await read(page, `${PAGE}#p07`);
  await page.locator("[data-language-selector] summary").click();
  const russian = page.locator("[data-language-selector] a[hreflang='ru']");
  await expect(russian).toHaveAttribute("href", new RegExp("#p07$"));
  await russian.click();
  await expect(page).toHaveURL(new RegExp("/doc/ru/.*#p07$"));
});

test("the chosen theme is already on the page at the first frame", async ({
  page,
}) => {
  await page.goto(PAGE);
  await page.evaluate(() =>
    window.localStorage.setItem("vibe-doc:theme", "dark"),
  );

  await page.addInitScript(() => {
    const record = window as unknown as { __themeAtFirstFrame?: string | null };
    record.__themeAtFirstFrame = "not-measured";
    requestAnimationFrame(() => {
      record.__themeAtFirstFrame =
        document.documentElement.getAttribute("data-theme");
    });
  });

  await page.reload();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () =>
            (window as unknown as { __themeAtFirstFrame?: string | null })
              .__themeAtFirstFrame,
        ),
      { timeout: 4000 },
    )
    .toBe("dark");
});

/**
 * One theme switch, in the header, on every address of the site.
 *
 * It used to be the landing's alone and the reading panel's on a
 * document page — two controls for one choice, and none at all on the
 * catalogue or on a documentation's own page, which have a header but no
 * reading panel. The two questions this pins are «is it offered here»
 * and «is it offered twice».
 */
test("the theme is offered in the header of every documentation page", async ({
  page,
}) => {
  const addresses = [
    "/doc/",
    "/doc/com.example.docs/fixture-manual/0.1.0/",
    PAGE,
  ];
  for (const at of addresses) {
    await page.goto(at);
    await expect(page.locator("html")).toHaveAttribute("data-site-lang", /./);
    await expect(page.locator(".docs-header [data-theme-switch]")).toHaveCount(
      1,
    );
    await expect(page.locator("[data-theme-choice]")).toHaveCount(3);

    await page.locator('.docs-header [data-theme-choice="dark"]').click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.locator('.docs-header [data-theme-choice="light"]').click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  }
});

test("the reading panel offers the reading and no longer the theme", async ({
  page,
}) => {
  await read(page);
  await page.locator("[data-settings-toggle]").click();
  const panel = page.locator("[data-settings-panel]");
  await expect(panel).toBeVisible();
  await expect(panel.locator("[data-theme-choice]")).toHaveCount(0);
  await expect(panel.locator("[data-step='font']")).toHaveCount(2);
  await expect(panel.locator("[data-toggle='anchors']")).toHaveCount(1);
});

test("the settings panel changes the reading and remembers it", async ({
  page,
}) => {
  await read(page);
  await page.locator("[data-settings-toggle]").click();
  // Scoped to the panel: the same handles sit in the quick row, which is
  // hidden until the reader is actually reading.
  const panel = page.locator("[data-settings-panel]");
  await panel.locator("[data-step='font'][data-delta='1']").click();
  await panel.locator("[data-step='width'][data-delta='1']").click();

  await expect(panel.locator("[data-value='font']")).toHaveText("110%");
  await expect(panel.locator("[data-value='width']")).toHaveText("900");

  await page.reload();
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const column = document.querySelector(".prose");
          if (!(column instanceof HTMLElement)) return null;
          return {
            font: column.style.getPropertyValue("--reader-font"),
            width: column.style.getPropertyValue("--measure"),
          };
        }),
      { timeout: 5000 },
    )
    .toEqual({ font: "1.1", width: "900px" });
});

test("turning the block numbers off leaves the ids in place", async ({
  page,
}) => {
  await read(page);
  await page.locator("[data-settings-toggle]").click();
  await page
    .locator("[data-settings-panel] [data-toggle='anchors']")
    .first()
    .click();

  await expect(page.locator("a.p-anchor#p03")).toBeHidden();
  // The link still lands: the numbers are a setting, not the addresses.
  await page.goto(`${PAGE}#p03`);
  expect(
    await page.evaluate(() => document.getElementById("p03") !== null),
  ).toBe(true);
});

test("the platform switch hides the other platforms and is remembered", async ({
  page,
}) => {
  await read(page);
  await page.locator("[data-when-switch] button[value='linux']").click();
  await expect(page.locator("[data-when='os:linux']")).toBeVisible();
  await expect(page.locator("[data-when='os:windows']")).toBeHidden();

  await page.reload();
  await expect(page.locator("[data-when='os:linux']")).toBeVisible();
  await expect(page.locator("[data-when='os:windows']")).toBeHidden();
});

test("a quoted rule opens beside itself with the text already in the page", async ({
  page,
}) => {
  await read(page);
  /* The quotation is inside a disclosure that arrives closed
     (`##READER-RULE-FOLDED`), so the line is opened first: what this test
     is about is the panel, and the fold has its own file. */
  const fold = page.locator("details.rule-fold").first();
  await fold.locator("summary").click();
  await fold.locator("blockquote.rule a.rule").click();
  const panel = page.locator("[data-rule-panel]");
  await expect(panel).toBeVisible();
  await expect(panel.locator("[data-rule-uri]")).toHaveText(
    "spec://com.example/subject/common/PROP-001#A-RULE",
  );
  await expect(panel.locator("[data-rule-text]")).toContainText("MUST");
});

test("the agent surface cites the block the reader is standing on", async ({
  page,
}) => {
  await page.goto(`${PAGE}#p11`);
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => document.querySelector("[data-agent-uri]")?.textContent ?? "",
        ),
      { timeout: 5000 },
    )
    .toMatch(
      /spec:\/\/com\.example\.docs\/fixture-manual@0\.1\.0\/guide\/every-block#p\d\d/,
    );
});

test("an adaptation serves the page it has in its own language", async ({
  page,
}) => {
  await page.goto(RU_PAGE);
  await expect(page.locator("[data-language-selector] summary")).toContainText(
    "ru",
  );
  await expect(page.locator("[data-island]")).toBeVisible();
});

/**
 * The version is on every page of the manual and beside the language,
 * which is the pair the owner's review asked to see together: which text
 * am I reading, and which publication of it.
 *
 * `latest` stands first because it is the address to keep — a number
 * shows whatever that number currently holds and is a permanent link to
 * nothing — and choosing it lands on the SAME document rather than at
 * the top of the manual.
 */
test("a page offers the version it is read at, latest first", async ({
  page,
}) => {
  await page.goto(PAGE);
  const entries = page.locator(".version-switch a");
  await expect(entries).toHaveText(["latest", "0.1.0"]);
  await expect(entries.nth(1)).toHaveAttribute("aria-current", "true");

  await entries.nth(0).click();
  await expect(page).toHaveURL(new RegExp("/latest/guide/every-block/$"));
  await expect(page.locator("[data-island] h1")).toHaveText("Every block once");
  await expect(page.locator(".version-switch a").nth(0)).toHaveAttribute(
    "aria-current",
    "true",
  );
});

test("a documentation's own page offers it as well", async ({ page }) => {
  await page.goto("/doc/com.example.docs/fixture-manual/0.1.0/");
  const entries = page.locator(".version-switch a");
  await expect(entries).toHaveText(["latest", "0.1.0"]);
  await entries.nth(0).click();
  await expect(page).toHaveURL(
    new RegExp("/doc/com\\.example\\.docs/fixture-manual/latest/$"),
  );
});

/**
 * ≡ — the button in the reading controls, and the list it is named after.
 *
 * It used to ask for `[data-toc]`, which is the OTHER list: the headings
 * of the page the reader is already on. So the contents never opened, and
 * on a wide screen the page scrolled to a sticky column and landed
 * wherever that column happened to be standing — «does not expand that
 * very contents, and it sends you to the wrong coordinates» (owner,
 * 2026-09-26).
 *
 * The two shapes of the list get the two answers a reader's question has,
 * and both are measured: folded, the list opens under the header and the
 * way back is offered; in the column, the page does not move at all.
 */

/**
 * Where the page has come to rest.
 *
 * The site scrolls smoothly (`base.css`), so every measurement of a
 * position has to wait for the movement to end — a reading taken one
 * frame after a scroll was asked for is a reading of the animation.
 */
async function settled(page: Page): Promise<number> {
  let last = Number.NaN;
  for (let tries = 0; tries < 40; tries += 1) {
    const now = await page.evaluate(() => window.scrollY);
    if (now === last) return now;
    last = now;
    await page.waitForTimeout(150);
  }
  return last;
}

/** Get the reader reading, so the quick row with ≡ in it is shown. */
async function reading(page: Page, at: string = PAGE): Promise<number> {
  await read(page, at);
  await page.evaluate(() => window.scrollTo(0, 2200));
  await expect(page.locator("[data-quick-toc]")).toBeVisible();
  return settled(page);
}

test("the contents button opens the contents under the header", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const was = await reading(page);
  expect(was).toBeGreaterThan(400);

  const contents = page.locator("[data-contents]");
  await expect(contents.locator("a").first()).toBeHidden();
  await page.locator("[data-quick-toc]").click();

  /* Open, which is what the button is for. */
  await expect(contents.locator("a").first()).toBeVisible();

  /* And standing clear of the sticky header rather than behind it, once
     the movement has ended. */
  await settled(page);
  const where = await page.evaluate(() => {
    const summary = document.querySelector("[data-contents] summary");
    const header = document.querySelector(".docs-header");
    if (summary === null || header === null) return null;
    return {
      top: summary.getBoundingClientRect().top,
      under: header.getBoundingClientRect().bottom,
      window: window.innerHeight,
    };
  });
  expect(where).not.toBeNull();
  expect(where?.top ?? -1).toBeGreaterThanOrEqual(where?.under ?? 0);
  expect(where?.top ?? -1).toBeLessThan(where?.window ?? 0);

  /* The keyboard is where the eye is. */
  await expect(page.locator("[data-contents] summary")).toBeFocused();

  /* Going to the contents is leaving a place, so the way back is offered
     — and it leads back to the block the reader was standing on. */
  const back = page.locator("[data-return]").first();
  await expect(back).toBeVisible();
  await back.click();
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 4000 })
    .toBeGreaterThan(1000);
});

test("in the column the contents button moves the column and not the page", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await reading(page);

  const before = await settled(page);
  await page.locator("[data-quick-toc]").click();

  /* The page is where it was: the list is already on the screen, and a
     jump to a sticky column is a jump to nowhere in particular. Asked
     after the page has had time to move, so that «it did not move» is a
     measurement and not a race. */
  expect(await settled(page)).toBe(before);

  /* What moved is the column's own scroller, far enough to show the page
     the reader is on — and that entry is what the keyboard now holds. */
  const current = page.locator("[data-contents] .contents__link--current");
  await expect(current.first()).toBeFocused();
  /* And it says so for a moment, because nothing else on the page did. */
  await expect(current.first()).toHaveClass(/contents__link--cued/);
  const inside = await page.evaluate(() => {
    const column = document.querySelector("[data-contents]");
    const links = [
      ...document.querySelectorAll("[data-contents] .contents__link--current"),
    ].filter(
      (link) => link instanceof HTMLElement && link.offsetParent !== null,
    );
    const link = links[0];
    if (column === null || link === undefined) return null;
    const frame = column.getBoundingClientRect();
    const box = link.getBoundingClientRect();
    return box.top >= frame.top - 1 && box.bottom <= frame.bottom + 1;
  });
  expect(inside).toBe(true);
});
