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

### CANDIDATE-004 — unnecessary direct dependency stopped A1 locked verification

During M-01-A.1 the executor added a direct semver dependency to vibe-spec, requiring Cargo.lock outside its named write scope. The first exact test refused before compilation under --locked (selected-vibe-first.log). The executor removed the added edge and used the incumbent vibe_core::VersionSpec owner; Cargo.lock stayed unchanged and five initial exact cases each passed 1/1. This was an implementation choice failure, not a product runtime fault or justification to loosen locked builds. Evidence: /fast/git/v/research/2026-10-12-vibevm-first-agent-start/M01-A1/.

A1 independent source review found a pending candidate validation gap: public-field legacy SpecAddress can carry an invalid anchor into strict ResourceAddress conversion. Root sent a bounded regression/fix request through Zap (message.6a6939b7-c47f-4f63-9ad3-679523f5faa7); an executed red→green verdict is not yet claimed. The interim private export-name concern was withdrawn after the public path used validated package-owned declarations, so it is not counted as a product defect.

### ZAP-007 — queued G3 review message reached failed delivery

Authenticated read-only chat.page.v1 returned HTTP 200 and showed `message.0a5cb7af-2e80-4838-98df-95b7d4c5ab5f` as failed; it had originally returned queued. The later redundant request `message.0155f101-7718-4c52-bd88-0cf9f74d4be9` is answered and its native turn completed. This resolves OP-011's uncertainty about the earlier message: it is not still queued. The public message schema carries no failure reason, so no cause is asserted. G3 and the campaign progressed; do not repair the queue fundamentally on this observation alone. Current A1 correction message was still queued while its original source turn was live. Observe existing messages through chat.page/actual rollout, without reposting as a status probe.

A1 full-workspace cargo fmt --all -- --check returned exit 1 with 15 diff hunks outside A1; touched-file formatting is assessed separately. The existing formatting debt is not repaired in this source atom and the full gate is not claimed green. Evidence: M01-A1/cargo-fmt.log under the current campaign's operational evidence directory.

## VIBEVM-ADDRESS-001 — mutated legacy anchor bypassed strict value validation

Confirmed by exact regression `mutated_legacy_anchor_cannot_publish_an_invalid_resource`: 0/1 before, exit 101; after reusing the incumbent anchor-segment predicate in the strict value constructor, 1/1. Final new A1 corpus passed 7/7, zero ignored; two affected legacy/descriptor cases passed 1/1 each. Root reviewed the public conversion path, not just a private helper. Package-owned export membership is checked against selected group/name/version; the earlier export concern is withdrawn. These are logical identities, not physical-read or captured-byte proofs.

A1 operator check failure: unavailable `python` exited 127 before XML validation; `python3` repeated it successfully. Root's integration specmap generation exited 0, with zero unresolved host edges and the same 138 existing gated orphans; new source scopes and seven PROP-070 anchors are linked. Full workspace fmt remains red as recorded above.

ZAP-007 second occurrence: correction message `message.6a6939b7-c47f-4f63-9ad3-679523f5faa7` was queued at 02:06:14.337Z and failed at 02:12:40.546Z, immediately after the original A1 message became answered at 02:12:40.444Z. Original native task completed at 02:12:39.664Z. Independent source research identifies an idle-status/active-turn steering race as a reproducible hypothesis; the actual incident error code remains unavailable. See QUEUED-CHAT-FAILURE-REVIEW.md in the operational evidence directory. No replay or runtime mutation was made to diagnose it. A1 source correction nevertheless completed via the independent report and is not pending execution.

### LIB-CHECK-001 — existing safefs component family returned 15/16 in A2

During M-01-A.2 verification the existing `overlap_and_containment_use_the_same_component_boundaries` case failed on Linux. The executor reports a Windows-style Path/native paths_overlap mismatch; that cause is an assessment, not an independently executed fix. Current A2 diff does not change vibe-safefs source. The family result is 15/16, not green; six new A2 cases passed. Relevant split_relative/identity_key cases are being selected separately. Preserve this result in M01-A2 evidence, and do not repair unrelated component behavior merely to mask a red family. A2 acceptance remains pending.

ZAP-007 deterministic source reproduction: existing FakeProcess plus InProcessWorkspaceService/memory SQLite reproduced idle→completed as failed/stale_epoch with no followup RPC. Idle alone produced host_refused/failed after an old-turn steer; inverse order passed. Source-only candidate now selects start/steer under serialization, fences the captured process epoch and retains active admission until terminal completion. Targeted tests reported 18/18 including pause, restart, direct-steer and no-replay controls. Candidate is not yet Root-accepted or deployed; historical incident error codes remain unknown. Failed operator checks during this repair: test-config typecheck first found three candidate extraction/profile typing errors, then one post-extraction unused import; both were corrected with final exit 0. Selected lint initially rejected a non-null assertion/numeric interpolation and was corrected; final source report/log binding remains pending. All these tests used fake ports, not real agents/messages.

## VIBEVM-LOOKUP-001 — physical ID aliases were missed by A2 comparison

A2 exact regressions for an upper-case physical extension (.MD) and a terminal-dot physical file name each failed 0/1 before repair, exit 101. ID lookup now recognizes these as portable aliases and refuses without normalizing/opening the alias. Final A2 corpus7/7, A1 compatibility7/7, four affected resolver cases1/1 and four shared component/identity cases1/1. Root reviewed source/lock delta; the lock changed only by connecting the existing safefs dependency. Canonical specmap generation exited0, zero unresolved host edges and unchanged138 orphan debt. This is read-side preflight; race-free reading/native Windows/macOS remain unproven.

A2 setup failure: initial test redirection failed because its evidence directory did not exist (shell exit1, no test ran); after creating the directory, the command ran. Full workspace format remained red on the same15 unrelated hunks; package format passed. LIB-CHECK-001 remains an open existing shared-family failure, not fixed or claimed green by A2.

Queued-chat source candidate final verification:19/19, test-config typecheck0, selected lint/format0 and diff-check0. Independent Astra source review found no concrete defect. Source extraction preserved one FakeProcess; adapter594, events470, helpers398, old index test469, shared fake226, new race448 lines. Historical delivery cause remains inferred. Runtime remains unchanged while a matched backport is prepared separately; no old message is replayed.

