/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PLACEHOLDERS-GENERATED */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import {
  BridgeSignatures,
  type BridgeAuthorship,
} from "../bridge-signatures/index.tsx";
import {
  KindGlyph,
  kindLabel,
  type PackageKind,
} from "../kind-glyph/index.tsx";
import styles from "./styles.css?inline";

export type PackageHeaderProps = {
  /** The display title from the card. */
  readonly title: string;
  /** The group that published it, printed beside the title. */
  readonly publisher: string;
  /** The coordinate, small: identity, as opposed to the name. */
  readonly coordinate: string;
  /** The one-line subtitle, when the package declares one. */
  readonly description?: string;
  /** The banner's address when the package ships one. */
  readonly banner?: string;
  /** The icon's address when the package ships one. */
  readonly icon?: string;
  /**
   * What kind of package this page is about, when the caller knows it
   * (VIBEVM-SPEC §4.1). The placeholder then carries that kind's mark —
   * the same mark the package wears on every shelf it stands on — and
   * the glyph below is what a page whose kind nobody named falls back
   * to.
   */
  readonly kind?: PackageKind;
  /** One character for the generated placeholder, chosen by package kind. */
  readonly glyph: string;
  /**
   * The two authorships of a bridge, when this package is one (PROP-023
   * `##AUTHORSHIP-SEPARATION`). Absent for everything else: a package
   * that wraps nobody's work has one authorship and needs no line
   * separating it from a second.
   */
  readonly bridge?: BridgeAuthorship;
};

/**
 * The head of a package's page: the banner when there is one, the icon,
 * the title, the publisher.
 *
 * **A package that ships no banner gets no banner.** It used to get a
 * drawn one — a wash of the theme's own tokens at three-to-one — and on
 * a page it read as a wide empty field held open above the name for a
 * picture that does not exist. A card's placeholder is a different
 * thing and stays: it stands in a row of cards, where a gap would break
 * the row and where the glyph tells one kind of package from another
 * (`##CARD-PLACEHOLDERS-GENERATED`). A page has no row to keep, so the
 * band appears with a picture in it or not at all (F-32), and the page
 * begins at the title.
 *
 * Nothing is fetched for the missing one and nothing ever will be: a
 * site that went to someone else's host for a decorative picture would
 * tell that host who is reading the documentation (D-20, R-09).
 *
 * The addresses are still not in the shell. The pipeline derives a
 * package's pictures, the manifest is gaining the fields, and the day
 * they arrive this component already knows what to do with them — the
 * gap is a field in the manifest, not a function in this file.
 *
 * A bridge shows two names under the publisher and an ordinary package
 * shows none. Who maintains a wrapper and who wrote what it wraps are
 * two facts, and the head of a page is exactly where a reader decides
 * whose work they are about to read (PROP-023 `##AUTHORSHIP-
 * SEPARATION`).
 */
export const PackageHeader = component$<PackageHeaderProps>((props) => {
  useStyles$(styles);
  return (
    <header
      class={
        props.banner === undefined
          ? "package-header package-header--unbannered"
          : "package-header"
      }
    >
      {props.banner === undefined ? null : (
        <img class="package-header__banner" src={props.banner} alt="" />
      )}
      <div class="package-header__row">
        {props.icon === undefined ? (
          props.kind === undefined ? (
            <span class="package-header__placeholder" aria-hidden="true">
              {props.glyph}
            </span>
          ) : (
            /* Named for the reason the card's tile is: with a kind in it
               the tile states a fact the page does not state anywhere
               else, and the mark inside stays hidden so it is announced
               once. */
            <span
              class="package-header__placeholder"
              role="img"
              title={kindLabel(props.kind)}
              aria-label={kindLabel(props.kind)}
            >
              <KindGlyph kind={props.kind} />
            </span>
          )
        ) : (
          <img
            class="package-header__icon"
            src={props.icon}
            alt=""
            width="72"
            height="72"
          />
        )}
        <div class="package-header__body">
          <h1 class="package-header__title">{props.title}</h1>
          <p class="package-header__publisher">
            {props.publisher}
            <span class="package-header__coordinate">{props.coordinate}</span>
          </p>
          {props.bridge === undefined ? null : (
            <BridgeSignatures bridge={props.bridge} />
          )}
          {props.description === undefined ? null : (
            <p class="package-header__description">{props.description}</p>
          )}
          <div class="package-header__actions">
            <Slot />
          </div>
        </div>
      </div>
    </header>
  );
});
