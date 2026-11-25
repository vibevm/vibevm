/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The roadmap's drawings, authored as JSX for the reason every page's are:
 * each class is a hook the stylesheet colours through a token, so the
 * pictures follow the reader's theme.
 *
 * One vocabulary across the page, and it is the family's: a terracotta
 * CIRCLE is the project as it stands — the human mark, «you are here»;
 * plain INK is the deterministic thing built — a core, a plate, a plane;
 * GOLD is the specification layer — a package, the hub that joins the
 * parts; a COBALT plate is the machine's tooling — an editor, a plugin;
 * Zap's GREEN is Zap's, as it is everywhere on the site. Thin lines are
 * edges a reader can follow, and one long thick ink bar is the road. Six
 * stations stand on it, and every one of them is built of the shapes of
 * the one before: a square that opens, an orbital map, a grid of squares,
 * plates in frames, shelves of covers, and at the top a whole system
 * assembled out of all of them (`emblems.tsx` draws each of them large).
 *
 * Every element that draws or pops in carries `--at`, its place in the
 * choreography, as the map's drawings do: where the reveal is driven by
 * scrolling it is a percentage of the plate's own entry into the window,
 * and where it is not, a delay of that many times fifty milliseconds
 * (`styles.css`). A rotated shape pops inside a group, never on its own:
 * the pop is a transform and would otherwise replace the rotation.
 */

/**
 * The hero: the road as one long ink diagonal rising to the right, from
 * the terracotta circle at its foot to the whole system at its top. Six
 * hollow square nodes sit on the bar — the path of squares — and each
 * station stands over its node on a thin edge, while a faint dashed drop
 * ties every node down to the one floor. A faint gold disc stands behind
 * the last station: the horizon the road is drawn toward. It stands
 * above the fold and animates once, at load.
 */
