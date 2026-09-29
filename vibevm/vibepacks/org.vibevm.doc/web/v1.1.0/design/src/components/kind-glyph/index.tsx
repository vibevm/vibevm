/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PLACEHOLDERS-GENERATED */

import { component$, useStyles$, type JSXOutput } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * One of the eight installable package kinds (VIBEVM-SPEC §4.1).
 *
 * The vocabulary is closed and this list is the whole of it, spelled the
 * way a package manifest spells it. A ninth word is an amendment to the
 * specification rather than a value a card may invent, which is why the
 * type is a union and not a string: a caller that cannot name the kind
 * of the thing it is drawing passes none, and the card falls back to the
 * placeholder it drew before.
 */
export type PackageKind =
  "flow" | "feat" | "stack" | "tool" | "mcp" | "lang" | "doc" | "app";

/** The grid the eight marks are cut on, and therefore what a stroke is in. */
const GRID = "0 0 24 24";

/**
 * The eight marks, as contours.
 *
 * Each one is the plainest available picture of what the kind IS, and
 * none of them is a letter: a card in a row of cards is scanned rather
 * than read, and a monogram would have to be learnt before it said
 * anything. A flow is a course that turns and keeps going; a feat is a
 * spark, the one thing a product does; a stack is layers; a tool is a
 * wrench; an mcp is a plug an agent connects to; a lang is braces, the
 * shape of written code; a doc is a page with lines on it; an app is a
 * window with a frame of its own.
 *
 * They are geometry and nothing else — no weight, no colour, no size.
 * All three belong to the stylesheet beside this file, so that eight
 * marks drawn at three sizes in two themes are one hand rather than
 * eight, and so that changing the weight is one edit rather than eight.
 *
 * `satisfies` is the whole exhaustiveness check: a kind admitted to the
 * union above and not drawn here fails the build, which is the only
 * moment such an omission is cheap to notice.
 */
const CONTOURS = {
  flow: () => (
    <>
      <path d="M4 5h8.5a3.5 3.5 0 0 1 0 7h-2a3.5 3.5 0 0 0 0 7H20" />
      <path d="M17 16l3 3-3 3" />
      <circle class="kind-glyph__solid" cx="4" cy="5" r="1.25" />
    </>
  ),
  feat: () => (
    <path d="M12 3l2.2 6.8L21 12l-6.8 2.2L12 21l-2.2-6.8L3 12l6.8-2.2z" />
  ),
  stack: () => (
    <>
      <path d="M12 4l8 4-8 4-8-4z" />
      <path d="M4 12l8 4 8-4" />
      <path d="M4 16l8 4 8-4" />
    </>
  ),
  tool: () => (
    <path d="M20.5 7.5a5 5 0 0 1-6.6 6.6L7 21a2 2 0 0 1-3-3l6.9-6.9a5 5 0 0 1 6.6-6.6l-3 3 .5 2.5 2.5.5z" />
  ),
  mcp: () => (
    <>
      <path d="M9 3v4M15 3v4" />
      <path d="M6 7h12v3a6 6 0 0 1-12 0z" />
      <path d="M12 16v5" />
    </>
  ),
  lang: () => (
    <>
      <path d="M8.5 4C6.5 4 5.5 5 5.5 7v3c0 1-.8 2-2 2 1.2 0 2 1 2 2v3c0 2 1 3 3 3" />
      <path d="M15.5 4c2 0 3 1 3 3v3c0 1 .8 2 2 2-1.2 0-2 1-2 2v3c0 2-1 3-3 3" />
    </>
  ),
  doc: () => (
    <>
      <path d="M6 3h8l5 5v13H6z" />
      <path d="M14 3v5h5" />
      <path d="M9 12h7M9 16h7" />
    </>
  ),
  app: () => (
    <>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <path d="M3 9.5h18" />
      <path d="M6.25 7.25h.01M9.25 7.25h.01" />
    </>
  ),
} as const satisfies Record<PackageKind, () => JSXOutput>;

/**
 * What the mark is called for a reader who cannot see it.
 *
 * The kind's own word, with the plain noun after it. The word is the
 * manifest's and is not translated, for the reason the standings are
 * not — it is the vocabulary of a field, and a reader comparing a card
 * against the package behind it must find the same word in both. The
 * noun is there because a bare «flow» announced beside a title reads as
 * part of the title.
 *
 * It is not exported through the seam: the mark and its name are one
 * decision, and the application never needs to say it twice.
 */
export function kindLabel(kind: PackageKind): string {
  return `${kind} package`;
}

export type KindGlyphProps = {
  readonly kind: PackageKind;
};

/**
 * The mark of one package kind, drawn rather than fetched.
 *
 * It is drawn for the reason every other picture on this site is: a mark
 * pulled from someone else's host would tell that host who is reading
 * the documentation (D-20, R-09). It is drawn rather than typed for a
 * second reason — a character out of a font is that font's opinion about
 * a wrench, and eight kinds picked out of whatever a reader's system
 * happens to ship would not be a set.
 *
 * The element carries no accessible name and is `aria-hidden`. The name
 * belongs to the thing the mark stands FOR — the card, the head of a
 * page — which is where a reader is told what kind of package they are
 * looking at, and where a caller that has no kind to name shows nothing
 * at all rather than an empty label.
 */
export const KindGlyph = component$<KindGlyphProps>((props) => {
  useStyles$(styles);
  return (
    <svg class="kind-glyph" viewBox={GRID} aria-hidden="true">
      {CONTOURS[props.kind]()}
    </svg>
  );
});
