/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-FOR-AGENT */

/**
 * The citation a reader hands to an agent, kept pointing at where they
 * actually are.
 *
 * The page's own `spec://` address arrives from the build; what this
 * module adds is the block — `#p12` — recomputed as the reader scrolls,
 * so a citation names a place in the page and not merely the page. The
 * version is already in the address and stays there: `latest` would name
 * a page that moves under the agent between the question and the answer
 * (R-26).
 *
 * Nothing here is fetched and nothing is sent anywhere. Copying is the
 * whole interaction, and the button says so only when the copy actually
 * happened — an insecure origin has no clipboard, and a page claiming to
 * have copied nothing is worse than a page that says it cannot.
 */

import { all, copy } from "./dom.ts";
import { currentBlock } from "./anchors.ts";

const TICK_MS = 1500;

/** How often the citation is re-pointed while the reader scrolls. */
const FOLLOW_MS = 400;

export function startAgentSurface(uri: string): () => void {
  const surfaces = all("[data-for-agent]");
  if (surfaces.length === 0) return () => undefined;

  let shown = uri;

  const repaint = (): void => {
    const block = currentBlock();
    const next = block === null ? uri : `${uri}#${block}`;
    if (next === shown) return;
    shown = next;
    for (const node of all("[data-agent-uri]")) node.textContent = shown;
  };

  let timer: number | null = null;
  const onScroll = (): void => {
    if (timer !== null) return;
    timer = window.setTimeout(() => {
      timer = null;
      repaint();
    }, FOLLOW_MS);
  };

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const button = target.closest("[data-agent-copy]");
    if (!(button instanceof HTMLElement)) return;
    void copy(shown).then((done) => {
      if (!done) return;
      button.classList.add("copied");
      window.setTimeout(() => button.classList.remove("copied"), TICK_MS);
    });
  };

  const onFab = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const button = target.closest(".fab__button");
    if (button === null) return;
    const panel = document.getElementById("fab-panel");
    if (!(panel instanceof HTMLElement)) return;
    const open = panel.hidden;
    panel.hidden = !open;
    button.setAttribute("aria-expanded", open ? "true" : "false");
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  document.addEventListener("click", onClick);
  document.addEventListener("click", onFab);
  repaint();

  return () => {
    window.removeEventListener("scroll", onScroll);
    document.removeEventListener("click", onClick);
    document.removeEventListener("click", onFab);
    if (timer !== null) window.clearTimeout(timer);
  };
}
