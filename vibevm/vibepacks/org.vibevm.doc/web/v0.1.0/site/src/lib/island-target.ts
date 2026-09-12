/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS */

/**
 * What was clicked inside the island — the whole of the island's event
 * contract, and the reason there is one listener on the region instead
 * of a listener per block.
 *
 * The island is finished HTML from the Rust pipeline. Qwik never renders
 * it, so Qwik cannot attach a handler to anything in it; and the page
 * may carry hundreds of numbered blocks, so attaching handlers to them
 * by hand would mean walking the DOM after every insertion. Delegation
 * answers both: the region listens once, and this function turns the
 * node that was clicked into the intent the reader had.
 *
 * It is a pure function over an element chain on purpose — it is the
 * part that can be tested without a browser, and the part the reader's
 * behaviour will be built on top of.
 */

/** The four things a click inside a rendered page can mean. */
export type IslandIntent =
  /** A block's number — the address a reader copies (`a.p-anchor`). */
  | { readonly kind: "anchor"; readonly block: string }
  /** A quoted rule, which carries its `spec://` address in `data-uri`. */
  | { readonly kind: "rule"; readonly uri: string }
  /** Any other link: let the browser do what it does with links. */
  | { readonly kind: "link"; readonly href: string }
  /** Text, whitespace, a table cell — nothing to act on. */
  | { readonly kind: "none" };

/**
 * The minimum an element must expose for this to work — named as a
 * structural type rather than `Element` so the function can be tested
 * with plain objects and never needs a DOM to exist.
 */
export type ClickedNode = {
  readonly parent: ClickedNode | null;
  /** The element's class list, already split. */
  readonly classes: readonly string[];
  /** Attribute lookup; `null` when the element does not carry it. */
  readonly attribute: (name: string) => string | null;
};

/** How far up the tree a click is traced before it counts as «none». */
const MAX_DEPTH = 12;

/**
 * Walk up from the clicked node to the first element that means
 * something, stopping at `MAX_DEPTH`.
 *
 * The depth limit is not a micro-optimisation: without it a click on a
 * deeply nested span in a table would walk out of the island, out of the
 * page and into `<html>`, and the first ancestor with a `href` it met on
 * the way — a wrapping link somewhere up the layout — would be reported
 * as the reader's intent.
 */
export function islandTarget(from: ClickedNode | null): IslandIntent {
  let node = from;
  for (let depth = 0; node !== null && depth < MAX_DEPTH; depth += 1) {
    if (node.classes.includes("p-anchor")) {
      const id = node.attribute("id");
      if (id !== null) return { kind: "anchor", block: id };
    }
    if (node.classes.includes("rule")) {
      const uri = node.attribute("data-uri");
      if (uri !== null) return { kind: "rule", uri };
    }
    const link = node.attribute("href");
    if (link !== null) return { kind: "link", href: link };
    node = node.parent;
  }
  return { kind: "none" };
}

/** Lift a real DOM element into the shape `islandTarget` reads. */
export function fromElement(element: Element | null): ClickedNode | null {
  if (element === null) return null;
  return {
    parent: fromElement(element.parentElement),
    classes: Array.from(element.classList),
    attribute: (name: string) => element.getAttribute(name),
  };
}
