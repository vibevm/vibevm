/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * Which VibeVM this is — and which it is not — everywhere a crawler reads
 * it.
 *
 * Search engines and model crawlers had been joining this project with
 * its namesakes — Phala Network's VibeVM, a development sandbox on Phala
 * Cloud; the tgbyte project of the same name; and, for the AI Native
 * Languages, the karanchawla VVM project (owner, 2026-09-26). The site
 * says so in four places, in the owner's own sentences: at the foot of
 * both landings, in the root `llms.txt` and `llms-full.txt`, and in the
 * structured data. A sentence that quietly went missing from one of them
 * would be the index's cue to join the two again, so each place is held
 * here.
 *
 * What is NOT here is anything about those projects. Each note names one
 * and says nothing further about it, and nothing on the site links to
 * one; the tests below check the words this site publishes, in the exact
 * form it publishes them.
 *
 * Since 2026-09-27 the other half of that question is held here too: what
 * the project IS called. The name is one string in `landing/identity.ts`
 * and several surfaces print it — the first small-print note, the two meta
 * values, the signature at the foot of every landing page, the graph's
 * application and website nodes, the `isPartOf` of every subpage, and the
 * title of the feed. A surface that kept the short name while its
 * neighbours took the full one is the same defect as a missing note: an
 * index with two answers to one question.
 */

import { expect, test } from "@playwright/test";

import {
  AI_NATIVE_LANGUAGES_EN,
  AI_NATIVE_LANGUAGES_NOT_EN,
  AI_NATIVE_LANGUAGES_WHAT_EN,
  NAME_STATEMENT_EN,
  PHALA_DISAMBIGUATION_EN,
  TGBYTE_DISAMBIGUATION_EN,
} from "../src/landing/identity.ts";
import { STRINGS } from "../src/landing/i18n.ts";
import { CREATOR_SITE } from "../src/lib/people.ts";

const LANDINGS = [
  { route: "/", locale: "en", says: "Phala Network's VibeVM" },
  { route: "/ru/", locale: "ru", says: "VibeVM от Phala Network" },
] as const;

for (const one of LANDINGS) {
  test(`${one.route} ends with the four notes, in the owner's order`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const notes = page.locator("main .landing-disambiguation");
    await expect(notes).toHaveCount(4);

    const strings = STRINGS[one.locale];
    await expect(notes.nth(0)).toBeVisible();
    await expect(notes.nth(0)).toContainText("Anarchic VibeVM");
    await expect(
      notes.nth(0).locator('a[href="https://anarchic.pro"]'),
    ).toHaveCount(1);
    await expect(notes.nth(0).locator(`a[href="${CREATOR_SITE}"]`)).toHaveCount(
      1,
    );
    await expect(notes.nth(1)).toContainText(one.says);
    await expect(notes.nth(1)).toContainText("github.com/Phala-Network/VibeVM");
    await expect(notes.nth(1)).toContainText("Phala Cloud");

    /* The last two, word for word: they are the owner's sentences and
       the page is the place they are read, so nothing is matched loosely
       here. */
    await expect(notes.nth(2)).toHaveText(strings.disambiguationTgbyte);
    await expect(notes.nth(3)).toHaveText(strings.disambiguationAiNative);

    /* Last in the page's own content: small print after everything the
       page says, not a line inside it. */
    const last = page.locator("main > :last-child");
    await expect(last).toHaveClass(/\blanding-disambiguation\b/);
  });
}

/** The English wording is one string in one place; the page reads it. */
test("/ carries the two English notes exactly as they are written", async ({
  page,
}) => {
  await page.goto("/");
  const notes = page.locator("main .landing-disambiguation");
  await expect(notes.nth(2)).toHaveText(TGBYTE_DISAMBIGUATION_EN);
  await expect(notes.nth(3)).toHaveText(AI_NATIVE_LANGUAGES_EN);
});

/**
 * The two VibeVM disambiguation notes identify this project by its full
 * name (owner, 2026-09-27), and only at the first mention.
 *
 * Which is what these sentences are FOR: they exist to be read by
 * somebody who has arrived holding the wrong project, and the first thing
 * such a reader needs is the name of the right one. The rest of each
 * paragraph goes on in the short form — a note that said «Anarchic
 * VibeVM» five times would be arguing with the reader rather than telling
 * them apart.
 */
