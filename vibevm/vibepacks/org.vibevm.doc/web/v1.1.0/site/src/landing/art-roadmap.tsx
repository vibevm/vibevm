/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The map's drawing for the roadmap — the ninth destination, in a file of
 * its own because the second row's four already fill `art-story.tsx` to
 * the discipline's budget. It is the roadmap page's own hero drawn small,
 * in the page's own vocabulary (`roadmap/art.tsx`): the terracotta circle
 * at the foot is where the project stands; the ink bar rises to the right
 * through six hollow nodes — the path of squares; over each node a
 * station in the stage's own shapes — the opened core, Zap's orbit, a
 * grid of packages, two plates in frames, shelves of covers, and the
 * whole system at the top under a faint gold horizon. Every element that
 * draws or pops in carries `--at`, its place in the map's choreography
 * (`map.css`).
 */
export const RoadmapArt = component$(() => (
  <svg
    class="lm-road"
    viewBox="0 0 560 200"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-cross" stroke-width="1">
      <path d="M60 30v12M54 36h12" />
      <path d="M300 168v12M294 174h12" />
      <path d="M534 130v12M528 136h12" />
    </g>
    <circle class="lm-road__horizon" cx="486" cy="40" r="54" />
    <line
      class="lm-road__floor lm-draw"
      x1="28"
      y1="186"
      x2="532"
      y2="186"
      pathLength="1"
      style="--at:0"
    />
    <g class="lm-tick" stroke-width="1">
      <path d="M120 183v6" />
      <path d="M220 183v6" />
      <path d="M320 183v6" />
      <path d="M420 183v6" />
    </g>
    <g class="lm-road__drops">
      <path d="M153 154v30" />
      <path d="M217 138v46" />
      <path d="M282 123v61" />
      <path d="M346 107v77" />
      <path d="M411 92v92" />
      <path d="M476 77v107" />
    </g>
    <line
      class="lm-road__bar lm-draw"
      x1="96"
      y1="160"
      x2="500"
      y2="64"
      pathLength="1"
      style="--at:2"
    />
    <g class="lm-pop" style="--at:1">
      <circle class="lm-road__halo" cx="72" cy="160" r="24" />
      <circle class="lm-road__here" cx="72" cy="160" r="17" />
    </g>
    <rect
      class="lm-road__node lm-pop"
      x="148"
      y="142"
      width="10"
      height="10"
      style="--at:6"
    />
    <rect
      class="lm-road__node lm-pop"
      x="212"
      y="126"
      width="10"
      height="10"
      style="--at:9"
    />
    <rect
      class="lm-road__node lm-pop"
      x="277"
      y="111"
      width="10"
      height="10"
      style="--at:12"
    />
    <rect
      class="lm-road__node lm-pop"
      x="341"
      y="95"
      width="10"
      height="10"
      style="--at:15"
    />
    <rect
      class="lm-road__node lm-pop"
      x="406"
      y="80"
      width="10"
      height="10"
      style="--at:18"
    />
    <rect
      class="lm-road__node lm-pop"
      x="471"
      y="65"
      width="10"
      height="10"
      style="--at:21"
    />
    {/* the core opens */}
    <g class="lm-pop" style="--at:7">
      <rect class="lm-road__ink" x="145" y="119" width="16" height="16" />
      <rect class="lm-road__terra" x="157" y="124" width="6" height="6" />
    </g>
    {/* Zap's orbit */}
    <g class="lm-pop" style="--at:10">
      <circle class="lm-road__ring" cx="217" cy="110" r="9" />
      <path class="lm-road__orbit" d="M217 110 228 104" />
      <circle class="lm-road__zap" cx="217" cy="110" r="2.5" />
      <circle class="lm-road__terra" cx="223" cy="114" r="1.6" />
    </g>
    {/* the registries fill */}
    <g class="lm-pop" style="--at:13">
      <rect class="lm-road__terra" x="271" y="90" width="6" height="6" />
      <rect class="lm-road__ink" x="279" y="90" width="6" height="6" />
      <rect class="lm-road__gold" x="287" y="90" width="6" height="6" />
      <rect class="lm-road__ink" x="271" y="98" width="6" height="6" />
      <rect class="lm-road__hollow" x="279" y="98" width="6" height="6" />
      <rect class="lm-road__ink" x="287" y="98" width="6" height="6" />
    </g>
    {/* two plates dock */}
    <g class="lm-pop" style="--at:16">
      <rect class="lm-road__hollow" x="329" y="76" width="14" height="12" />
      <rect class="lm-road__hollow" x="349" y="76" width="14" height="12" />
      <rect class="lm-road__cobalt" x="332" y="79" width="8" height="6" />
      <rect class="lm-road__ink" x="352" y="79" width="8" height="6" />
    </g>
    {/* shelves of covers */}
    <g class="lm-pop" style="--at:19">
      <path class="lm-road__shelf" d="M386 60H436M386 73H436" />
      <rect class="lm-road__ink" x="389" y="64" width="4" height="9" />
      <rect class="lm-road__gold" x="395" y="62" width="3" height="11" />
      <rect class="lm-road__ink" x="400" y="65" width="5" height="8" />
      <rect class="lm-road__cobalt" x="407" y="63" width="4" height="10" />
      <rect class="lm-road__ink" x="413" y="64" width="5" height="9" />
      <rect class="lm-road__ink" x="420" y="62" width="3" height="11" />
      <rect class="lm-road__terra" x="425" y="65" width="5" height="8" />
      <rect class="lm-road__ink" x="389" y="51" width="5" height="9" />
      <rect class="lm-road__ink" x="396" y="50" width="4" height="10" />
      <rect class="lm-road__gold" x="402" y="52" width="5" height="8" />
      <rect class="lm-road__ink" x="409" y="49" width="3" height="11" />
      <rect class="lm-road__cobalt" x="414" y="51" width="5" height="9" />
      <rect class="lm-road__ink" x="421" y="50" width="4" height="10" />
      <rect class="lm-road__ink" x="427" y="52" width="4" height="8" />
    </g>
    {/* the whole system */}
    <g class="lm-pop" style="--at:22">
      <rect class="lm-road__plane" x="445" y="12" width="62" height="46" />
      <rect class="lm-road__ink" x="451" y="18" width="14" height="14" />
      <rect class="lm-road__terra" x="462" y="23" width="5" height="5" />
      <rect class="lm-road__ink" x="472" y="18" width="4" height="4" />
      <rect class="lm-road__ink" x="478" y="18" width="4" height="4" />
      <rect class="lm-road__hollow" x="484" y="18" width="4" height="4" />
      <rect class="lm-road__ink" x="472" y="24" width="4" height="4" />
      <rect class="lm-road__gold" x="478" y="24" width="4" height="4" />
      <rect class="lm-road__ink" x="484" y="24" width="4" height="4" />
      <rect class="lm-road__hollow" x="451" y="40" width="16" height="12" />
      <rect class="lm-road__cobalt" x="454" y="43" width="10" height="6" />
      <path class="lm-road__shelf" d="M472 52H500" />
      <rect class="lm-road__ink" x="474" y="45" width="3" height="7" />
      <rect class="lm-road__gold" x="479" y="43" width="4" height="9" />
      <rect class="lm-road__ink" x="485" y="46" width="3" height="6" />
      <rect class="lm-road__terra" x="490" y="44" width="4" height="8" />
      <rect class="lm-road__ink" x="496" y="46" width="3" height="6" />
      <rect
        class="lm-road__gold"
        x="473"
        y="32"
        width="6"
        height="6"
        transform="rotate(45 476 35)"
      />
    </g>
  </svg>
));
