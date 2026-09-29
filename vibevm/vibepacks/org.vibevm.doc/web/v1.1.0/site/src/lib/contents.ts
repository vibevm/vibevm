/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-SHELL-PARSES-NOTHING */

/**
 * The manual's own pages, as the column beside the text lists them.
 *
 * It is a file of its own and not a function in `view.ts` for the reason
 * that file gives about itself: `view.ts` answers what one ADDRESS
 * means, and this answers what one DOCUMENTATION looks like from the
 * inside — the same answer on every page of it, and on the
 * documentation's own front page, which has no document in its address
 * at all.
 *
 * Four rules and nothing else shapes what comes out.
 *
 * **The order is the manifest's.** The pages arrive in the order the
 * layer law gave them — text that stands still before text that moves
 * with the product — and nothing here sorts, groups alphabetically or
 * moves a page for looking better. A navigation is a small correction to
 * a list the manifest already holds, never a second table of contents
 * beside it (`##NAV-PINNED`).
 *
 * **The list is the SOURCE's pages, in every language.** An adaptation
 * in progress is not a smaller manual: every page exists at every
 * language's address, and one an adaptation has not reached is served in
 * the source's words with a notice. A column built from the chosen
 * edition's own manifest would silently hide the pages a reader most
 * needs to be told about.
 *
 * **A section's name is the reader's edition's, its identity is the
 * source's.** The manifest keys a section by the folder the pages live
 * in, which is the same in every language, and carries the title in the
 * package's own words — so a Russian adaptation names the same sections
 * as its source while showing other words. A folder no manifest names is
 * shown under its own directory name, which is a fallback and never a
 * translation.
 *
 * **A page's name is the reader's edition's for the same reason.** The
 * list answers which pages exist; the words on it answer what each one
 * is called where the reader is standing, and those are two questions.
 * A column built entirely from the source's titles read «Every block
 * once» over a Russian page whose own card on the package page calls it
 * «Каждый блок по разу» — the site disagreeing with itself about the
 * name of one page, twice on the same screen. The source's words stand
 * in only where the adaptation has not reached the page, which is
 * exactly the text that address will serve: the link then says what
 * opening it will show, which is the whole job of a label.
 *
 * **And there are two orders, for two audiences.** The four rules above
 * are about the list the manifest gives, which is the layer law's: text
 * that stands still before text that moves, written for an agent's
 * prompt cache. A documentation MAY also declare a learning path — the
 * chapters a person reads it in — and where it has, the column opens on
 * the path and offers the folders beside it (`##NAV-CHAPTERS`,
 * `##NAV-CHAPTERS-READER`). Neither order is computed here and neither
 * moves the other: the path is a list of documents the author wrote
 * down, and the same rules about names, editions and fallbacks govern
 * both views of it.
 */

import { docHref } from "./href.ts";
import {
  addressOf,
  documentOf,
  editions,
  resolvePage,
  sourceEdition,
  type Edition,
  type Library,
} from "./library.ts";

import type { NavigationChapter } from "../generated/doc-manifest.ts";

import type {
  ContentsChapter,
  ContentsItem,
  ContentsSection,
  PagerChapter,
  PagerLink,
} from "@vibe-docs/design";

/** The manual's pages, grouped the way the column shows them. */
export type Contents = {
  /**
   * The pages the documentation asked to stand first, outside every
   * group, in the order it named them. Empty when it asked for none.
   */
  readonly pinned: readonly ContentsItem[];
  readonly sections: readonly ContentsSection[];
  /**
   * The declared learning path, in the order a reader walks it. Empty
   * when the documentation declared none, which is the column exactly as
   * it was before a path could be declared — and the one value the
   * column reads to decide whether to offer the switch at all.
   */
  readonly chapters: readonly ContentsChapter[];
};

/** The folder a document lives in, or the empty string at the root. */
function folderOf(document: string): string {
  const cut = document.indexOf("/");
  return cut === -1 ? "" : document.slice(0, cut);
}

/**
 * The folder's own directory name as a heading: the name with its first
 * letter raised, and nothing else done to it.
 *
 * It is what a manifest that names no sections leaves the column to show
 * — the generated type says so in as many words — and it is deliberately
 * not clever: splitting `how-to` into words or looking a folder up in a
 * table of English nouns would be the shell inventing a title for a
 * documentation that did not give one.
 */
