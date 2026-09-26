/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The map's drawings for the header's second row — the argument: the
 * essay and the three products it is the worldview of. Each is the
 * page's own signature drawn small, in the page's own vocabulary and
 * colours, so a reader who has seen the page recognises the plate and
 * a reader who has not learns the vocabulary before opening it:
 *
 *   · the essay — the terracotta circle of human intention, the tilted
 *     cobalt square of machine intention, the gold arc of the
 *     specification between them, edges dropping to one floor;
 *   · VibeVM — the terracotta wedge of an installed discipline entering
 *     a field of drift and resolving into a pinned lattice;
 *   · Zap — the green orbital map: a goal core, dashed rings of
 *     decomposition, one trajectory leaving, one terracotta node for the
 *     project in the family;
 *   · AI-Native — the gold core projecting onto three language marks,
 *     each clamped by the discipline's brackets, on one floor with one
 *     exit notch.
 *
 * Every element that draws or pops in carries `--at`, its place in the
 * choreography (`map.css`); a rotated shape pops inside a group, so the
 * pop's transform never replaces the rotation it was drawn with.
 */

/** Vision: two sources of intention over one deterministic floor. */
export const VisionArt = component$(() => (
  <svg
    class="lm-vision"
    viewBox="0 0 560 200"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-cross" stroke-width="1">
      <path d="M60 30v12M54 36h12" />
      <path d="M300 22v12M294 28h12" />
      <path d="M520 62v12M514 68h12" />
    </g>
    <line
      class="lm-vision__floor lm-draw"
      x1="28"
      y1="176"
      x2="532"
      y2="176"
      pathLength="1"
      style="--at:0"
    />
    <g class="lm-tick" stroke-width="1">
      <path d="M100 173v6" />
      <path d="M180 173v6" />
      <path d="M260 173v6" />
      <path d="M340 173v6" />
      <path d="M420 173v6" />
      <path d="M500 173v6" />
    </g>
    <path
      class="lm-vision__arc lm-draw"
      d="M196 62C250 4 316 6 348 46"
      pathLength="1"
      style="--at:6"
    />
    <g class="lm-pop" style="--at:8">
      <circle class="lm-vision__halo" cx="150" cy="88" r="58" />
      <circle class="lm-vision__human" cx="150" cy="88" r="46" />
    </g>
    <g class="lm-pop" style="--at:12">
      <rect
        class="lm-vision__machine"
        x="341"
        y="49"
        width="78"
        height="78"
        transform="rotate(12 380 88)"
      />
      <path
        class="lm-vision__hair"
        d="M380 30v12M380 134v12M322 88h-12M438 88h12"
      />
    </g>
    <g class="lm-vision__edges">
      <path class="lm-draw" d="M150 134v40" pathLength="1" style="--at:14" />
      <path class="lm-draw" d="M382 132v42" pathLength="1" style="--at:18" />
      <path
        class="lm-draw"
        d="M190 118 244 140"
        pathLength="1"
        style="--at:16"
      />
      <path class="lm-draw" d="M244 140v34" pathLength="1" style="--at:20" />
      <path
        class="lm-draw"
        d="M338 116 300 148"
        pathLength="1"
        style="--at:18"
      />
      <path class="lm-draw" d="M300 148v26" pathLength="1" style="--at:22" />
    </g>
    <rect
      class="lm-vision__node-m lm-pop"
      x="238"
      y="134"
      width="12"
      height="12"
      style="--at:22"
    />
    <circle
      class="lm-vision__node-h lm-pop"
      cx="300"
      cy="148"
      r="5"
      style="--at:24"
    />
  </svg>
));