export const RoadAhead = component$(() => (
  <svg
    class="rm-road"
    viewBox="0 0 1200 440"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="rm-road__cross" stroke-width="1">
      <path d="M90 70v12M84 76h12" />
      <path d="M420 40v12M414 46h12" />
      <path d="M760 320v12M754 326h12" />
      <path d="M1130 250v12M1124 256h12" />
      <path d="M300 380v12M294 386h12" />
    </g>

    <circle class="rm-road__horizon" cx="1000" cy="80" r="112" />

    <line
      class="rm-road__floor rm-draw"
      x1="70"
      y1="412"
      x2="1130"
      y2="412"
      pathLength="1"
      style="--at:0"
    />
    <g class="rm-s-tick" stroke-width="1">
      <path d="M200 409v6" />
      <path d="M330 409v6" />
      <path d="M460 409v6" />
      <path d="M590 409v6" />
      <path d="M720 409v6" />
      <path d="M850 409v6" />
      <path d="M980 409v6" />
      <path d="M1110 409v6" />
    </g>

    <g class="rm-road__drops">
      <path d="M292 343v67" />
      <path d="M429 306v104" />
      <path d="M567 268v142" />
      <path d="M705 230v180" />
      <path d="M842 192v218" />
      <path d="M988 152v258" />
    </g>

    <line
      class="rm-road__bar rm-draw"
      x1="180"
      y1="364"
      x2="1040"
      y2="128"
      pathLength="1"
      style="--at:3"
    />

    <g class="rm-pop" style="--at:1">
      <circle class="rm-road__halo" cx="140" cy="366" r="52" />
      <circle class="rm-s-terra" cx="140" cy="366" r="38" />
    </g>

    <g class="rm-road__edges">
      <path class="rm-draw" d="M292 325v-14" pathLength="1" style="--at:10" />
      <path class="rm-draw" d="M429 288v-14" pathLength="1" style="--at:14" />
      <path class="rm-draw" d="M567 250v-14" pathLength="1" style="--at:17" />
      <path class="rm-draw" d="M705 212v-14" pathLength="1" style="--at:21" />
      <path class="rm-draw" d="M842 174v-14" pathLength="1" style="--at:24" />
      <path class="rm-draw" d="M988 134v-14" pathLength="1" style="--at:28" />
    </g>

    <rect
      class="rm-road__node rm-pop"
      x="284"
      y="325"
      width="16"
      height="16"
      style="--at:9"
    />
    <rect
      class="rm-road__node rm-pop"
      x="421"
      y="288"
      width="16"
      height="16"
      style="--at:13"
    />
    <rect
      class="rm-road__node rm-pop"
      x="559"
      y="250"
      width="16"
      height="16"
      style="--at:16"
    />
    <rect
      class="rm-road__node rm-pop"
      x="697"
      y="212"
      width="16"
      height="16"
      style="--at:20"
    />
    <rect
      class="rm-road__node rm-pop"
      x="834"
      y="174"
      width="16"
      height="16"
      style="--at:23"
    />
    <rect
      class="rm-road__node rm-pop"
      x="980"
      y="134"
      width="16"
      height="16"
      style="--at:27"
    />

    {/* station 1 — the core opens */}
    <g class="rm-pop" style="--at:11">
      <path class="rm-s-ink" d="M272 271H312V283H300V299H312V311H272Z" />
      <rect class="rm-s-terra" x="304" y="286" width="10" height="10" />
    </g>

    {/* station 2 — Zap: the orbital map, small */}
    <g class="rm-pop" style="--at:15">
      <circle class="rm-s-ring" cx="429" cy="248" r="26" />
      <circle class="rm-s-ring" cx="429" cy="248" r="14" />
      <path class="rm-s-orbit" d="M429 248 447 238 472 231" />
      <circle class="rm-s-zap" cx="429" cy="248" r="4.5" />
      <circle class="rm-s-zap" cx="447" cy="238" r="2.6" />
      <circle class="rm-s-zap-hollow" cx="417" cy="259" r="2.6" />
      <circle class="rm-s-terra" cx="450" cy="257" r="2.6" />
    </g>

    {/* station 3 — the registries fill */}
    <g class="rm-pop" style="--at:18">
      <rect class="rm-s-terra" x="537" y="192" width="12" height="12" />
      <rect class="rm-s-ink" x="553" y="192" width="12" height="12" />
      <rect class="rm-s-gold" x="569" y="192" width="12" height="12" />
      <rect class="rm-s-hollow" x="585" y="192" width="12" height="12" />
      <rect class="rm-s-hollow" x="537" y="208" width="12" height="12" />
      <rect class="rm-s-ink" x="553" y="208" width="12" height="12" />
      <rect class="rm-s-ink" x="569" y="208" width="12" height="12" />
      <rect class="rm-s-ink" x="585" y="208" width="12" height="12" />
      <rect class="rm-s-ink" x="537" y="224" width="12" height="12" />
      <rect class="rm-s-gold" x="553" y="224" width="12" height="12" />
      <rect class="rm-s-ink" x="569" y="224" width="12" height="12" />
      <rect class="rm-s-ink" x="585" y="224" width="12" height="12" />
      <rect class="rm-s-ink" x="607" y="176" width="12" height="12" />
      <path class="rm-s-dash" d="M605 186 595 194" />
    </g>

    {/* station 4 — two plates dock into two frames */}
    <g class="rm-pop" style="--at:22">
      <g class="rm-s-bracket">
        <path d="M666 178V170H674M692 170H700V178M666 190V198H674M692 198H700V190" />
        <path d="M710 178V170H718M736 170H744V178M710 190V198H718M736 198H744V190" />
      </g>
      <rect class="rm-s-cobalt" x="672" y="176" width="22" height="16" />
      <rect class="rm-s-ink" x="716" y="176" width="22" height="16" />
    </g>

    {/* station 5 — shelves of covers */}
    <g class="rm-pop" style="--at:25">
      <path class="rm-s-shelf" d="M796 134H888M796 160H888" />
      <rect class="rm-s-ink" x="800" y="140" width="10" height="20" />
      <rect class="rm-s-gold" x="812" y="136" width="8" height="24" />
      <rect class="rm-s-ink" x="822" y="142" width="12" height="18" />
      <rect class="rm-s-cobalt" x="836" y="138" width="9" height="22" />
      <rect class="rm-s-ink" x="847" y="140" width="11" height="20" />
      <rect class="rm-s-ink" x="860" y="137" width="8" height="23" />
      <rect class="rm-s-terra" x="870" y="143" width="12" height="17" />
      <rect class="rm-s-ink" x="800" y="114" width="11" height="20" />
      <rect class="rm-s-ink" x="813" y="112" width="9" height="22" />
      <rect class="rm-s-gold" x="824" y="117" width="12" height="17" />
      <rect class="rm-s-ink" x="838" y="110" width="8" height="24" />
      <rect class="rm-s-cobalt" x="848" y="115" width="11" height="19" />
      <rect class="rm-s-ink" x="861" y="113" width="10" height="21" />
      <rect class="rm-s-ink" x="873" y="116" width="10" height="18" />
    </g>

    {/* station 6 — the whole system, built of the same shapes */}
    <g class="rm-pop" style="--at:29">
      <rect class="rm-s-plane" x="913" y="8" width="150" height="112" />
      <path class="rm-s-ink" d="M927 23H965V35H957V47H965V61H927Z" />
      <rect class="rm-s-terra" x="960" y="37" width="8" height="8" />
      <rect class="rm-s-ink" x="981" y="23" width="8" height="8" />
      <rect class="rm-s-ink" x="992" y="23" width="8" height="8" />
      <rect class="rm-s-hollow" x="1003" y="23" width="8" height="8" />
      <rect class="rm-s-ink" x="981" y="34" width="8" height="8" />
      <rect class="rm-s-gold" x="992" y="34" width="8" height="8" />
      <rect class="rm-s-ink" x="1003" y="34" width="8" height="8" />
      <g class="rm-s-bracket">
        <path d="M927 81V75H933M961 75H967V81M927 97V103H933M961 103H967V97" />
      </g>
      <rect class="rm-s-cobalt" x="935" y="82" width="24" height="14" />
      <path class="rm-s-shelf" d="M979 103H1049" />
      <rect class="rm-s-ink" x="982" y="89" width="6" height="14" />
      <rect class="rm-s-gold" x="990" y="86" width="8" height="17" />
      <rect class="rm-s-ink" x="1000" y="91" width="5" height="12" />
      <rect class="rm-s-ink" x="1007" y="88" width="7" height="15" />
      <rect class="rm-s-terra" x="1016" y="90" width="8" height="13" />
      <rect class="rm-s-ink" x="1026" y="87" width="6" height="16" />
      <rect class="rm-s-ink" x="1034" y="92" width="6" height="11" />
      <g class="rm-s-hub">
        <path d="M988 64 965 49M988 64 996 45M988 64 967 81M988 64 1007 85" />
      </g>
      <rect
        class="rm-s-gold"
        x="982"
        y="58"
        width="12"
        height="12"
        transform="rotate(45 988 64)"
      />
    </g>
  </svg>
));

