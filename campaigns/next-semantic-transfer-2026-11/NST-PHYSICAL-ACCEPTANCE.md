# NST — приёмка на физических машинах владельцем

Жанр: руководство ручной приёмки и форма протокола, не новый implementation plan.
Дата: 2026-11-18. Статус: сценарии подготовлены; создание документа не означает их выполнения.

**Current disposition (2026-11-20):** The later owner waiver in `NST-AUTONOMOUS-EXECUTION.md` governs owner-attended checks. The dated table below is a historical/manual route, not a request to repeat waived checks. A waiver never changes an observed automated failure into a pass. The scheduled blind USER-PROJECT adoption trial is separate: it remains UNRUN for lack of verified isolated runner access, not waived and not a negative behavioral result. See `UP-ADOPTION-RUNNER-BLOCKER-20261120.md` in the user-project campaign.

## 1. Зачем нужен этот документ

Автоматический test, SSH-команда и успешная сборка не доказывают, что пользователь увидел правильное окно, смог работать в своём terminal, безопасно отключил USB-диск или осознанно разрешил реальному агенту выполнить действие.

Здесь собраны именно такие законченные пользовательские сценарии. Автоматически проверяемую механику повторять вручную ради количества галочек не требуется. Это **не полная повторная приёмка всех 222 задач NST** и не основание объявлять непроверенными уже принятые результаты. Документ закрывает известные физические/attended пробелы MT-04, MT-05, MT-06 и текущего PRIOR-APP-STEPS.

### Real-agent adoption is a separate NST exit check

