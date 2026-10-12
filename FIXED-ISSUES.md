# Сбои Zap и PodBay при разработке VibeVM

Канонический журнал по указанию владельца2026-10-12. Записываем каждый наблюдённый сбой, обход и исправление; неразрешённые случаи сохраняем явно. Изменение исходников не считается установленным исправлением. Повторные наблюдения одного incident_id — обновления, а не новые баги. Для приоритизации сохраняем компонент, этап, симптом, причину или помеченную гипотезу, действие, стоимость/повторы и доказательство продвижения. Секреты, pairing-ссылки и личные сообщения сюда не попадают.

## Текущий запуск

### ZAP-001 — Не запущены Wayfinder и исполнитель

- Статус: исправлено штатным запуском, без исходного патча.
- Наблюдение: жив только Rust runtime; слушателя Wayfinder/веб-страницы и Codex executor не было.
- Действие: запущен установленный Wayfinder, выполнены настоящие pairing/session.start/chat.post.
- Проверка: Wayfinder242128 и Codex246395; native thread01a12d12-399e-7bc3-a994-ee6f03670e85 принял C4 packet и начал работу по deploy_profile. Это не приёмка C4/всей кампании.

### ZAP-002 — Потерян rollout старой сессии

- Статус: восстановлено существующим API, история сохранена.
- Симптом: session.start вернул409 unavailable: no rollout found for прежний thread.
- Причина: сохранённая execution-проекция running при отсутствующем старом исполнителе/rollout.
- Действие: project.stop exactrev62 → uncertain rev64; recover exactrev64/epoch1 → uninitialized rev65, старая сессия retained retired_uncertain; новый session.start после сверки состояния.
- Проверка: новое исполнениеrev66; старый ввод/эффекты не объявлялись отсутствующими и не переотправлялись вслепую.

### ZAP-003 — Recovery сообщает identity changed из-за состояния

- Статус: обойдён через штатный stop→uncertain→recover, без установки/патча.
- Наблюдение: установленный recovery принимает только uncertain; исходники уже допускают owner-attested running с теми же guards. Running rev62 отклонялся как identity changed.
- Направление будущей починки: точная ошибка о допустимом состоянии и автоматическая цепочка восстановления; не фундаментальная переработка.
- Проверка: та же установленная версия успешно восстановила exact uncertain rev64; работа продолжается.

### ZAP-004 — Веб-сервер страницы отсутствовал

- Статус: сервер восстановлен; подключение конкретного браузера ещё диагностируется.
- Наблюдение: порт4174 не слушался, хотя gateway37777 и runtime32192 работали.
- Действие: запущен Vite preview уже установленного renderer, без пересборки/изменения immutable generation.
- Проверка: HTML и JS/CSS HTTP200. Live frontend process handle20204; URL http://127.0.0.1:4174/.

### ZAP-005 — Пустой список проектов / invalid_data в браузере

- Статус: браузерный запрос и появление проекта подтверждены; повторное открытие через прямой gateway37777 подтверждено; диагностический proxy40193 остановлен.
- Наблюдение: после реального pairing IAB показывает историю, но sidebar пуст; reload/новая вкладка иногда invalid_data.
- Проверенные факты: bare project.list.v1 с одной authenticated cookie →200/один project.vibevm-next; лишние projectId/contextId →409 invalid_data. Установленный browser client/loadProjects уже использует Workspace project.list, не Product registry.
- Отозванная гипотеза: пустой Product registry исключает VibeVM. Она не подтвердилась; никаких fallback/дублирующих регистраций по ней не сделано.
- Последующая проверка: metadata-only proxy наблюдал именно IAB pairing200, authenticated bare project.list200 с одним проектом и чтения project/snapshot/execution200. После окончания загрузки sidebar показал VibeVM next Revision1. Причина первоначальной пустоты не доказана; ранний снимок загрузки недостаточен для продуктового диагноза. Исходники не менялись. planning/repository/managed-work возвращали unavailable; их готовность не заявлена. Позднейшая AX-проверка снова показала проект, но native click и DOM click завершились CDP timeout; открытие проектного экрана не подтверждено. Прямое подключение без proxy повторно показало VibeVM next Revision1; снимок web-direct-project-visible.png сохранён в operational evidence. Не смешивать ошибку управления браузером с доказанным дефектом приложения.

