/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

/**
 * The map's drawings for the header's first row — the software, where it
 * is kept and where it is spoken about — authored as JSX for the reason
 * every page's are: each class is a hook the stylesheet colours through a
 * token, so the pictures follow the reader's theme.
 *
 * The vocabulary is the family's and not a new one. Plain INK is the
 * deterministic thing — a page, a commit, the square that stands for the
 * source; the site's TERRACOTTA marks what is the project's own — a block
 * number, a branch, the voice that broadcasts; the GROUND shows through
 * where a page has lines on it. Thin lines are edges a reader can follow.
 *
 * Every element that draws or pops in carries `--at`, its place in the
 * choreography: where the reveal is scroll-driven it is a percentage of
 * the plate's own entry into the window, and where it is not, a delay of
 * that many times fifty milliseconds (`map.css`). A rotated shape pops
 * inside a group, never on its own: the pop is a CSS transform and would
 * otherwise replace the rotation it was drawn with.
 */

/**
 * Documentation: the manual as a page. A tall dark plane with pale lines
 * of text, a second page behind it, the project's terracotta block number
 * over the corner — and one long thin diagonal behind all of it, the
 * line a reader follows through a manual.
 */
export const DocumentationArt = component$(() => (
  <svg
    class="lm-doc"
    viewBox="0 0 400 360"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-cross" stroke-width="1">
      <path d="M44 60v12M38 66h12" />
      <path d="M352 44v12M346 50h12" />
      <path d="M64 312v12M58 318h12" />
    </g>
    <path
      class="lm-doc__line lm-draw"
      d="M36 330 364 50"
      pathLength="1"
      style="--at:0"
    />
    <rect
      class="lm-doc__ghost lm-pop"
      x="158"
      y="34"
      width="164"
      height="248"
      style="--at:4"
    />
    <rect
      class="lm-doc__page lm-pop"
      x="118"
      y="52"
      width="164"
      height="248"
      style="--at:8"
    />
    <g class="lm-doc__text lm-pop" style="--at:14">
      <rect x="142" y="92" width="84" height="6" rx="3" />
      <rect x="142" y="116" width="112" height="6" rx="3" />
      <rect x="142" y="140" width="64" height="6" rx="3" />
      <rect x="142" y="176" width="100" height="6" rx="3" />
      <rect x="142" y="200" width="72" height="6" rx="3" />
      <rect x="142" y="224" width="108" height="6" rx="3" />
      <rect x="142" y="260" width="56" height="6" rx="3" />
    </g>
    <rect
      class="lm-doc__number lm-pop"
      x="98"
      y="70"
      width="34"
      height="34"
      style="--at:20"
    />
  </svg>
));

/**
 * A repository, as the two mirrors share it: a trunk of pinned commits,
 * one terracotta branch that leaves and merges back, and a square that
 * stands for the source. The canonical one is solid; the mirror flips
 * the picture and hollows the square — a copy, and the same copy.
 */
const RepoGraph = component$<{ mirror: boolean }>((props) => (
  <svg
    class="lm-repo"
    viewBox="0 0 240 180"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g transform={props.mirror ? "matrix(-1 0 0 1 240 0)" : undefined}>
      <g class="lm-cross" stroke-width="1">
        <path d="M30 34v12M24 40h12" />
        <path d="M206 148v12M200 154h12" />
      </g>
      <line class="lm-repo__floor" x1="22" y1="166" x2="218" y2="166" />
      <line
        class="lm-repo__trunk lm-draw"
        x1="80"
        y1="16"
        x2="80"
        y2="166"
        pathLength="1"
        style="--at:0"
      />
      <path
        class="lm-repo__branch lm-draw"
        d="M80 52C80 78 138 62 138 88V104C138 130 80 114 80 140"
        pathLength="1"
        style="--at:6"
      />
      <circle
        class="lm-repo__pin lm-pop"
        cx="80"
        cy="40"
        r="7"
        style="--at:8"
      />
      <circle
        class="lm-repo__pin lm-pop"
        cx="80"
        cy="96"
        r="7"
        style="--at:12"
      />
      <circle
        class="lm-repo__pin lm-pop"
        cx="80"
        cy="150"
        r="7"
        style="--at:16"
      />
      <circle
        class="lm-repo__head lm-pop"
        cx="138"
        cy="96"
        r="7"
        style="--at:14"
      />
      <rect
        class={`lm-repo__source lm-pop${props.mirror ? " lm-repo__source--hollow" : ""}`}
        x="164"
        y="34"
        width="46"
        height="46"
        style="--at:10"
      />
    </g>
  </svg>
));

/** GitHub: the canonical repository — the branch leaves to the right. */
export const GithubArt = component$(() => <RepoGraph mirror={false} />);

/** GitVerse: the same repository in the mirror — the branch leaves to the left. */
export const GitverseArt = component$(() => <RepoGraph mirror={true} />);

/**
 * News & support: a broadcast. The project's terracotta circle sends
 * three dashed arcs up and to the right; two small planes at the far end
 * receive them — a solid one and a hollow one, a message and a reply —
 * and one thin line runs back to the circle, which is what support is.
 */
export const NewsArt = component$(() => (
  <svg
    class="lm-news"
    viewBox="0 0 240 280"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="lm-cross" stroke-width="1">
      <path d="M36 44v12M30 50h12" />
      <path d="M204 26v12M198 32h12" />
    </g>
    <line class="lm-news__floor" x1="18" y1="254" x2="226" y2="254" />
    <g class="lm-news__tick" stroke-width="1">
      <path d="M88 251v6" />
      <path d="M164 251v6" />
    </g>
    <path
      class="lm-news__arc lm-draw"
      d="M71.6 159.2A56 56 0 0 1 115.7 208.1"
      pathLength="1"
      style="--at:6"
    />
    <path
      class="lm-news__arc lm-draw"
      d="M79.1 124A92 92 0 0 1 151.5 204.4"
      pathLength="1"
      style="--at:10"
    />
    <path
      class="lm-news__arc lm-draw"
      d="M86.6 88.8A128 128 0 0 1 187.3 200.6"
      pathLength="1"
      style="--at:14"
    />
    <path
      class="lm-news__reply lm-draw"
      d="M172 106 83 192"
      pathLength="1"
      style="--at:22"
    />
    <g class="lm-news__source lm-pop" style="--at:0">
      <circle class="lm-news__halo" cx="60" cy="214" r="34" />
      <circle class="lm-news__voice" cx="60" cy="214" r="26" />
    </g>
    <rect
      class="lm-news__plane lm-pop"
      x="172"
      y="78"
      width="44"
      height="28"
      style="--at:18"
    />
    <g class="lm-pop" style="--at:20">
      <rect
        class="lm-news__plane--hollow"
        x="190"
        y="136"
        width="22"
        height="22"
        transform="rotate(16 201 147)"
      />
    </g>
  </svg>
));
