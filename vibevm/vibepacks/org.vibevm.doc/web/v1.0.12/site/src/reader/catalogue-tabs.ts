/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * Which of the door's three shelves a reader is looking at.
 *
 * The shelves are all in the document. The build writes one page for the
 * catalogue and the reader moves between the shelves without fetching
 * anything, which is why this hides rather than renders: three shelves
 * of a few cards each cost less than one request, and a reader comparing
 * what is featured against everything there is should not wait for the
 * comparison.
 *
 * The tab is in the address as a query, and that is not decoration. A
 * reader who has found the projections shelf has something to send
 * somebody, and `?tab=projections` is the whole of what needs sending —
 * it is written with `replaceState`, so the back button still means the
 * page before this one rather than the tab before this one.
 *
 * The order of answers is the order of how deliberate they are: the
 * address a reader arrived at, then the tab they last chose, then the
 * shelf the build opened the page on.
 */

import { all, one } from "./dom.ts";
import { readLocal, writeLocal } from "./storage.ts";

/** Where the choice lives, beside the theme and the two languages. */
const KEY = "catalogue-tab";

/** The name the pills and the panels of THIS row share. */
const GROUP = "catalogue";

/** The query the address carries the tab in. */
const PARAM = "tab";

function pills(): HTMLElement[] {
  return all(`[data-tab-switch="${GROUP}"] button[value]`);
}

function panels(): HTMLElement[] {
  return all(`[data-tab-panel][data-tab-group="${GROUP}"]`);
}

/** The tabs this page actually offers, in the order it offers them. */
function offered(): string[] {
  const out: string[] = [];
  for (const pill of pills()) {
    const value = pill.getAttribute("value");
    if (value !== null && value.length > 0) out.push(value);
  }
  return out;
}

/** The shelf the build opened the page on, which is the last word. */
function opened(): string | null {
  const shown = panels().find((panel) => !panel.hidden);
  return shown?.dataset["tabPanel"] ?? null;
}

function chosen(available: readonly string[]): string {
  const asked = new URL(window.location.href).searchParams.get(PARAM);
  if (asked !== null && available.includes(asked)) return asked;
  const stored = readLocal(KEY);
  if (stored !== null && available.includes(stored)) return stored;
  return opened() ?? available[0] ?? "";
}

/**
 * Show one shelf, mark its pill, and leave the others where they are.
 *
 * The row is stamped with the answer as well as the pills, because the
 * pills carry the mark the BUILD wrote until this runs and the two are
 * indistinguishable by looking. One attribute in one place says which
 * shelf is showing because a reader asked for it, which is what a style
 * and a test both need and neither can work out.
 */
export function applyCatalogueTab(tab: string): void {
  for (const pill of pills()) {
    const current = pill.getAttribute("value") === tab;
    pill.setAttribute("aria-pressed", current ? "true" : "false");
    pill.classList.toggle("tab-pills__pill--current", current);
  }
  for (const panel of panels()) {
    panel.hidden = panel.dataset["tabPanel"] !== tab;
  }
  const row = one(`[data-tab-switch="${GROUP}"]`);
  row?.setAttribute("data-tab-current", tab);
}

/** Put the tab in the address, without adding a step to the history. */
function address(tab: string): void {
  try {
    const url = new URL(window.location.href);
    url.searchParams.set(PARAM, tab);
    window.history.replaceState(window.history.state, "", url.toString());
  } catch {
    /* A browser that refuses the history keeps the shelf and loses the
       shareable address, which is the right way round. */
  }
}

export function startCatalogueTabs(): () => void {
  const row = one(`[data-tab-switch="${GROUP}"]`);
  if (row === null) return () => undefined;

  const available = offered();
  if (available.length === 0) return () => undefined;

  const tab = chosen(available);
  applyCatalogueTab(tab);

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const pill = target.closest("button[value]");
    if (pill === null || !row.contains(pill)) return;
    const value = pill.getAttribute("value");
    if (value === null) return;
    writeLocal(KEY, value);
    applyCatalogueTab(value);
    address(value);
  };

  row.addEventListener("click", onClick);
  return () => row.removeEventListener("click", onClick);
}