`UP-AGENT-ADOPTION` is planned, not passed. This is an isolated real-agent behavioral trial, not a new owner-attended Windows requirement. Use the [blind writing, fresh-session recall and cross-project control](../user-project-2026-11/UP-AGENT-ADOPTION-01.md#blind-baseline-script-supervisor-only). Neither a manual UI observation nor a mechanical storage test proves that an agent chose the correct destination without prompting. Record the actual outcome in the adoption task, not twice.

### Решение владельца о Windows

2026-11-18 владелец разрешил принимать задачи внутри NST без Windows-проверок, требующих их ручного участия на физической машине, включая `l`/Ctrl+C и связанные Terminal assertions.

**Эти проверки не блокируют текущую приёмку. Их результат — `NOT RUN / owner exception`, а не PASS.** Документ сохраняет их как необязательный будущий physical acceptance route, не возобновляет обязательство выполнить их и не требует снова запускать подготовленный скрипт.

Исключение не распространяется автоматически на все GUI, физические носители, macOS или реальный agent provider. По каждому оставшемуся обязательному критерию нужны либо ручной результат, либо отдельное явное решение владельца. Не считать отсутствие оборудования ошибкой реализации: это `BLOCKED`, пока не изменено решение о приёмке.

## 2. Что действительно требует владельца

| ID | Законченный сценарий | Незаменимое ручное действие | Решение о текущей обязательности |
|---|---|---|---|
| W-TERM | Fold/live log и остановка ordinary install в Windows Terminal | Увидеть вывод, нажать `l`/Ctrl+C, оценить восстановление terminal | Не блокирует: owner exception от 2026-11-18 |
| W-SHELL | Возврат из `vibe shell` в исходный cmd/PowerShell/pwsh | Работать в настоящем terminal каждого shell | Не блокирует в пределах того же Windows manual exception |
| W-GUI | Открытие gVim и Start Menu entry как обычным пользователем | Увидеть настоящее приложение и открыть shortcut | Остаточный GUI claim; не считать автоматически отменённым Windows Terminal exception |
| M-GUI | Запуск установленного приложения из Finder/Launchpad | Открыть `.app`, увидеть нужное приложение/Terminal и завершить его | Нужен ручной результат либо отдельное решение владельца |
| M-TERM | Работа настоящего Vim через proxy в macOS Terminal | Поработать в редакторе, нажать Ctrl+C, нормально выйти | Автоматическая signal/PTY proof уже существует; фактическое визуальное/attended утверждение отдельно не подтверждено |
| MEDIA | Приложение на реально съёмном носителе: отключение, отказ, возврат, move обратно | Назвать безопасный носитель, физически отключить/подключить его | Нужен ручной результат либо отдельное решение владельца; выполнить на заявляемых ОС |
| AGENT | Согласие → настоящая attended agent session → interruption → resume | Осознанно ответить на consent и работать с настоящим агентом | Нужен ручной результат и допустимый execution context либо отдельное решение владельца |

«Нужен ручной результат» означает обязательность **для соответствующего ещё открытого утверждения**, а не требование прямо сейчас проводить всё перечисленное. Не переносить эти пункты в новый бесконечный prerequisite campaign.

## Ведение документа по мере выполнения NST

Прямая инструкция владельца2026-11-18: **обновлять этот документ по мере продвижения работы над NST**. Это живой acceptance guide, не замороженный одноразовый checklist.

Architect отвечает за актуальность сценариев и границ доказательств; builder передаёт изменения поведения, готовые команды/комплекты и фактические receipts. Обновлять документ в той же поставке, которая меняет соответствующий пользовательский сценарий или его acceptance, а также при появлении native/manual результата либо нового решения владельца. Не переписывать его целиком после каждого внутреннего edit.

При обновлении:

- Добавлять только реально возникшие physical/attended claims, а не гипотетические permutations. Для нового сценария указывать, почему автоматического доказательства недостаточно, конкретное действие владельца и ожидаемый результат.
- Удалять из оставшегося ручного маршрута то, что уже достаточно проверено автоматически; сохранять границу visual/hardware/real-provider proof. Не заставлять владельца повторять принятую неизменившуюся проверку.
- Обновлять package/source/binary pins и готовность команд в сопровождающем приёмочном комплекте. Изменившийся runtime инвалидирует только затронутые claims; test-only или documentary delta не требует автоматически повторять физическую приёмку.
- Записывать PASS/FAIL/BLOCKED/NOT RUN и owner exceptions с датой и evidence pointers. Не превращать старую приёмку в текущую без проверки применимости и не стирать историю отказов или исключений.
- Согласовывать таблицу обязательности и оставшиеся вопросы с actual authoritative multi-user-planning; сначала обновлять исходный план, затем штатный task-tree snapshot. Если результат ещё не проверен architect, обозначать его как candidate/reported.
- На существенной границе NST и перед финальным acceptance сверять документ с завершёнными пользовательскими сценариями, actual receipts и решениями владельца. Не открывать новый рекурсивный review campaign из-за самого checklist.

### Журнал значимых обновлений

| Дата | Изменение | Основание / граница |
|---|---|---|
| 2026-11-18 | Создан единый physical acceptance guide; Windows manual exception отделён от PASS | Прямые запросы владельца; MT-04/05/06 и текущий PRIOR |
| 2026-11-18 | Установлено обязательное сопровождение по ходу NST | Прямая инструкция владельца: обновлять документ по мере продвижения |

### Уже имеющиеся автоматические native evidence

2026-11-18 architect независимо прочитал Windows/macOS full-log receipts на runtime source `f8dcdbe72c288761670e411ce6b7b5d3562e0226`: ordinary install успешен, каждый полный log2400032bytes сохраняет четыре markers, bounded previews проверены отдельно. Runtime equivalence test-only follow-up `d0632efdbc85cdc4c3491c73a819b543107d9187` проверена по diff. Это не visual GUI proof.

Также независимо прочитан `/fast/tmp/prior-macos-terminal-f8.log`: настоящий macOS PTY, два отправленных `l`, Ctrl+C и положительный ответ на stop question; exit130, record `interrupted`, partial log121bytes, terminal flags восстановлены. **Отправка клавиш и protocol/console restoration подтверждены; визуальное раскрытие fold, Finder и настоящий Vim GUI этим не доказаны.** Эти автоматические сценарии не требуется вновь выполнять руками только для повторения механики.

Evidence paths локальны для текущей машины; при передаче acceptance комплекта включить безопасные копии receipts, не считать эти пути доступными другому contributor по умолчанию.

## 3. Подготовка одного приёмочного комплекта

Подготовку выполняет builder; владелец не должен самостоятельно собирать тестовую инфраструктуру.

Перед началом builder передаёт:

1. Один точный source commit, платформу/архитектуру, версии и SHA-256 `vibe` и `vibe-app-proxy`. Если native binary построен на runtime-equivalent commit, разницу явно указать.
2. Уже готовую isolated test root с отдельными settings, caches, HOME и platform user directories. Не использовать обычный пользовательский `~/.vibe`.
3. Точные package coordinate/version, источник и hash payload. Наличие Vim/gVim подтвердить до приглашения владельца; не скачивать «latest» и не подменять Vim generic marker program.
4. Проверенные команды под эту сборку и fixture. Старые `@1.1.0`, `@1.1.2`, `vim92`, protocol `/3` и пути из исторических MT не считать текущими значениями.
5. Короткий launcher/script с process-local environment и source/hash admission. Никаких `setx`, постоянных изменений PATH, auth-store копирования, установки нового provider или отключения security policy.
6. Место для локального протокола и логов. Не писать токены, provider auth, приватные prompt contents или environment dumps в чат/Git.

На Windows запускать обычный **не elevated** Windows Terminal. На macOS — обычную desktop session владельца, не только SSH. Для MEDIA владелец отдельно называет пустой test directory на своём removable volume. Нельзя выбирать диск по догадке, форматировать носитель или отключать серверный/system/shared volume.

**Начальная проверка:** source/hash комплект совпадает, fixture открывается, обычные установки владельца не изменены. При несовпадении остановить только этот сценарий и вернуть комплект builder; не чинить пользовательскую машину на ходу.

## 4. W-TERM — log/fold/stop в Windows Terminal (необязательно)

**Основание:** MT-04 `STEP-13-HOTKEY`; ordinary contribution logging и cooperative stop. Текущая приёмка разрешена без этого прогона.

**Подготовка:** готовый hash-bound script с длинной безопасной contribution, которая пишет различимые строки и полный log; исходный console input mode записан скриптом. Не использовать дорогую реальную сборку лишь для создания длительного вывода.

1. Открыть обычный Windows Terminal. Запустить переданную builder единственную команду.
   - **Ожидается:** fixture выполняется, виден работающий блок и fold последних строк; shell остаётся отзывчивым.
2. Нажать `l`, затем ещё раз `l` во время вывода.
   - **Ожидается:** сначала live view, затем возврат к fold; набранные `l` не попадают в child input и не печатаются как мусор.
3. Нажать Ctrl+C. Если появляется вопрос остановки, выбрать остановку.
   - **Ожидается:** вопрос можно прочитать и ответить; competing key reader не перехватывает ответ; процесс завершает остановку без зависания. Запись операции отражает interruption, частичный log доступен.
4. После завершения набрать простую команду в исходном shell.
   - **Ожидается:** ввод/echo/обычный Ctrl+C работают как до прогона; автоматический check console-property restoration успешен. Владельцу не требуется вручную менять console settings.

**Evidence:** ответы «fold открылся/закрылся», «вопрос был/ответ прочитан», «shell восстановлен», exit/record/log path из скрипта. Отсутствие видимого вопроса не считать PASS, если fixture должна его вызывать.

## 5. W-SHELL — настоящий родительский shell Windows (необязательно)

**Основание:** MT-04 `STEP-5-SHELL`. Не блокирует текущую приёмку в пределах owner exception.

Builder выдаёт одинаковый isolated launcher для cmd.exe, Windows PowerShell и установленного pwsh; отсутствие pwsh записать как отсутствующий вариант, не устанавливать молча.

Для каждого shell:

1. Запустить `vibe shell -g` через подготовленный launcher.
   - **Ожидается:** child использует тот же shell family; `Get-Command`/`where` для fixture-команды указывает на isolated application proxy, не системную установку.
2. Выполнить безопасную fixture-команду и выйти `exit`.
   - **Ожидается:** команда запускается, возврат происходит в исходный shell; process-local выбор и PATH не стали постоянными.

**Evidence:** shell/version, путь fixture-команды, её marker/exit, успешный возврат. SSH exit0 без этих наблюдений не является таким результатом.

## 6. W-GUI — реальный gVim и Start Menu

**Основание:** MT-04 `STEP-3-LAUNCHERS`, `STEP-11-PROJECTION`; это GUI acceptance, не повтор автоматического exe-proxy test.

**Подготовка:** admitted Vim package с настоящим gVim, установленный в isolated settings. Export-system destination заранее проверен builder: HOME/APPDATA указывают на fixture; существующие Start Menu entries владельца не перезаписываются. Если entry нельзя безопасно увидеть вне реального профиля, сначала согласовать конкретный test-only destination, не выполнять обычный export в пользовательский профиль по догадке.

1. Выполнить подготовленную команду `vibe app start -g <exact-package>`.
   - **Ожидается:** открывается именно gVim с рабочим редактором, а не только процесс или пустое окно wrapper. Открыть маленький fixture-файл, ввести строку, сохранить только в test root.
2. Закрыть приложение обычным способом.
   - **Ожидается:** wrapper завершается корректно; нет лишнего зависшего cmd/PowerShell окна и потерянного foreground process.
3. После export dry-run и export открыть **точный подготовленный test shortcut** через Explorer/Start Menu.
   - **Ожидается:** правильное имя/иконка, открывается тот же isolated payload. Для TUI entry открывается terminal, для GUI — приложение; не принимается запуск другой установленной версии из PATH.
4. Builder удаляет только созданную fixture projection штатным remove-system.
   - **Ожидается:** test entries исчезли, чужие ярлыки и установки не изменены.

**Evidence:** screenshot без приватных данных, exact shortcut/payload path, файл с сохранённой строкой, результат закрытия и cleanup. Разрешение не выполнять Terminal hotkeys само по себе не отмечает этот GUI сценарий PASS.

## 7. M-GUI — Finder/Launchpad до рабочего приложения

**Основание:** MT-05 `STEP-PROJECTION`, `STEP-3-RUNTIME`.

**Подготовка:** admitted package, рабочий payload и fixture-only `.app`. Builder сначала выполняет dry-run, показывает точный destination и убеждается, что существующий `Vim.app` не будет заменён. Установка/изменение desktop projection — только в согласованной области.

1. Открыть подготовленную `.app` **из Finder**, не подменять это SSH-командой или чтением `Info.plist`.
   - **Ожидается:** виден правильный application name/icon; TUI Vim открывается в Terminal и получает interactive input. GUI launcher открывает своё окно, а не лишний terminal.
2. Открыть fixture-файл, проверить отображение и ввод, сохранить изменение в test root.
   - **Ожидается:** runtime resources/подсветка доступны из установленного payload; нет обращений к удалённому build directory или другой системной версии.
3. Закрыть приложение, повторно открыть тем же Finder entry.
   - **Ожидается:** повторный запуск работает и использует ту же isolated installation.
4. Выполнить согласованный remove-system для fixture.
   - **Ожидается:** только созданная projection исчезает; основной профиль и приложения владельца сохранены.

**Evidence:** screenshot/короткая запись наблюдения, app/payload/source pins, сохранённый fixture-файл и cleanup result. Успешное создание `.app` на SSH не заменяет визуальный шаг.

## 8. M-TERM — настоящий Vim в macOS Terminal

**Основание:** MT-05 `STEP-3-RUNTIME`, `STEP-4-SIGNALS`. Generic native signal42/43/137 и shell PTY evidence уже проверяют свою механику; вручную проверяется именно взаимодействие с настоящим Vim.

1. В обычном Terminal запустить exact isolated Vim proxy с fixture-файлом.
   - **Ожидается:** ввод и отрисовка работают, Vim использует payload runtime, файл можно редактировать.
2. Нажать Ctrl+C, затем выйти штатной командой Vim `:q` или `:q!` для несохранённых fixture-изменений.
   - **Ожидается:** Ctrl+C обрабатывает Vim, wrapper не исчезает неожиданно; нормальный выход возвращает рабочий shell без испорченного terminal input mode.

**Evidence:** наблюдение владельца и exit. Не требовать вручную повторять все уже проверенные signal permutations.

## 9. MEDIA — настоящий removable volume: use → unplug → refuse → return

**Основание:** MT-04 `STEP-7-STORAGE`, MT-05 `STEP-6-VOLUME`. Directory rename, mount-point imitation и mock unavailable path не доказывают физическое поведение.

**Подготовка:** владелец называет конкретный носитель и пустой directory; builder фиксирует mount/path, свободное место, exact installation key и готовит `storage add/move/list` команды для актуальной CLI. На носителе нет используемых этим сценарием чужих данных; приложение перед отключением закрыто. Не выдёргивать накопитель при активной записи.

1. Добавить directory как test library и штатно переместить fixture application в неё.
   - **Ожидается:** copy/publish завершён, index указывает на новый payload; checksum/fixture data сохранены, обычный proxy запускает приложение с носителя.
2. Закрыть приложение и безопасно извлечь диск штатными средствами ОС, затем физически отключить его.
   - **Ожидается:** только выбранный removable disk отключён. Если ОС сообщает, что он занят, не force-unmount: остановить сценарий и выяснить владельца handle.
3. Выполнить `app list` и попытаться запустить fixture через прежний proxy.
   - **Ожидается:** installation показана unavailable; запуск даёт понятный отказ с указанием недоступной library. Нет скрытой переустановки, потери записи, запуска другой версии или удаления данных.
4. Подключить тот же диск обратно. Если mount path изменился, использовать штатную attach/relocation procedure, которую builder проверил для этой версии, а не править JSON index.
   - **Ожидается:** library снова доступна; fixture данные сохранены; запуск работает.
5. Штатно переместить installation обратно в default library и повторить запуск.
   - **Ожидается:** data и proxy сохранены, состояние index согласовано. Удаление/GC — только в owned test области после проверки, не blanket cleanup диска.

**Evidence:** ОС/носитель/test directory, до/после payload hashes, unavailable diagnostic/exit, повторный запуск после возврата, сохранность данных. Серийные номера и имена приватных томов не публиковать без необходимости.

## 10. AGENT — осознанный consent и настоящая attended session

**Основание:** MT-06 `STEP-3-DECLINE`, `STEP-4-INTERACTIVE`, `STEP-5-UNATTENDED`, `STEP-6-PARK`; полный manual sign-off не выводится из fake-agent tests.

**Подготовка:** builder предоставляет fixture, где prompt разрешает только написать один безвредный файл в throwaway payload. Владелец выбирает существующий разрешённый wrapper/provider. Проверена применимая credential isolation и execution authority. Исторический `~/opt/bin/claude-codex` — пример из MT-06, **не текущая гарантия доступности или разрешение запускать его**. Не читать/копировать auth stores и не менять gateway ради теста.

1. Запустить fixture в attended CLI mode без автоматического consent; прочитать предложенный prompt и command.
   - **Ожидается:** scope понятен, actual test work directory указан, секретов нет. Сначала отказать.
   - **После отказа ожидается:** реальный агент не запускается, payload work не выполнен; outcome сообщает необходимость consent. Механика уже автоматизирована, здесь владелец подтверждает осознанность интерфейса.
2. Повторить в новом согласованном run, дать consent и наблюдать настоящий agent terminal.
   - **Ожидается:** агент получает предназначенный prompt в test work directory, выполняет только разрешённую запись; после штатного выхода `vibe` проверяет ответ, публикует payload и показывает понятный результат. Владелец видит, как вернуться к исходному shell.
3. Для отдельного run нажать Ctrl+C во время настоящей session, затем выполнить подготовленную resume-команду.
   - **Ожидается:** interruption отражён честно; нет зависшего дочернего агента; resume не повторяет уже завершённые эффекты и завершает оставшееся действие с валидированным ответом.
4. Сверить сохранённые prompt/context/result/transcript и видимый outcome.
   - **Ожидается:** receipts принадлежат exact run и mode; нельзя подменить результат старым fake-agent transcript. Provider-sensitive transcript остаётся локальным; в общий report идут только безопасные факты.

**Необязательно повторять вручную:** все combinations selection/default/external, no-agent, parked75 и fake unattended transcript уже покрываются соответствующими executable fixtures. Если требуется заново заявить current real-provider print mode, builder выполняет один отдельно разрешённый run; это не основание строить новый provider harness.

## 11. Форма протокола владельца

Для каждого реально выбранного сценария заполнить:

```text
Scenario ID:
Date/time and operator:
OS/version/CPU; physical desktop/terminal:
Source commit; binary SHA256; package/payload pin:
Isolated root; projection/removable destination if applicable:
Steps actually performed:
Observed result per step:
Status: PASS | FAIL | BLOCKED | NOT RUN / owner exception
Exit/run/log/evidence pointers:
Fixture cleanup and ordinary-user-state preservation:
Remaining limitation / explicit owner decision:
```

При FAIL достаточно первого конкретного несовпадения, логов и минимальных шагов воспроизведения. Не продолжать разрушительные или credential-related действия, чтобы «дойти до конца».

## 12. Как закрывать acceptance без искажения результата

- Builder готовит один конечный список actual remaining manual claims после автоматического gate, не просит владельца снова проходить весь test suite.
- Владелец выполняет выбранные обязательные сценарии или явно принимает конкретные tasks без них. `Owner exception` — acceptance disposition, не successful test evidence.
- Architect проверяет соответствие claims и результатов исходным требованиям; автоматические, physical и исторические receipts остаются раздельными.
- Сначала обновляются узлы/acceptance/evidence authoritative multi-user-planning, затем штатным renderer обновляется дерево; видимые evidence включают непроведённые сценарии и исключения.
- После завершения PRIOR-APP-STEPS действует выбранный владельцем STOP. Этот документ не запускает тесты, скачивания, server maintenance, Hermes/Pi или NST repair автоматически.

## Источники и границы

- `vibevm/vibespecs/manual-tests/MT-04-app-windows.xml` — native Windows, GUI/console/shell/storage.
- `vibevm/vibespecs/manual-tests/MT-05-app-macos.xml` — native macOS, Finder/runtime/Terminal/storage.
- `vibevm/vibespecs/manual-tests/MT-06-agent-hooks.xml` — настоящая agent session и согласие владельца.
- `campaigns/app-steps-2026-11/PRIOR-APP-STEPS-COMPLETE-01.md` — текущая finite scope и owner exceptions.
- `spec://org.vibevm.core/vibevm/common/PROP-059#acceptance`, `spec://org.vibevm.core/vibevm/common/PROP-064#acceptance`, `spec://org.vibevm.core/vibevm/common/PROP-065#display`, `spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP` — owning contracts; точные версии/CLI сверяются при подготовке комплекта.

Этот документ не изменяет owning contracts и не объявляет готовность неподготовленного physical run.
