/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

/**
 * What the channels page tells a crawler it is: the `<title>` and the
 * `description`, in both languages.
 *
 * They live beside the address map for the reason `WHY_META` and
 * `VISION_META` do: a route composes its head from one table, and nothing
 * else about the page's head is a decision the route takes.
 *
 * The `description` is the owner's, word for word. The `<title>` is the
 * one value on this page composed rather than quoted — the owner named the
 * page and not its tab — and it is composed the smallest way there is:
 * the page's own heading, then the product. The neighbouring pages spell
 * their title as «name — argument», and the argument in each of those is
 * the owner's own line about that page; inventing one here would put a
 * sentence nobody wrote in the one place a search result shows first.
 *
 * The structured data is the default `WebPage` the shared head builder
 * gives every subpage. That is deliberate and not an omission: the site
 * graph belongs to the root and is published there once, and this page is
 * a page of the site rather than a second claim about what the product
 * is. Naming the five channels as the product's `sameAs` would be such a
 * claim, and it belongs to the root's graph — where the decision to
 * change it is the owner's, taken deliberately (`head.ts`).
 */

import type { Locale } from "../landing/i18n.ts";
import { CHANNEL_URLS } from "./i18n.ts";
import { newsPath } from "./paths.ts";

export type NewsMeta = {
  readonly title: string;
  readonly description: string;
};

export const NEWS_META: Readonly<Record<Locale, NewsMeta>> = {
  en: {
    title: "News & support — VibeVM",
    description:
      "News, support and discussion for VibeVM: the Telegram news channel, the support chat, Reddit, and the creator on X.",
  },
  ru: {
    title: "Новости и поддержка — VibeVM",
    description:
      "Новости, поддержка и обсуждения VibeVM: новостной канал в Telegram, чат поддержки, Reddit и создатель в X.",
  },
};

/**
 * How the root `llms.txt` names this page under «## Project» — the same
 * shape as the essay's and the Why pages' entries, for the same reader:
 * an agent deciding what to fetch, and here also deciding which channels
 * are this project's own. English only, as the file is.
 */
export const NEWS_LLMS = {
  label: "News & support",
  gloss: `the official channels: the Telegram news channel (${CHANNEL_URLS.news}), the support and bug-report chat (${CHANNEL_URLS.support}), the community on Reddit (${CHANNEL_URLS.reddit}), a chat for general discussion (${CHANNEL_URLS.conversation}), and the creator of VibeVM, Oleg Chirukhin, on X (${CHANNEL_URLS.creator}).`,
} as const;

/** The `llms.txt` entry of the channels page, against a given origin. */
export function newsLlmsLine(origin: string): string {
  return `- [${NEWS_LLMS.label}](${origin}/${newsPath()}): ${NEWS_LLMS.gloss}`;
}