### ZAP-006 — FIFO сначала доставил старую wind-down команду

- Статус: согласовано без повторного ввода/нового исполнителя.
- Наблюдение: новая C4 задача стояла после старой queued команды завершения сеанса.
- Действие: дали завершиться ограниченному старому turn, затем настоящая очередь доставила исходную C4 задачу и её supplement.
- Проверка: старый turn выполнил два узких теста1/1; новыйturn01a12d18-aa7f-7b03-b7c5-105da6485c2a явно принял текущий packet и выбрал deploy_profile.

## Ошибки оператора и инструментария — не считать их багами продукта

### OP-001 — Локальные bearer-токены попали в tool output

- Статус: четыре секретных файла/ссылки конфигурации заменены0600 и runtime штатно перезапущен; старые bearer отклонены. Значения не повторяются/не записаны.
- Причина: диагностический blacklist пропустил ключ bearer. Ответственность Root и его воркера.
- Действие: whitelist безопасных полей, прежние principal/human IDs и история сохранены.
- Проверка: новыйreader200/старыйreader401; oldowner/coordinator runtime401. Data использует отдельный agent-channel, поэтому его полная независимая role-specific проверка и положительная проверка всех новых каналов пока не заявлены. Не выдавать частичную auth-проверку за полную.

### OP-002 — Неверные CLI и файловые предположения

- Статус: исправлены правильными существующими entry points; продуктовых патчей не понадобилось.
- Случаи: --help интерпретировался как operation/config; отсутствующий python заменёнpython3; неверный runtime_service.rs заменён runtime_service/mod.rs/application.rs; Root исправил неверный canonical backlog path и guessed mod.rs/schema filenames. В C4 codegen первый вызов из /tmp использовал относительный jtd-codegen и завершился127; абсолютный путь подтвердил установленный0.4.1, повторная генерация записала wire reader. Итог тестов этой правки ещё ожидается.
- Причина: Root/воркер предположили имена и флаги без проверки. Факты ошибок сохранены для статистики процесса разработки.

### OP-003 — Авторизованный документальный commit принят за конфликт writer

- Статус: источник расхождения объяснён и task packet обновлён.
- Наблюдение: HEAD34359633b→9ca242fde во время старого узкого теста; executor остановился для проверки.
- Причина: Root добавил только incident ledger10строк, продукт/задачи/schema неизменны.
- Действие: подтверждён docs-only delta, дальнейшие HEAD-коммиты во время активных source/tests отложены; совместимые проверки не превращаются в потерянные.

### OP-004 — Путь файла ошибочно передан как artifactRef

- Статус: запрос исправлен, задача доставлена существующему исполнителю.
- Наблюдение: Root передал путь packet в artifactRefs; API вернул409 storage_failure. Контракт требует opaque ID, а не путь.
- Проверка: после отказа idempotency rows для этого request отсутствовали; повтор с artifactRefs=[] и путём в тексте получил200 host_accepted, затем turn_started23:09:19.200Z.
- Будущее улучшение: ошибка валидации должна объяснять неверный artifactRef. Продуктовый патч сейчас не требовался.

### OP-005 — Первый Агент задержал ревью готового результата

- Статус: промежуточная правка проверена и принята c2dc48f56; следующий turn уже работает.
- Наблюдение: task_complete22:40:30.073Z → следующий task_started23:09:19.200Z:1729.127секунд (28мин49сек).
- Причина: Root занимался диагностикой UI и не принял готовую правку вовремя. Это ошибка управления, не сбой Zap.
- Действие: отдельный read-only монитор следит за public commentary/final и реальным процессом; Root принимает результат перед необязательной диагностикой.

### OP-006 — Ошибка типа в первой компиляции C4 profile selection

