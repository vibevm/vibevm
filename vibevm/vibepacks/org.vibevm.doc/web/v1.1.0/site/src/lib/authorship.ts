/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP */

/**
 * Who wrote the prose, as a shelf is narrowed by it.
 *
 * The manifest answers with one of three words or with silence, and a
 * reader is offered two named groups and «all» — which is three answers
 * offered over four possible states, so the mapping between them is the
 * whole of this file and is written down once for the two places that
 * need it: the door, which renders the control, and the behaviour, which
 * hides the cards the control excludes.
 *
 * Two rules make that mapping, and both come from `##CARD-AUTHORSHIP`.
 *
 * **`mixed` is in both named groups.** Both hands are in the text, so a
 * reader asking for what people wrote and a reader asking for what a
 * model wrote are each being told the truth about it. Putting it in a
 * third group of its own would answer neither of their questions.
 *
 * **Silence is in neither.** A documentation that declares nothing is
 * unknown, not human and not generated, and it shows up under «all» and
 * nowhere else. The alternative — reading silence as «a person wrote
 * it», which is what a site would do by defaulting — is the one claim
 * this field exists to stop being made by accident, and it is the claim
 * a reader would most be misled by.
 *
 * Nothing here reaches for a library or a card. It is a rule about one
 * value, so it takes one value, which is what lets the door, the
 * browser behaviour and a test all ask the same function.
 */

import { Authorship } from "../generated/doc-manifest.ts";

import type { AuthorshipChoice } from "@vibe-docs/design";

/**
 * The entry that hides nothing, spelled as the language filter spells
 * its own: `*` is not a word of the vocabulary and cannot be mistaken
 * for one.
 */
export const EVERY_AUTHORSHIP = "*";

/**
 * Whether a card is admitted by a choice.
 *
 * `declared` is what the card carries — one of the three words, or
 * nothing at all when the documentation said nothing. A word this build
 * does not know behaves as silence rather than as an error: a manifest
 * from a newer pipeline is a document to be shelved, not a page to
 * refuse, and the named groups stay exactly what they say they are.
 */
export function admitsAuthorship(
  choice: string,
  declared: string | undefined,
): boolean {
  if (choice === EVERY_AUTHORSHIP) return true;
  if (declared === Authorship.Mixed) {
    return choice === Authorship.Human || choice === Authorship.Ai;
  }
  return declared === choice;
}

/**
 * The three entries the control offers, in the order it offers them:
 * everything first, then the two named groups.
 *
 * «All» stands first because it is where a reader starts and where they
 * come back to, and because the note under it is the only place the
 * silence rule can be stated in words — a group that is not offered
 * cannot explain itself.
 *
 * The current entry is «all» as the build writes the page: which group a
 * reader last asked for is remembered in their own browser and marked
 * there, exactly as the language filter's is.
 */
export function authorshipChoices(): AuthorshipChoice[] {
  return [
    {
      value: EVERY_AUTHORSHIP,
      label: "All",
      /* The closed pill says «anyone» and not «all». The language filter
         beside it prints «all» when it is narrowing nothing, and two
         pills reading ALL in one row would be two controls a reader
         cannot tell apart without opening either. «Anyone» is also the
         honest answer to the question this one asks. */
      note: "every documentation, including the ones that do not say who wrote them",
      pill: "anyone",
      current: true,
    },
    {
      value: Authorship.Human,
      label: "Human-authored",
      note: "a person wrote the prose; a document written by both hands is here too",
      pill: "human",
      current: false,
    },
    {
      value: Authorship.Ai,
      label: "AI-generated",
      note: "a model wrote the prose; a document written by both hands is here too",
      pill: "ai",
      current: false,
    },
  ];
}
