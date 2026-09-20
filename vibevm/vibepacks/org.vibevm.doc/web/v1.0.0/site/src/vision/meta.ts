/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

/**
 * What the essay tells a crawler it is: the `<title>`, the
 * `description`, and the `Article` graph — beside its `llms.txt` line.
 *
 * They live beside the address map for the reason `WHY_META` does: a
 * route composes its head from one table, and nothing else about the
 * page's head is a decision the route takes.
 *
 * The structured data is an `Article` rather than the `WebPage` every
 * other subpage carries, because that is what the page IS — an authored
 * essay with a headline and a position, not a product surface. The
 * fields are only what the site can honestly derive: the author is the
 * person the whole domain's footer names, the publisher is the site,
 * and there is no `datePublished` because the essay declares itself a
 * living text and the build has no publication date to swear to.
 */

import { SITE } from "../config.ts";
import { type Locale, STRINGS as LANDING } from "../landing/i18n.ts";
import { href } from "../lib/href.ts";
import { visionPath } from "./paths.ts";

export type VisionMeta = {
  readonly title: string;
  readonly description: string;
};

export const VISION_META: Readonly<Record<Locale, VisionMeta>> = {
  en: {
    title: "The Big Vision — an essay on the new technological order",
    description:
      "Intention no longer comes from humans alone. The VibeVM essay: AI as a second source of meaning, the specification/code dichotomy, traceable edges — and why everything that can be done without an LLM must be done without an LLM.",
  },
  ru: {
    title: "Большой Вижен — эссе о новом технологическом укладе",
    description:
      "Намерение больше не исходит только от человека. Эссе VibeVM: ИИ как второй источник смысла, дихотомия спецификации и кода, трассируемые рёбра — и почему всё, что можно сделать без LLM, должно быть сделано без LLM.",
  },
};

/**
 * The `Article` graph of one edition of the essay.
 *
 * The author is derived, not invented: the domain's footer signs every
 * page «© Олег Чирухин» / “Oleg Chirukhin”, and an essay is exactly the
 * page that claim is about. `inLanguage` and the addresses follow the
 * edition; `isPartOf` names the site the way every `WebPage` graph on
 * the domain does.
 */
export function visionArticleGraph(locale: Locale): string {
  const canonical = `${SITE.origin}${href(
    `${locale === "en" ? "" : `${locale}/`}${visionPath()}`,
  )}`;
  const meta = VISION_META[locale];
  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "Article",
    headline: locale === "ru" ? "Большой Вижен" : "The Big Vision",
    name: meta.title,
    description: meta.description,
    url: canonical,
    mainEntityOfPage: canonical,
    inLanguage: LANDING[locale].htmlLang,
    author: { "@type": "Person", name: "Oleg Chirukhin" },
    publisher: { "@type": "Organization", name: "VibeVM", url: SITE.origin },
    image: `${SITE.origin}${href("og.png")}`,
    isPartOf: { "@type": "WebSite", name: "VibeVM", url: SITE.origin },
  });
}

/**
 * How the root `llms.txt` names the essay under «## Project» — the same
 * shape as the three Why entries, for the same reader: an agent deciding
 * what to fetch. English only, as the file is.
 */
export const VISION_LLMS = {
  label: "The Big Vision",
  gloss:
    "the worldview essay — AI as a second source of intention, the specification/code dichotomy, traceable and checkable edges, and the rule that everything that can be done without an LLM is done without an LLM.",
} as const;

/** The `llms.txt` entry of the essay, against a given origin. */
export function visionLlmsLine(origin: string): string {
  return `- [${VISION_LLMS.label}](${origin}/${visionPath()}): ${VISION_LLMS.gloss}`;
}