/** Why VibeVM: the wedge of discipline entering a field of drift. */
export const WhyVibevmArt = component$(() => (
  <svg
    class="lm-vv"
    viewBox="0 0 300 190"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-vv__chaos">
      <rect
        x="24"
        y="50"
        width="26"
        height="17"
        transform="rotate(-14 37 58)"
      />
      <rect x="58" y="26" width="20" height="13" transform="rotate(11 68 32)" />
      <rect
        x="44"
        y="98"
        width="22"
        height="14"
        transform="rotate(-7 55 105)"
      />
      <rect x="86" y="70" width="18" height="12" transform="rotate(21 95 76)" />
      <rect
        x="30"
        y="142"
        width="20"
        height="13"
        transform="rotate(-18 40 148)"
      />
      <rect
        x="74"
        y="124"
        width="24"
        height="15"
        transform="rotate(6 86 131)"
      />
      <rect
        x="104"
        y="150"
        width="18"
        height="12"
        transform="rotate(-11 113 156)"
      />
      <rect
        class="lm-vv__bright"
        x="118"
        y="42"
        width="16"
        height="11"
        transform="rotate(-21 126 47)"
      />
    </g>
    <polygon class="lm-vv__wedge lm-enter" points="306,-8 306,84 112,124" />
    <rect
      class="lm-vv__bar"
      x="118"
      y="150"
      width="52"
      height="6"
      transform="rotate(-32 144 153)"
    />
    <g class="lm-vv__edges">
      <line
        class="lm-draw"
        x1="176"
        y1="142"
        x2="214"
        y2="124"
        pathLength="1"
        style="--at:8"
      />
      <line
        class="lm-draw"
        x1="176"
        y1="142"
        x2="214"
        y2="160"
        pathLength="1"
        style="--at:10"
      />
      <line
        class="lm-draw"
        x1="214"
        y1="124"
        x2="252"
        y2="114"
        pathLength="1"
        style="--at:12"
      />
      <line
        class="lm-draw"
        x1="214"
        y1="124"
        x2="252"
        y2="134"
        pathLength="1"
        style="--at:14"
      />
      <line
        class="lm-draw"
        x1="214"
        y1="160"
        x2="252"
        y2="156"
        pathLength="1"
        style="--at:16"
      />
      <line
        class="lm-draw"
        x1="252"
        y1="134"
        x2="288"
        y2="134"
        pathLength="1"
        style="--at:18"
      />
    </g>
    <rect
      class="lm-vv__root lm-pop"
      x="168"
      y="134"
      width="16"
      height="16"
      style="--at:6"
    />
    <rect
      class="lm-vv__sq lm-pop"
      x="208"
      y="118"
      width="12"
      height="12"
      style="--at:12"
    />
    <rect
      class="lm-vv__sq lm-pop"
      x="208"
      y="154"
      width="12"
      height="12"
      style="--at:14"
    />
    <circle class="lm-vv__pin lm-pop" cx="252" cy="114" r="5" style="--at:16" />
    <circle class="lm-vv__pin lm-pop" cx="252" cy="134" r="5" style="--at:18" />
    <circle class="lm-vv__pin lm-pop" cx="252" cy="156" r="5" style="--at:20" />
    <circle
      class="lm-vv__pin lm-vv__pin--faint lm-pop"
      cx="288"
      cy="134"
      r="4.5"
      style="--at:22"
    />
  </svg>
));

/** Why Zap: one campaign as an orbital map, announced and not shipped. */
export const WhyZapArt = component$(() => (
  <svg
    class="lm-zap"
    viewBox="0 0 300 190"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-zap__stars">
      <circle cx="24" cy="28" r="1.2" />
      <circle cx="88" cy="18" r="1.1" />
      <circle cx="182" cy="26" r="1.4" />
      <circle cx="262" cy="20" r="1.1" />
      <circle cx="284" cy="74" r="1.3" />
      <circle cx="22" cy="120" r="1.2" />
      <circle cx="150" cy="178" r="1.1" />
      <circle cx="230" cy="170" r="1.3" />
    </g>
    <g class="lm-zap__orbits">
      <circle class="lm-zap__ring" cx="120" cy="100" r="34" />
      <circle class="lm-zap__ring" cx="120" cy="100" r="60" />
      <circle class="lm-zap__ring" cx="120" cy="100" r="86" />
      <circle
        class="lm-zap__ring lm-zap__ring--far"
        cx="120"
        cy="100"
        r="112"
      />
    </g>
    <path
      class="lm-zap__path lm-draw"
      d="M120 100 154 82 214 74 262 110 296 124"
      pathLength="1"
      style="--at:8"
    />
    <path
      class="lm-zap__arrow lm-pop"
      d="M286 114 297 124 284 129"
      style="--at:22"
    />
    <g class="lm-pop" style="--at:2">
      <circle class="lm-zap__halo" cx="120" cy="100" r="18" />
      <circle class="lm-zap__core-ring" cx="120" cy="100" r="12" />
      <circle class="lm-zap__core" cx="120" cy="100" r="7" />
    </g>
    <circle
      class="lm-zap__node lm-zap__node--fill lm-pop"
      cx="154"
      cy="82"
      r="5"
      style="--at:10"
    />
    <circle
      class="lm-zap__node lm-pop"
      cx="86"
      cy="124"
      r="4"
      style="--at:12"
    />
    <circle
      class="lm-zap__node lm-zap__node--fill lm-pop"
      cx="214"
      cy="74"
      r="5"
      style="--at:14"
    />
    <circle
      class="lm-zap__node lm-zap__node--project lm-pop"
      cx="170"
      cy="132"
      r="5"
      style="--at:16"
    />
    <circle class="lm-zap__node lm-pop" cx="62" cy="72" r="4" style="--at:18" />
    <circle
      class="lm-zap__node lm-zap__node--fill lm-pop"
      cx="262"
      cy="110"
      r="6"
      style="--at:20"
    />
  </svg>
));

