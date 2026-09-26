/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * Which VibeVM this is not, everywhere a crawler reads it.
 *
 * Search engines and model crawlers had been joining this project with
 * Phala Network's VibeVM, a development sandbox on Phala Cloud (owner,
 * 2026-09-26). The site now says so in four places, in one sentence: at
 * the foot of both landings, in the root `llms.txt` and `llms-full.txt`,
 * and as `disambiguatingDescription` in the structured data. A sentence
 * that quietly went missing from one of them would be the index's cue to
 * join the two again, so each place is held here.
 */

import { expect, test } from "@playwright/test";

import { PHALA_DISAMBIGUATION_EN } from "../src/landing/i18n.ts";

const LANDINGS = [
  { route: "/", says: "Phala Network's VibeVM" },
  { route: "/ru/", says: "VibeVM от Phala Network" },
] as const;

for (const one of LANDINGS) {
  test(`${one.route} ends with which VibeVM it is not`, async ({ page }) => {
    await page.goto(one.route);
    const note = page.locator("main .landing-disambiguation");
    await expect(note).toBeVisible();
    await expect(note).toContainText(one.says);
    await expect(note).toContainText("github.com/Phala-Network/VibeVM");
    await expect(note).toContainText("Phala Cloud");

    /* Last in the page's own content: small print after everything the
       page says, not a line inside it. */
    const last = page.locator("main > :last-child");
    await expect(last).toHaveClass(/\blanding-disambiguation\b/);
  });
}

test("the root llms.txt names Phala's VibeVM right after the disambiguation", async ({
  request,
}) => {
  const text = await (await request.get("/llms.txt")).text();
  const general = text.indexOf("Disambiguation:");
  const phala = text.indexOf(PHALA_DISAMBIGUATION_EN);
  expect(general).toBeGreaterThan(-1);
  expect(phala).toBeGreaterThan(general);
  expect(text.indexOf("## Project")).toBeGreaterThan(phala);
});

test("the root llms-full.txt says it in its header", async ({ request }) => {
  const text = await (await request.get("/llms-full.txt")).text();
  const header = text.slice(0, text.indexOf("\n---\n"));
  expect(header).toContain(PHALA_DISAMBIGUATION_EN);
});

test("the structured data carries it as disambiguatingDescription", async ({
  page,
}) => {
  await page.goto("/");
  const graphs = await page
    .locator('script[type="application/ld+json"]')
    .allTextContents();
  const software = graphs
    .flatMap((raw) => {
      const data = JSON.parse(raw) as { "@graph"?: Record<string, unknown>[] };
      return data["@graph"] ?? [];
    })
    .find((node) => node["@type"] === "SoftwareApplication");
  expect(software?.["disambiguatingDescription"]).toBe(PHALA_DISAMBIGUATION_EN);
});