const NAMED_NOTES = [
  { route: "/", locale: "en" },
  { route: "/ru/", locale: "ru" },
] as const;

for (const one of NAMED_NOTES) {
  test(`${one.route} opens its two VibeVM notes with the full name`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const notes = page.locator("main .landing-disambiguation");
    for (const index of [1, 2]) {
      const text = await notes.nth(index).innerText();
      expect(text.startsWith("Anarchic VibeVM")).toBe(true);
      expect(text.slice("Anarchic VibeVM".length)).not.toContain("Anarchic");
    }
    /* And the fourth note is about the languages and says nothing about
       the project's name: it was not the owner's to change here. */
    await expect(notes.nth(3)).toHaveText(
      STRINGS[one.locale].disambiguationAiNative,
    );
  });
}

/**
 * The two text files an agent reads open with the name too: the heading
 * carries it, and the sentence that states it stands before every note
 * about somebody else's project.
 */
test("the root llms.txt is headed by the full name and states it first", async ({
  request,
}) => {
  const text = await (await request.get("/llms.txt")).text();
  expect(text.startsWith("# Anarchic VibeVM\n")).toBe(true);

  const statement = text.indexOf(NAME_STATEMENT_EN);
  expect(statement).toBeGreaterThan(-1);
  expect(text.indexOf("Disambiguation:")).toBeGreaterThan(statement);
  expect(text.indexOf(PHALA_DISAMBIGUATION_EN)).toBeGreaterThan(statement);

  /* Plain text, so the two addresses are in parentheses rather than
     behind words, and they are the tables' own. */
  expect(NAME_STATEMENT_EN).toContain("(https://anarchic.pro)");
  expect(NAME_STATEMENT_EN).toContain(`(${CREATOR_SITE})`);
});

test("the root llms-full.txt is headed by the full name and states it first", async ({
  request,
}) => {
  const text = await (await request.get("/llms-full.txt")).text();
  expect(text.startsWith("# Anarchic VibeVM — vibevm.org (full text)\n")).toBe(
    true,
  );
  const header = text.slice(0, text.indexOf("\n---\n"));
  const statement = header.indexOf(NAME_STATEMENT_EN);
  expect(statement).toBeGreaterThan(-1);
  expect(header.indexOf(PHALA_DISAMBIGUATION_EN)).toBeGreaterThan(statement);
  expect(header.indexOf(TGBYTE_DISAMBIGUATION_EN)).toBeGreaterThan(statement);
  expect(header.indexOf(AI_NATIVE_LANGUAGES_EN)).toBeGreaterThan(statement);
});

/**
 * And the graph says it in its own vocabulary: the application's name is
 * the full one, the short one is an alternate name for the same thing, and
 * the person who conceived and built it is a `Person` a crawler can
 * resolve (owner, 2026-09-27).
 */
test("the structured data names the project in full and names its author", async ({
  page,
}) => {
  await page.goto("/");
  const nodes = graphNodes(
    await page.locator('script[type="application/ld+json"]').allTextContents(),
  );
  const software = nodes.find(
    (node) => node["@type"] === "SoftwareApplication",
  );
  expect(software?.["name"]).toBe("Anarchic VibeVM");
  expect(software?.["alternateName"]).toEqual(["VibeVM"]);
  expect(software?.["author"]).toEqual({
    "@type": "Person",
    name: "Oleg Chirukhin",
    url: CREATOR_SITE,
  });
});

/**
 * And the site is named the way the project is (owner, 2026-09-27): the
 * `WebSite` node carries the full name with the short one beside it, and
 * every subpage that says which site it belongs to repeats the same name.
 *
 * The subpages are asked one per kind of graph the marketing half writes
 * — the default `WebPage`, the roadmap's `WebPage` with its list, and the
 * essay's `Article` — because each of them spells `isPartOf` in its own
 * file and a name that moved in one is a name that can stay behind in the
 * other two.
 */
test("the site graph names the site in full", async ({ page }) => {
  await page.goto("/");
  const nodes = graphNodes(
    await page.locator('script[type="application/ld+json"]').allTextContents(),
  );
  const site = nodes.find((node) => node["@type"] === "WebSite");
  expect(site?.["name"]).toBe("Anarchic VibeVM");
  expect(site?.["alternateName"]).toEqual(["VibeVM"]);
});

