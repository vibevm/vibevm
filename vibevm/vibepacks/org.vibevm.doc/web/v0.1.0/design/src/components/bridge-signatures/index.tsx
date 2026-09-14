/** @scope spec://org.vibevm.core/vibevm/common/PROP-023#AUTHORSHIP-SEPARATION */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

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
        <dd>
          {bridge.maintainers.length === 0
            ? NOBODY
            : bridge.maintainers.join(", ")}
        </dd>
      </div>
      <div class="bridge-signatures__row">
        <dt>Destination author</dt>
        <dd>
          {bridge.upstreamAuthors.length === 0
            ? NOBODY
            : bridge.upstreamAuthors.join(", ")}
        </dd>
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
