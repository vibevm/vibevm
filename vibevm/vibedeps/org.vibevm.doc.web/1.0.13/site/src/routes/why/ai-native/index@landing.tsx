/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { component$ } from "@qwik.dev/core";
import type { DocumentHead } from "@qwik.dev/router";

import { pageHead } from "../../../landing/head.ts";
import { WhyAiNative } from "../../../why/ai-native/index.tsx";
import { WHY_META } from "../../../why/meta.ts";
import { whyPath } from "../../../why/paths.ts";

/**
 * `https://vibevm.org/why/ai-native/` — the English page for the
 * AI-Native Code Discipline.
 *
 * One address, and it is the short one. The review worktree that
 * authored this page wrote `/why/ai-native-language/` while the family
 * was still being named; the owner settled on `ai-native`, and the port
 * carries exactly that. There is no second route and no redirect from
 * the longer form — it was never served from this domain, so there is
 * nothing to keep alive.
 */
export default component$(() => <WhyAiNative locale="en" />);

export const head: DocumentHead = pageHead({
  locale: "en",
  path: whyPath("ai-native"),
  ...WHY_META["ai-native"].en,
});
