/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The front door's hero: the release it is showing, the one way on beside
 * it, and — since 2026-09-27 — what the project is properly called.
 *
 * The pill said «Early Access» and stood alone in a row with room to
 * spare; it now names the release — «Developer Preview 1», one name in
 * both languages, because a release name is not translated — and carries
 * beside it the link a reader who has just met the product wants: what
 * this thing is, in the manual, in the language they are reading (owner,
 * 2026-09-26).
 *
 * The address is asserted per language, because that is the half a test
 * is for: a button that led to the English page from the Russian landing
 * would look right on both.
 */

import { expect, test } from "@playwright/test";

import { ANARCHIC_URL } from "../src/landing/identity.ts";
import { STRINGS as LANDING } from "../src/landing/i18n.ts";
import { CREATOR_SITE } from "../src/lib/people.ts";

const CMD_INSTALL =
  'curl.exe -fsSL https://vibevm.org/install.cmd -o "%TEMP%\\vibevm-install.cmd" && call "%TEMP%\\vibevm-install.cmd"';
const POWERSHELL_INSTALL = "irm https://vibevm.org/install.ps1 | iex";

const PAGES = [
  {
    route: "/",
    locale: "en",
    badge: "Developer Preview 1",
    roadmap: "/roadmap/",
    label: "What is VibeVM?",
    href: "/doc/org.vibevm.core/vibevm-docs/latest/start/what-vibevm-is/",
    manual: "/doc/org.vibevm.core/vibevm-docs/1.0.0/start/install-vibe/#p09",
  },
  {
    route: "/ru/",
    locale: "ru",
    badge: "Developer Preview 1",
    roadmap: "/ru/roadmap/",
    label: "Что такое VibeVM?",
    href: "/doc/ru/org.vibevm.core/vibevm-docs/latest/start/what-vibevm-is/",
    manual: "/doc/ru/org.vibevm.core/vibevm-docs/1.0.0/start/install-vibe/#p09",
  },
] as const;

