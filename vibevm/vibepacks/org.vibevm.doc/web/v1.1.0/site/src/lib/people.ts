/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The people this site names, and the one address each of them keeps.
 *
 * A name is not decoration: wherever the site prints somebody's, a
 * reader who wants to know who that is has one question, and the answer
 * is the person's own site. So the site holds a table of the people it
 * knows and links every mention of them — the footer's signature, a
 * bridge's maintainer on the catalogue, an author in the structured data
 * — rather than each surface deciding for itself.
 *
 * Both spellings of a name are rows of their own, because the site is
 * read in two languages and the Russian pages sign themselves «Олег
 * Чирухин». They lead to the same place: a person has one site, however
 * their name is written.
 *
 * A name the table does not know stays text. Most names on the catalogue
 * are a package's own statement about who wrote it, and inventing an
 * address for one of them would be the site answering a question the
 * package did not ask — a link is a claim, and this file is the whole of
 * what the site is prepared to claim.
 *
 * Nothing here belongs in a code sample or a data example. A name inside
 * a JSON block in the manual is a VALUE a reader may copy, and a link in
 * it would be a link in somebody's configuration file.
 */

/** The creator of VibeVM: the site he publishes, and the name on it. */
export const CREATOR_SITE = "https://oleg.guru";

/** Every name the site links, in both spellings, to one address each. */
export const LINKED_PEOPLE: ReadonlyArray<LinkedPerson> = [
  { name: "Oleg Chirukhin", href: CREATOR_SITE },
  { name: "Олег Чирухин", href: CREATOR_SITE },
];

/** One person the site links: what they are called, and where they are. */
export type LinkedPerson = {
  readonly name: string;
  readonly href: string;
};

/**
 * The address of a person the site knows, or `null` for everybody else.
 *
 * `null` rather than the name back, because the caller has two different
 * things to render and the difference is the whole point: a link for a
 * name the site vouches for, plain text for one it does not.
 */
export function siteOfPerson(name: string): string | null {
  const found = LINKED_PEOPLE.find((person) => person.name === name.trim());
  return found === undefined ? null : found.href;
}
