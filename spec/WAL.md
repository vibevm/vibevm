# WAL — Project Continuation State {#root}

_Updated: 2026-08-14 (**Ф0 закрыта целиком, Ф1.1 села** — три спайка исполнены и
приняты, два дефекта плана исправлены, `snapshot → copy` доведён до конца, реестр
форматов построен и `FormatId` генерируется из него. Всё раскатано, панель
зелёная, долг 0/0. Сворачивание №20: `CONTINUE.md` и `NEXT-SESSION-PROMPT.md`
переписаны на вход **Ф1.2**.)_

@fact:WAL-READ-THE-PROMPT-FIRST **Порядок работы — `NEXT-SESSION-PROMPT.md`**:
boot → четыре документа предмета целиком → короткий доклад → **сразу
исполнение**. Главная полоса Ф0→Ф6; независимая S1→S3→S6. Промт есть слово
владельца, отдельного «да» ждать не нужно. @status:impl/done

@fact:WAL-THE-LAW **PROP-044 — ратифицированный стоячий закон каждого формата**
(2026-08-13). Терминология §2b обязательна: snapshot ↔ frozen — антонимы;
«канал» — авторский указатель версий (PROP-005 §2.18); capture/named ref —
провайдерское; режим материализации — `copy`. @status:impl/done

