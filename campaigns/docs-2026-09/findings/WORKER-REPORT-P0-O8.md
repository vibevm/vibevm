# WORKER-REPORT-P0-O8 — цена рендера хоста (спайк A0.28)

Дерево: `C:\Users\olegc\git\v\vibevm-docs`, HEAD `b1291b06` (проверено
`git rev-parse --short HEAD`). Бинарник для проб — `target/debug/vibe.exe`,
`vibe --version` → `vibe 1.0.0`. Все сборки — в
`CARGO_TARGET_DIR=<scratch>/vibe-docs-phase0/P0-O8/target`;
в дерево репозитория не записано ни байта, git-командами только читал
(`status`, `rev-parse`).

## Готово

1. Источник «latest» установлен по PROP-019 и коду — находка, раздел 1:
   `latest` внутри чекаута = **текущее рабочее дерево как есть**, теги не
   читаются; git даёт только ярлык версии. Вне дерева `latest` = ветка
   `origin/main`, тоже не тег.
2. Замеры сделаны: холодная release (6m 42s / 403 с), инкрементальная release
   после правки одного файла (3m 50s / 231 с), плюс no-op (5.78 с), доплата за
   `vibe-index` (6m 06s), холодная dev (1m 42s) и инкрементальная dev (22.55 с).
   Числа ядер и модель процессора, размеры бинарников и кэша — в находке.
3. Оценка «раз в час на четырёх ядрах» записана как порядок величины с явной
   оговоркой, что пересчёт по ядрам груб.
4. Оба варианта для `derived` описаны с ценой; вариант (б) — только перечень
   требований, без прототипа.

Находка: `findings/A0.28-render-cost.md`.

## Решения (интерпретации задания)

- **Метод для пункта 2(б).** Задание само отвергает `touch` файла в дереве и
  предлагает копию через `robocopy`. Сделал копию: `crates/`, `xtask/`,
  `.cargo/`, `formats/`, `schemas/`,
  `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v1.0.0/` и
  корневые манифесты → `…/P0-O8/tree-copy` (2213 файлов, 21.25 МБ). **Копия
  собралась**, обходных путей не потребовалось, поэтому оговорка задания «если
  копия не собирается из-за путей vibedeps — ограничься замером (а)» не
  сработала. Уточнение к заданию: внешние path-зависимости лежат не в
  `vibevm/vibedeps/`, а в `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v1.0.0/`
  (`Cargo.toml:148–155`); копировать надо именно их.
- **Общий `CARGO_TARGET_DIR` для дерева и копии.** Копия собиралась в тот же
  scratch-каталог сборки. Это сэкономило вторую холодную сборку и на результат
  не влияет: первая сборка копии заняла 0.80 с и не перекомпилировала ничего
  (cargo признал артефакты дерева свежими — отпечатки не привязаны к
  абсолютному пути корня), а после дописанной пустой строки в
  `tree-copy/crates/vibe-cli/src/main.rs` cargo перекомпилировал ровно один
  крейт. То есть замер (б) — это честная «пересборка после правки одного
  файла», а не «пересборка всех крейтов рабочего пространства».
- **Контрольный второй метод для (б).** Независимо от копии удалил отпечатки
  `vibe-cli` в scratch-каталоге сборки (`rm -rf target/release/.fingerprint/vibe-cli-*`
  — это каталог артефактов, не дерево) и пересобрал из дерева: 3m 43s против
  3m 50s, расхождение 3 %. Два метода сходятся.
- **`--locked` во всех сборках.** Чтобы cargo физически не мог переписать
  `Cargo.lock` в дереве. Ни одна сборка на это не пожаловалась.
- **`--manifest-path` вместо `cd` в копию** — чтобы рабочий каталог оставался
  корнем репозитория и вызовы были воспроизводимы.

## Отклонения и добавления к пакету

- **Добавлено сверх задания (4 замера).** No-op пересборка (6 с) — она задаёт
  пол цены рендера для коммита, не трогающего `crates/`, а это типичный коммит
  кампании документации. Доплата за `vibe-index` (6m 06s) — потому что
  `vibe self install` собирает **два** бинарника
  (`crates/vibe-cli/src/commands/vvm/builder.rs:99–112`), и рендереру эта
  доплата не нужна. Холодная и инкрементальная сборка на профиле `dev`
  (1m 42s и 22.55 с) — потому что вывод `vibe --help` у release- и
  debug-бинарника совпал побайтово (`diff`, rc=0), то есть профиль не меняет
  содержимое derived-блока, но меняет цену в 4–10 раз. Это прямо отвечает на
  пункт 4(а) «сколько минут дебаунса разумно».
- **Время запуска derived-блока** (`vibe --help` — 0.410 с) замерено, хотя не
  требовалось: без него цена рендера выглядит как «только сборка».
- Отклонений от запретов пакета нет: в дерево не писал, git не менял, ничего
  глобально не устанавливал, `~/.vibe/*.token` и `infra` не открывал, секретов
  не встречал.

## Вывод самопроверки (дословно)

Времена и коды выхода сборок — команды выполнялись с
`export CARGO_TARGET_DIR="<scratch>/vibe-docs-phase0/P0-O8/target"`
из корня `C:\Users\olegc\git\v\vibevm-docs`:

