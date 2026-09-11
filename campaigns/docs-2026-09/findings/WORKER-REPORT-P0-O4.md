# WORKER-REPORT-P0-O4 — раннер примеров, макет (спайк A0.12)

Дата: 2026-09-11. Дерево: `C:\Users\olegc\git\v\vibevm-docs`, HEAD `b1291b06`.

Артефакты:

- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.12-example-runner.md` — находка.
- `C:\Users\olegc\git\v\vibe-docs-vision\findings\A0.12-runner-mock.py` — макет раннера
  (478 строк, Python 3.11, только stdlib).
- Корпус примеров и вспомогательные пробы — в scratch,
  `<scratch>/vibe-docs-phase0\P0-O4\`
  (`examples\` — семь примеров с `example.toml` и благословлёнными
  `expect.out` / `expect.err`; `jtd_probe.py` — отдельная проба схем;
  `probe-home\`, `cwdprobe\` — пробные прогоны).

## Решения

1. **Язык макета — Python, не bash.** Пакет разрешал оба. Нормализация
   (регулярки, замены путей в четырёх написаниях, сортировка блоков) и
   валидатор JTD на bash получились бы длиннее и хрупче; `tomllib` в
   стандартной библиотеке 3.11 снимает зависимость на парсер TOML.
2. **cwd примера — песочница.** Команда в примере не называет путь, а
   `--path` остаётся `.`. Это и делает вывод `vibe install` свободным от
   абсолютных путей, и держит запуск подальше от рабочего дерева.
3. **Окружение изоляции не меняет поведение продукта.** Раннер выставляет
   `VIBE_SETTINGS`, `VIBE_REGISTRY_CACHE`, `VIBEVM_SEARCH_CACHE_DIR` и
   `NO_COLOR`, а поведенческие унаследованные переменные (`VIBE_OFFLINE`,
   `VIBE_UNATTENDED`, `VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`,
   `VIBETERM`, `VIBEFRAME`) удаляет. `--offline` и `--assume-yes` обязаны
   стоять в самой команде примера, иначе пример зеленеет по причине,
   которой читатель не видит.
4. **Валидатор JTD написан в макете.** В дереве рантайм-валидатора JTD
   нет (см. находку, п. 4): схемы — вход кодогенерации. Реализованы все
   восемь форм RFC 8927.
5. **Семь примеров вместо трёх.** Пакет требовал три (`--version`,
   `--help`, команда с реестром). Добавлены `install-alpha-json` (для
   пункта 4 — валидация `--json`), `cache-path` (даёт настоящую строку со
   смешанными разделителями), `vars` (даёт настоящую утечку имени
   учётки) и `install-missing` (единственный непустой stderr и
   единственный ненулевой код выхода). Без них таблица классов
   расхождений была бы гипотетической, а ветка `expect.err` —
   непроверенной.
6. **`match`-сравнение по шаблону не введено.** Все классы закрылись
   именованными правилами нормализации; обоснование и единственный
   честный кандидат на исключение — в находке, п. 5.

## Отклонения

1. **В рабочем дереве во время проб появился один untracked-файл;
   удалён, дерево возвращено в исходное состояние.**
   `vibevm/vibedeps/.gitignore` (162 байта, mtime 23:00:15) возник при
   первой пробной установке, запущенной с cwd = корень дерева и `--path`
   в scratch. Worktree создан в 22:45, файла в нём не было; после
   удаления `git status --short` чист. Четыре контрольных прогона в
   scratch запись не воспроизвели — подробности и непроверенная
   переменная описаны в находке, «Открытые вопросы», п. 1. Причина
   отклонения: команда была запущена из корня дерева до того, как стало
   понятно, что продукт может писать рядом с cwd. Дальнейшие пробы
   запускались из scratch.
2. **`~/.vibe` снят «до» не в самом начале сессии**, а перед прогонами
   макета — первые пробы `vibe --version` / `--help` / `init` уже
   прошли. Все они выполнялись с выставленными тремя переменными
   изоляции, и списки «до» и «после» совпадают дословно; настоящий дом не
   изменялся.
3. **Секреты.** В `~/.vibe` присутствуют `github.publish.token`,
   `zai.api.token`, `zai.api.token.2` (R-12). Ни один не открывался, ни
   одно значение не печаталось; в листингах самопроверки фигурируют
   только имена файлов, чего требует сам пакет.

## Вывод самопроверки

### 1. Прогон макета

Команда:

```
python C:/Users/olegc/git/v/vibe-docs-vision/findings/A0.12-runner-mock.py \
  --examples <scratch>/vibe-docs-phase0/P0-O4/examples \
  --repo .
