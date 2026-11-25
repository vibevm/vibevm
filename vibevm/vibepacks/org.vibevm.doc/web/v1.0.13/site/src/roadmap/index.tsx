/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import { type Locale } from "../landing/i18n.ts";
import { href } from "../lib/href.ts";
import { newsHref } from "../news/paths.ts";
import { visionHref } from "../vision/paths.ts";
import { whyHref } from "../why/paths.ts";
import shared from "../why/shared.css?inline";
import { HereMark, RoadAhead, RoadmapMark, RouteGlyph } from "./art.tsx";
import { StageEmblem } from "./emblems.tsx";
import { STRINGS, type Stage } from "./i18n.ts";
import { SPEC_DRIVEN_LINUX_URL } from "./paths.ts";
import styles from "./styles.css?inline";

export type RoadmapProps = {
  readonly locale: Locale;
};

/**
 * `/roadmap/` — where VibeVM is going, in the owner's order.
 *
 * The composition is a road read twice. First as one picture: a hero
 * over a full-width drawing in which the road is a long ink diagonal and
 * the six stations stand on it, growing from a single square into a whole
 * system. Then as a walk: the marker that says where the project stands
 * today, and under it the six stages down a rail, each with an emblem
 * that says its meaning in geometry and three sentences that say it in
 * words — its name, what it is, what it changes for the reader. One rule
 * on the family's inverted cream band says how the page is to be read:
 * in this order, and with no dates. Three ways out close it.
 *
 * Nothing hydrates. The road draws itself, the stations pop, each plate
 * reveals as it enters the window — all of it CSS over markup the server
 * already wrote, and all of it still for a reader who asked for stillness.
 */
export const Roadmap = component$<RoadmapProps>((props) => {
  useStyles$(shared);
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const locale = props.locale;

  /* The two stages that lead somewhere: Zap to its own page on this site,
     in the reader's language; Spec-Driven Linux to its future home off
     it. Resolved here from the stage's id, so the string table carries
     words and never a path (`lib/href.ts`). */
  const stageHref = (stage: Stage): string | null => {
    switch (stage.id) {
      case "zap":
        return whyHref("zap", locale);
      case "linux":
        return SPEC_DRIVEN_LINUX_URL;
      default:
        return null;
    }
  };

  return (
    <div class="why-page rm-page">
      <div class="rm-hero">
        <div class="why-shell">
          <p class="why-eyebrow">
            <span class="why-dot" aria-hidden="true" />
            {t.eyebrow}
          </p>
          <h1 class="why-headline rm-title">{t.title}</h1>
          <p class="rm-lead">{t.lead}</p>
        </div>
        <figure class="rm-canvas" role="img" aria-label={t.heroAlt}>
          <div class="rm-canvas__frame">
            <RoadAhead />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.heroCaption}
          </figcaption>
        </figure>
      </div>

      <section class="rm-section" aria-labelledby="rm-stages-h">
        <div class="why-shell">
          <p class="rm-k">{t.stagesK}</p>
          <h2 class="rm-h" id="rm-stages-h">
            {t.stagesH}
          </h2>

          <div class="rm-route">
            {/* The marker: not a stage, and not numbered — the point the
                rail starts from. Its node is the terracotta circle that
                stands at the foot of the road in the hero. */}
            <div class="rm-here">
              <span class="rm-here__dot" aria-hidden="true" />
              <figure class="rm-here__fig" role="img" aria-label={t.hereAlt}>
                <div class="rm-plate">
                  <HereMark />
                </div>
              </figure>
              <div class="rm-here__text">
                <p class="rm-here__k">{t.hereK}</p>
                <h3 class="rm-here__name">{t.hereName}</h3>
                <p class="rm-here__body">{t.hereBody}</p>
              </div>
            </div>

            {/* An ordered list, and it says so: the order is the content,
                and a reader using a screen reader is told how many stages
                there are before walking them. The printed number is
                decoration over that; the list carries the count. */}
            <ol class="rm-stages" aria-label={t.routeLabel}>
              {t.stages.map((stage, index) => {
                const to = stageHref(stage);
                return (
                  <li key={stage.id} class={`rm-stage rm-stage--${stage.id}`}>
                    <span class="rm-stage__n" aria-hidden="true">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <figure
                      class="rm-stage__fig"
                      role="img"
                      aria-label={stage.alt}
                    >
                      <div class="rm-plate">
                        <StageEmblem id={stage.id} />
                      </div>
                    </figure>
                    <div class="rm-stage__text">
                      <h3 class="rm-stage__name">{stage.name}</h3>
                      <p class="rm-stage__sub">{stage.sub}</p>
                      <p class="rm-stage__body">{stage.body}</p>
                      {stage.link !== undefined && to !== null && (
                        <p class="rm-stage__home">
                          {stage.link.lead}{" "}
                          <a
                            href={to}
                            {...(to === SPEC_DRIVEN_LINUX_URL
                              ? { rel: "noopener" }
                              : {})}
                          >
                            {stage.link.text}
                          </a>
                        </p>
                      )}
                    </div>
                  </li>
                );
              })}
            </ol>
          </div>
        </div>
      </section>

      <div class="rm-break" aria-hidden="true">
        <RoadmapMark />
      </div>

      <section class="rm-band" aria-labelledby="rm-band-h">
        <div class="why-shell rm-band__grid">
          <div>
            <p class="rm-band__k">{t.bandK}</p>
            <blockquote class="rm-band__quote" id="rm-band-h">
              {t.bandQuote}
            </blockquote>
            <p class="rm-band__body">{t.bandBody}</p>
          </div>
          <div class="rm-band__fig" aria-hidden="true">
            <RouteGlyph />
          </div>
        </div>
      </section>

      <section class="rm-section rm-ties-wrap" aria-labelledby="rm-ties-k">
        <div class="why-shell">
          <p class="rm-k" id="rm-ties-k">
            {t.tiesK}
          </p>
          <div class="rm-ties">
            <a class="rm-tie rm-tie--news" href={newsHref(locale)}>
              <b>{t.tieNewsHead}</b>
              <p>{t.tieNewsBody}</p>
              <span class="rm-tie__link">{t.tieNewsLink} →</span>
            </a>
            <a class="rm-tie rm-tie--vision" href={visionHref(locale)}>
              <b>{t.tieVisionHead}</b>
              <p>{t.tieVisionBody}</p>
              <span class="rm-tie__link">{t.tieVisionLink} →</span>
            </a>
            <a class="rm-tie rm-tie--docs" href={href("doc/")}>
              <b>{t.tieDocsHead}</b>
              <p>{t.tieDocsBody}</p>
              <span class="rm-tie__link">{t.tieDocsLink} →</span>
            </a>
          </div>
        </div>
      </section>
    </div>
  );
});
