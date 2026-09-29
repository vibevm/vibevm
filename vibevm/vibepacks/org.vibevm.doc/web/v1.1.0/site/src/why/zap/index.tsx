/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

/* `href`, `localePath` and `COMMANDS` serve only the start section, which
   is commented out below until the release; they come back with it. */
// import { href } from "../../lib/href.ts";
import type { Locale /* , localePath */ } from "../../landing/i18n.ts";
import shared from "../shared.css?inline";
import { whyHref } from "../paths.ts";
import { Orbital, ReadingGoal, ReadingOrder } from "./art.tsx";
import { /* COMMANDS, */ RELEASE_MONTH, STRINGS } from "./i18n.ts";
import styles from "./styles.css?inline";

export type WhyZapProps = {
  readonly locale: Locale;
};

/**
 * `/why/zap` — the product page for Zap: the engine, Wayfinder and Quick
 * Lens.
 *
 * The composition is the source's, section for section: a two-column
 * hero with the orbital map beside it, three problem cards, the
 * architecture as a flow of three nodes, the two readings of the plan
 * graph, four capability cards, five laws on hairlines, the start list
 * with two end-notes, and a closing paragraph.
 *
 * Over all of it runs one strip the source did not have: the release
 * banner, added by the owner after the port. It is the first thing in
 * the page and it borrows the hero's own drawing rather than a new one —
 * the ping becomes a beacon, and the trajectory that leaves the orbital
 * map becomes the line that runs from the words to the month. It is a
 * paragraph, not a heading: the page still opens its outline with its
 * headline. Until that month there is nothing to install, so the start
 * section — the list of commands «from zero to a running coordinator»
 * with its two end-notes — is commented out by the owner's decision of
 * 2026-09-25, together with the hero's button that led to it. Its
 * strings stay in `i18n.ts`, so it returns by removing the comments.
 *
 * The page is drawn in a darker room than the rest of the site — a
 * deep-space ground with its own panels and hairlines — and that is the
 * design and not an accident of the port. In the light theme the room
 * has no counterpart, so the page keeps the site's surfaces there and
 * carries its identity in its green alone; the dark theme, which is what
 * the page was drawn on and what this site serves by default, is the
 * drawing.
 *
 * Nothing here hydrates. The orbits turn, the trajectory draws itself,
 * the ping expands and the banner's spark runs its line — all of it in
 * CSS, over elements the server already wrote, and all of it off for a
 * reader who asked for stillness.
 */