/**
 * The marker: where the project stands today. The terracotta circle with
 * its halo — the same circle that stands at the foot of the road — and
 * beside it one small ink square joined by a single edge: the core it
 * stands on, and nothing built on that core yet.
 */
export const HereMark = component$(() => (
  <svg
    class="rm-em rm-em--here"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="rm-em__cross" stroke-width="1">
      <path d="M40 34v12M34 40h12" />
      <path d="M286 178v12M280 184h12" />
    </g>
    <line class="rm-s-floor" x1="24" y1="196" x2="296" y2="196" />
    <g class="rm-s-tick" stroke-width="1">
      <path d="M120 193v6" />
      <path d="M232 193v6" />
    </g>
    <path class="rm-s-hair" d="M120 18v14M120 172v14M32 106h14M194 106h14" />
    <path
      class="rm-s-edge rm-draw"
      d="M178 106 210 106"
      pathLength="1"
      style="--at:7"
    />
    <path
      class="rm-s-edge rm-draw"
      d="M232 130v66"
      pathLength="1"
      style="--at:10"
    />
    <g class="rm-pop" style="--at:1">
      <circle class="rm-road__halo" cx="120" cy="106" r="74" />
      <circle class="rm-s-terra" cx="120" cy="106" r="56" />
    </g>
    <rect
      class="rm-s-ink rm-pop"
      x="210"
      y="84"
      width="44"
      height="44"
      style="--at:6"
    />
  </svg>
));

/**
 * The route in one line, for the inverted band: the circle at the foot,
 * a bar rising to the right, six squares on it growing toward the last,
 * which is gold. It is drawn on the cream band, where the ground is the
 * same in either theme, so its tones are the band's and not the page's.
 */
export const RouteGlyph = component$(() => (
  <svg
    class="rm-glyph"
    viewBox="0 0 360 72"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <line class="rm-glyph__bar" x1="32" y1="52" x2="332" y2="16" />
    <circle class="rm-glyph__here" cx="20" cy="54" r="12" />
    <rect class="rm-glyph__step" x="70" y="44" width="7" height="7" />
    <rect class="rm-glyph__step" x="118" y="37" width="8" height="8" />
    <rect class="rm-glyph__step" x="165" y="30" width="10" height="10" />
    <rect class="rm-glyph__step" x="212" y="24" width="12" height="12" />
    <rect class="rm-glyph__step" x="259" y="17" width="14" height="14" />
    <rect
      class="rm-glyph__step rm-glyph__step--last"
      x="305"
      y="9"
      width="18"
      height="18"
    />
  </svg>
));

/**
 * The pause between sections: the family's three masses in one small
 * line — human circle, gold diamond of the specification, machine
 * square — as the essay and the map pause with them.
 */
export const RoadmapMark = component$(() => (
  <svg
    class="rm-mark"
    viewBox="0 0 76 16"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <circle class="rm-mark__human" cx="8" cy="8" r="6" />
    <rect
      class="rm-mark__spec"
      x="33"
      y="3"
      width="10"
      height="10"
      transform="rotate(45 38 8)"
    />
    <rect class="rm-mark__machine" x="62" y="2" width="12" height="12" />
  </svg>
));
