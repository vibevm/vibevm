# WORKER-REPORT-P0-O2

Пакет: P0-O2 «Рёбра из документов — хостовая или движковая сторона» (спайк A0.6)
Дата: 2026-09-11
Дерево: `C:\Users\olegc\git\v\vibevm-docs`, HEAD `b1291b06`
Находка: `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.6-edges-host-or-engine.md`

## Решения

1. **Ответ на вопрос 1 дан отрицательным и подтверждён данными, а не только
   чтением кода.** Помимо сигнатур (`ENGINE/src/index.rs:72-73`) я проверил
   живой `specmap.json` корня дерева: 2961 ребро, ни одно не исходит из файла
   с расширением `.md` или `.xml`. Это устраняет риск «в коде так, а на
   практике иначе».
2. **Вопрос 4 признан главным и разобран до уровня файлов.** Установлено, что
   хост зависит не от авторской, а от **вендоренной** копии движка
   (`Cargo.toml:153` → `rust-ai-native-lang/.../crates/vendor/`), и что
   авторская и вендоренная копии сейчас байт-идентичны (`diff -r`, код 0).
   Это делает R-21 не теорией, а измеренным фактом.
3. **Найдена вторая, независимая реализация того же закрытого XML-диалекта
   в хосте** (`crates/vibe-specdoc/src/xml_in.rs`, `xml_blocks.rs`), паритет
   с движком закреплён тестами. Это меняет цену обоих вариантов развилки и
   вынесено в таблицы находки — пакет этого файла не называл, я дошёл до него
   от комментария `ENGINE/src/xmlspec/doc.rs:12-13`.
4. **Найдены два прецедента хостового обхода движка**, прямо относящиеся к
   F-10/F-28: `crates/vibe-spec/src/lib.rs:9-14` (осознанный отказ
   переиспользовать вендоренную грамматику именно потому, что она
   sync-engines-gated) и `crates/vibe-spec/src/link_table.rs` (рёбра
   документ→документ уже живут в хосте, вне `specmap.json`). Рекомендация
   опирается на них.
5. **Рекомендация: хостовая сторона.** Обоснование сведено к цене: движковый
   вариант расширяет закрытый язык, общий для трёх языковых стеков, и
   переписывает 6 вендоренных копий; хостовый — новый модуль плюс две точки
   инъекции в `xtask/src/specmap.rs`. Шов для инъекции (`CodeScanner`,
   `build_with_scanner`/`check_with_scanner`) уже публичен и создан ровно
   под это.
6. **Отдельно отмечена ловушка**: `run_resolve_gate` (`xtask/src/specmap.rs:58`)
   строит карту **вторым независимым вызовом** `index::build`. При хостовом
   сканере инъекцию нужно продублировать, иначе гейт покрытия зелёный по
   пустоте.
7. **Пины.** Ответ сведён к одному наблюдению: единственный вход в suspect —
   `if let Some(p) = e.pinnedR.as_deref()`. Ребро без пина не может стать
   suspect ни при каком изменении спеки. Прецедент беспинового ребра уже
   есть (`ENGINE/src/jtd.rs:115-119`).
8. **Формат находки.** Цитаты кода ограничены 15 строками, как требует
   `PACKET-COMMON.md`; длинные перечисления оформлены таблицами, а не
   кодовыми блоками.

## Отклонения

- **Отклонений от пакета нет.** Все шесть названных источников прочитаны;
  дополнительно (в рамках вопроса 4, «если оба — кто главный») прочитаны
  `crates/vibe-specdoc/**`, `crates/vibe-spec/**`, `xtask/src/sync_engines.rs`
  и `crates/vibe-cli/src/commands/specmap.rs` — без них ответ на вопрос 4
  был бы неполным.
