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

import { expect, type Page, test } from "@playwright/test";

const PAGE = "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/";

test.beforeEach(async ({ page }) => {
  await page.goto(PAGE);
  await page.evaluate(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });
});

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

/**
 * The manual's own pages, in a column and never in a row.
 *
 * The row they used to stand in held about six entries and then scrolled
 * sideways, so a manual of thirty pages put everything after the sixth
 * behind a gesture. What is measured here is the column's two shapes and
 * the promise that replaced the row: no sideways scroll at either width.
 *
 * Every case below is about the SECTIONS view — the pinned pages and the
 * folders of `##NAV-PINNED`, which a declared learning path does not
 * change. The fixture now declares a path, so that view is the second one
 * the column offers and its pane is named rather than assumed: the whole
 * point of `##NAV-CHAPTERS-READER` is that this list is still there and
 * still exactly what it was. The path itself is measured in
 * `learning-path.spec.ts`.
 */
test("the manual's pages stand in a column, grouped by their folders", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PAGE);

  const column = page.locator("[data-contents]");
  await expect(column.locator(".contents__summary")).toBeHidden();
  const sections = column.locator("[data-contents-sections]");
  /* The manifest pins this page and names one folder, so the column is
     the documentation's own statement about itself: the pinned page
     first and outside every group, then the folder under the words the
     package chose rather than the name of the directory. */
  await expect(sections.locator("a").first()).toHaveText("Every block once");
  await expect(sections.locator(".contents__heading")).toHaveText([
    "Reference pages",
  ]);
  await expect(sections.locator("a")).toHaveText([
    "Every block once",
    "Addresses",
  ]);
  await expect(sections.locator("a[aria-current='page']")).toHaveText(
    "Every block once",
  );
});

test("the contents scrollbar belongs to the documentation column", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 600 });
  await page.goto(PAGE);
  const column = page.locator("[data-contents]");
  await expect(page.locator(".doc-view--page")).toHaveClass(/has-sidebar/);

  const styles = await column.evaluate((element) => ({
    overflow: getComputedStyle(element).overflowY,
    gutter: getComputedStyle(element).scrollbarGutter,
    rail: getComputedStyle(element).backgroundImage,
    firefoxWidth: getComputedStyle(element).scrollbarWidth,
    webkitWidth: getComputedStyle(element, "::-webkit-scrollbar").width,
    buttonDisplay: getComputedStyle(element, "::-webkit-scrollbar-button")
      .display,
    track: getComputedStyle(element, "::-webkit-scrollbar-track")
      .backgroundColor,
  }));
  expect(styles).toMatchObject({
    overflow: "scroll",
    gutter: "stable",
    firefoxWidth: "thin",
    webkitWidth: "6px",
    buttonDisplay: "none",
  });
  expect(styles.rail).toContain("linear-gradient");
  expect(styles.track).not.toBe("rgba(0, 0, 0, 0)");
});

/**
 * A section's identity is the source's and its name is the reader's
 * edition's: the folder is the same in every language, and what stands
 * over it is what that edition's own manifest calls it. The links under
 * it follow the same rule one level down, which is what the second half
 * of this test is about.
 */
test("a folder is named in the words of the edition being read", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(
    "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
  );

  const sections = page.locator("[data-contents] [data-contents-sections]");
  await expect(sections.locator(".contents__heading")).toHaveText([
    "Справочные страницы",
  ]);
  /* The pins are the source's — a pin names a document, and a document
     is the same one in every language — and they lead to the
     adaptation's own addresses. */
  await expect(sections.locator("a").first()).toHaveAttribute(
    "href",
    "/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
  );
  /* Both halves of the rule about the words, on one screen. The
     adaptation carries the pinned page and named it, so the link is its
     name; it has not reached the other page, so that link keeps the
     source's — which is the text that address actually serves. */
  await expect(sections.locator("a")).toHaveText([
    "Каждый блок по разу",
    "Addresses",
  ]);
});

test("a phone gets the same pages as a block it opens itself", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(PAGE);

  const column = page.locator("[data-contents]");
  await expect(column.locator(".contents__summary")).toBeVisible();
  await expect(column.locator("a").first()).toBeHidden();

  // `<details>` and nothing else: the browser opens it, not a script.
  await column.locator("summary").click();
  await expect(column.locator("a").first()).toBeVisible();
});