- Статус: исполнитель исправляет и повторяет тот же check; исполнитель сообщил об успешных повторных compile checks orchestrator/MCP/CLI; тесты нового поведения ещё выполняются.
- Наблюдение: compile check exit101: новый вызов SelectedFileWitness::from_bytes передал &Vec<u8> вместо Vec<u8>.
- Классификация: ошибка текущей реализации, не эксплуатационный сбой Zap/PodBay. Два setup-вызова с относительными путями из /tmp не запускали generator/formatter и не меняли продукт.
- Свидетельство: current-turn-public.jsonl, public commentary2026-10-11T23:19:28.014Z; отдельного нового исполнителя не запускали.

### CANDIDATE-002 — Повтор ключа с другим разрешённым Deploy-профилем

- Статус: воспроизведено текущим C4-тестом; executor сообщил о прохождении owner rerun2026-10-11T23:27:02.280Z. Root просмотрел comparison diff; Root проверил точные логи: owner2/2, bridge10/10, CLI10/10, MCP6/6, negotiation7/7, wire1/1+1/1. Узкий candidate принят в b70c25d82; весь C4 не закрыт.
- Наблюдение: новый host принимает профиль из allowlist; incumbent retained_mechanism.rs при existing key пропускает новое selection и сразу возвращает cold owner observation. Изменённый разрешённый профиль может получить результат исходного запроса.
- Действие: через Zap доставлен queued review message.ef5b59e0-5443-417f-8a24-546a8ed54826 (HTTP200), с требованием same key/A→B conflict и same key/A→A attach без второго эффекта. Это блокер приёмки текущего нового поведения, не повод возобновлять Zap+Lean.
- Воспроизведение: public commentary2026-10-11T23:23:24.893Z: успешный alt Deploy → legacy e1 submit с тем же C1 key вернул result вместо ожидаемого error;0/1,exit101. Исполнитель исправляет incumbent C1/Deploy owner comparison.
- Дополнительный source-review case2026-10-11T23:39:35.673Z: explicit primary-profile request и legacy omitted-profile request имели одинаковый resulting plan. Executor сохраняет requested_deploy_profile только в новом canonical intent, старые e1 bytes остаются прежними; оба направления проверены в owner family2/2, legacy bridge10/10 сохранён. Не считать этот source case отдельным воспроизведённым runtime incident до тестового evidence.
- Ограничение: существующий fresh owner требует один authored target; поддержка реально разных targets пока не доказана.

### CANDIDATE-003 — Новый format стал обязательным для legacy frame clients

- Статус: executor сообщил о прохождении exact cli_mcp_fixed_deploy10/10 и unnegotiated-profile negative1/1 после исправления; Root проверил negotiation diff. Узкий candidate принят в b70c25d82; полный conform verdict остаётся красным.
- Наблюдение: legacy target6/10 failed на NoCommonSchema до исполнения. Server требовал шестой schema offer, старые e1 клиенты предлагают пять.
- Причина: ошибка текущей adapter-правки, не эксплуатационный сбой установленного Zap/PodBay.
- Действие: сохранить старые пять как допустимый набор; profiled request доступен при согласованном шестом. Повтор того же точного target, без общего расширения кампании.
- Свидетельство: current-turn-public.jsonl public2026-10-11T23:31:13.714Z.

### OP-007 — Смена cwd повторно пересобрала codegen xtask

- Статус: command завершён; executor сохранил два byte-identical codegen runs, итоговый digest в C4-NEXT-PROFILE-REPORT.md.
- Наблюдение: repeat codegen запущен из repo root вместо прежнего /tmp; изменился Cargo build fingerprint, xtask повторно компилируется в том же private target.
- Причина: setup-ошибка исполнителя; точная дополнительная длительность пока не измерена. Не считать продуктовой регрессией.
- Свидетельство: current-turn-public.jsonl public2026-10-11T23:34:27Z. Будущие одинаковые checks должны сохранять cwd/environment/config.

### GATE-001 — wire-diff refused by unchanged index fixture projection

