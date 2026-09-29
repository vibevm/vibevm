/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The shape of the copy `/why/ai-native` is written in.
 *
 * Every value is the owner's, moved byte for byte from the Astro
 * component that carried it (`src/components/why/WhyAiNative.astro`) —
 * the same exception the landing's string table records, for the same
 * reason.
 *
 * The page keeps the words «AI-Native Language» while its address is
 * `/why/ai-native`. That is deliberate: the owner settled the ADDRESS,
 * and the copy is the copy. A port that retitled the page on its way
 * across would be editing the owner's text to agree with a routing
 * decision.
 */

import type { Floor } from "./artifacts.ts";

export type Stack = {
  readonly mark: Floor["key"];
  readonly head: string;
  readonly body: string;
};

export type FamilyMember = {
  readonly mark: Floor["key"];
  readonly name: string;
  readonly tag: string;
  readonly body: string;
};

export type StatusColumn = {
  readonly head: string;
  readonly items: readonly string[];
};

export type Step =
  | {
      readonly kind: "cmd";
      readonly cmd: string;
      readonly body: string;
      readonly linkText?: string;
    }
  | {
      readonly kind: "head";
      readonly head: string;
      readonly body: string;
    };

export type Strings = {
  readonly eyebrow: string;
  readonly badge: string;
  readonly headlineHtml: string;
  readonly lead: string;
  readonly ctaTry: string;
  readonly ctaVibevm: string;
  readonly heroAlt: string;
  readonly heroCaption: string;
  /**
   * The doorway into the essay, standing in the hero. These three are
   * the page's own words about `/vision/`, added with the essay itself
   * (2026-09) — not part of the byte-for-byte Astro copy above, and
   * marked so the copy's provenance claim stays checkable.
   */
  readonly visionK: string;
  readonly visionHead: string;
  readonly visionSub: string;

  readonly problemK: string;
  readonly problemH: string;
  readonly problemBody: string;
  readonly problemPoints: readonly string[];

  readonly lawK: string;
  readonly lawQuoteHtml: string;
  readonly lawBody: string;
  readonly lawNote: string;
  readonly envelopeAlt: string;
  readonly envelopeCaption: string;
  readonly contractHead: string;
  readonly contractBodyHtml: string;

  readonly archK: string;
  readonly archH: string;
  readonly archBody: string;
  readonly archAlt: string;
  readonly archCoreHead: string;
  readonly archCoreTag: string;
  readonly archCoreBody: string;
  readonly archStacks: readonly Stack[];
  readonly archProjHead: string;
  readonly archProjTag: string;
  readonly archProjBody: string;
  readonly archNote: string;

  readonly compareK: string;
  readonly compareH: string;
  readonly compareBody: string;
  readonly compareYours: string;
  readonly compareAdded: string;
  readonly compareAdds: Readonly<Record<Floor["key"], readonly string[]>>;

  readonly mechK: string;
  readonly mechH: string;
  readonly m1H: string;
  readonly m1Body: string;
  readonly m1BeforeLabel: string;
  readonly m1AfterLabel: string;
  readonly m1Caption: string;
  readonly m1TagsLabel: string;
  readonly m2H: string;
  readonly m2Body: string;
  readonly m3H: string;
  readonly m3Body: string;
  readonly m3Caption: string;
  readonly m4H: string;
  readonly m4Body: string;
  readonly m4Caption: string;
  readonly m5H: string;
  readonly m5Body: string;
  readonly m5Caption: string;

  readonly familyK: string;
  readonly familyH: string;
  readonly familyBody: string;
  readonly family: readonly FamilyMember[];
  readonly familyNote: string;

  readonly dogK: string;
  readonly dogH: string;
  readonly dogBody: string;
  readonly dogPoints: readonly string[];

  readonly statusK: string;
  readonly statusH: string;
  readonly statusCols: readonly StatusColumn[];
  readonly statusTail: string;

  readonly startK: string;
  readonly startH: string;
  readonly steps: readonly Step[];
  readonly ctaGithub: string;
  readonly ctaGitverse: string;
  readonly tieVibevmHead: string;
  readonly tieVibevmBody: string;
  readonly tieVibevmLink: string;
  readonly tieZapHead: string;
  readonly tieZapBody: string;
  readonly tieZapLink: string;

  readonly summaryK: string;
  readonly summaryHtml: string;
};
