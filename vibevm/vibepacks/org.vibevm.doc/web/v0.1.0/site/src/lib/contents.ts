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
 * Three rules and nothing else shapes what comes out.
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
 */

import { docHref } from "./href.ts";
import {
  addressOf,
  documentOf,
  editions,
  sourceEdition,
  type Edition,
  type Library,
} from "./library.ts";

import type { ContentsItem, ContentsSection } from "@vibe-docs/design";

/** The manual's pages, grouped the way the column shows them. */
export type Contents = {
  /**
   * The pages the documentation asked to stand first, outside every
   * group, in the order it named them. Empty when it asked for none.
   */
  readonly pinned: readonly ContentsItem[];
  readonly sections: readonly ContentsSection[];
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

/** The edition the reader is at, or the source when there is no such one. */
function editionAt(library: Library, at: string | null): Edition {
  return (
    editions(library).find((one) => one.segment === at) ??
    sourceEdition(library)
  );
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

  const titles = new Map<string, string>();
  for (const page of source.pages) {
    const document = documentOf(page.path);
    titles.set(document, page.title);
  }

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
    items.push(entry(document, page.title));
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
  };
}