export const WhyZap = component$<WhyZapProps>((props) => {
  useStyles$(shared);
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const vibevmHref = whyHref("vibevm", props.locale);
  /* The link out of the first step goes to the landing in this page's
     own language, through the same function every other address on the
     site is written with. Commented out with the start section. */
  // const homeHref = href(localePath(props.locale));

  return (
    <div class="why-page why-zap">
      <div class="wz-soon">
        <p class="why-shell wz-soon__line">
          <span class="wz-soon__beacon" aria-hidden="true" />
          <span class="wz-soon__label">{t.soonLabel}</span>
          <span class="wz-soon__path" aria-hidden="true">
            <i />
          </span>
          <time class="wz-soon__date" dateTime={RELEASE_MONTH}>
            {t.soonDate}
          </time>
        </p>
      </div>

      <section class="wz-hero" aria-labelledby="wz-headline">
        <div class="why-shell wz-hero__grid">
          <div class="wz-hero__copy">
            <p class="why-eyebrow">
              <span class="why-dot" aria-hidden="true" />
              {t.eyebrow}
            </p>
            <h1
              class="why-headline wz-headline"
              id="wz-headline"
              dangerouslySetInnerHTML={t.headlineHtml}
            />
            <p class="why-lead wz-lead">{t.lead}</p>
            <div class="why-cta">
              {/* The button to the start section, commented out with it.
              <a class="why-btn why-btn--primary" href="#zap-start">
                {t.ctaStart}
                <svg
                  class="why-arrow"
                  width="14"
                  height="14"
                  viewBox="0 0 14 14"
                  fill="none"
                  aria-hidden="true"
                >
                  <path
                    d="M7 3v8M3 7l4 4 4-4"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </a>
              */}
              <a class="why-btn why-btn--ghost" href={vibevmHref}>
                {t.ctaVibevm}
              </a>
              <p class="why-release">
                <span class="why-release-dot" aria-hidden="true" />
                {t.badge}
              </p>
            </div>
          </div>
          <figure class="wz-hero__art" role="img" aria-label={t.heroArtAlt}>
            <Orbital />
            <figcaption aria-hidden="true">{t.heroCaption}</figcaption>
          </figure>
        </div>
      </section>

      <section class="wz-section" aria-labelledby="wz-problem-h">
        <div class="why-shell">
          <p class="wz-k">{t.problemK}</p>
          <h2 class="wz-h" id="wz-problem-h">
            {t.problemH}
          </h2>
          <div class="wz-problems">
            {t.problems.map((one) => (
              <article class="wz-card" key={one.head}>
                <h3>{one.head}</h3>
                <p>{one.body}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section class="wz-section" aria-labelledby="wz-thesis-h">
        <div class="why-shell">
          <p class="wz-k">{t.thesisK}</p>
          <h2 class="wz-h" id="wz-thesis-h">
            {t.thesisH}
          </h2>
          <p class="wz-body">{t.thesisBody}</p>

          <div class="wz-arch" role="img" aria-label={t.archAlt}>
            <div class="wz-arch__node">
              <b>{t.archAgentsHead}</b>
              <span>{t.archAgentsBody}</span>
            </div>
            <div class="wz-arch__link" aria-hidden="true">
              <i />
              <em>{t.archLink1}</em>
            </div>
            <div class="wz-arch__node wz-arch__node--core">
              <b>{t.archCoreHead}</b>
              <span>{t.archCoreBody}</span>
            </div>
            <div class="wz-arch__link" aria-hidden="true">
              <i />
              <em>{t.archLink2}</em>
            </div>
            <div class="wz-arch__node">
              <b>{t.archViewHead}</b>
              <span>{t.archViewBody}</span>
            </div>
          </div>
          <p class="wz-note">{t.archNote}</p>
        </div>
      </section>

      <section class="wz-section" aria-labelledby="wz-map-h">
        <div class="why-shell">
          <p class="wz-k">{t.mapK}</p>
          <h2 class="wz-h" id="wz-map-h">
            {t.mapH}
          </h2>
          <p class="wz-body">{t.mapBody}</p>
          <div class="wz-readings">
            <figure class="wz-reading">
              <ReadingGoal />
              <figcaption>
                <b>{t.readingGoalHead}</b> — {t.readingGoalBody}
              </figcaption>
            </figure>
            <figure class="wz-reading">
              <ReadingOrder />
              <figcaption>
                <b>{t.readingOrderHead}</b> — {t.readingOrderBody}
              </figcaption>
            </figure>
          </div>
        </div>
      </section>

      <section class="wz-section" aria-labelledby="wz-caps-h">
        <div class="why-shell">
          <p class="wz-k">{t.capsK}</p>
          <h2 class="wz-h" id="wz-caps-h">
            {t.capsH}
          </h2>
          <div class="wz-caps">
            {t.caps.map((one) => (
              <article class="wz-card" key={one.head}>
                <h3>{one.head}</h3>
                <p>{one.body}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section class="wz-section" aria-labelledby="wz-laws-h">
        <div class="why-shell">
          <p class="wz-k">{t.lawsK}</p>
          <h2 class="wz-h" id="wz-laws-h">
            {t.lawsH}
          </h2>
          <ul class="wz-laws">
            {t.laws.map((law) => (
              <li key={law.rule}>
                <span class="wz-laws__mark" aria-hidden="true" />
                <div>
                  <b>{law.rule}</b>
                  <p>{law.why}</p>
                </div>
              </li>
            ))}
          </ul>
        </div>
      </section>

      {/* The start section, «from zero to a running coordinator»: commented
          out by the owner until the release (2026-09-25).
      <section class="wz-section" id="zap-start" aria-labelledby="wz-start-h">
        <div class="why-shell">
          <p class="wz-k">{t.startK}</p>
          <h2 class="wz-h" id="wz-start-h">
            {t.startH}
          </h2>
          <p class="wz-body">{t.startNote}</p>
          <ol class="wz-steps">
            {t.steps.map((step) => (
              <li key={step.kind === "cmd" ? step.cmd : step.head}>
                {step.kind === "cmd" ? (
                  <div class="why-cmd wz-cmd">
                    <span class="why-cmd__prompt">{COMMANDS.prompt}</span>
                    <code>{step.cmd}</code>
                  </div>
                ) : (
                  <h3>{step.head}</h3>
                )}
                <p>
                  {step.body}
                  {step.kind === "cmd" && step.linkText !== undefined && (
                    <>
                      {" "}
                      <a class="wz-link" href={homeHref}>
                        {step.linkText} →
                      </a>
                    </>
                  )}
                </p>
              </li>
            ))}
          </ol>

          <div class="wz-endnotes">
            <article class="wz-card">
              <h3>{t.statusHead}</h3>
              <p>{t.statusBody}</p>
            </article>
            <article class="wz-card wz-family">
              <h3>{t.familyHead}</h3>
              <p>{t.familyBody}</p>
              <a class="wz-link wz-family__link" href={vibevmHref}>
                {t.familyLink} →
              </a>
            </article>
          </div>
        </div>
      </section>
      */}

      <section
        class="wz-section wz-summary-wrap"
        aria-labelledby="wz-summary-k"
      >
        <div class="why-shell">
          <p class="wz-k" id="wz-summary-k">
            {t.summaryK}
          </p>
          <blockquote
            class="wz-summary"
            dangerouslySetInnerHTML={t.summaryHtml}
          />
        </div>
      </section>
    </div>
  );
});