/** AI-Native Language: one neutral core projected onto three languages. */
export const WhyAiNativeArt = component$(() => (
  <svg
    class="lm-an"
    viewBox="0 0 300 190"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-cross" stroke-width="1">
      <path d="M60 150v12M54 156h12" />
      <path d="M150 22v12M144 28h12" />
      <path d="M274 44v12M268 50h12" />
    </g>
    <line
      class="lm-an__floor lm-draw"
      x1="16"
      y1="172"
      x2="268"
      y2="172"
      pathLength="1"
      style="--at:0"
    />
    <g class="lm-tick" stroke-width="1">
      <path d="M90 169v6" />
      <path d="M180 169v6" />
    </g>
    <rect class="lm-an__exit" x="272" y="164" width="12" height="12" />
    <g class="lm-an__drops">
      <path d="M52 110v60" />
      <path d="M156 74v96" />
      <path d="M212 110v60" />
      <path d="M262 158v12" />
    </g>
    <rect class="lm-an__halo" x="22" y="46" width="60" height="60" />
    <rect
      class="lm-an__core lm-pop"
      x="30"
      y="54"
      width="44"
      height="44"
      style="--at:2"
    />
    <path class="lm-an__hair" d="M52 38v10M52 104v10M14 76h10M82 76h10" />
    <g class="lm-an__rays">
      <line
        class="lm-draw"
        x1="74"
        y1="72"
        x2="140"
        y2="56"
        pathLength="1"
        style="--at:6"
      />
      <line
        class="lm-draw"
        x1="74"
        y1="78"
        x2="196"
        y2="92"
        pathLength="1"
        style="--at:10"
      />
      <line
        class="lm-draw"
        x1="74"
        y1="84"
        x2="250"
        y2="128"
        pathLength="1"
        style="--at:14"
      />
    </g>
    <g class="lm-pop" style="--at:12">
      <rect class="lm-an__rust" x="140" y="40" width="32" height="32" />
      <g class="lm-an__brackets">
        <path d="M136 34H126v10" />
        <path d="M176 34h10v10" />
        <path d="M136 78H126V68" />
        <path d="M176 78h10V68" />
      </g>
    </g>
    <g class="lm-pop" style="--at:16">
      <circle class="lm-an__ts" cx="212" cy="92" r="17" />
      <g class="lm-an__brackets">
        <path d="M195 71h-10v10" />
        <path d="M229 71h10v10" />
        <path d="M195 113h-10v-10" />
        <path d="M229 113h10v-10" />
      </g>
    </g>
    <g class="lm-pop" style="--at:20">
      <path class="lm-an__go" d="M262 122 280 156H244Z" />
      <g class="lm-an__brackets">
        <path d="M248 116h-10v10" />
        <path d="M276 116h10v10" />
        <path d="M248 162h-10v-10" />
        <path d="M276 162h10v-10" />
      </g>
    </g>
  </svg>
));