for (const one of PAGES) {
  test(`${one.route} keeps optional install paths collapsed`, async ({
    page,
  }) => {
    await page.goto(one.route);

    const windows = page.locator('[data-install-drawer="windows"]');
    const releases = page.locator('[data-install-drawer="releases"]');
    await expect(windows).not.toHaveAttribute("open", "");
    await expect(releases).not.toHaveAttribute("open", "");
    await expect(
      page.getByText(LANDING[one.locale].installBash, { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText(POWERSHELL_INSTALL, { exact: true }),
    ).toBeHidden();
    await expect(page.getByText(CMD_INSTALL, { exact: true })).toBeHidden();
    await expect(
      page.getByRole("link", { name: LANDING[one.locale].installManual }),
    ).toBeHidden();
    await expect(
      page.getByRole("link", {
        name: LANDING[one.locale].installBinaryReleases,
      }),
    ).toBeHidden();
  });

  test(`${one.route} offers the exact cmd.exe installer command`, async ({
    baseURL,
    context,
    page,
  }) => {
    if (baseURL === undefined)
      throw new Error("Playwright baseURL is required");
    await context.grantPermissions(["clipboard-read", "clipboard-write"], {
      origin: new URL(baseURL).origin,
    });
    await page.goto(one.route);

    await page.locator('[data-install-drawer="windows"] summary').click();
    await expect(
      page.getByText(POWERSHELL_INSTALL, { exact: true }),
    ).toBeVisible();
    const command = page.getByText(CMD_INSTALL, { exact: true });
    await expect(command).toBeVisible();
    const line = command.locator(
      "xpath=ancestor::*[contains(@class, 'install__command')][1]",
    );
    await expect(
      line.getByText("Windows cmd.exe", { exact: true }),
    ).toHaveCount(1);
    await expect(line.getByText(">", { exact: true })).toHaveCount(1);

    await line
      .getByRole("button", { name: LANDING[one.locale].copyCommand })
      .click();
    await expect
      .poll(() => page.evaluate(() => navigator.clipboard.readText()))
      .toBe(CMD_INSTALL);
  });

  test(`${one.route} opens its localized install links`, async ({ page }) => {
    await page.goto(one.route);

    const windows = page.locator('[data-install-drawer="windows"]');
    await windows.locator("summary").click();
    await expect(windows).toHaveAttribute("open", "");
    await expect(
      windows.getByRole("link", {
        name: LANDING[one.locale].installManual,
      }),
    ).toHaveAttribute("href", one.manual);

    const releases = page.locator('[data-install-drawer="releases"]');
    await releases.locator("summary").click();
    await expect(releases).toHaveAttribute("open", "");
    await expect(
      releases.getByRole("link", {
        name: LANDING[one.locale].installBinaryReleases,
      }),
    ).toHaveAttribute("href", "https://github.com/vibevm/vibevm/releases");
    await expect(
      releases.getByRole("link", {
        name: LANDING[one.locale].installSourceGitHub,
      }),
    ).toHaveAttribute("href", "https://github.com/vibevm/vibevm");
    await expect(
      releases.getByRole("link", {
        name: LANDING[one.locale].installSourceGitVerse,
      }),
    ).toHaveAttribute("href", "https://gitverse.ru/vibevm/vibevm");
  });

  test(`${one.route} exposes native drawer keyboard focus`, async ({
    page,
  }) => {
    await page.goto(one.route);
    const drawer = page.locator('[data-install-drawer="windows"]');
    const summary = drawer.locator("summary");

    await summary.focus();
    await expect(summary).toBeFocused();
    expect(
      await summary.evaluate(
        (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
      ),
    ).not.toBe("0px");
    await page.keyboard.press("Enter");
    await expect(drawer).toHaveAttribute("open", "");
  });

  test(`${one.route} styles long command scrollbars as part of the panel`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(one.route);
    const windows = page.locator('[data-install-drawer="windows"]');
    await windows.locator("summary").click();
    const line = windows.locator(".install__line").last();

    const styles = await line.evaluate((element) => ({
      firefoxWidth: getComputedStyle(element).scrollbarWidth,
      webkitHeight: getComputedStyle(element, "::-webkit-scrollbar").height,
      buttonDisplay: getComputedStyle(element, "::-webkit-scrollbar-button")
        .display,
      track: getComputedStyle(element, "::-webkit-scrollbar-track")
        .backgroundColor,
    }));
    expect(styles).toEqual({
      firefoxWidth: "thin",
      webkitHeight: "6px",
      buttonDisplay: "none",
      track: "rgba(0, 0, 0, 0)",
    });
  });
}

for (const one of PAGES) {
  test(`${one.route} names the release on its pill`, async ({ page }) => {
    await page.goto(one.route);
    const pill = page.locator("main .hero__release");
    await expect(pill).toHaveText(one.badge);
    /* The dot stays: it is what makes the pill a pill. */
    await expect(pill.locator(".hero__release-dot")).toHaveCount(1);
  });
}

/**
 * And the pill leads to the plan the release is a point on (owner,
 * 2026-09-26), in the reader's own language.
 *
 * The ADDRESS is what is held here and not the page behind it: that page
 * is written on another branch and lands after this, and a test that
 * fetched it would be red about somebody else's work in progress.
 */
for (const one of PAGES) {
  test(`${one.route} sends the release to the roadmap`, async ({ page }) => {
    await page.goto(one.route);
    const pill = page.locator("main [data-hero-release]");
    await expect(pill).toHaveCount(1);
    await expect(pill).toHaveAttribute("href", one.roadmap);
    await expect(pill).toHaveClass(/hero__release/);

    /* A keyboard can see it, which is the half of «it is a control» that
       a look at the page does not prove. */
    await pill.focus();
    await expect(pill).toBeFocused();
    const outline = await pill.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

for (const one of PAGES) {
  test(`${one.route} offers the manual's answer beside it`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(one.route);

    const guide = page.locator("main .hero__run [data-hero-guide]");
    await expect(guide).toHaveCount(1);
    await expect(guide).toContainText(one.label);
    await expect(guide).toHaveAttribute("href", one.href);

    /* In the same row as the pill, and level with it: one line, two
       things, neither above the other. */
    const level = await page.evaluate(() => {
      const pill = document.querySelector(".hero__release");
      const link = document.querySelector("[data-hero-guide]");
      if (pill === null || link === null) return null;
      const one = pill.getBoundingClientRect();
      const other = link.getBoundingClientRect();
      return {
        beside: other.left > one.right,
        apart: Math.abs(
          one.top + one.height / 2 - (other.top + other.height / 2),
        ),
      };
    });
    expect(level?.beside).toBe(true);
    expect(level?.apart ?? 99).toBeLessThan(4);

    /* A keyboard can see where it is. */
    await guide.focus();
    await expect(guide).toBeFocused();
    const outline = await guide.evaluate(
      (element) => getComputedStyle(element, ":focus-visible").outlineWidth,
    );
    expect(outline).not.toBe("0px");
  });
}

/**
 * The project's full name, where the owner put it: the note directly under
 * the lead, and the lead's own first words (owner, 2026-09-27).
 *
 * The sentence is compared whole, with its markup taken off, because it is
 * the owner's sentence and the page is where it is read — and the two
 * addresses inside it are compared against the tables that own them, not
 * against a spelling repeated here. A note whose words survived while its
 * link went to the wrong place would look finished from the outside.
 */
const NAMED = [
  {
    route: "/",
    locale: "en",
    note: "The correct full name of the project is Anarchic VibeVM. It was originally conceived and built by Oleg Chirukhin as a way to make personal and group vibe coding simpler.",
    person: "Oleg Chirukhin",
    lead: "Anarchic VibeVM is an ultimate prompt library, package manager, and agentic system for Spec-Driven Development — declarative context assembled from versioned stacks, flows, and skills.",
  },
  {
    route: "/ru/",
    locale: "ru",
    note: "Правильное полное название проекта — Anarchic VibeVM. Изначально её придумал и реализовал Олег Чирухин как средство для упрощения персонального и группового вайбкодинга.",
    person: "Олег Чирухин",
    lead: "Anarchic VibeVM — ультимативная библиотека промптов, пакетный менеджер и агентная система для Spec-Driven Development: декларативный контекст, собранный из версионируемых стеков, флоу и навыков.",
  },
] as const;

for (const one of NAMED) {
  test(`${one.route} moves the proper-name note into the small print`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(one.route);

    await expect(page.locator("main .hero__note")).toHaveCount(0);
    const notes = page.locator("main .landing-disambiguation");
    await expect(notes).toHaveCount(4);
    const note = notes.first();
    await expect(note).toHaveCount(1);
    expect((await note.innerText()).replace(/\s+/g, " ").trim()).toBe(one.note);

    /* The brand is a place, and so is the person. */
    await expect(note.locator(`a[href="${ANARCHIC_URL}"]`)).toHaveText(
      "Anarchic",
    );
    await expect(note.locator(`a[href="${CREATOR_SITE}"]`)).toHaveText(
      one.person,
    );
    await expect(note.locator("a")).toHaveCount(2);

    /* Immediately before the Phala disambiguation, exactly where the
       owner moved it — not merely somewhere else near the page foot. */
    await expect(notes.nth(1)).toContainText(/Phala/i);
    expect(
      await note.evaluate(
        (element) => element.nextElementSibling?.textContent ?? "",
      ),
    ).toMatch(/Phala/i);
  });
}

for (const one of NAMED) {
  test(`${one.route} opens its lead with the full name`, async ({ page }) => {
    await page.goto(one.route);
    const lead = page.locator("main .hero__lead");
    expect((await lead.innerText()).replace(/\s+/g, " ").trim()).toBe(one.lead);

    /* The word in the name that is also a place, and the descriptor that
       must appear verbatim — one `<a>`, one `<strong>`, and the descriptor
       is the string table's own. */
    await expect(lead.locator(`a[href="${ANARCHIC_URL}"]`)).toHaveText(
      "Anarchic",
    );
    await expect(lead.locator("a")).toHaveCount(1);
    const strong = await lead.locator("strong").innerText();
    expect(LANDING[one.locale].leadHtml).toContain(
      `<strong>${strong}</strong>`,
    );
  });
}

/** On a phone the pair wraps instead of shrinking, and nothing overflows. */
for (const one of PAGES) {
  test(`${one.route} wraps the pair on a phone`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(one.route);

    await page.locator('[data-install-drawer="windows"] summary').click();
    await page.locator('[data-install-drawer="releases"] summary').click();

    const under = await page.evaluate(() => {
      const pill = document.querySelector(".hero__release");
      const link = document.querySelector("[data-hero-guide]");
      if (pill === null || link === null) return null;
      return (
        link.getBoundingClientRect().top >= pill.getBoundingClientRect().bottom
      );
    });
    expect(under).toBe(true);
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth >
          document.documentElement.clientWidth,
      ),
    ).toBe(false);
  });
}
