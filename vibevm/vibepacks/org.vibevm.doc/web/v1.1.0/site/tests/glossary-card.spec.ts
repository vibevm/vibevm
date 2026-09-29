/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-GLOSSARY-CARD */

/**
 * A glossary term's card in a real browser: the pause before it opens, where
 * it stands, and the three widths at which it must not.
 *
 * None of it can be measured anywhere else. That the card stands beside the
 * link without covering it, and that nothing it adds gives the page a
 * horizontal scrollbar, are statements about layout in a window of a given
 * size. That a touch screen gets no card and a tap goes to the glossary
 * instead is a statement about what the browser reports as a pointer. And
 * that the definition was in the page all along is measured by the fact that
 * the card fills with no request at all — the island carries the hidden
 * block, and the reader clones it.
 *
 * The fixture manual declares its reference page as its glossary and links
 * two of its entries from the page every block rides on, which is why this
 * reads that page.
 */

import { expect, type Locator, type Page, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";
const GLOSSARY = "/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/";

/** The term the fixture page links first, and the words it is defined in. */
const TERM = "island";
const DEFINITION = "A page's content as finished HTML";

/** Open a page and wait until the reader's behaviours are listening. */
async function read(page: Page, at: string = PAGE): Promise<void> {
  await page.goto(at);
  await expect(page.locator("html")).toHaveAttribute("data-reader", /./);
}

function term(page: Page): Locator {
  return page.locator(`[data-island] a[data-gloss="${TERM}"]`).first();
}

function card(page: Page): Locator {
  return page.locator("[data-gloss-card]");
}

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
});

