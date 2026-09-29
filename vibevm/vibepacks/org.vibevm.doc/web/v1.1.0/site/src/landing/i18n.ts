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
 * and the Russian one does not; `installBash`, `installPowerShell` and
 * `installCmd` are technical labels and are not translated in either
 * language. The parity test would report any of these as a difference,
 * which is precisely why they are copied rather than improved.
 *
 * Two texts live here that the Astro site kept in its markup rather than
 * in its string table — the install commands and the 404 page — because
 * a port that left them inline would have no way to prove they came
 * across unchanged.
 *
 * A few entries are the PORT's own and say so where they stand:
 * `documentation` for the way in to the manual the shared header gained
 * (D-28), the two words the copy button beside an install line needs,
 * the four the theme switch names its three states with, the three the
 * search box needs, and the name of the two letters in the corner —
 * which the Astro site drew without one. They are marked rather than
 * mixed in, so that «every other value is the owner's, byte for byte»
 * stays a claim a reader can check.
 *
 * The map at the foot of the landing (owner, 2026-09-26) adds one more
 * such group, marked the same way: the `map*` entries below are the
 * words the map says over the header's own labels — a kicker, a title, a
 * line, the two rows' names, and for each destination a line about it
 * and its drawing in words. The English is written for the page; the
 * Russian is an adaptation of it and not a gloss.
 *
 * The project's full NAME arrived after all of that (owner, 2026-09-27):
 * it is Anarchic VibeVM, conceived and built by Oleg Chirukhin, and every
 * formulation that names the project on the front door says so — the
 * lead, the note under it, the two meta values, and the first mention in
 * each of the two notes at the foot of the page. Those are the owner's
 * own words again, in both languages, and the addresses inside them are
 * the tables' rather than this file's.
 *
 * What the whole DOMAIN asserts about which project this is — the name
 * statement and the three notes about the namesakes — is not here but in
 * `identity.ts`, because each of those sentences is said in four places
 * and this file is what one page says. The values below that carry them
 * read them from there.
 */

import { CREATOR_SITE } from "../lib/people.ts";
import {
  AI_NATIVE_LANGUAGES_EN,
  ANARCHIC_URL,
  PHALA_DISAMBIGUATION_EN,
  TGBYTE_DISAMBIGUATION_EN,
} from "./identity.ts";
import type { MenuId, MenuRowId } from "./menu.ts";

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

/**
 * One plate of the map: what a reader finds behind the destination, and
 * what its drawing shows, for a reader who cannot see it. The name is
 * not here — it is the header's own label, read off the same list the
 * header prints (`menu.ts`), so the two can never say different things.
 */
export type MapPlate = {
  /** One or two lines: what is there. */
  readonly body: string;
  /** The drawing, in words — the plate's `aria-label`. */
  readonly alt: string;
};

