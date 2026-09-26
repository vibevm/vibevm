/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

import {
  VisionArt,
  WhyAiNativeArt,
  WhyVibevmArt,
  WhyZapArt,
} from "./art-story.tsx";
import {
  DocumentationArt,
  GithubArt,
  GitverseArt,
  NewsArt,
} from "./art-tools.tsx";
import type { MenuId } from "./menu.ts";

export type MenuArtProps = {
  /** Which destination's drawing this is. */
  readonly id: MenuId;
};

/**
 * The drawing of one destination on the map — one of the eight, by the
 * id the header's list names it with (`menu.ts`).
 *
 * A switch and not a table, for the reason the compiler is asked to be
 * in the room at all: the cases are the ids, the switch is exhaustive,
 * and a destination added to the list without a drawing is a type error
 * here rather than a plate with a hole where its picture should be. The
 * drawings themselves live in two files by row — the first row's tools
 * and the second row's argument — because eight of them in one file
 * would cross the discipline's file budget.
 *
 * Nothing here is announced: every drawing is `aria-hidden`, and the
 * plate around it carries the description once, on a `role="img"`.
 */
export const MenuArt = component$<MenuArtProps>((props) => {
  switch (props.id) {
    case "documentation":
      return <DocumentationArt />;
    case "github":
      return <GithubArt />;
    case "gitverse":
      return <GitverseArt />;
    case "news":
      return <NewsArt />;
    case "vision":
      return <VisionArt />;
    case "why-vibevm":
      return <WhyVibevmArt />;
    case "why-zap":
      return <WhyZapArt />;
    case "why-ai-native":
      return <WhyAiNativeArt />;
  }
});
