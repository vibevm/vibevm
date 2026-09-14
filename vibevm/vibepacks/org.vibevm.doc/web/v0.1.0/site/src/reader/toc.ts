/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

/**
 * The table of contents: this page's headings, built from the page,
 * highlighted as it is read, and laid out by the room it has.
 *
 * It is built here rather than at build time because the pipeline's
 * manifest carries the page's NAMED anchors — its sections and its facts
 * together — and a contents list needs the headings, which only the
 * rendered document distinguishes. Reading `h2` and `h3` out of a
 * document already in the page is a walk of the DOM, not a second
 * renderer: nothing is parsed and nothing is re-rendered.
 *
 * The class it decides — `has-sidebar` — is the whole page's, not this
 * column's: the manual's pages stand in a column on the other side of
 * the text and appear and fold with this one, because the decision is
 * about the room the page has and is the same decision for both.
 *
 * The active item is found by an IntersectionObserver with the margin
 * the design system arrived at — `-15% 0px -70% 0px`. It is not a taste:
 * the band it leaves is a strip near the top of the window, so the item
 * that lights up is the section a reader is reading rather than the one
 * that happens to be tallest on the screen.
 */

import { all, island, one } from "./dom.ts";

/** The band the active heading is chosen in — the design system's. */
const ROOT_MARGIN = "-15% 0px -70% 0px";

/** Above this window width a sidebar has room; below it, it does not. */
const SIDEBAR_FROM = 1100;

/** And above this column width the reader has taken the sidebar's room. */
const COLUMN_LIMIT = 1100;

/** The id a heading is addressed by: its own, or its section's. */
function addressOf(heading: Element): string | null {
  const own = heading.getAttribute("id");
  if (own !== null && own.length > 0) return own;
  const section = heading.closest("section[id]");
  return section === null ? null : section.getAttribute("id");
}

function buildList(region: HTMLElement, list: HTMLElement): Element[] {
  const targets: Element[] = [];
  list.replaceChildren();
  for (const heading of Array.from(region.querySelectorAll("h2, h3"))) {
    const id = addressOf(heading);
    if (id === null) continue;
    const item = document.createElement("li");
    item.className =
      heading.tagName === "H3" ? "toc__item toc__item--h3" : "toc__item";
    const link = document.createElement("a");
    link.href = `#${id}`;
    link.textContent = heading.textContent ?? id;
    link.dataset["tocFor"] = id;
    item.appendChild(link);
    list.appendChild(item);
    targets.push(heading.closest("section[id]") ?? heading);
  }
  return targets;
}

export function startToc(): () => void {
  const region = island();
  const details = one("[data-toc]");
  const list = one("[data-toc-list]");
  if (region === null || details === null || list === null) {
    return () => undefined;
  }

  const targets = buildList(region, list);

  const page = details.closest(".doc-view");

  /**
   * Sidebar or block, decided by both widths. The reader's own column
   * setting is part of it: widening the text past the sidebar's limit is
   * asking for the width, and the contents move above the text instead
   * of being squeezed against it.
   */
  const layout = (): void => {
    if (page === null) return;
    const column = one(".prose");
    const measure = Number.parseInt(
      column?.style.getPropertyValue("--measure") ?? "740",
      10,
    );
    const wide =
      window.innerWidth >= SIDEBAR_FROM &&
      (Number.isNaN(measure) || measure <= COLUMN_LIMIT);
    page.classList.toggle("has-sidebar", wide);
    if (details instanceof HTMLDetailsElement && wide) details.open = true;
  };

  layout();

  const links = new Map<string, HTMLElement>();
  for (const link of all("[data-toc-for]", list)) {
    const id = link.dataset["tocFor"];
    if (id !== undefined) links.set(id, link);
  }

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const id = entry.target.getAttribute("id");
        if (id === null) continue;
        for (const link of links.values()) link.classList.remove("is-active");
        links.get(id)?.classList.add("is-active");
      }
    },
    { rootMargin: ROOT_MARGIN },
  );
  for (const target of targets) observer.observe(target);

  const onResize = (): void => layout();
  window.addEventListener("resize", onResize, { passive: true });

  /** The column's width is a reader's setting, so watch it change. */
  const column = one(".prose");
  const watcher =
    column === null
      ? null
      : new MutationObserver(() => {
          layout();
        });
  if (column !== null && watcher !== null) {
    watcher.observe(column, { attributes: true, attributeFilter: ["style"] });
  }

  return () => {
    observer.disconnect();
    watcher?.disconnect();
    window.removeEventListener("resize", onResize);
  };
}
