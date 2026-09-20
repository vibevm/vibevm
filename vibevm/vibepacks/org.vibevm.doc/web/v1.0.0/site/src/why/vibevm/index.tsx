/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import { GITHUB_URL, GITVERSE_URL, type Locale } from "../../landing/i18n.ts";
import shared from "../shared.css?inline";
import { whyHref } from "../paths.ts";
import { Frieze } from "./frieze.tsx";
import { COMMANDS, STRINGS } from "./i18n.ts";
import styles from "./styles.css?inline";

export type WhyVibevmProps = {
  readonly locale: Locale;
};

/**
 * `/why/vibevm` — the product page for VibeVM itself: the `vibe` CLI and
 * the package model behind it.
 *
 * The composition is the source's, section for section: a hero over a
 * full-width frieze, the problem in a split column, the two trees and
 * the computed lane drawn as nodes, an inverted cream band carrying the
 * token-economics argument, four capability cards, five commands, the
 * fit-and-not pair, the start block with the tie to Zap, and a closing
 * paragraph. Nothing was reordered and nothing was dropped.
 *
 * It renders to markup and stays there. Two things on the page move —
 * the frieze draws itself once and the status dot breathes — and both
 * are CSS over elements the server already wrote. There is no state, no
 * handler and no `useVisibleTask$`, which is what makes a marketing page
 * of this length cost a reader nothing after the first paint.
 *
 * The two `dangerouslySetInnerHTML` calls carry the same justification
 * the landing's hero carries: both strings hold exactly one inline
 * element — an `<em>` around the accent word, a `<code>` around a
 * command — and both come from the site's own compiled string table,
 * never from a request, a file on disk, or a reader.
 */
