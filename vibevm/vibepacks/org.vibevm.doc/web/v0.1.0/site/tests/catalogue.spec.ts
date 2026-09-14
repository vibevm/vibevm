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

/**
 * The third question the door answers: whose words are in a
 * documentation.
 *
 * The fixture pair is written to be the interesting case — the source
 * says a model wrote it, the adaptation says both hands did — so «human-
 * authored» is measured on a document that is also in the other group.
 * A mark on a card and a group in a menu have to agree, and this is
 * where they are asked together.
 */
test("the shelf narrows to who wrote the prose, both hands in both groups", async ({
  page,
}) => {
  await door(page);
  const documents = page.locator('[data-tab-panel="documents"]');
  const standing = documents.locator(".card").filter({ visible: true });
  await expect(standing).toHaveCount(2);
  await expect(documents.locator(".badge--authorship")).toHaveText([
    "AI",
    "Mixed",
  ]);

  await page.locator("[data-authorship-filter] summary").click();
  await page.locator('[data-authorship-choice="human"]').click();
  await expect(standing).toHaveCount(1);
  await expect(standing.locator(".badge--authorship")).toHaveText("Mixed");
  await expect(
    page.locator("[data-authorship-filter] .authorship-filter__tag"),
  ).toHaveText("human");

  // The same document is in the other group, because both hands are in it.
  await page.locator("[data-authorship-filter] summary").click();
  await page.locator('[data-authorship-choice="ai"]').click();
  await expect(standing).toHaveCount(2);
});

/**
 * A bridge has two names to show and the one failure worth a test is
 * the two becoming one line: the maintainer of a wrapper printed as the
 * author of the work it wraps (PROP-023 `##AUTHORSHIP-SEPARATION`).
 *
 * The fixture source is the bridge and the adaptation beside it is not,
 * so both answers are on one shelf — and the head of the package's own
 * page has to give the same one as its card.
 */
test("a bridge names its maintainer and the upstream's author apart", async ({
  page,
}) => {
  await door(page);
  const cards = page.locator('[data-tab-panel="documents"] .card');
  const bridged = cards.first();
  await expect(bridged.locator(".bridge-signatures__row dt")).toHaveText([
    "Bridge maintainer",
    "Destination author",
    "Upstream licence",
  ]);
  await expect(bridged.locator(".bridge-signatures__row dd")).toHaveText([
    "The fixture's maintainer",
    "The author of the bytes the fixture wraps",
    "Apache-2.0",
  ]);
  // The adaptation is not a bridge, and its card is what it always was.
  await expect(cards.nth(1).locator(".bridge-signatures")).toHaveCount(0);
  await expect(cards.nth(1).locator(".card__publisher")).toContainText(
    "com.example.docs",
  );

  await page.goto("/doc/com.example.docs/fixture-manual/0.1.0/");
  const head = page.locator(".package-header");
  await expect(head.locator(".bridge-signatures__row dd")).toHaveText([
    "The fixture's maintainer",
    "The author of the bytes the fixture wraps",
    "Apache-2.0",
  ]);
  await expect(head.locator(".package-header__publisher")).toContainText(
    "com.example.docs",
  );

  await page.goto("/doc/ru/com.example.docs/fixture-manual/0.1.0/");
  await expect(page.locator(".package-header .bridge-signatures")).toHaveCount(
    0,
  );
});

/**
 * Two filters over one shelf, and the shelf is what is left when both
 * admit a card. Neither may quietly widen the other, which is exactly
 * what would happen if each hid cards on its own.
 */
test("the two filters narrow together, and a shelf they empty says so", async ({
  page,
}) => {
  await door(page);
  const documents = page.locator('[data-tab-panel="documents"]');
  const standing = documents.locator(".card").filter({ visible: true });

  await page.locator("[data-authorship-filter] summary").click();
  await page.locator('[data-authorship-choice="human"]').click();
  await expect(standing).toHaveCount(1);

  // The source is the one a model wrote, so asking for it and for what a
  // person wrote asks for nothing — and the shelf says so rather than
  // looking broken.
  await page.locator("[data-language-selector] summary").click();
  await page.locator('[data-doc-lang="en"]').click();
  await expect(standing).toHaveCount(0);
  await expect(documents.locator(".shelf__empty")).toBeVisible();

  // And the authorship the reader chose survives the page, as the
  // language and the shelf do.
  await door(page);
  await expect(
    page.locator("[data-authorship-filter] .authorship-filter__tag"),
  ).toHaveText("human");
});
