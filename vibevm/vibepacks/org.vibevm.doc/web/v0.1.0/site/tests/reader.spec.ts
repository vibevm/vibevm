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

import { expect, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const RU_PAGE =
  "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/";

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
  await page.goto(PAGE);
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
  await page.goto(PAGE);
  await page.evaluate(() => window.scrollTo(0, 2200));
  // The position is saved at most once a second, by design.
  await page.waitForTimeout(1400);

  await page.reload();
  await expect(page.locator("[data-return]")).toBeVisible();
  expect(await page.evaluate(() => window.scrollY)).toBe(0);

  await page.locator("[data-return]").click();
  await expect
    .poll(async () => page.evaluate(() => window.scrollY), { timeout: 4000 })
    .toBeGreaterThan(200);
});

test("changing the language keeps the fragment", async ({ page }) => {
  await page.goto(`${PAGE}#p07`);
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

test("the settings panel changes the reading and remembers it", async ({
  page,
}) => {
  await page.goto(PAGE);
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
  await page.goto(PAGE);
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
  await page.goto(PAGE);
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
  await page.goto(PAGE);
  await page.locator("blockquote.rule a.rule").first().click();
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
