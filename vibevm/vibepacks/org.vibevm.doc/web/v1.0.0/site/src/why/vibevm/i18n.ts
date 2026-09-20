/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The copy of `/why/vibevm`, in both languages.
 *
 * Every value is the owner's, moved byte for byte from the Astro
 * component that carried it (`src/components/why/WhyVibevm.astro`). The
 * landing's string table records why that is allowed and required here:
 * content is authorship, not technique, and a paraphrase would be a
 * rewrite wearing a port's name.
 *
 * So the asymmetries travel too and are not defects to be tidied: the
 * English lead ends its clauses differently from the Russian one, the
 * step commands are untranslated in both languages because a command is
 * typed rather than read, and the install labels are technical strings
 * that were never translated. The parity test would report any of these
 * as a difference, which is exactly why they are copied.
 *
 * Two values are the PORT's own and say so where they stand: the label
 * the architecture diagram carries for a reader who cannot see it was
 * already the source's, but the diagram is now a picture with a name,
 * and the caption under the frieze was `aria-hidden` in the source and
 * stays so here.
 */

export type Cap = {
  readonly label: string;
  readonly head: string;
  readonly body: string;
};

export type Step = {
  readonly cmd: string;
  readonly body: string;
};

export type Strings = {
  readonly eyebrow: string;
  readonly headlineHtml: string;
  readonly lead: string;
  readonly ctaInstall: string;
  readonly ctaZap: string;
  readonly friezeAlt: string;
  readonly friezeCaption: string;

  readonly problemK: string;
  readonly problemH: string;
  readonly problemBody: string;
  readonly problemPoints: readonly string[];

  readonly thesisK: string;
  readonly thesisH: string;
  readonly thesisBody: string;
  readonly archAlt: string;
  readonly archYoursHead: string;
  readonly archYoursTag: string;
  readonly archYoursBody: string;
  readonly archDepsHead: string;
  readonly archDepsTag: string;
  readonly archDepsBody: string;
  readonly archLaneHead: string;
  readonly archLaneTag: string;
  readonly archLaneStatic: string;
  readonly archLaneIndex: string;
  readonly archAgentHead: string;
  readonly archAgentBody: string;
  readonly archNote: string;

  readonly tokenK: string;
  readonly tokenH: string;
  readonly tokenBody: string;
  readonly tokenTag: string;

  readonly capsK: string;
  readonly capsH: string;
  readonly caps: readonly Cap[];

  readonly howK: string;
  readonly howH: string;
  readonly steps: readonly Step[];

  readonly fitK: string;
  readonly fitH: string;
  readonly fitForHead: string;
  readonly fitFor: readonly string[];
  readonly fitNotHead: string;
  readonly fitNot: readonly string[];
  readonly statusHead: string;
  readonly statusBody: string;

  readonly ctaK: string;
  readonly ctaH: string;
  readonly ctaBashLabel: string;
  readonly ctaPsLabel: string;
  readonly ctaThen: string;
  readonly ctaGithub: string;
  readonly ctaGitverse: string;
  readonly zapTieHead: string;
  readonly zapTieBody: string;
  readonly zapTieLink: string;

  readonly summaryK: string;
  readonly summaryHtml: string;
};

/**
 * The commands the page prints, identical in both languages.
 *
 * They were inline in the Astro markup rather than in its string table,
 * and they stay untranslated for the reason a shell prompt does: a
 * command is typed, not read. Naming them here is what lets the port
 * prove they came across unchanged.
 */
export const COMMANDS = {
  bash: "curl -fsSL https://vibevm.org/install.sh | bash",
  powerShell: "irm https://vibevm.org/install.ps1 | iex",
  then: "vibe install org.vibevm.world/redbook",
  bashPrompt: "$",
  powerShellPrompt: "PS>",
} as const;