export const WhyVibevm = component$<WhyVibevmProps>((props) => {
  useStyles$(shared);
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const zapHref = whyHref("zap", props.locale);

  return (
    <div class="why-page why-vv">
      <section class="vv-hero" aria-labelledby="vv-headline">
        <div class="why-shell">
          <p class="why-eyebrow">
            <span class="why-dot" aria-hidden="true" />
            {t.eyebrow}
          </p>
          <h1
            class="why-headline vv-headline"
            id="vv-headline"
            dangerouslySetInnerHTML={t.headlineHtml}
          />
          <p class="why-lead vv-lead">{t.lead}</p>
          <div class="why-cta">
            <a class="why-btn why-btn--primary" href="#vv-start">
              {t.ctaInstall}
              <svg
                class="why-arrow vv-arrow"
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
            <a class="why-btn why-btn--ghost" href={zapHref}>
              {t.ctaZap}
            </a>
          </div>
        </div>
        <figure class="vv-frieze-wrap" role="img" aria-label={t.friezeAlt}>
          <div class="vv-frieze-canvas">
            <Frieze />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.friezeCaption}
          </figcaption>
        </figure>
      </section>

      <section class="vv-section" aria-labelledby="vv-problem-h">
        <div class="why-shell vv-split">
          <div>
            <p class="vv-k">{t.problemK}</p>
            <h2 class="vv-h" id="vv-problem-h">
              {t.problemH}
            </h2>
          </div>
          <div>
            <p class="vv-body">{t.problemBody}</p>
            <ul class="vv-symptoms">
              {t.problemPoints.map((point) => (
                <li key={point}>{point}</li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section class="vv-section" aria-labelledby="vv-thesis-h">
        <div class="why-shell">
          <p class="vv-k">{t.thesisK}</p>
          <h2 class="vv-h" id="vv-thesis-h">
            {t.thesisH}
          </h2>
          <p class="vv-body vv-body--wide">{t.thesisBody}</p>

          <div class="vv-arch" role="img" aria-label={t.archAlt}>
            <div class="vv-arch__col">
              <div class="vv-arch__node">
                <span class="vv-tag">{t.archYoursTag}</span>
                <b>{t.archYoursHead}</b>
                <span class="vv-arch__sub">{t.archYoursBody}</span>
              </div>
              <div class="vv-arch__node">
                <span class="vv-tag vv-tag--accent">{t.archDepsTag}</span>
                <b>{t.archDepsHead}</b>
                <span class="vv-arch__sub">{t.archDepsBody}</span>
              </div>
            </div>
            <div class="vv-arch__join" aria-hidden="true">
              <i />
            </div>
            <div class="vv-arch__node vv-arch__node--lane">
              <span class="vv-tag vv-tag--accent">{t.archLaneTag}</span>
              <b>{t.archLaneHead}</b>
              <span class="vv-lane-row">
                <em>1</em>
                {t.archLaneStatic}
              </span>
              <span class="vv-lane-row">
                <em>2</em>
                {t.archLaneIndex}
              </span>
            </div>
            <div class="vv-arch__join vv-arch__join--arrow" aria-hidden="true">
              <i />
            </div>
            <div class="vv-arch__node">
              <b>{t.archAgentHead}</b>
              <span class="vv-arch__sub">{t.archAgentBody}</span>
            </div>
          </div>
          <p class="vv-note">{t.archNote}</p>
        </div>
      </section>

      <section class="vv-token" aria-labelledby="vv-token-h">
        <div class="why-shell">
          <p class="vv-k vv-k--ink">{t.tokenK}</p>
          <h2 class="vv-h vv-h--ink" id="vv-token-h">
            {t.tokenH}
          </h2>
          <p class="vv-token__body">{t.tokenBody}</p>
          <p class="vv-token__tag">{t.tokenTag}</p>
        </div>
      </section>

      <section class="vv-section" aria-labelledby="vv-caps-h">
        <div class="why-shell">
          <p class="vv-k">{t.capsK}</p>
          <h2 class="vv-h" id="vv-caps-h">
            {t.capsH}
          </h2>
          <div class="vv-caps">
            {t.caps.map((cap) => (
              <article class="vv-cap" key={cap.label}>
                <div class="vv-cap__k">{cap.label}</div>
                <h3>{cap.head}</h3>
                <p>{cap.body}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section class="vv-section" aria-labelledby="vv-how-h">
        <div class="why-shell">
          <p class="vv-k">{t.howK}</p>
          <h2 class="vv-h" id="vv-how-h">
            {t.howH}
          </h2>
          <ol class="vv-steps">
            {t.steps.map((step) => (
              <li key={step.cmd}>
                <div class="why-cmd vv-cmd">
                  <span class="why-cmd__prompt">{COMMANDS.bashPrompt}</span>
                  <code>{step.cmd}</code>
                </div>
                <p>{step.body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>

      <section class="vv-section" aria-labelledby="vv-fit-h">
        <div class="why-shell">
          <p class="vv-k">{t.fitK}</p>
          <h2 class="vv-h" id="vv-fit-h">
            {t.fitH}
          </h2>
          <div class="vv-fit">
            <div class="vv-fit__col">
              <h3>{t.fitForHead}</h3>
              <ul>
                {t.fitFor.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </div>
            <div class="vv-fit__col vv-fit__col--not">
              <h3>{t.fitNotHead}</h3>
              <ul>
                {t.fitNot.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </div>
          </div>
          <div class="vv-status">
            <b>{t.statusHead}</b>
            <p>{t.statusBody}</p>
          </div>
        </div>
      </section>

      <section class="vv-section" id="vv-start" aria-labelledby="vv-cta-h">
        <div class="why-shell">
          <p class="vv-k">{t.ctaK}</p>
          <h2 class="vv-h" id="vv-cta-h">
            {t.ctaH}
          </h2>
          <div class="vv-cta-grid">
            <div class="vv-cta-install">
              <div class="why-install">
                <div class="why-install__label">{t.ctaBashLabel}</div>
                <div class="why-cmd">
                  <span class="why-cmd__prompt">{COMMANDS.bashPrompt}</span>
                  <code>{COMMANDS.bash}</code>
                </div>
              </div>
              <div class="why-install">
                <div class="why-install__label">{t.ctaPsLabel}</div>
                <div class="why-cmd">
                  <span class="why-cmd__prompt">
                    {COMMANDS.powerShellPrompt}
                  </span>
                  <code>{COMMANDS.powerShell}</code>
                </div>
              </div>
              <div class="why-install">
                <div class="why-install__label">{t.ctaThen}</div>
                <div class="why-cmd">
                  <span class="why-cmd__prompt">{COMMANDS.bashPrompt}</span>
                  <code>{COMMANDS.then}</code>
                </div>
              </div>
              <div class="vv-cta-links">
                <a
                  class="why-btn why-btn--primary"
                  href={GITHUB_URL}
                  rel="noopener"
                >
                  {t.ctaGithub}
                </a>
                <a
                  class="why-btn why-btn--ghost"
                  href={GITVERSE_URL}
                  rel="noopener"
                >
                  {t.ctaGitverse}
                </a>
              </div>
            </div>
            <a class="vv-zap-tie" href={zapHref}>
              <span class="vv-zap-tie__orb" aria-hidden="true">
                <svg viewBox="0 0 64 64" fill="none" aria-hidden="true">
                  <circle
                    cx="32"
                    cy="32"
                    r="26"
                    stroke="currentColor"
                    stroke-width="1"
                    stroke-dasharray="1.5 7"
                    stroke-linecap="round"
                  />
                  <circle cx="32" cy="32" r="5" fill="currentColor" />
                  <circle cx="49" cy="22" r="3.5" fill="currentColor" />
                </svg>
              </span>
              <b>{t.zapTieHead}</b>
              <p>{t.zapTieBody}</p>
              <span class="vv-zap-tie__link">{t.zapTieLink} →</span>
            </a>
          </div>
        </div>
      </section>

      <section
        class="vv-section vv-summary-wrap"
        aria-labelledby="vv-summary-k"
      >
        <div class="why-shell">
          <p class="vv-k" id="vv-summary-k">
            {t.summaryK}
          </p>
          <blockquote
            class="vv-summary"
            dangerouslySetInnerHTML={t.summaryHtml}
          />
        </div>
      </section>
    </div>
  );
});
