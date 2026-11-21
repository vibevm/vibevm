/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

/**
 * What the roadmap tells a crawler it is: the `<title>`, the
 * `description`, and a `WebPage` whose main entity is the ordered list of
 * stages — beside its `llms.txt` line.
 *
 * They live beside the address map for the reason `VISION_META` does: a
 * route composes its head from one table, and nothing else about the
 * page's head is a decision the route takes.
 *
 * The structured data is the `WebPage` every subpage carries, with one
 * addition the page earns: an `ItemList` of the six stages in the
 * owner's order, because the order IS the content of this page and
 * schema.org has a vocabulary for exactly that. Each item carries the
 * stage's name and its one-sentence definition — the same words the page
 * prints — and the two stages that have an address of their own carry
 * it: Zap's page on this site, and the future home of Spec-Driven Linux.
 */

import { SITE } from "../config.ts";
import { PROJECT_NAME } from "../landing/identity.ts";
import {
  type Locale,
  STRINGS as LANDING,
  localePath,
} from "../landing/i18n.ts";
import { href } from "../lib/href.ts";
import { whyPath } from "../why/paths.ts";
import { STRINGS } from "./i18n.ts";
import { SPEC_DRIVEN_LINUX_URL, roadmapPath } from "./paths.ts";
import type { StageId } from "./strings.ts";

export type RoadmapMeta = {
  readonly title: string;
  readonly description: string;
};

export const ROADMAP_META: Readonly<Record<Locale, RoadmapMeta>> = {
  en: {
    title: "Roadmap — from a preview to an operating system",
    description:
      "Where VibeVM is going, in order: Developer Preview 2 with a more extensible core, Zap (the announced local workspace for coding agents), the first community packages in the registries, IDE plugins for IntelliJ IDEA and VS Code, a library and marketplace in the spirit of Steam, and Spec-Driven Linux, a Linux whose package manager is vibevm. No dates.",
  },
  ru: {
    title: "Дорожная карта — от превью до операционной системы",
    description:
      "Куда идёт VibeVM, по порядку: Developer Preview 2 с более расширяемым ядром, Zap (анонсированное локальное рабочее пространство для кодовых агентов), первые пакеты от сообщества в реестрах, плагины для IntelliJ IDEA и VS Code, библиотека и маркетплейс в духе Steam и Spec-Driven Linux — дистрибутив, в котором пакетный менеджер — vibevm. Без дат.",
  },
};

/** The absolute address of a page of this site, in one language. */
function absolute(locale: Locale, path: string): string {
  return `${SITE.origin}${href(`${localePath(locale)}${path}`)}`;
}

/**
 * The address a stage's list item carries, when the stage has one: the
 * same two addresses the page links to, so the graph and the page agree.
 */
export function stageUrl(id: StageId, locale: Locale): string | undefined {
  switch (id) {
    case "zap":
      return absolute(locale, whyPath("zap"));
    case "linux":
      return SPEC_DRIVEN_LINUX_URL;
    default:
      return undefined;
  }
}

/**
 * The `WebPage` graph of one edition, with the stages as its main entity.
 *
 * `inLanguage` and the addresses follow the edition; `isPartOf` names the
 * site the way every `WebPage` graph on the domain does. The list is
 * `ItemListOrderAscending` because that is the claim the page makes — the
 * first item comes first — and `numberOfItems` is derived from the tuple
 * rather than written, so the graph cannot disagree with the page about
 * how many stages there are.
 */
export function roadmapGraph(locale: Locale): string {
  const canonical = absolute(locale, roadmapPath());
  const meta = ROADMAP_META[locale];
  const t = STRINGS[locale];
  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "WebPage",
    name: meta.title,
    url: canonical,
    description: meta.description,
    inLanguage: LANDING[locale].htmlLang,
    isPartOf: { "@type": "WebSite", name: PROJECT_NAME, url: SITE.origin },
    mainEntity: {
      "@type": "ItemList",
      name: t.stagesH,
      itemListOrder: "https://schema.org/ItemListOrderAscending",
      numberOfItems: t.stages.length,
      itemListElement: t.stages.map((stage, index) => {
        const url = stageUrl(stage.id, locale);
        return {
          "@type": "ListItem",
          position: index + 1,
          name: stage.name,
          description: stage.sub,
          ...(url === undefined ? {} : { url }),
        };
      }),
    },
  });
}

/**
 * How the root `llms.txt` names the roadmap under «## Project» — the same
 * shape as the essay's and the Why pages' entries, for the same reader:
 * an agent deciding what to fetch, and here also learning the order of
 * what is planned without fetching anything. English only, as the file
 * is.
 */
export const ROADMAP_LLMS = {
  label: "Roadmap",
  gloss:
    "the six stages after Developer Preview 1, in order — Developer Preview 2 (a more flexible and extensible core), Zap (the announced local workspace for running coding agents in parallel), the first community packages in the registries, IDE plugins for IntelliJ IDEA and VS Code, a library and marketplace of VibeVM's own in the spirit of Steam, and Spec-Driven Linux (a Linux distribution whose package manager is vibevm; future home specdrivenlinux.org). No dates.",
} as const;

/** The `llms.txt` entry of the roadmap, against a given origin. */
export function roadmapLlmsLine(origin: string): string {
  return `- [${ROADMAP_LLMS.label}](${origin}/${roadmapPath()}): ${ROADMAP_LLMS.gloss}`;
}