- Статус: общий gate exit1; причина до нового формата, сохранена в BACKLOG B-227. Зелёный глобальный verdict не заявлен.
- Наблюдение: rebuild --check сообщил drift formats/corpora/index/e1/repomd.json относительно собственного journal. Root проверил отсутствие Git delta у обоих.
- Классификация: несвязанный с текущей schema/adapter-правкой baseline gate failure, не эксплуатационный сбой Zap/PodBay. Узкие wire/codegen результаты оцениваются отдельно по affected-targets policy.
- Действие: не чинить index фундаментально в C4; сохранить точное evidence и вернуть к проверке итогового полного panel, если тогда он продолжит отказывать.

### GATE-002 — scoped vibe-mcp conform reported100 findings

- Статус: красный verdict сохранён; старый долг отложен в BACKLOG B-228, candidate-added violations убираются.
- Наблюдение: initial100 findings при пустом baseline, большинство старые; native_tests.rs689строк. Два expect в перенесённом negotiation модуле относятся к текущей правке.
- Действие: исправлять только введённые текущим candidate нарушения. Не заявлять crate conform зелёным по одним целевым behavioral tests и не угадывать итоговое remaining count.
- Свидетельство: public2026-10-11T23:38:16.191Z, conform recheck:98 findings,exit1; negotiation-модуль не добавляет findings.

### OP-008 — Root placed a Markdown paragraph after XML child sections

- Статус: исправлено до коммита отдельной XML-секцией; финальный specmapexit0 и0 unresolved host edges.
- Наблюдение: standard XML parsing прошёл, но project XML dialect отказал: p cannot follow nested section. Производная карта потеряла102 units и получила252 unresolved host edges; exit0 генератора сам по себе недостаточен.
- Причина: моя ошибка размещения новой записи PROP-080, не отказ рабочего C4 кода. Root исправляет только эту запись.
- Дополнительная setup-ошибка: direct xtask без CARGO_MANIFEST_DIR отказал до генерации; повтор использует явный manifest dir.

### CANDIDATE-001 — Перезаписан существующий тест в остановленной B3-правке

- Статус: незавершённый candidate сохранён в бэклоге; Main не затронут и патч не интегрирован.
- Причина: Root packet ошибочно назвал existing workspace-store/recovery.test.ts новым.
- Действие: остановлены старые доработки; immutable baseline и partialworktree сохранены. Восстановление/merge обязательно только если candidate понадобится для конкретного следующего блокера.

## Install missing-report verification — no observed product defect

The existing exact-local Install path passed MCP family5/5 and a cold CLI/MCP fixture1/1, with0 ignored. Accepted test commit is recorded in Git as test(api): retain partial install truth across cold adapters. After completed Install and fixture report deletion, all adapters retain the original install run, partial view and uncertain effect; cold inspection leaves the C1 index and lock bytes unchanged. This establishes reading that scenario, not automatic reconstruction of a lost final report or full C4 completion.

Operator read-only path mistakes in this step: worker queried absent lifecycle_adapter/decision.rs and machine_operation/exact_local_install/; actual definitions are lifecycle_decisions.rs and exact_local_install.rs. Root also guessed an absent exact_local_install/tests.rs before resolving source files with rg --files. No product state was changed by these failed probes. Exact selected test logs and report are preserved under research/2026-10-12-vibevm-first-agent-start; event times for the probes were not separately captured.

## Acceptance-binding boundary review — pending classification

Root independently found that the five C1/C2 lifecycle-route matrix does not classify the registered effectful manifest_migration_operation tool. The tool has apply/inspect/recover and its own explicit host authority and migration journal. A missing classification is not yet a demonstrated implementation defect. Root requested a bounded read-only source/contract/receipt addendum through Zap (message.d81469a2-752c-48e4-b3d7-8f985e957eed); no new migration implementation or competing journal is authorized by this finding. Boundary resolved before acceptance: PROP-015 MIGRATION-CELL-LAW assigns migration its sole PROP-044 contract without hosted lifecycle/provider authority. Root accepted C4 lifecycle scope in campaigns/next/evidence/M-16-C.4/accepted-2026-10-12.md; migration runtime completion is not newly claimed.

## PodBay

