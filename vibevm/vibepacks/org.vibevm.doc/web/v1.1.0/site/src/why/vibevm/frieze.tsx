/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The hero frieze of `/why/vibevm`, read left to right like the
 * product's own story.
 *
 * Scattered tilted rectangles are hand-copied conventions drifting
 * apart; the terracotta wedge is an installed discipline entering the
 * project; past it, packages align into a resolved dependency lattice,
 * pinned at every node — the lockfile. The hollow ring is the registry
 * the wedge was fetched from.
 *
 * It is authored as JSX rather than fetched as a file or written in
 * through `dangerouslySetInnerHTML`, and that is the whole point: every
 * class on it is a hook the page's stylesheet colours through a token,
 * so the drawing follows the theme the reader chose instead of carrying
 * a hex of its own. Astro inlined the same picture with `set:html` from
 * a `?raw` import, which is the same bytes arriving unchecked; here the
 * compiler sees them.
 *
 * Every coordinate and every delay is the source drawing's, to the unit
 * — `pathLength="1"` normalises each edge so one dash pattern draws all
 * fourteen of them, and each `--d` is written on the element so the
 * choreography lives where the shape is.
 *
 * No hydration: the animation is CSS, there is no handler, and a reader
 * who asked for stillness is answered by the stylesheet rather than by a
 * script that has to load first.
 */
