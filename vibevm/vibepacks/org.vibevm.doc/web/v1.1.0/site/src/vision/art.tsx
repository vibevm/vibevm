/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The essay's drawings, authored as JSX for the reason every page's are:
 * each class is a hook the stylesheet colours through a token, so the
 * pictures follow the reader's theme.
 *
 * One vocabulary across all four, so the argument can be read off the
 * figures alone: a CIRCLE is human intention (terracotta), a tilted
 * SQUARE is machine intention (cobalt), GOLD is the specification layer
 * — expensive, probabilistic, sampled — and plain INK is code: cheap,
 * deterministic, exactly where it was put. Thin lines between them are
 * the traceable edges the essay is about.
 */

/**
 * The hero field: the two sources of intention over one deterministic
 * floor. The human circle and the machine square each drop edges through
 * a field of small nodes down to the code bus at the bottom; one gold
 * arc joins the two sources through the specification layer. The square
 * stands tilted — intention that arrived from outside the human upright.
 */
export const IntentField = component$(() => (
  <svg
    class="bv-field"
    viewBox="0 0 1200 360"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* drafting crosshairs, faint — the family's own air */}
    <g class="bv-field__cross" stroke-width="1">
      <path d="M120 60v12M114 66h12" />
      <path d="M520 44v12M514 50h12" />
      <path d="M1010 70v12M1004 76h12" />
      <path d="M170 260v12M164 266h12" />
      <path d="M1120 218v12M1114 224h12" />
    </g>

    {/* the deterministic floor: the code bus */}
    <line
      class="bv-field__floor"
      x1="80"
      y1="312"
      x2="1120"
      y2="312"
      pathLength="1"
    />
    <g class="bv-field__tick" stroke-width="1">
      <path d="M180 309v6" />
      <path d="M300 309v6" />
      <path d="M420 309v6" />
      <path d="M540 309v6" />
      <path d="M660 309v6" />
      <path d="M780 309v6" />
      <path d="M900 309v6" />
      <path d="M1020 309v6" />
    </g>

    {/* the gold arc: the specification joins the two intentions */}
    <path class="bv-field__arc" d="M382 104C520 30 664 32 754 84" />

    {/* human intention: the circle */}
    <g class="bv-field__source" style="--d:.1s">
      <circle class="bv-field__halo" cx="300" cy="150" r="98" />
      <circle class="bv-field__human" cx="300" cy="150" r="76" />
    </g>
    <line
      class="bv-field__edge"
      x1="300"
      y1="230"
      x2="300"
      y2="310"
      pathLength="1"
      style="--d:.35s"
    />

    {/* machine intention: the tilted square */}
    <g class="bv-field__source" style="--d:.25s">
      <rect
        class="bv-field__machine"
        x="756"
        y="56"
        width="128"
        height="128"
        transform="rotate(12 820 120)"
      />
      <path
        class="bv-field__hair"
        d="M820 22v22M820 196v22M736 120h-24M904 120h24"
      />
    </g>
    <line
      class="bv-field__edge"
      x1="822"
      y1="200"
      x2="822"
      y2="310"
      pathLength="1"
      style="--d:.5s"
    />

    {/* the field of nodes, and the edges that trace through it */}
    <g class="bv-field__edges">
      <path
        class="bv-field__edge"
        d="M366 196 480 240"
        pathLength="1"
        style="--d:.45s"
      />
      <path
        class="bv-field__edge"
        d="M480 240 560 202"
        pathLength="1"
        style="--d:.55s"
      />
      <path
        class="bv-field__edge"
        d="M560 202 640 256"
        pathLength="1"
        style="--d:.65s"
      />
      <path
        class="bv-field__edge"
        d="M640 256 640 310"
        pathLength="1"
        style="--d:.75s"
      />
      <path
        class="bv-field__edge"
        d="M770 176 700 236"
        pathLength="1"
        style="--d:.6s"
      />
      <path
        class="bv-field__edge"
        d="M700 236 700 310"
        pathLength="1"
        style="--d:.7s"
      />
      <path
        class="bv-field__edge"
        d="M876 196 962 224"
        pathLength="1"
        style="--d:.7s"
      />
      <path
        class="bv-field__edge"
        d="M962 224 1040 274"
        pathLength="1"
        style="--d:.8s"
      />
      <path
        class="bv-field__edge"
        d="M1040 274 1040 310"
        pathLength="1"
        style="--d:.9s"
      />
      <path
        class="bv-field__edge"
        d="M480 240 480 310"
        pathLength="1"
        style="--d:.85s"
      />
    </g>
    <g class="bv-field__nodes">
      <rect class="bv-field__node--m" x="474" y="234" width="12" height="12" />
      <circle class="bv-field__node--h" cx="560" cy="202" r="5" />
      <rect class="bv-field__node--m" x="634" y="250" width="12" height="12" />
      <circle class="bv-field__node--h" cx="700" cy="236" r="5" />
      <rect class="bv-field__node--m" x="956" y="218" width="12" height="12" />
      <circle class="bv-field__node--h" cx="1040" cy="274" r="5" />
    </g>
  </svg>
));

