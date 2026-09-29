/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-CITATIONS-RESOLVED */

/**
 * The rule transclusion: click a quoted rule, see the rule.
 *
 * The whole point is what this module does NOT do. The rule's text is
 * already in the page — the pipeline resolved the citation at build time
 * and put the fact's words inside the island, with its `spec://` address
 * on `data-uri` — so the panel is filled from the element that was
 * clicked and nothing is fetched. A shell that went to the network for a
 * rule would be a shell that cannot show one offline, in the local
 * reader, or behind a firewall; and it would be a second resolver
 * disagreeing with the one that already ran.
 *
 * A rule whose text this build could not resolve says so: the pipeline
 * marks it `data-unresolved` and prints its address instead of its
 * words, and the panel repeats that rather than pretending.
 */

import { copy, island, one, targetOf } from "./dom.ts";

/** How long the «copied» tick stays up, in step with the block anchors. */
const TICK_MS = 1500;

function panel(): HTMLElement | null {
  return one("[data-rule-panel]");
}

function close(): void {
  const box = panel();
  if (box === null) return;
  box.hidden = true;
  box.removeAttribute("style");
}

function place(box: HTMLElement, near: Element): void {
  const rect = near.getBoundingClientRect();
  box.hidden = false;
  box.style.top = `${rect.bottom + window.scrollY + 8}px`;
  const left = Math.max(12, rect.left + window.scrollX);
  box.style.left = `${left}px`;
}

function fill(box: HTMLElement, quote: Element): void {
  const uri = quote.getAttribute("data-uri") ?? "";
  const unresolved =
    quote.closest("[data-unresolved]") !== null ||
    quote.getAttribute("data-unresolved") !== null;

  const address = one("[data-rule-uri]", box);
  if (address !== null) address.textContent = uri;

  const text = one("[data-rule-text]", box);
  if (text !== null) {
    text.textContent = quote.textContent ?? "";
    text.classList.toggle("rule-panel__text--unresolved", unresolved);
  }

  const link = one("[data-rule-link]", box);
  if (link instanceof HTMLAnchorElement) {
    link.href = quote.getAttribute("href") ?? "#";
  }

  const button = one("[data-rule-copy]", box);
  if (button !== null) button.dataset["uri"] = uri;
}

/**
 * Wire the transclusion over the island.
 *
 * One listener on the region, as everywhere else: the island may carry
 * any number of rules and none of them can hold a handler of its own.
 */
export function startRuleTransclusion(): () => void {
  const region = island();
  const box = panel();
  if (region === null || box === null) return () => undefined;

  const onIslandClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;
    const quote = target.closest("a.rule");
    if (quote === null) return;
    event.preventDefault();
    fill(box, quote);
    place(box, quote.closest("blockquote") ?? quote);
  };

  const onPanelClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;
    if (target.closest("[data-rule-close]") !== null) {
      close();
      return;
    }
    const button = target.closest("[data-rule-copy]");
    if (!(button instanceof HTMLElement)) return;
    const uri = button.dataset["uri"] ?? "";
    if (uri.length === 0) return;
    void copy(uri).then((done) => {
      if (!done) return;
      button.classList.add("copied");
      window.setTimeout(() => button.classList.remove("copied"), TICK_MS);
    });
  };

  const onDocumentClick = (event: Event): void => {
    if (box.hidden) return;
    const target = targetOf(event);
    if (target === null) return;
    if (box.contains(target) || target.closest("a.rule") !== null) return;
    close();
  };

  const onKey = (event: KeyboardEvent): void => {
    if (event.key === "Escape") close();
  };

  region.addEventListener("click", onIslandClick);
  box.addEventListener("click", onPanelClick);
  document.addEventListener("click", onDocumentClick);
  document.addEventListener("keydown", onKey);

  return () => {
    region.removeEventListener("click", onIslandClick);
    box.removeEventListener("click", onPanelClick);
    document.removeEventListener("click", onDocumentClick);
    document.removeEventListener("keydown", onKey);
  };
}
