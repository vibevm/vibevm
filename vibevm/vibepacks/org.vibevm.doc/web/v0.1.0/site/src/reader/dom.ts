/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-ONE-CONTENT-PATH */

/**
 * The four DOM questions every reader behaviour asks, answered once.
 *
 * The island is finished HTML from the Rust pipeline: Qwik did not
 * render it, so no handler can be attached to anything inside it, and
 * every behaviour reaches it the same way — find the region, listen once
 * on it, and narrow whatever was clicked. Writing that out per module
 * would be four chances to narrow it differently.
 *
 * There is no type assertion anywhere below and there is not meant to
 * be. `querySelector` answers `Element | null` because the element may
 * not be there, and a cast would turn a missing element into a crash one
 * call later, in a module that has nothing to do with why it is missing.
 */

/** The region the rendered page was inserted into, if this page has one. */
export function island(): HTMLElement | null {
  const found = document.querySelector("[data-island]");
  return found instanceof HTMLElement ? found : null;
}

/** One element by id, narrowed to the kind of element that can be styled. */
export function byId(id: string): HTMLElement | null {
  const found = document.getElementById(id);
  return found instanceof HTMLElement ? found : null;
}

/** The first element matching a selector, narrowed the same way. */
export function one(selector: string, within?: ParentNode): HTMLElement | null {
  const found = (within ?? document).querySelector(selector);
  return found instanceof HTMLElement ? found : null;
}

/** Every element matching a selector, as an array rather than a live list. */
export function all(selector: string, within?: ParentNode): HTMLElement[] {
  const found = Array.from((within ?? document).querySelectorAll(selector));
  return found.filter(
    (node): node is HTMLElement => node instanceof HTMLElement,
  );
}

/** What an event happened on, when that is an element at all. */
export function targetOf(event: Event): Element | null {
  const target = event.target;
  return target instanceof Element ? target : null;
}

/**
 * Copy text and say whether it worked.
 *
 * The clipboard is not available everywhere: an insecure origin has no
 * `navigator.clipboard`, and a permission may be refused. Both are
 * ordinary answers, not errors — the caller shows its tick only on a
 * true, so a reader never sees a page claim to have copied nothing.
 */
export async function copy(text: string): Promise<boolean> {
  if (!("clipboard" in navigator)) return false;
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