export const Frieze = component$(() => (
  <svg
    class="vv-frieze"
    viewBox="0 0 1200 420"
    fill="none"
    aria-hidden="true"
    focusable="false"
    preserveAspectRatio="xMidYMid meet"
  >
    {/* Registry ring, arcing off the canvas */}
    <circle class="vv-frieze__ring" cx="170" cy="24" r="110" />

    {/* Chaos: unmanaged instruction files */}
    <g class="vv-frieze__chaos">
      <rect
        x="60"
        y="110"
        width="44"
        height="28"
        transform="rotate(-14 82 124)"
      />
      <rect
        x="150"
        y="70"
        width="32"
        height="21"
        transform="rotate(11 166 80)"
      />
      <rect
        x="110"
        y="190"
        width="36"
        height="23"
        transform="rotate(-7 128 201)"
      />
      <rect
        x="215"
        y="140"
        width="28"
        height="19"
        transform="rotate(21 229 149)"
      />
      <rect
        x="75"
        y="280"
        width="32"
        height="21"
        transform="rotate(-18 91 290)"
      />
      <rect
        x="185"
        y="250"
        width="40"
        height="25"
        transform="rotate(6 205 262)"
      />
      <rect
        x="135"
        y="340"
        width="30"
        height="19"
        transform="rotate(-11 150 349)"
      />
      <rect
        x="255"
        y="300"
        width="26"
        height="17"
        transform="rotate(16 268 308)"
      />
      <rect
        class="vv-frieze__bright"
        x="250"
        y="60"
        width="24"
        height="16"
        transform="rotate(-21 262 68)"
      />
      <rect
        class="vv-frieze__bright"
        x="300"
        y="200"
        width="28"
        height="18"
        transform="rotate(9 314 209)"
      />
      <rect
        class="vv-frieze__turn"
        x="360"
        y="240"
        width="27"
        height="17"
        transform="rotate(-6 373 248)"
      />
      <rect
        class="vv-frieze__turn"
        x="415"
        y="262"
        width="27"
        height="17"
        transform="rotate(-2 428 270)"
      />
    </g>

    {/* The wedge: discipline entering the project */}
    <polygon class="vv-frieze__wedge" points="1206,-30 1206,150 330,228" />

    {/* Counterweight bar */}
    <rect
      class="vv-frieze__bar"
      x="430"
      y="330"
      width="90"
      height="10"
      transform="rotate(-32 475 335)"
    />

    {/* Order: the resolved, pinned lattice */}
    <g class="vv-frieze__edges">
      <line
        x1="560"
        y1="330"
        x2="640"
        y2="290"
        pathLength="1"
        style="--d:.55s"
      />
      <line
        x1="560"
        y1="330"
        x2="640"
        y2="370"
        pathLength="1"
        style="--d:.62s"
      />
      <line
        x1="640"
        y1="290"
        x2="720"
        y2="270"
        pathLength="1"
        style="--d:.7s"
      />
      <line
        x1="640"
        y1="290"
        x2="720"
        y2="310"
        pathLength="1"
        style="--d:.74s"
      />
      <line
        x1="640"
        y1="370"
        x2="720"
        y2="350"
        pathLength="1"
        style="--d:.78s"
      />
      <line
        x1="640"
        y1="370"
        x2="720"
        y2="390"
        pathLength="1"
        style="--d:.82s"
      />
      <line
        x1="720"
        y1="310"
        x2="800"
        y2="290"
        pathLength="1"
        style="--d:.9s"
      />
      <line
        x1="720"
        y1="310"
        x2="800"
        y2="330"
        pathLength="1"
        style="--d:.94s"
      />
      <line
        x1="720"
        y1="350"
        x2="800"
        y2="370"
        pathLength="1"
        style="--d:.98s"
      />
      <line
        x1="800"
        y1="290"
        x2="880"
        y2="270"
        pathLength="1"
        style="--d:1.06s"
      />
      <line
        x1="800"
        y1="330"
        x2="880"
        y2="330"
        pathLength="1"
        style="--d:1.1s"
      />
      <line
        x1="800"
        y1="370"
        x2="960"
        y2="390"
        pathLength="1"
        style="--d:1.14s"
      />
      <line
        x1="880"
        y1="330"
        x2="960"
        y2="330"
        pathLength="1"
        style="--d:1.2s"
      />
      <line
        x1="960"
        y1="330"
        x2="1040"
        y2="330"
        pathLength="1"
        style="--d:1.26s"
      />
    </g>
    <g class="vv-frieze__nodes">
      <rect
        class="vv-frieze__root"
        x="549"
        y="319"
        width="22"
        height="22"
        style="--d:.45s"
      />
      <rect
        class="vv-frieze__sq"
        x="633"
        y="283"
        width="14"
        height="14"
        style="--d:.62s"
      />
      <rect
        class="vv-frieze__sq"
        x="633"
        y="363"
        width="14"
        height="14"
        style="--d:.68s"
      />
      <circle
        class="vv-frieze__pin"
        cx="720"
        cy="270"
        r="6.5"
        style="--d:.78s"
      />
      <rect
        class="vv-frieze__sq"
        x="713.5"
        y="303.5"
        width="13"
        height="13"
        style="--d:.82s"
      />
      <circle
        class="vv-frieze__pin"
        cx="720"
        cy="350"
        r="6.5"
        style="--d:.86s"
      />
      <rect
        class="vv-frieze__sq"
        x="713.5"
        y="383.5"
        width="13"
        height="13"
        style="--d:.9s"
      />
      <circle class="vv-frieze__pin" cx="800" cy="290" r="6" style="--d:.98s" />
      <rect
        class="vv-frieze__sq"
        x="793.5"
        y="323.5"
        width="13"
        height="13"
        style="--d:1.02s"
      />
      <circle
        class="vv-frieze__pin"
        cx="800"
        cy="370"
        r="6"
        style="--d:1.06s"
      />
      <circle
        class="vv-frieze__pin"
        cx="880"
        cy="270"
        r="5.5"
        style="--d:1.14s"
      />
      <rect
        class="vv-frieze__sq"
        x="874"
        y="324"
        width="12"
        height="12"
        style="--d:1.18s"
      />
      <circle
        class="vv-frieze__pin"
        cx="960"
        cy="330"
        r="6"
        style="--d:1.26s"
      />
      <circle
        class="vv-frieze__pin"
        cx="960"
        cy="390"
        r="5.5"
        style="--d:1.3s"
      />
      <circle
        class="vv-frieze__pin vv-frieze__pin--faint"
        cx="1040"
        cy="330"
        r="5"
        style="--d:1.36s"
      />
    </g>
  </svg>
));
