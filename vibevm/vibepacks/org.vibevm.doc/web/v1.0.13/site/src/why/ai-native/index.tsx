/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import { href } from "../../lib/href.ts";
import {
  GITHUB_URL,
  GITVERSE_URL,
  type Locale,
  localePath,
} from "../../landing/i18n.ts";
import { visionHref } from "../../vision/paths.ts";
import shared from "../shared.css?inline";
import { whyHref } from "../paths.ts";
import { Envelope, Projection } from "./art.tsx";
import {
  BASELINE_JSON,
  CODE_AFTER,
  CODE_BEFORE,
  type CodeLine,
  DEVIATION_CODE,
  FLOORS,
  ORACLE_CMD,
  PROMPT,
  TAG_ROWS,
} from "./artifacts.ts";
import { STRINGS } from "./i18n.ts";
import styles from "./styles.css?inline";

export type WhyAiNativeProps = {
  readonly locale: Locale;
};

/**
 * A code sample, printed line by line.
 *
 * The lines the discipline added carry the page's accent, and they carry
 * it as an ELEMENT rather than as markup written into the document from
 * a string. The source escaped each line by hand and wrapped some of
 * them in a `<span>` before handing the result to `set:html`; here the
 * distinction is a flag on a line, and the escaping is the framework's
 * job rather than the page's.
 */
const Code = component$<{ lines: readonly CodeLine[] }>((props) => (
  <pre class="an-code">
    <code>
      {props.lines.flatMap((line, index) => {
        /* A plain line is text and nothing else — no wrapper around it.
           That is not tidiness: the element around a line would split
           the sample into one text node per line, and the parity gate
           compares what a page shows as a set of text fragments. The
           reference puts the newlines BETWEEN the marked spans rather
           than inside them, so this does too. */
        const node =
          line.marked === true ? (
            <span key={`marked-${index}`} class="an-code__hl">
              {line.text}
            </span>
          ) : (
            line.text
          );
        return index === props.lines.length - 1 ? [node] : [node, "\n"];
      })}
    </code>
  </pre>
));

/**
 * `/why/ai-native` — the page for the AI-Native Code Discipline over
 * Rust, TypeScript and Go.
 *
 * The composition is the source's, section for section: a hero over a
 * full-width blueprint, the problem in a split column, an inverted cream
 * band carrying the central law beside the envelope drawing, the
 * architecture as core → stacks → your repository, three per-language
 * floor cards, five numbered mechanisms with their artifacts, the family
 * of three projections, the dogfood list, a three-column status panel,
 * the start list with the two family ties, and a closing paragraph.
 *
 * The address is `/why/ai-native` and the copy says «AI-Native
 * Language». Both are deliberate: the owner settled the address, and the
 * text is the owner's.
 *
 * Nothing hydrates. The blueprint draws its floor and its three rays,
 * the stations pop and their brackets clamp — all of it CSS over markup
 * the server already wrote, and all of it still for a reader who asked
 * for stillness.
 */