Отдельного нового подтверждённого сбоя PodBay в этом операционном запуске пока не выявлено. Старые незавершённые задачи не превращаем в новые инциденты без реального воспроизведения. Они сохранены в Zap BACKLOG P2-014 и полном deferred inventory.

## Свидетельства и стоимость

Подробные последовательные события: campaigns/next/operations/INCIDENTS.jsonl и /fast/git/v/research/2026-10-12-vibevm-first-agent-start/{START-REPORT.md,progress.json,incident.jsonl}. Время/стоимость считать по измеренным данным этих записей; неизвестные величины не оценивать задним числом. Текущая цель — продвижение VibeVM; чиним только наблюдённый блокер очередного этапа.

Operator probe correction during boundary review: Root first guessed short PROP-015.xml/PROP-044.xml filenames; rg refused. Resolved actual PROP-015-mcp-integration.xml and PROP-044-change-native-formats.xml via rg --files and checked the sole-contract clause. Read-only failures changed no product state.

### OP-010 — Root committed log mirrors after whitespace check refused

Root's shell sequence continued to git commit after diff --cached --check reported trailing blank lines in four log copies. Corrected by EOF-normalizing only repository mirrors and recording raw/repository SHA256 pairs; external raw evidence remains unchanged. History is preserved. Future multi-command mutation scripts must stop on failed checks. This is an operator command-guard failure, not a product or test failure.

## B-224 — normal compiled units repeat in the final consumer lane

Current M-06-G.1 fixture reproduces the ignored whole-lane regression: alpha2 instead of1, exit101,0/1. Separate units are internally once-only; combined lane repeats1/2/3/3. Root selected final-consumer-lane identity-based once-only under existing PROP-035/038; PROP-009 partial compiled-artifact residual does not excuse direct normal entries. This is an actual task blocker, not a new qualification prerequisite. Native candidate and preservation/short-link/order proof remain unaccepted. Evidence: M06-G1/REPORT.md, cross-entry-ignored.log and B224 scope/hook reports under the current startup evidence directory.

M06-G1 operator fixture failure: first #source placement test reused root anchor in multiple merged sources and correctly failed duplicate-fact validation, exit101. Corrected fixture uses distinct anchors; exactcase1/1. Simple/normal, embed, unresolved-precommit and perunit/wholelane characterization each passed1/1. NativeWindows remains unperformed; no product pass is inferred from the failed draft.

B-224 nested follow-up: Root review identified top-level-only traversal. Executed nested H2 fixture failed0/1,exit101 with BETA_NESTED_SOLUTION twice. Native switched to existing DocTree::sections() and structural text_without spans; exact nested retry1/1 retains parent and unused sibling. Post-fix affected checks and candidate acceptance still pending. Raw first/fixed logs are preserved in M06-G1/nested-overlap-first.log and nested-overlap-fixed.log.

B-224 preamble follow-up: three distinct normal entries partial→whole→direct reproduced BETA_PREAMBLE2instead1,exit101. Same-document preamble coverage corrected in the existing linker; exactcase1/1 and7affectedrechecks1/1each. Root later found candidate files656/687lines; new helper/tests extracted into sibling modules, final sizes578/85/494/199. Nine post-extraction exactcases1/1,0ignored; Root reviewed source/logs and accepted G1/B224. Native Windows remains open. No further product failure or broad baseline repair is claimed.

## VIBEVM-INIT-001 — init package silently changed XML host to Markdown

Actualcurrentbinary returnedexit0 yet removedSTATIC.xml, createdSTATIC.md andchangedINDEX/launcherblocks. ExactXMLregression0/1,exit101; derivedvibedepsbefore/afterhashesmatch. Cause: inithelperunconditionallypassedSpecFormat::Mixed. Fixed byusingrootconsumer'sdeclaredformatwithMixedfallback. FourexactCLIcases1/1,0ignored verifyXML,Markdown,defaults,marker; RootreviewacceptedM06G2. Incumbenthelperandisolatedtestmoduleonly; no install/cacheframework. Fixture setup's initialnonTTYinstall refusal was expected preeffect and rerunwithexplicitassume-yes; retained separately fromproductfailure. Windowsproofunperformed/open. Evidence campaigns/next/evidence/M-06-G.2/accepted-2026-10-12.md.

