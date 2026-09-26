/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The channels page's copy, in both languages.
 *
 * Every string below is the owner's, word for word, in the order the
 * owner grouped them: where the project posts its news, where a reader
 * asks for help or reports a bug, and where everything else is talked
 * about. The same exception applies here as to the landing's string
 * table — content is authorship, and a page that improved a channel's
 * one-line description on its way in would be writing rather than
 * publishing.
 *
 * What is NOT copy is the address a card shows. It is derived from the
 * address the card leads to, by `shownAddress` below, so that the two can
 * never disagree: a card whose visible `t.me/vibevm` pointed at anything
 * else would be the one kind of defect a reader cannot catch by reading.
 *
 * The platform is a word and not a mark. No third-party logo enters this
 * tree: a page of five links would otherwise carry four foreign brands
 * with their own colour, their own clear space and their own licence, and
 * the one thing a reader needs from them — which app opens this — is a
 * word. So each card says «Telegram channel» or «Reddit» in the mono face
 * the rest of the site labels things with.
 */

import type { Locale } from "../landing/i18n.ts";

/** One channel: a place, on a platform, at an address, for a reason. */
/**
 * The five addresses, once. The page's two languages, the root
 * `llms.txt` (`meta.ts`) and the structured data's `sameAs`
 * (`landing/head.ts`) all read them from here, so a channel that moves is
 * one edit and no reader, person or crawler, is left holding the old one.
 */
export const CHANNEL_URLS = {
  news: "https://t.me/vibevm",
  support: "https://t.me/vibevm_chat",
  reddit: "https://www.reddit.com/r/vibevm/",
  conversation: "https://t.me/chat_1red2black",
  creator: "https://x.com/1red2black",
} as const;

export type Channel = {
  /** What the channel calls itself. */
  readonly name: string;
  /** Which app or site opens it, as a word. */
  readonly platform: string;
  /** The address, absolute — what the card leads to and shows. */
  readonly href: string;
  /** One line: what a reader will find there. */
  readonly body: string;
};

/** One group of channels, under the owner's own heading. */
export type ChannelGroup = {
  /** The id the group's heading is announced by; the same in both languages. */
  readonly id: string;
  readonly head: string;
  readonly channels: readonly Channel[];
};

export type NewsStrings = {
  /** The page's heading, which is also what the menu entry leads to. */
  readonly title: string;
  /** The one paragraph under it, saying what the three groups are. */
  readonly lede: string;
  readonly groups: readonly ChannelGroup[];
};

/**
 * The address as a card prints it: no scheme, no `www.`, no trailing
 * slash.
 *
 * A reader deciding whether to follow a link wants to know where it goes,
 * and `https://` at the front of five addresses is five times the same
 * word. What is left is the part that differs — the host and the handle —
 * which is also the part a reader could type, recognise or check against
 * what they were told elsewhere. It is derived and never written down
 * twice (`##SITE-ONE-SITE`: one fact, one place).
 */
export function shownAddress(href: string): string {
  return href
    .replace(/^https?:\/\//, "")
    .replace(/^www\./, "")
    .replace(/\/$/, "");
}

const EN: NewsStrings = {
  title: "News & support",
  lede: "Where VibeVM posts its news, where to ask for help and report a bug, and where to talk about everything else.",
  groups: [
    {
      id: "news",
      head: "News",
      channels: [
        {
          name: "VibeVM News",
          platform: "Telegram channel",
          href: CHANNEL_URLS.news,
          body: "Releases and announcements.",
        },
        {
          name: "Oleg Chirukhin",
          platform: "X",
          href: CHANNEL_URLS.creator,
          body: "The creator of VibeVM.",
        },
      ],
    },
    {
      id: "support",
      head: "Support",
      channels: [
        {
          name: "VibeVM Chat",
          platform: "Telegram chat",
          href: CHANNEL_URLS.support,
          body: "Support and bug reports.",
        },
        {
          name: "r/vibevm",
          platform: "Reddit",
          href: CHANNEL_URLS.reddit,
          body: "The VibeVM community on Reddit.",
        },
      ],
    },
    {
      id: "conversation",
      head: "Conversation",
      channels: [
        {
          name: "1red2black chat",
          platform: "Telegram chat",
          href: CHANNEL_URLS.conversation,
          body: "Off-topic and general discussion.",
        },
      ],
    },
  ],
};

const RU: NewsStrings = {
  title: "Новости и поддержка",
  lede: "Где VibeVM публикует новости, где попросить помощи и сообщить об ошибке — и где поговорить обо всём остальном.",
  groups: [
    {
      id: "news",
      head: "Новости",
      channels: [
        {
          name: "VibeVM News",
          platform: "Канал в Telegram",
          href: CHANNEL_URLS.news,
          body: "Выпуски и анонсы.",
        },
        {
          name: "Олег Чирухин",
          platform: "X",
          href: CHANNEL_URLS.creator,
          body: "Создатель VibeVM.",
        },
      ],
    },
    {
      id: "support",
      head: "Поддержка",
      channels: [
        {
          name: "VibeVM Chat",
          platform: "Чат в Telegram",
          href: CHANNEL_URLS.support,
          body: "Поддержка и баг-репорты.",
        },
        {
          name: "r/vibevm",
          platform: "Reddit",
          href: CHANNEL_URLS.reddit,
          body: "Сообщество VibeVM на Reddit.",
        },
      ],
    },
    {
      id: "conversation",
      head: "Разговоры",
      channels: [
        {
          name: "Чат 1red2black",
          platform: "Чат в Telegram",
          href: CHANNEL_URLS.conversation,
          body: "Флуд и общие обсуждения.",
        },
      ],
    },
  ],
};

export const STRINGS: Readonly<Record<Locale, NewsStrings>> = {
  en: EN,
  ru: RU,
};