export const WhyAiNative = component$<WhyAiNativeProps>((props) => {
  useStyles$(shared);
  useStyles$(styles);
  const t = STRINGS[props.locale];
  const vibevmHref = whyHref("vibevm", props.locale);
  const zapHref = whyHref("zap", props.locale);
  const homeHref = href(localePath(props.locale));

  return (
    <div class="why-page why-an">
      <section class="an-hero" aria-labelledby="an-headline">
        <div class="why-shell">
          <p class="why-eyebrow">
            <span class="why-dot" aria-hidden="true" />
            {t.eyebrow}
          </p>
          <h1
            class="why-headline an-headline"
            id="an-headline"
            dangerouslySetInnerHTML={t.headlineHtml}
          />
          <p class="why-lead an-lead">{t.lead}</p>
          <div class="why-cta">
            <a class="why-btn why-btn--primary" href="#an-start">
              {t.ctaTry}
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
            <a class="why-btn why-btn--ghost" href={vibevmHref}>
              {t.ctaVibevm}
            </a>
            <p class="why-release">
              <span class="why-release-dot" aria-hidden="true" />
              {t.badge}
            </p>
          </div>
          {/* The doorway into the essay this page is one argument of.
              A reading entry, not a call to action: no button, no
              install — a kicker, a title and one line of what waits
              behind it. It is a plain block on purpose: the layout
              gates scan flex and grid containers, and a doorway is not
              a composition. */}
          <a class="an-vision-entry" href={visionHref(props.locale)}>
            <span class="an-vision-entry__k">{t.visionK}</span>
            <span class="an-vision-entry__head">{t.visionHead} →</span>
            <span class="an-vision-entry__sub">{t.visionSub}</span>
          </a>
        </div>
        <figure class="an-blueprint" role="img" aria-label={t.heroAlt}>
          <div class="an-blueprint__canvas">
            <Projection />
          </div>
          <figcaption class="why-shell" aria-hidden="true">
            {t.heroCaption}
          </figcaption>
        </figure>
      </section>

      <section class="an-section" aria-labelledby="an-problem-h">
        <div class="why-shell an-split">
          <div>
            <p class="an-k">{t.problemK}</p>
            <h2 class="an-h" id="an-problem-h">
              {t.problemH}
            </h2>
          </div>
          <div>
            <p class="an-body">{t.problemBody}</p>
            <ul class="an-symptoms">
              {t.problemPoints.map((point) => (
                <li key={point}>{point}</li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      <section class="an-law" aria-labelledby="an-law-h">
        <div class="why-shell an-law__grid">
          <div>
            <p class="an-k an-k--ink">{t.lawK}</p>
            <blockquote
              class="an-law__quote"
              id="an-law-h"
              dangerouslySetInnerHTML={t.lawQuoteHtml}
            />
            <p class="an-law__body">{t.lawBody}</p>
            <p class="an-law__note">{t.lawNote}</p>
          </div>
          <figure class="an-envelope-fig" role="img" aria-label={t.envelopeAlt}>
            <Envelope />
            <figcaption aria-hidden="true">{t.envelopeCaption}</figcaption>
          </figure>
        </div>
        <div class="why-shell">
          <div class="an-law__contract">
            <b>{t.contractHead}</b>
            <p dangerouslySetInnerHTML={t.contractBodyHtml} />
          </div>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-arch-h">
        <div class="why-shell">
          <p class="an-k">{t.archK}</p>
          <h2 class="an-h" id="an-arch-h">
            {t.archH}
          </h2>
          <p class="an-body an-body--wide">{t.archBody}</p>

          <div class="an-arch" role="img" aria-label={t.archAlt}>
            <div class="an-arch__node an-arch__node--core">
              <span class="an-tag an-tag--gold">{t.archCoreTag}</span>
              <b>{t.archCoreHead}</b>
              <span class="an-arch__sub">{t.archCoreBody}</span>
            </div>
            <div class="an-arch__join" aria-hidden="true">
              <i />
            </div>
            <div class="an-arch__stacks">
              {t.archStacks.map((stack) => (
                <div
                  class={`an-arch__node an-arch__node--stack an-b-${stack.mark}`}
                  key={stack.head}
                >
                  <span
                    class={`an-mark an-mark--${stack.mark}`}
                    aria-hidden="true"
                  />
                  <b>{stack.head}</b>
                  <span class="an-arch__sub">{stack.body}</span>
                </div>
              ))}
            </div>
            <div class="an-arch__join an-arch__join--arrow" aria-hidden="true">
              <i />
            </div>
            <div class="an-arch__node an-arch__node--project">
              <span class="an-tag an-tag--gold">{t.archProjTag}</span>
              <b>{t.archProjHead}</b>
              <span class="an-arch__sub">{t.archProjBody}</span>
            </div>
          </div>
          <p class="an-note">{t.archNote}</p>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-compare-h">
        <div class="why-shell">
          <p class="an-k">{t.compareK}</p>
          <h2 class="an-h" id="an-compare-h">
            {t.compareH}
          </h2>
          <p class="an-body an-body--wide">{t.compareBody}</p>
          <div class="an-compare">
            {FLOORS.map((floor) => (
              <article class={`an-lang-card an-b-${floor.key}`} key={floor.key}>
                <h3>
                  <span
                    class={`an-mark an-mark--${floor.key}`}
                    aria-hidden="true"
                  />
                  {floor.name}
                </h3>
                <p class="an-floor-label">{t.compareYours}</p>
                <ol class="an-floor-chain">
                  {floor.yours.map((one) => (
                    <li key={one}>{one}</li>
                  ))}
                  {floor.added.map((one) => (
                    <li class="an-floor-chain__add" key={`add-${one}`}>
                      {one}
                    </li>
                  ))}
                </ol>
                <p class="an-floor-label an-floor-label--add">
                  {t.compareAdded}
                </p>
                <ul class="an-adds">
                  {t.compareAdds[floor.key].map((one) => (
                    <li key={one}>{one}</li>
                  ))}
                </ul>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-mech-h">
        <div class="why-shell">
          <p class="an-k">{t.mechK}</p>
          <h2 class="an-h" id="an-mech-h">
            {t.mechH}
          </h2>

          <ol class="an-mechs">
            <li>
              <h3>{t.m1H}</h3>
              <p>{t.m1Body}</p>
              <div class="an-beforeafter">
                <figure class="an-codefig">
                  <figcaption>{t.m1BeforeLabel}</figcaption>
                  <Code lines={CODE_BEFORE} />
                </figure>
                <figure class="an-codefig an-codefig--after">
                  <figcaption>{t.m1AfterLabel}</figcaption>
                  <Code lines={CODE_AFTER} />
                </figure>
              </div>
              <p class="an-artifact-note">{t.m1Caption}</p>
              <div class="an-tagrows">
                <p class="an-floor-label">{t.m1TagsLabel}</p>
                {TAG_ROWS.map((row) => (
                  <div class="an-tagrow" key={row.mark}>
                    <span
                      class={`an-mark an-mark--${row.mark}`}
                      aria-hidden="true"
                    />
                    <code>{row.code}</code>
                  </div>
                ))}
              </div>
            </li>
            <li>
              <h3>{t.m2H}</h3>
              <p>{t.m2Body}</p>
            </li>
            <li>
              <h3>{t.m3H}</h3>
              <p>{t.m3Body}</p>
              <figure class="an-codefig an-codefig--baseline">
                <pre class="an-code">
                  <code>{BASELINE_JSON}</code>
                </pre>
                <figcaption class="an-artifact-note">{t.m3Caption}</figcaption>
              </figure>
            </li>
            <li>
              <h3>{t.m4H}</h3>
              <p>{t.m4Body}</p>
              <figure class="an-codefig">
                <pre class="an-code">
                  <code>{DEVIATION_CODE}</code>
                </pre>
                <figcaption class="an-artifact-note">{t.m4Caption}</figcaption>
              </figure>
            </li>
            <li>
              <h3>{t.m5H}</h3>
              <p>{t.m5Body}</p>
              <figure class="an-codefig">
                <pre class="an-code">
                  <code>{ORACLE_CMD}</code>
                </pre>
                <figcaption class="an-artifact-note">{t.m5Caption}</figcaption>
              </figure>
            </li>
          </ol>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-family-h">
        <div class="why-shell">
          <p class="an-k">{t.familyK}</p>
          <h2 class="an-h" id="an-family-h">
            {t.familyH}
          </h2>
          <p class="an-body an-body--wide">{t.familyBody}</p>
          <div class="an-family">
            {t.family.map((one) => (
              <article
                class={`an-lang-card an-family__card an-b-${one.mark}`}
                key={one.name}
              >
                <p class="an-family__tag">{one.tag}</p>
                <h3>
                  <span
                    class={`an-mark an-mark--${one.mark}`}
                    aria-hidden="true"
                  />
                  {one.name}
                </h3>
                <p>{one.body}</p>
              </article>
            ))}
          </div>
          <p class="an-note">{t.familyNote}</p>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-dog-h">
        <div class="why-shell">
          <p class="an-k">{t.dogK}</p>
          <h2 class="an-h" id="an-dog-h">
            {t.dogH}
          </h2>
          <p class="an-body an-body--wide">{t.dogBody}</p>
          <ul class="an-doglist">
            {t.dogPoints.map((point) => (
              <li key={point}>{point}</li>
            ))}
          </ul>
        </div>
      </section>

      <section class="an-section" aria-labelledby="an-status-h">
        <div class="why-shell">
          <p class="an-k">{t.statusK}</p>
          <h2 class="an-h" id="an-status-h">
            {t.statusH}
          </h2>
          <div class="an-status-grid">
            {t.statusCols.map((column, index) => (
              <div
                class={`an-status-col an-status-col--${index}`}
                key={column.head}
              >
                <h3>{column.head}</h3>
                <ul>
                  {column.items.map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <p class="an-status-tail">{t.statusTail}</p>
        </div>
      </section>

      <section class="an-section" id="an-start" aria-labelledby="an-start-h">
        <div class="why-shell">
          <p class="an-k">{t.startK}</p>
          <h2 class="an-h" id="an-start-h">
            {t.startH}
          </h2>
          <ol class="an-steps">
            {t.steps.map((step) => (
              <li key={step.kind === "cmd" ? step.cmd : step.head}>
                {step.kind === "cmd" ? (
                  <div class="why-cmd an-cmd">
                    <span class="why-cmd__prompt">{PROMPT}</span>
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
                      <a class="an-link" href={homeHref}>
                        {step.linkText} →
                      </a>
                    </>
                  )}
                </p>
              </li>
            ))}
          </ol>
          <div class="an-cta-links">
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

          <div class="an-ties">
            <a class="an-tie an-tie--vibevm" href={vibevmHref}>
              <b>{t.tieVibevmHead}</b>
              <p>{t.tieVibevmBody}</p>
              <span class="an-tie__link">{t.tieVibevmLink} →</span>
            </a>
            <a class="an-tie an-tie--zap" href={zapHref}>
              <b>{t.tieZapHead}</b>
              <p>{t.tieZapBody}</p>
              <span class="an-tie__link">{t.tieZapLink} →</span>
            </a>
          </div>
        </div>
      </section>

      <section
        class="an-section an-summary-wrap"
        aria-labelledby="an-summary-k"
      >
        <div class="why-shell">
          <p class="an-k" id="an-summary-k">
            {t.summaryK}
          </p>
          <blockquote
            class="an-summary"
            dangerouslySetInnerHTML={t.summaryHtml}
          />
        </div>
      </section>
    </div>
  );
});