ZAP-007 source accepted in Zap commit eff8aff. Matched old-runtime JS reproduced four failed invariants (7/11 before) and passed11/11 after an adapter/events-only semantic backport. Executable payload8152 entries:8150 unchanged; staged digest61250b5220f7eafde5fd5750c2a0f620b989ef1993024ccdd66a32083309f53f. Root verified patched-file/engine hashes. At the verified A2 terminal boundary, project.stop returned stopped/rev68 and the old Codex/Owner PIDs disappeared; Rust Zap service stayed live. Root launched the explicit staged wrapper with the same approved configuration, then Continue returned running/rev70 with the same session/conversation/native thread and a new process epoch. Main A3 actually started on restored CodexPID823461 under WayfinderPID819356. Original installed generation/launcher is unchanged: this is a reviewed operator runtime selection, not an installer-generation release. A queued effect-free ACK (message.dd8ba1bb-8413-4fa4-ace7-251bb820a7d0) is pending real terminal delivery proof; do not call that proof complete before its observed answer.

Operational attempts during backport/switch: derived JS test harness first had an overbroad import rewrite and failed syntax before any test; corrected. Optional SDK bare-root/CJS export probes failed MODULE_NOT_FOUND and ERR_PACKAGE_PATH_NOT_EXPORTED; actual exported module paths resolved afterward. Old supervisorhandle72513 was unavailable after graceful Owner termination; Root verified actual PIDs instead of restarting from an observation timeout. session.start on stopped state correctly refused409; explicit Continue succeeded200. These are setup/observation/guard outcomes, not additional provider crashes. All raw logs remain in QUEUED-CHAT-FIX.

Web reconnection: old gateway port retired during the intentional switch. First app-panel open was queued and its one-time redirect expired before navigation; Root's subsequent immediate browser open succeeded. Actual IAB tab7 authenticated new gateway46327, displayed VibeVM next and retained history, with pairing fragment removed. The renderer remains on4174. Project-specific pane was not separately exercised in this reconnection.

A3 in-progress failures: resolver/glob corpus initially1/3 because a slot-file error became absence and an ambiguous glob member disappeared; repaired3/3. Section reader and provider prompt duplicate-anchor probes each failed0/1 by returning the first body. Correction is ongoing, no final acceptance claimed. Executor read a small doctree.rs slice before its requested packet READ extension; no file mutation occurred, the procedural error is retained and Root explicitly granted the defining owner reads.

ZAP-007 live terminal proof completed: authenticated chat.page.v1 showed the actual A3 original message answered at03:21:12.069Z; queued ACK answered at03:21:14.810Z, with exactly matched reply QUEUE_BACKPORT_ACK_20261012 and causation bound to message.dd8ba1bb. The next separately queued scope grant answered at03:21:17.691Z. Both were posted once while the original turn was active and consumed automatically after settlement, without old-message replay or task restart. The current operator runtime resolves the observed delivery failure path. Exact backport remains the selected wrapper; ordinary installed default is still the old generation, so a future normal installation/update must carry source eff8aff rather than claim its original bytes fixed. Historical incident error codes remain unproven.

## VIBEVM-LOOKUP-002 — callers lost failure or selected the first duplicate anchor

A3 resolver/glob initial corpus1/3 reproduced a structural slot error reported as missing and an ambiguous pattern member silently excluded. Separate section-reader0/1 and prompt1/2 probes reproduced first-body selection for duplicate anchors. The repair propagates all non-DocNotFound pattern errors, refuses colliding member names, uses the existing duplicate-anchor index, and preflights descriptor-owned disk reads. Observed source disappearance or unequal successive byte reads is SourceUnstable; initial absence remains absent. No retained-handle/race-free guarantee is claimed.

Final A3 resolver8/8, read seam2/2, prompt2/2 and selected prompt1/1; two compiler and XML embed cases1/1 each. A1/A2 compatibility7/7 and incumbent glob12/12 passed. Root verified compiler frontend routes source_metadata to read_resolved_source and preserves errors. Canonical specmap exited0 with zero unresolved host edges and unchanged138 orphan debt. Package formatting passed; full workspace formatting retains15 unrelated hunks. A3 compile setup initially hit E0277 from a generic fs::read function argument, corrected to a closure. The first touched format check was red, then package-scoped formatting corrected it. Raw failed attempts remain in M01-A3 evidence. Root acceptance is recorded in campaigns/next/evidence/M-01-A.3/accepted-2026-10-12.md.

### OP-012 — technical scope answer used the wrong answer kind

Root first answered B1's single-choice question with short_text, causing409 invalid_input before answer commit. After reading the actual item, Root submitted optionlisted at its unchanged revision1 and got200. No duplicate chat was used as an observation. Exact result records are M01-B1/ROOT-READ-SCOPE-ANSWER*.json. Executor separately reported an early epoch.rs read before correction; no write occurred, the procedure error is retained. Root granted the five named defining owners and the independent seam review, with no write expansion. These are packet/answer handling failures, not provider/cache effects or source-authority proof.

### CANDIDATE-005 — B1 prepared fixture failed before authority construction

The initial four prepared-capture tests returned0/4, exit101 because the fixture used an invalid TOML package reference (`requires.packages.dep`). No constructor verdict was measured by those failures. After fixing only the fixture spelling, its four cases passed; two inert roster cases also passed. Raw attempts are retained under M01-B1. These six passes do not establish the separate visibility seal.

B1 current source review: public from_prepared accepted an arbitrary visible ID set and checked reachable closure, so it could admit a private-transitive dependency. The optional install bridge initially compared two caller-controlled root sets and accepted a caller owner ID; it did not close that bypass. Root sent addressed amendments at question revisions2 and3, both HTTP200, requiring owner-derived identity/direct roots and existing visibility computation or a bound verdict. Actual SafeFS snapshots/digest checks are retained as adequate capture inputs under the stated MVP boundary; no additional kernel/proof machinery is requested. B1 remains unaccepted pending the exact public-constructor counterexample/fix.

B1 public visibility defect resolved: the identical member→A→private B public-constructor regression failed0/1 before and passed1/1 after existing core visibility analysis replaced caller-set admission. Final corpus8/8, public install doctest1/1 includes actual snapshots and refusal of graph roots/edges/version drift. Early7/7 was insufficient evidence, not acceptance. The first correction compile failed on the override-table adapter, then passed after repair. Root traceability found three newly unlinked capture elements; existing contract scope added, with no new implementation behavior. Canonical B1 evidence retains raw failures and final limits.

