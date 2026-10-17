# M-05-E.4 — offline correction delta review

**Recommend accept original scope: 0 P1, 0 P2.** Предыдущий единственный P2 закрыт. Это independent advisory review; acceptance/commit/mirror остаются у Root. Предыдущие REPORT/RESULT сохранены без изменения.

## Связь candidate и actual WIP

HEAD остаётся `644bbd0de25e1335e21e513b3228c1a688256bf0`. Новый candidate `managed-result.b6f7ddc29992e5180dda72458182d21c5433751695dd503db1417279c188e689`, collected revision11270. SHA256/размеры report `9fd9e9713e22a5ec6e62a36be356c564ded5f078aebe3282648d8e8437d8df84` (1227983B) и patch `fa8eb56f6ed961542e6fb999871f3aced441aea74b78467bf187c2c0cc4867ba` (79026B) проверены по bytes. Все26 actual paths (22 tracked,4 untracked) совпадают с patch. Read-only `git apply --reverse --check` вернул0. Полный прежний report действительно встроен в новый побайтно, включая failures.

Сравнение с26 сохранёнными SHA предыдущего review подтвердило **ровно5 changed /21 byte-identical**. В RESULT.json записаны все текущие/прежние hashes.

Delta:

- `crates/vibe-cli/src/commands/install/direct.rs`: передаёт root_offline в существующий choice adapter.
- `crates/vibe-cli/src/commands/install/choices.rs`: штатная effective offline ladder, early mismatch refusal и один новый arm в существующем CLI case.
- `crates/vibe-orchestrator/src/machine_operation/configuration_choice.rs`: pure Questions публикует immutable retained InstallRequest.offline.
- `crates/vibe-orchestrator/src/machine_operation/configuration_choice_fixture.rs`: online-posture counting/refusing source fixture для этого arm.
- `vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml`: только уточнение existing M05E4-PREPARED-OPERATIONS про сохранение offline posture.

## Почему прежний P2 закрыт

`direct::run` теперь сохраняет root flag при передаче в choices. Adapter загружает UserConfig и вызывает тот же incumbent `output::resolve_offline(root_offline || args.offline, user_config.net.offline)`. Проверенный helper объединяет flags, VIBE_OFFLINE и config; отдельный новый policy resolver не появился.

Первый запрос — только существующий pure Questions над retained host; source preparation/acquisition и apply там отсутствуют. Если caller effective offline, adapter требует `reply["offline"].as_bool() == Some(true)`. False, absent и wrong type отказывают до любого Evaluate/Prepare/Apply и до dialoguer. Non-needs/unavailable ответ возвращается без последующих операций. Онлайн-caller может использовать более строгий offline host. Host/source не реконфигурируются клиентским флагом: это проверка совместимости с caller-trusted immutable host, а не новая authority из произвольного JSON.

Source diff не меняет ChoiceOperation selection, shown-definition/default binding, B2 evaluation, C1 source transaction, pending plan identity, E3 apply preconditions или D1 persistence. Остальные21 файл, включая MCP adapter, resolver projection, wire codec tests и исходные owner guards, побайтно неизменны.

## Deciding regression и retained checks

В том же `commands::install::choices::m_05_e_4_cases::production_install_machine_and_tty` добавлен один offline arm. Он идёт через normal parser/direct adapter с explicit shown/default consent, root_offline=true и online-posture host. Его source increment+panic при normalized transaction/solve/metadata/fetch/materialization делает ошибочный source entry наблюдаемым. Child получает expected offline-host error; parent проверяет counter==0 и отсутствие vibe.lock/vibedeps. Это проверяет реальный guard, а не заранее записанный final state. Существующие JSON missing/default и настоящий PTY radio/checkbox/apply arms остаются и проходят.

Свежие встроенные логи проверены отдельно от producer dispositions:

- CLI:1 passed,0 failed,966 filtered,0.34s; включает mismatch и retained JSON/PTY story.
- Service:7 passed,0 failed,668 filtered,0.13s.
- MCP:2 passed,0 failed,219 filtered,0.14s.
- Scoped rustfmt и diff-check имеют пустой diagnostic output; report сохраняет exit0. XML дополнительно распарсен read-only при review.
- Global fmt — **Failed**, те же7 baseline paths. Они отсутствуют в changed-path inventory. Никакого green/waiver этому check не присвоено.
- Wire2 и B2 9 — прежние успешные observations, не новые запуски. Их owner bytes неизменны, полный старый report сохранён. Новых schema/registered-format/epoch изменений нет; прежняя оценка conditional codegen/wire применимости остаётся в силе, global pass не заявлен.

Новые тесты самим reviewer не запускались; builds/models/API/store/process/Git/source mutations не выполнялись. Дополнительная матрица root/local/env/config не нужна для решения этого delta: production использует существующий общий helper, а один arm решает конкретный прежний missing-input defect.

## Original scope и specs

Оба исходных критерия неизменны: один CLI/service/MCP request + prepared effect graph; machine завершение или typed input requirements без TTY wait. Предыдущий full semantic review подтвердил shared host, настоящие adapter/Server ingress, attended PTY, exact definition/default consent и D1/E3 owners; пять исправленных файлов это не ослабляют. Universal host lifecycle/новый MachineSession format/новая recovery campaign не требуются.

Все6 E4 anchors остаются `spec/plan`; меняется только offline sentence в existing prepared-operations anchor. Независимые предыдущие законы не удалены, статусы не promoted, acceptance не записан в спеку. Рекомендуется Root принять исходный E4 scope по совокупности прежнего review и этой correction, сохранив baseline fmt failure как есть.
