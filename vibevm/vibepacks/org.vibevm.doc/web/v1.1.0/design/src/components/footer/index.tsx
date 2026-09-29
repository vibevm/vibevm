/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useStyles$, type JSXOutput } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** Somebody named in the signature, and the site they keep. */
export type FooterPerson = {
  readonly name: string;
  readonly href: string;
};

export type FooterProps = {
  /** The copyright line, already composed by the caller in its language. */
  readonly copyright: string;
  /**
   * The people named in that line whose name leads to them. Which names
   * those are is the site's own statement and arrives from the site: this
   * component knows that a signature may hold a person, never who.
   */
  readonly people: ReadonlyArray<FooterPerson>;
};

/**
 * The catalogue footer, shared by the landing and the documentation —
 * one footer for one site, which is the whole reason both live in one
 * application.
 *
 * It sinks below the page's ground rather than floating on it: the
 * footer is the end of the page, and the eye should be able to tell that
 * without reading anything.
 */
export const Footer = component$<FooterProps>((props) => {
  useStyles$(styles);
  return (
    <footer class="footer">
      <div class="footer__inner">
        <div class="footer__columns">
          <Slot />
        </div>
        <p class="footer__copyright">{signed(props.copyright, props.people)}</p>
      </div>
    </footer>
  );
});

/**
 * The signature, with the person in it as a link to themselves.
 *
 * The line arrives as one string because it is one sentence in two
 * languages, and the NAME inside it is what a reader clicks: «© 2026 »
 * is a year and a symbol and leads nowhere. So the string is cut at the
 * name rather than assembled from pieces — the piece before it and the
 * piece after it stay text nodes of their own, which is also what lets
 * the site's language switch move the name and leave the year alone.
 *
 * The first name that is actually in the line wins, and a line with
 * nobody the site knows in it is returned untouched.
 */
function signed(line: string, people: ReadonlyArray<FooterPerson>): JSXOutput {
  for (const person of people) {
    const at = line.indexOf(person.name);
    if (at === -1) continue;
    return (
      <>
        {line.slice(0, at)}
        <a class="footer__person" href={person.href}>
          {person.name}
        </a>
        {line.slice(at + person.name.length)}
      </>
    );
  }
  return line;
}
