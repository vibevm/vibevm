# WAL — Project Continuation State {#root}

_Updated: 2026-08-14 (**Ф1.2 и Ф1.3 сели**; независимой полосой закрыт S3,
измерены S6.1 и S1. Шесть делегированных пакетов, 6/6 приняты, ноль кругов
доработки; три из шести были замерами и опровергли четыре записанных
утверждения. Сворачивание №21: `CONTINUE.md` и `NEXT-SESSION-PROMPT.md`
переписаны на вход **Ф1.4**.)_

@fact:WAL-READ-THE-PROMPT-FIRST **Порядок работы — `NEXT-SESSION-PROMPT.md`**:
boot → документы предмета целиком → короткий доклад → **сразу исполнение**.
Промт есть слово владельца, отдельного «да» ждать не нужно. @status:impl/done

@fact:WAL-THE-LAW **PROP-044 — ратифицированный стоячий закон каждого формата**
(2026-08-13). Терминология §2b обязательна: snapshot ↔ frozen — антонимы;
«канал» — авторский указатель версий (PROP-005 §2.18); capture/named ref —
провайдерское; режим материализации — `copy`. @status:impl/done

@fact:WAL-THE-PLANS **Три плана строек, взаимно сцеплены:**
[`TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](../campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md)
(главная стройка Ф0–Ф6; **Ф0 закрыта, Ф1.1–Ф1.3 сели, следующий шаг — Ф1.4**),
[`TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md`](../campaigns/packages-2026-09/TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md)
(**S3 ЗАКРЫТ**; S1 и S6 измерены и готовы к нарезке; S2 — СТОП-ВЛАДЕЛЕЦ;
S4/S5 заперты до Ф3; S7 — судейская),
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

@fact:WAL-PHASE **Change-native стройка, ТЗ №1: Ф0 ЗАКРЫТА, Ф1.1, Ф1.2 и Ф1.3
СЕЛИ. Следующий шаг — Ф1.4 (слоты `must_understand` / `yanked` / `frozen` /
`tombstone`).** Порядок внутри Ф1 жёсткий: Ф1.4 → Ф1.5, каждый шаг отдельным
коммитом. @status:impl/done

@fact:WAL-STATE **Состояние на чекпойнте** (команды главнее): `main` чист, HEAD
`b2e7973e`; зеркала синхронны (`mirror --check` — gitverse sync, github sync);
панель `self-check: all green` (полный прогон на слайсе Ф1.3); корпус 281 файл,
0 неразмеченных; долг **0 неосуждённых, 0 осиротевших**, 33 stale (стоячий долг —
кампания S7); `vibe check` — 0 errors, 1 warning, 44 info; карта 6051 юнит /
968 рёбер / 0 сирот. @status:impl/done

@fact:WAL-EXPECTED-DIAGNOSTICS **Два ожидаемых числа, которые НЕ дефекты — не
чинить, не удивляться.** *(i)* `vibe check` даёт **44 info** `manifest_epoch` —
это до-эпоховые манифесты под `packages/`, помечаемые намеренно (Ф1.2, решение
D5): счётчик и есть сигнал, который осушит волна кодмода. *(ii)* Одно
**warning** `local_source_freshness` по `org.vibevm.world/addressable-specs` —
следствие правки исходника пакета в S3 при намеренно НЕ запущенном
`vibe install` (§S3 шаг 3); гаснет следующим переизданием пакета.
**Если этих чисел станет больше — это находка, а не шум** (см.
`##WAL-KI-RECIPE0-REGRESSION`). @status:impl/done

## Next {#next}

1. @fact:WAL-NEXT-F14 **Ф1.4 — слоты (решения D7 + D14):** в типы записи
   каталога `must_understand: Vec<String>` (skip если пуст), `yanked: bool`,
   `frozen: bool` (skip если false), `tombstone` на уровне `NameEntry`
   (`Option<Tombstone>{reason, superseded_by}`); в манифест — `frozen:
   Option<bool>` рядом с `epoch` из Ф1.2
   (`crates/vibe-core/src/manifest/package.rs`); писатели проецируют флаг из
   манифеста; читатели учат ОДНО поведение: запись с незнакомым
   `must_understand` — пропуск с warn (полноценный карантин — Ф6.2). @status:spec/plan
2. @fact:WAL-NEXT-MEASURE-FIRST **Перед нарезкой Ф1.4 — замер, как перед Ф1.3.**
   Три пакета этой сессии были замерами; два опровергли по опорной координате,
   третий опроверг два утверждения самого плана и заставил переписать
   конструкцию до нарезки. Спросить дерево дешевле, чем строить по записанному. @status:spec/plan
3. @fact:WAL-NEXT-INDEPENDENT **Независимая полоса готова к нарезке:** S1
   (двухъярусный publish — замер
   [`harvest/s1-publish-tiers.md`](../campaigns/packages-2026-09/harvest/s1-publish-tiers.md))
   и S6 (соединение движков по данным — замер
   [`harvest/s6-conform-artifact.md`](../campaigns/packages-2026-09/harvest/s6-conform-artifact.md)).
   Оба несут по одному вопросу, который решает БОСС при нарезке, а не воркер —
   см. `##WAL-KI-S1-PRIVATE-REPO` и `##WAL-KI-S6-NO-TIMESTAMP`. @status:spec/plan
4. @fact:WAL-NEXT-OWNER **За владельцем:** S2 (переименовать `_`-репо в
   vibespecs — нужны org-права). Ближайший СТОП главной полосы — смена вида
   `content_hash` в локфайле — **ОБОЙДЁН по конструкции**: реестровая копия
   осталась на рецепте 0, локфайл не сдвинулся. @status:impl/done

## Constraints — do not violate {#constraints}

- @fact:WAL-C-PROP044-IS-THE-LAW **Вся идеология форматов — ратифицированный
  PROP-044**; здесь не пересказывается. Терминология §2b обязательна в каждом
  новом тексте и коде. @status:impl/done
- @fact:WAL-C-JUDGE-SAME-PASS **Новый/правленый спек-файл размечен и осуждён тем
  же заходом**: scan → **mirror** → batch JSON → `merge-verdicts.py` → `seal` →
  долг 0. Мержу предшествует `progress mirror`, иначе долг не проявится.
  **`campaigns/**` и `spec/WAL.md` в судимый корпус НЕ входят.** @status:impl/done
- @fact:WAL-C-HONEST-SEAL **Печать — только за проверенное**: seal файла ручается
  за ВСЕ его вердикты, поэтому файл со стоячим stale печатать нельзя, пока не
  проверен каждый двинувшийся факт (`tasks/text-stability.py` называет их
  поимённо; прецедент отката — `498e8c8b`). @status:impl/done
- @fact:WAL-C-SPECMAP **Правка текста спеки и новый код двигают карту**:
  `cargo xtask specmap` в той же посадке; `--check` зелёный. **Новый `.rs`-файл
  обязан нести `specmark::scope!`** — иначе трещотка ловит его pub-функции как
  сирот. @status:impl/done
- @fact:WAL-C-CODEGEN-STAGING **`check-codegen` — это `git diff --exit-code` по
  generated-каталогам**, то есть рабочее дерево против ИНДЕКСА, и untracked-файлы
  он не видит. Слайс, трогающий `generated/**`, садится в порядке
  **стейдж → панель → коммит**. @status:impl/done
- @fact:WAL-C-PHASE-ORDER **Порядок шагов внутри фазы жёсткий**, каждый шаг —
  отдельный коммит; граница ФАЗЫ (не шага) = полная панель + правка карты +
  раскатка `cargo xtask mirror`. @status:impl/done
- @fact:WAL-C-DELEGATION **Транспорт воркеров**:
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` читает БОСС ЦЕЛИКОМ; режим —
  `SUBAGENT-MODE.toml` (`claudez`, владельцем подтверждён 2026-08-14); пакеты
  режет босс; воркеры не запускают git; диффы читает, правит и коммитит босс;
  ревью не делегируется никогда. §8 файла несёт четырнадцать оплаченных уроков,
  §9 — разбор чистого фан-аута. @status:impl/done
- @fact:WAL-C-SHELL **Shell-ловушки**: cwd Bash-инструмента персистентен —
  **никогда голый `cd`**, только `( cd … && … )` / `git -C` / абсолютный путь;
  `cmd | tail; echo $?` печатает код ПАЙПА — редирект в файл и `$?` сразу;
  **правки только editor-инструментами** (heredoc через оболочку съедает
  `\` — оплачено дважды за эту сессию); python — в файл, `PYTHONIOENCODING=utf-8`;
  бюджет файла 600 строк ПОСЛЕ `cargo fmt`. @status:impl/done
- @fact:WAL-C-GIT **Git**: никогда `git add -A` по всему дереву — явные пути;
  коммиты heredoc `-F -`; раскатка только `cargo xtask mirror` (fast-forward,
  никогда `--force`); Rules 1–4 связывают каждый коммит. @status:impl/done
- @fact:WAL-C-VENDORED **Вендоренные копии не редактируются**; движки дисциплины
  не трогаются. @status:impl/done
- @fact:WAL-C-AUTONOMY **Промт авторизует исполнение** — остановки только на
  СТОП-ВЛАДЕЛЕЦ, по одной за раз; доклады статусов, не вопросы. @status:impl/done

## Done (collapsed — see `git log`) {#done}

@fact:WAL-DONE **2026-08-14, 8 коммитов (`2907a679`..`b2e7973e`):** Ф1.2 (поле
`epoch` + ячейка линтера `manifest_epoch` + 44 info) → S3 закрыт и свёрнут в
могильник (закон джойнера стал свойством, а не запретом на символ; PROP-029
`##joiner-why` перестал ждать follow-up) → три замера посажены находками
(`f1-3-hash-blast-radius`, `s6-conform-artifact`, `s1-publish-tiers`) → Ф1.3
(рецепт хэша: `formats/hash_recipes/1.toml`, `sha256-tree/1:`, настоящий
кросс-тест реализаций, фикстура-ловушка) → шесть уроков делегирования в §8 →
чекпойнт и план очищены от опровергнутой посылки. Панель зелёная, раскатано. @status:impl/done

## In progress {#in-progress}

@fact:WAL-INFLIGHT **Ничего в полёте.** Воркеров нет, worktree'ов нет, дерево
чисто. Следующий шаг начинает свежая нарезка замера под Ф1.4. @status:impl/done

## Known issues {#known-issues}

- @fact:WAL-KI-RECIPE0-REGRESSION **Самая дорогая находка сессии, и она о
  верификации, а не о воркерах.** Реализация Ф1.3 сменила порядок рецепта 0 —
  сортировку `Vec<PathBuf>` на сортировку строки, — а `Ord` у `Path`
  **покомпонентный**. Это тихо сдвигает хэш всякого дерева, где имя каталога
  префиксует имя соседа. **Ни один из трёх заслонов сработать не мог**: старый
  golden не имеет такой пары, новый golden морозил только рецепт 1 (решение
  босса, верное по причине и снявшее единственного нужного сторожа), кросс-тест
  остался истинным, потому что обе копии сломались одинаково. Поймал
  ПОТРЕБИТЕЛЬ: `vibe check` ушёл с 0 на 7 предупреждений
  `local_source_freshness`, шесть по нетронутым пакетам, и вернулся к 1 после
  починки. **Правила отсюда: фикстура, построенная упражнять расхождение,
  доказывается против СОХРАНЯЕМОГО поведения, а не против правки; «заморозить
  только новое значение» — точка, где гарантия теряет сторожа; счётчики
  предупреждений панели — доказательство, а не шум.** @status:impl/done
- @fact:WAL-KI-HASH-RECIPE-FACTS **Что Ф1.3 исправила в записанном** (находка
  [`harvest/f1-3-hash-blast-radius.md`](../campaigns/packages-2026-09/harvest/f1-3-hash-blast-radius.md)).
  (1) **Продюсеров формы `sha256:` ТРИ, а не два**: третий,
  `commit_content_hash` (`git_package_registry/fetch.rs:484`), выводит хэш из
  коммита, а не из дерева, и именно он пишет `content_hash` в локфайл для
  **in-place** пакетов; рецепт, описанный как «исключения + нормализация +
  порядок обхода», его не описывает вовсе — **именованный дефер**.
  (2) **Посылка Б.3 о расхождении Windows/Unix опровергнута**: `Path::cmp`
  покомпонентный, разделителя не видит, рецепт 0 платформенно стабилен;
  рецепты различает порядок «по компонентам» против «по нормализованным байтам»
  и расходятся они лишь на соседе с байтом **ниже** `/` (`spec-x.md`, 0x2D) —
  на байте, названном планом (`specX.md`, 0x58), они СОВПАДАЮТ. Фикстура
  `golden-order-trap-0.1.0` несёт обе формы: ловушку и контроль.
  (3) **Паритет-тест не сторожил того, чем назван** — звал одну реализацию;
  теперь сверяет обе при обоих рецептах. @status:impl/done
- @fact:WAL-KI-S1-PRIVATE-REPO **S1 — решение босса до нарезки.** Хост отвечает
  `Repository not found` на приватный репозиторий, невидимый токену, — тем же
  ответом, что и на несуществующий (свойство хоста, кодом не снимается). Значит
  «не найден ⇒ создать» надёжен только для публичных/операторских репо, а текст
  вопроса яруса 2 из §S1.3 («does not exist — create it?») **лжёт** в остальных
  случаях. Строить надо честную формулировку («не видим твоим кредам») плюс
  внятную обработку «уже существует». Хорошая новость: трёхсторонний
  дискриминатор УЖЕ построен и оттестирован (`GitError::{RepoNotFound,
  AuthFailed, NetworkUnreachable}`, `classify_stderr_message`
  `git_backend/shell.rs:492`, работает на `ls-remote --tags` `:212`) — слайс
  дешевле, чем считал план; недостаёт лишь того, что publish-сторона плющит всё
  в `PublishError::Git(String)` (`vibe-publish/src/lib.rs:187`). @status:impl/done
- @fact:WAL-KI-S6-NO-TIMESTAMP **S6 — решение босса до нарезки.** Обязательная
  владельцем оговорка свежести опирается на факт, которого в данных НЕТ: SARIF
  намеренно «no wall-clock», fact-store ключуется хэшем, baseline времени не
  несёт. Артефакт находок — `target/conform/report.sarif`, пишется только
  `check` и лежит под `.gitignore`; единственный КОММИЧЕННЫЙ артефакт
  (`conform-baseline.json`) хранит отпечатки `rule|file|carrier` **без номеров
  строк**, то есть как (file,line)-ключ непригоден. Свежесть придётся завести
  отдельно (mtime SARIF + явное «нет файла ⇒ не измерено»). Маршрут (3) при этом
  цел: `vibe-trace` держит specmap и НЕ держит conform, а `HitSource`
  (`vibe-trace/src/search.rs:41`) документирован как дверь второго поставщика. @status:impl/done
- @fact:WAL-KI-WIREGEN-IS-A-PIPELINE **`vibe-wire-gen` — конвейер, а не текстовый
  фильтр** (находка Ф0.2, `harvest/f0-gen-poc.md` §7): преобразования А.5 №3
  (`x-empty`) и №4 (строгость по роли) требуют входов, которых в сгенерированном
  Rust НЕТ — `metadata."x-empty"` из JTD-схемы и `foreign_parsers` из реестра.
  Форма Ф4.2 — `(схема + реестр + сгенерированный Rust) → Rust`. Тегированные
  объединения (`RepomdFileEntry`, Ф1.5) этим путём НЕ покрываются — нужен
  отдельный путь кодогенерации; версия в заголовке выхода генератора врёт
  (`v0.2.1` при бинаре 0.4.1) — пинить надо бинарь. @status:impl/done
- @fact:WAL-KI-G11-BY-CAP **`by-cap`/`by-purl` публикуются без ридера файла**
  (находка Ф0.3): `inverted.rs` определяет `CapabilityRow:66` и `PurlRow:75`, но
  `pub fn read` в нём нет — тогда как `primary.rs:75` и `by_name.rs:57` его имеют.
  Открытое нарушение G11. Схемы им добавлены в Ф4.1; ридеры даёт Ф4.2. @status:impl/done
- @fact:WAL-KI-F21-CLOCK-SITES **Ф2.1 упрётся в фикстурный конструктор:**
  `Utc::now()` в периметре гейта живёт не только у писателя —
  `types/entry/mod.rs:167` (`VersionEntry::minimal`) штампует `indexed_at`. Пять
  сайтов всего: `index/memory.rs:86`, `:249`, `types/entry/mod.rs:167`,
  `cli/add.rs:113`, `cli/reindex.rs:232`. Решение (параметр `at` против
  `#[cfg(test)]`) принимается в пакете Ф2.1, не оставляется воркеру. @status:impl/done
- @fact:WAL-KI-STALE **33 файла stale** (стоячий долг сдвигов байтов) —
  адресовано кампанией S7 (рулинг №4); до неё живут видимыми. @status:impl/done
- @fact:WAL-KI-TASKS-STALE **`TASKS.md` объявляет слайс «draining the backlog
  (2026-08-06)»**, которого давно нет — главная полоса с тех пор ушла в
  change-native стройку. Правка — боссовым коммитом на ближайшей границе фазы. @status:impl/done
- @fact:WAL-KI-B074 **Второй якорь факта в абзаце глотается молча** (B-074) —
  механизм чека не построен. @status:impl/done
- @fact:WAL-KI-B075 **Панель может мигнуть на чистом дереве** (B-075) — читать
  отказ, не перезапускать вслепую. @status:impl/done
- @fact:WAL-KI-PHASE-E-SIX **Шесть файлов корпуса на `work`** — условие выходного
  гейта Phase E, ждёт рулинга. @status:impl/done

## Session context {#session-context}

@fact:WAL-CTX-BOOT **Холодная сессия читает `CONTINUE.md` →
`NEXT-SESSION-PROMPT.md` → документы предмета (PROP-044 + три ТЗ) →
`SUBAGENT-LAUNCHERS.md` целиком перед фан-аутом** — и берёт каждое число из
команд сверху, после rescan. Этот файл главнее `CONTINUE.md`; ратифицированный
PROP-044 главнее обоих. @status:impl/done
