# Пакет NS-O1 — страница News & support и пункт в верхнем меню (opus5, High) — 2026-09-26

##subagent-quiet-clause
«Ты работаешь в СУБАГЕНТСКОМ режиме: твой экранный текст не читает никто,
деливерабл — только артефакты. НЕ пиши на экран ничего сверх предписанного
заданием. Предписанное ОБЯЗАТЕЛЬНО и не отменяется этой клаузой:
heartbeat'ы `echo "PROGRESS: …"` перед каждым шагом, файл отчёта
`campaigns/docs-2026-09/findings/WORKER-REPORT-NS-O1.md` (решения,
отклонения, вывод самопроверки), финальный `echo "TASK-DONE"`. Запрещено:
приветствия, пересказ задачи, промежуточные рассуждения в чат, финальное
резюме сделанного (оно живёт в отчёте, не в чате).»

## Где ты работаешь

- Твоё рабочее дерево — отдельный git worktree от `main` (твой текущий
  каталог). В основном дереве `C:\Users\olegc\git\v\vibevm` параллельно
  работает другой исполнитель (глоссарий: `reader/`, `design/components/`,
  фикстуры); туда не пиши. Git Bash; Write/Edit; UTF-8, LF. **Git только
  читающий: ничего не коммить** — центральная сессия заберёт дифф твоего
  дерева.
- Пакет сайта: `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/` от корня
  твоего дерева (ниже — `web/`).
- `node_modules` пакета сайта в worktree нет. В
  `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0` своего дерева выполни
  `pnpm install --frozen-lockfile --offline` (из локального хранилища pnpm,
  без сети, за секунды). Junction на основной `node_modules` НЕ годится:
  ссылка на `@vibe-docs/design` в нём ведёт в `design/` основного дерева
  (поправка 2026-09-26, отправлена исполнителю сообщением).
- Порт e2e `4173` занимают прогоны другого исполнителя. Для своих e2e
  сделай копию `site/tests/playwright.config.ts` на порту 4373 вне
  отслеживаемых файлов (или удали её в конце) и гоняй весь набор с ней.
- Не трогай файлы, которые правит исполнитель глоссария
  (`web/site/src/reader/**`, `web/design/src/components/prose/**`,
  `glossary-card`, фикстуры острова); `web/specmap.json` перегенерируй в
  своём дереве — центральная сессия сведёт его при слиянии.

## Зачем

Владелец: нужна страница «News & support» со ссылками на каналы проекта, и
ссылка на неё — в верхнем меню, в первом ряду, самым правым пунктом.

## Прочитай первым

1. `web/site/src/landing/chrome.tsx` (два ряда меню: первый —
   Documentation, GitHub, GitVerse; второй — Vision и страницы Why; как
   страница отмечает себя текущей), `web/site/src/landing/i18n.ts`,
   `web/site/src/landing/head.ts` (canonical, hreflang, путь внутри локали).
2. Образец отдельной страницы с лейаутом лендинга — Vision:
   `web/site/src/routes/vision/`, `web/site/src/routes/ru/vision/`,
   `web/site/src/vision/**`; страницы Why — `web/site/src/why/**`
   (`paths.ts`, `meta.ts`, `shared.css`) как образец стиля карточек.
3. Как лендинговые адреса попадают в sitemap и в проверки ссылок (ищи, где
   перечислены `vision` и `why/…`: `web/site/src/seo/`, `web/tools/`).
4. Тесты: `web/site/tests/why.spec.ts` (текущая страница в шапке, двойник
   на другом языке, адрес, фокус), `chrome.spec.ts`.
5. Правила стека TypeScript:
   `vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang/1.0.0/vibevm/vibespecs/boot/20-stack-typescript-ai-native-lang.xml`.

## Что сделать

### 1. Адрес и меню

- Страница по адресам `/news-and-support/` и `/ru/news-and-support/`, на
  лейауте лендинга, как Vision.
- Пункт меню в ПЕРВОМ ряду, самым правым, после GitVerse: EN «News &
  support», RU «Новости и поддержка»; ведёт на страницу в языке, который
  читают; на самой странице отмечен текущим (`aria-current="page"`), как
  страницы второго ряда.
- Метаданные: заголовок, описание, canonical, hreflang-пара и `x-default`,
  структурированные данные — как у соседних страниц; адрес в sitemap.

### 2. Страница (текст владельца — дословно; разметка и стиль — твои)