function fromDirectoryName(folder: string): string {
  return folder.charAt(0).toUpperCase() + folder.slice(1);
}

/** What a folder is called here, asking the reader's edition first. */
function titleOf(here: Edition, source: Edition, folder: string): string {
  for (const edition of [here, source]) {
    const named = edition.navigation?.sections.find(
      (section) => section.id === folder,
    );
    if (named !== undefined) return named.title;
  }
  return fromDirectoryName(folder);
}

/**
 * What a chapter of the path is called here, asking the reader's edition
 * first.
 *
 * The ladder is the folders' one rung shorter. A translation names the
 * chapters of the path it takes from its source and names them by `id`
 * (`##NAV-CHAPTERS-TRANSLATION`), so a chapter it has not named keeps the
 * source's words — and there is no third rung, because a chapter has no
 * directory to fall back on: its title is the only name it has.
 */
function chapterTitleOf(here: Edition, chapter: NavigationChapter): string {
  const named = here.navigation?.chapters?.find((row) => row.id === chapter.id);
  return named === undefined ? chapter.title : named.title;
}

/** The edition the reader is at, or the source when there is no such one. */
function editionAt(library: Library, at: string | null): Edition {
  return (
    editions(library).find((one) => one.segment === at) ??
    sourceEdition(library)
  );
}

/**
 * What each page of the source documentation is called where the reader
 * is standing.
 *
 * The list is the source's pages and the words are the edition's:
 * `resolvePage` is the one place that knows which edition will answer for
 * an address, so a label and the text behind the link cannot disagree. A
 * page this edition carries is named in its own words; one it has not
 * reached resolves to the source's page, which is both the title shown
 * and the text that address serves.
 *
 * It is a map rather than a lookup per entry because the pinned list, the
 * folders and the chapters all name the same pages, and each of them asks
 * this question about every page it names.
 */
function servedTitles(
  library: Library,
  at: string | null,
): ReadonlyMap<string, string> {
  const titles = new Map<string, string>();
  for (const page of sourceEdition(library).pages) {
    const document = documentOf(page.path);
    const served = resolvePage(library, at, document);
    titles.set(document, (served?.page ?? page).title);
  }
  return titles;
}

/** One page of the declared learning path, as the reader will open it. */
export type PathPage = {
  /** The document path without its extension, as the path spells it. */
  readonly document: string;
  /** Its title, in the words of the edition that will serve it. */
  readonly label: string;
  readonly href: string;
};

/** One chapter of the declared learning path. */
export type PathChapter = {
  readonly id: string;
  /** The number the reader sees, `1`; empty for an appendix chapter. */
  readonly number: string;
  readonly title: string;
  readonly appendix: boolean;
  readonly pages: readonly PathPage[];
};

/**
 * The learning path this documentation declared, chapter by chapter, or
 * `null` when it declared none (`##NAV-CHAPTERS`).
 *
 * `null` and not an empty list, because the two are different answers and
 * every reader of this function turns on the difference: a documentation
 * with no path is shown exactly as it was before paths existed, while one
 * that opened the table and named no chapter has made a statement about
 * itself. The structure of the path is always the SOURCE's — a
 * translation takes the path of the documentation it adapts and only
 * renames its chapters (`##NAV-CHAPTERS-TRANSLATION`) — and the words are
 * the reader's edition's, one rung at a time, exactly as they are for a
 * folder and for a page.
 *
 * The numbers are the chapters' places in the count, and an appendix
 * takes no place in it: it is pages a reader looks things up in rather
 * than reads through, so it is named and not numbered
 * (`##NAV-CHAPTERS-READER`), and the chapter after it keeps the number it
 * would have had.
 *
 * A chapter's page that names no page of this documentation is passed
 * over rather than drawn as a dead entry, for the reason a stale pin is:
 * `vibe check` is where a path that names a missing page is reported, and
 * a site that rendered a link to nothing would be the second opinion.
 */
export function learningPath(
  library: Library,
  at: string | null,
): readonly PathChapter[] | null {
  const source = sourceEdition(library);
  const declared = source.navigation?.chapters;
  if (declared === undefined) return null;

  const here = editionAt(library, at);
  const titles = servedTitles(library, at);
  let counted = 0;

  return declared.map((chapter) => {
    const appendix = chapter.appendix === true;
    if (!appendix) counted += 1;
    return {
      id: chapter.id,
      number: appendix ? "" : String(counted),
      title: chapterTitleOf(here, chapter),
      appendix,
      pages: chapter.pages
        .filter((document) => titles.has(document))
        .map((document) => ({
          document,
          label: titles.get(document) ?? document,
          href: docHref(addressOf(library, at, document)),
        })),
    };
  });
}

