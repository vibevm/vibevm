/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The three drawings of `/why/zap`, authored as JSX for the reason the
 * VibeVM frieze is: every class on them is a hook the page's stylesheet
 * colours through a token, so the pictures follow the reader's theme
 * rather than carrying hexes of their own.
 *
 * All three are decorative. What they mean is said in the HTML around
 * them — an `aria-label` on the figure, a caption under it — so the SVG
 * itself is `aria-hidden` and out of the tab order, and a reader using a
 * screen reader gets one description instead of a list of circles.
 */

/**
 * The hero: an orbital reading of one campaign.
 *
 * The mapping is kept honest and is the source's. The core is the active
 * outcome; the dotted rings are decomposition placed outward from it —
 * the «Goal structure» reading; the drawn trajectory follows verified
 * prerequisites out of the system — the «Work order» reading; the
 * expanding ping is a question waiting for a human; the single terracotta
 * node is the registered project, the VibeVM family tie.
 */
export const Orbital = component$(() => (
  <svg
    class="wz-orbital"
    viewBox="0 0 560 520"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="wz-orbital__stars">
      <circle cx="60" cy="80" r="1.2" />
      <circle cx="160" cy="58" r="1.1" />
      <circle cx="330" cy="60" r="1.5" />
      <circle cx="430" cy="40" r="1.1" />
      <circle cx="490" cy="110" r="1.3" />
      <circle cx="505" cy="215" r="1.1" />
      <circle cx="40" cy="180" r="1.4" />
      <circle cx="70" cy="340" r="1.2" />
      <circle cx="250" cy="396" r="1.1" />
      <circle cx="380" cy="470" r="1.3" />
      <circle cx="520" cy="420" r="1.5" />
      <circle cx="284" cy="480" r="1.1" />
    </g>

    <g class="wz-orbital__orbits">
      <circle class="wz-orbital__ring" cx="210" cy="250" r="64" />
      <circle class="wz-orbital__ring" cx="210" cy="250" r="112" />
      <circle class="wz-orbital__ring" cx="210" cy="250" r="168" />
      <circle
        class="wz-orbital__ring wz-orbital__ring--far"
        cx="210"
        cy="250"
        r="232"
      />
    </g>

    {/* Work-order trajectory: core outward, following prerequisites. */}
    <path
      class="wz-orbital__path"
      d="M210 250 L265 218 L372 207 L441 270 L548 316"
      pathLength="1"
    />
    <path class="wz-orbital__arrow" d="M536 304 L551 317 L532 322" />

    {/* Question ping: a durable question waiting for its human. */}
    <circle class="wz-orbital__ping" cx="239" cy="142" r="8" />
    <circle
      class="wz-orbital__ping wz-orbital__ping--late"
      cx="239"
      cy="142"
      r="8"
    />

    <g class="wz-orbital__nodes">
      <circle class="wz-orbital__halo" cx="210" cy="250" r="27" />
      <circle class="wz-orbital__core-ring" cx="210" cy="250" r="19" />
      <circle class="wz-orbital__core" cx="210" cy="250" r="11" />

      <circle
        class="wz-orbital__node wz-orbital__node--fill"
        cx="265"
        cy="218"
        r="6"
        style="--d:.5s"
      />
      <circle
        class="wz-orbital__node"
        cx="155"
        cy="282"
        r="5"
        style="--d:.62s"
      />
      <circle
        class="wz-orbital__node wz-orbital__node--fill"
        cx="239"
        cy="142"
        r="7"
        style="--d:.7s"
      />
      <circle
        class="wz-orbital__node wz-orbital__node--project"
        cx="315"
        cy="288"
        r="6.5"
        style="--d:.78s"
      />
      <circle
        class="wz-orbital__node"
        cx="105"
        cy="212"
        r="5"
        style="--d:.86s"
      />
      <circle
        class="wz-orbital__node wz-orbital__node--fill"
        cx="372"
        cy="207"
        r="7"
        style="--d:.94s"
      />
      <circle
        class="wz-orbital__node"
        cx="126"
        cy="395"
        r="6"
        style="--d:1.02s"
      />
      <circle
        class="wz-orbital__node"
        cx="114"
        cy="112"
        r="5"
        style="--d:1.1s"
      />
      <circle
        class="wz-orbital__node wz-orbital__node--fill"
        cx="441"
        cy="270"
        r="8"
        style="--d:1.18s"
      />
      <circle
        class="wz-orbital__node"
        cx="190"
        cy="19"
        r="5"
        style="--d:1.26s"
      />
    </g>
  </svg>
));

/**
 * The «Goal structure» reading: decomposition placed outward from the one
 * source-identified active outcome. The rings are distance in
 * decomposition, not time.
 */
export const ReadingGoal = component$(() => (
  <svg
    class="wz-reading-art"
    viewBox="0 0 220 150"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <circle class="wz-reading-art__ring" cx="96" cy="75" r="26" />
    <circle class="wz-reading-art__ring" cx="96" cy="75" r="48" />
    <circle class="wz-reading-art__ring" cx="96" cy="75" r="70" />
    <circle class="wz-reading-art__core" cx="96" cy="75" r="7" />
    <circle class="wz-reading-art__dot" cx="118" cy="59" r="4" />
    <circle class="wz-reading-art__dot" cx="74" cy="92" r="4" />
    <circle
      class="wz-reading-art__dot wz-reading-art__dot--open"
      cx="132"
      cy="103"
      r="4"
    />
    <circle
      class="wz-reading-art__dot wz-reading-art__dot--open"
      cx="55"
      cy="49"
      r="4"
    />
    <circle class="wz-reading-art__dot" cx="160" cy="63" r="4" />
    <circle
      class="wz-reading-art__dot wz-reading-art__dot--open"
      cx="38"
      cy="102"
      r="4"
    />
  </svg>
));

/**
 * The «Work order» reading: left-to-right steps built only from verified
 * prerequisite-to-dependent relations. The two detached dots below are
 * one stage with no order inside it — honestly unordered.
 */
export const ReadingOrder = component$(() => (
  <svg
    class="wz-reading-art"
    viewBox="0 0 220 150"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <path
      class="wz-reading-art__chain"
      d="M18 78 L62 56 L106 84 L150 58 L196 74"
    />
    <path class="wz-reading-art__tip" d="M188 66 L198 74 L187 81" />
    <circle class="wz-reading-art__dot" cx="18" cy="78" r="5" />
    <circle class="wz-reading-art__dot" cx="62" cy="56" r="5" />
    <circle class="wz-reading-art__dot" cx="106" cy="84" r="5" />
    <circle class="wz-reading-art__dot" cx="150" cy="58" r="5" />
    <circle
      class="wz-reading-art__dot wz-reading-art__dot--open"
      cx="92"
      cy="122"
      r="4"
    />
    <circle
      class="wz-reading-art__dot wz-reading-art__dot--open"
      cx="124"
      cy="122"
      r="4"
    />
  </svg>
));