test("nothing on a documentation page or the door scrolls sideways", async ({
  page,
}) => {
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 900 });
    for (const at of [PAGE, "/doc/"]) {
      await page.goto(at);
      expect(await scrolls(page), `${at} at ${width}px`).toBe(false);
    }
  }
});

test("the page's own headings are built from it and stick beside it", async ({
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

test("widening the column past the sidebar's limit folds both lists", async ({
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
  await expect(page.locator("[data-contents] summary")).toBeVisible();
});

test("a narrow screen gets both lists as blocks above the text", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(PAGE);
  await expect(page.locator(".doc-view--page")).not.toHaveClass(/has-sidebar/);
  await expect(page.locator("[data-toc] summary")).toBeVisible();
  await expect(page.locator("[data-contents] summary")).toBeVisible();
});

/**
 * The rules the page cites, folded under the end of it.
 *
 * They used to hang in the side column under the headings, where a few
 * `spec://` addresses made the block taller than the contents above it.
 * What matters now is that the list is the same list, that the summary
 * says how many before it is opened, and that it is at the END of the
 * reading column rather than beside it.
 */
test("the rules the page cites are folded under it, once each", async ({
  page,
}) => {
  const block = page.locator("[data-page-rules]");
  await expect(block).toBeVisible();
  await expect(block.locator("summary")).toHaveText(
    "Rules this page cites (2)",
  );

  const rules = block.locator("li");
  await expect(rules.first()).toBeHidden();
  await block.locator("summary").click();
  await expect(rules.first()).toBeVisible();
  expect(await rules.count()).toBe(2);
  await expect(rules.first()).toContainText("spec://com.example/subject");

  // Under the text, not beside it: the column on the right holds the
  // page's headings and nothing else now.
  await expect(page.locator("[data-toc] [data-page-rules]")).toHaveCount(0);
  const meta = await page
    .locator(".prose")
    .evaluate((column) => column.getBoundingClientRect().bottom);
  const rulesTop = await block.evaluate(
    (element) => element.getBoundingClientRect().top,
  );
  expect(rulesTop).toBeLessThan(meta);
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

/**
 * A line longer than the block is FOLDED, not hidden behind a gesture.
 *
 * The owner's report was about the live manual: «every code example is
 * shown without word wrap, so long lines cannot be read», and the request
 * block at the top of a task page was one single line (2026-09-26). The
 * block used to scroll sideways inside itself, which on a phone showed
 * the half of a command that says nothing and on any screen hid the end
 * of a path.
 *
 * The long line is written into the fixture's bare fence here rather than
 * carried by it, and the reason is in `src/fixtures/README.md`: the
 * fixture library cannot grow a page (the policy's inline-script ceiling,
 * X-044) and its own lines are short, so the case a real manual is full
 * of — a `cargo` command, a `spec://` address — has to be put there.
 * Nothing about the STYLESHEET is faked by it: the block is the
 * pipeline's own numbered fence, in the built page, and what is measured
 * is what the browser does with a line in it. The blocks measured after
 * it carry the fixture's own words at the width where they are already
 * too long.
 */
const LONG =
  "cargo install --locked --path crates/vibe-cli --root /usr/local --features full # spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS";

/** Whether an element has to be scrolled to be read to the end. */
async function scrollsInside(page: Page, selector: string): Promise<boolean> {
  return page.evaluate((one) => {
    const element = document.querySelector(one);
    if (element === null) return true;
    return element.scrollWidth > element.clientWidth + 1;
  }, selector);
}

/** How many lines tall a block's text is, by its own line height. */
async function lines(page: Page, selector: string): Promise<number> {
  return page.evaluate((one) => {
    const element = document.querySelector(one);
    if (!(element instanceof HTMLElement)) return 0;
    const step = Number.parseFloat(getComputedStyle(element).lineHeight);
    if (Number.isNaN(step) || step === 0) return 0;
    return Math.round(element.getBoundingClientRect().height / step);
  }, selector);
}

/** Put one line into a block of the page, as a manual's own page carries it. */
async function write(
  page: Page,
  selector: string,
  text: string,
): Promise<void> {
  await page.evaluate(
    ([one, words]) => {
      const element = document.querySelector(one ?? "");
      if (element !== null) element.textContent = words ?? "";
    },
    [selector, text],
  );
}

test("a fence folds a line too long for it and keeps its number", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(PAGE);

  const fence = "[data-p='8'] code";
  await write(page, fence, LONG);

  expect(await scrollsInside(page, fence)).toBe(false);
  expect(await lines(page, fence)).toBeGreaterThan(1);

  /* And the block still says which block it is. The number hangs in the
     margin of the `pre`, which is why the fold is on the `code`: an
     overflow box would clip it. */
  const anchor = page.locator("a.p-anchor#p08");
  await expect(anchor).toBeVisible();
  const box = await anchor.boundingBox();
  expect(box?.width ?? 0).toBeGreaterThan(0);
  expect(box?.x ?? -1).toBeGreaterThan(0);

  /* Nor does the page itself scroll: a fold inside the column cannot
     widen the document. */
  expect(await scrolls(page)).toBe(false);
});

test("a request folds at its spaces, as prose does", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(PAGE);

  const prompt = "[data-p='17'] .prompt-text code";
  await write(
    page,
    prompt,
    "Install vibe on this machine, add the redbook stack to it, and then show me the version of every package the lockfile now pins.",
  );

  expect(await scrollsInside(page, prompt)).toBe(false);
  expect(await lines(page, prompt)).toBeGreaterThan(1);
  await expect(page.locator("a.p-anchor#p17")).toBeVisible();
});

/**
 * And at the width of a phone the fixture's own lines are already longer
 * than the column, so nothing is written in: what is measured here is the
 * page exactly as the pipeline rendered it.
 */
test("on a phone the blocks the page carries fold by themselves", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(PAGE);

  for (const selector of [
    "[data-p='17'] .prompt-text code",
    "[data-p='15'] .example-stderr code",
  ]) {
    expect(await scrollsInside(page, selector), selector).toBe(false);
    expect(await lines(page, selector), selector).toBeGreaterThan(1);
  }
  expect(await scrolls(page)).toBe(false);
});

/**
 * The way into the rest of the manual, where there is no column to show
 * it in.
 *
 * It was a faint uppercase label in the mono face, at 0.7rem, in the
 * quietest text tone — and on a phone it is the only way to another page
 * of the manual. «A person will not notice it» (owner, 2026-09-26). What
 * is pinned here is that it is a CONTROL: a box a thumb can hit, 44px of
 * it, that opens the list it names.
 */
test("the way into the manual is a control a thumb can hit", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(PAGE);

  const summary = page.locator("[data-contents] .contents__summary");
  await expect(summary).toBeVisible();
  const box = await summary.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);

  /* It looks like something that can be pressed: a border of its own and
     a ground under it, and the interface's own face rather than the mono
     eyebrow it was. */
  const drawn = await summary.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      border: style.borderTopWidth,
      transform: style.textTransform,
    };
  });
  expect(drawn.border).not.toBe("0px");
  expect(drawn.transform).toBe("none");

  /* And it opens the manual's pages, by being a `<details>` and nothing
     else. */
  await expect(page.locator("[data-contents] a").first()).toBeHidden();
  await summary.click();
  await expect(page.locator("[data-contents] a").first()).toBeVisible();

  /* The page's own headings are the same shape one step quieter, so the
     pair reads as navigation with one of the two leading. */
  const toc = page.locator("[data-toc] .toc__summary");
  await expect(toc).toBeVisible();
  const both = await page.evaluate(() => {
    const read = (selector: string): Record<string, string> => {
      const element = document.querySelector(selector);
      if (element === null) return {};
      const style = getComputedStyle(element);
      return { ground: style.backgroundColor, size: style.fontSize };
    };
    return {
      contents: read(".contents__summary"),
      toc: read(".toc__summary"),
    };
  });
  expect(both.contents["ground"]).not.toBe(both.toc["ground"]);
  expect(Number.parseFloat(both.contents["size"] ?? "0")).toBeGreaterThan(
    Number.parseFloat(both.toc["size"] ?? "0"),
  );
});

/** On a wide screen the column shows the pages and the control stays away. */
test("the control is absent where the column shows the pages", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(PAGE);
  await expect(page.locator("[data-contents] .contents__summary")).toBeHidden();
  await expect(page.locator("[data-toc] .toc__summary")).toBeHidden();
});
