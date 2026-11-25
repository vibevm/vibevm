/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The shape of the roadmap's copy.
 *
 * The English edition is the source and the Russian edition is an
 * adaptation of it — the same six stages in the same order, written in
 * its own sentences rather than translated word for word. The two live in
 * `copy-en.ts` and `copy-ru.ts` and are assembled by `i18n.ts`, the split
 * the essay and the AI-Native page keep and for the same two reasons: an
 * editor works on one edition at a time, and the discipline's file
 * budget is real.
 *
 * The field names follow the page's own spine: the hero, the marker that
 * says where the project stands today, the six stages, the one rule the
 * page asks to be read by, and the three ways out.
 */

/**
 * The six stages, by a short name each, in the owner's order
 * (2026-09-26). The order is the content: a roadmap is a sequence before
 * it is a set, and a tuple type keeps the sequence where the compiler
 * can see it.
 */
export const STAGE_IDS = [
  "dp2",
  "zap",
  "community",
  "ide",
  "library",
  "linux",
] as const;

export type StageId = (typeof STAGE_IDS)[number];

/**
 * A stage's one way out, when it has one: Zap has a page of its own on
 * this site, and Spec-Driven Linux has a future home off it. The address
 * is not here — it is resolved by the page from the stage's id, so a
 * string table never carries a path (`lib/href.ts`).
 */
export type StageLink = {
  /** The words before the link. */
  readonly lead: string;
  /** The link's own text. */
  readonly text: string;
};

/** One stage of the road: what it is called, what it is, what it changes. */
export type Stage = {
  readonly id: StageId;
  readonly name: string;
  /** One sentence: what the stage IS. */
  readonly sub: string;
  /** A short paragraph: what it changes for the reader. */
  readonly body: string;
  /** The stage's emblem, in words — the figure's `aria-label`. */
  readonly alt: string;
  readonly link?: StageLink;
};

export type Stages = readonly [Stage, Stage, Stage, Stage, Stage, Stage];

export type RoadmapStrings = {
  readonly eyebrow: string;
  readonly title: string;
  readonly lead: string;
  readonly heroAlt: string;
  readonly heroCaption: string;

  /** The marker: where the project stands today. */
  readonly hereK: string;
  readonly hereName: string;
  readonly hereBody: string;
  readonly hereAlt: string;

  /** The list of stages, and how a screen reader is told what it is. */
  readonly stagesK: string;
  readonly stagesH: string;
  readonly routeLabel: string;
  readonly stages: Stages;

  /** The rule on the inverted band: how this page is to be read. */
  readonly bandK: string;
  readonly bandQuote: string;
  readonly bandBody: string;

  /** The three ways out: the news, the essay, the manual. */
  readonly tiesK: string;
  readonly tieNewsHead: string;
  readonly tieNewsBody: string;
  readonly tieNewsLink: string;
  readonly tieVisionHead: string;
  readonly tieVisionBody: string;
  readonly tieVisionLink: string;
  readonly tieDocsHead: string;
  readonly tieDocsBody: string;
  readonly tieDocsLink: string;
};