Стиль — язык страниц лендинга (токены, шрифты, карточки как на Why); без
новых цветов; без логотипов сторонних сервисов (платформа — словом, мелкой
моноширинной меткой). Каждая карточка целиком — ссылка наружу
(`rel="noopener"`), с видимым фокусом; адрес ссылки виден в карточке
(`t.me/vibevm` и т. п.). Три группы, в таком порядке.

**EN**

- Заголовок страницы: `News & support`
- Лид: `Where VibeVM posts its news, where to ask for help and report a bug,
  and where to talk about everything else.`
- Группа `News`:
  - `VibeVM News` · метка `Telegram channel` · `https://t.me/vibevm` ·
    `Releases and announcements.`
  - `Oleg Chirukhin` · метка `X` · `https://x.com/1red2black` · `The creator
    of VibeVM.`
- Группа `Support`:
  - `VibeVM Chat` · метка `Telegram chat` · `https://t.me/vibevm_chat` ·
    `Support and bug reports.`
  - `r/vibevm` · метка `Reddit` · `https://www.reddit.com/r/vibevm/` · `The
    VibeVM community on Reddit.`
- Группа `Conversation`:
  - `1red2black chat` · метка `Telegram chat` ·
    `https://t.me/chat_1red2black` · `Off-topic and general discussion.`

**RU**

- Заголовок: `Новости и поддержка`
- Лид: `Где VibeVM публикует новости, где попросить помощи и сообщить об
  ошибке — и где поговорить обо всём остальном.`
- Группа `Новости`:
  - `VibeVM News` · `Канал в Telegram` · `https://t.me/vibevm` · `Выпуски и
    анонсы.`
  - `Олег Чирухин` · `X` · `https://x.com/1red2black` · `Создатель VibeVM.`
- Группа `Поддержка`:
  - `VibeVM Chat` · `Чат в Telegram` · `https://t.me/vibevm_chat` ·
    `Поддержка и баг-репорты.`
  - `r/vibevm` · `Reddit` · `https://www.reddit.com/r/vibevm/` ·
    `Сообщество VibeVM на Reddit.`
- Группа `Разговоры`:
  - `Чат 1red2black` · `Чат в Telegram` · `https://t.me/chat_1red2black` ·
    `Флуд и общие обсуждения.`

Описание страницы для метаданных — EN `News, support and discussion for
VibeVM: the Telegram news channel, the support chat, Reddit, and the
creator on X.`; RU `Новости, поддержка и обсуждения VibeVM: новостной
канал в Telegram, чат поддержки, Reddit и создатель в X.`

### 3. Тесты

- e2e: обе страницы отвечают; в первом ряду шапки пункт стоит последним и
  ведёт на страницу своего языка на лендинге, на страницах Why и Vision; на
  самой странице отмечен текущим; двойник на другом языке; пять ссылок с
  точными `href` и `rel="noopener"`; видимый фокус у карточки; нет
  горизонтального скролла на 390, 834, 1440. Существующие тесты шапки,
  которые считают пункты первого ряда, поправь явно.

## Периметр

Можно: `web/site/src/**`, `web/site/tests/**`, `web/design/src/components/**`
(только если без нового компонента не обойтись), `web/tools/**` (списки
адресов для parity/sitemap, если они там), `web/specmap.json`
(перегенерируй: `C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe specmap --path <web>`),
отчёт. Нельзя: `crates/**`, спеки, пакеты руководства, `BACKLOG.md`,
корневой `specmap.json`. Нужно больше — стоп и «дефект пакета» в отчёте.

## Самопроверка (вывод и коды выхода — в отчёт дословно)

```bash
cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0
TYPESCRIPT_AI_NATIVE="C:/Users/olegc/git/v/vibevm/target/debug/typescript-ai-native.exe" node tools/floor.mjs --keep-going; echo "EXIT=$?"
node design/audit/contrast.mjs; echo "EXIT=$?"
node tools/build.mjs static; echo "EXIT=$?"
node node_modules/@playwright/test/cli.js test -c <твоя копия конфига на порту 4373>; echo "EXIT=$?"
node tools/build.mjs embedded; echo "EXIT=$?"
```

## Отчёт

`WORKER-REPORT-NS-O1.md`: файлы; адреса и меню; как устроена страница;
sitemap и метаданные; вывод самопроверки дословно; отклонения. Затем
`echo "TASK-DONE"`.