- **Правок в рабочем дереве не делал; git не трогал** (только читающие
  `git rev-parse`, `git log`, `git status`). Файл находки и этот отчёт
  записаны в `vibe-docs-vision/findings/`, временный вывод команды — в
  `<scratch>/vibe-docs-phase0\P0-O2\`.
- **Секретов не встречал.** `~/.vibe/*.token`, `C:\Users\olegc\git\infra\**`
  не открывал. IP-адресов, портов, путей на сервере в находку не попало.
- **Дефектов пакета не обнаружил.**

## Вывод самопроверки (дословно)

### 1. `cargo xtask specmap --check 2>&1 | tail -3`

Команда выполнена один раз, как разрешено пакетом (наблюдение). Вывод
перенаправлялся в файл; ниже — последние три строки вывода команды:

```
  drift: units added: 20
  drift: edges added: 3
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
```

Код выхода: **1**.

Первая строка вывода, для контекста:

```
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
```

Всего 12 строк вывода: 1 строка `Error:`, 8 строк `drift: unbumped-hash: …`,
`drift: units added: 20`, `drift: edges added: 3`, финальная строка `Run …`.

**Гейт был красным до меня.** Я не менял ни одного файла в дереве, а ветка
`--check` в движке физически ничего не пишет: единственный вызов
`specmap_core::index::write` в
`vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v1.0.0/crates/rust-ai-native-specmap/src/lib.rs:37`
лежит в ветке `else` (не-`check`). Причина красноты — несведённый
`specmap.json` на HEAD `b1291b06`.

### 2. `ls findings/A0.6-edges-host-or-engine.md`

```
C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.6-edges-host-or-engine.md
```

Существует.

### 3. `git -C C:/Users/olegc/git/v/vibevm-docs status --short`

```
 M .claude/agents/opus5.md
?? vibevm/vibedeps/.gitignore
```

- ` M .claude/agents/opus5.md` — ожидаемо пакетом, не моё.
- `?? vibevm/vibedeps/.gitignore` — **не ожидалось пакетом; не моё.**
  Это неотслеживаемый файл (в истории его нет:
  `git log --oneline -1 -- vibevm/vibedeps/.gitignore` — пусто), содержимое:

```
# Build output produced inside materialised dependency slots.
# Managed by vibe; additional entries are preserved until `vibe clean`.
**/target/
**/node_modules/
```

  Пишет его `vibe-workspace` (`crates/vibe-workspace/src/vibedeps/build_ignore.rs`)
  — путь семейства `vibe install`, который `cargo xtask specmap --check`
  никогда не вызывает (xtask по specmap зависит только от `specmap_core` и
  `rust_ai_native_specmap`). Мои команды его создать не могли: я запускал
  только читающие команды плюс один `cargo xtask specmap --check`.
  Наиболее вероятный источник — параллельный воркер фазы 0, выполнявший
  `vibe install` в том же дереве. Оставил как есть: правка дерева пакетом
  запрещена, удаление чужого файла — тем более.

## Что не сделано и почему

1. **Не собирал workspace целиком** — пакет прямо это запретил
   (`cargo build` всего workspace — нет). `cargo xtask specmap --check`
   собрал только то, что нужно самому xtask.
2. **Не проверял поведение `vibe specmap` запуском** на doc-пакете —
   пакет просил разобрать форму и механику по коду, а не ставить
   эксперимент; пробного пакета в scratch не создавал, чтобы не плодить
   артефакты.
3. **Не определил формат страниц документации** (закрытый диалект PROP-045
   или собственный) — этого нет ни в пакете, ни в коде; вынесено в открытые
   вопросы находки, потому что от ответа зависит около половины строк в
   таблице цены.
4. **Не читал `AGENT-PLAN.md` и вижен-документы** — запрещено
   `PACKET-COMMON.md`. Развилки F-10/F-28 и правило R-21 использованы как
   данность, по формулировкам самого пакета.
5. **Не читал boot-лейн репозитория** (`vibevm/vibespecs/boot/**`,
   `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`) — запрещено пакетом. PROP-014 читал
   точечно по указанному пакетом пути в `vibevm/vibedeps/`.