```
(а) cargo build --release -p vibe-cli --locked
rc=0
    Finished `release` profile [optimized] target(s) in 6m 42s
start=1789158209 end=1789158612   → стена 403 с; «Compiling» 354

(б) echo "" >> …/P0-O8/tree-copy/crates/vibe-cli/src/main.rs
    cargo build --release -p vibe-cli --locked --manifest-path …/P0-O8/tree-copy/Cargo.toml
rc=0
    Finished `release` profile [optimized] target(s) in 3m 50s
start=1789159332 end=1789159563   → стена 231 с; «Compiling» 1
```

Прочие замеры (те же условия):

```
no-op release          rc=0  Finished … in 5.78s     start=1789158629 end=1789158635  Compiling 0
контроль (б)           rc=0  Finished … in 3m 43s    start=1789159056 end=1789159279  Compiling 1
+ vibe-index           rc=0  Finished … in 6m 06s    start=1789158647 end=1789159013  Compiling 73
холодная dev           rc=0  Finished `dev` … 1m 42s start=1789159616 end=1789159719  Compiling 354
инкрементальная dev    rc=0  Finished `dev` … 22.55s start=1789159725 end=1789159748  Compiling 1
```

`ls -la <CARGO_TARGET_DIR>/release/vibe.exe`:

```
-rwxr-xr-x 2 olegc 197121 73571328 Sep 11 23:46 <scratch>/vibe-docs-phase0/P0-O8/target/release/vibe.exe
```

`ls -la findings/A0.28-render-cost.md`:

```
-rw-r--r-- 1 olegc 197121 18790 Sep 11 23:52 /c/Users/olegc/git/v/vibe-docs-vision/findings/A0.28-render-cost.md
ls_rc=0
```

`git -C C:/Users/olegc/git/v/vibevm-docs status --short`:

```
 M .claude/agents/opus5.md
 M vibevm/vibespecs/common/PROP-028-package-families.xml
 M vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml
 M vibevm/vibespecs/modules/vibe-facts/PROP-043-facts-markup.xml
 M vibevm/vibespecs/modules/vibe-progress/PROP-047-progress-campaigns.xml
?? vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml
git_rc=0
```

`git -C C:/Users/olegc/git/v/vibevm-docs rev-parse --short HEAD` → `b1291b06`.

**Самопроверка по git не чистая — но изменения не мои.** Ожидалось пусто, кроме
` M .claude/agents/opus5.md`. В начале работы `git status --short` показывал
ровно эту одну строку — проверено дважды: перед запуском первой сборки
(≈23:22) и во время неё (unix 1789158382 ≈ 23:26). Пять спецификационных
файлов изменились позже, во время моего прогона:

```
ls -la --time-style=full-iso …
23:46:42  vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml   (новый, ??)
23:49:45  vibevm/vibespecs/common/PROP-028-package-families.xml
23:49:58  vibevm/vibespecs/modules/vibe-facts/PROP-043-facts-markup.xml
23:50:02  vibevm/vibespecs/modules/vibe-progress/PROP-047-progress-campaigns.xml
23:52:03  vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml
(для сравнения: .claude/agents/opus5.md — 22:51:46, до начала моих замеров)
```

Я эти файлы не открывал и не писал: из спецификаций читал только
`vibevm/vibespecs/common/PROP-019-version-manager.xml`, а писал только в
`C:\Users\olegc\git\v\vibe-docs-vision\findings\` и в scratch. Сборки шли в
отдельный `CARGO_TARGET_DIR`, единственный build-script проекта
(`crates/vibe-cli/build.rs`) ничего не пишет в дерево. Похоже на параллельную
работу центральной сессии (имя нового файла — PROP-057 «documentation packages
and site»). Центральной сессии стоит это подтвердить: если писатель не она,
значит в дереве работает кто-то ещё.

## Что не сделано и почему

- **Сборка на Linux/в Docker не мерилась** — у меня Windows-машина; в находке
  это отмечено как неизмеренная поправка.
- **Первая загрузка зависимостей не мерилась**: кэш реестра crates.io был
  прогрет, `cargo` ничего не скачивал. Для холодного контейнера это отдельная
  добавка (573 пакета в `Cargo.lock`).
- **Инкрементальная сборка после правки нижнего крейта** (например
  `vibe-core`) не мерилась — задание просило один файл, и я взял
  `crates/vibe-cli/src/main.rs`. Такая правка будет дороже 3m 50s и дешевле
  холодных 6m 42s; точное число не установлено.
- **Вариант (б), `derived.json`, не прототипировался** — задание прямо это
  запрещает без слова владельца; в находке только перечень требований.
- **Характеристики сервера не выяснялись** (правило R-25), поэтому все
  серверные числа в находке помечены как порядок величины.

## Мусор после работы

В `<scratch>/vibe-docs-phase0\P0-O8\` осталось
~9.3 ГБ: `target/` (кэш сборки, 9.27 ГБ), `tree-copy/` (копия исходников,
21 МБ) и журналы сборок `build-*.log`. Удалять не стал — из них проверяемы все
числа находки. Каталог вне репозитория; можно удалить целиком.
