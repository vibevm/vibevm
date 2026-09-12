/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PLACEHOLDERS-GENERATED */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
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
  /** One character for the generated placeholder, chosen by package kind. */
  readonly glyph: string;
};

/**
 * The head of a package's page: the banner, the icon, the title, the
 * publisher.
 *
 * A package that ships no banner gets one anyway. It is drawn here out
 * of the theme's own tokens rather than fetched, because the law is that
 * placeholders are generated and never stored, and because a site that
 * went to someone else's host for a decorative picture would tell that
 * host who is reading the documentation (D-20, R-09).
 *
 * What this placeholder does NOT yet do is vary with the coordinate. The
 * vision asks for a gradient computed from the package's own address, so
 * two packages are told apart at a glance and look the same on the site
 * and in the local reader — and the pipeline already derives exactly
 * that. It does not reach the shell: the page manifest carries no media
 * fields, and computing a second hash here in TypeScript would put the
 * same rule in two languages, where the two would drift apart quietly.
 * The gap is a field in the manifest, not a function in this file.
 */
export const PackageHeader = component$<PackageHeaderProps>((props) => {
  useStyles$(styles);
  return (
    <header class="package-header">
      {props.banner === undefined ? (
        <div class="package-header__banner package-header__banner--drawn" />
      ) : (
        <img class="package-header__banner" src={props.banner} alt="" />
      )}
      <div class="package-header__row">
        {props.icon === undefined ? (
          <span class="package-header__placeholder" aria-hidden="true">
            {props.glyph}
          </span>
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