B1 Root evidence packaging initially failed staged diff-check because normalized mirrors retained empty trailing lines. Commit was stopped; only repository mirrors were corrected to one terminal newline, raw evidence remained unchanged.

B2 Root dispatch first refused409 invalid_data before message creation: command omitted required causationMessageId. Corrected envelope accepted; no previously accepted task replay.

B2 packet Root named nonexistent crates/vibe-spec/src/source_map.rs; executor caught it before reading. rg --files confirms crates/vibe-spec/src/resolver/source_map.rs; exact READ correction sent, no WRITE expansion or task restart.

B2 scope correction completed through addressed question-group.152d9891 single_choice correct, HTTP200. Root question-list observation first omitted required state/limit and refused409; schema read then valid request200, no state mutation from failed read.

B2 candidate attempts: initial family compile failed101 (private slot_record import and fixture function shadowing), tests did not run. Next family3/4, selected descriptor case failed; diagnosis remains pending. Astra independently proposed Copy→InPlace lock/epoch mismatch counterexample from source; no executed failure yet. Root requested exact reproduction/correction within B2 before acceptance.

B2 candidate review followup: the3/4 attempt failed before descriptor construction because SlotRecord fixture file rows were not sorted, then corrected family4/4 and expanded6/6. B1/spec compatibility1/1 each passed. Astra found two additional initial-join omissions from source: lock materialization/root binding and descriptor source-path mapping with equal output bytes; exact red→green remains pending. Finite governing recheck now covers membership/manifests/records as well as lock, with a passing between-pass mutation case. Executor acknowledged review at04:24:21Z and is adding deciding fixtures; no acceptance or mixed-source production incident claimed. Raw logs/report remain in M01-B2.

B2 initial-join product defects reproduced: valid lock Copy→InPlace old-root and descriptor mapA/recordB each returned Ok, exact0/1 before; family6/8. Corrected materialization/derived-root and source-path joins gave8/8. Astra final source review confirms these joins closed. Remaining final-pass governing gap: second complete selected-file pass reads manifests/records before outputs, then only lock rechecks; changing descriptor during final outputs can still evade observed governing continuity. Root requested finite governing-only postcheck and exact final-output-phase fixture, not another whole pass.

### OP-013 — Root polling exhausted Wayfinder pairing sessions

Every isolated controller-client invocation paired anew and discarded its browser cookie. Owner ticket CLI then exited1 with generic `running Wayfinder did not issue a ticket`; authenticated owner control returned503 `Wayfinder pairing capacity is full`. Existing service/Wayfinder/Codex PIDs remain live. Source default is16 sessions and30-minute idle expiry. Root caused avoidable session leakage; no provider crash or task restart. Controller now persists/reuses one private cookie jar0600, creates a new pairing only on401. Recovery waits for an unused session to expire; no capacity bypass or active-session revocation. B2 pending PackageRef READ answer was not delivered on the failed authentication attempts.

OP-013 recovered: first corrected controller call paired successfully after idle capacity became available; next request reused persisted cookie and PackageRef READ answer returned200. Native Codex continued the same task throughout. No service restart or session-capacity bypass.

B2 final acceptance evidence: wrong selected edge pin leaf@2 vs epoch/row leaf@1 returnedOk in exact0/1 before, fixed by existing LockedPackage::as_package_ref comparison with valid-pin positive. Last-output descriptor mutation returnedOk in exact0/1 before; finite governing-file/lock postcheck after final outputs gives1/1. Final B2 family10/10, B1/spec compatibility1/1 each; packagefmt/diff0. spec_format equality concern refuted by incumbent citation owner expected_format=None, no blanket check added. Root clarified contract metadata closure versus visible output access to reflect actual visibility inputs. Root evidence observation used a nonexistent final-window-green.log name once (rg exit2); actual final-window-fixed.log and final-family files inspected and passed, no test replay. Native executor completed same task; no provider fetch or deferred campaign work.

### VIBEVM-OBSERVATION-001 — resolver used independent pathname reads

C1 source review found actual FsSectionSource consumer composed two fs::read pathname opens after portable preflight. Its historical cfg(test) algorithm accepts a distinct-object same-byte rebind; that example is not a restored-production red test. Production consumer now uses existing SafeFS bounded held snapshot, digest/parser input from its returned bytes. Both actual embed and SafeFS same-byte rename fixtures refuse final-name race and confirm different identities; final families4/4 each. XML/compiler/symlink cases1/1 each, ordinary lib check0, packageformat/diff0. Early replacement test changed bytes and was insufficient to isolate identity detection; Root required the deciding identical-byte proof. No compile/setup/selected-test failure occurred; old fullfmt15hunk debt remains. Root acceptance and source hashes live in campaigns/next/evidence/M-01-C.1.

C2 candidate source review found a public proof bypass: SourceMembershipIndex::from_verified_replay can accept equal raw selected/replayed hashes without canonical replay, and output_strength can promote that public value to SourceProven. Initial workspace1/1 checked shape only; install3/3 actual replay wrapper is optional, not closure of the public boundary. Astra source counterexample, no executed public-route failure yet. Root requested exact coordinated tamper/public-constructor reproduction and mandatory seal at existing replay owner before acceptance.

C2 public proof-seal bypass reproduced through actual public composition: pristine selected hash fixed independently, fresh B2 world after coordinated payload+record mutation locally valid; public unreplayed H,H index led to SourceProven. Exact public-seal-red.log0/1, exit101, not a malformed fixture. Correction is underway: workspace raw catalogue becomes inert/local strength, mandatory canonical replay owner alone seals opaque source proof and promotes strength. No hash recipe duplication or advanced security mechanism requested.

C2 public seal defect resolved: raw workspace catalogue no longer carries expected hash or grants SourceProven. Existing mandatory bounded replay==selectedH is the only minter of private VerifiedSourceMembership; promotion matches current selected pin/hash and original member bytes. Exact public-route0/1 before→green in final4/4 matrix; replay3/3, inertindex1/1, twocompatibility1/1. Astra final review found no further concrete defect. Larger/Tree1 proof remains explicitly unavailable and is deferred in B-229, not a new campaign blocker.

