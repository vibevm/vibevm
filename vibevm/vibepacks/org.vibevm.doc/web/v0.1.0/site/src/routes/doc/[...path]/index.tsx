/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { component$ } from "@qwik.dev/core";
import { useLocation, type StaticGenerateHandler } from "@qwik.dev/router";
import { Prose, TabPills } from "@vibe-docs/design";

import { Island } from "../../../components/island/index.tsx";
import { ISLAND_HTML } from "../../../lib/island-source.ts";
import { parseDocAddress } from "../../../lib/href.ts";
import { documentationParams } from "../../../lib/pages.ts";

/** The reading measure, until the reader's own setting overrides it. */
const MEASURE = 740;

/**
 * One documentation page, at the address D-06 gives it.
 *
 * A single catch-all route, not a tree of `[group]/[name]/[version]`
 * folders, because the address has an optional language in front of it
 * and a document path of any depth behind it — a pattern-per-shape route
 * tree would need one folder per possible depth and would still not
 * settle the language. The address map is deterministic instead: the
 * segments are parsed by one function, which is tested, and the same
 * function's inverse is what every link on the site is built with.
 *
 * The pages this route generates come from the page manifest, and the
 * build gate compares that number with the number the generator reports.
 */
export default component$(() => {
  const location = useLocation();
  const raw = location.params["path"] ?? "";
  const address = parseDocAddress(raw.split("/"));

  return (
    <Prose measure={MEASURE}>
      <TabPills
        label="Platform"
        items={[
          { label: "Windows", value: "windows", current: true },
          { label: "macOS", value: "macos", current: false },
          { label: "Linux", value: "linux", current: false },
        ]}
      />
      {address === null ? (
        <p class="doc-page__unaddressed">
          This is not a documentation address: <code>{raw}</code>
        </p>
      ) : (
        <Island html={ISLAND_HTML} />
      )}
    </Prose>
  );
});

/**
 * Which addresses the static generator writes to disk.
 *
 * It answers from the manifest rather than from a list, so the build
 * gate's two numbers — pages declared, pages generated — cannot drift
 * apart by someone editing one of them.
 */
export const onStaticGenerate: StaticGenerateHandler = () => {
  return { params: documentationParams() };
};