for (const route of [
  "/why/vibevm/",
  "/ru/why/vibevm/",
  "/roadmap/",
  "/ru/roadmap/",
  "/vision/",
  "/ru/vision/",
  "/news-and-support/",
]) {
  test(`${route} says it is part of the site by its full name`, async ({
    page,
  }) => {
    await page.goto(route);
    const graph = await page
      .locator('script[type="application/ld+json"]')
      .first()
      .textContent();
    const parsed = JSON.parse(graph ?? "{}") as {
      isPartOf?: { name?: string };
    };
    expect(parsed.isPartOf?.name).toBe("Anarchic VibeVM");
  });
}

/**
 * The signature at the foot of every landing page: the project's full
 * name, then the owner's tagline in the reader's own language.
 *
 * The wordmark in the corner of the header is deliberately NOT this — it
 * is a mark and stays the short name — and the two standing side by side
 * on one page is exactly the kind of thing that gets "tidied" by somebody
 * who has not read why. So both are pinned, together.
 */
const SIGNED = [
  { route: "/", says: "Anarchic VibeVM — Spec-Driven Development, packaged." },
  {
    route: "/ru/",
    says: "Anarchic VibeVM — Spec-Driven Development, в пакетах.",
  },
  {
    route: "/why/zap/",
    says: "Anarchic VibeVM — Spec-Driven Development, packaged.",
  },
] as const;

for (const one of SIGNED) {
  test(`${one.route} signs itself with the full name`, async ({ page }) => {
    await page.goto(one.route);
    const brand = page.locator("footer .landing-footer__brand");
    expect((await brand.innerText()).replace(/\s+/g, " ").trim()).toBe(
      one.says,
    );
    await expect(page.locator("header .docs-header__brand")).toHaveText(
      "VibeVM",
    );
  });
}

/** The feed is the site's, so its channel carries the site's name. */
test("the feed names its channel in full", async ({ request }) => {
  const xml = await (await request.get("/feed.xml")).text();
  expect(xml).toContain("<title>Anarchic VibeVM</title>");
});

/**
 * And the index an agent reads names the site the way the footer signs it
 * (owner, 2026-09-27): the entry under «## Project» that points at this
 * domain carries the full name and the tagline after it.
 *
 * `llms-full.txt` has no such line to follow — its header lists the three
 * authoritative addresses bare, without a tagline — so what is held there
 * is that the heading and the name statement above it still carry the
 * name, which the two tests above already do.
 */
test("the root llms.txt names the site in full under Project", async ({
  request,
}) => {
  const text = await (await request.get("/llms.txt")).text();
  const line = text
    .split("\n")
    .find((one) => one.startsWith("- [Official site]"));
  expect(line).toBe(
    "- [Official site](https://vibevm.org): Anarchic VibeVM — spec-driven development, packaged.",
  );
  expect(text.indexOf("## Project")).toBeLessThan(text.indexOf(line ?? ""));
});

/** The same name is the site's, where a share card reads it. */
for (const one of NAMED_NOTES) {
  test(`${one.route} shares itself under the full name`, async ({ page }) => {
    await page.goto(one.route);
    const site = await page.getAttribute(
      'meta[property="og:site_name"]',
      "content",
    );
    expect(site).toBe("Anarchic VibeVM");
    await expect(page).toHaveTitle(STRINGS[one.locale].metaTitle);
    expect(STRINGS[one.locale].metaTitle.startsWith("Anarchic VibeVM")).toBe(
      true,
    );
    const description = await page.getAttribute(
      'meta[name="description"]',
      "content",
    );
    expect(description).toBe(STRINGS[one.locale].metaDescription);
    expect(description?.startsWith("Anarchic VibeVM")).toBe(true);
  });
}

