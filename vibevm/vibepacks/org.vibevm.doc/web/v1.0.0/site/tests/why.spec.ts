/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The three Why pages, in both languages: that they are served, that they
 * say who they are to a crawler, that the two letters in the corner keep
 * a reader where they are, that a reader who asked for stillness gets it,
 * and that none of the six scrolls sideways at any width.
 *
 * All of it is measured in a browser over the BUILT bytes, for the reason
 * every other run in this directory is: these are questions about a live
 * document — what `document.getAnimations()` actually returns, what the
 * document's scroll width actually is — and a component rendered in
 * isolation cannot answer them.
 */

import { expect, type Page, test } from "@playwright/test";

/** The six addresses, and what each one must say it is. */
const PAGES = [
  {
    route: "/why/vibevm/",
    twin: "/ru/why/vibevm/",
    language: "en",
    heading: "Discipline you can install.",
    title: "Why VibeVM — discipline you can install",
    nav: "Why VibeVM",
  },
  {
    route: "/ru/why/vibevm/",
    twin: "/why/vibevm/",
    language: "ru",
    heading: "Дисциплина, которую можно установить.",
    title: "Почему VibeVM — дисциплина, которую можно установить",
    nav: "Почему VibeVM",
  },
  {
    route: "/why/zap/",
    twin: "/ru/why/zap/",
    language: "en",
    heading: "Your agents, on one map.",
    title: "Why Zap — your coding agents, on one map",
    nav: "Why Zap",
  },
  {
    route: "/ru/why/zap/",
    twin: "/why/zap/",
    language: "ru",
    heading: "Все агенты — на одной карте.",
    title: "Почему Zap — ваши кодовые агенты на одной карте",
    nav: "Почему Zap",
  },
  {
    route: "/why/ai-native/",
    twin: "/ru/why/ai-native/",
    language: "en",
    heading: "Code that AI agents actually understand.",
    title: "AI-Native Language — code AI agents actually understand",
    nav: "AI-Native Language",
  },
  {
    route: "/ru/why/ai-native/",
    twin: "/why/ai-native/",
    language: "ru",
    heading: "Код, который по-настоящему понятен AI-агентам.",
    title: "AI-Native Языки — код, по-настоящему понятный AI-агентам",
    nav: "AI-Native Языки",
  },
] as const;

const WIDTHS = [1440, 834, 390] as const;

/** Whether the document is wider than the window a reader is holding. */
async function scrolls(page: Page): Promise<boolean> {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
}

/**
 * Every page is served, and the first thing on it is its own headline.
 *
 * The heading is compared with its whitespace collapsed because two of
 * the three carry an `<em>` inside the sentence and one carries a
 * non-breaking space: what is being checked is the sentence, not how the
 * markup broke it up.
 */
for (const one of PAGES) {
  test(`${one.route} is served and opens with its own headline`, async ({
    page,
  }) => {
    const response = await page.goto(one.route);
    expect(response?.status()).toBe(200);

    const heading = page.locator("h1");
    await expect(heading).toHaveCount(1);
    const text = (await heading.innerText()).replace(/\s+/g, " ").trim();
    expect(text).toBe(one.heading);

    await expect(page).toHaveTitle(one.title);
    expect(await page.getAttribute("html", "lang")).toBe(one.language);
  });
}

/**
 * The Zap page announces its release before anything else.
 *
 * The banner is the first thing inside the page — above the hero, under
 * the site's header — and it names the same month in both languages
 * twice: once in words for a reader, once in the form a machine reads,
 * `<time datetime>`. It is a paragraph and not a heading, which is what
 * keeps the outline test below true: the page's first heading is still
 * its headline.
 */
const RELEASE = [
  { route: "/why/zap/", label: "Coming soon", date: "October 2026" },
  { route: "/ru/why/zap/", label: "Скоро", date: "Октябрь 2026" },
] as const;

