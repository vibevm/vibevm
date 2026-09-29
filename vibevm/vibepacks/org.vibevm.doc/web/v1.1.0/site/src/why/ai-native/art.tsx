/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The two drawings of `/why/ai-native`, authored as JSX for the reason
 * the other pages' are: every class is a hook the stylesheet colours
 * through a token, so the picture follows the reader's theme.
 */

/**
 * The hero blueprint: one neutral core projects three rays onto three
 * language stations — Rust a solid oxide square, TypeScript an outlined
 * slate circle, Go a translucent teal triangle — each framed by gold
 * envelope brackets. Everything stands on one thin verification floor
 * that ends in a single gate notch: one exit code.
 */
export const Projection = component$(() => (
  <svg
    class="an-projection"
    viewBox="0 0 1200 340"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* drafting crosshairs, faint */}
    <g class="an-projection__cross" stroke-width="1">
      <path d="M420 56v12M414 62h12" />
      <path d="M706 66v12M700 72h12" />
      <path d="M906 52v12M900 58h12" />
      <path d="M348 246v12M342 252h12" />
      <path d="M742 264v12M736 270h12" />
      <path d="M1096 116v12M1090 122h12" />
      <path d="M96 262v12M90 268h12" />
    </g>

    {/* the verification floor */}
    <line
      class="an-projection__floor"
      x1="70"
      y1="302"
      x2="1104"
      y2="302"
      pathLength="1"
    />
    <g class="an-projection__tick" stroke-width="1">
      <path d="M170 299v6" />
      <path d="M290 299v6" />
      <path d="M410 299v6" />
      <path d="M530 299v6" />
      <path d="M650 299v6" />
      <path d="M770 299v6" />
      <path d="M890 299v6" />
      <path d="M1010 299v6" />
    </g>
    <rect class="an-projection__exit" x="1108" y="294" width="16" height="16" />

    {/* the neutral core */}
    <g class="an-projection__core">
      <rect
        class="an-projection__halo"
        x="120"
        y="75"
        width="150"
        height="150"
      />
      <rect
        class="an-projection__core-body"
        x="138"
        y="93"
        width="114"
        height="114"
      />
      <path
        class="an-projection__hair"
        d="M195 60v30M195 210v30M105 150h24M262 150h24"
      />
    </g>
    <line class="an-projection__drop" x1="195" y1="242" x2="195" y2="298" />

    {/* projection rays */}
    <g class="an-projection__rays">
      <line
        class="an-projection__ray"
        x1="268"
        y1="142"
        x2="532"
        y2="126"
        pathLength="1"
        style="--d:.25s"
      />
      <line
        class="an-projection__ray"
        x1="268"
        y1="152"
        x2="756"
        y2="176"
        pathLength="1"
        style="--d:.45s"
      />
      <line
        class="an-projection__ray"
        x1="268"
        y1="162"
        x2="960"
        y2="228"
        pathLength="1"
        style="--d:.65s"
      />
    </g>

    {/* station: Rust (solid oxide square) */}
    <g class="an-projection__station" style="--d:.55s">
      <rect
        class="an-projection__mark an-projection__mark--rust"
        x="551"
        y="94"
        width="68"
        height="68"
      />
      <g class="an-projection__dots">
        <circle class="an-projection__dot--r" cx="569" cy="112" r="2.3" />
        <circle class="an-projection__dot--r" cx="585" cy="112" r="2.3" />
        <circle class="an-projection__dot--r" cx="601" cy="112" r="2.3" />
        <circle class="an-projection__dot--r" cx="569" cy="128" r="2.3" />
        <circle class="an-projection__dot--r" cx="585" cy="128" r="2.3" />
        <circle class="an-projection__dot--r" cx="601" cy="128" r="2.3" />
        <circle class="an-projection__dot--r" cx="569" cy="144" r="2.3" />
        <circle class="an-projection__dot--r" cx="585" cy="144" r="2.3" />
        <circle class="an-projection__dot--r" cx="601" cy="144" r="2.3" />
      </g>
      <g class="an-projection__brackets">
        <path class="an-projection__bracket" d="M555 82H539V98" />
        <path class="an-projection__bracket" d="M615 82h16v16" />
        <path class="an-projection__bracket" d="M555 174H539v-16" />
        <path class="an-projection__bracket" d="M615 174h16v-16" />
      </g>
    </g>
    <line class="an-projection__drop" x1="585" y1="182" x2="585" y2="298" />

    {/* station: TypeScript (outlined slate circle) */}
    <g class="an-projection__station" style="--d:.75s">
      <circle
        class="an-projection__mark an-projection__mark--ts"
        cx="810"
        cy="178"
        r="35"
      />
      <g class="an-projection__dots">
        <circle class="an-projection__dot--t" cx="794" cy="162" r="2.3" />
        <circle class="an-projection__dot--t" cx="810" cy="162" r="2.3" />
        <circle class="an-projection__dot--t" cx="826" cy="162" r="2.3" />
        <circle class="an-projection__dot--t" cx="794" cy="178" r="2.3" />
        <circle class="an-projection__dot--t" cx="810" cy="178" r="2.3" />
        <circle class="an-projection__dot--t" cx="826" cy="178" r="2.3" />
        <circle class="an-projection__dot--t" cx="794" cy="194" r="2.3" />
        <circle class="an-projection__dot--t" cx="810" cy="194" r="2.3" />
        <circle class="an-projection__dot--t" cx="826" cy="194" r="2.3" />
      </g>
      <g class="an-projection__brackets">
        <path class="an-projection__bracket" d="M779 131H763v16" />
        <path class="an-projection__bracket" d="M841 131h16v16" />
        <path class="an-projection__bracket" d="M779 225H763v-16" />
        <path class="an-projection__bracket" d="M841 225h16v-16" />
      </g>
    </g>
    <line class="an-projection__drop" x1="810" y1="233" x2="810" y2="298" />

    {/* station: Go (translucent teal triangle) */}
    <g class="an-projection__station" style="--d:.95s">
      <path
        class="an-projection__mark an-projection__mark--go"
        d="M1015 196 1051 258H979Z"
      />
      <g class="an-projection__dots">
        <circle class="an-projection__dot--g" cx="1015" cy="228" r="2.3" />
        <circle class="an-projection__dot--g" cx="1003" cy="246" r="2.3" />
        <circle class="an-projection__dot--g" cx="1027" cy="246" r="2.3" />
      </g>
      <g class="an-projection__brackets">
        <path class="an-projection__bracket" d="M983 184H967v16" />
        <path class="an-projection__bracket" d="M1047 184h16v16" />
        <path class="an-projection__bracket" d="M983 270H967v-16" />
        <path class="an-projection__bracket" d="M1047 270h16v-16" />
      </g>
    </g>
    <line class="an-projection__drop" x1="1015" y1="274" x2="1015" y2="298" />
  </svg>
));