C3 research correction: locator reviewer initially guessed schema1, then read the actual core specmap index and corrected it to schema3 before any implementation choice. Root preserved schema3 content_hash meaning and selected explicit version4 carrier changes; no source patch followed the wrong guess. This is a research-status error, not a runtime/schema crash. Converter and overlay reviews identify existing pure kernel and Registry::load_with_witnesses preparation; standalone load+hash is not used as proof.

C3 proof-test placement correction: draft install child unit tests were placed under a lib harness configured test=false. Executor detected before claiming execution, requested exact integration target; Root granted tests/m_01_c_3_cases.rs. Production cargo check passed, but disabled draft supplied no proof and is not counted as a passing test. Actual integration matrix remains pending.

C3 first actual integration matrix0/4 failed during fixture setup: source document placed under spec/DOC.md instead of incumbent layout::current_specs_root(). Executor corrected fixture through layout API; repeated matrix4/4, zeroignored. No proof-logic verdict came from initial failures. Root review also found current positive only covered Markdown→XML output; exact existing declared-XML original positive requested within same integration target before acceptance.

C3 selected-locator source review: whole-slot SpecFormat Mixed was treated as document media and refused valid originals; VibeDoc semantic parser accepted doc XML while native-range parser used Spec-only vocabulary. Astra source findings, no executed defects yet; Root requested exact selectedtrace fixtures and minimum media/vocabulary fixes. Prepared selected explain is current API boundary; baseline explain(root) remains separately labelled carried metadata, no wholeCLI proof claimed. Sync active9sets clean after48operations/6specmapcopies; fullcheck1 from2preexisting unregistered Lean dirs, not touched.

C3 Mixed selected-slot case reproduced0/1 after valid fixture setup, rejection selected Vibe output; media/slot-mode dispatch correction underway. VibeDoc range deciding fixture had three manifest setup refusals (documents/title/abstract requirements), no locator verdict yet. Root granted exact manifest/package.rs READ to avoid further guessed fixtures. These setup failures are separate from confirmed Mixed product defect.

C3 locator corrections: Mixed deciding case0/1 before→1/1 after separating slot policy from descriptor/disposition-derived actual output media. Valid VibeDoc selected trace1/1 now uses matching vocabulary-aware native-span frontend; prior Doc manifest/lock failures were setup, not old valid-locator red. Exact multiline opening-tag case exposed startline5 vs3, then1/1 after existing event collector retained opening begin-position. Final affected families/evidence pending.

C3 final failed-attempt accounting: initial canonical engine test used root manifest and failed offline target-triple resolution, then ambiguous package name; correct nested canonical manifest executed4/4. First rustfmt child registration failed through integration #[path], fixed explicit registration. Independently authored XML test0/1 used invalid spec/planned token; fixture corrected spec/plan, final XML original proof included7/7. Relative rustfmt invocation from/tmp failed before Cargo, correctedcwd. RecordedOutput rename caused C2 row.format compile error, corrected slot_format with C2compat1/1. Orchestration SyntaxError stopped before nested effects. These are setup/compile/operator failures, separate from measured Mixed and multiline defects.

C3 final matrices: SpecDoc7/7, selectedtrace4/4, nativecore4/4, originalXML/builtinproof7/7, workspace2/2 plus affectedcompatibility1/1 cases. Both independent final reviews found no further concrete defect. Root verified79 candidate hashes and recompiled current xtask (schema4); first fresh map exposed4 new unlinked items. ExistingC3contract scopes added to derivation/canonical_node modules only, behavior unchanged. Final traceability pending below; wholeworkspacefmt remains red on13 outside-owner Diff entries (different count metric from old15hunks), and overallsync gate1 from2oldLean dirs is deferredB230. No wholeCLI/native/broadgate proof claimed.

C4 packet correction: strict CLI/orchestrator/applied-outcome owners were omitted from literal task packet; exact bounded READ/WRITE extension granted. Executor read specdoc/lib.rs before noticing missing READ, no source effect; procedure error retained. Astra source review identified whole-install strict-publication risk (early manifest/per-slot writes precede late invalid conversion), no executed failure yet. Exact goodA→invalidB previous-state fixture required; reuse preflight before first publication, no new rollback machinery.

OP-014 — Root routed VibeVM research/review directly to native Astra collaboration outside Zap while implementation used actual Zap actor. Owner corrected routing on2026-10-12. Review assignment now registered via Zap chat.post200; delivery queued behind current C4 work. No new direct native reviewer launches; actual independent reviewer/profile dispatch remains unverified until Zap evidence. Completed earlier reports remain evidence, not acceptance. Main C4 writer not duplicated/restarted.

OP-015 — First Agent response delay during C4: two bounded test-owner questions remained unanswered while Root handled owner discussion. The actual executor turn settled at 2026-10-12T07:21:34.131Z with public install/CLI cases still outstanding. Root then granted the exact fixture READ and CLI child WRITE through question.answer.v1 (both HTTP200), preserving the same actor/task. This is operator delay, not a Zap crash or a product test pass. Earlier routine grant responses and the owner mentoring/proxy documentation remained queued; canonical documentation is not yet saved.

OP-014 classification clarification: direct private-harness reviews and the staged ZAP-007 runtime repair are assisted interventions outside the ordinary customer path. Product task acceptance and successful normal-route queued delivery do not prove autonomous campaign management or customer repair without source access. For each future intervention retain cause, missing supported mechanism, change/evidence, ordinary-route retest and customer recoverability. Existing reviews remain evidence; new review requests route through Zap.

C4 further procedure error: executor read the first190 lines of slot_integrity_verify.rs before its exact READ grant. No write occurred; Root granted the defining fixture read afterward. Local operator command `python` exited127 before any controller action; corrected to python3. No task was restarted.

ZAP-008 — C4 provider capacity failure stranded pending chat. Observed native task_complete at2026-10-12T07:21:34.131Z carries codex_error_info server_overloaded and "Selected model is at capacity. Please try a different model.", no final agent answer. Public session.get.v1 is failed/rev155 at07:21:34.221Z; project execution still running/rev70 and original C4 delivery host_accepted with queued grants/documentation. This is a genuine terminal provider failure, not evidence that a live task needs more time. Ordinary queue cannot advance while descriptor failed. No original message replay, database edit or task acceptance occurred. Emergency read-only AstraMedium diagnostic outside stalled Zap route explicitly announced; report will be retained in operational evidence. Safe normal-command recovery is under review.

