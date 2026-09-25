/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The copy of `/why/zap`, in both languages.
 *
 * Every value is the owner's, moved byte for byte from the Astro
 * component that carried it (`src/components/why/WhyZap.astro`) — the
 * same exception the landing's string table records, for the same
 * reason: content is authorship, and a port that paraphrased it would
 * be a rewrite wearing a port's name.
 *
 * Two shapes here are worth knowing before touching them. A start step
 * carries EITHER a command or a heading, never both, and the source's
 * markup branched on which — so the type below is a union rather than an
 * object with two optional fields, and the page cannot forget to handle
 * one. And the first step's link is a locale-relative address, which is
 * why it travels as a flag rather than as a string: a path written into
 * a string table is a path nothing rewrites when the base changes.
 *
 * Two changes came later than the port, both on the owner's word of
 * 2026-09-25. The first is the release banner over the hero: its English
 * is the owner's own words, its Russian an adaptation of them rather than
 * a gloss. The second follows from it: there is no preview of Zap, so no
 * line here may say there is one or invite a reader to install it today.
 * The badge, the first button, the status card and the closing paragraph
 * were corrected for that and nothing else, and the start list gained one
 * sentence saying when its commands begin to work. The month the banner
 * names is written once more in machine form, `RELEASE_MONTH`, and the
 * two sit side by side here so that a date moved in one is seen to be
 * moved in the other.
 */

export type Law = {
  readonly rule: string;
  readonly why: string;
};

export type Problem = {
  readonly head: string;
  readonly body: string;
};

/**
 * A step of the start list: a command with a caption, or a heading with
 * one. The first kind may carry a link back to the landing.
 */
export type Step =
  | {
      readonly kind: "cmd";
      readonly cmd: string;
      readonly body: string;
      /** The words of the link back to the landing, when there is one. */
      readonly linkText?: string;
    }
  | {
      readonly kind: "head";
      readonly head: string;
      readonly body: string;
    };

export type Strings = {
  /** The banner over the hero: what is announced, and when. */
  readonly soonLabel: string;
  readonly soonDate: string;

  readonly eyebrow: string;
  readonly headlineHtml: string;
  readonly lead: string;
  readonly badge: string;
  readonly ctaStart: string;
  readonly ctaVibevm: string;
  readonly heroArtAlt: string;
  readonly heroCaption: string;

  readonly problemK: string;
  readonly problemH: string;
  readonly problems: readonly Problem[];

  readonly thesisK: string;
  readonly thesisH: string;
  readonly thesisBody: string;
  readonly archAlt: string;
  readonly archAgentsHead: string;
  readonly archAgentsBody: string;
  readonly archLink1: string;
  readonly archCoreHead: string;
  readonly archCoreBody: string;
  readonly archLink2: string;
  readonly archViewHead: string;
  readonly archViewBody: string;
  readonly archNote: string;

  readonly mapK: string;
  readonly mapH: string;
  readonly mapBody: string;
  readonly readingGoalHead: string;
  readonly readingGoalBody: string;
  readonly readingOrderHead: string;
  readonly readingOrderBody: string;

  readonly capsK: string;
  readonly capsH: string;
  readonly caps: readonly Problem[];

  readonly lawsK: string;
  readonly lawsH: string;
  readonly laws: readonly Law[];

  readonly startK: string;
  readonly startH: string;
  /** When the commands of the start list begin to work. */
  readonly startNote: string;
  readonly steps: readonly Step[];
  readonly statusHead: string;
  readonly statusBody: string;
  readonly familyHead: string;
  readonly familyBody: string;
  readonly familyLink: string;

  readonly summaryK: string;
  readonly summaryHtml: string;
};

/** The prompt and the two commands the start list prints. */
export const COMMANDS = {
  prompt: "$",
  install: "vibe install -g org.vibevm.zap/zap",
  quicklens: "zap-quicklens",
} as const;

/**
 * The month the banner announces, as a `<time datetime>` reads it: the
 * `soonDate` of both languages, in the one form that has no language.
 */
export const RELEASE_MONTH = "2026-10";

