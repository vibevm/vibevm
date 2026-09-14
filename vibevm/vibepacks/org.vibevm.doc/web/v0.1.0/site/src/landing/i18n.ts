/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The landing's copy, in both languages, moved across one string at a
 * time.
 *
 * Every value below is the owner's text from the Astro site's `i18n.ts`,
 * byte for byte. This file is the one place in the port where copying is
 * not only allowed but required: R-28 forbids lifting another
 * repository's code and makes exactly one exception for the landing's
 * content, because the content is authorship and not technique, and a
 * port that paraphrased it would be a rewrite wearing a port's name
 * (D-28, F-74).
 *
 * So the asymmetries travel too, and they are not defects to be tidied
 * here: the English eyebrow lower-cases «driven development» where the
 * Russian one title-cases it; the English headline ends in a full stop
 * and the Russian one does not; `installBash` and `installPowerShell`
 * are technical labels and are not translated in either language. The
 * parity test would report any of these as a difference, which is
 * precisely why they are copied rather than improved.
 *
 * Two texts live here that the Astro site kept in its markup rather than
 * in its string table — the install commands and the 404 page — because
 * a port that left them inline would have no way to prove they came
 * across unchanged.
 *
 * A few entries are the PORT's own and say so where they stand:
 * `documentation` for the way in to the manual the shared header gained
 * (D-28), and the two words the copy button beside an install line needs.
 * They are marked rather than mixed in, so that «every other value is the
 * owner's, byte for byte» stays a claim a reader can check.
 */

export const LOCALES = ["en", "ru"] as const;
export type Locale = (typeof LOCALES)[number];

export const GITHUB_URL = "https://github.com/vibevm/vibevm";
export const GITVERSE_URL = "https://gitverse.ru/vibevm/vibevm";

/** One of the three capability cards under the hero. */
export type Cap = {
  readonly label: string;
  readonly head: string;
  readonly body: string;
};

export type Strings = {
  readonly htmlLang: string;
  readonly ogLocale: string;
  readonly metaTitle: string;
  readonly metaDescription: string;
  readonly eyebrow: string;
  /** The small status pill beside the install block. */
  readonly badge: string;
  /** May contain a single `<em>` around the accent word. */
  readonly headlineHtml: string;
  /** Contains the mandated descriptor, verbatim, inside `<strong>`. */
  readonly leadHtml: string;
  readonly ctaPrimary: string;
  readonly ctaSecondary: string;
  readonly installTitle: string;
  readonly installLead: string;
  readonly installBash: string;
  readonly installPowerShell: string;
  readonly installNext: string;
  /** What the copy button beside an install line IS; it shows no word. */
  readonly copyCommand: string;
  /** What the page says back once the command is on the clipboard. */
  readonly copied: string;
  readonly caps: readonly [Cap, Cap, Cap];
  readonly footerTagline: string;
  readonly copyright: string;
  /** The entry the port adds to the shared header (D-28). */
  readonly documentation: string;
};