export type Strings = {
  readonly htmlLang: string;
  readonly ogLocale: string;
  readonly metaTitle: string;
  readonly metaDescription: string;
  /** The three Why pages, as the header and the footer name them. */
  readonly navWhyVibevm: string;
  readonly navWhyZap: string;
  readonly navWhyAiNative: string;
  /**
   * The essay's entry in the header and the footer. A pointer's label,
   * not the essay's title: the page calls itself «Большой Вижен» /
   * “The Big Vision” — the author's voice — and a menu names where a
   * link leads in one word.
   */
  readonly navVision: string;
  /**
   * The roadmap's entry in the header, right after the essay (owner,
   * 2026-09-26). A pointer's label like the essay's: the page heads
   * itself «From a preview to an operating system» / «От превью до
   * операционной системы», and the menu says in one or two words where
   * the link leads.
   */
  readonly navRoadmap: string;
  /**
   * The channels page's entry in the header. Also a pointer's label: the
   * page heads itself «News & support» / «Новости и поддержка» and the
   * menu says the same, because here the page's name IS where the link
   * leads — there is no shorter true word for «news, and also help».
   */
  readonly navNews: string;
  /**
   * The law page's entry in the header, in the first row right after the
   * channels (owner, 2026-09-27). A pointer's label like its neighbours:
   * the page of the manual it leads to heads itself «Applicable law», and
   * the menu says in one word which kind of question is answered there —
   * which law governs dealings with the project, and what it does with
   * personal data.
   */
  readonly navLegal: string;
  readonly eyebrow: string;
  /**
   * The small status pill beside the install block. It carries the
   * RELEASE's name, which is one name in both languages: «Developer
   * Preview 1» is what this build is called, not a description of it.
   */
  readonly badge: string;
  /**
   * The one link beside that pill, to the page of the manual that
   * answers the question a reader who has just met the product has
   * (owner, 2026-09-26). A label and not a sentence: the page it leads
   * to is called «What VibeVM is», and this is the reader's own question.
   */
  readonly badgeAction: string;
  /** May contain a single `<em>` around the accent word. */
  readonly headlineHtml: string;
  /**
   * Contains the mandated descriptor, verbatim, inside `<strong>`, and
   * opens with the project's full name — whose first word is a link
   * (owner, 2026-09-27).
   */
  readonly leadHtml: string;
  /**
   * The first sentence in the small print at the foot of the landing: what
   * the project is properly called, who made it and what for (owner, moved
   * here 2026-09-28). It carries two links — the brand in the name and the
   * person — and stays one string so a translator sees the whole sentence.
   * The two addresses are interpolated from their owning tables.
   */
  readonly nameNoteHtml: string;
  readonly ctaPrimary: string;
  readonly ctaSecondary: string;
  readonly installTitle: string;
  readonly installLead: string;
  readonly installBash: string;
  readonly installPowerShell: string;
  readonly installCmd: string;
  readonly installWindows: string;
  readonly installWindowsLead: string;
  readonly installManual: string;
  readonly installManualLead: string;
  readonly installReleases: string;
  readonly installReleasesLead: string;
  readonly installBinaryReleases: string;
  readonly installBinaryReleasesLead: string;
  readonly installSourceGitHub: string;
  readonly installSourceGitVerse: string;
  readonly installSourceLead: string;
  readonly installNext: string;
  /** What the copy button beside an install line IS; it shows no word. */
  readonly copyCommand: string;
  /** What the page says back once the command is on the clipboard. */
  readonly copied: string;
  readonly caps: readonly [Cap, Cap, Cap];
  /**
   * The small print at the foot of the landing that tells this VibeVM
   * apart from Phala Network's project of the same name (owner,
   * 2026-09-26). It is written for a crawler as much as for a person:
   * both projects are named in full, with their addresses, so that an
   * index that has been joining the two has a sentence to split them on.
   * The same words reach `llms.txt`, `llms-full.txt` and the structured
   * data.
   */
  readonly disambiguation: string;
  /**
   * The second and third paragraphs of that small print, in the order a
   * reader meets them (owner, 2026-09-26): what VibeVM does with
   * environments and machines and which namesake it is not, then what the
   * AI Native Languages are and which namesake THEY are not. Each travels
   * to the same four places as the first, and neither describes the
   * project it names.
   */
  readonly disambiguationTgbyte: string;
  readonly disambiguationAiNative: string;
  readonly footerTagline: string;
  readonly copyright: string;
  /** The entry the port adds to the shared header (D-28). */
  readonly documentation: string;
  /** What the two letters in the corner are, for a reader who cannot see them. */
  readonly siteLanguage: string;
  /** The theme switch in the header, and its three states. */
  readonly theme: string;
  readonly themeLight: string;
  readonly themeDark: string;
  readonly themeSystem: string;
  /** The search box the header shares with the documentation's. */
  readonly search: string;
  readonly searchPlaceholder: string;
  readonly searchEmpty: string;
  /**
   * The map under the landing's content: the header's destinations drawn
   * large, for the reader who does not read a menu (owner, 2026-09-26).
   * The port's own words, marked as such — a kicker, a title, the line
   * under it, the two rows' names, and one plate per destination.
   */
  readonly mapK: string;
  readonly mapTitle: string;
  readonly mapLede: string;
  readonly mapRows: Readonly<Record<MenuRowId, string>>;
  readonly mapPlates: Readonly<Record<MenuId, MapPlate>>;
};