@fact:WAL-THE-PLANS **Три плана строек, взаимно сцеплены:**
[`TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](../campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md)
(главная стройка Ф0–Ф6; **Ф0 закрыта, Ф1.1 села, следующий шаг — Ф1.2**),
[`TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md`](../campaigns/packages-2026-09/TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md)
(S1/S3/S6 стартуемы сразу; S2 — СТОП-ВЛАДЕЛЕЦ; S4/S5 заперты до Ф3; S7 —
судейская),
[`TZ-CHANGE-NATIVE-WAVE2-v0.1.md`](../campaigns/packages-2026-09/TZ-CHANGE-NATIVE-WAVE2-v0.1.md)
(W1–W4 заперты до посадки Ф1–Ф5). @status:impl/done

@fact:WAL-NUMBERS-COME-FROM-COMMANDS **Числа воспроизводятся командами; сперва
rescan, иначе они отвечают про кэш:** @status:impl/done

```bash
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask mirror --check
```

## Current phase {#current-phase}

@fact:WAL-PHASE **Change-native стройка, ТЗ №1: Ф0 ЗАКРЫТА, Ф1.1 СЕЛА.
Следующий шаг — Ф1.2 (поле `epoch` в `vibe.toml`).** Порядок внутри Ф1 жёсткий:
Ф1.2 → Ф1.3 → Ф1.4 → Ф1.5, каждый шаг отдельным коммитом. @status:impl/done

@fact:WAL-STATE **Состояние на чекпойнте** (команды главнее): `main` чист, HEAD
— сворачивание №20; зеркала синхронны (`mirror --check` — gitverse sync, github sync);
панель зелёная (полный прогон на слайсе Ф1.1); корпус 281 файл, 0
неразмеченных; долг **0 неосуждённых, 0 осиротевших**, 33 stale (стоячий долг —
кампания S7). @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-F12 **Ф1.2 — эпоха в манифесте (решение D5):** поле
   `epoch: Option<u32>` в `[package]` (`crates/vibe-core/src/manifest/package.rs`,
   рядом с `frozen` из Ф1.4); отсутствие = состояние «до эпох», НЕ «эпоха 1»;
   `vibe init` и шаблоны пишут `epoch = 1`; `vibe check` сообщает отсутствие как
   info. Замороженный ридер №0 — Приложение Б.4. @status:spec/plan
2. @fact:WAL-NEXT-F13-TRAPS **Ф1.3 несёт две ловушки, найденные до старта**
   (детали — `##WAL-KI-HASH-RECIPE-TRAPS`): третий участник `ContentHash` и
   golden-фикстура, которая не ловит то, ради чего нужна. @status:spec/plan
3. @fact:WAL-NEXT-OWNER **За владельцем:** S2 (переименовать `_`-репо в
   vibespecs — нужны org-права). Ближайший СТОП по главной полосе — смена вида
   `content_hash` в локфайле (Ф1.3), и он обходится по конструкции плана:
   локфайл продолжает нести старый вид, новый пишет только индекс. @status:spec/plan

## Constraints — do not violate {#constraints}

- @fact:WAL-C-PROP044-IS-THE-LAW **Вся идеология форматов — ратифицированный
  PROP-044**; здесь не пересказывается. Терминология §2b обязательна в каждом
  новом тексте и коде. @status:impl/done
- @fact:WAL-C-JUDGE-SAME-PASS **Новый/правленый спек-файл размечен и осуждён тем
  же заходом**: scan → mirror → batch JSON → `merge-verdicts.py` → `seal` → долг
  0. **`campaigns/**` в судимый корпус НЕ входит** (проверено 2026-08-14) —
  правки планов и находок судейского долга не создают. @status:impl/done
- @fact:WAL-C-HONEST-SEAL **Печать — только за проверенное**: seal файла ручается
  за ВСЕ его вердикты (прецедент отката — `498e8c8b`). @status:impl/done
- @fact:WAL-C-SPECMAP **Правка текста спеки — и переименование тестовой функции с
  `#[verifies(…)]` — двигают карту**: `cargo xtask specmap` в той же посадке;
  `--check` зелёный. @status:impl/done
- @fact:WAL-C-CODEGEN-STAGING **`check-codegen` — это `git diff --exit-code` по
  generated-каталогам**, то есть рабочее дерево против ИНДЕКСА, и untracked-файлы
  он не видит. Слайс, трогающий `generated/**`, садится в порядке
  **стейдж → панель → коммит**: `git add` достаточно, чтобы гейт позеленел. @status:impl/done
- @fact:WAL-C-PHASE-ORDER **Порядок шагов внутри фазы жёсткий**, каждый шаг —
  отдельный коммит; граница ФАЗЫ (не шага) = полная панель + раскатка
  `cargo xtask mirror`. @status:impl/done
- @fact:WAL-C-DELEGATION **Транспорт воркеров**:
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` читает БОСС ЦЕЛИКОМ; режим —
  `SUBAGENT-MODE.toml` (`claudez`, владельцем 2026-08-14 подтверждён); пакеты
  собирает босс; воркеры не запускают git; диффы читает и коммитит босс. §9 файла
  несёт разбор чистого фан-аута — читать перед нарезкой пакетов. @status:impl/done
- @fact:WAL-C-SHELL **Shell-ловушки**: cwd Bash-инструмента персистентен —
  **никогда голый `cd`**, только `( cd … && … )` / `git -C` / абсолютный путь
  (иначе следующие команды молча читают чужое дерево); `cmd | tail; echo $?`
  печатает код ПАЙПА, а не команды — редирект в файл и `$?` сразу; правки только
  editor-инструментами; python — в файл, `PYTHONIOENCODING=utf-8`; бюджет файла
  600 строк ПОСЛЕ `cargo fmt`. @status:impl/done
- @fact:WAL-C-GIT **Git**: никогда `git add -A` по всему дереву — явные пути;
  коммиты heredoc `-F -`; раскатка только `cargo xtask mirror` (fast-forward,
  никогда `--force`); Rules 1–4 связывают каждый коммит. @status:impl/done
- @fact:WAL-C-VENDORED **Вендоренные копии не редактируются**; движки дисциплины
  не трогаются. @status:impl/done
- @fact:WAL-C-AUTONOMY **Промт авторизует исполнение** — остановки только на
  СТОП-ВЛАДЕЛЕЦ, по одной за раз; доклады статусов, не вопросы. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-14, 7 коммитов (`406e0ee4`..`fd817814`):** Ф0 исполнена
тремя делегированными спайками (две линии `claudez`/`claudez2`, 3/3 приняты с
первого прохода) → находки посажены → два дефекта базовой линии плана исправлены
и четыре вопроса Ф1.1 закрыты как «РЕШЕНО» → `snapshot → copy` доведён до конца
(21 место, 14 файлов) → капканы фан-аута записаны в `SUBAGENT-LAUNCHERS.md` §9 →
**Ф1.1**: `formats/REGISTRY.toml` (20 записей) + генерация `FormatId` из него +
тест полноты в обе стороны. Панель зелёная, раскатано. @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Ничего в полёте.** Воркеров нет, worktree'ов нет, дерево
чисто. Следующий шаг начинает свежая нарезка пакета Ф1.2. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-WIREGEN-IS-A-PIPELINE **`vibe-wire-gen` — конвейер, а не текстовый
  фильтр** (находка Ф0.2, `harvest/f0-gen-poc.md` §7): преобразования А.5 №3
  (`x-empty`) и №4 (строгость по роли) требуют входов, которых в сгенерированном
  Rust НЕТ — `metadata."x-empty"` из JTD-схемы и `foreign_parsers` из реестра.
  Форма Ф4.2 — `(схема + реестр + сгенерированный Rust) → Rust`. Плюс:
  тегированные объединения (`RepomdFileEntry`, Ф1.5) этим путём НЕ покрываются —
  нужен отдельный путь кодогенерации; и версия в заголовке выхода генератора
  врёт (`v0.2.1` при бинаре 0.4.1) — пинить надо бинарь. @status:impl/done
- @fact:WAL-KI-HASH-RECIPE-TRAPS **Ф1.3 — две ловушки, найденные боссом заранее.**
  (1) Третий участник, которого план не назвал: `crates/vibe-core/src/content_hash.rs`
  — newtype `ContentHash` с `PREFIX = "sha256:"` и проверкой НА ГРАНИЦЕ
  ДЕСЕРИАЛИЗАЦИИ (`try_from = "String"`); как только индекс начнёт эмитить
  `sha256-tree/1:`, всякий путь через этот тип упадёт. `parse` обязан выучить обе
  формы. Реализаций алгоритма ровно две: `vibe-index/src/content_hash.rs:40` и
  **`vibe-registry/src/shippable.rs:77`** (не `content_hash.rs`, как говорит план).
  (2) Golden-фикстура `golden-flow-wal-0.1.0` НЕ ловит расхождение порядка, ради
  которого нужна: сортировка идёт по `PathBuf` ДО нормализации `\`→`/`, а разница
  проявляется, только когда сосед каталога несёт символ между 0x2F и 0x5C
  (например каталог `spec/` рядом с файлом `specX.md`). Такой пары в фикстуре
  нет — её надо добавить, иначе рецепт 1 замораживается, ничего не доказывая. @status:impl/done
- @fact:WAL-KI-G11-BY-CAP **`by-cap`/`by-purl` публикуются без ридера файла**
  (находка Ф0.3): `inverted.rs` определяет `CapabilityRow:66` и `PurlRow:75`, но
  `pub fn read` в нём нет — тогда как `primary.rs:75` и `by_name.rs:57` его имеют.
  Открытое нарушение G11. Схемы им добавлены в Ф4.1; ридеры даёт Ф4.2. @status:impl/done
- @fact:WAL-KI-F21-CLOCK-SITES **Ф2.1 упрётся в фикстурный конструктор:**
  `Utc::now()` в периметре гейта живёт не только у писателя — `types/entry/mod.rs:167`
  (`VersionEntry::minimal`) штампует `indexed_at`. Пять сайтов всего:
  `index/memory.rs:86`, `:249`, `types/entry/mod.rs:167`, `cli/add.rs:113`,
  `cli/reindex.rs:232`. Решение (параметр `at` против `#[cfg(test)]`) принимается
  в пакете Ф2.1, не оставляется воркеру. @status:impl/done
- @fact:WAL-KI-STALE **33 файла stale** (стоячий долг сдвигов байтов) —
  адресовано кампанией S7 (рулинг №4); до неё живут видимыми. @status:impl/done
- @fact:WAL-KI-B074 **Второй якорь факта в абзаце глотается молча** (B-074) —
  механизм чека не построен. @status:impl/done
- @fact:WAL-KI-B075 **Панель может мигнуть на чистом дереве** (B-075) — читать
  отказ, не перезапускать вслепую. @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Шесть файлов корпуса на `work`** — условие выходного
  гейта Phase E, ждёт рулинга. @status:impl/done
- @fact:WAL-KI-UPSTREAM-JOINER **Юнит `#modules` флоу addressable-specs всё ещё
  говорит «joiner never `.`»** — чинится слайсом S3. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **Холодная сессия читает `CONTINUE.md` →
`NEXT-SESSION-PROMPT.md` → четыре документа предмета (PROP-044 + три ТЗ) →
`SUBAGENT-LAUNCHERS.md` целиком перед фан-аутом** — и берёт каждое число из
команд сверху, после rescan. Этот файл главнее `CONTINUE.md`; ратифицированный
PROP-044 главнее обоих. @status:impl/done