for (const one of RELEASE) {
  test(`${one.route} opens with its release banner`, async ({ page }) => {
    await page.goto(one.route);
    const first = page.locator("main .why-page > :first-child");
    await expect(first).toHaveClass(/\bwz-soon\b/);

    await expect(first.locator(".wz-soon__label")).toHaveText(one.label);
    const month = first.locator("time");
    await expect(month).toHaveText(one.date);
    await expect(month).toHaveAttribute("datetime", "2026-10");

    await expect(first.locator("h1, h2, h3, h4, h5, h6")).toHaveCount(0);
  });

  /* Zap is not working software yet, so the page is an announcement and
     tells nobody how to install it (owner, 2026-09-25): no command a
     reader could run, and no button down to the start section, which is
     commented out until the release. */
  test(`${one.route} announces Zap and does not install it`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const main = page.locator("main");
    await expect(main.locator("#zap-start")).toHaveCount(0);
    await expect(main.locator("a[href='#zap-start']")).toHaveCount(0);
    await expect(main.locator(".why-cmd, .wz-steps")).toHaveCount(0);
    expect(await main.innerText()).not.toMatch(/vibe install|zap-quicklens/);
  });
}

/**
 * What each page tells a crawler about itself.
 *
 * `canonical` names this address and not the landing; the `hreflang` set
 * names both spellings of THIS page and an `x-default`; and the
 * structured data is a `WebPage` that says which site it is part of —
 * the site graph belongs to the root and is published there once.
 */
for (const one of PAGES) {
  test(`${one.route} declares its own address`, async ({ page }) => {
    await page.goto(one.route);
    const origin = new URL(page.url()).origin;

    const canonical = await page.getAttribute("link[rel=canonical]", "href");
    expect(canonical).toBe(`https://vibevm.org${one.route}`);

    const alternates = await page
      .locator("link[rel=alternate][hreflang]")
      .evaluateAll((links) =>
        links
          .map(
            (link) =>
              `${link.getAttribute("hreflang")} ${link.getAttribute("href")}`,
          )
          .sort(),
      );
    const english = one.language === "en" ? one.route : one.twin;
    const russian = one.language === "ru" ? one.route : one.twin;
    expect(alternates).toEqual([
      `en https://vibevm.org${english}`,
      `ru https://vibevm.org${russian}`,
      `x-default https://vibevm.org${english}`,
    ]);

    const ogUrl = await page.getAttribute('meta[property="og:url"]', "content");
    expect(ogUrl).toBe(`https://vibevm.org${one.route}`);
    const description = await page.getAttribute(
      'meta[name="description"]',
      "content",
    );
    expect(description).toBeTruthy();

    const graph = await page
      .locator('script[type="application/ld+json"]')
      .first()
      .textContent();
    const parsed = JSON.parse(graph ?? "{}");
    expect(parsed["@type"]).toBe("WebPage");
    expect(parsed.url).toBe(`https://vibevm.org${one.route}`);
    expect(parsed.inLanguage).toBe(one.language);
    expect(parsed.isPartOf?.name).toBe("VibeVM");
    /* The page is served from a loopback origin and still names the live
       domain: `canonical` is absolute by specification, and the origin
       comes from the build environment rather than from the request. */
    expect(origin).not.toBe("https://vibevm.org");
  });
}

/**
 * The two letters in the corner keep a reader where they are.
 *
 * This is the whole reason the chrome reads the address instead of
 * taking a prop: a reader three sections into the Russian Zap page who
 * wants it in English wants THAT page in English, not the front door.
 */
for (const one of PAGES) {
  test(`${one.route} offers its twin, not the front door`, async ({ page }) => {
    await page.goto(one.route);
    const other = one.language === "en" ? "ru" : "en";
    const link = page.locator(
      `[data-site-language] [data-site-lang-choice="${other}"]`,
    );
    await expect(link).toHaveAttribute("href", one.twin);

    await link.click();
    await page.waitForURL(`**${one.twin}`);
    expect(new URL(page.url()).pathname).toBe(one.twin);
  });
}

