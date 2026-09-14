/** @scope spec://org.vibevm.core/vibevm/common/PROP-023#AUTHORSHIP-SEPARATION */

/**
 * A bridge's two authorships, as a page is handed them.
 *
 * It is four lines and a file of its own, because the four lines are the
 * one place the two lists could be confused with each other and the
 * whole point of the field is that they never are (PROP-023
 * `##AUTHORSHIP-SEPARATION`). A card and a package's head both need the
 * answer, so it is arrived at once rather than beside each of them.
 *
 * Nothing is computed here and nothing is guessed. The pipeline already
 * worked out who wrote the wrapper — the package's own authors — and who
 * wrote the bytes it points at — the upstream authors of every embedded
 * source, each named once — and carried both across, exactly so that the
 * shell would not have to derive them from a list of names it cannot
 * tell apart (`##PIPE-SHELL-PARSES-NOTHING`). What is left is the
 * spelling: the wire writes `upstream_authors` and the components read
 * `upstreamAuthors`, and this is the line where that changes.
 *
 * An absent `bridge` is «this package is not a bridge» and is answered
 * with nothing at all. Emptiness INSIDE it is a different answer and is
 * carried whole: a bridge that names no maintainer is a published fact,
 * and the one thing that must never happen is either list being filled
 * from the other.
 */

import type { DocPackage } from "../generated/doc-manifest.ts";

import type { BridgeAuthorship } from "@vibe-docs/design";

export function bridgeOf(card: DocPackage): BridgeAuthorship | undefined {
  const bridge = card.bridge;
  if (bridge === undefined) return undefined;
  return {
    maintainers: bridge.maintainers,
    upstreamAuthors: bridge.upstream_authors,
    ...(bridge.upstream_license === undefined
      ? {}
      : { upstreamLicense: bridge.upstream_license }),
  };
}
