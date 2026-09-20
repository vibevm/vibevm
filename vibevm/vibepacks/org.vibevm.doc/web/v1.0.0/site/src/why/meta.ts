/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

/**
 * What each Why page tells a crawler it is: the `<title>` and the
 * `description`, in both languages.
 *
 * They live beside the address map rather than inside each page for the
 * reason `pageHead` exists at all — six routes declaring the same six
 * tags by hand is six chances to forget one. The route composes the two
 * with its locale and its slug and hands them over; nothing else about
 * a page's head is a decision the page takes.
 *
 * Every string below is the owner's, moved byte for byte from the Astro
 * pages that carried them (`src/pages/why/*.astro` and their Russian
 * twins). The same exception applies here as to the landing's string
 * table: content is authorship, and a port that improved a description
 * on its way across would be a rewrite wearing a port's name.
 *
 * The AI-Native page keeps the words «AI-Native Language» while its
 * address is `/why/ai-native`. That is deliberate and not a leftover:
 * the owner settled the ADDRESS, and the copy is the copy. A port that
 * quietly retitled the page to match its new URL would be editing the
 * owner's text to agree with a routing decision.
 */

import type { Locale } from "../landing/i18n.ts";
import { type WhyPage, whyPath } from "./paths.ts";

export type WhyMeta = {
  readonly title: string;
  readonly description: string;
};

export const WHY_META: Readonly<
  Record<WhyPage, Readonly<Record<Locale, WhyMeta>>>
> = {
  vibevm: {
    en: {
      title: "Why VibeVM — discipline you can install",
      description:
        "Coding agents start every session blank. VibeVM installs process disciplines and specs as versioned, fingerprint-pinned packages and computes the exact text your agent reads first — pure file reading, reproducible on any machine, for any agent.",
    },
    ru: {
      title: "Почему VibeVM — дисциплина, которую можно установить",
      description:
        "Кодовый агент начинает каждую сессию с нуля. VibeVM ставит дисциплины процесса и спецификации как версионируемые пакеты, закреплённые отпечатком содержимого, и вычисляет точный текст, который агент читает первым, — чистое чтение файлов, воспроизводимое на любой машине, для любого агента.",
    },
  },
  zap: {
    en: {
      title: "Why Zap — your coding agents, on one map",
      description:
        "Zap brings projects, agent conversations, durable questions, managed work, and Git worktrees onto one local surface. Codex, Claude Code, OpenCode, and Qwen Code — with your own logins, and no inference until you press Start.",
    },
    ru: {
      title: "Почему Zap — ваши кодовые агенты на одной карте",
      description:
        "Zap собирает проекты, переписку агентов, устойчивые вопросы, управляемые задачи и Git-worktree на одной локальной поверхности. Codex, Claude Code, OpenCode и Qwen Code — с вашими логинами, без инференса до нажатия Start.",
    },
  },
  "ai-native": {
    en: {
      title: "AI-Native Language — code AI agents actually understand",
      description:
        "Code that AI agents actually understand: one discipline over Rust, TypeScript, and Go — every line's intent machine-traceable, architecture enforced by deterministic gates beside rustfmt, tsc, and gofmt. Research beta, dogfooded on VibeVM.",
    },
    ru: {
      title: "AI-Native Языки — код, по-настоящему понятный AI-агентам",
      description:
        "Код, по-настоящему понятный AI-агентам: одна дисциплина поверх Rust, TypeScript и Go — смысл каждой строки трассируем машиной, архитектура под детерминированными гейтами рядом с rustfmt, tsc и gofmt. Исследовательская бета, обкатана на VibeVM.",
    },
  },
};

/**
 * How the root `llms.txt` names the three pages under «## Project».
 *
 * A separate line from the crawler description above, because the two
 * answer different questions: a `meta description` is what a search
 * result should say, and this is what an agent that will never load the
 * page needs to know about it before deciding to. Both are the owner's,
 * moved from the Astro site's `public/llms.txt`.
 *
 * English only, as the file is. The `href` is composed rather than
 * copied, so the address in the index is the address the site serves —
 * which is how the AI-Native entry arrives at `/why/ai-native/` with the
 * owner's own words about it unchanged.
 */
export const WHY_LLMS: readonly {
  readonly page: WhyPage;
  readonly label: string;
  readonly gloss: string;
}[] = [
  {
    page: "vibevm",
    label: "Why VibeVM",
    gloss:
      "the product thesis — discipline you can install; two trees, the computed boot lane, the lockfile.",
  },
  {
    page: "zap",
    label: "Why Zap",
    gloss:
      "Zap, the VibeVM-family agent workspace — projects, durable questions, managed work, and Git worktrees on one local map.",
  },
  {
    page: "ai-native",
    label: "Why AI-Native Language",
    gloss:
      "the AI-Native Code Discipline — code AI agents actually understand: Rust, TypeScript, and Go stacks that make every line's intent machine-traceable over ordinary toolchains. Research beta.",
  },
];

/** The `llms.txt` entry of one Why page, against a given origin. */
export function whyLlmsLine(
  origin: string,
  entry: (typeof WHY_LLMS)[number],
): string {
  return `- [${entry.label}](${origin}/${whyPath(entry.page)}): ${entry.gloss}`;
}