test("the definition travels in the page, and the link says which one", async ({
  page,
}) => {
  await read(page);
  // The attributes the pipeline wrote: the entry, and the hidden block that
  // describes it on every device.
  await expect(term(page)).toHaveAttribute("aria-describedby", `gloss-${TERM}`);
  const hidden = page.locator(`[data-gloss-defs] #gloss-${TERM}`);
  await expect(hidden).toBeHidden();
  await expect(hidden).toContainText(DEFINITION);
  // And the link is unchanged: selecting it still opens the glossary.
  await expect(term(page)).toHaveAttribute("href", /reference\/addresses\/#island$/);
});

test("pointing at a term opens a card with its term and definition", async ({
  page,
}) => {
  await read(page);
  await expect(card(page)).toBeHidden();

  await term(page).hover();
  await expect(card(page)).toBeVisible({ timeout: 2000 });
  await expect(card(page).locator("[data-gloss-card-term]")).toHaveText(TERM);
  await expect(card(page).locator("[data-gloss-card-text]")).toContainText(
    DEFINITION,
  );
  // The reader hears the definition through the link, so the card says
  // nothing of its own.
  await expect(card(page)).toHaveAttribute("aria-hidden", "true");
});

test("the card waits for the pointer to rest before it opens", async ({
  page,
}) => {
  await read(page);
  await term(page).hover();
  // Straight after the hover there is nothing yet: the pause is what keeps a
  // pointer crossing the paragraph from opening every term on its way.
  expect(await card(page).isVisible()).toBe(false);
  await expect(card(page)).toBeVisible({ timeout: 2000 });
});

test("the card stands beside the term and covers neither it nor the window", async ({
  page,
}) => {
  await read(page);
  await term(page).hover();
  await expect(card(page)).toBeVisible({ timeout: 2000 });

  const link = await term(page).boundingBox();
  const box = await card(page).boundingBox();
  expect(link, "the term has no box").not.toBeNull();
  expect(box, "the card has no box").not.toBeNull();
  const above = (box?.y ?? 0) + (box?.height ?? 0) <= (link?.y ?? 0);
  const below = (box?.y ?? 0) >= (link?.y ?? 0) + (link?.height ?? 0);
  expect(above || below, "the card overlaps the term it explains").toBe(true);

  const width = page.viewportSize()?.width ?? 0;
  expect(box?.x ?? -1).toBeGreaterThanOrEqual(0);
  expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(width);
  expect(await scrolls(page), "the card scrolled the page sideways").toBe(false);
});

test("leaving the term closes the card, and Escape closes it too", async ({
  page,
}) => {
  await read(page);
  await term(page).hover();
  await expect(card(page)).toBeVisible({ timeout: 2000 });

  await page.mouse.move(2, 2);
  await expect(card(page)).toBeHidden({ timeout: 2000 });

  await term(page).hover();
  await expect(card(page)).toBeVisible({ timeout: 2000 });
  await page.keyboard.press("Escape");
  await expect(card(page)).toBeHidden();
});

test("keyboard focus on a term opens the card", async ({ page }) => {
  await read(page);
  await term(page).focus();
  await expect(card(page)).toBeVisible({ timeout: 2000 });
  await expect(card(page).locator("[data-gloss-card-term]")).toHaveText(TERM);
});

test("the glossary page itself shows no card", async ({ page }) => {
  await read(page, GLOSSARY);
  // Its own links to its own entries are ordinary links: the definitions are
  // on this page, and a card would repeat the paragraph under the cursor.
  await expect(page.locator("[data-island] a[data-gloss]")).toHaveCount(0);
  await expect(page.locator("[data-gloss-defs]")).toHaveCount(0);

  const first = page.locator("[data-island] a").first();
  await first.hover();
  await page.waitForTimeout(700);
  await expect(card(page)).toBeHidden();
});

test("the narrow layout shows no card, and a term is still a link", async ({
  page,
}) => {
  await page.setViewportSize({ width: 834, height: 1112 });
  await read(page);
  // The contents column has no room beside the text, which is the layout the
  // norm withholds the card in.
  await expect(page.locator(".doc-view.has-sidebar")).toHaveCount(0);

  await term(page).hover();
  await page.waitForTimeout(700);
  await expect(card(page)).toBeHidden();
  await expect(term(page)).toHaveAttribute("href", /reference\/addresses\/#island$/);
  expect(await scrolls(page)).toBe(false);
});

test("a touch screen shows no card, and a tap opens the glossary", async ({
  browser,
}) => {
  // A phone in every way the browser can be asked about: the width, the
  // touch points, and the pointer the media query reads.
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 },
    hasTouch: true,
    isMobile: true,
  });
  const page = await context.newPage();
  await read(page);

  await term(page).dispatchEvent("pointerover", { pointerType: "touch" });
  await page.waitForTimeout(700);
  await expect(card(page)).toBeHidden();

  await term(page).click();
  await expect(page).toHaveURL(new RegExp("reference/addresses/#island$"));
  await context.close();
});

/**
 * The card arrives for a reader who wants movement and arrives without the
 * fade for one who does not. The base stylesheet takes every transition's
 * DURATION away under `prefers-reduced-motion`, which makes a fade a jump
 * rather than no fade — so the card refuses the transition itself.
 */
test("the card does not animate under reduced motion", async ({ page }) => {
  const transition = (): Promise<string> =>
    card(page).evaluate(
      (one) => window.getComputedStyle(one).transitionProperty,
    );

  await page.emulateMedia({ reducedMotion: "no-preference" });
  await read(page);
  expect(await transition()).toBe("opacity");

  await page.emulateMedia({ reducedMotion: "reduce" });
  await read(page);
  expect(await transition()).toBe("none");

  // The card still arrives; it is the fade towards it that is refused.
  await term(page).hover();
  await expect(card(page)).toBeVisible({ timeout: 2000 });
});

/**
 * The dotted underline is the only thing that says which words have a card,
 * and it says it only where a card can appear.
 */
test("a term is underlined dotted in the desktop layout and plainly below it", async ({
  page,
}) => {
  const style = (): Promise<string> =>
    term(page).evaluate(
      (one) => window.getComputedStyle(one).textDecorationStyle,
    );

  await read(page);
  expect(await style()).toBe("dotted");

  await page.setViewportSize({ width: 834, height: 1112 });
  await read(page);
  expect(await style()).toBe("solid");
});
