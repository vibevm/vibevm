/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-RULE-FOLDED */

/**
 * A cited rule in a real browser: closed on arrival, opened by a click or
 * by Enter, numbered while it is closed, and open on paper.
 *
 * None of it can be measured anywhere else. That the page arrives folded
 * is a statement about the bytes the pipeline wrote and no script — the
 * disclosure has no `open` attribute, so there is nothing for a unit test
 * to call. That the line holds to one line, that the block's number still
 * hangs in the margin beside it, and that nothing of the fold scrolls
 * sideways on a phone are statements about layout. And what a print does
 * is only observable as the events a print fires.
 *
 * The fixture page carries the two cases the norm distinguishes: a rule
 * whose own words describe it, and one this build could not resolve,
 * which shows the edition's generic line instead. The edition here is
 * English; that a Russian edition says so in Russian is decided in the
 * pipeline and measured there (`crates/vibe-doc/src/html/rule/tests.rs`),
 * because the fixture library has no Russian island to serve — its
 * adaptation falls back to the source's page and therefore to the
 * source's bytes.
 */

import { expect, type Locator, type Page, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";

/** The description the fixture's one resolved rule is cut down to. */
const GIST = "A package MUST declare its kind…";

/** Open a page and wait until the reader's behaviours are listening. */
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

/** The rule whose own words describe it, and the one that has none. */
function described(page: Page): Locator {
  return page.locator("details.rule-fold").first();
}

function generic(page: Page): Locator {
  return page.locator("details.rule-fold").nth(1);
}

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

/** What a print does, as the two events a print fires. */
async function print(page: Page): Promise<void> {
  await page.evaluate(() => window.dispatchEvent(new Event("beforeprint")));
}

async function printed(page: Page): Promise<void> {
  await page.evaluate(() => window.dispatchEvent(new Event("afterprint")));
}

test("a cited rule arrives folded to one line", async ({ page }) => {
  await read(page);
  const fold = described(page);

  await expect(fold.locator(".rule-fold__mark")).toBeVisible();
  await expect(fold.locator(".rule-fold__kind")).toHaveText("spec:");
  await expect(fold.locator(".rule-fold__gist")).toHaveText(GIST);
  await expect(fold.locator("blockquote.rule")).toBeHidden();

  // And the line is one line: the description is cut by the browser, not
  // wrapped past the height of the block it is hiding.
  const line = fold.locator("summary");
  const height = await line.evaluate((one) => one.getBoundingClientRect().height);
  const single = await line.evaluate((one) =>
    Number.parseFloat(window.getComputedStyle(one).lineHeight),
  );
  expect(height).toBeLessThan(single * 2.5);
});

test("a rule with nothing to describe shows the edition's own words", async ({
  page,
}) => {
  await read(page);
  const fold = generic(page);
  await expect(fold.locator(".rule-fold__gist--generic")).toHaveText(
    "quote from the specification",
  );
  // The label introduced a quotation of the rule; there is none to
  // introduce, so it is not there either.
  await expect(fold.locator(".rule-fold__kind")).toHaveCount(0);
  await expect(fold).toHaveAttribute("data-unresolved", "true");
});

test("clicking the line opens the rule, and the mark turns", async ({
  page,
}) => {
  await read(page);
  const fold = described(page);
  const quote = fold.locator("blockquote.rule");

  await fold.locator("summary").click();
  await expect(quote).toBeVisible();
  await expect(quote.locator("a.rule")).toContainText("MUST");
  await expect
    .poll(async () =>
      fold
        .locator(".rule-fold__mark")
        .evaluate((one) => window.getComputedStyle(one).transform),
    )
    .toBe("matrix(0, 1, -1, 0, 0, 0)");

  await fold.locator("summary").click();
  await expect(quote).toBeHidden();
});

test("Enter on the line opens the rule and closes it again", async ({
  page,
}) => {
  await read(page);
  const fold = described(page);
  const quote = fold.locator("blockquote.rule");

  await fold.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(quote).toBeVisible();

  await page.keyboard.press("Enter");
  await expect(quote).toBeHidden();
});

/**
 * The number is the address a reader quotes, so it cannot wait for the
 * fold to be opened — and a click on it copies that address rather than
 * opening anything, which is the one place the fold and the numbers could
 * have fought over a click.
 */
test("the block's number stands in the margin while the rule is closed", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await read(page);
  const fold = described(page);
  const anchor = fold.locator("a.p-anchor");

  await expect(anchor).toBeVisible();
  await expect(anchor).toHaveText("11");

  const number = await anchor.boundingBox();
  const line = await fold.locator("summary").boundingBox();
  expect(number, "the number has no box").not.toBeNull();
  expect(line, "the line has no box").not.toBeNull();
  expect((number?.x ?? 0) + (number?.width ?? 0)).toBeLessThanOrEqual(
    line?.x ?? 0,
  );

  await anchor.click();
  await expect(fold.locator("blockquote.rule")).toBeHidden();
  await expect(page).toHaveURL(`${PAGE}#p11`);
});