OP-016 — Root read an unfiltered Wayfinder startup log and exposed a one-time pairing URL in tool output. Root immediately identified the leak and switched to selected safe fields; never retain the URL/token in project records. The existing authenticated cookie client is reused. Additional source path guesses returned file-not-found before effects; no provider restart or source mutation resulted.

ZAP-008 recovery verified: public project.pause.v1 HTTP200 settled paused/rev72 at07:56:13.497Z; project.continue.v1 HTTP200 settled running/rev74 at07:56:31.475Z. Same session/process epoch and native thread retained. Persisted rollout observed task_started07:56:31.604Z, proving actual queue advancement. No original C4 repost, direct native RPC, database edit, source patch or process restart. Root-assisted normal control recovered progress; automatic retry without First Agent remains unproven and an operational improvement candidate. Source review/report: research evidence M01-C4/ZAP-COMPLETION-STALL-REVIEW.md, capacity control results beside it. This confirms recovery only, not C4 acceptance or eventual final reply for the failed original.

ZAP-009 — Independent managed review route unavailable. Actual C4 actor managed_work_profiles returned unsupported_operation: managed runtime not configured, reconnect through trusted transport adapter. Current preprovisioned binding forbids reconnect. No managed reviewer or independent acceptance produced. Existing single writer continues; Root review remains assisted campaign control. Deferred minimum configuration/capability investigation in standalone Zap BACKLOG P2-016; capacity recovery follow-up in P2-015.

First Agent resolved three stale coordinator questions through public question.answer.v1 HTTP200: M06G1 whole-lane once-only and M06G3 source-member skip use accepted receipts, without reopening accepted tasks. M14 selects preparation of exact eight-file import review when its stage arrives, without unreviewed blob import or new evidence schema. These are delegated First Agent decisions, not personally issued human replies.

ZAP-008 recurrence: restored C4 turn terminated again at2026-10-12T08:02:13.389Z with server_overloaded, no final answer. Before termination, actual public integration target compile failed E0425 because vibe_install::Result is not exported; no test verdict came from that compile. Root read the public model policy, then verified the live coordinator uses its separately configured gpt-6-sol/medium profile, not that policy tier selection.

Capacity model recovery: public Stop HTTP200 settled stopped/rev76 at08:05:20.179Z and old Codex PID disappeared. Root saved a protected machine-local configuration backup, changed only profile.codex.local model to gpt-6.1-sol, terminated old Wayfinder PID819356 after the stop boundary, and launched the same explicitly staged ZAP-007 runtime as PID1369040. Public Continue HTTP200 settled running/rev78 at08:06:54.707Z; same session/native thread/queue restored under new process epoch. Actual rollout task_started08:06:54.733Z and turn_context model gpt-6.1-sol/medium08:06:55.850Z prove new-model admission. Original C4 not reposted; source unchanged; C4 tests and final reply remain pending. Model failover required First Agent local configuration/runtime intervention and is not proof of automatic ordinary-customer failover. Protected backup/config/logs stay outside repository; no credentials copied here.

C4 public CLI fixture compile failure: common::spec_rel returns String but the fixture called to_string_lossy; corrected only the fixture type use. Initial cli-default-check-strict-initial.log preserved, no test verdict from compile failure. Corrected public warning/check/Fresh strict selector passed1/1, JSON applied counts1/1, public default apply1/1 and goodA→badB strict preservation1/1, all zeroignored. Final candidate verification/hashes/acceptance still pending.

Owner mentoring initial documentation accepted separately: standalone Zap227c62b adds five methodology and four protocol anchors, all spec/plan; VibeVM471db5397 adds operational crossreferences. Root parsed both XML, read new clauses and matched all three producer file hashes; scoped diff-check passed. This does not yet close later owner positioning/proxy/repair-responsibility supplements or claim implemented autonomy. No runtime/source change followed this documentation acceptance.

C4 Root review found an uncovered legacy path: transform_file for packages without explicit source descriptors maps failed Markdown parse without overlay to Ok(None); the caller records ordinary Copied, and strict preparation sees no failed-conversion row. Current green fixtures all use package.sources. This is a concrete source counterexample to full C4 coverage, not an executed red yet. Exact legacy default/strict reproduction and shared typed-outcome correction assigned to the same Zap writer in message.28aa5e1d; acceptance withheld. No scope expansion to a new architectural feature.

C4 legacy finding reproduced independently by executor during final source inspection: after fixing a missing fixture import, legacy-fallback-before-fixed-fixture.log0/1 fails the typed disposition assertion. Shared typed-outcome correction in incumbent transform_file gives legacy-fallback-fixed.log1/1, preserving ordinary Markdown outside spec-layout. Root observed the same uncovered path separately; do not attribute discovery solely to the queued Root message. No second parser engine or new feature introduced. Final source epoch and affected public compatibility remain pending.

C4 final attempt accounting from producer report: E0507 after non-Copy disposition fixed by authorized clone; wrong generated enum name fixed with incumbent alias; generator compile inherited E0507, then issue-list schema required x-empty=emit; relative generator invocation from/tmp refused before effects. One orchestration SyntaxError stopped before calls; broker wrong input shapes/missing ACK clientRequestId refused before effects, corrected exact grants491/492 only. Legacy first fixed retry used repo cwd and caused a separate fingerprint rebuild, then returned to/tmp. These are compiler/schema/setup/procedure failures, not successful tests or new provider crashes.

C4 accepted final boundary: current derived3/3, slot-record2/2, install2/2, CLI2/2 plus Fresh1/1, typed parser1/1, hardlink1/1, wire1/1 and registryepoch2/2, zeroignored. Root verified48 product source hashes and exact log hashes/results. Conditional codegenrepeat0, touchedformat/diff0, fullfmt1 on13 baseline entries. Fresh compiled Root specmap0: zero unresolved host edges, unchanged138 gated orphans. Legacy and descriptor failure corrections remain visible; no wholecampaign/release/platform or automatic Zap recovery claim. Canonical acceptance: campaigns/next/evidence/M-01-C.4.

