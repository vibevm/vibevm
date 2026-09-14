/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * The door's three shelves, in a browser, over the built page.
 *
 * All three are in the document and the reader moves between them
 * without a request, which is the thing worth measuring: a tab that
 * fetched would be a tab that could fail, and a tab that re-rendered
 * would be a second opinion about a list the build already wrote.
 *
 * What this build features is the deployment's default — the two
 * coordinates of the real domain — and this build carries neither, so
 * the featured shelf is empty and the door opens on the documents. That
 * is not a defect being pinned: it is the rule that a door must not open
 * on an empty shelf, measured where it actually happens.
 */

import { expect, type Page, test } from "@playwright/test";

const DOOR = "/doc/";

const ROW = '[data-tab-switch="catalogue"]';

/**
 * Open the door and wait until the shelves answer to the reader rather
 * than to the build.
 *
 * The stamp on the row is the behaviour saying it has read the address
 * and the reader's memory; until it is there the pills carry the mark
 * the build wrote, and a click lands on markup that is not listening
 * yet. The page is small enough for that to be the common case rather
 * than a rare one.
 */
async function door(page: Page, at: string = DOOR): Promise<void> {
  await page.goto(at);
  await expect(page.locator(ROW)).toHaveAttribute("data-tab-current", /./);
}

test.beforeEach(async ({ page }) => {
  await page.goto(DOOR);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
    document.cookie = "lang=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/";
  });
});

test("the door offers three shelves and opens on one with something on it", async ({
  page,
}) => {
  await door(page);
  const pills = page.locator('[data-tab-switch="catalogue"] button');
  await expect(pills).toHaveText(["Featured", "Documents", "Projections"]);

  await expect(page.locator('[data-tab-panel="documents"]')).toBeVisible();
  await expect(page.locator('[data-tab-panel="featured"]')).toBeHidden();
  await expect(page.locator('[data-tab-panel="projections"]')).toBeHidden();
  await expect(
    page.locator('[data-tab-switch="catalogue"] button[value="documents"]'),
  ).toHaveAttribute("aria-pressed", "true");
});

/**
 * An empty shelf still speaks: the alternative — hiding it — answers the
 * reader's question by making the question disappear.
 */
test("a shelf with nothing on it says so rather than vanishing", async ({
  page,
}) => {
  await door(page);
  await page.locator(`${ROW} button[value="featured"]`).click();
  const featured = page.locator('[data-tab-panel="featured"]');
  await expect(featured).toBeVisible();
  await expect(featured.locator(".shelf__empty")).toContainText(
    "named as featured",
  );
  await expect(page.locator('[data-tab-panel="documents"]')).toBeHidden();
});

test("the shelf a reader is on is in the address and is remembered", async ({
  page,
}) => {
  await door(page);
  await page.locator(`${ROW} button[value="projections"]`).click();
  await expect(page).toHaveURL(new RegExp("/doc/\\?tab=projections$"));

  // Remembered: the door opens where the reader left it.
  await door(page);
  await expect(page.locator('[data-tab-panel="projections"]')).toBeVisible();

  // And the address is the stronger of the two, for a link somebody sent.
  await door(page, `${DOOR}?tab=documents`);
  await expect(page.locator('[data-tab-panel="documents"]')).toBeVisible();
});

/**
 * The two narrowings are two questions. A tab says what KIND of thing a
 * reader wants; the language filter says which edition of it. Neither
 * may quietly answer the other.
 */
test("the language filter works inside the shelf a reader is on", async ({
  page,
}) => {
  await door(page);
  const documents = page.locator('[data-tab-panel="documents"]');
  await expect(documents.locator(".card")).toHaveCount(2);

  await page.locator("[data-language-selector] summary").click();
  await page.locator('[data-doc-lang="ru"]').click();

  await expect(
    documents.locator(".card").filter({ visible: true }),
  ).toHaveCount(1);
  await expect(page.locator('[data-tab-panel="documents"]')).toBeVisible();
});
