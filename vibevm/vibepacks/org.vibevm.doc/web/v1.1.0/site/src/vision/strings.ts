/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The shape of the essay's copy.
 *
 * The Russian edition is the owner's original, carried through a
 * proofreading pass that fixed typos, agreement and punctuation and
 * changed nothing the author meant; the English edition is an
 * adaptation — the same argument written for an English reader, not a
 * word-for-word translation. The two live in `copy-ru.ts` and
 * `copy-en.ts` and are assembled by `i18n.ts`, the same split the
 * AI-Native page keeps and for the same two reasons: an editor works on
 * one edition at a time, and the discipline's six-hundred-line file
 * budget is real.
 *
 * The field names follow the essay's own sections rather than a generic
 * scheme, so that a reader of this file sees the argument's spine: the
 * lede, the new beings, the second source of intention, SDD and its
 * price, the specification/code split, the one law on the band, the
 * traceable edges, the collaboration close, the starting point, and the
 * disclaimer the author insists stays visible.
 */

export type VisionStrings = {
  readonly eyebrow: string;
  readonly title: string;
  readonly heroAlt: string;
  readonly heroCaption: string;

  /** The opening paragraph, set large — the essay's own first words. */
  readonly lede: string;

  readonly beingsH: string;
  readonly beingsP1: string;
  readonly beingsP2: string;

  readonly intentH: string;
  readonly intentP1: string;
  readonly intentP2: string;
  readonly intentP3: string;

  readonly sddH: string;
  readonly sddP1: string;
  readonly sddP2: string;
  readonly sddP3: string;
  readonly starAlt: string;
  readonly starCaption: string;

  readonly splitH: string;
  readonly splitP1: string;
  readonly splitP2: string;
  readonly splitP3: string;
  readonly splitP4: string;
  /** The two-column inset: the dichotomy, in the essay's own words. */
  readonly dichotomyLabel: string;
  readonly specHead: string;
  readonly specPoints: readonly string[];
  readonly codeHead: string;
  readonly codePoints: readonly string[];
  readonly splitP5: string;
  readonly splitP6: string;
  readonly layersAlt: string;
  readonly layersCaption: string;

  /** The law on the inverted band: lead-in, the rule, the rest. */
  readonly lawK: string;
  readonly lawLead: string;
  readonly lawQuote: string;
  readonly lawBody: string;

  readonly edgesH: string;
  readonly edgesP1: string;
  readonly edgesP2: string;
  readonly edgesP3: string;
  /** The OOP sentence, pulled out of the paragraph it closes. */
  readonly oopQuote: string;
  readonly ladderAlt: string;
  readonly ladderCaption: string;

  readonly togetherH: string;
  readonly togetherP1: string;

  readonly startH: string;
  readonly startP1: string;

  readonly disclaimerK: string;
  /** Carries one `<em>` around the word the author set apart. */
  readonly disclaimerHtml: string;

  /** The one way out: where the worldview becomes a discipline. */
  readonly tieK: string;
  readonly tieHead: string;
  readonly tieBody: string;
  readonly tieLink: string;
};
