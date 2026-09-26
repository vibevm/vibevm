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
 * kept and where it is spoken about; the second is the argument for it,
 * the essay first and the three products it is the worldview of after.
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

import { href } from "../lib/href.ts";
import { isNewsPath, newsHref } from "../news/paths.ts";
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

/** One destination — the same eight the header shows, by name. */
export type MenuId =
  | "documentation"
  | "github"
  | "gitverse"
  | "news"
  | "vision"
  | `why-${WhyPage}`;

/**
 * The size of an entry's plate on the map, in the twelve columns of its
 * row: a tower two rows high, a small square, a wide plate, a third of
 * the row, or the whole of it. Five shapes and not eight, because the
 * composition repeats them: the two mirrors are two small squares, the
 * three products three thirds.
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
  const vision: MenuEntry = {
    id: "vision",
    label: t.navVision,
    href: visionHref(locale),
    offSite: false,
    current: isVisionPath(here),
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
      { id: "tools", entries: [documentation, github, gitverse, news] },
      {
        id: "story",
        entries: [vision, ...WHY_PAGES.map((page) => whys[page])],
      },
    ],
    entries: {
      documentation,
      github,
      gitverse,
      news,
      vision,
      "why-vibevm": whys.vibevm,
      "why-zap": whys.zap,
      "why-ai-native": whys["ai-native"],
    },
  };
}
