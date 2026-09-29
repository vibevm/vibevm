/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";

import type { StageId } from "./strings.ts";

/**
 * The six stage emblems, each the meaning of its stage said in geometry
 * and each drawn in the vocabulary `art.tsx` declares: ink is the thing
 * built, terracotta is the project's own, gold is the specification,
 * cobalt is the machine's tooling, green is Zap's. All six stand on the
 * same floor line at the same height, so that read down the page they
 * are one series and not six pictures — and the sixth is built out of
 * the first five, which is what the road says.
 *
 * Every element that pops or draws in carries `--at`, its place in the
 * choreography (`styles.css`): each plate reveals itself as it enters
 * the window, so a reader scrolling down the road watches each station
 * assemble. The grids and shelves are written as small tables rather than
 * as forty hand-placed rectangles, so a reader of this file sees the
 * composition and not its arithmetic.
 */

/** A square of the community grid: where it is, and what it is. */
type Cell = readonly [x: number, y: number, kind: Kind];

/** A cover on a shelf: where it starts, how wide, how tall, what colour. */
type Cover = readonly [x: number, w: number, h: number, kind: Kind];

type Kind = "ink" | "gold" | "terra" | "cobalt" | "hollow";

const KIND_CLASS: Readonly<Record<Kind, string>> = {
  ink: "rm-s-ink",
  gold: "rm-s-gold",
  terra: "rm-s-terra",
  cobalt: "rm-s-cobalt",
  hollow: "rm-s-hollow",
};

/** The drafting air every emblem is drawn in: two crosshairs, one floor. */
const Air = component$(() => (
  <>
    <g class="rm-em__cross" stroke-width="1">
      <path d="M40 34v12M34 40h12" />
      <path d="M286 178v12M280 184h12" />
    </g>
    <line class="rm-s-floor" x1="24" y1="196" x2="296" y2="196" />
    <g class="rm-s-tick" stroke-width="1">
      <path d="M100 193v6" />
      <path d="M220 193v6" />
    </g>
  </>
));

/**
 * Developer Preview 2: the core opens. One large ink square with a
 * doorway cut into its side; three module squares drawn out of the
 * doorway on thin edges, still hollow, and one filled terracotta module
 * already docked in the opening.
 */
const CoreOpens = component$(() => (
  <svg
    class="rm-em"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <Air />
    <path class="rm-s-hair" d="M128 36v12M128 172v12" />
    <path
      class="rm-s-ink rm-pop"
      d="M70 52H186V90H160V130H186V168H70Z"
      style="--at:1"
    />
    <g class="rm-em__edges">
      <path
        class="rm-s-edge rm-draw"
        d="M194 110 224 71"
        pathLength="1"
        style="--at:7"
      />
      <path
        class="rm-s-edge rm-draw"
        d="M194 110 256 113"
        pathLength="1"
        style="--at:9"
      />
      <path
        class="rm-s-edge rm-draw"
        d="M194 110 224 161"
        pathLength="1"
        style="--at:11"
      />
    </g>
    <rect
      class="rm-s-terra rm-pop"
      x="166"
      y="96"
      width="28"
      height="28"
      style="--at:5"
    />
    <rect
      class="rm-s-hollow rm-pop"
      x="224"
      y="56"
      width="30"
      height="30"
      style="--at:10"
    />
    <rect
      class="rm-s-hollow rm-pop"
      x="256"
      y="98"
      width="30"
      height="30"
      style="--at:12"
    />
    <rect
      class="rm-s-hollow rm-pop"
      x="224"
      y="146"
      width="30"
      height="30"
      style="--at:14"
    />
  </svg>
));

/**
 * Zap: the orbital map, the page's own signature drawn in the family's
 * air. A goal core, dashed rings of decomposition around it, work nodes
 * on the rings, one terracotta node for the registered project, and one
 * trajectory of verified prerequisites leaving toward the edge. The rings
 * turn once in four minutes, as they do on the page they come from.
 */