export const STRINGS: Readonly<Record<"en" | "ru", Strings>> = {
  en: {
    soonLabel: "Coming soon",
    soonDate: "October 2026",

    eyebrow: "org.vibevm.zap · local-first agent workspace",
    headlineHtml: "Your agents, on <em>one map</em>.",
    lead: "Zap is a local workspace for running coding agents in parallel: projects, conversations, questions, managed work, and Git worktrees on one navigable surface. Zap Wayfinder keeps durable state on your machine; Zap Quick Lens opens it in the browser or Electron. Codex, Claude Code, OpenCode, and Qwen Code connect with the logins you already have.",
    badge: "In development",
    ctaStart: "How it starts",
    ctaVibevm: "Why VibeVM",
    heroArtAlt:
      "An orbital map of one campaign: dotted rings of decomposition around a goal core, work nodes on the rings, one trajectory of verified prerequisites leaving the system, and an expanding ping marking a question that waits for a human.",
    heroCaption:
      "One campaign, two readings: decomposition around the goal, prerequisites along the trajectory. The ping is a question waiting for you.",

    problemK: "The problem",
    problemH: "The dark between terminals",
    problems: [
      {
        head: "The lost question",
        body: "An agent hits an ambiguity and asks — into a terminal that scrolled past an hour ago. In Zap a question is a durable object: it stays pending until answered, cancelled, or expired, and the conversation keeps moving in parallel.",
      },
      {
        head: "The shared checkout",
        body: "Two tasks in one working tree end in stepped-on edits. Zap prepares an owned root worktree per plan and isolated child worktrees per managed task — resume returns to the same worktree, uncommitted edits intact.",
      },
      {
        head: "Exit 0 — then what?",
        body: "A clean exit says a process ended, not that the work is right. Zap records process exit, the typed report, and your acceptance as three separate records — success never skips review.",
      },
    ],

    thesisK: "What Zap is",
    thesisH: "A planning engine with a viewer — not a wrapper",
    thesisBody:
      "Zap ships as two packages under one discipline. The Rust engine — flow:org.vibevm.zap/zap — keeps one typed durable history: intent, obligations, uncertainty, decisions, lowering, agent packets, execution, evidence, holds, recovery. The client — tool:org.vibevm.zap/lens — is Zap Wayfinder, the local service that owns that state, and Zap Quick Lens, the viewer that renders it. The coordinator may replan inside an Owner-defined envelope, and authority and economics gates stand in front of every live effect.",
    archAlt:
      "Architecture: agent CLIs connect over authenticated loopback MCP to Zap Wayfinder, which owns the durable journal; Quick Lens viewers attach to Wayfinder through one-time paired sessions.",
    archAgentsHead: "Your agent CLIs",
    archAgentsBody: "Codex · Claude Code · OpenCode · Qwen Code",
    archLink1: "MCP over authenticated loopback",
    archCoreHead: "Zap Wayfinder",
    archCoreBody:
      "durable journal (SQLite) · questions & inboxes · worktree registry · routing catalog",
    archLink2: "one-time paired session",
    archViewHead: "Zap Quick Lens",
    archViewBody: "browser · Electron · zap-server headless",
    archNote:
      "Two separate channels: agents authenticate to an exact workspace scope, and a UI login never grants agent authority. Everything binds loopback. Opening or closing a viewer neither starts nor stops agents.",

    mapK: "Navigation",
    mapH: "A map that refuses to guess",
    mapBody:
      "The workspace renders your projects as regions on one pan-and-zoom map, and the plan graph has exactly two readings. Missing members, partial relations, and prerequisite cycles are called out on the graph instead of being smoothed into a plausible-looking order — and coordinates never promise a duration the source did not state.",
    readingGoalHead: "Goal structure",
    readingGoalBody:
      "Decomposition placed outward from the one active outcome the source identifies.",
    readingOrderHead: "Work order",
    readingOrderBody:
      "Left-to-right steps built only from verified prerequisites. A stage has no invented internal order.",

    capsK: "Capabilities",
    capsH: "What that buys you",
    caps: [
      {
        head: "Questions that survive restarts",
        body: "Question groups carry choices, multi-select, and free text, with drafts, cancellation, and amendment history. In the recorded acceptance flow, a live agent published a question, the browser answered it, the agent read the answer through a bounded inbox and filed a typed report — and after a Wayfinder restart, all of it was still there.",
      },
      {
        head: "Integration with a gate",
        body: "Promotion into your main line passes a candidate diff, registered checks, and human review under a single writer lease. Conflicts become a managed resolution task. There is no automatic stash, reset, or force-push.",
      },
      {
        head: "Routing you can read",
        body: "Named configurations over your existing accounts, task specializations, and an economy/quality slider. Subscription meters show the observed reading: an unknown value stays unknown, and percentages are never converted into an invented number of tokens.",
      },
      {
        head: "Nothing starts implicitly",
        body: "Registering a project spends zero inference. The coordinator starts when you press Start development. zap-server brings up the whole stack headless without a single model turn.",
      },
    ],

    lawsK: "Trust",
    lawsH: "Rules Zap refuses to break",
    laws: [
      {
        rule: "A completed process is not an accepted result.",
        why: "Exit, typed report, acceptance — three records.",
      },
      {
        rule: "Unknown stays unknown.",
        why: "No invented durations, orders, or token counts.",
      },
      {
        rule: "No demo data in live views.",
        why: "An unavailable planning source renders as unavailable.",
      },
      {
        rule: "No bot author.",
        why: "Commit-producing operations refuse until your Git identity is configured.",
      },
      {
        rule: "Controls tell the truth.",
        why: "The UI shows only what the selected provider adapter actually supports.",
      },
    ],

    startK: "Start",
    startH: "From zero to a running coordinator",
    startNote:
      "The commands below will work once Zap is released in October 2026.",
    steps: [
      {
        kind: "cmd",
        cmd: "vibe install -g org.vibevm.zap/zap",
        body: "One global application through vibe.",
        linkText: "Need vibe first? Install VibeVM",
      },
      {
        kind: "cmd",
        cmd: "zap-quicklens",
        body: "Reuses or starts the local Wayfinder owner and opens a paired browser session. --electron for the desktop shell, zap-server for headless.",
      },
      {
        kind: "head",
        head: "Connect an account you already have",
        body: "Codex, Claude Code, OpenCode, or Qwen Code — Zap uses that application’s protected login. It does not create accounts and does not sell inference.",
      },
      {
        kind: "head",
        head: "Add a project, press Start development",
        body: "Registration is free of inference. The first model turn happens on Start — not before.",
      },
    ],
    statusHead: "Where it is today",
    statusBody:
      "Zap is in development: there is no preview yet, and the release is due in October 2026. It will be a local release: one human, one machine, full journal retained. Gamelens, IDE clients, and multi-person collaboration are named future components — not features of that release. Authoritative planning operations need a configured planning source; a pending source is shown as pending, not simulated.",
    familyHead: "In the VibeVM family",
    familyBody:
      "The engine also installs into a VibeVM project as a flow package, adding its specifications and three skills — zap-draft, zap-state, zap-run. Installation activates nothing until you say so.",
    familyLink: "Why VibeVM",

    summaryK: "In one paragraph",
    summaryHtml:
      "Zap puts projects, agent conversations, questions, managed work, and Git worktrees on one local map. Questions survive restarts, integration passes through your review, and nothing spends a model turn until you press Start. Arriving in October 2026, in the VibeVM family: <code>vibe install -g org.vibevm.zap/zap</code>.",
  },
  ru: {
    soonLabel: "Скоро",
    soonDate: "Октябрь 2026",

    eyebrow: "org.vibevm.zap · локальное рабочее пространство агентов",
    headlineHtml: "Все агенты&nbsp;— на&nbsp;<em>одной карте</em>.",
    lead: "Zap — локальное рабочее пространство для параллельной работы кодовых агентов: проекты, переписка, вопросы, управляемые задачи и Git-worktree на одной навигируемой поверхности. Zap Wayfinder держит устойчивое состояние на вашей машине; Zap Quick Lens открывает его в браузере или Electron. Codex, Claude Code, OpenCode и Qwen Code подключаются с теми логинами, которые у вас уже есть.",
    badge: "В разработке",
    ctaStart: "Как всё начнётся",
    ctaVibevm: "Почему VibeVM",
    heroArtAlt:
      "Орбитальная карта одной кампании: пунктирные кольца декомпозиции вокруг ядра-цели, узлы работы на кольцах, одна траектория проверенных пререквизитов, уходящая из системы, и расходящийся сигнал — вопрос, ожидающий человека.",
    heroCaption:
      "Одна кампания, два прочтения: декомпозиция вокруг цели, пререквизиты вдоль траектории. Сигнал — вопрос, который ждёт вас.",

    problemK: "Проблема",
    problemH: "Темнота между терминалами",
    problems: [
      {
        head: "Потерянный вопрос",
        body: "Агент упирается в неоднозначность и спрашивает — в терминал, который прокрутился час назад. В Zap вопрос — устойчивый объект: он ждёт ответа, отмены или истечения срока, а переписка тем временем продолжается.",
      },
      {
        head: "Общий checkout",
        body: "Две задачи в одном рабочем дереве заканчиваются затёртыми правками. Zap готовит корневой worktree на каждый план и изолированные дочерние worktree на управляемые задачи — возобновление возвращает в тот же worktree, незакоммиченные правки целы.",
      },
      {
        head: "Exit 0 — и что дальше?",
        body: "Чистый код выхода говорит, что процесс завершился, а не что работа верна. Zap записывает выход процесса, типизированный отчёт и вашу приёмку как три отдельные записи — успех не перепрыгивает ревью.",
      },
    ],

    thesisK: "Что такое Zap",
    thesisH: "Движок планирования с вьюером — а не обёртка",
    thesisBody:
      "Zap поставляется двумя пакетами одной дисциплины. Rust-движок — flow:org.vibevm.zap/zap — ведёт одну типизированную устойчивую историю: намерения, обязательства, неопределённость, решения, lowering, пакеты для агентов, исполнение, свидетельства, удержания, восстановление. Клиент — tool:org.vibevm.zap/lens — это Zap Wayfinder, локальный сервис-владелец состояния, и Zap Quick Lens, вьюер, который его отрисовывает. Координатор может перестраивать маршрут внутри конверта, заданного владельцем, а перед каждым живым эффектом стоят шлюзы полномочий и экономики.",
    archAlt:
      "Архитектура: агентские CLI подключаются по аутентифицированному loopback-MCP к Zap Wayfinder, владеющему устойчивым журналом; вьюеры Quick Lens присоединяются к Wayfinder через одноразовые парные сессии.",
    archAgentsHead: "Ваши агентские CLI",
    archAgentsBody: "Codex · Claude Code · OpenCode · Qwen Code",
    archLink1: "MCP по аутентифицированному loopback",
    archCoreHead: "Zap Wayfinder",
    archCoreBody:
      "устойчивый журнал (SQLite) · вопросы и inbox · реестр worktree · каталог маршрутизации",
    archLink2: "одноразовая парная сессия",
    archViewHead: "Zap Quick Lens",
    archViewBody: "браузер · Electron · zap-server без окна",
    archNote:
      "Два раздельных канала: агенты аутентифицируются на точный scope рабочего пространства, а вход в UI не даёт агентских полномочий. Всё слушает loopback. Открытие и закрытие вьюера не запускает и не останавливает агентов.",

    mapK: "Навигация",
    mapH: "Карта, которая не угадывает",
    mapBody:
      "Рабочее пространство отрисовывает проекты регионами на одной карте с панорамированием и зумом, а у графа плана ровно два прочтения. Недостающие элементы, частичные связи и циклы пререквизитов показываются на графе, а не сглаживаются в правдоподобный порядок — и координаты не обещают длительность, которой источник не называл.",
    readingGoalHead: "Goal structure",
    readingGoalBody:
      "Декомпозиция, разложенная наружу от единственной активной цели, которую называет источник.",
    readingOrderHead: "Work order",
    readingOrderBody:
      "Шаги слева направо — только по проверенным пререквизитам. Внутри этапа порядок не выдумывается.",

    capsK: "Возможности",
    capsH: "Что это даёт",
    caps: [
      {
        head: "Вопросы переживают рестарты",
        body: "Группы вопросов несут варианты, множественный выбор и свободный текст — с черновиками, отменой и историей поправок. В записанном приёмочном прогоне живой агент опубликовал вопрос, браузер ответил, агент прочитал ответ через ограниченный inbox и отправил типизированный отчёт — и после рестарта Wayfinder всё это осталось на месте.",
      },
      {
        head: "Интеграция со шлюзом",
        body: "Продвижение в основную ветку проходит через diff кандидата, зарегистрированные проверки и ревью человека под единственной writer-арендой. Конфликты становятся управляемой задачей разрешения. Автоматических stash, reset и force-push нет.",
      },
      {
        head: "Маршрутизация, которую можно прочитать",
        body: "Именованные конфигурации поверх ваших существующих аккаунтов, специализации задач и слайдер экономия/качество. Счётчики подписок показывают наблюдаемое значение: неизвестное остаётся неизвестным, а проценты никогда не превращаются в выдуманное число токенов.",
      },
      {
        head: "Ничего не запускается неявно",
        body: "Регистрация проекта не тратит ни одного вызова модели. Координатор стартует, когда вы нажимаете Start development. zap-server поднимает весь стек без окна — и без единого хода модели.",
      },
    ],

    lawsK: "Доверие",
    lawsH: "Правила, которые Zap не нарушает",
    laws: [
      {
        rule: "Завершённый процесс — не принятый результат.",
        why: "Выход, типизированный отчёт, приёмка — три разные записи.",
      },
      {
        rule: "Неизвестное остаётся неизвестным.",
        why: "Никаких выдуманных длительностей, порядков и чисел токенов.",
      },
      {
        rule: "Никаких демо-данных в живых видах.",
        why: "Недоступный источник планирования показан как недоступный.",
      },
      {
        rule: "Никакого бот-автора.",
        why: "Операции, создающие коммит, отказывают, пока не настроена ваша Git-идентичность.",
      },
      {
        rule: "Контролы говорят правду.",
        why: "UI показывает только то, что выбранный адаптер провайдера реально поддерживает.",
      },
    ],

    startK: "Старт",
    startH: "От нуля до работающего координатора",
    startNote: "Команды ниже заработают с выходом Zap — в октябре 2026 года.",
    steps: [
      {
        kind: "cmd",
        cmd: "vibe install -g org.vibevm.zap/zap",
        body: "Одно глобальное приложение через vibe.",
        linkText: "Нет vibe? Установите VibeVM",
      },
      {
        kind: "cmd",
        cmd: "zap-quicklens",
        body: "Переиспользует или запускает локального владельца Wayfinder и открывает парную сессию браузера. --electron — десктопная оболочка, zap-server — без окна.",
      },
      {
        kind: "head",
        head: "Подключите аккаунт, который у вас уже есть",
        body: "Codex, Claude Code, OpenCode или Qwen Code — Zap использует защищённый логин самого приложения. Он не создаёт аккаунты и не продаёт инференс.",
      },
      {
        kind: "head",
        head: "Добавьте проект и нажмите Start development",
        body: "Регистрация не тратит инференс. Первый ход модели происходит на Start — не раньше.",
      },
    ],
    statusHead: "Где продукт сегодня",
    statusBody:
      "Zap в разработке: превью пока нет, релиз ожидается в октябре 2026 года. Это будет локальный релиз: один человек, одна машина, журнал хранится целиком. Gamelens, IDE-клиенты и совместная работа нескольких людей названы будущими компонентами — это не функции этого релиза. Авторитетные операции планирования требуют настроенного источника; ожидающий источник показан как ожидающий, а не симулируется.",
    familyHead: "В семье VibeVM",
    familyBody:
      "Движок также ставится в VibeVM-проект как flow-пакет, добавляя свои спецификации и три скилла — zap-draft, zap-state, zap-run. Установка ничего не активирует, пока вы не скажете.",
    familyLink: "Почему VibeVM",

    summaryK: "Одним абзацем",
    summaryHtml:
      "Zap собирает проекты, переписку агентов, вопросы, управляемые задачи и Git-worktree на одной локальной карте. Вопросы переживают рестарты, интеграция проходит через ваше ревью, и ни один ход модели не тратится, пока вы не нажали Start. Выходит в октябре 2026 года, в семье VibeVM: <code>vibe install -g org.vibevm.zap/zap</code>.",
  },
};
