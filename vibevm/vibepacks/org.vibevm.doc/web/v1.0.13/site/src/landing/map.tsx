/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import { MenuArt } from "./art.tsx";
import { type Locale, STRINGS } from "./i18n.ts";
import styles from "./map.css?inline";
import { landingMenu } from "./menu.ts";

export type LandingMapProps = {
  readonly locale: Locale;
};

/**
 * The map under the landing's content: the header's destinations, drawn
 * large (owner, 2026-09-26).
 *
 * The header is fourteen-pixel words in two rows, and there is a kind of
 * reader who never reads them. This is the same list for that reader:
 * every destination as a thing of its own, with a picture, its name and
 * one or two lines about what is there — and the whole of it is a link.
 * The list is the header's (`menu.ts`), not a second copy, so the map
 * says exactly what the header says, in the header's order, and a new
 * entry in one is a new plate in the other.
 *
 * The composition is two bands, one per header row, and inside each the
 * plates are planes of unequal size on a twelve-column ground, placed by
 * the grid from the size each entry declares: in the first band a tower
 * two rows high for the manual, two small squares for the two mirrors of
 * the source, a wide plate for the channels; in the second the essay as
 * one band across the whole row, and the three products as three equal
 * thirds under it — three accents under one roof, with the worldview
 * they are pieces of above them. Two bands, two different arrangements,
 * because a suprematist composition is asymmetric or it is a grid.
 *
 * Each plate is one link and the whole plate is it, as the channels page
 * does it: one target, one stop, one ring. Its accessible name is the
 * destination's name alone (`aria-labelledby`), so a list of links reads
 * as the header reads; the drawing is described once, on the frame
 * around it, and the `svg` inside is hidden. Nothing hydrates: the
 * reveal, the hover and the one turning orbit are CSS over markup the
 * server already wrote, and all of it stands still for a reader who
 * asked for stillness.
 */
export const LandingMap = component$<LandingMapProps>((props) => {
  useStyles$(styles);
  const t = STRINGS[props.locale];
  /* The landing is no destination of its own, so nothing is current. */
  const menu = landingMenu(props.locale, "");

  return (
    <section class="landing-map" aria-labelledby="landing-map-h">
      <div class="landing-map__head">
        {/* The family's three masses, as the essay pauses between its
            sections with them: human circle, gold specification, machine
            square. It says which family this page belongs to before a
            word is read. */}
        <svg
          class="landing-map__mark"
          viewBox="0 0 76 16"
          fill="none"
          aria-hidden="true"
          focusable="false"
        >
          <circle class="landing-map__mark-human" cx="8" cy="8" r="6" />
          <rect
            class="landing-map__mark-spec"
            x="33"
            y="3"
            width="10"
            height="10"
            transform="rotate(45 38 8)"
          />
          <rect
            class="landing-map__mark-machine"
            x="62"
            y="2"
            width="12"
            height="12"
          />
        </svg>
        <p class="landing-map__k">{t.mapK}</p>
        <h2 class="landing-map__title" id="landing-map-h">
          {t.mapTitle}
        </h2>
        <p class="landing-map__lede">{t.mapLede}</p>
      </div>

      {menu.rows.map((row) => (
        <div class="landing-map__band" key={row.id}>
          <p class="landing-map__row-k" id={`landing-map-${row.id}-k`}>
            {t.mapRows[row.id]}
          </p>
          {/* A list, and it says so: a reader using a screen reader is told
              how many places a row holds before walking them. */}
          <ul
            class="landing-map__grid"
            aria-labelledby={`landing-map-${row.id}-k`}
          >
            {row.entries.map((one) => {
              const plate = t.mapPlates[one.id];
              return (
                <li
                  key={one.id}
                  class={`landing-map__cell landing-map__cell--${one.plate}`}
                >
                  <a
                    class={`landing-map__plate landing-map__plate--${one.plate} landing-map__plate--${one.id}`}
                    href={one.href}
                    aria-labelledby={`landing-map-${one.id}-name`}
                    {...(one.offSite ? { rel: "noopener" } : {})}
                  >
                    <div
                      class="landing-map__art"
                      role="img"
                      aria-label={plate.alt}
                    >
                      <MenuArt id={one.id} />
                    </div>
                    <div class="landing-map__text">
                      <h3
                        class="landing-map__name"
                        id={`landing-map-${one.id}-name`}
                      >
                        {one.label}
                      </h3>
                      <p class="landing-map__body">{plate.body}</p>
                    </div>
                  </a>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </section>
  );
});