const Orbital = component$(() => (
  <svg
    class="rm-em rm-em--zap"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <Air />
    <g class="rm-em__stars">
      <circle cx="30" cy="30" r="1.2" />
      <circle cx="96" cy="16" r="1.1" />
      <circle cx="210" cy="24" r="1.4" />
      <circle cx="292" cy="58" r="1.1" />
      <circle cx="20" cy="130" r="1.2" />
      <circle cx="262" cy="150" r="1.3" />
    </g>
    <g class="rm-em__orbits">
      <circle class="rm-s-ring" cx="128" cy="112" r="30" />
      <circle class="rm-s-ring" cx="128" cy="112" r="54" />
      <circle class="rm-s-ring" cx="128" cy="112" r="78" />
    </g>
    <path
      class="rm-s-orbit rm-draw"
      d="M128 112 160 94 216 86 262 118 296 130"
      pathLength="1"
      style="--at:8"
    />
    <path
      class="rm-s-orbit-arrow rm-pop"
      d="M286 120 297 130 284 135"
      style="--at:20"
    />
    <g class="rm-pop" style="--at:2">
      <circle class="rm-s-zap-halo" cx="128" cy="112" r="16" />
      <circle class="rm-s-zap-ring" cx="128" cy="112" r="11" />
      <circle class="rm-s-zap" cx="128" cy="112" r="6" />
    </g>
    <circle class="rm-s-zap rm-pop" cx="160" cy="94" r="4.5" style="--at:10" />
    <circle
      class="rm-s-zap-hollow rm-pop"
      cx="98"
      cy="136"
      r="4"
      style="--at:12"
    />
    <circle class="rm-s-zap rm-pop" cx="216" cy="86" r="4.5" style="--at:14" />
    <circle
      class="rm-s-terra rm-pop"
      cx="172"
      cy="140"
      r="4.5"
      style="--at:16"
    />
    <circle
      class="rm-s-zap-hollow rm-pop"
      cx="74"
      cy="86"
      r="4"
      style="--at:17"
    />
    <circle class="rm-s-zap rm-pop" cx="262" cy="118" r="5.5" style="--at:19" />
  </svg>
));

/** The community grid: six by four, and what stands in each cell. */
const GRID: readonly Cell[] = [
  [56, 40, "terra"],
  [90, 40, "ink"],
  [124, 40, "gold"],
  [158, 40, "ink"],
  [192, 40, "ink"],
  [226, 40, "hollow"],
  [56, 74, "hollow"],
  [90, 74, "ink"],
  [124, 74, "ink"],
  [158, 74, "hollow"],
  [192, 74, "ink"],
  [226, 74, "ink"],
  [56, 108, "ink"],
  [90, 108, "gold"],
  [124, 108, "ink"],
  [158, 108, "ink"],
  [192, 108, "gold"],
  [226, 108, "ink"],
  [56, 142, "ink"],
  [90, 142, "ink"],
  [124, 142, "ink"],
  [158, 142, "gold"],
  [192, 142, "ink"],
  [226, 142, "hollow"],
];

/**
 * The first community packages: a grid of squares, most of them ink, a
 * few gold, a few still empty; three more arrive from outside along
 * dashed diagonals to fill the gaps. The one terracotta square in the
 * corner is the project's own — the packages that were there first.
 */
const PackagesFill = component$(() => (
  <svg
    class="rm-em"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <Air />
    {GRID.map(([x, y, kind], index) => (
      <rect
        key={`${x}-${y}`}
        class={`${KIND_CLASS[kind]} rm-pop`}
        x={x}
        y={y}
        width="26"
        height="26"
        style={`--at:${1 + (index % 6) + Math.floor(index / 6) * 2}`}
      />
    ))}
    <g class="rm-em__arrivals">
      <path class="rm-s-dash rm-fade" d="M272 36 256 46" style="--at:14" />
      <path class="rm-s-dash rm-fade" d="M44 100 54 93" style="--at:16" />
      <path class="rm-s-dash rm-fade" d="M272 165 256 160" style="--at:18" />
    </g>
    <rect
      class="rm-s-ink rm-pop"
      x="272"
      y="14"
      width="26"
      height="26"
      style="--at:15"
    />
    <rect
      class="rm-s-ink rm-pop"
      x="18"
      y="96"
      width="26"
      height="26"
      style="--at:17"
    />
    <rect
      class="rm-s-ink rm-pop"
      x="272"
      y="156"
      width="26"
      height="26"
      style="--at:19"
    />
  </svg>
));

/**
 * IDE plugins: two frames of corner brackets side by side — the two
 * editors — and two plates sliding into them, a cobalt one from the left
 * and an ink one from the right. Under both, one small terracotta square
 * joined to each frame by a thin edge: the same vibe behind both panels.
 */
