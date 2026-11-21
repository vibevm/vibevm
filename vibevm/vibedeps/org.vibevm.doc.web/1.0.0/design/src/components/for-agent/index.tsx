/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-FOR-AGENT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type AgentLink = {
  readonly label: string;
  readonly href: string;
};

export type ForAgentProps = {
  readonly title: string;
  /** One sentence saying what the citation is and why it carries a version. */
  readonly lead: string;
  /** The page's `spec://` citation; the block the reader is at is appended. */
  readonly uri: string;
  readonly links: ReadonlyArray<AgentLink>;
  readonly copyLabel: string;
  /** The panel form, for the floating button; the section form otherwise. */
  readonly compact: boolean;
};

/**
 * What a reader hands to an agent instead of a screenshot: the page's
 * `spec://` address, and the machine projections that lie beside it.
 *
 * The address shown is the page's, and the behaviour appends the block
 * the reader is standing on — `#p12` — so a citation names a place and
 * not a document. It carries the VERSION and never `latest`: an agent
 * handed `latest` would quote a page that moves under it between the
 * question and the answer (R-26).
 *
 * The same component is the section in the page and the panel inside the
 * floating button, because they say the same thing and a second copy
 * would eventually say it differently. It is not a chat, and there is no
 * place in it for one (D-21).
 */
export const ForAgent = component$<ForAgentProps>((props) => {
  useStyles$(styles);
  return (
    <section
      class={props.compact ? "for-agent for-agent--compact" : "for-agent"}
      aria-label={props.title}
      data-for-agent
    >
      {props.compact ? null : (
        <>
          <h2 class="for-agent__title">{props.title}</h2>
          <p class="for-agent__lead">{props.lead}</p>
        </>
      )}
      <p class="for-agent__uri">
        <code data-agent-uri>{props.uri}</code>
      </p>
      <p class="for-agent__actions">
        <button class="for-agent__button" type="button" data-agent-copy>
          {props.copyLabel}
        </button>
        {props.links.map((link) => (
          <a key={link.href} class="for-agent__button" href={link.href}>
            {link.label}
          </a>
        ))}
      </p>
    </section>
  );
});
