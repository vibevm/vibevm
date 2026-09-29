/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import styles from "./field.css?inline";

/**
 * The ground the whole landing stands on: four large planes and a few
 * drafting marks, laid behind everything from the hero to the map.
 *
 * The owner asked for the page to read as one work in the suprematist
 * language of the essay and the AI-Native page, not as a page with an
 * illustrated section at the bottom. A composition is one when its parts
 * share a ground, and this is the ground: a tilted terracotta plane
 * behind the hero's constellation, a long thin ink bar cutting under the
 * install panel toward the capability cards, a cobalt square and a gold
 * disc behind the map's two bands — the family's four masses, at the
 * faintness of the glow the page already had, so that the text over them
 * stays what the contrast audit measured. The hero, the cards and the
 * plates are opaque and cut the planes where they stand; that overlap is
 * the composition, not an accident of it.
 *
 * It is markup rather than a background image because a plane drawn in
 * CSS takes its colour from a token and follows the theme, and it is a
 * few empty elements rather than one SVG because an SVG stretched over a
 * page whose height depends on the language would distort every shape.
 * Decorative and said so: hidden from the accessibility tree, under the
 * pointer, clipped to the page so nothing here can widen it.
 */
export const LandingField = component$(() => {
  useStyles$(styles);
  return (
    <div class="landing-field" aria-hidden="true">
      <i class="landing-field__plane landing-field__plane--a" />
      <i class="landing-field__plane landing-field__plane--b" />
      <i class="landing-field__plane landing-field__plane--c" />
      <i class="landing-field__plane landing-field__plane--d" />
      <i class="landing-field__cross landing-field__cross--1" />
      <i class="landing-field__cross landing-field__cross--2" />
      <i class="landing-field__cross landing-field__cross--3" />
      <i class="landing-field__cross landing-field__cross--4" />
    </div>
  );
});