export const STRINGS: Readonly<Record<Locale, Strings>> = {
  en: {
    htmlLang: "en",
    ogLocale: "en_US",
    metaTitle:
      "Anarchic VibeVM — a package manager for Spec-Driven Development",
    metaDescription:
      "Anarchic VibeVM: an ultimate prompt library, package manager, and agentic system for Spec-Driven Development.",
    navWhyVibevm: "Why VibeVM",
    navWhyZap: "Why Zap",
    navWhyAiNative: "AI-Native Language",
    navVision: "Vision",
    navRoadmap: "Roadmap",
    navNews: "News & support",
    navLegal: "Legal",
    eyebrow: "Open source · Spec-driven development",
    badge: "Developer Preview 1",
    badgeAction: "What is VibeVM?",
    headlineHtml: "Install the <em>context</em> your agents run on.",
    leadHtml: `<a href="${ANARCHIC_URL}">Anarchic</a> VibeVM is <strong>an ultimate prompt library, package manager, and agentic system for Spec-Driven Development</strong> — declarative context assembled from versioned stacks, flows, and skills.`,
    nameNoteHtml: `The correct full name of the project is <a href="${ANARCHIC_URL}">Anarchic</a> VibeVM. It was originally conceived and built by <a href="${CREATOR_SITE}">Oleg Chirukhin</a> as a way to make personal and group vibe coding simpler.`,
    ctaPrimary: "View on GitHub",
    ctaSecondary: "Browse on GitVerse",
    installTitle: "Install VibeVM",
    installLead:
      "One command installs vibe, vibe-index, and the matching source tree.",
    installBash: "Linux · macOS · WSL",
    installPowerShell: "Windows PowerShell",
    installCmd: "Windows cmd.exe",
    installWindows: "Windows",
    installWindowsLead: "PowerShell, cmd.exe, and a manual path",
    installManual: "Manual installation",
    installManualLead: "Follow the complete Windows installation guide",
    installReleases: "Releases for other platforms",
    installReleasesLead: "Binaries and source repositories",
    installBinaryReleases: "Binary releases (full list)",
    installBinaryReleasesLead: "Browse every published build on GitHub",
    installSourceGitHub: "Source: GitHub",
    installSourceGitVerse: "Source: GitVerse",
    installSourceLead: "Build VibeVM from the source tree",
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
    disambiguation: PHALA_DISAMBIGUATION_EN,
    disambiguationTgbyte: TGBYTE_DISAMBIGUATION_EN,
    disambiguationAiNative: AI_NATIVE_LANGUAGES_EN,
    footerTagline: "Spec-Driven Development, packaged.",
    copyright: "© 2026 Oleg Chirukhin",
    documentation: "Documentation",
    siteLanguage: "Site language",
    theme: "Theme",
    themeLight: "Light",
    themeDark: "Dark",
    themeSystem: "System",
    search: "Search the documentation",
    searchPlaceholder: "Search",
    searchEmpty: "Nothing here carries that word.",
    mapK: "Where to go",
    mapTitle: "Everything here, on one plane.",
    mapLede: "Every place the menu at the top leads to, drawn large. Pick one.",
    mapRows: { tools: "The software", story: "The argument" },
    mapPlates: {
      documentation: {
        body: "The manual for vibe and the packages it installs — read online, page by page, in English and in Russian.",
        alt: "A page as a tall dark plane: short pale lines of text, a terracotta block number over its corner, a second page behind it, and one thin diagonal — the line a reader follows.",
      },
      github: {
        body: "The canonical repository: the code, the issues, the releases.",
        alt: "A commit graph as planes: a dark trunk with three pinned nodes, one terracotta branch leaving to the right and merging back, and a solid dark square standing for the source.",
      },
      gitverse: {
        body: "The same source, mirrored on GitVerse.",
        alt: "The same commit graph, mirrored: the branch leaves to the left, and the square standing for the source is hollow — a copy.",
      },
      news: {
        body: "Release news on Telegram, a chat for questions and bug reports, the community on Reddit — and the creator on X.",
        alt: "A broadcast: a terracotta circle sends three dashed arcs up and to the right, two small dark planes receive them, and one thin line runs back to the circle.",
      },
      legal: {
        body: "Which law governs dealings with VibeVM, and how the project treats personal data.",
        alt: "Scales as planes: a long dark beam resting level on a small terracotta triangle, a dark square hanging from one end and a terracotta circle from the other, each on two thin lines.",
      },
      vision: {
        body: "The essay behind everything on this site: two sources of intention, an expensive probabilistic layer over a cheap deterministic one, and traceable edges between them.",
        alt: "The essay's two sources of intention: a terracotta circle and a tilted cobalt square joined by a gold arc, each dropping an edge to one floor, with smaller nodes traced between them.",
      },
      roadmap: {
        body: "Where this is going, in order: a more extensible core, Zap, the first community packages, IDE plugins, a library and marketplace of our own, and a Linux whose package manager is vibevm. No dates.",
        alt: "A road as one ink diagonal rising to the right from a terracotta circle at its foot; along it six stations grow from a single square into a whole system of squares, over one floor line.",
      },
      "why-vibevm": {
        body: "The product thesis: discipline you can install. Specs, flows and skills as versioned, pinned packages — and the context an agent boots from, computed from them.",
        alt: "A terracotta wedge entering a field of scattered tilted rectangles and, past it, a lattice of squares pinned in place.",
      },
      "why-zap": {
        body: "Announced for October 2026: a local workspace that puts coding agents, their questions and their worktrees on one map. A declaration of intent, not a release.",
        alt: "An orbital map in green: a core with dashed rings around it, work nodes on the rings, one terracotta node for the project, and a trajectory leaving toward the edge.",
      },
      "why-ai-native": {
        body: "One discipline over Rust, TypeScript and Go: the code stays ordinary, and the strictness lives around it — in gates a machine runs.",
        alt: "A gold core square projecting three rays onto an oxide square, a slate circle and a teal triangle, each clamped by gold brackets, all standing on one floor with a single gate notch.",
      },
    },
  },
  ru: {
    htmlLang: "ru",
    ogLocale: "ru_RU",
    metaTitle:
      "Anarchic VibeVM — пакетный менеджер для Spec-Driven Development",
    metaDescription:
      "Anarchic VibeVM — ультимативная библиотека промптов, пакетный менеджер и агентная система для Spec-Driven Development.",
    navWhyVibevm: "Почему VibeVM",
    navWhyZap: "Почему Zap",
    navWhyAiNative: "AI-Native Языки",
    navVision: "Видение",
    /* «Роадмап» and not «Дорожная карта» in the menu, for the bar's sake:
       the second row is the widest thing in the header, and the longer
       label pushed the Russian bar past a tablet held upright, the width
       the two-row composition is measured at. The page itself says
       «Дорожная карта» in its eyebrow and its title; the menu says where
       the link leads in one word, as «Видение» does for the essay. */
    navRoadmap: "Роадмап",
    navNews: "Новости и поддержка",
    navLegal: "Право",
    eyebrow: "Открытый код · Spec-Driven Development",
    badge: "Developer Preview 1",
    badgeAction: "Что такое VibeVM?",
    headlineHtml: "Установите <em>контекст</em> для вашего агента",
    leadHtml: `<a href="${ANARCHIC_URL}">Anarchic</a> VibeVM — <strong>ультимативная библиотека промптов, пакетный менеджер и агентная система для Spec-Driven Development</strong>: декларативный контекст, собранный из версионируемых стеков, флоу и навыков.`,
    nameNoteHtml: `Правильное полное название проекта — <a href="${ANARCHIC_URL}">Anarchic</a> VibeVM. Изначально её придумал и реализовал <a href="${CREATOR_SITE}">Олег Чирухин</a> как средство для упрощения персонального и группового вайбкодинга.`,
    ctaPrimary: "Открыть на GitHub",
    ctaSecondary: "Открыть на GitVerse",
    installTitle: "Установите VibeVM",
    installLead:
      "Одна команда установит vibe, vibe-index и соответствующее дерево исходников.",
    installBash: "Linux · macOS · WSL",
    installPowerShell: "Windows PowerShell",
    installCmd: "Windows cmd.exe",
    installWindows: "Windows",
    installWindowsLead: "PowerShell, cmd.exe и ручная установка",
    installManual: "Ручная установка",
    installManualLead: "Полная инструкция по установке в Windows",
    installReleases: "Релизы для других платформ",
    installReleasesLead: "Бинарные сборки и репозитории с исходным кодом",
    installBinaryReleases: "Бинарные релизы (полный список)",
    installBinaryReleasesLead: "Все опубликованные сборки на GitHub",
    installSourceGitHub: "Исходники: GitHub",
    installSourceGitVerse: "Исходники: GitVerse",
    installSourceLead: "Соберите VibeVM из дерева исходного кода",
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
    disambiguation:
      "Anarchic VibeVM на vibevm.org не связан с Phala Cloud. Это не VibeVM от Phala Network (github.com/Phala-Network/VibeVM) — песочница для разработки в конфиденциальной виртуальной машине на Phala Cloud. Это два разных, никак не связанных проекта с одинаковым названием.",
    disambiguationTgbyte:
      "Anarchic VibeVM может использоваться для подготовки произвольных сред и операционных систем, в том числе для создания throwaway virtual machines, в том числе в целях создания confidential sandboxing. К одноимённому проекту tgbyte VibeVM не имеет никакого отношения.",
    disambiguationAiNative:
      "Проект AI Native Languages (например, AI Native Rust) — это специализированный предметно-ориентированный язык (DSL), где в качестве среды выполнения (runtime) выступает сама большая языковая модель (LLM). С его помощью разработчики описывают сложные ветвления логики, параллельное выполнение задач ИИ и управление контекстом. Никакого отношения к проекту karanchawla VVM он не имеет.",
    footerTagline: "Spec-Driven Development, в пакетах.",
    copyright: "© 2026 Олег Чирухин",
    documentation: "Документация",
    siteLanguage: "Язык сайта",
    theme: "Тема",
    themeLight: "Светлая",
    themeDark: "Тёмная",
    themeSystem: "Системная",
    search: "Искать в документации",
    searchPlaceholder: "Поиск",
    searchEmpty: "Здесь нет ничего с этим словом.",
    mapK: "Куда дальше",
    mapTitle: "Весь сайт на одной плоскости.",
    mapLede: "Всё, куда ведёт меню наверху, — крупно и в картинках. Выбирайте.",
    mapRows: { tools: "Софт", story: "Смысл" },
    mapPlates: {
      documentation: {
        body: "Руководство по vibe и пакетам, которые он ставит: страница за страницей, на русском и английском.",
        alt: "Страница как высокая тёмная плоскость: короткие светлые строки текста, терракотовый номер блока поверх угла, вторая страница позади и одна тонкая диагональ — линия, по которой идёт читатель.",
      },
      github: {
        body: "Основной репозиторий: код, issues, релизы.",
        alt: "Граф коммитов из плоскостей: тёмный ствол с тремя пришпиленными узлами, терракотовая ветка уходит вправо и вливается обратно, рядом стоит сплошной тёмный квадрат — исходник.",
      },
      gitverse: {
        body: "Тот же исходный код — зеркало на GitVerse.",
        alt: "Тот же граф коммитов, отражённый: ветка уходит влево, а квадрат-исходник полый — это копия.",
      },
      news: {
        body: "Новости релизов в Telegram, чат для вопросов и багов, сообщество на Reddit — и создатель в X.",
        alt: "Вещание: терракотовый круг посылает три пунктирные дуги вверх и вправо, две маленькие тёмные плоскости их принимают, и одна тонкая линия возвращается к кругу.",
      },
      legal: {
        body: "Какое право действует в делах с VibeVM и как проект обходится с персональными данными.",
        alt: "Весы из плоскостей: длинная тёмная балка ровно лежит на маленьком терракотовом треугольнике, с одного её конца свисает тёмный квадрат, с другого — терракотовый круг, каждый на двух тонких линиях.",
      },
      vision: {
        body: "Эссе, из которого выросло всё на этом сайте: два источника намерения, дорогой вероятностный слой над дешёвым детерминированным — и трассируемые рёбра между ними.",
        alt: "Два источника намерения из эссе: терракотовый круг и наклонённый кобальтовый квадрат, соединённые золотой дугой; каждый опускает ребро на общий пол, между ними — узлы поменьше.",
      },
      roadmap: {
        body: "Куда всё идёт, по порядку: более расширяемое ядро, Zap, первые пакеты от сообщества, плагины для IDE, своя библиотека и маркетплейс — и Linux, в котором пакетный менеджер — vibevm. Без дат.",
        alt: "Дорога — одна чернильная диагональ, поднимающаяся вправо от терракотового круга у подножия; вдоль неё шесть станций растут из одного квадрата в целую систему квадратов над одной линией пола.",
      },
      "why-vibevm": {
        body: "Тезис продукта: дисциплина, которую можно установить. Спеки, флоу и навыки — версионируемые, закреплённые пакеты, а из них вычисляется контекст, с которого стартует агент.",
        alt: "Терракотовый клин входит в поле разбросанных наклонённых прямоугольников, а за ним — решётка квадратов, закреплённых на своих местах.",
      },
      "why-zap": {
        body: "Анонс на октябрь 2026 года: локальное рабочее пространство, где кодовые агенты, их вопросы и worktree — на одной карте. Заявление о намерениях, а не релиз.",
        alt: "Орбитальная карта в зелёном: ядро с пунктирными кольцами, узлы работы на кольцах, один терракотовый узел — проект, и траектория, уходящая к краю.",
      },
      "why-ai-native": {
        body: "Одна дисциплина поверх Rust, TypeScript и Go: код остаётся обычным, а строгость живёт вокруг него — в гейтах, которые запускает машина.",
        alt: "Золотой квадрат-ядро проецирует три луча на оксидный квадрат, сланцевый круг и бирюзовый треугольник; каждый зажат золотыми скобками, все стоят на одном полу с единственной насечкой-выходом.",
      },
    },
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
  cmdPrompt: ">",
  cmdCommand:
    'curl.exe -fsSL https://vibevm.org/install.cmd -o "%TEMP%\\vibevm-install.cmd" && call "%TEMP%\\vibevm-install.cmd"',
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

/**
 * The same address with its language taken off — what a page IS, rather
 * than which edition of it a reader is on.
 *
 * It is what lets the two letters in the corner keep a reader where they
 * are: `/ru/why/zap/` and `/why/zap/` both reduce to `why/zap/`, so the
 * switch can offer the other language's spelling of THIS page instead of
 * sending everyone back to the front door. The landing reduces to the
 * empty string, which is its own address in both languages and needs no
 * special case.
 *
 * Root-relative and without a leading slash, in the form `href` takes,
 * so the result composes with `localePath` by concatenation.
 *
 * A FILE is not a place and answers the empty string. `/404.html` is the
 * one address on this site that is a file rather than a directory — the
 * router builds it to exactly that name because that is what
 * `error_page 404` serves — and it exists once, in English. Treating it
 * as a page path produced `/404.html/` in the language switch and in the
 * footer: two addresses that are nothing, on the one page a reader
 * reaches by already being lost. The Astro layout answered the same
 * question by being called without a path at all; here the address is
 * the only input, so the rule is read off the address.
 */
export function pathWithinLocale(pathname: string): string {
  const parts = pathname.split("/").filter((part) => part.length > 0);
  const rest = LOCALES.some((locale) => locale === parts[0])
    ? parts.slice(1)
    : parts;
  const last = rest[rest.length - 1];
  if (last === undefined || last.includes(".")) return "";
  return `${rest.join("/")}/`;
}