const PlatesDock = component$(() => (
  <svg
    class="rm-em"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <Air />
    <path class="rm-s-hair" d="M96 40v12M224 40v12" />
    <g class="rm-s-bracket">
      <path d="M40 74V60H54M138 60H152V74M40 134V148H54M138 148H152V134" />
      <path d="M168 74V60H182M266 60H280V74M168 134V148H182M266 148H280V134" />
    </g>
    <rect
      class="rm-s-cobalt rm-slide-l"
      x="60"
      y="78"
      width="72"
      height="52"
      style="--at:6"
    />
    <rect
      class="rm-s-ink rm-slide-r"
      x="188"
      y="78"
      width="72"
      height="52"
      style="--at:9"
    />
    <g class="rm-em__edges">
      <path
        class="rm-s-edge rm-draw"
        d="M160 166 96 150"
        pathLength="1"
        style="--at:15"
      />
      <path
        class="rm-s-edge rm-draw"
        d="M160 166 224 150"
        pathLength="1"
        style="--at:17"
      />
    </g>
    <rect
      class="rm-s-terra rm-pop"
      x="150"
      y="166"
      width="20"
      height="20"
      style="--at:13"
    />
  </svg>
));

/** Three shelves of covers: the bottom of each shelf, and what stands on it. */
const SHELVES: readonly {
  readonly y: number;
  readonly covers: readonly Cover[];
}[] = [
  {
    y: 88,
    covers: [
      [44, 16, 38, "ink"],
      [64, 20, 44, "gold"],
      [88, 14, 34, "ink"],
      [106, 22, 40, "cobalt"],
      [132, 16, 46, "ink"],
      [152, 18, 36, "ink"],
      [174, 24, 42, "terra"],
      [202, 14, 32, "ink"],
      [220, 20, 44, "ink"],
      [244, 16, 38, "gold"],
    ],
  },
  {
    y: 142,
    covers: [
      [44, 22, 42, "cobalt"],
      [70, 16, 36, "ink"],
      [90, 20, 46, "ink"],
      [114, 14, 32, "gold"],
      [132, 18, 40, "ink"],
      [154, 24, 44, "ink"],
      [182, 16, 34, "terra"],
      [202, 20, 42, "ink"],
      [226, 14, 38, "ink"],
    ],
  },
  {
    y: 196,
    covers: [
      [44, 18, 40, "ink"],
      [66, 22, 46, "gold"],
      [92, 16, 36, "ink"],
      [112, 20, 42, "ink"],
      [136, 14, 34, "cobalt"],
      [154, 18, 44, "ink"],
      [204, 22, 40, "ink"],
      [230, 16, 38, "ink"],
    ],
  },
];

/**
 * The library and marketplace: three shelves of covers of unequal height
 * and width, in every colour of the family, and one cover drawn forward
 * and tilted — picked up — leaving its gap on the bottom shelf.
 */
const ShelfOfCovers = component$(() => (
  <svg
    class="rm-em"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="rm-em__cross" stroke-width="1">
      <path d="M40 34v12M34 40h12" />
      <path d="M296 44v12M290 50h12" />
    </g>
    <path class="rm-s-shelf" d="M36 88H284M36 142H284" />
    <line class="rm-s-floor" x1="24" y1="196" x2="296" y2="196" />
    {SHELVES.flatMap((shelf, s) =>
      shelf.covers.map(([x, w, h, kind], index) => (
        <rect
          key={`${shelf.y}-${x}`}
          class={`${KIND_CLASS[kind]} rm-pop`}
          x={x}
          y={shelf.y - h}
          width={w}
          height={h}
          style={`--at:${1 + s * 2 + index}`}
        />
      )),
    )}
    <g class="rm-rise" style="--at:19">
      <rect
        class="rm-s-terra"
        x="256"
        y="100"
        width="40"
        height="58"
        transform="rotate(-10 276 129)"
      />
    </g>
  </svg>
));

/**
 * Spec-Driven Linux: one large plane standing on the floor, and inside it
 * the five earlier shapes assembled into one construction — the opened
 * core, the grid of packages, a plate in its frame, a shelf of covers —
 * joined through a gold diamond at the centre: the package manager that
 * holds the system together. The terracotta circle at the base is the
 * human beside the system.
 */