/**
 * The header says which of the three pages a reader is standing on.
 *
 * Scoped to the nav entries rather than to every link in the bar,
 * because two things in that bar are legitimately current at once: the
 * page's own entry, and the language the page is written in — the
 * switch marks its current choice with the same attribute, and both
 * statements are true.
 */
for (const one of PAGES) {
  test(`${one.route} marks itself current in the header`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const current = page.locator(
      'header .landing-nav__link[aria-current="page"]',
    );
    await expect(current).toHaveCount(1);
    await expect(current).toHaveText(one.nav);
    await expect(current).toHaveAttribute("href", one.route);

    /* And the language the page is in is the one marked in the corner. */
    const language = page.locator(
      `[data-site-language] [data-site-lang-choice="${one.language}"]`,
    );
    await expect(language).toHaveAttribute("aria-current", "page");
  });
}

/**
 * No page scrolls sideways, at any of the three widths.
 *
 * Two of the three ride a drawing past the edge of the window on
 * purpose, and one prints command lines that cannot be broken. Both are
 * exactly the shapes that turn into a horizontal scrollbar when a
 * minimum size is forgotten, and a phone is where it is noticed last.
 */
for (const one of PAGES) {
  for (const width of WIDTHS) {
    test(`${one.route} does not scroll sideways at ${width}px`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(one.route);
      await page.evaluate(() => document.fonts.ready);
      expect(await scrolls(page)).toBe(false);
    });
  }
}

/**
 * A reader who asked for stillness gets it.
 *
 * Measured on the animations the document actually has rather than on
 * the stylesheet: `prefers-reduced-motion` is answered in three places —
 * the design system's blanket rule, each page's own motion-safety block,
 * and the entry animations that hold their end state — and what matters
 * is the total. Every animation still attached to the document must be
 * one the reader cannot see moving.
 */
for (const one of PAGES) {
  test(`${one.route} stops its decoration for a reader who asked`, async ({
    browser,
  }) => {
    /* An explicit context rather than `test.use`: the preference has to
       be on the context the page is created in, and a page that was
       created without it reports the animations of a reader who never
       asked — which is how this test first passed the wrong thing. */
    const context = await browser.newContext({ reducedMotion: "reduce" });
    const page = await context.newPage();
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);

    /* Nothing at all, and that is what the blanket rule produces: with
       every duration cut to a thousandth of a millisecond and every
       iteration count to one, each animation finishes on the frame it
       started and the document is left holding none. */
    const running = await page.evaluate(() => document.getAnimations().length);
    expect(running).toBe(0);

    /* And the entry animations left their subjects visible rather than
       at the opacity they start from: a drawing that animates in from
       nothing must be THERE for a reader who turned the animation off. */
    const invisible = await page.evaluate(
      () =>
        [...document.querySelectorAll("main svg *")].filter((element) => {
          const box = element.getBoundingClientRect();
          if (box.width < 2 || box.height < 2) return false;
          return Number(getComputedStyle(element).opacity) === 0;
        }).length,
    );
    /* The one thing that is meant to be invisible when still is the
       expanding ping on the Zap page: it IS its motion, and a frozen
       ring around a node would read as a second node. */
    expect(invisible).toBeLessThanOrEqual(2);

    await context.close();
  });
}

/**
 * With motion allowed, the decoration is there.
 *
 * The negative half of the test above: a page that carried no animation
 * at all would pass the reduced-motion check for the wrong reason, and
 * the two drawings that draw themselves are half of what these pages
 * say.
 */
for (const one of PAGES) {
  test(`${one.route} animates when motion is allowed`, async ({ page }) => {
    await page.goto(one.route);
    await page.evaluate(() => document.fonts.ready);
    const count = await page.evaluate(() => document.getAnimations().length);
    expect(count).toBeGreaterThan(0);
  });
}

