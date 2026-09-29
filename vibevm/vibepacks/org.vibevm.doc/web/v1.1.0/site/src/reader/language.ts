/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

/**
 * Keeping the reader's place across a change of language.
 *
 * The selector's destinations are decided at build time and are right
 * about the PAGE: an adaptation is served under the source's coordinate
 * with a language segment in front, so the other language's address is
 * this one with a segment changed. What a build cannot know is WHERE in
 * the page the reader is standing, and that is the whole of what this
 * module adds — the current fragment, appended on the way out.
 *
 * It is possible at all because an adaptation mirrors the source block
 * for block, which the translation check enforces: `#p12` names the same
 * block in both, so a reader who switches language keeps their line
 * rather than landing at the top of a page in a language they just asked
 * for.
 */

import { all } from "./dom.ts";
import { rememberLanguage } from "./fallback.ts";

/** Strip any fragment a destination already has, then add the live one. */
function withFragment(href: string, fragment: string): string {
  const base = href.split("#")[0] ?? href;
  return fragment.length === 0 ? base : `${base}${fragment}`;
}

export function startLanguageSwitch(): () => void {
  const repaint = (): void => {
    const fragment = window.location.hash;
    for (const link of all("[data-language-selector] a[href]")) {
      const href = link.getAttribute("href");
      if (href === null) continue;
      link.setAttribute("href", withFragment(href, fragment));
    }
  };

  const onHash = (): void => repaint();
  /**
   * Also just before the click: the fragment moves as the reader clicks
   * block numbers, and `hashchange` does not fire for the `replaceState`
   * the block anchors use.
   */
  const onPointer = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest("[data-language-selector]") === null) return;
    repaint();
    /**
     * An explicit click is the strongest statement a reader can make
     * about language, so it is recorded BEFORE the page leaves. Without
     * this, choosing the source language at the door would be undone at
     * the door: the catalogue reads the remembered choice, and the one
     * just made would not be there yet.
     */
    const choice = target.closest("[data-lang-choice]");
    if (choice instanceof HTMLElement) {
      const tag = choice.dataset["langChoice"];
      if (tag !== undefined) rememberLanguage(tag);
    }
  };

  window.addEventListener("hashchange", onHash);
  document.addEventListener("pointerdown", onPointer, true);
  document.addEventListener("focusin", onPointer, true);
  repaint();

  return () => {
    window.removeEventListener("hashchange", onHash);
    document.removeEventListener("pointerdown", onPointer, true);
    document.removeEventListener("focusin", onPointer, true);
  };
}
