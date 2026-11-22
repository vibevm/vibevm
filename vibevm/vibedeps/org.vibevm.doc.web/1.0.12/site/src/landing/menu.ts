/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The header's destinations, as one list that two things read.
 *
 * The header prints them small, in two rows; the map at the foot of the
 * landing prints the same list large, with a drawing for each (owner,
 * 2026-09-26: «there is a kind of reader who does not read the words at
 * the top»). Two renderings of one list, and the list is here — so a
 * destination added to the header is on the map the same moment, and
 * one that leaves the header leaves the map with it. Nothing about the
 * order is decided twice: the first row is the software, where it is
 * kept, where it is spoken about and which law the dealings with it fall
 * under; the second is the argument for it, the essay first and the
 * three products it is the worldview of after.
 *
 * What an entry carries is what both renderings need and nothing either
 * of them could disagree about: its label in the page's language, its
 * address, whether it leads off the domain, whether it is the page the
 * chrome is standing over — and the size of its plate on the map, which
 * is the one fact here that belongs to the map alone. It stands with the
 * entry rather than in the stylesheet because a plate that had to be
 * placed by hand for every new entry would put the composition back in
 * two files.
 *
 * The words the map adds — a line under each name, the drawing in words
 * — are the string table's (`i18n.ts`), keyed by the same ids, so the
 * compiler refuses an entry whose plate has nothing to say. The drawing
 * itself is `art.tsx`'s, keyed the same way and refused the same way.
 */

import { docHref, href } from "../lib/href.ts";
import { isNewsPath, newsHref } from "../news/paths.ts";
import { isRoadmapPath, roadmapHref } from "../roadmap/paths.ts";
import { isVisionPath, visionHref } from "../vision/paths.ts";
import { WHY_PAGES, type WhyPage, whyHref, whyPath } from "../why/paths.ts";
import {
  GITHUB_URL,
  GITVERSE_URL,
  type Locale,
  STRINGS,
  type Strings,
} from "./i18n.ts";

/** The two rows of the header, by what each is about. */
export type MenuRowId = "tools" | "story";

/** One destination — the same ten the header shows, by name. */
export type MenuId =
  | "documentation"
  | "github"
  | "gitverse"
  | "news"
  | "legal"
  | "vision"
  | "roadmap"
  | `why-${WhyPage}`;

/**
 * The size of an entry's plate on the map, in the twelve columns of its
 * row: a tower two rows high, a small square, a wide plate, a third of
 * the row, or the whole of it. Five shapes and not ten, because the
 * composition repeats them: the two mirrors are two small squares, the
 * three products three thirds, the essay and the roadmap two bands —
 * the second the mirror image of the first (`map.css`).
 *
 * The law page is the third band, and it is a band because the twelve
 * columns say so rather than because a fourth shape was wanted: the
 * tower, the two squares and the wide plate fill the first row's two
 * lines exactly, so the entry that joined them has a line of its own,
 * and a plate that took half of it would leave the other half a hole.
 */
export type Plate = "tall" | "small" | "wide" | "third" | "band";

export type MenuEntry = {
  readonly id: MenuId;
  /** The name the header and the map both print, in the page's language. */
  readonly label: string;
  readonly href: string;
  /** Leads off the domain; the link says so with `rel="noopener"`. */
  readonly offSite: boolean;
  /** The page the chrome is standing over is this one. */
  readonly current: boolean;
  readonly plate: Plate;
};

export type MenuRow = {
  readonly id: MenuRowId;
  readonly entries: readonly MenuEntry[];
};

export type LandingMenu = {
  /** The two rows, in the header's order. */
  readonly rows: readonly MenuRow[];
  /** The same entries by name, for whoever lists them in another order. */
  readonly entries: Readonly<Record<MenuId, MenuEntry>>;
};

/** The header's label for one of the three Why pages. */
function whyLabel(t: Strings, page: WhyPage): string {
  switch (page) {
    case "vibevm":
      return t.navWhyVibevm;
    case "zap":
      return t.navWhyZap;
    case "ai-native":
      return t.navWhyAiNative;
  }
}

/**
 * The destinations in one language, marked against the page the chrome
 * is standing over — `""` for the landing, `"why/zap/"` for a Why page,
 * in the form `pathWithinLocale` answers.
 */
export function landingMenu(locale: Locale, here: string): LandingMenu {
  const t = STRINGS[locale];

  const documentation: MenuEntry = {
    id: "documentation",
    label: t.documentation,
    href: href("doc/"),
    offSite: false,
    /* The manual wears its own chrome, so this entry is never the page
       the landing's chrome stands over. */
    current: false,
    plate: "tall",
  };
  const github: MenuEntry = {
    id: "github",
    label: "GitHub",
    href: GITHUB_URL,
    offSite: true,
    current: false,
    plate: "small",
  };
  const gitverse: MenuEntry = {
    id: "gitverse",
    label: "GitVerse",
    href: GITVERSE_URL,
    offSite: true,
    current: false,
    plate: "small",
  };
  const news: MenuEntry = {
    id: "news",
    label: t.navNews,
    href: newsHref(locale),
    offSite: false,
    current: isNewsPath(here),
    plate: "wide",
  };
  /* Right after the channels, by the owner's placement (2026-09-27), and
     in the first row because that is the row of the thing itself rather
     than of the argument for it: which law the dealings fall under is a
     fact about the software, beside where it is kept and where it is
     spoken about. The page is the manual's, in the reader's own edition,
     so the address is built from a coordinate like every other page of
     the manual and the entry is never the page the chrome stands over. */
  const legal: MenuEntry = {
    id: "legal",
    label: t.navLegal,
    href: docHref({
      lang: locale === "en" ? null : locale,
      group: "org.vibevm.core",
      name: "vibevm-docs",
      version: "latest",
      document: "legal/applicable-law",
    }),
    offSite: false,
    current: false,
    plate: "band",
  };
  const vision: MenuEntry = {
    id: "vision",
    label: t.navVision,
    href: visionHref(locale),
    offSite: false,
    current: isVisionPath(here),
    plate: "band",
  };
  /* Right after the essay, by the owner's placement (2026-09-26): the
     worldview, then where it is going, then the three products it is the
     worldview of. */
  const roadmap: MenuEntry = {
    id: "roadmap",
    label: t.navRoadmap,
    href: roadmapHref(locale),
    offSite: false,
    current: isRoadmapPath(here),
    plate: "band",
  };
  const why = (page: WhyPage): MenuEntry => ({
    id: `why-${page}`,
    label: whyLabel(t, page),
    href: whyHref(page, locale),
    offSite: false,
    current: here === whyPath(page),
    plate: "third",
  });
  /* Built once and read twice, so the row and the record hold the same
     objects rather than two spellings of them. */
  const whys: Readonly<Record<WhyPage, MenuEntry>> = {
    vibevm: why("vibevm"),
    zap: why("zap"),
    "ai-native": why("ai-native"),
  };

  return {
    rows: [
      { id: "tools", entries: [documentation, github, gitverse, news, legal] },
      {
        id: "story",
        entries: [vision, roadmap, ...WHY_PAGES.map((page) => whys[page])],
      },
    ],
    entries: {
      documentation,
      github,
      gitverse,
      news,
      legal,
      vision,
      roadmap,
      "why-vibevm": whys.vibevm,
      "why-zap": whys.zap,
      "why-ai-native": whys["ai-native"],
    },
  };
}