/**
 * The landmarks and the heading order.
 *
 * One `main`, one `h1`, and no level skipped on the way down: a reader
 * moving by heading must not fall from a section title into a card title
 * two levels below it. The pages are long — the AI-Native one carries
 * five mechanisms with their own sub-headings — which is exactly where
 * an order goes wrong without anyone seeing it.
 */
for (const one of PAGES) {
  test(`${one.route} has one main and an unbroken heading order`, async ({
    page,
  }) => {
    await page.goto(one.route);
    await expect(page.locator("main")).toHaveCount(1);
    await expect(page.locator("header")).toHaveCount(1);
    await expect(page.locator("footer")).toHaveCount(1);

    const levels = await page
      .locator("main h1, main h2, main h3, main h4")
      .evaluateAll((headings) =>
        headings.map((heading) => Number(heading.tagName.slice(1))),
      );
    expect(levels[0]).toBe(1);
    expect(levels.filter((level) => level === 1)).toHaveLength(1);
    for (let i = 1; i < levels.length; i += 1) {
      expect(levels[i] - levels[i - 1]).toBeLessThanOrEqual(1);
    }
  });
}

/**
 * Every section that names itself is announced by the name it uses.
 *
 * `aria-labelledby` pointing at an id that is not on the page is worse
 * than no label: a screen reader announces an unlabelled region and
 * nothing says the reference was broken.
 */
for (const one of PAGES) {
  test(`${one.route} labels every section with an id that exists`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const broken = await page.evaluate(() =>
      [...document.querySelectorAll("main [aria-labelledby]")]
        .map((element) => element.getAttribute("aria-labelledby") ?? "")
        .filter((id) => document.getElementById(id) === null),
    );
    expect(broken).toEqual([]);
  });
}

/**
 * The keyboard reaches the page.
 *
 * Tabbing from the top must arrive at the page's own first action — the
 * first button under the hero — after the chrome's controls, and the
 * element that has focus must be one a reader can see has it. What is
 * checked is that focus lands on the link and that the ring is drawn,
 * which is the design system's one rule about focus and the only one a
 * page can break by removing an outline.
 *
 * The action is found by where it stands, not by where it leads: on the
 * Zap page, whose start section waits for the release, the first button
 * leads to another page rather than down this one.
 */
for (const one of PAGES) {
  test(`${one.route} gives its first action a visible focus ring`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(one.route);
    const action = page.locator("main .why-cta a").first();
    await action.focus();
    await expect(action).toBeFocused();
    const outline = await action.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

/**
 * The drawings are drawings, and they are not announced as pictures
 * twice.
 *
 * Each figure carries the description; the `svg` inside it is
 * `aria-hidden` and out of the tab order, so a reader using a screen
 * reader is told what the picture shows once instead of being walked
 * through a list of circles.
 */
for (const one of PAGES) {
  test(`${one.route} describes each drawing once`, async ({ page }) => {
    await page.goto(one.route);
    const figures = page.locator('main [role="img"][aria-label]');
    expect(await figures.count()).toBeGreaterThan(0);
    const exposed = await page.evaluate(
      () =>
        [...document.querySelectorAll("main svg")].filter(
          (svg) => svg.getAttribute("aria-hidden") !== "true",
        ).length,
    );
    expect(exposed).toBe(0);
  });
}

/**
 * Nothing of the framework this site replaced is in the output.
 *
 * The one check that is about the migration rather than about the pages:
 * an `_astro` asset, or a module the other framework's runtime would
 * have loaded, means the build is serving two sites.
 */
for (const one of PAGES) {
  test(`${one.route} ships no trace of the previous framework`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const html = await page.content();
    expect(html).not.toContain("_astro");
    expect(html).not.toContain("astro-island");
    expect(html).not.toContain("data-astro");
  });
}