## VIBEVM-BOOT-002 — generated package lanes entered authored simple inputs

M-06-G.3 current-tree XML fixture failed with exit 101: a generated multi-document STATIC.xml was parsed as a single authored spec. Two materialization traversals copied generated boot artifacts; after filtering those, static composition still substituted the newly generated unit cache as one simple input. Candidate filters both traversals and supplies the authored static zone's members/fragments separately. XML/Markdown nested multifragment, derived materialization and three existing affected checks each passed 1/1. Candidate remains unaccepted pending the complete G3 boundary. These successive red probes are attempts on the same defect, not three independent incidents.

## VIBEVM-BOOT-003 — consumer install writes into source package members

Open. Exact `consumer_install_keeps_source_workspace_member_bytes` failed 0/1, exit 101: install added AGENTS.md, CLAUDE.md, GEMINI.md, boot/INDEX.md and boot/.vibe-boot-artifacts.lock to an authored workspace package member. Root is resolving the publication boundary; no completed fix or successful CLI check is claimed. Evidence for both boot defects: /fast/git/v/research/2026-10-12-vibevm-first-agent-start/M06-G3/IN-PROGRESS.md and its named raw logs.

G3 operator/candidate failures: two compile attempts missed newly threaded arguments/imports, then were corrected. Three disposable CLI setup attempts refused (missing offline registry, package absent from ad hoc registry, incomplete installed-world projection); they do not establish a product check verdict. Raw failed attempts are retained in the same evidence directory. Source executor completed its attempt and waited for Root's semantic ruling; that wait is not a crashed worker.

G3 continuation: Root ruled that selected non-root package members, matched by kind/group/name rather than transient source path or version, are protected source inputs in consumer operations. Both publication branches and redirect validation now use that policy; root/project/unselected entrypoints and consumer dependency-unit emission remain. Executed source snapshot/durable version-drift, standalone root, project-member and unselected-member cases passed 1/1 each. A valid current-binary CLI fixture passed install, member check and root check with exit 0, unchanged source-member hash inventories and the selected body once in root STATIC. Independent Astra source review found no concrete publication-policy defect. These results are candidate evidence until Root acceptance.

Static-zone follow-up reproduced the same BOOT-002 failure with package-suggested static link, exit 101. Candidate now follows declared link → snippet suggestion → node default; the combined XML/Markdown fixture covers all three. Additional operator failures: one native exact selector ran 0 tests and was rejected; the corrected module-qualified selector ran 1/1. A manual CLI fixture initially lacked the host boot directory and failed before install. A project-member fixture initially looked for dynamically inherited foundation text inside STATIC instead of its INDEX entry, then was corrected. Failed probes remain in raw evidence; they are not additional product defects.

### OP-011 — Root reposted a queued command to inspect delivery

I incorrectly used a repeated `chat.post.v1` as an observation of delivery, assuming the same clientRequestId would reuse the message. The second response returned a different message ID (`message.0155f101-7718-4c52-bd88-0cf9f74d4be9`, host_accepted) from the earlier queued message (`message.0a5cb7af-2e80-4838-98df-95b7d4c5ab5f`). This created a redundant G3 review request; no separate executor was launched by Root and no external effect was requested. Treat clientRequestId as correlation unless deduplication is separately proven; observe through the actual rollout/read surface, never by reposting. Whether the earlier queued message will later be consumed remains unproven. This is an operator mistake, not a claimed contract violation by Zap.

G3 accepted by Root: final twelve owner cases 1/1 each; five affected cases rerun after precedence change 1/1 each; separate suggested/default red→green and explicit-static/dynamic-override cases 1/1. Fresh final CLI install, member/root check and repeat install exit 0 with equal complete source-member hash inventories. BOOT-002 and BOOT-003 are resolved within the recorded consumer/publication boundary. Canonical receipt: campaigns/next/evidence/M-06-G.3/accepted-2026-10-12.md. Windows and broad campaign gates remain open; they are not claimed by these results.