/**
 * Absolute SDD, priced: a small dark datacenter on the ground, a long
 * cord across the sky, and the plug arriving at a star. Two smaller
 * stars wait their turn — «possibly more than one».
 */
export const StarPlug = component$(() => (
  <svg
    class="bv-star"
    viewBox="0 0 900 420"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* the star, cropped by the frame the way a sun refuses a canvas */}
    <circle class="bv-star__sun" cx="764" cy="104" r="148" />
    <circle class="bv-star__corona" cx="764" cy="104" r="186" />
    <g class="bv-star__ray" stroke-width="1.6">
      <path d="M566 240 596 216" />
      <path d="M540 160h38" />
      <path d="M580 66 610 84" />
      <path d="M690 292l14-30" />
      <path d="M806 300v-34" />
    </g>

    {/* the two spare stars */}
    <circle class="bv-star__extra" cx="184" cy="78" r="12" />
    <circle class="bv-star__extra" cx="316" cy="44" r="7" />

    {/* the ground, and the datacenter standing on it */}
    <line class="bv-star__ground" x1="60" y1="366" x2="620" y2="366" />
    <g class="bv-star__dc">
      <rect class="bv-star__dc-body" x="92" y="298" width="124" height="68" />
      <rect class="bv-star__dc-win" x="106" y="314" width="18" height="10" />
      <rect class="bv-star__dc-win" x="132" y="314" width="18" height="10" />
      <rect class="bv-star__dc-win" x="158" y="314" width="18" height="10" />
      <rect class="bv-star__dc-win" x="106" y="336" width="18" height="10" />
      <rect class="bv-star__dc-win" x="132" y="336" width="18" height="10" />
    </g>

    {/* the cord, drawn from the socket outwards */}
    <path
      class="bv-star__cord"
      d="M216 330C360 322 470 300 608 216"
      pathLength="1"
    />

    {/* the plug, aimed at the photosphere */}
    <g class="bv-star__plug" transform="rotate(-31 618 210)">
      <rect class="bv-star__plug-body" x="606" y="200" width="26" height="20" />
      <path class="bv-star__plug-pin" d="M632 205h16M632 215h16" />
    </g>
  </svg>
));

/**
 * The dichotomy, as two layers: the specification above — a gold cloud
 * of samples, blurry at its own edge — and code below, a crisp grid of
 * ink blocks on the hardware. Between them, the traceable edges; one
 * block hangs with no edge at all and carries the terracotta cross the
 * essay names the most frequent case: simply an error to fix.
 */
export const TwoLayers = component$(() => (
  <svg
    class="bv-layers"
    viewBox="0 0 1000 460"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* the two intentions feed the cloud */}
    <circle class="bv-layers__src-h" cx="152" cy="26" r="9" />
    <rect
      class="bv-layers__src-m"
      x="192"
      y="14"
      width="17"
      height="17"
      transform="rotate(12 200.5 22.5)"
    />
    <path class="bv-layers__feed" d="M156 38 176 66M200 36 196 64" />

    {/* the specification cloud: a sampled, unstable outline */}
    <path
      class="bv-layers__cloud"
      d="M132 132C120 92 176 62 250 68C300 40 420 34 500 56C590 30 720 42 780 74C852 66 892 100 872 138C896 168 848 196 780 192C700 214 560 216 470 198C360 216 240 208 190 186C136 180 118 158 132 132Z"
    />
    <g class="bv-layers__samples">
      <circle cx="228" cy="120" r="3" />
      <circle cx="268" cy="96" r="2" />
      <circle cx="296" cy="142" r="4" />
      <circle cx="342" cy="108" r="2.4" />
      <circle cx="368" cy="156" r="3" />
      <circle cx="404" cy="92" r="2" />
      <circle cx="436" cy="128" r="6" />
      <circle cx="470" cy="164" r="2.4" />
      <circle cx="506" cy="100" r="3.4" />
      <circle cx="548" cy="140" r="2" />
      <circle cx="584" cy="86" r="2.6" />
      <circle cx="612" cy="150" r="4" />
      <circle cx="648" cy="112" r="2" />
      <circle cx="688" cy="160" r="3" />
      <circle cx="716" cy="96" r="5" />
      <circle cx="754" cy="136" r="2.4" />
      <circle cx="790" cy="112" r="2" />
      <circle cx="818" cy="152" r="3" />
      <circle cx="252" cy="168" r="2" />
      <circle cx="522" cy="182" r="2.6" />
      <circle cx="676" cy="70" r="2" />
      <circle cx="356" cy="66" r="2.2" />
    </g>

    {/* the traceable edges, each landing on a block through a joint */}
    <g class="bv-layers__edges">
      <path
        class="bv-layers__edge"
        d="M236 196v104"
        pathLength="1"
        style="--d:.15s"
      />
      <path
        class="bv-layers__edge"
        d="M436 210v90"
        pathLength="1"
        style="--d:.3s"
      />
      <path
        class="bv-layers__edge"
        d="M612 206v94"
        pathLength="1"
        style="--d:.45s"
      />
      <path
        class="bv-layers__edge"
        d="M788 196 748 300"
        pathLength="1"
        style="--d:.6s"
      />
      <path
        class="bv-layers__edge"
        d="M236 300v56"
        pathLength="1"
        style="--d:.75s"
      />
    </g>
    <g class="bv-layers__joints">
      <rect
        x="230"
        y="242"
        width="11"
        height="11"
        transform="rotate(45 235.5 247.5)"
      />
      <rect
        x="430"
        y="248"
        width="11"
        height="11"
        transform="rotate(45 435.5 253.5)"
      />
      <rect
        x="606"
        y="246"
        width="11"
        height="11"
        transform="rotate(45 611.5 251.5)"
      />
      <rect
        x="762"
        y="240"
        width="11"
        height="11"
        transform="rotate(45 767.5 245.5)"
      />
    </g>

    {/* the code grid: crisp, cheap, on the hardware it has today */}
    <g class="bv-layers__blocks">
      <rect x="120" y="300" width="150" height="34" />
      <rect x="284" y="300" width="96" height="34" />
      <rect x="394" y="300" width="128" height="34" />
      <rect x="536" y="300" width="88" height="34" />
      <rect x="638" y="300" width="152" height="34" />
      <rect x="120" y="356" width="96" height="34" />
      <rect x="230" y="356" width="140" height="34" />
      <rect x="384" y="356" width="84" height="34" />
      <rect x="482" y="356" width="150" height="34" />
      <rect x="646" y="356" width="96" height="34" />
      <rect
        class="bv-layers__block--loose"
        x="804"
        y="300"
        width="76"
        height="34"
      />
      <rect x="756" y="356" width="124" height="34" />
    </g>

    {/* the most frequent case: no edge at all — an error to fix */}
    <path class="bv-layers__flag" d="M830 268l24 24M854 268l-24 24" />

    {/* the hardware line under the grid */}
    <line class="bv-layers__metal" x1="120" y1="412" x2="880" y2="412" />
    <g class="bv-layers__tick" stroke-width="1">
      <path d="M200 409v6" />
      <path d="M330 409v6" />
      <path d="M460 409v6" />
      <path d="M590 409v6" />
      <path d="M720 409v6" />
      <path d="M850 409v6" />
    </g>
  </svg>
));

