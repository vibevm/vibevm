/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE */

/**
 * What the local reader is actually serving, asked of the reader itself.
 *
 * The shell `vibe` carries is ONE prerendered route template. It was
 * built against the web package's fixture library, because at the moment
 * it is embedded nobody knows which documentation it will ever be
 * pointed at — so everything the template says about a package is the
 * fixture's: the navigation, the meta row, the switches, the addresses
 * in the «for an agent» block. `vibe doc serve` corrects the two things
 * a server can correct in bytes (the title, the three alternates) and
 * cannot correct the rest without rendering the shell, which would make
 * it a second renderer of the site.
 *
 * So the shell asks. The reader already publishes `<base>manifest.json`
 * — the same manifest `vibe doc build` writes, of the package this
 * reader was pointed at — and one fetch at the start of a page turns the
 * fixture's chrome into this documentation's. Nothing is computed from
 * the island and nothing is parsed out of the page
 * (`##PIPE-SHELL-PARSES-NOTHING`): the values arrive named, exactly as
 * they do on the public site, and the only difference is that they
 * arrive over loopback instead of being baked in.
 *
 * The island is never touched. It came finished in the page the server
 * dressed, and a shell that re-rendered it on the client would find the
 * marker rather than the page — the serialised state still carries the
 * marker as a string (finding P4-O4, anomaly A-4). Links between pages
 * are therefore ordinary links: the server renders the page a reader
 * asked for, per request, which is what `##LOCAL-SERVE` says it does.
 */

import { parseLibrary, type Library } from "../lib/library.ts";
import {
  cataloguePath,
  href,
  llmsHref,
  parseDocTarget,
  projectionHref,
} from "../lib/href.ts";
import { viewOf, type DocView } from "../lib/view.ts";

/** What one page of the local reader shows, once the manifest is in. */
export type ServedPage = {
  /** The mount this reader answers at, ending in a slash. */
  readonly base: string;
  /** The library of the one package this reader was pointed at. */
  readonly library: Library;
  /** The address the browser is on, as everything a page shows. */
  readonly view: DocView;
};

/**
 * Where this reader is mounted.
 *
 * The site's own mount, which is also the reader's default — and the
 * only base the shell can work at, because the addresses of its chunks,
 * its stylesheet and its fonts are baked into the template at build time
 * and nothing rewrites them. A reader started with another `--base`
 * therefore serves a page whose scripts 404, and this function is not
 * what would have to change to fix that.
 *
 * It is still read out of the document when the document says: a
 * `text/plain` alternate the shell did not write itself is
 * `<base>llms.txt`, put there by the party that knows the base. Nothing
 * writes one today, so the constant is what answers.
 */
export function servedBase(doc: Document): string {
  const link = doc.querySelector(
    `link[rel="alternate"][type="text/plain"]:not([${WRITTEN}])`,
  );
  const at = link?.getAttribute("href") ?? "";
  const suffix = "llms.txt";
  if (at.startsWith("/") && at.endsWith(suffix)) {
    return at.slice(0, at.length - suffix.length);
  }
  return href(cataloguePath(null));
}

/** The part of a served address that names a document, or `null`. */
function addressIn(pathname: string, base: string): string | null {
  if (!pathname.startsWith(base)) return null;
  const rest = pathname.slice(base.length).replace(/^\/+|\/+$/g, "");
  return rest.length === 0 ? null : rest;
}

/**
 * The manifest this reader publishes, as a library of one edition.
 *
 * One edition, because a reader is pointed at one package and a
 * translation is another package with a manifest of its own. The
 * language selector therefore offers the one language there is, which is
 * the truth about a machine store rather than a missing feature.
 */
async function libraryAt(base: string): Promise<Library | null> {
  const at = `${base}manifest.json`;
  try {
    const response = await fetch(at, {
      headers: { accept: "application/json" },
    });
    if (!response.ok) {
      console.error(`the reader answered ${response.status} for ${at}`);
      return null;
    }
    const body: unknown = await response.json();
    return parseLibrary([{ name: at, value: body }]);
  } catch (reason) {
    /* The parser fails with the path of the field that failed in it, and
       that sentence is the whole value of failing here rather than
       rendering half a shelf. The page keeps the island the server
       glued in — the text is readable with no chrome at all, which is
       what the bare shell already is. */
    console.error(`${at} could not be read: ${String(reason)}`);
    return null;
  }
}

/**
 * Everything the page the browser is on should show, or `null` when this
 * is not an address of the documentation being served.
 *
 * `null` is not an error state to hide: it is «the shell is standing
 * somewhere it cannot describe», and the caller leaves the chrome empty
 * rather than showing the fixture's, because the fixture's chrome is
 * what this whole file exists to stop showing.
 */
export async function readServedPage(
  at: Location = window.location,
  doc: Document = document,
): Promise<ServedPage | null> {
  const base = servedBase(doc);
  const address = addressIn(at.pathname, base);
  if (address === null) return null;
  if (parseDocTarget(address.split("/")) === null) return null;

  const library = await libraryAt(base);
  if (library === null) return null;
  const view = viewOf(library, address);
  return view === null ? null : { base, library, view };
}

/** The attribute that says a tag here was written by the shell, not prerendered. */
const WRITTEN = "data-shell-head";

/**
 * The two things a document head says about a PAGE, written once the
 * shell knows which page it is showing.
 *
 * They used to be prerendered into the template and corrected by the
 * server on the way out. They cannot be: an element inserted into the
 * head of a resumed Qwik container desynchronises the container from its
 * own serialised shape, and the page stops resuming altogether — no
 * table of contents, no anchors, no settings, no manifest. Appending
 * AFTER everything the container knows about is the one edit it
 * tolerates, measured over a served page, and that is what this does.
 *
 * The values are the manifest's, which is the same source the server was
 * reading; the difference is that the party writing them is the one that
 * knows which address the browser is actually on.
 */
export function dressHead(page: ServedPage, doc: Document = document): void {
  const view = page.view;
  if (view.kind === "catalogue") return;
  doc.title = view.title;
  for (const one of doc.head.querySelectorAll(`[${WRITTEN}]`)) one.remove();
  if (view.kind !== "page") return;

  const address = view.address;
  const alternates: readonly (readonly [string, string, string])[] = [
    ["text/markdown", projectionHref(address, "md"), "This page as Markdown"],
    [
      "application/xml",
      projectionHref(address, "xml"),
      "This page as the dialect XML",
    ],
    ["text/plain", llmsHref(address), "llms.txt of this documentation"],
  ];
  for (const [type, at, title] of alternates) {
    const link = doc.createElement("link");
    link.setAttribute(WRITTEN, "");
    link.rel = "alternate";
    link.type = type;
    link.href = at;
    link.title = title;
    doc.head.append(link);
  }
}
