/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import { type Locale } from "../landing/i18n.ts";
import { whyHref } from "../why/paths.ts";
import shared from "../why/shared.css?inline";
import {
  IntentField,
  InheritLadder,
  SectionMark,
  StarPlug,
  TwoLayers,
} from "./art.tsx";
import { STRINGS } from "./i18n.ts";
import styles from "./styles.css?inline";

export type BigVisionProps = {
  readonly locale: Locale;
};

/**
 * `/vision/` — the essay «Большой Вижен» / “The Big Vision”, the
 * worldview the rest of the marketing half argues pieces of.
 *
 * The composition is an editorial long read rather than a product page:
 * a hero over a full-width drawing, the author's own first paragraph set
 * large as the lede, sections in a reading measure, four figures placed
 * where the argument needs them, one technical inset for the
 * specification/code dichotomy, the essay's single law printed on the
 * family's inverted cream band, the author's OOP sentence pulled out as
 * the one quote, the disclaimer in its own visible frame, and a single
 * tie out to the page where the worldview becomes a discipline.
 *
 * The visual vocabulary is declared once in `art.tsx` and repeated by
 * the stylesheet: a terracotta circle is human intention, a tilted
 * cobalt square is machine intention, gold is the expensive
 * probabilistic specification layer, plain ink is the cheap
 * deterministic code — and thin lines between them are the traceable
 * edges. Colour never carries the distinction alone: the shapes differ
 * before the tones do.
 *
 * Nothing hydrates. The edges draw themselves and the two sources pop —
 * all of it CSS over markup the server already wrote, and all of it
 * still for a reader who asked for stillness.
 */
export const BigVision = component$<BigVisionProps>((props) => {
  useStyles$(shared);
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const aiNativeHref = whyHref("ai-native", props.locale);

  return (
    <div class="why-page bv-page">
      <div class="bv-hero">
        <div class="why-shell">
          <p class="why-eyebrow">
            <span class="why-dot" aria-hidden="true" />
            {t.eyebrow}
          </p>
          <h1 class="bv-title">{t.title}</h1>
        </div>
        <figure class="bv-canvas" role="img" aria-label={t.heroAlt}>
          <div class="bv-canvas__frame">
            <IntentField />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.heroCaption}
          </figcaption>
        </figure>
      </div>

      <div class="why-shell bv-measure">
        <p class="bv-lede">{t.lede}</p>
      </div>

      <section class="bv-section" aria-labelledby="bv-beings-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-beings-h">
            {t.beingsH}
          </h2>
          <p>{t.beingsP1}</p>
          <p>{t.beingsP2}</p>
        </div>
      </section>

      <section class="bv-section" aria-labelledby="bv-intent-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-intent-h">
            {t.intentH}
          </h2>
          <p>{t.intentP1}</p>
          <p>{t.intentP2}</p>
          <p>{t.intentP3}</p>
        </div>
      </section>

      <div class="bv-break" aria-hidden="true">
        <SectionMark />
      </div>

      <section class="bv-section" aria-labelledby="bv-sdd-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-sdd-h">
            {t.sddH}
          </h2>
          <p>{t.sddP1}</p>
          <p>{t.sddP2}</p>
          <p>{t.sddP3}</p>
        </div>
        <figure class="bv-figure" role="img" aria-label={t.starAlt}>
          <div class="why-shell bv-figure__frame bv-figure__frame--star">
            <StarPlug />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.starCaption}
          </figcaption>
        </figure>
      </section>

      <section class="bv-section" aria-labelledby="bv-split-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-split-h">
            {t.splitH}
          </h2>
          <p>{t.splitP1}</p>
          <p>{t.splitP2}</p>
          <p>{t.splitP3}</p>
          <p>{t.splitP4}</p>
        </div>
        <div class="why-shell">
          <div class="bv-dichotomy" role="group" aria-label={t.dichotomyLabel}>
            <div class="bv-dichotomy__col bv-dichotomy__col--spec">
              <h3>
                <span class="bv-chip bv-chip--spec" aria-hidden="true" />
                {t.specHead}
              </h3>
              <ul>
                {t.specPoints.map((point) => (
                  <li key={point}>{point}</li>
                ))}
              </ul>
            </div>
            <div class="bv-dichotomy__col bv-dichotomy__col--code">
              <h3>
                <span class="bv-chip bv-chip--code" aria-hidden="true" />
                {t.codeHead}
              </h3>
              <ul>
                {t.codePoints.map((point) => (
                  <li key={point}>{point}</li>
                ))}
              </ul>
            </div>
          </div>
        </div>
        <div class="why-shell bv-measure">
          <p>{t.splitP5}</p>
          <p>{t.splitP6}</p>
        </div>
        <figure class="bv-figure" role="img" aria-label={t.layersAlt}>
          <div class="why-shell bv-figure__frame">
            <TwoLayers />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.layersCaption}
          </figcaption>
        </figure>
      </section>

      <section class="bv-law" aria-labelledby="bv-law-h">
        <div class="why-shell bv-law__inner">
          <p class="bv-law__k">{t.lawK}</p>
          <p class="bv-law__lead">{t.lawLead}</p>
          <blockquote class="bv-law__quote" id="bv-law-h">
            {t.lawQuote}
          </blockquote>
          <p class="bv-law__body">{t.lawBody}</p>
        </div>
      </section>

      <section class="bv-section" aria-labelledby="bv-edges-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-edges-h">
            {t.edgesH}
          </h2>
          <p>{t.edgesP1}</p>
          <p>{t.edgesP2}</p>
        </div>
        <div class="why-shell bv-inherit">
          <div class="bv-measure bv-inherit__text">
            <p>{t.edgesP3}</p>
            <blockquote class="bv-pull">{t.oopQuote}</blockquote>
          </div>
          <figure
            class="bv-inherit__figure"
            role="img"
            aria-label={t.ladderAlt}
          >
            <InheritLadder />
            <figcaption aria-hidden="true">{t.ladderCaption}</figcaption>
          </figure>
        </div>
      </section>

      <div class="bv-break" aria-hidden="true">
        <SectionMark />
      </div>

      <section class="bv-section" aria-labelledby="bv-together-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-together-h">
            {t.togetherH}
          </h2>
          <p>{t.togetherP1}</p>
        </div>
      </section>

      <section class="bv-section" aria-labelledby="bv-start-h">
        <div class="why-shell bv-measure">
          <h2 class="bv-h" id="bv-start-h">
            {t.startH}
          </h2>
          <p>{t.startP1}</p>
        </div>
      </section>

      <div class="why-shell bv-measure">
        <aside class="bv-disclaimer" aria-label={t.disclaimerK}>
          <p class="bv-disclaimer__k" aria-hidden="true">
            {t.disclaimerK}
          </p>
          <p
            class="bv-disclaimer__body"
            dangerouslySetInnerHTML={t.disclaimerHtml}
          />
        </aside>

        <a class="bv-tie" href={aiNativeHref}>
          <span class="bv-tie__k">{t.tieK}</span>
          <b>{t.tieHead}</b>
          <p>{t.tieBody}</p>
          <span class="bv-tie__link">{t.tieLink} →</span>
        </a>
      </div>
    </div>
  );
});