/**
 * Specification inheritance, drawn the way the essay says it: each level
 * carries the outline of the one above it — the fairy tale about OOP,
 * suddenly load-bearing. The bottom square is code, and the inherited
 * contours are still visible inside it.
 */
export const InheritLadder = component$(() => (
  <svg
    class="bv-ladder"
    viewBox="0 0 340 320"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* the base specification */}
    <rect class="bv-ladder__spec" x="120" y="14" width="100" height="70" />
    <g class="bv-ladder__lines">
      <rect x="134" y="30" width="52" height="5" rx="2.5" />
      <rect x="134" y="44" width="70" height="5" rx="2.5" />
      <rect x="134" y="58" width="40" height="5" rx="2.5" />
    </g>

    <line class="bv-ladder__edge" x1="170" y1="84" x2="170" y2="112" />
    <rect
      class="bv-ladder__joint"
      x="164.5"
      y="92.5"
      width="11"
      height="11"
      transform="rotate(45 170 98)"
    />

    {/* the heir: the parent's outline is still inside it */}
    <rect class="bv-ladder__heir" x="106" y="112" width="128" height="84" />
    <rect class="bv-ladder__ghost" x="140" y="126" width="60" height="42" />
    <g class="bv-ladder__lines">
      <rect x="120" y="176" width="76" height="5" rx="2.5" />
    </g>

    <line class="bv-ladder__edge" x1="170" y1="196" x2="170" y2="224" />
    <rect
      class="bv-ladder__joint"
      x="164.5"
      y="204.5"
      width="11"
      height="11"
      transform="rotate(45 170 210)"
    />

    {/* code: both ancestors visible, the rest is implementation */}
    <rect class="bv-ladder__code" x="92" y="224" width="156" height="82" />
    <rect
      class="bv-ladder__ghost--heir"
      x="112"
      y="236"
      width="76"
      height="50"
    />
    <rect class="bv-ladder__ghost" x="132" y="246" width="36" height="26" />
    <g class="bv-ladder__code-lines">
      <rect x="200" y="240" width="34" height="5" rx="2.5" />
      <rect x="200" y="254" width="26" height="5" rx="2.5" />
      <rect x="200" y="268" width="38" height="5" rx="2.5" />
      <rect x="112" y="292" width="122" height="5" rx="2.5" />
    </g>
  </svg>
));

/**
 * The pause between sections: the essay's three masses in one small
 * line — human circle, machine square, gold diamond of the
 * specification between them.
 */
export const SectionMark = component$(() => (
  <svg
    class="bv-mark"
    viewBox="0 0 76 16"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <circle class="bv-mark__human" cx="8" cy="8" r="6" />
    <rect
      class="bv-mark__spec"
      x="33"
      y="3"
      width="10"
      height="10"
      transform="rotate(45 38 8)"
    />
    <rect class="bv-mark__machine" x="62" y="2" width="12" height="12" />
  </svg>
));