```

Вывод дословно:

```
[MATCH ] cache-path  exit=0
[MATCH ] help  exit=0
[MATCH ] install-alpha  exit=0
[MATCH ] install-alpha-json  exit=0
    3 JSON document(s) on stdout
      `install:plan`: NO schema declared - unchecked
      `install:closure-diff`: NO schema declared - unchecked
      `install`: valid against schemas/install_report.jtd.json
[MATCH ] install-missing  exit=1
[MATCH ] vars  exit=0
[MATCH ] version  exit=0

7/7 example(s) matched
runner-exit=0
```

Три команды, которые назвал пакет: `vibe --version` — код выхода 0,
совпало; `vibe --help` — код выхода 0, совпало; `vibe install
flow:org.vibevm/integration-alpha --registry ${REGISTRY} --assume-yes
--offline` — код выхода 0, совпало. Фикстура, которая ставится офлайн,
нашлась: реестр `fixtures/registry`, пакет
`org.vibevm/integration-alpha@1.0.0`. Седьмой пример, `install-missing`,
ловит ненулевой код выхода (1) и непустой stderr.

### 2. Негативный контроль (сравнение действительно кусается)

Подмена строки в `examples/version/expect.out`, затем возврат:

```
[DIFFER] version  exit=0
    stdout differs
    --- expect.out
    +++ actual.out
    @@ -1 +1 @@
    -vibe 1.0.0
    +vibe <VERSION>

0/1 example(s) matched
--- negative-control exit=1 ---
[MATCH ] version  exit=0

1/1 example(s) matched
--- restored exit=0 ---
```

### 3. `ls ~/.vibe` до и после

До (снято перед прогонами макета):

```
aiui
cache
config.toml
github.publish.token
opt
progress-cache
registries
registry.toml
settings.toml
state
steward
zai.api.token
zai.api.token.2
```

После (снято по завершении всех прогонов):

```
aiui
cache
config.toml
github.publish.token
opt
progress-cache
registries
registry.toml
settings.toml
state
steward
zai.api.token
zai.api.token.2
ls-exit=0
```

Совпадает. Настоящий дом не тронут.

### 4. `git status --short`

```
 M .claude/agents/opus5.md
status-exit=0
```

Пусто, кроме допущенного пакетом ` M .claude/agents/opus5.md`.

## Что не сделано и почему

1. **TTY-ветка продукта не замерена.** Продукт ветвится на
   `std::io::stdin().is_terminal()` минимум в пяти местах, а `vibe tree`
   под не-TTY рендерит статичное ASCII-дерево. Раннер всегда не-TTY,
   значит часть поведения он документировать не может. Поднять PTY — это
   отдельная задача и отдельное решение (в находке, «Открытые вопросы»,
   п. 2), в A0.12 не входило.
2. **Класс «локаль и даты» оставлен без правила.** Ни один из семи
   примеров не породил ни даты, ни локализованной строки. Писать правило,
   не видя настоящей строки, значит угадывать; класс зафиксирован как
   открытый.
3. **Причина появления `vibevm/vibedeps/.gitignore` не установлена
   окончательно.** Непроверенной осталась ровно одна переменная — cwd
   внутри рабочего дерева; её проверка означала бы намеренную запись в
   репозиторий, что пакет запрещает. Практический вывод для раннера от
   причины не зависит: cwd примера — песочница, плюс tripwire на
   неизменность исходного дерева по образцу `tools/user-home-tripwire.sh`.
4. **Взаимный порядок stdout и stderr не проверяется** — потоки
   захватываются раздельно, и пример, документирующий их чередование,
   потребует отдельного режима (общий поток). Обе ветви самого сравнения
   закрыты: шесть примеров с пустым stderr и `install-missing` с
   настоящим сообщением об ошибке и кодом выхода 1.