const WholeSystem = component$(() => (
  <svg
    class="rm-em"
    viewBox="0 0 320 220"
    fill="none"
    aria-hidden="true"
    focusable="false"
  >
    <g class="rm-em__cross" stroke-width="1">
      <path d="M36 36v12M30 42h12" />
      <path d="M290 60v12M284 66h12" />
    </g>
    <line class="rm-s-floor" x1="24" y1="196" x2="296" y2="196" />
    <path class="rm-s-hair" d="M160 8v14M282 110h-14" />
    <rect
      class="rm-s-plane rm-draw"
      x="60"
      y="24"
      width="200"
      height="172"
      pathLength="1"
      style="--at:1"
    />
    <circle class="rm-s-terra rm-pop" cx="42" cy="182" r="14" style="--at:4" />
    <g class="rm-pop" style="--at:8">
      <path class="rm-s-ink" d="M80 44H132V60H120V78H132V96H80Z" />
      <rect class="rm-s-terra" x="124" y="62" width="14" height="14" />
    </g>
    <g class="rm-pop" style="--at:11">
      <rect class="rm-s-ink" x="160" y="44" width="10" height="10" />
      <rect class="rm-s-ink" x="174" y="44" width="10" height="10" />
      <rect class="rm-s-ink" x="188" y="44" width="10" height="10" />
      <rect class="rm-s-ink" x="202" y="44" width="10" height="10" />
      <rect class="rm-s-ink" x="160" y="58" width="10" height="10" />
      <rect class="rm-s-ink" x="174" y="58" width="10" height="10" />
      <rect class="rm-s-gold" x="188" y="58" width="10" height="10" />
      <rect class="rm-s-ink" x="202" y="58" width="10" height="10" />
      <rect class="rm-s-ink" x="160" y="72" width="10" height="10" />
      <rect class="rm-s-ink" x="174" y="72" width="10" height="10" />
      <rect class="rm-s-ink" x="188" y="72" width="10" height="10" />
      <rect class="rm-s-hollow" x="202" y="72" width="10" height="10" />
    </g>
    <g class="rm-pop" style="--at:14">
      <g class="rm-s-bracket">
        <path d="M80 126V118H88M132 118H140V126M80 156V164H88M132 164H140V156" />
      </g>
      <rect class="rm-s-cobalt" x="92" y="128" width="36" height="26" />
    </g>
    <g class="rm-pop" style="--at:17">
      <path class="rm-s-shelf" d="M158 164H242" />
      <rect class="rm-s-ink" x="162" y="144" width="8" height="20" />
      <rect class="rm-s-gold" x="173" y="140" width="10" height="24" />
      <rect class="rm-s-ink" x="186" y="146" width="7" height="18" />
      <rect class="rm-s-ink" x="196" y="142" width="9" height="22" />
      <rect class="rm-s-terra" x="208" y="144" width="11" height="20" />
      <rect class="rm-s-ink" x="222" y="141" width="8" height="23" />
      <rect class="rm-s-ink" x="233" y="147" width="8" height="17" />
    </g>
    <g class="rm-s-hub">
      <path
        class="rm-draw"
        d="M160 106 126 96"
        pathLength="1"
        style="--at:20"
      />
      <path
        class="rm-draw"
        d="M160 106 181 82"
        pathLength="1"
        style="--at:21"
      />
      <path
        class="rm-draw"
        d="M160 106 110 118"
        pathLength="1"
        style="--at:22"
      />
      <path
        class="rm-draw"
        d="M160 106 200 140"
        pathLength="1"
        style="--at:23"
      />
    </g>
    <g class="rm-pop" style="--at:24">
      <rect
        class="rm-s-gold"
        x="152"
        y="98"
        width="16"
        height="16"
        transform="rotate(45 160 106)"
      />
    </g>
  </svg>
));

export type StageEmblemProps = {
  /** Which stage's emblem this is. */
  readonly id: StageId;
};

/**
 * The emblem of one stage, by the id the string table names it with.
 *
 * A switch and not a table, for the reason the map's `MenuArt` is one:
 * the cases are the ids, the switch is exhaustive, and a stage added to
 * the list without a drawing is a type error here rather than a plate
 * with a hole where its picture should be.
 */
export const StageEmblem = component$<StageEmblemProps>((props) => {
  switch (props.id) {
    case "dp2":
      return <CoreOpens />;
    case "zap":
      return <Orbital />;
    case "community":
      return <PackagesFill />;
    case "ide":
      return <PlatesDock />;
    case "library":
      return <ShelfOfCovers />;
    case "linux":
      return <WholeSystem />;
  }
});