test("the root llms.txt names each namesake, in order, before the index", async ({
  request,
}) => {
  const text = await (await request.get("/llms.txt")).text();
  const general = text.indexOf("Disambiguation:");
  const phala = text.indexOf(PHALA_DISAMBIGUATION_EN);
  const tgbyte = text.indexOf(TGBYTE_DISAMBIGUATION_EN);
  const aiNative = text.indexOf(AI_NATIVE_LANGUAGES_EN);
  expect(general).toBeGreaterThan(-1);
  expect(phala).toBeGreaterThan(general);
  expect(tgbyte).toBeGreaterThan(phala);
  expect(aiNative).toBeGreaterThan(tgbyte);
  expect(text.indexOf("## Project")).toBeGreaterThan(aiNative);
});

test("the root llms-full.txt says all three in its header", async ({
  request,
}) => {
  const text = await (await request.get("/llms-full.txt")).text();
  const header = text.slice(0, text.indexOf("\n---\n"));
  expect(header).toContain(PHALA_DISAMBIGUATION_EN);
  expect(header).toContain(TGBYTE_DISAMBIGUATION_EN);
  expect(header).toContain(AI_NATIVE_LANGUAGES_EN);
});

/** Every node of the site graph, whichever script it arrived in. */
function graphNodes(
  content: readonly string[],
): ReadonlyArray<Record<string, unknown>> {
  return content.flatMap((raw) => {
    const data = JSON.parse(raw) as { "@graph"?: Record<string, unknown>[] };
    return data["@graph"] ?? [];
  });
}

test("the structured data carries both application notes in one property", async ({
  page,
}) => {
  await page.goto("/");
  const nodes = graphNodes(
    await page.locator('script[type="application/ld+json"]').allTextContents(),
  );
  const software = nodes.find(
    (node) => node["@type"] === "SoftwareApplication",
  );
  expect(software?.["disambiguatingDescription"]).toBe(
    `${PHALA_DISAMBIGUATION_EN} ${TGBYTE_DISAMBIGUATION_EN}`,
  );
});

/**
 * And the languages as an entity of their own: what they are in
 * `description`, which project they are not in the property schema.org
 * keeps for that, and the page on this domain that argues for them.
 */
test("the structured data carries the languages as their own entity", async ({
  page,
}) => {
  await page.goto("/");
  const nodes = graphNodes(
    await page.locator('script[type="application/ld+json"]').allTextContents(),
  );
  const language = nodes.find((node) => node["@type"] === "ComputerLanguage");
  expect(language?.["name"]).toBe("AI Native Languages");
  expect(language?.["alternateName"]).toEqual([
    "AI Native Rust",
    "AI-Native Rust",
  ]);
  expect(language?.["url"]).toBe("https://vibevm.org/why/ai-native/");
  expect(language?.["description"]).toBe(AI_NATIVE_LANGUAGES_WHAT_EN);
  expect(language?.["disambiguatingDescription"]).toBe(
    AI_NATIVE_LANGUAGES_NOT_EN,
  );
});

/**
 * A namesake is named and never described, and never linked to.
 *
 * The rule the owner set: the words appear inside these sentences and
 * nowhere else, and no link on the page leads to either project. A link
 * would be this site recommending what it has just said it is not.
 */
for (const one of LANDINGS) {
  test(`${one.route} names the namesakes only in the small print`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const outside = await page.evaluate(() => {
      const notes = [...document.querySelectorAll(".landing-disambiguation")];
      const walker = document.createTreeWalker(
        document.body,
        NodeFilter.SHOW_TEXT,
      );
      const found: string[] = [];
      for (
        let node = walker.nextNode();
        node !== null;
        node = walker.nextNode()
      ) {
        const text = node.textContent ?? "";
        if (!/karanchawla|tgbyte/i.test(text)) continue;
        if (notes.some((note) => note.contains(node))) continue;
        /* What a reader READS, so the machine surfaces are passed over:
           the head's structured data says the same sentences on purpose,
           and the framework's serialised state carries a copy of them. */
        const owner = node.parentElement;
        if (owner === null) continue;
        if (owner.closest("script, style, template, pre, code") !== null) {
          continue;
        }
        found.push(text.trim());
      }
      return found;
    });
    expect(outside).toEqual([]);

    const hrefs = await page
      .locator("a[href]")
      .evaluateAll((links) =>
        links.map((link) => link.getAttribute("href") ?? ""),
      );
    expect(hrefs.filter((at) => /karanchawla|tgbyte/i.test(at))).toEqual([]);
  });
}