/** Where the path leads from one page of it, in both directions. */
export type PathNeighbours = {
  /** Absent on the first page of the path. */
  readonly previous?: PagerLink;
  /** Absent on the last. */
  readonly next?: PagerLink;
};

/** The chapter a neighbour stands in, as its caption states it. */
function captionOf(chapter: PathChapter): PagerChapter {
  return {
    ...(chapter.appendix ? {} : { number: chapter.number }),
    title: chapter.title,
  };
}

/**
 * The page before this one on the path and the page after it, or `null`
 * when there is no path or this page is not on it
 * (`##NAV-CHAPTERS-READER`).
 *
 * The second `null` is not a case `vibe check` allows — a path that
 * leaves a page of the package out is an error of the package — and it is
 * answered all the same, because the site renders whatever manifest a
 * deployment handed it and a page that is off the path has no previous
 * and no next to show.
 *
 * A neighbour carries its chapter only when that chapter is not this
 * page's. Inside a chapter every page shares it, and printing it on each
 * would be one heading repeated once per page; at the seam between two it
 * is the whole point — a reader is being told they are leaving one lesson
 * for the next.
 */
export function neighboursOf(
  library: Library,
  at: string | null,
  document: string,
): PathNeighbours | null {
  const path = learningPath(library, at);
  if (path === null) return null;

  const walked = path.flatMap((chapter) =>
    chapter.pages.map((page) => ({ page, chapter })),
  );
  const standing = walked.findIndex((step) => step.page.document === document);
  if (standing === -1) return null;

  const here = walked[standing]?.chapter;
  const link = (
    step: (typeof walked)[number] | undefined,
  ): PagerLink | null => {
    if (step === undefined) return null;
    const crossing = step.chapter.id !== here?.id;
    return {
      href: step.page.href,
      title: step.page.label,
      ...(crossing ? { chapter: captionOf(step.chapter) } : {}),
    };
  };

  const previous = link(walked[standing - 1]);
  const next = link(walked[standing + 1]);
  return {
    ...(previous === null ? {} : { previous }),
    ...(next === null ? {} : { next }),
  };
}

/**
 * The whole column for one documentation, in one language, with the page
 * the reader is on marked.
 *
 * `currentDocument` is `null` on the documentation's own front page,
 * where every entry is somewhere else.
 */
export function contentsOf(
  library: Library,
  at: string | null,
  currentDocument: string | null,
): Contents {
  const source = sourceEdition(library);
  const here = editionAt(library, at);

  const entry = (document: string, label: string): ContentsItem => ({
    label,
    href: docHref(addressOf(library, at, document)),
    current: document === currentDocument,
  });

  const titles = servedTitles(library, at);

  /* A pin names a document, and a pin that names no page of this
     documentation is passed over rather than drawn as a dead entry: the
     package's own `vibe check` is where a stale pin is reported, and a
     site that rendered a link to nothing would be the second opinion. */
  const pinned = (source.navigation?.pinned ?? []).filter((document) =>
    titles.has(document),
  );
  const isPinned = new Set(pinned);

  const grouped = new Map<string, ContentsItem[]>();
  for (const page of source.pages) {
    const document = documentOf(page.path);
    if (isPinned.has(document)) continue;
    const folder = folderOf(document);
    const items = grouped.get(folder) ?? [];
    if (items.length === 0) grouped.set(folder, items);
    items.push(entry(document, titles.get(document) ?? page.title));
  }

  return {
    pinned: pinned.map((document) =>
      entry(document, titles.get(document) ?? document),
    ),
    sections: [...grouped.entries()].map(([folder, items]) => ({
      id: folder,
      title: folder.length === 0 ? "" : titleOf(here, source, folder),
      items,
    })),
    /* The path as the column shows it: the chapters the documentation
       declared, with the page the reader is on marked in this view too.
       A documentation that declared none gives the empty list, which is
       what the column reads as «offer no switch». */
    chapters: (learningPath(library, at) ?? []).map((chapter) => ({
      id: chapter.id,
      number: chapter.number,
      title: chapter.title,
      items: chapter.pages.map((page) => entry(page.document, page.label)),
    })),
  };
}