Owner supplements fully promoted: standalone Zap04624ed records closed/small-model days-to-years multi-goal targets, algorithmic constraint model and limits, visual/text communication, scoped Codex strengths, persistent First Agent proxy provenance and continuing Zap repair/necessary-feature responsibility. Root read changed clauses, parsed both complete XML and checked13 clauses uniquely spec/plan; scoped diff-check0. VibeVM operational crossreferences retain exact source commits. This is durable project law/design, not installed-runtime qualification, autonomous campaign proof or permission to revive deferred Lean+Zap work.

OP-017 — Stale Root C4 frontier and redundant late crossreference. The Root review file still described pre-acceptance pending work after f7cd66691. Coordinator observed the stale text, checked the canonical receipt and correctly did not reopen C4. Root refreshed its own frontier to accepted/source commits; history preserved. A late executor paragraph repeated the already accepted repair crossreference; Root removed that duplicate only, leaving7317bf3fb content intact. No source/test change or acceptance rollback.

Queue continuation observed: after acknowledging remaining old C4 notices without redoing accepted work, the same actual Zap executor started M-01-D.1 at2026-10-12T08:38:39.000Z and announced contract/owner inspection at08:38:46.263Z. Original capacity-failed C4 reply settlement remains separately unproven; product49/222 acceptance is not proof one-original/one-final recovery. First Agent will batch additional guidance and avoid redundant resume messages while the actual actor is already progressing.

OP-018 — D1 packet omitted exact embed facade and named descriptor-baseline reports. Worker requested question-group.f750b76a instead of loading boot or reading ungranted owner. Guessed four leaf receipt directories were absent (rg exit2); Root owns the incomplete baseline pointer. Root verified existing docs/releases/developer-preview-1-to-2/evidence/{source-descriptor-a1,commonmark-a2,xml-data-a3,source-consumers-a4}-central-review.md and their acceptance commits ebe279c69/9bedfcf90/62144fe6e/d40d02cb6 as ancestors of HEAD. Public grant HTTP200 supplies exact READ embed.rs and all four actual reports plus defining contracts; no WRITE expansion/new gate/requalification.

OP-019 — Root git log --all failed exit128 on bad object refs/codex/turn-diffs/captures/1791620530533/4c0bcc6d-134f-48c5-ab9c-d2398db3b1b5/base. Cause unknown; no refs/history deleted or rewritten. HEAD-scoped log and all four exact ancestry checks succeeded, allowing D1 baseline resolution. This is local tool metadata/observation failure, not evidence of a Zap runtime crash or corrupt accepted product commits. Broader ref repair is deferred until needed.

D1 in-progress Root source review: selection valid_segment requires ASCII leading letter and splits dots, while incumbent xml_in_ids mint_heading admits nonempty trimmed single-line heading IDs and old canonical_node_hash matched literal IDs. Delegating the old public canonical hash to new dotted selector may drop valid numeric/underscore/literal-dot identities without changing recipe1. This is a source counterexample, not executed red yet. Exact public-parser/hash compatibility reproduction and shared-index correction assigned through Zap message.b24b8afb, same writer; acceptance pending.

D1 procedure attempts retained: worker broker question drafts first used wrong input wrapper/missing draft fields; invalid-arguments refusals preceded publication, then one question published. Over-budget source reads were reread in bounded chunks, not claimed complete. Root feedback request construction initially failed Python SyntaxError before any command, then controller request-file lookup failed; corrected triple-quoted request posted once HTTP200. No duplicate dispatch, source mutation or task restart resulted.

D1 selector checks: new selector family5/5 and existing canonical-node family7/7, zeroignored. These helpers do not yet prove actual compiler/workspace/index integration or the numeric/literal-dot anchor counterexample. Worker relative rustfmt from/tmp failed to locate files, while the shell still ran Cargo; format success was not falsely inferred from green tests. Corrected cwd/format verification remains to be bound in final evidence.

D1 actual-consumer integration correction: question-group.3ba24c04 identifies FsSectionSource section_text/read_resolved using DocTree after projection, which cannot retain table/Doc IR parity. Root granted exact WRITE embed.rs plus embed/selection.rs and READ descriptor_map.rs via public HTTP200; held capture and C3 actual-media/slot-policy distinction remain required. Current managed review route again unsupported_operation; no independent reviewer launched and existing ZAP-009/P2-016 gap remains deferred.

D1 workspace extraction first compile failed E0609: code assumed HostProvider.id while incumbent uses identity. Corrected host coordinate through existing field; same selected five-case family then5/5, zeroignored. Actual captured-world tests cover literal CommonMark/no fact facets, invalid XML/raw same-stem preservation, opaque invalid encoding, declared XML under misleading suffix/emitted basis and nested selection/candidate refusals. No actual compiler/index proof or anchor compatibility is inferred from those passes. Core actual index/explain tests are newly bound to native MD/XML CC-A-4-1/CC-A-4-2 and quote fixtures; before/fixed result still pending.

D1 canonical index/explain reproduction: first native fixture compile failed E0277 because generated types lack Debug; only diagnostic output corrected. Repeated exact test then0/1, missing spec://project/DOC#CC-A-4-1 while actual map carries only#root. This is deciding product evidence, not the failed fixture compile. Incumbent scanners' td/quote omission is being corrected within D1; no final fix claimed yet. Public question-group.781bd7be grants existing source_export.rs READ and generator-only six registered specmap mirrors READ/WRITE scope, HTTP200. No mirror writes before canonical tests/pre-check; unrelated engine metadata and existing B230 Lean debt remain outside scope.

D1 native table-fact range defect reproduced0/1: FIRST reported5..6 instead of5..5, including the next body row. Minimal incumbent range branch correction yields expanded core index/explain/native range/inline-code/header-structure family4/4, zeroignored. Affected old XML quote regression updated for D1's explicit new quote-fact contract; unanchored cell and literal-code negatives retained. Existing progress-core default grammar treats header/delimiter as structural; no generic header-fact capability is inferred. These scoped passes do not close full D1, mirror sync, actual embed, export join or anchor compatibility.

