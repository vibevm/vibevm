/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * The landing's signature: a dependency graph drawn as a constellation.
 *
 * It is the one picture on the site that is literally its subject —
 * packages resolved through their edges — and it earns its place by
 * drawing itself: the edges are stroked in, the nodes pop, and the root
 * keeps a slow ring. Nothing here is decoration hunting for a reason.
 *
 * The geometry and the timings are the Astro landing's, coordinate for
 * coordinate (D-28: the landing moves one to one). Two details are worth
 * knowing before touching them. Every edge carries `pathLength="1"`,
 * which normalises its length to one unit so the draw animation needs no
 * geometry arithmetic — one dash pattern fits all ten lines. And every
 * delay is a `--d` custom property written on the element itself, so the
 * choreography lives in the markup where the shape is and the stylesheet
 * stays a single rule per kind.
 *
 * No hydration. Qwik renders this to markup and never touches it again:
 * the animation is CSS, there is no handler, and a reader who asked for
 * stillness is answered by the stylesheet rather than by a script that
 * has to load first.
 */
export const DepGraph = component$(() => {
  useStyles$(styles);
  return (
    <svg
      class="dep-graph"
      viewBox="0 0 420 380"
      role="img"
      aria-label="A dependency graph: one root package resolving its transitive dependencies."
    >
      <g class="dep-graph__edges">
        <line
          class="dep-graph__edge"
          x1="210"
          y1="196"
          x2="150"
          y2="120"
          pathLength="1"
          style="--d:.25s"
        />
        <line
          class="dep-graph__edge"
          x1="210"
          y1="196"
          x2="286"
          y2="128"
          pathLength="1"
          style="--d:.32s"
        />
        <line
          class="dep-graph__edge"
          x1="210"
          y1="196"
          x2="176"
          y2="300"
          pathLength="1"
          style="--d:.4s"
        />
        <line
          class="dep-graph__edge"
          x1="210"
          y1="196"
          x2="300"
          y2="256"
          pathLength="1"
          style="--d:.48s"
        />
        <line
          class="dep-graph__edge"
          x1="150"
          y1="120"
          x2="78"
          y2="72"
          pathLength="1"
          style="--d:.6s"
        />
        <line
          class="dep-graph__edge"
          x1="150"
          y1="120"
          x2="70"
          y2="196"
          pathLength="1"
          style="--d:.68s"
        />
        <line
          class="dep-graph__edge"
          x1="286"
          y1="128"
          x2="352"
          y2="80"
          pathLength="1"
          style="--d:.72s"
        />
        <line
          class="dep-graph__edge"
          x1="286"
          y1="128"
          x2="336"
          y2="196"
          pathLength="1"
          style="--d:.8s"
        />
        <line
          class="dep-graph__edge"
          x1="300"
          y1="256"
          x2="352"
          y2="322"
          pathLength="1"
          style="--d:.86s"
        />
        <line
          class="dep-graph__edge"
          x1="176"
          y1="300"
          x2="112"
          y2="330"
          pathLength="1"
          style="--d:.9s"
        />
      </g>
      <g class="dep-graph__nodes">
        <circle
          class="dep-graph__node dep-graph__node--solid"
          cx="210"
          cy="196"
          r="15"
          style="--d:.15s"
        />
        <circle class="dep-graph__pulse" cx="210" cy="196" r="15" />
        <circle
          class="dep-graph__node"
          cx="150"
          cy="120"
          r="9"
          style="--d:.55s"
        />
        <circle
          class="dep-graph__node"
          cx="286"
          cy="128"
          r="10"
          style="--d:.6s"
        />
        <circle
          class="dep-graph__node"
          cx="176"
          cy="300"
          r="8"
          style="--d:.68s"
        />
        <circle
          class="dep-graph__node"
          cx="300"
          cy="256"
          r="9"
          style="--d:.72s"
        />
        <circle class="dep-graph__node" cx="78" cy="72" r="6" style="--d:.9s" />
        <circle
          class="dep-graph__node"
          cx="70"
          cy="196"
          r="6"
          style="--d:.96s"
        />
        <circle class="dep-graph__node" cx="352" cy="80" r="7" style="--d:1s" />
        <circle
          class="dep-graph__node"
          cx="336"
          cy="196"
          r="6"
          style="--d:1.04s"
        />
        <circle
          class="dep-graph__node"
          cx="352"
          cy="322"
          r="6"
          style="--d:1.1s"
        />
        <circle
          class="dep-graph__node"
          cx="112"
          cy="330"
          r="5"
          style="--d:1.14s"
        />
      </g>
    </svg>
  );
});
