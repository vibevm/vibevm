/** @scope spec://org.vibevm.core/vibevm/common/PROP-023#AUTHORSHIP-SEPARATION */

import { component$, useStyles$, type JSXOutput } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** Somebody named in a signature, and the site they keep. */
export type BridgePerson = {
  readonly name: string;
  readonly href: string;
};

/** The two authorships a bridge holds, as a page is handed them. */
export type BridgeAuthorship = {
  /**
   * Who wrote the bridge: the authors of the wrapper, its metadata and
   * its adapters, and of nothing upstream. Empty is a fact about a
   * published bridge and is shown as one.
   */
  readonly maintainers: ReadonlyArray<string>;
  /** Who wrote the bytes the bridge points at, each named once. */
  readonly upstreamAuthors: ReadonlyArray<string>;
  /** The licence of those bytes, when every source states the same one. */
  readonly upstreamLicense?: string;
  /**
   * Which of the names above lead to the person, and where.
   *
   * It arrives with the data because WHO the site is prepared to link is
   * the site's own statement and never this component's: a name in either
   * list is a package's word for somebody, and a link the site invented
   * for one of them would be an address the package never gave. Absent is
   * the ordinary case — every name is then text, which is what a name in
   * a manifest is.
   */
  readonly people?: ReadonlyArray<BridgePerson>;
};

export type BridgeSignaturesProps = {
  readonly bridge: BridgeAuthorship;
};

/** What a list of names reads as when there is nobody in it. */
const NOBODY = "—";

/**
 * The two signatures of a bridge, kept apart by construction.
 *
 * A bridge is a wrapper one maintainer publishes around somebody else's
 * repository, so a page about it has two names to show and must never
 * merge them into one line (PROP-023 `##AUTHORSHIP-SEPARATION`). The
 * failure this component exists to make impossible is a small one to
 * commit and a serious one to publish: the maintainer of a wrapper
 * printed as the author of the work it wraps.
 *
 * Which is why the two rows are drawn by one component used in both
 * places a bridge appears — a card on a shelf and the head of its own
 * page — rather than written out twice. Two copies would be two chances
 * to label them differently, and a reader comparing the shelf against
 * the page would have to work out whether «maintainer» there and
 * «author» here were the same claim.
 *
 * **An empty list is shown as empty.** A bridge that names no
 * maintainer prints a dash, not the upstream names and not nothing at
 * all: a row that vanished would read as «this is not a bridge», which
 * is the one thing it is not. The licence is the exception and is shown
 * only when it came — the pipeline carries it when every embedded source
 * states the same one, and absent means they disagree or none said.
 *
 * The publisher stays where it is, above this. It is the GROUP that
 * published the package and is a third fact again: who to complain to,
 * as opposed to who wrote either half.
 */
export const BridgeSignatures = component$<BridgeSignaturesProps>((props) => {
  useStyles$(styles);
  const bridge = props.bridge;
  return (
    <dl class="bridge-signatures">
      <div class="bridge-signatures__row">
        <dt>Bridge maintainer</dt>
        <dd>{signatures(bridge.maintainers, bridge.people)}</dd>
      </div>
      <div class="bridge-signatures__row">
        <dt>Destination author</dt>
        <dd>{signatures(bridge.upstreamAuthors, bridge.people)}</dd>
      </div>
      {bridge.upstreamLicense === undefined ? null : (
        <div class="bridge-signatures__row">
          <dt>Upstream licence</dt>
          <dd>
            <code>{bridge.upstreamLicense}</code>
          </dd>
        </div>
      )}
    </dl>
  );
});

/**
 * One list of names, with the people the site knows leading to them.
 *
 * The comma between two names is drawn by the stylesheet rather than
 * written here: each name stands in an element of its own so that one of
 * them can be a link and the language switch can move the one it knows,
 * and a separator inside a joined string would have made both impossible.
 *
 * An empty list is still the dash it was — a bridge that names no
 * maintainer says so (`##AUTHORSHIP-SEPARATION`), and a row that fell
 * silent would read as «this is not a bridge».
 */
function signatures(
  names: ReadonlyArray<string>,
  people: ReadonlyArray<BridgePerson> | undefined,
): JSXOutput {
  if (names.length === 0) return NOBODY;
  const known = people ?? [];
  return names.map((name) => {
    const person = known.find((one) => one.name === name.trim());
    return (
      <span key={name} class="bridge-signatures__name">
        {person === undefined ? (
          name
        ) : (
          <a class="bridge-signatures__person" href={person.href}>
            {name}
          </a>
        )}
      </span>
    );
  });
}