/**
 * The central law, drawn: a source file whose interior is ordinary
 * in-distribution code — plain ink bars — wrapped by the discipline's
 * envelope: an ochre dashed ring with four carrier nodes (types,
 * contracts, metadata, the verification loop) and clamp brackets.
 *
 * It is rendered on the cream band, where the ground is the same in
 * either theme, so its tones come from the band's tokens rather than
 * from the page's.
 */
export const Envelope = component$(() => (
  <svg
    class="an-envelope"
    viewBox="0 0 340 260"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    {/* envelope ring + carrier nodes */}
    <rect
      class="an-envelope__ring"
      x="70"
      y="10"
      width="200"
      height="212"
      rx="18"
    />
    <g class="an-envelope__node">
      <rect x="165" y="5" width="10" height="10" />
      <rect x="265" y="111" width="10" height="10" />
      <rect x="165" y="217" width="10" height="10" />
      <rect x="65" y="111" width="10" height="10" />
    </g>

    {/* clamp brackets */}
    <g class="an-envelope__brackets">
      <path class="an-envelope__bracket" d="M114 26H98v16" />
      <path class="an-envelope__bracket" d="M226 26h16v16" />
      <path class="an-envelope__bracket" d="M114 206H98v-16" />
      <path class="an-envelope__bracket" d="M226 206h16v-16" />
    </g>

    {/* the file: ordinary code inside */}
    <rect
      class="an-envelope__file"
      x="110"
      y="38"
      width="120"
      height="156"
      rx="6"
    />
    <g class="an-envelope__code">
      <rect x="126" y="56" width="70" height="6" rx="3" />
      <rect x="126" y="72" width="88" height="6" rx="3" />
      <rect x="138" y="88" width="62" height="6" rx="3" />
      <rect x="138" y="104" width="76" height="6" rx="3" />
      <rect x="126" y="120" width="52" height="6" rx="3" />
      <rect x="138" y="136" width="70" height="6" rx="3" />
      <rect x="138" y="152" width="44" height="6" rx="3" />
      <rect x="126" y="168" width="80" height="6" rx="3" />
    </g>
  </svg>
));