ZAP-008 recurrence in D1: gpt-6.1-sol/medium actual native task_complete at2026-10-12T09:20:48.250Z reports server_overloaded, no final reply. D1 source/logs preserved unaccepted. First Agent public Pause HTTP200 settled paused/rev80 at09:25:59.851Z, Continue HTTP200 settled running/rev82 at09:26:13.429Z; same session/process epoch, no source patch/model switch/DB mutation/original-task repost. Actual task_started09:26:13.502Z and subsequent compatibility-check commentary09:26:48.002Z confirm progress resumed. Operator-assisted recovery via public supported controls is available; autonomous retry/one-original-final settlement remain separately unproven under existing P2-015. Raw control results: research evidence M01-D1/CAPACITY-{PAUSE,CONTINUE}-RESULT.json. No duplicate writer or task acceptance.

D1 anchor compatibility source counterexample confirmed by actual-parser/public-hash tests: literal-anchor-hash-before.log0/2, numeric and literal-dot heading IDs both return None instead of their unchanged recipe1 XML preimage hashes. Not merely a missing fixture or invented new address grammar. Same writer now separates incumbent flat literal identity from dotted-path interpretation; no fix/acceptance claimed before deciding rerun. Provider recovery admitted this queued review without reposting original D1.

D1 literal-anchor regression corrected: select_exact_anchor uses byte-exact authored ID over the shared index; canonical_node_hash retains its old literal input and recipe1 XML preimage. General select_node returns ambiguity if literal-dot identity and dotted ancestry both match. Actual-parser numeric/literal-dot0/2 now covered by specdoc-selector-literal-fixed.log9/9, zeroignored, including collision negative and prior selector cases. Root reviewed actual lookup/hash wiring; full D1 consumer/sync/final-epoch acceptance remains pending.

OP-020 — Browser reconnection after First Agent Wayfinder model switch: prior tab7 was absent in current browser inventory. Root first passed numeric tab ID (tool type error), then stale string ID (not found), before obtaining actual inventory. New unpaired local page showed generic invalid-data transport/setup refusal while coordinator HTTP remained healthy. Normal existing-owner one-time pairing refresh delivered its fragment only to the intended browser login target; authenticated tab10 visibly loads VibeVM next and retained history, fragment removed. Old temporary tab9 closed; tab10 retained as deliverable. Screenshot evidence: research/M01-D1/ZAP-UI-RECONNECT.jpg, no credential in image or ledger. No runtime/source/session restart.

UI automation limit: project-button interaction timed out in CDP DOM.getNodeForLocation, Runtime.evaluate and Emulation.setFocusEmulationEnabled; accessibility and screenshot observations succeeded. Root does not infer a product navigation defect from those automation refusals or claim project-detail controls were verified. Authenticated project listing/history recovery is proved; detail navigation remains unperformed. This is operator-assisted ordinary reconnect, not stable pairing/session-identity qualification.

D1 materialized-media factory correction: question-group.02b5a05e requests descriptor_map.rs WRITE after its granted READ identified loss of actual media between captured factory and compiler facade. Public grant HTTP200 permits only existing same-witness media propagation (converted target versus copied/failed original) and preserves public row literals. Root response delayed during browser reconnect checks; independent export work remained available. This is operator latency, not missing human authorization or a new prerequisite. Actual embed/media integration tests remain pending.

ZAP-010 — Quick Lens frontend performance. Owner reported severe UI/graph slowness. Source inspection found per-event accumulated-history dedup/sort/copy, unbounded rendered activity, root activity subscriptions, canvas invalidations for output-only items, and Sigma recreation on selection/background loading. Bounded AstraMedium source candidate14files preserves every retained event, batches publication80ms, renders100rows/page, fences scope/history reads, separates output refresh, retains graph during background reads and updates selection via reducer. Root independently read source/hash-bound candidate; all14SHA256 match, affected diff-check0, existing worker evidence6Node+2Vitest tests and browser/test types/lint0. Illustrative Node3751-event merge1665.81ms→1.85ms is not browser latency.

Root frontend-only Vite production build exits0; pre-existing mixed async/defer warning retained. Verified old PID282772 preview command before graceful termination, started separately staged preview PID1581091 on same4174 and proved served index byte-equal staged output. Installed generation untouched; Wayfinder1369040, Codex1369856, Rust service240039 preserved. Authenticated browser project selection, Plan rendering, Zoom in and Back succeeded without previous automation timeouts. Initial activity DOM shows100of15966 events, remaining records pageable; later selected scope shows33events and Latest loaded correctly disabled on its onlypage (Root click attempt refused disabled; no missing-data bug inferred). Full browser latency/camera regression not proved. Narrow graph still has overlapping labels; retained memory grows; imported36/222 banner remains historical rather than accepted49/222. Standalone Zap BACKLOG P2-017 records these limits. Evidence UI-PERFORMANCE/{CAUSE-AND-PATCH.md,changed-source-hashes.json,ROOT-LIVE-RECEIPT.json,LIVE-GRAPH.jpg}. This is explicitly Owner-authorized First Agent Zap maintenance outside coordinator VibeVM route, not autonomous customer repair proof.

D1 connected-path proof: full compile_static/read_spec parity first0/1 and corrected comparison still0/1 because fixture included generated provenance/framing whitespace. Fixture now isolates exactly one payload and compares parsed IR plus canonical rendered content without changing authored raw bytes/parser semantics; workspace-actual-compiler-parity-semantic.log1/1. Actual facade embed family3/3 already green. Root read test and producer/consumer wiring and recorded current subject hashes in M01-D1/ROOT-CONSUMER-REVIEW-SUBJECT.json. Factory media, mirrors and final acceptance remain open. These fixture corrections do not imply byte-identical compiler wrappers or a raw-source corruption repair.

OP-021 — Root preliminary inventory named nonexistent D1 factory_cases.rs and proposed D2 read_spec.rs/spec.rs/schema registry paths before inventory. Read/search exits2, no write or dispatch; corrected to actual file inventory, explicitly marks D2 adapters proposed and format registry formats/REGISTRY.toml. No new product bug or missing accepted prerequisite inferred. Preparation evidence NEXT-M01-D2-PACKET-PREPARATION.md; it is not a task launch or product implementation.

