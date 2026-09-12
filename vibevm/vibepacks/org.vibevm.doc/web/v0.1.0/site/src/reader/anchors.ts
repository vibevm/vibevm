/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS */

/**
 * The block numbers: what a click on one does, and where a fragment
 * lands.
 *
 * The numbers themselves are not this module's business and must never
 * become it. They are assigned by the Rust pipeline at build time, before
 * any `when` filtering, and they are the same in the HTML island, in the
 * `.md` and `.xml` projections and in `llms-full.txt` — which is the
 * whole point: a human and an agent cite one place. A script that
 * numbered blocks in the browser would put a number in the page that
 * exists nowhere else (R-26).
 *
 * So this module only reacts. A click on a number puts it in the address
 * bar without adding a history entry, copies the whole address, and ticks
 * for a second and a half; arriving with a fragment centres the block
 * rather than pinning it to the top edge, where the header would cover it
 * and the reader could not see what came before.
 */

import { copy, island, targetOf } from "./dom.ts";

/** How long the copied tick stays up. */
const TICK_MS = 1500;

/** The browser has already scrolled by then, and this is the correction. */
const SETTLE_MS = 100;

/** The block a fragment names, when the page has it. */
function targetOfHash(): HTMLElement | null {
  const hash = window.location.hash.slice(1);
  if (hash.length === 0) return null;
  const found = document.getElementById(decodeURIComponent(hash));
  return found instanceof HTMLElement ? found : null;
}

/** Mark the block a fragment names, so the eye finds it after the scroll. */
function mark(element: HTMLElement | null): void {
  for (const previous of Array.from(document.querySelectorAll(".is-target"))) {
    previous.classList.remove("is-target");
  }
  if (element === null) return;
  const block = element.closest("[data-p]") ?? element;
  block.classList.add("is-target");
}

/**
 * Centre the block a fragment names.
 *
 * The browser has already jumped it to the top edge by the time this
 * runs, and the correction is deliberate: a block against the top of the
 * window has no context above it, and the sticky header covers its first
 * line.
 */
function centre(): void {
  const element = targetOfHash();
  if (element === null) return;
  window.setTimeout(() => {
    element.scrollIntoView({ block: "center" });
    mark(element);
  }, SETTLE_MS);
}

export function startAnchors(): () => void {
  const region = island();
  if (region === null) return () => undefined;

  const onClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;
    const anchor = target.closest("a.p-anchor");
    if (!(anchor instanceof HTMLElement)) return;
    const id = anchor.getAttribute("id");
    if (id === null) return;

    event.preventDefault();
    const address = `${window.location.href.split("#")[0] ?? ""}#${id}`;
    window.history.replaceState(null, "", `#${id}`);
    mark(anchor);

    void copy(address).then((done) => {
      if (!done) return;
      anchor.classList.add("copied");
      window.setTimeout(() => anchor.classList.remove("copied"), TICK_MS);
    });
  };

  const onHash = (): void => centre();

  region.addEventListener("click", onClick);
  window.addEventListener("hashchange", onHash);
  centre();

  return () => {
    region.removeEventListener("click", onClick);
    window.removeEventListener("hashchange", onHash);
  };
}

/** The block a reader is standing on, for a citation that names a place. */
export function currentBlock(): string | null {
  const region = island();
  if (region === null) return null;
  let best: string | null = null;
  for (const node of Array.from(region.querySelectorAll("a.p-anchor[id]"))) {
    if (!(node instanceof HTMLElement)) continue;
    if (node.getBoundingClientRect().top > 5) break;
    best = node.getAttribute("id");
  }
  return best;
}