export const STRINGS: Readonly<Record<Locale, Strings>> = {
  en: {
    htmlLang: "en",
    ogLocale: "en_US",
    metaTitle: "VibeVM — a package manager for Spec-Driven Development",
    metaDescription:
      "An ultimate prompt library, package manager, and agentic system for Spec-Driven Development.",
    eyebrow: "Open source · Spec-driven development",
    badge: "Early Access",
    headlineHtml: "Install the <em>context</em> your agents run on.",
    leadHtml:
      "VibeVM is <strong>an ultimate prompt library, package manager, and agentic system for Spec-Driven Development</strong> — declarative context assembled from versioned stacks, flows, and skills.",
    ctaPrimary: "View on GitHub",
    ctaSecondary: "Browse on GitVerse",
    installTitle: "Install VibeVM",
    installLead:
      "One command installs vibe, vibe-index, and the matching source tree.",
    installBash: "Linux · macOS · WSL",
    installPowerShell: "Windows PowerShell",
    installNext: "Then add a spec stack:",
    copyCommand: "Copy",
    copied: "Copied",
    caps: [
      {
        label: "Registry",
        head: "Installed like dependencies",
        body: "Specs, flows, and skills resolve through a decentralized registry and pin to a lockfile.",
      },
      {
        label: "Prompts",
        head: "Addressable and reusable",
        body: "A library of prompts and specifications you cite by URI — never by paraphrase.",
      },
      {
        label: "Agents",
        head: "Context computed at boot",
        body: "Any coding agent boots from a spec-driven context assembled at the start of a session.",
      },
    ],
    footerTagline: "Spec-Driven Development, packaged.",
    copyright: "© 2026 Oleg Chirukhin",
    documentation: "Documentation",
  },
  ru: {
    htmlLang: "ru",
    ogLocale: "ru_RU",
    metaTitle: "VibeVM — пакетный менеджер для Spec-Driven Development",
    metaDescription:
      "Ультимативная библиотека промптов, пакетный менеджер и агентная система для Spec-Driven Development.",
    eyebrow: "Открытый код · Spec-Driven Development",
    badge: "Ранний доступ",
    headlineHtml: "Установите <em>контекст</em> для вашего агента",
    leadHtml:
      "VibeVM — <strong>ультимативная библиотека промптов, пакетный менеджер и агентная система для Spec-Driven Development</strong>: декларативный контекст, собранный из версионируемых стеков, флоу и навыков.",
    ctaPrimary: "Открыть на GitHub",
    ctaSecondary: "Открыть на GitVerse",
    installTitle: "Установите VibeVM",
    installLead:
      "Одна команда установит vibe, vibe-index и соответствующее дерево исходников.",
    installBash: "Linux · macOS · WSL",
    installPowerShell: "Windows PowerShell",
    installNext: "Затем добавьте стек спецификаций:",
    copyCommand: "Скопировать",
    copied: "Скопировано",
    caps: [
      {
        label: "Реестр",
        head: "Ставятся как зависимости",
        body: "Спеки, флоу и навыки резолвятся через децентрализованный реестр и пиннятся в lockfile.",
      },
      {
        label: "Промпты",
        head: "Адресуемые и переиспользуемые",
        body: "Библиотека промптов и спецификаций, на которые ссылаются по URI, а не пересказом.",
      },
      {
        label: "Агенты",
        head: "Контекст собран на старте",
        body: "Любой кодовый агент стартует из spec-driven контекста, собранного в начале сессии.",
      },
    ],
    footerTagline: "Spec-Driven Development, в пакетах.",
    copyright: "© 2026 Олег Чирухин",
    documentation: "Документация",
  },
};

/**
 * The install commands, identical in both languages.
 *
 * They were inline in the Astro markup, not in its string table, and
 * they stay untranslated for the same reason a shell prompt does: a
 * command is typed, not read.
 */
export const INSTALL = {
  bashPrompt: "$",
  bashCommand: "curl -fsSL https://vibevm.org/install.sh | bash",
  powerShellPrompt: "PS>",
  powerShellCommand: "irm https://vibevm.org/install.ps1 | iex",
  /** The follow-up line: `vibe` is set apart, the rest is the argument. */
  nextCommandHead: "vibe",
  nextCommandTail: " install org.vibevm.world/redbook",
} as const;

/**
 * The 404 page, English only — as it was.
 *
 * The Astro site served one 404 for the whole domain, in English, with
 * the English page's head. Both facts are part of what is being moved,
 * and the Russian reader who lands on it sees what they saw before.
 */
export const NOT_FOUND = {
  eyebrow: "404",
  headlineHtml: "This page isn't in the <em>lockfile</em>.",
  lead: "The URL resolved to nothing. Head back to the homepage or find the project on GitHub.",
  ctaPrimary: "Go home",
  ctaSecondary: "View on GitHub",
} as const;

/** The address of a locale's landing, root-relative and slash-ended. */
export function localePath(locale: Locale): string {
  return locale === "en" ? "" : `${locale}/`;
}

/** The other language — there are two, and the switch says which. */
export function otherLocale(locale: Locale): Locale {
  return locale === "en" ? "ru" : "en";
}

/**
 * Which language a landing address is in.
 *
 * English has no prefix and every other language is a directory, which
 * is the address map the Astro site chose and the port keeps: the
 * strongest URL — the root — serves content rather than redirecting to a
 * prefixed copy of itself. So the question is only whether the first
 * segment is a known language, and `/`, `/en/` and `/404.html` all
 * answer English.
 */
export function localeFromPath(pathname: string): Locale {
  const first = pathname.split("/").find((part) => part.length > 0);
  return LOCALES.find((locale) => locale === first) ?? "en";
}
