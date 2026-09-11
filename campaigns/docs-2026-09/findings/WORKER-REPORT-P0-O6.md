# WORKER-REPORT-P0-O6 — дисциплина TypeScript для web-пакета (спайк A0.14)

Дата: 2026-09-11. Дерево: `C:\Users\olegc\git\v\vibevm-docs`, b1291b06.
Находка: `findings/A0.14-ts-floor.md`.
Scratch: `<scratch>/vibe-docs-phase0\P0-O6\`
(копии `ts-demo`, зеркало раскладки репозитория, Qwik-проба, все логи и
скрипты прогонов).

## Решения

1. **Сборку инструментов вёл через `vibe bin build`, как велит пакет**, но
   с `--assume-yes`: без флага команда отказывает (группа
   `org.vibevm.ai-native` не в allow-list). Пакет разрешает сборку явно, флаг
   — её механическая форма, не расширение полномочий.
2. **Проверил не один маршрут запуска, а четыре** (`vibe bin exec`, готовый
   бинарник слота из корня пакета, он же с `--path`, `cargo run
   --manifest-path`), потому что первый оказался с тремя предусловиями и для
   web-пакета внутри репозитория заведомо дороже остальных. Все четыре
   доведены до кода выхода 0.
3. **Для честного вердикта по `ts-demo` построил зеркало раскладки
   репозитория.** Первая копия в scratch ломала `file:`-зависимость
   eslint-плагина (относительный путь уходил в никуда), и красный шаг
   `eslint` был бы артефактом копирования. В зеркале
   (`…/P0-O6/mirror/research/ts-demo` + копия
   `vibevm/vibepacks/.../tools/eslint-plugin-ai-native` на том же
   относительном расстоянии) `npm ci` даёт ровно репозиторную картину; она и
   записана в находку.
4. **Вопрос «как floor проверяет флаги tsconfig» решал экспериментом, а не
   чтением.** Подменил `tsconfig.json` копии на ослабленный и прогнал
   `floor --keep-going`; плюс поиск имён флагов по исходникам слота. Вывод:
   механической проверки нет, ответ отрицательный и подтверждён с двух
   сторон.
5. **Qwik проверял на живом пакете, а не по документации.** Поставил
   `@qwik.dev/core@latest` (2.0.0-beta.43) и `typescript@6.0.3` в scratch,
   написал два компонента (`component$`, `useSignal`, `useStore`, `Slot`,
   брендированный проп, чтение `process.env` в корне), собрал объединённый
   `tsconfig` из стартера апстрима и флагов дисциплины, прогнал `tsc`, затем
   `typescript-ai-native init`, `specmap` и полный floor по `.tsx`-дереву.
   Базовый `tsconfig` Qwik взят WebFetch'ем из
   `QwikDev/qwik@main:starters/apps/base/tsconfig.json`, потому что находки
   P0-O5 (`findings/A0.10-qwik-probe.md`) на момент работы не существовало.
6. **Записал три `REVIEW:`** (не зелёный `ts-demo`, устаревшая схема его
   `vibe.lock`, его же `tsconfig` ниже пола GUIDE §1). Решений не менял и
   обходов не изобретал; в scratch-копиях правки делал только чтобы увидеть
   все семь шагов и зафиксировать рецепт.

## Отклонения

- **`--assume-yes` к `vibe bin build`** — см. решение 1. Без флага команда
  не выполняется вовсе.
- **Правки в scratch-копиях** (`prettier --write src/main.ts`, `npm install`
  внутри копии eslint-плагина, `@scope` в шапке `src/main.ts` + перевыпуск
  `specmap.json`, временная подмена `tsconfig.json` и `conform.toml`) — все
  во временном каталоге, ни одного байта в репозиторий и в
  `vibevm/vibepacks/`. Рабочее дерево осталось чистым (см. самопроверку).
- **`vibe install` в scratch-копии `ts-demo`** — понадобился, чтобы
  довести маршрут `vibe bin exec` до конца (lockfile схемы 7 + слот пакета).
  Выполнен `--offline` из репозиторного реестра, только в scratch.
- **Сборка в `vibevm/vibedeps/.../1.0.0/target/`** внутри рабочего дерева —
  это то, что делает `vibe bin build` по устройству; каталог покрыт
  `.gitignore` (`**/target/`), статус дерева не изменился.
- Пакет предполагал возможность «`vibe bin` не знает стек» — не
  подтвердилось: `vibe bin list` показывает все 10 объявленных бинарников,
  включая пять typescript-овых. Маршрут `cargo run --manifest-path` всё
  равно проверен, как и просил пакет.

## Вывод самопроверки

Команда floor, которая реально сработала, на копии `ts-demo`
(`…\P0-O6\ts-demo`, каталог — корень пакета):

```
$ "…/vibevm/vibedeps/org.vibevm.ai-native.typescript-ai-native-lang/1.0.0/target/release/typescript-ai-native.exe" floor --quiet
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
FLOOR_EXIT=0
```

Полный vibe-нативный маршрут из корня того же пакета:

```
$ vibe bin build typescript-ai-native --assume-yes
BUILD_EXIT=0 ELAPSED_SECS=23
$ vibe bin exec typescript-ai-native -- floor
EXEC_EXIT=0 ELAPSED_SECS=5
floor: all green (7 step(s) run, 0 disabled by policy).
```

Файл находки:

```
$ ls -l C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.14-ts-floor.md
-rw-r--r-- 1 olegc 197121 29905 Sep 11 23:16 C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.14-ts-floor.md
LS_EXIT=0
```

Состояние рабочего дерева:

```
$ cd C:/Users/olegc/git/v/vibevm-docs && git status --short
 M .claude/agents/opus5.md
```

(единственная строка — та, что пакет назвал чужой).

## Что не сделано и почему

- **`vite build` Qwik не запускался**: проверен только типовой слой
  (`tsc --noEmit`). Совместимость `verbatimModuleSyntax` /
  `erasableSyntaxOnly` со сборкой и оптимизатором Qwik остаётся открытой
  (открытый вопрос 1 в находке).
- **`@qwik.dev/router` не ставился**: пакет спрашивал про `tsconfig`, а не
  про маршрутизацию; типы роутера под объединённым конфигом не проверены.
- **Резолв `@org.vibevm/eslint-plugin-ai-native` починен только в scratch**
  (`npm install` внутри копии плагина). Коммитируемого решения не предлагаю:
  это правка в `vibevm/vibepacks/`, то есть работа за пределами фазы 0.
- **`health`, `fast-loop`, `tcg` на `.tsx` не проверялись** — вне вопросов
  пакета.
- **Сообщения `deviates` для ослабленного флага не наблюдал**: механизма в
  коде нет (см. §3 находки), так что и сообщения не существует; записан
  отрицательный результат с доказательствами.
