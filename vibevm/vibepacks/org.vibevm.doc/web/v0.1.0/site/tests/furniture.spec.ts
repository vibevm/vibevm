/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

/**
 * The furniture around the text: the contents, the rules it cites, the
 * tables, the pictures and the fences.
 *
 * All of it is attached to an island the framework never rendered, which
 * is exactly why it is measured in a browser: whether a table ends up
 * inside a scrolling region, and whether the control that expands it is
 * still where the reader can reach it, are questions about a live
 * document.
 */

import { expect, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";

test.beforeEach(async ({ page }) => {
  await page.goto(PAGE);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });
});

test("the contents are built from the page and stick beside it", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PAGE);

  const items = page.locator("[data-toc-list] .toc__item");
  await expect(items.first()).toBeVisible();
  expect(await items.count()).toBeGreaterThanOrEqual(4);
  await expect(page.locator(".doc-view--page")).toHaveClass(/has-sidebar/);
  await expect(page.locator("[data-toc] summary")).toBeHidden();
});

test("widening the column past the sidebar's limit folds the contents", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PAGE);
  await page.locator("[data-settings-toggle]").click();
  const panel = page.locator("[data-settings-panel]");
  // 740 → 900 → 1100 → 1400: the last step is past the limit.
  await panel.locator("[data-step='width'][data-delta='1']").click();
  await panel.locator("[data-step='width'][data-delta='1']").click();
  await panel.locator("[data-step='width'][data-delta='1']").click();

  await expect(page.locator(".doc-view--page")).not.toHaveClass(/has-sidebar/);
  await expect(page.locator("[data-toc] summary")).toBeVisible();
});

test("a narrow screen gets the contents as a block above the text", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(PAGE);
  await expect(page.locator(".doc-view--page")).not.toHaveClass(/has-sidebar/);
  await expect(page.locator("[data-toc] summary")).toBeVisible();
});

test("the rules the page cites are listed, once each", async ({ page }) => {
  const rules = page.locator("[data-page-rules-list] li");
  await expect(rules.first()).toBeVisible();
  expect(await rules.count()).toBe(2);
  await expect(rules.first()).toContainText("spec://com.example/subject");
});

test("a table gets a scrolling region and expands into the overlay", async ({
  page,
}) => {
  const region = page.locator(".table-block .table-scroll");
  await expect(region).toBeVisible();
  await expect(region).toHaveAttribute("tabindex", "0");

  await page.locator("[data-expand-table]").first().click();
  await expect(page.locator("[data-lightbox]")).toBeVisible();
  await expect(page.locator("[data-lightbox-table] table")).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.locator("[data-lightbox]")).toBeHidden();
  // The original never moved: the numbered block IS the table, and it
  // is still one table with its number on it.
  await expect(page.locator("table[data-p='5']")).toHaveCount(1);
  await expect(page.locator("table[data-p='5'] a.p-anchor")).toHaveCount(1);
});

test("a picture opens in the overlay and closes from below it", async ({
  page,
}) => {
  await page.locator(".prose figure img").click();
  await expect(page.locator("[data-lightbox]")).toBeVisible();
  await expect(page.locator("[data-lightbox-image]")).toBeVisible();
  await page.locator("[data-lightbox-close]").click();
  await expect(page.locator("[data-lightbox]")).toBeHidden();
});

test("a fence says its language and hands over its text", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const fence = page.locator("[data-p='7'] .code-toolbar");
  await expect(fence.locator(".code-toolbar__lang")).toHaveText("rust");

  await fence.locator("[data-copy-code]").click({ force: true });
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied).toContain("let answer = 42;");
});

test("an executed example says which of its blocks is the expectation", async ({
  page,
}) => {
  await expect(page.locator("[data-p='14'] .block-note").first()).toHaveText(
    "expected output",
  );
  await expect(page.locator("[data-p='15'] .block-note").nth(1)).toHaveText(
    "expected error output",
  );
});

/**
 * The defect this pins was invisible to every other check and visible in
 * the first screenshot: a block that scrolls sideways lost its NUMBER,
 * because an overflow box clips its absolutely positioned descendants.
 * It hit exactly the blocks a bug report is most likely to cite — the
 * fences, the generated output and the wide table.
 */
test("a block that scrolls sideways keeps its number in the margin", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PAGE);
  for (const id of ["p05", "p07", "p08", "p13"]) {
    const anchor = page.locator(`a.p-anchor#${id}`);
    await expect(anchor).toBeVisible();
    const box = await anchor.boundingBox();
    expect(box, `${id} has no box`).not.toBeNull();
    expect(box?.width ?? 0).toBeGreaterThan(0);
  }
});

test("a generated block says what generated it and from what", async ({
  page,
}) => {
  // The note stands ABOVE the block, as a sibling: it is read before the
  // text it explains rather than after it.
  const note = page.locator(".block-note", { hasText: "generated from" });
  await expect(note).toContainText("generated from cli-help");
  await expect(note.locator("code")).toHaveText("vibe list --help");
  await expect(
    page.locator(".block-note + pre.derived[data-p='13']"),
  ).toHaveCount(1);
});