export const STRINGS: Readonly<Record<"en" | "ru", Strings>> = {
  en: {
    eyebrow: "vibe · spec-driven development, packaged",
    headlineHtml: "Discipline you can <em>install</em>.",
    lead: "A coding agent starts every session knowing nothing about your project. VibeVM fixes that the way a package manager fixed missing libraries: your project declares the rule sets it follows, and vibe resolves them, pins them, and computes the exact text an agent reads first. When a discipline improves, you update it like any other dependency.",
    ctaInstall: "Install VibeVM",
    ctaZap: "Why Zap",
    friezeAlt:
      "A suprematist frieze: scattered tilted rectangles of hand-copied conventions on the left; a large terracotta wedge entering from the right; past the wedge, packages aligned into a resolved dependency lattice, pinned at every node.",
    friezeCaption:
      "Left to right: conventions drifting apart, an installed discipline entering, a resolved graph — pinned at every node.",

    problemK: "The problem",
    problemH: "The copy-paste era of agent context",
    problemBody:
      "Instruction files became load-bearing: CLAUDE.md, AGENTS.md, and their cousins now decide how agents behave in your repositories. But that text is still managed like a wiki — copied by hand between projects, edited in place, never versioned as the dependency it actually is.",
    problemPoints: [
      "The same convention lives in nine repos, in nine slowly diverging copies.",
      "A fix to your process ships to exactly one project — the one you happened to be in.",
      "Nothing pins what an agent read yesterday, so nothing can reproduce it today.",
    ],

    thesisK: "The model",
    thesisH: "Two trees, one computed lane",
    thesisBody:
      "vibe holds a hard line between what you write and what it materialises. Your specifications live in vibevm/vibespecs; installed packages are copied byte-exact into vibevm/vibedeps — committed, so a fresh clone reads without a network, without a tool, without knowing vibe exists. Installing a package never edits your text. Removing one never leaves a trace in it.",
    archAlt:
      "Architecture: the authored vibespecs tree and the materialised vibedeps tree both feed a computed boot lane — STATIC.md read first and in full, then the INDEX.md manifest — which any agent consumes by pure file reading.",
    archYoursHead: "vibevm/vibespecs",
    archYoursTag: "yours",
    archYoursBody: "your specs, rules, WAL — installs never edit them",
    archDepsHead: "vibevm/vibedeps",
    archDepsTag: "materialised",
    archDepsBody:
      "packages copied byte-exact, committed — never edited by hand",
    archLaneHead: "the boot lane",
    archLaneTag: "computed",
    archLaneStatic: "STATIC.md — the priority lane, read first and in full",
    archLaneIndex:
      "INDEX.md — a manifest of entries, order derived from the graph",
    archAgentHead: "any agent",
    archAgentBody:
      "Claude Code · Codex · OpenCode · Cursor — pure file reading, no tool in the loop",
    archNote:
      "The reading order is derived, not declared: foundation, then the project, then dependencies in topological order, then your overrides. Nobody numbers a snippet, and no package can fight another for position.",

    tokenK: "Attention economics",
    tokenH: "Every word in the lane is paid for on every session start",
    tokenBody:
      "So the lane holds instructions, not explanations. Reference depth stays in the packages, loaded when a question actually arises. And because a byte-identical lane is shared, the provider cache one agent warms serves the boss and every worker — which is why nothing per-session, no names, ids, or timestamps, may enter it.",
    tokenTag: "Text costs tokens. vibe is built around that cost.",

    capsK: "Capabilities",
    capsH: "What the machinery buys you",
    caps: [
      {
        label: "Reproduce",
        head: "The lock diff is the changelog of what your agent reads",
        body: "vibe.lock records the whole resolved graph — version, registry, and a fingerprint of every file. A fresh clone installs the identical graph, and a pull request that changes the lock shows precisely what your agent will now read.",
      },
      {
        label: "Refuse",
        head: "Integrity that stops before writing",
        body: "If a source serves different bytes under a known version — a force-pushed tag, a compromised mirror — vibe refuses before writing anything, naming the fingerprint it expected. Offline mode never touches the network, and a miss is a hard, named error, never a silent partial install.",
      },
      {
        label: "Address",
        head: "Corrections in twenty tokens",
        body: "Specs are cited as spec:// addresses with immutable anchors. “Fix spec://…#RETRY-COUNT” lands on one line; errors name the violated rule as an address, and vibe explain prints it. Renames leave tombstones, so an address you cited keeps resolving.",
      },
      {
        label: "Trust",
        head: "No model inside",
        body: "Every subsystem has a complete algorithmic mode: vibe check is a deterministic linter, and resolution is pure computation. When an operation needs reasoning, vibe parks an instruction in a relay mailbox for your agent to drain. Holding an API key activates nothing.",
      },
    ],

    howK: "How it works",
    howH: "Five commands to a booted agent",
    steps: [
      {
        cmd: "curl -fsSL https://vibevm.org/install.sh | bash",
        body: "One command installs vibe, vibe-index, and the matching source tree. On Windows: irm https://vibevm.org/install.ps1 | iex.",
      },
      {
        cmd: "vibe init hello-vibe",
        body: "An idempotent scaffold: manifest, lock, boot files, and managed blocks in CLAUDE.md, AGENTS.md, GEMINI.md — vibe only ever writes between its own markers.",
      },
      {
        cmd: "vibe install org.vibevm.world/redbook --path hello-vibe",
        body: "Resolves the discipline and its dependencies, shows the plan — which packages, which files — and writes nothing until you confirm.",
      },
      {
        cmd: "vibe tree --plain --path hello-vibe",
        body: "Prints the computed reading list: exactly what an agent will read at session start, package by package. vibe check verifies the project deterministically.",
      },
      {
        cmd: "claude",
        body: "Or codex, opencode, cursor — any agent that can read a file boots from the same lane. The agent never runs vibe to start.",
      },
    ],

    fitK: "Fit",
    fitH: "Where it fits — and where it does not",
    fitForHead: "Reach for VibeVM when",
    fitFor: [
      "the same process discipline should hold across many repositories, and drift is already costing you reviews;",
      "several agents — or agents plus humans — work one codebase and need one boot context, byte-identical for each of them;",
      "you want your agent corrected by address, not by paraphrase, and your context changes reviewed like code.",
    ],
    fitNotHead: "It is not",
    fitNot: [
      "an agent — there is no model inside, and nothing activates because a key exists;",
      "a replacement for npm or cargo — it manages specs and process discipline, not runtime dependencies;",
      "a wiki — the lane is deliberately short, because text costs tokens on every session start;",
      "a hosted service — everything runs locally, and a registry is just a git hosting organization.",
    ],
    statusHead: "Status, plainly",
    statusBody:
      "The current release is 1.0.0 — a closed alpha, not a compatibility promise: it will break while public = false, and the recovery path is re-init and re-fetch rather than migrations. Open source under UPL-1.0.",

    ctaK: "Start",
    ctaH: "Install it, then install a discipline",
    ctaBashLabel: "Linux · macOS · WSL",
    ctaPsLabel: "Windows PowerShell",
    ctaThen: "Then, in a project:",
    ctaGithub: "View on GitHub",
    ctaGitverse: "Browse on GitVerse",
    zapTieHead: "Then meet Zap",
    zapTieBody:
      "The first product of the VibeVM family: projects, agent conversations, durable questions, and Git worktrees — on one local map.",
    zapTieLink: "Why Zap",

    summaryK: "In one paragraph",
    summaryHtml:
      "VibeVM treats the context coding agents boot from as a dependency: process disciplines, specs, and skills installed as versioned packages, pinned by content fingerprint, materialised into a committed tree, and compiled into a boot lane any agent consumes by pure file reading. Closed alpha, open source under UPL-1.0: <code>curl -fsSL https://vibevm.org/install.sh | bash</code>.",
  },
  ru: {
    eyebrow: "vibe · spec-driven development, в пакетах",
    headlineHtml: "Дисциплина, которую можно <em>установить</em>.",
    lead: "Кодовый агент начинает каждую сессию, не зная о проекте ничего. VibeVM чинит это так же, как пакетный менеджер починил недостающие библиотеки: проект объявляет, каким наборам правил он следует, а vibe резолвит их, закрепляет и вычисляет точный текст, который агент читает первым. Когда дисциплина улучшается, вы обновляете её как любую другую зависимость.",
    ctaInstall: "Установить VibeVM",
    ctaZap: "Почему Zap",
    friezeAlt:
      "Супрематический фриз: слева — разбросанные наклонённые прямоугольники скопированных вручную конвенций; справа входит большой терракотовый клин; за клином пакеты выстроены в разрешённую решётку зависимостей, закреплённую в каждом узле.",
    friezeCaption:
      "Слева направо: расползающиеся конвенции, входящая установленная дисциплина, разрешённый граф — закреплённый в каждом узле.",

    problemK: "Проблема",
    problemH: "Эпоха copy-paste агентного контекста",
    problemBody:
      "Инструкционные файлы стали несущими: CLAUDE.md, AGENTS.md и их родня теперь решают, как агенты ведут себя в ваших репозиториях. Но этот текст всё ещё ведётся как вики — копируется руками между проектами, правится на месте и не версионируется, хотя фактически он давно зависимость.",
    problemPoints: [
      "Одна и та же конвенция живёт в девяти репозиториях — девятью медленно расходящимися копиями.",
      "Улучшение процесса доезжает ровно до одного проекта — того, в котором вы сейчас оказались.",
      "Ничто не фиксирует, что агент читал вчера, — значит, ничто не воспроизведёт это сегодня.",
    ],

    thesisK: "Модель",
    thesisH: "Два дерева, одна вычисленная полоса",
    thesisBody:
      "vibe держит жёсткую границу между тем, что пишете вы, и тем, что материализует он. Ваши спецификации живут в vibevm/vibespecs; установленные пакеты копируются байт-в-байт в vibevm/vibedeps — и коммитятся, чтобы свежий клон читался без сети, без инструмента и без знания о том, что vibe существует. Установка пакета никогда не редактирует ваш текст. Удаление не оставляет в нём следа.",
    archAlt:
      "Архитектура: авторское дерево vibespecs и материализованное дерево vibedeps питают вычисленную boot-полосу — STATIC.md читается первым и целиком, затем манифест INDEX.md — которую любой агент потребляет чистым чтением файлов.",
    archYoursHead: "vibevm/vibespecs",
    archYoursTag: "ваше",
    archYoursBody: "ваши спеки, правила, WAL — установка их не редактирует",
    archDepsHead: "vibevm/vibedeps",
    archDepsTag: "материализовано",
    archDepsBody:
      "пакеты скопированы байт-в-байт, закоммичены — руками не правятся",
    archLaneHead: "boot-полоса",
    archLaneTag: "вычислено",
    archLaneStatic:
      "STATIC.md — приоритетная полоса, читается первой и целиком",
    archLaneIndex: "INDEX.md — манифест записей, порядок выведен из графа",
    archAgentHead: "любой агент",
    archAgentBody:
      "Claude Code · Codex · OpenCode · Cursor — чистое чтение файлов, без инструмента в цикле",
    archNote:
      "Порядок чтения выводится, а не декларируется: foundation, затем проект, затем зависимости топологически, затем ваши переопределения. Авторы не нумеруют сниппеты, и два пакета не могут подраться за позицию.",

    tokenK: "Экономика внимания",
    tokenH: "Каждое слово полосы оплачивается на каждом старте сессии",
    tokenBody:
      "Поэтому полоса несёт инструкции, а не объяснения. Справочная глубина остаётся в пакетах и загружается, когда вопрос действительно возник. А поскольку байт-идентичная полоса общая, кэш провайдера, прогретый одним агентом, обслуживает и босса, и всех воркеров — вот почему в неё не может попасть ничто посессионное: ни имена, ни id, ни таймстемпы.",
    tokenTag: "Текст стоит токенов. vibe построен вокруг этой цены.",

    capsK: "Возможности",
    capsH: "Что эта механика даёт",
    caps: [
      {
        label: "Воспроизводимость",
        head: "Diff lock-файла — это changelog того, что прочитает агент",
        body: "vibe.lock записывает весь разрешённый граф: версию, реестр и отпечаток каждого файла. Свежий клон устанавливает идентичный граф, а pull request, меняющий lock, показывает ровно то, что агент теперь будет читать.",
      },
      {
        label: "Отказ",
        head: "Целостность, которая останавливает до записи",
        body: "Если источник отдал другие байты под известной версией — force-push тега, скомпрометированное зеркало — vibe отказывает до записи чего-либо, называя ожидаемый отпечаток. Офлайн-режим не трогает сеть вообще, а промах — жёсткая именованная ошибка, никогда не тихая частичная установка.",
      },
      {
        label: "Адресация",
        head: "Поправка за двадцать токенов",
        body: "Спеки цитируются адресами spec:// с неизменяемыми якорями. «Почини spec://…#RETRY-COUNT» попадает в одну строку; ошибки называют нарушенное правило адресом, а vibe explain печатает его. Переименования оставляют tombstone — процитированный адрес продолжает резолвиться.",
      },
      {
        label: "Доверие",
        head: "Внутри нет модели",
        body: "У каждой подсистемы есть полный алгоритмический режим: vibe check — детерминированный линтер, резолюция — чистое вычисление. Когда операции нужно рассуждение, vibe паркует инструкцию в relay-ящик, который опустошает ваш агент. Наличие API-ключа не включает ничего.",
      },
    ],

    howK: "Как это работает",
    howH: "Пять команд до загруженного агента",
    steps: [
      {
        cmd: "curl -fsSL https://vibevm.org/install.sh | bash",
        body: "Одна команда ставит vibe, vibe-index и соответствующее дерево исходников. На Windows: irm https://vibevm.org/install.ps1 | iex.",
      },
      {
        cmd: "vibe init hello-vibe",
        body: "Идемпотентный каркас: манифест, lock, boot-файлы и managed-блоки в CLAUDE.md, AGENTS.md, GEMINI.md — vibe пишет только между собственными маркерами.",
      },
      {
        cmd: "vibe install org.vibevm.world/redbook --path hello-vibe",
        body: "Резолвит дисциплину с зависимостями, показывает план — какие пакеты, какие файлы — и ничего не пишет до подтверждения.",
      },
      {
        cmd: "vibe tree --plain --path hello-vibe",
        body: "Печатает вычисленный список чтения: ровно то, что агент прочитает на старте сессии, пакет за пакетом. vibe check детерминированно проверяет проект.",
      },
      {
        cmd: "claude",
        body: "Или codex, opencode, cursor — любой агент, умеющий читать файлы, загружается из одной и той же полосы. Агент не запускает vibe, чтобы стартовать.",
      },
    ],

    fitK: "Границы",
    fitH: "Где это уместно — и чем это не является",
    fitForHead: "Берите VibeVM, когда",
    fitFor: [
      "одна дисциплина процесса должна держаться в десятке репозиториев, а дрейф уже стоит вам ревью;",
      "несколько агентов — или агенты вместе с людьми — работают в одной кодовой базе и нуждаются в одном boot-контексте, байт-идентичном для каждого;",
      "вы хотите поправлять агента адресом, а не пересказом, и ревьюить изменения контекста как код.",
    ],
    fitNotHead: "Это не",
    fitNot: [
      "агент — внутри нет модели, и ничего не включается от наличия ключа;",
      "замена npm или cargo — управляются спеки и дисциплина процесса, а не runtime-зависимости;",
      "вики — полоса намеренно короткая, потому что текст стоит токенов на каждом старте;",
      "хостинг-сервис — всё работает локально, а реестр — это просто git-хостинг организация.",
    ],
    statusHead: "Статус, прямо",
    statusBody:
      "Текущий релиз — 1.0.0, закрытая альфа, а не обещание совместимости: он будет ломаться, пока public = false, и путь восстановления — re-init и повторная выборка, а не миграции. Открытый код под UPL-1.0.",

    ctaK: "Старт",
    ctaH: "Установите vibe — затем установите дисциплину",
    ctaBashLabel: "Linux · macOS · WSL",
    ctaPsLabel: "Windows PowerShell",
    ctaThen: "Затем, в проекте:",
    ctaGithub: "Открыть на GitHub",
    ctaGitverse: "Открыть на GitVerse",
    zapTieHead: "А затем — Zap",
    zapTieBody:
      "Первый продукт семьи VibeVM: проекты, переписка агентов, устойчивые вопросы и Git-worktree — на одной локальной карте.",
    zapTieLink: "Почему Zap",

    summaryK: "Одним абзацем",
    summaryHtml:
      "VibeVM обращается с контекстом, из которого загружаются кодовые агенты, как с зависимостью: дисциплины процесса, спеки и скиллы ставятся версионируемыми пакетами, закрепляются отпечатком содержимого, материализуются в коммитимое дерево и компилируются в boot-полосу, которую любой агент потребляет чистым чтением файлов. Закрытая альфа, открытый код под UPL-1.0: <code>curl -fsSL https://vibevm.org/install.sh | bash</code>.",
  },
};
