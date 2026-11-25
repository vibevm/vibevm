/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$, useStyles$ } from "@qwik.dev/core";

import type { Locale } from "../landing/i18n.ts";
import { STRINGS, shownAddress } from "./i18n.ts";
import styles from "./styles.css?inline";

export type NewsAndSupportProps = {
  readonly locale: Locale;
};

/**
 * `/news-and-support/` — the five places the project is spoken in.
 *
 * The composition is a page of destinations rather than an argument: a
 * heading, one line saying what the three groups are for, and then the
 * groups — news, support, everything else — in the owner's order. It
 * stands in the landing's column and not in the Why pages' full bleed,
 * because there is nothing here to bleed: a card is as wide as a reading
 * measure allows and five of them are a list, not an essay.
 *
 * Each card is one link and the whole card is it. A card with a title
 * link inside it gives a reader two targets of different sizes for one
 * destination — the small one being the only one that works — and gives a
 * keyboard reader either two stops or a stop that is not where the ring
 * is drawn. So the `<a>` is the card: one target, one stop, one ring, and
 * the ring follows the card's own corner rather than the global 3px.
 *
 * And the card says where it goes. The name and the platform answer «what
 * is this», the line under them answers «what is there», and the address
 * at the foot answers «where does this take me» — in the form a reader
 * could read out loud, derived from the `href` itself so the two cannot
 * drift (`shownAddress`).
 *
 * Nothing hydrates: five links and three headings are HTML, and the only
 * motion is the hover the rest of the family already has.
 */
export const NewsAndSupport = component$<NewsAndSupportProps>((props) => {
  useStyles$(styles);
  const t = STRINGS[props.locale];

  return (
    <div class="ns-page">
      <h1 class="ns-title">{t.title}</h1>
      <p class="ns-lede">{t.lede}</p>

      {t.groups.map((group) => (
        <section
          key={group.id}
          class="ns-group"
          aria-labelledby={`ns-${group.id}-h`}
        >
          <h2 class="ns-h" id={`ns-${group.id}-h`}>
            {group.head}
          </h2>
          <ul class="ns-cards">
            {group.channels.map((channel) => (
              <li key={channel.href}>
                {/* `rel="noopener"` on a link that opens in this tab is
                    not a fix for a vulnerability — there is no second
                    window to reach back through — it is the site's one
                    spelling for «this leads off the domain», which the
                    header's two source mirrors already carry. One idiom
                    per operation, so a reader of this tree can find every
                    outbound link by searching for it. */}
                <a class="ns-card" href={channel.href} rel="noopener">
                  <span class="ns-card__platform">{channel.platform}</span>
                  <span class="ns-card__name">{channel.name}</span>
                  <span class="ns-card__body">{channel.body}</span>
                  <span class="ns-card__at">{shownAddress(channel.href)}</span>
                </a>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
});
