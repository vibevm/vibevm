/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import {
  CapabilityCard,
  CapabilityRow,
  DepGraph,
  Hero,
  InstallBlock,
} from "@vibe-docs/design";

import {
  GITHUB_URL,
  GITVERSE_URL,
  INSTALL,
  type Locale,
  STRINGS,
} from "./i18n.ts";

/**
 * The landing itself: a hero with an install panel and a constellation,
 * then three cards that say what the thing is.
 *
 * The route is a composition and nothing else — every element it places
 * comes from `design/`, and every word it places comes from `i18n.ts`.
 * That is the shape the port was aiming at: the same page the Astro site
 * serves, assembled out of the components the documentation is also
 * assembled from, so that a change to a button is one change (D-28).
 */
export type LandingProps = {
  readonly locale: Locale;
};

/**
 * The label the capabilities row carries for a screen reader.
 *
 * English on both pages, because it was English on both pages: the
 * Astro markup wrote it inline rather than through the string table and
 * the Russian page inherited it. Copied rather than fixed — the port
 * moves the landing as it is, and a string that was never translated is
 * the translator's to add, with a line in the string table to add it to
 * (F-74).
 */
const CAPABILITIES_LABEL = "What VibeVM is";

/** The id the install panel's heading carries, and its `aria-labelledby`. */
const INSTALL_HEADING_ID = "install-title";

export const Landing = component$<LandingProps>((props) => {
  const t = STRINGS[props.locale];
  return (
    <>
      <Hero
        eyebrow={t.eyebrow}
        headlineHtml={t.headlineHtml}
        leadHtml={t.leadHtml}
        primary={{ label: t.ctaPrimary, href: GITHUB_URL }}
        secondary={{ label: t.ctaSecondary, href: GITVERSE_URL }}
        badge={t.badge}
      >
        <InstallBlock
          headingId={INSTALL_HEADING_ID}
          title={t.installTitle}
          lead={t.installLead}
          commands={[
            {
              label: t.installBash,
              prompt: INSTALL.bashPrompt,
              command: INSTALL.bashCommand,
            },
            {
              label: t.installPowerShell,
              prompt: INSTALL.powerShellPrompt,
              command: INSTALL.powerShellCommand,
            },
          ]}
          nextLabel={t.installNext}
          nextCommandHead={INSTALL.nextCommandHead}
          nextCommandTail={INSTALL.nextCommandTail}
        />
        <div q:slot="aside">
          <DepGraph />
        </div>
      </Hero>

      <CapabilityRow label={CAPABILITIES_LABEL}>
        {t.caps.map((cap) => (
          <CapabilityCard
            key={cap.label}
            label={cap.label}
            heading={cap.head}
            body={cap.body}
          />
        ))}
      </CapabilityRow>
    </>
  );
});
