/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** A link the meta row offers beside the page. */
export type MetaLink = {
  readonly label: string;
  readonly href: string;
};

export type PageMetaProps = {
  /** The group that published the edition being read. */
  readonly publisher: string;
  /** The package version this page belongs to. */
  readonly version: string;
  /** Whether that version is the newest this build carries. */
  readonly latest: boolean;
  /** When this build rendered the page — the first of the two dates. */
  readonly renderedAt: string;
  /** When a human last read the page aloud — the second, when it exists. */
  readonly reviewedAt?: string;
  /** For an adaptation: the coordinate of the documentation it adapts. */
  readonly adapts?: string;
  /** Who the page is written for, from the markup and never declared. */
  readonly audiences: ReadonlyArray<string>;
  /** Minutes to read, rounded up, never below one. */
  readonly readingMinutes: number;
  /** The machine projections and the agent surface, beside the page. */
  readonly links: ReadonlyArray<MetaLink>;
};

/** A date is shown as the day it names, and never as a time of day. */
function day(stamp: string): string {
  return stamp.slice(0, 10);
}

/**
 * The row under a page's title: whose it is, which version, when it was
 * rendered, when a human last read it aloud, for whom it is written, how
 * long it takes, and where its machine projections are.
 *
 * **Exactly two dates**, and this is the component that has to hold that
 * line. A reader may see when the build rendered the page and when a
 * human last read it aloud — and nothing else, because every other date
 * a documentation site normally shows («updated», «last changed», «3
 * versions behind») is a claim about history, and the product keeps no
 * history to make it out of (`##OBS-NOTHING-LEAKS`, D-27). The package's
 * publication date exists in the manifest and belongs to the sitemap's
 * `lastmod`, not to this row.
 *
 * The version is printed even when it is the newest one, with the word
 * `latest` beside it rather than instead of it: an address carrying a
 * number always shows the current content of that number, so the number
 * is the honest thing to quote and `latest` is only a fact about today.
 */
export const PageMeta = component$<PageMetaProps>((props) => {
  useStyles$(styles);
  return (
    <div class="page-meta">
      <dl class="page-meta__facts">
        <div class="page-meta__fact">
          <dt>Publisher</dt>
          <dd>{props.publisher}</dd>
        </div>
        <div class="page-meta__fact">
          <dt>Version</dt>
          <dd>
            <code>{props.version}</code>
            {props.latest ? <span class="page-meta__flag">latest</span> : null}
          </dd>
        </div>
        {props.adapts === undefined ? null : (
          <div class="page-meta__fact">
            <dt>Adapts</dt>
            <dd>
              <code>{props.adapts}</code>
            </dd>
          </div>
        )}
        <div class="page-meta__fact">
          <dt>Audiences</dt>
          <dd>{props.audiences.join(", ")}</dd>
        </div>
        <div class="page-meta__fact">
          <dt>Reading time</dt>
          <dd>{props.readingMinutes} min</dd>
        </div>
        <div class="page-meta__fact">
          <dt>Rendered</dt>
          <dd>
            <time dateTime={props.renderedAt}>{day(props.renderedAt)}</time>
          </dd>
        </div>
        <div class="page-meta__fact">
          <dt>Read aloud</dt>
          <dd>
            {props.reviewedAt === undefined ? (
              <span class="page-meta__never">never</span>
            ) : (
              <time dateTime={props.reviewedAt}>{day(props.reviewedAt)}</time>
            )}
          </dd>
        </div>
      </dl>
      <p class="page-meta__links">
        {props.links.map((link) => (
          <a key={link.href} class="page-meta__link" href={link.href}>
            {link.label}
          </a>
        ))}
      </p>
    </div>
  );
});