D1 factory integration first compile failed E0308: fixture passed String to ContentHash::parse(&str), so factory-consumer-initial.log contains no executed test verdict. Same writer corrected borrow in fixture; no production defect inferred. Root reviewed actual materialise/read_validated_descriptor_map/FsSectionSource/compile_static fixture and found a weak copied-unconverted negative: any non-XML error could satisfy it, including unrelated missing anchor. One precise ordinary-route review queued as message.894c5659-b09d-4f17-a411-1996cbb95a46 requesting actual Markdown parse/media outcome and retained raw output; no new atom or duplicate writer. Deciding rerun and final acceptance pending.

D1 factory test rerun executes3cases:2passed/1failed. VibeDoc materialization rejects test package kind=flow as incompatible with declared media/Doc interpretation before compiler. Same writer requests exact READ of incumbent package.rs, package/meta.rs and derived/selection.rs, not weakening validator. First Agent public question.answer.v1 HTTP200 grants those three defining reads under Owner delegation; fixture-only correction within existing D1 scope. Raw factory-consumer-fixed-compile.log retained; final Doc/factory proof not yet green. Grant evidence M01-D1/ROOT-DOC-FIXTURE-READ-GRANT.json.

D1 final evidence selection error: final-spec-embed.log exits0 but runs0tests/995filtered, not a pass for facade obligations. First Agent sends exact actual family correction via public queued message.6dfcb93f-0787-46e0-8542-a034f1e651ff; previous3/3 retained, matching final epoch still owed. factory-consumer-valid-doc.log remains2/3 because fixture doc-kind manifest lacks required [[documents]] subject; actual validator refusal, not production behavior regression. Same writer continues correction under granted incumbent reads; no acceptance or broadened implementation.

D1 final acceptance: corrected Doc fixture and strengthened actual retained-byte/record/parser negative3/3; wrong compatibility zero selector corrected1/1. Final corpus38nonignored includes selector9,canonical7,workspace9,factory3,facade3,legacy1,world1,core4,quote1. All62 updated hashes and36mirror comparisons match. Initial Root acceptance guard detected a concurrent queued-review test update and refused before staging; final producer hashes/logs refreshed and native tasks terminal. Root compiled/current specmap0,0host unresolved/138oldorphans. Touched format0/fullformat13old entries; sync red solely B230. First Agent accepts via campaigns/next/evidence/M-01-D.1/accepted-2026-10-12.md; D2/full campaign remain open.

OP-022 — Root acceptance staging diff-check rejected trailing whitespace in copied rustfmt diagnostic lines, before commit. Normalized repository log line endings/trailing whitespace only, updated repository hashes and explicit normalization in manifest; exact raw evidence/hash retained. Product source unchanged.

D2 procedure/scope correction: executor probed nonexistent discovery/ directory (rg exit2); actual discovery.rs definition already granted and read. No source/provider effect. Additional exact READ resolves ResolvedDep/codegen/CLI context/native locator owners; common read_doc bridge writes granted narrowly in citations/sources.rs and proposed sources/selected.rs through public question.answer.v1 HTTP200. Existing whole-workspace discovery/all-slot from_lock behavior is not itself a defect; neither is admitted as blind point-query preparation. D2 preserves selected ancestor/visible closure so unrelated broken member/slot cannot veto point read. Governing new source-query clauses reviewed as spec/plan only; no implementation/test pass or acceptance. Grant latency records explicitly do not imply executor idle time. Evidence M01-D2/ROOT-{PREPARATION-DOC-BRIDGE,LITERAL-OWNER-READ}-GRANT.json and COORDINATION-METRICS.json.

D2 additional operator path mistakes: codegen.rs and CLI/context.rs requested as existing definitions were nonexistent; actual codegen/mod.rs already in granted subtree. Executor reports no read or product effect; no missing product owner inferred. First Agent grants reuse of existing specmap-core workspace dependency in authorized vibe-workspace manifest plus mechanical Cargo.lock update limited to that dependency row, all versions/checksums unchanged and offline. Central service reuses incumbent native MD range/C3 native XML range instead of adapter duplication. Grant HTTP200 in M01-D2/ROOT-NATIVE-LOCATOR-LINKAGE-GRANT.json; final actual lock diff/tests still pending.

D2 initial schema generation failed: mandatory candidates collection had no authored metadata x-empty policy. Registered generator refused before publishing the staged output; no successful generation/test claimed. Same writer adds explicit collection policy to source schemas; codegen-fixed-collection-policy.log completes generation. Root inspects current tracked output: existing module changes are schema-home boilerplate comments; generated root module/format-id registration and new source_query modules are expected. No manual generated fix, product runtime behavior or test pass inferred. Evidence M01-D2/{codegen-initial.log,codegen-fixed-collection-policy.log,ROOT-GENERATED-DIFF-REVIEW.json}.

D2 first wire contract1/1 green (request/descriptor/refusal closure and epoch); result/service/adapters not proved yet. Mechanical actual Cargo.lock delta is exactly existing core-ai-native-specmap edge under vibe-workspace; no package version/checksum changes. Broader offline cargo metadata failed on unavailable android_system_properties0.1.5 before HTTP request (offline forbids it), after correct lock linkage; no network/cache download or cross-platform qualification. Raw metadata-lock-resolved.log retained. Root finds outgoing cursor source counterexample: result validation only checks next-cursor presence and could admit empty/>256 while request rejects them. Not an executed red; exact existing-family correction sent once through ordinary Zap message.e3df0426-8fb0-4b6b-aa0a-e934dce8eb37. No acceptance/new framework claimed.

OP-023 — Root observer watched planned source_query/service* paths only and missed the actual permitted nested capture/continuity/service.rs layout. Manual git inventory detected implementation; corrected helper globs and live snapshot now list nested service/chunk. This is Root observation error, not stalled executor. Main session unchanged.

D2 first service source counterexample (not executed): outline/resolve return empty chunk total0 while selected_bytes is nonzero; wire result validator requires equality unconditionally, so valid non-read result would refuse. Exact mode-aware correction and actual positive modes requested through normal queued message.9623c9aa-d101-40c6-a90b-d0f4a45d8559. Existing UTF-8 chunk function explicitly refuses zero-progress scalar budgets; no UTF-8 defect inferred. Subject hashes ROOT-FIRST-SERVICE-REVIEW.json; service/preparation/adapters remain WIP, no service test green.