test("a print opens every closed rule and gives the page back after it", async ({
  page,
}) => {
  await read(page);
  const folds = page.locator("details.rule-fold");
  const total = await folds.count();
  expect(total).toBeGreaterThan(1);

  await print(page);
  await expect(page.locator("details.rule-fold[open]")).toHaveCount(total);

  await printed(page);
  await expect(page.locator("details.rule-fold[open]")).toHaveCount(0);
});

test("a rule the reader opened by hand survives the print", async ({
  page,
}) => {
  await read(page);
  const fold = described(page);
  await fold.locator("summary").click();

  await print(page);
  await printed(page);

  await expect(fold).toHaveAttribute("open", "");
  await expect(page.locator("details.rule-fold[open]")).toHaveCount(1);
});

test("the rule panel opens from the text the fold reveals", async ({
  page,
}) => {
  await read(page);
  const fold = described(page);
  await fold.locator("summary").click();
  await fold.locator("a.rule").click();

  const panel = page.locator("[data-rule-panel]");
  await expect(panel).toBeVisible();
  await expect(panel.locator("[data-rule-uri]")).toHaveText(
    "spec://com.example/subject/common/PROP-001#A-RULE",
  );
  await expect(panel.locator("[data-rule-text]")).toContainText("MUST");
});

test("the page lists the rules it cites, folded or opened", async ({
  page,
}) => {
  await read(page);
  const list = page.locator("[data-page-rules] [data-page-rules-list] a");
  await expect(list).toHaveCount(2);
  await expect(list.first()).toHaveText(
    "spec://com.example/subject/common/PROP-001#A-RULE",
  );
});

test("nothing the fold adds scrolls sideways at any width", async ({
  page,
}) => {
  for (const width of [390, 834, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await read(page);
    expect(await scrolls(page), `closed at ${width}px`).toBe(false);

    // And opened, which is where a long quotation could push the column
    // out: the print is the cheapest way to open all of them at once.
    await print(page);
    await expect(page.locator("details.rule-fold[open]").first()).toBeVisible();
    expect(await scrolls(page), `opened at ${width}px`).toBe(false);
  }
});

/**
 * The mark turns for a reader who wants movement and is already turned
 * for one who does not. The base stylesheet takes every transition's
 * DURATION away under `prefers-reduced-motion`, which makes a movement a
 * jump rather than no movement — so the mark refuses the transition
 * itself, and that is what is measured.
 */
test("the mark turns on opening, and does not animate under reduced motion", async ({
  page,
}) => {
  const transition = (): Promise<string> =>
    described(page)
      .locator(".rule-fold__mark")
      .evaluate((one) => window.getComputedStyle(one).transitionProperty);
  const turned = (): Promise<string> =>
    described(page)
      .locator(".rule-fold__mark")
      .evaluate((one) => window.getComputedStyle(one).transform);

  await page.emulateMedia({ reducedMotion: "no-preference" });
  await read(page);
  expect(await transition()).toBe("transform");

  await page.emulateMedia({ reducedMotion: "reduce" });
  await read(page);
  expect(await transition()).toBe("none");

  // The state still arrives; it is the movement towards it that is
  // refused.
  await described(page).locator("summary").click();
  await expect.poll(turned).toBe("matrix(0, 1, -1, 0, 0, 0)");
});
