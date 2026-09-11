# PP-C1 Проверки прозы без инструментов (P.6 и P.7)

Дата: 2026-09-12
Дерево: b1291b06

Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C1.md` (преамбул `campaigns/docs-2026-09/findings/PACKET-COMMON.md`). Ничего в репозитории не правлено; только чтение и этот отчёт.

## Самопроверка (сводка)

| метрика | значение |
|---|---|
| страниц разобрано | 44 |
| цитат spec:// всего (rule ref= + инлайн-проза) | 385 |
| из них не резолвится или якорь не найден | 3 |
| запрещённых слов/фраз (вхождений) | 11 |
| длинных фраз (сверх лимита) | 195 |
| длинных абзацев (>6 фраз) | 3 |
| проблемных заголовков | 12 |
| знаков (!, эмодзи, жирный, много тире) | 0 |
| нарушений термина-до-пояснения | 308 |
| битых межстраничных ссылок | 0 |
| проблем с id example/prompt | 0 |

Команда самопроверки XML (см. `WORKER-REPORT-PP-C1.md` за дословным выводом): `python -c "import xml.etree.ElementTree as ET,glob; [ET.parse(f) for f in glob.glob('vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**/*.xml', recursive=True)]; print('ok')"` -> `ok` (см. отчёт о работе).

## 1. Цитаты spec://…#ANCHOR (P.7)

Правила резолюции — ровно 3 паттерна из пакета (A: `org.vibevm.core/vibevm/<путь>/<DOC>` -> `vibevm/vibespecs/<путь>/<DOC>[-*].xml`; B: `org.vibevm.world/addressable-specs/flows/addressable-specs/<DOC>` -> vibedeps-копия; C: `org.vibevm.ai-native/core-ai-native/mechanisms/<DOC>` -> vibedeps-копия). Адреса вне этих 3 форм помечены `out-of-scope` (нет правила в пакете) или `malformed` (не парсится как `spec://GROUP/REST`); см. «Открытые вопросы».

| статус | количество |
|---|---|
| found | 373 |
| out-of-scope | 5 |
| malformed | 4 |
| anchor-not-found | 3 |

### 1.1 Не найдено (файл не резолвится или якорь отсутствует) — все случаи

| страница | адрес | файл/примечание | статус | 3 ближайших якоря |
|---|---|---|---|---|
| architecture/what-the-lifecycle-epic-delivered | spec://org.vibevm.core/vibevm/common/PROP-054#WHY-C-ABI | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | якорь не найден | why-c-abi, C-ABI-LAW, REJ-RUST-ABI |
| architecture/what-the-lifecycle-epic-delivered | spec://org.vibevm.core/vibevm/common/PROP-054#WASM | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | якорь не найден | WASM-DEFERRED, wasm, facts |
| lifecycle/extensions-and-providers | spec://org.vibevm.core/vibevm/common/PROP-054#WHY-C-ABI | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | якорь не найден | why-c-abi, C-ABI-LAW, REJ-RUST-ABI |

### 1.2 Вне 3 паттернов пакета (out-of-scope) и не разбираются как адрес (malformed)

| страница | источник | адрес | паттерн | примечание |
|---|---|---|---|---|
| agent/how-agents-read-this-manual | inline-prose | spec://… | malformed | эллипсис как обозначение адреса в пояснительном тексте, не реальная цитата |
| agent/how-agents-read-this-manual | inline-prose | spec://org.vibevm.core/vibevm-docs/ | out-of-scope | самоадресация страниц документации (`vibevm-docs/...`); ни один из 3 паттернов пакета не резолвит — дефект пакета |
| agent/how-agents-read-this-manual | inline-prose | spec://org.vibevm.core/vibevm-docs/model/boot-lane#p7 | out-of-scope | самоадресация страниц документации (`vibevm-docs/...`); ни один из 3 паттернов пакета не резолвит — дефект пакета |
| architecture/traceability | inline-prose | spec://…#anchor | malformed | эллипсис как обозначение адреса в пояснительном тексте, не реальная цитата |
| architecture/traceability | inline-prose | spec://… | malformed | эллипсис как обозначение адреса в пояснительном тексте, не реальная цитата |
| authoring/specs-agents-can-cite | inline-prose | spec://org.acme/notes-flow/flows/notes/PROTOCOL#RETRY-COUNT | out-of-scope | иллюстративный пример (org.acme — плейсхолдер-организация), не реальная цитата |
| authoring/write-a-flow | inline-prose | spec://org.acme/review-notes/flows/review-notes/PROTOCOL#anchor | out-of-scope | иллюстративный пример (org.acme — плейсхолдер-организация), не реальная цитата |
| authoring/write-documentation | inline-prose | spec://…#ANCHOR | malformed | эллипсис как обозначение адреса в пояснительном тексте, не реальная цитата |
| howto/read-documentation-locally | inline-prose | spec://org.vibevm.core/vibevm-docs/start/what-vibevm-is | out-of-scope | самоадресация страниц документации (`vibevm-docs/...`); ни один из 3 паттернов пакета не резолвит — дефект пакета |

### 1.3 Полная таблица всех цитат (385 строк)

| страница | источник | адрес | файл | статус |
|---|---|---|---|---|
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#EMPTY-SLOT-OK | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#MODE-INFERRED | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#NO-WRITE-BACK | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#ONE-OP-TWO-TRANSPORTS | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#RELAY-PARKS | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LLM-IS-AN-ENHANCEMENT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| agent/ask-your-agent | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/give-your-agent-the-skill | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-UNINSTALL | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/give-your-agent-the-skill | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#PROJECTION-DEF | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| agent/give-your-agent-the-skill | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#CONSENT-TRUST-ACT | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| agent/give-your-agent-the-skill | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#CONSENT-WRITE-SCOPE | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| agent/how-agents-read-this-manual | inline-prose | spec://org.vibevm.core/vibevm-docs/ |  | out-of-scope (см. §1.2) |
| agent/how-agents-read-this-manual | inline-prose | spec://org.vibevm.core/vibevm-docs/model/boot-lane#p7 |  | out-of-scope (см. §1.2) |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-DOC-NEVER-BOOTS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-WARMUP | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#OBS-AUDIENCE-AGENT | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LLMS-FILES | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#SEO-RAW-PROJECTIONS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| agent/how-agents-read-this-manual | inline-prose | spec://… |  | malformed (см. §1.2) |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#jtd | vibevm/vibespecs/common/PROP-000.xml | найдено |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#prod-arch | vibevm/vibespecs/common/PROP-000.xml | найдено |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#surfaces | vibevm/vibespecs/common/PROP-000.xml | найдено |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-044#M-FORMAT-REGISTRY | vibevm/vibespecs/common/PROP-044-change-native-formats.xml | найдено |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#SOLVER-TWO-IMPLS | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| architecture/how-vibe-is-built | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#TWO-PHASES-SPLIT | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#CONSEQUENCE-MAPPING-IS-CARRIED-NOT-GENERATED | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#DECISION-FOUR-UNIT-KINDS | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#FORCE-EDGES-TRAVEL-WITH-THE-ARTEFACTS | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#FORCE-INVARIANTS-ARE-MACHINE-CHECKED | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#INVALIDATION-SPEC-BUMP-MAKES-EDGES-SUSPECT | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#RULE-GENERATED-CODE-IS-EXCLUDED | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#RULE-IMPLEMENTS-IS-A-CLAIM-ABOUT-CODE-THAT-RUNS | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#RULE-VERBS-AND-MANDATORY-REASON | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| architecture/traceability | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EDGES-HOST-SIDE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| architecture/traceability | inline-prose | spec://… |  | malformed (см. §1.2) |
| architecture/traceability | inline-prose | spec://…#anchor |  | malformed (см. §1.2) |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LLM-IS-AN-ENHANCEMENT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-PACKAGE-DEPLOY | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#WASM | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | не найдено (якорь) |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#WHY-C-ABI | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | не найдено (якорь) |
| architecture/what-the-lifecycle-epic-delivered | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPED-TREE | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#OWN-WORKSPACE | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#ROOT-CODE | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#WHY-SOURCE-IDENTITY | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#EXACT-PIN-LAW | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#MCP-KIND-DEF | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#TABLE-ONLY-IN-KIND | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#VIBE-FREE-SERVING | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#BINARY-MUST | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#BINARY-TABLE | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#BUILD-CONSENT | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#HASHES-STABLE | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#OFFLINE-HONESTY | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/ship-tools-and-mcp-servers | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#TRUST-CURRENT-SLOT | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| authoring/specs-agents-can-cite | inline-prose | spec://org.acme/notes-flow/flows/notes/PROTOCOL#RETRY-COUNT |  | out-of-scope (см. §1.2) |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-045#NAMED-FACT-ELEMENTS | vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-045#XML-DIALECT-IS-THE-MD-SUBSET | vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-052#ADDRESSES-SURVIVE-THE-MOVE | vibevm/vibespecs/common/PROP-052-directory-layout.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-ANCHORS-IMMUTABLE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#DECISION-TWO-REGISTERS | vibevm/vibespecs/modules/vibe-facts/PROP-043-facts-markup.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#COMPILED-LANE-IS-NOT-A-CITATION-TARGET | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-VERSION-OPTIONAL | vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#FOR-POINT-CORRECTIONS-THE-URI-WINS | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#THE-SPEC-TREE-IS-THE-ONLY-CHANNEL | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#URI-SCHEME-IS-THE-FULL-GRAMMAR | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/authoring-rules#A-CHECKABLE-CLAIM-BELONGS-OUTSIDE-THE-FENCE | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/authoring-rules.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/authoring-rules#CONTRACT-STATEMENTS-USE-RFC-2119-VERBS | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/authoring-rules.xml | найдено |
| authoring/specs-agents-can-cite | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/authoring-rules#ONE-UNIT-CARRIES-ONE-DECISION | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/authoring-rules.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#CARD-TRANSLATION-INHERITS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-DOCUMENTS-MATCH | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-EXAMPLE-REF | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-LANGUAGE-FIELD | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-NO-REVISION | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-NORMATIVE-STAYS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-OFFICIAL-TRANSLATION | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/translate-documentation | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#I18N-DOC-PACKAGES | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| authoring/write-a-feat-or-stack | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#ROLE-AGGREGATOR | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| authoring/write-a-feat-or-stack | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#STACK-CONTRIBUTES-PRESET | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| authoring/write-a-feat-or-stack | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| authoring/write-a-feat-or-stack | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#SHAPE-REGISTRY-ARRAY | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| authoring/write-a-flow | inline-prose | spec://org.acme/review-notes/flows/review-notes/PROTOCOL#anchor |  | out-of-scope (см. §1.2) |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#PKG-PROJECT-LAW | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#SHIPPABLE-TREE-DEF | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#SPEC-SUBTREE | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-048#THE-LAYER-LAW | vibevm/vibespecs/common/PROP-048-tokenomics.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-049#SNIPPET-GENRE-RULE | vibevm/vibespecs/common/PROP-049-snippet-genre.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-052#ADDRESSES-SURVIVE-THE-MOVE | vibevm/vibespecs/common/PROP-052-directory-layout.xml | найдено |
| authoring/write-a-flow | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SUGGESTED-DEFAULT | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| authoring/write-a-lang-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET | vibevm/vibespecs/common/PROP-000.xml | найдено |
| authoring/write-a-lang-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#FAM-CORE | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| authoring/write-a-lang-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#FAMILY-DEF | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| authoring/write-a-lang-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#SURFACE-NAMING-LAW | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| authoring/write-a-lang-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#UNISON-LAW | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND | vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-README-LIMIT | vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PLACEHOLDERS-GENERATED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-DOC-CITES-NEVER-COPIES | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-DOCUMENT | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-NOT-EXECUTE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-DEFAULT-CONVENTION | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-OFFICIAL-IS-CONVERGENCE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-VERSION-SELECTION | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PAGE-SKELETON | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| authoring/write-documentation | inline-prose | spec://…#ANCHOR |  | malformed (см. §1.2) |
| diagnostics/errors | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#root | vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml | найдено |
| diagnostics/errors | inline-prose | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| diagnostics/errors | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#SURF-EXIT-CODE | vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#CF-EXACT | vibevm/vibespecs/common/PROP-000.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LLM-IS-AN-ENHANCEMENT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#RELATED-CLEAN | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#EFF-MIRROR-SUBSTITUTION | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#OVERRIDE-SHORT-CIRCUIT | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#KIND-METADATA | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-NO-DEGRADE | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-UNIFIED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#PLAN-UNIT | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| faq/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#VIBEDEPS-COMMITTED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#FORCE-INVARIANTS-ARE-MACHINE-CHECKED | vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms/PROP-014-specmap-bidirectional-traceability.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#CF-RANGE | vibevm/vibespecs/common/PROP-000.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET | vibevm/vibespecs/common/PROP-000.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#PROJECTION-DEF | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#RELAY-PARKS | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-024#PKG-PROJECT-LAW | vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#FAMILY-DEF | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#ROLE-DOCS | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-045#NAMED-FACT-ELEMENTS | vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#HANDLER-KINDS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-DEPLOY-RUNTIME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-DEPLOY-RUNTIME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-NATIVE-DEPLOY-PROVIDER | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPE-TERM-BOUNDARY | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-ANCHORS-IMMUTABLE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-OFFICIAL-TRANSLATION | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#READER-NUMBERED-BLOCKS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-NO-OFFICIAL-FLAG | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-OFFICIAL-IS-CONVERGENCE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-SOURCE-LANGUAGE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#REPOS-AUTHORITATIVE | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#MCP-KIND-DEF | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#GS-IDENTITY | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIRROR-WALK-SEMANTICS | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#OVERRIDE-SHORT-CIRCUIT | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#SHAPE-OWN-REPO | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#HASH-UNCHANGED | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#CACHE-MACHINE-GLOBAL | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#AMBIENT-DEFAULT | vibevm/vibespecs/modules/vibe-registry/PROP-030-embedded-registry.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#DESCRIBES-ON-SUBSKILLS | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#FEATURES-TABLE | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#ONE-MANIFEST | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#PACKAGE-XOR-PROJECT | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#WORKSPACE-TABLE | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INCLUSION-TYPES | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SCHEMA-BOOT-SNIPPET | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SESSION-START-ORDER | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#LOCKFILE-RESPECTING | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#ONE-BLOCK-LAW | vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#WHAT-HOOKS-ARE | vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml | найдено |
| glossary/index | rule-ref | spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#THE-SPEC-TREE-IS-THE-ONLY-CHANNEL | vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#CF-RANGE | vibevm/vibespecs/common/PROP-000.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#ROW-QUALIFIED-KIND-BEHAVIOUR | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#HOLD-THE-LOCK | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#LOCKFILE-RESPECTING | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#PHASE-RESOLUTION | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| howto/install-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#INSTALLATION-CONSENT-SUCCESSOR | vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml | найдено |
| howto/publish-a-package | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy | vibevm/vibespecs/common/PROP-000.xml | найдено |
| howto/publish-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#EFF-FORCE-PUSH-CAUGHT | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/publish-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#SHAPE-OWN-REPO | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/publish-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#PUBLISH-REGEN | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| howto/publish-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#TOKEN-NEVER-IN-ENV | vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml | найдено |
| howto/read-documentation-locally | inline-prose | spec://org.vibevm.core/vibevm-docs/start/what-vibevm-is |  | out-of-scope (см. §1.2) |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-LOCAL-IS-OFFLINE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-CSP | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-OFFLINE-SHELL | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/read-documentation-locally | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-WARMUP-CLOSURE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| howto/remove-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#CACHE-ACCRETIVE | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| howto/remove-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TREE-AUTHORED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| howto/remove-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#EFFECTS-EPHEMERAL | vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml | найдено |
| howto/remove-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-NEEDS-A-PROJECT | vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml | найдено |
| howto/remove-a-package | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-REMOVES-DERIVED | vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#EXPLICIT-MEMBERSHIP | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#MEMBER-IS-NODE | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#NESTING-PRINCIPLE | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#PACKAGE-XOR-PROJECT | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SCOPE-FLAG | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| howto/set-up-a-workspace | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TREE-VIBEDEPS | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| howto/update-packages | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#CF-RANGE | vibevm/vibespecs/common/PROP-000.xml | найдено |
| howto/update-packages | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#ROW-GS-BRANCH-MEANING | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/update-packages | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#REINSTALL-NO-FORCE | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| howto/update-packages | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#UPDATE-MOVES-LOCK | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#INDEX-OPTIONAL | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#AUTH-REGIMES | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIRROR-WALK-SEMANTICS | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#REGISTRY-WALK-ORDER | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#TOKEN-NEVER-ON-DISK | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/use-a-private-registry | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#USER-LEVEL-REGISTRIES | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| howto/work-offline | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIRROR-LAYER | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| howto/work-offline | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#AS-OF-LAST-REFRESH | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| howto/work-offline | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-HARD-ERROR | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| howto/work-offline | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-LAYERING | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| howto/work-offline | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-LOCAL-ONLY | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| lifecycle/build-package-deploy | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-CREATE | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/build-package-deploy | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-ARTIFACT-RUNTIME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/build-package-deploy | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-DEPLOY-RUNTIME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/build-package-deploy | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-NATIVE-DEPLOY-PROVIDER | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/build-package-deploy | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#R8-PLATFORM-APPLICABILITY | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-PROVIDER-SEAM | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-FIELDS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#HANDLER-KINDS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LLM-BUDGET | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LLM-ENHANCEMENT-MODES | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#OBS-LAW | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#POINT-GRAMMAR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/extensions-and-providers | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#WHY-C-ABI | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | не найдено (якорь) |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#EXISTING-VERBS-STAY | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#FRESHNESS-IS-PER-CONTRIBUTION | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#INSTALL-IS-CONSENT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLE-IS-FRAMEWORK | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/phases | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#DEFAULT-CONTRACT | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#PRODUCT-PRESERVATION | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#RELATED-CLEAN | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPE-MODE-EXCLUSIVITY | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPE-RECOVER-COMMAND | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPE-TERM-BOUNDARY | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| lifecycle/scrape | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-056#SCRAPED-TREE | vibevm/vibespecs/common/PROP-056-scraped-project-export.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-048#STATIC-ROLE | vibevm/vibespecs/common/PROP-048-tokenomics.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-048#THE-LAYER-LAW | vibevm/vibespecs/common/PROP-048-tokenomics.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#INV-DOC-NEVER-BOOTS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACT-INDEX-MD | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#CATEGORY-ORDER | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#COMPILED-LANE-IS-NOT-A-CITATION-TARGET | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INCLUSION-TYPES | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SESSION-START-ORDER | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/boot-lane | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#WHEN-FORCES-DYNAMIC | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#EFF-FORCE-PUSH-CAUGHT | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#CACHE-MACHINE-GLOBAL | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#EXPLICIT-RECLAIM | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#HASH-INTEGRITY-GATE | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#LAYOUT-EXTRACTED-DIRECTORIES | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-HARD-ERROR | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/lock-and-store | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-KEEPS-THE-LOCK | vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#KIND-SET | vibevm/vibespecs/common/PROP-000.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#ROLE-DOCS | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#UNISON-LAW | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-052#PACKAGES-CARRY-THE-LAYOUT-TOO | vibevm/vibespecs/common/PROP-052-directory-layout.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#GROUP-CHANGE-NEW-PACKAGE | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#KIND-METADATA | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| model/packages-and-kinds | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#ROW-SHORT-BEHAVIOUR | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-016#ORTHOGONALITY-LAW | vibevm/vibespecs/common/PROP-016-source-mirrors.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#INDEX-OPTIONAL | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#INDEX-URL-TODAY-IS-AN-ENVIRONMENT-VARIABLE | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#REPOS-AUTHORITATIVE | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIRROR-INTEGRITY-MANDATORY | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#OVERRIDE-SHORT-CIRCUIT | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#REGISTRY-WALK-ORDER | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#ROW-GS-BRANCH-MEANING | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#SHAPE-OWN-REPO | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#TOKEN-NEVER-ON-DISK | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#PROJECT-OVERRIDES | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| model/registries | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#AMBIENT-DEFAULT | vibevm/vibespecs/modules/vibe-registry/PROP-030-embedded-registry.xml | найдено |
| model/two-trees | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#REINSTALL | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/two-trees | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TWO-TREES | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/two-trees | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#VIBEDEPS-COMMITTED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| model/two-trees | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-KEEPS-AUTHORED | vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#CF-RANGE | vibevm/vibespecs/common/PROP-000.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#ACTIVATION-LAW | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#SEL-LATEST | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#SEL-STABLE | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-028#AGGREGATOR-PINS-DELIBERATE | vibevm/vibespecs/common/PROP-028-package-families.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#OBS-VERSION-CONTRACT | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-CONSEQUENCE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| model/versions | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#TRAIT-PIN-PREFERENCES | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#EFF-LOCKFILE-STABLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIR-CANONICAL-IN-LOCKFILE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#SOURCE-KIND-VALUES | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#LF-META-LANGUAGE | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#NESTING-PRINCIPLE | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#LOCKFILE-RESPECTING | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#NO-NEW-FIELD | vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml | найдено |
| reference/lock-file | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-053#CLEAN-KEEPS-THE-LOCK | vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml | найдено |
| reference/machine-formats | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#jtd | vibevm/vibespecs/common/PROP-000.xml | найдено |
| reference/machine-formats | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-044#FMT-MANIFEST | vibevm/vibespecs/common/PROP-044-change-native-formats.xml | найдено |
| reference/machine-formats | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-044#M-FORMAT-REGISTRY | vibevm/vibespecs/common/PROP-044-change-native-formats.xml | найдено |
| reference/machine-formats | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-044#POLICY-IS-COMPUTED | vibevm/vibespecs/common/PROP-044-change-native-formats.xml | найдено |
| reference/machine-formats | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-044#RISK-RAW-PARSERS | vibevm/vibespecs/common/PROP-044-change-native-formats.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-018#SKILL-TABLE-SHAPE | vibevm/vibespecs/common/PROP-018-agentic-standalone-modes.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-GRAMMAR | vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#LOC-PACKAGE-PER-LANGUAGE | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED | vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#TABLE-ONLY-IN-KIND | vibevm/vibespecs/modules/vibe-mcp/PROP-027-mcp-packages.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#MIRROR-LAYER | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#REGISTRY-ARRAY | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#GROUP-GRAMMAR | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#GROUP-MANDATORY | vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#FEATURES-TABLE | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#I18N-DECISION | vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#ONE-MANIFEST | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#PACKAGE-XOR-PROJECT | vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SCHEMA-BOOT-SNIPPET | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SCHEMA-LINK-FIELD | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#HOOKS-TABLE | vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml | найдено |
| reference/manifest | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#BINARY-TABLE | vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy | vibevm/vibespecs/common/PROP-000.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#LAYER-ENV-ADVISORY | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#INDEX-URL-TODAY-IS-AN-ENVIRONMENT-VARIABLE | vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#TOKEN-ENV-DEFAULTING | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#OFFLINE-LAYERING | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#PROJECT-OVERRIDES | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#REGISTRIES-KEEP-THEIR-OWN-FILE | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#L2-REPO-SHARED | vibevm/vibespecs/modules/vibe-settings/PROP-040-settings.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#MERGE-ARRAYS | vibevm/vibespecs/modules/vibe-settings/PROP-040-settings.xml | найдено |
| reference/settings-and-environment | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-settings/PROP-040#MERGE-SCALARS | vibevm/vibespecs/modules/vibe-settings/PROP-040-settings.xml | найдено |
| start/first-project | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-PAIR | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/first-project | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#CLASS-MALFORMED | vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml | найдено |
| start/first-project | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#ONE-BLOCK-LAW | vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml | найдено |
| start/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#IDENTITY-TUPLE | vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml | найдено |
| start/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| start/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INCLUDE-RULE | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#SESSION-START-ORDER | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/index | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#TWO-TREES | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#ACE-ACCEPTED | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#CMD-IMPORT | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#CMD-INSTALL | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#ROOT-DEFAULT | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#RULE-NEVER-CLOBBER | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-019#TOOLS-LIST | vibevm/vibespecs/common/PROP-019-version-manager.xml | найдено |
| start/install-vibe | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG | vibevm/vibespecs/modules/vibe-registry/PROP-010-local-package-cache.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-052#NO-LEGACY-LAYOUT | vibevm/vibespecs/common/PROP-052-directory-layout.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-052#THE-LAYOUT | vibevm/vibespecs/common/PROP-052-directory-layout.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-GENERATED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#REINSTALL-NO-FORCE | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#VIBEDEPS-COMMITTED | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/what-a-project-contains | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#OUTSIDE-PRESERVED | vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml | найдено |
| start/what-vibevm-is | rule-ref | spec://org.vibevm.core/vibevm/common/PROP-048#TOKENOMICS-IS-A-DESIGN-PRESSURE | vibevm/vibespecs/common/PROP-048-tokenomics.xml | найдено |
| start/what-vibevm-is | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INCLUDE-RULE | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |
| start/what-vibevm-is | rule-ref | spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#PURE-FILE-READING | vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml | найдено |

## 2. Запрещённые слова (P.6)

Список: `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/style/banned.en.txt`. Всего вхождений: 11.

| страница | слово/фраза | фильтр (just/simply/…) | где | контекст |
|---|---|---|---|---|
| authoring/write-a-feat-or-stack | capabilities |  | p | ts manifest declares the abilities it needs from a stack as capabilities, `namespace:name@constraint`, and nothing else… |
| authoring/write-a-feat-or-stack | capabilities |  | p | In the manifest: `[requires] capabilities = ["ui:page-host@^1"]` and, for a feat that needs a stack a |
| authoring/write-a-feat-or-stack | capabilities |  | p | lchain. Its manifest declares what it provides, `[provides] capabilities = ["ui:page-host@1.0"]`, and its specification… |
| authoring/write-a-feat-or-stack | capabilities |  | td | `vibevm/vibespecs/stacks/<name>/capabilities/<capability>.md` |
| howto/install-a-package | acts as |  | p | prefix, as in `flow:org.vibevm.world/wal`, is optional and acts as a check: if the package turns out to be of another ki… |
| model/boot-lane | specific | да | p | the condition holds for the current session, for example a specific operating system. |
| model/packages-and-kinds | capabilities |  | p | Some capabilities arrive as several packages that share a name stem: the lang |
| model/registries | just | да | p | rives. A registry without an index works exactly as before, just slower; a missing index is not an error. |
| reference/manifest | capabilities |  | td | `[requires] capabilities` |
| reference/manifest | capabilities |  | td | `[provides] capabilities` |
| start/first-project | just | да | p | ask it what rules it follows: it will name the package you just installed, because it read the lane before answering. |

Отдельное наблюдение (не решение — фиксирую факт для центральной сессии): все 4 вхождения `capabilities` и вхождение `capabilities` в `model/packages-and-kinds` — это точный технический термин продукта (глоссарий: *capability*, поле манифеста `[requires] capabilities` / `[provides] capabilities`), а не маркетинговая рыхлость, на которую рассчитан общий запрет. REVIEW-статус см. ниже.

## 3. Длина фраз и абзацев

Правило: в абзацах внутри секций «By hand…» и в `prompt`/`outcome` — лимит 20 слов/фразу; в прочих абзацах — лимит 25 слов/фразу, только если абзац содержит хотя бы один `код`-спан; абзацы длиннее 6 фраз — отдельно, независимо от лимита слов. Слово = токен по пробелу после маскировки кодовых спанов (`код` -> 1 токен) и markdown-ссылок (`[текст](путь)` -> `текст`).

### 3.1 Фразы сверх лимита слов (всего 195)

| страница | где | секция | № фразы | слов | лимит | начало фразы |
|---|---|---|---|---|---|---|
| agent/ask-your-agent | p | How vibe knows an agent is calling | 1 | 41 | 25 | vibe behaves the same whether a person or an agent types the… |
| agent/ask-your-agent | p | Two ways for an agent to reach vibe | 2 | 31 | 25 | Or it can talk to `vibe mcp serve`, a persistent server registered… |
| agent/ask-your-agent | p | When vibe hands the job back | 2 | 30 | 25 | When an operation needs reasoning, such as explaining a project in prose,… |
| agent/give-your-agent-the-skill | outcome |  | 1 | 29 | 20 | `vibe mcp status` shows an up-to-date server entry and skill for each… |
| agent/give-your-agent-the-skill | p | What happens | 2 | 47 | 25 | vibe detects the agents present on the machine and, for each, writes… |
| agent/give-your-agent-the-skill | p | Skills that packages bring | 1 | 34 | 25 | Packages can declare skills of their own, for any kind of package:… |
| agent/give-your-agent-the-skill | p | Servers that packages bring | 2 | 26 | 25 | `vibe mcp install` registers such servers in the agents' configurations beside vibe's… |
| agent/give-your-agent-the-skill | prompt |  | 1 | 37 | 20 | Wire vibe into every coding agent installed on this machine, for the… |
| agent/how-agents-read-this-manual | p | Citing a place | 1 | 30 | 25 | Every block on a page carries a number, `p12` and so on,… |
| agent/how-agents-read-this-manual | p | Citing a place | 3 | 32 | 25 | Headings keep their named anchors as well, and a named anchor never… |
| architecture/how-vibe-is-built | p | The seams | 3 | 30 | 25 | `DepProvider` is the solver's view of the world and `DepSolver` turns roots… |
| architecture/how-vibe-is-built | p | Where to read next | 1 | 36 | 25 | The specifications are the authority: `PROP-000` for the foundational decisions, `PROP-009` for… |
| architecture/traceability | p | Marks in the code | 1 | 35 | 25 | A Rust item that implements a rule carries an attribute naming the… |
| architecture/traceability | p | Marks in the code | 2 | 27 | 25 | A type declaration does not carry it: an edge from a declaration… |
| architecture/traceability | p | The map | 1 | 37 | 25 | `cargo xtask specmap` walks the crates for marks and the specification tree… |
| architecture/traceability | p | The map | 3 | 32 | 25 | The policy of what is scanned, which crates are gated and which… |
| architecture/traceability | p | Documentation in the map | 2 | 55 | 25 | Every `rule` block on a page is a `documents` edge from the… |
| architecture/what-the-lifecycle-epic-delivered | p | Compatibility boundaries and migrations | 1 | 26 | 25 | The dependency slot record and the lifecycle state are strict, versioned machine… |
| architecture/what-the-lifecycle-epic-delivered | p | Deliberately left for later | 1 | 35 | 25 | Deploy targets beyond the first genres, a WebAssembly extension tier, per-language and… |
| architecture/what-the-lifecycle-epic-delivered | p | Where the evidence lives | 1 | 31 | 25 | The stage-by-stage ledger with commit hashes is `campaigns/packages-2026-09/LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md` in the repository; the… |
| architecture/what-the-lifecycle-epic-delivered | p | Where the evidence lives | 2 | 26 | 25 | The ledger is a record, not a plan: it names what landed… |
| authoring/ship-tools-and-mcp-servers | outcome |  | 1 | 24 | 20 | the manifest carries a `[[binary]]` table; `vibe bin list` shows the tool;… |
| authoring/ship-tools-and-mcp-servers | p | What happens | 1 | 55 | 25 | The agent adds a `[[binary]]` table naming the tool and the crate… |
| authoring/ship-tools-and-mcp-servers | p | What happens | 2 | 27 | 25 | Consumers get the same: installing the package materialises its source, building on… |
| authoring/ship-tools-and-mcp-servers | p | Binaries | 2 | 29 | 25 | `vibe bin list` shows what the installed packages declare, `vibe bin build`… |
| authoring/ship-tools-and-mcp-servers | p | MCP servers | 1 | 27 | 25 | A package of kind `mcp` delivers a server an agent talks to:… |
| authoring/ship-tools-and-mcp-servers | p | MCP servers | 2 | 31 | 25 | The engines behind the agent's tools and the gates the consumer runs… |
| authoring/ship-tools-and-mcp-servers | p | Edge cases and rules | 1 | 27 | 25 | A tool's artifact belongs to the exact version installed; after an update… |
| authoring/ship-tools-and-mcp-servers | prompt |  | 1 | 37 | 20 | In the package packages/notes-tools of the current VibeVM project, declare the Rust… |
| authoring/specs-agents-can-cite | p | The address | 3 | 27 | 25 | Anchors are section ids and fact ids in one address space, so… |
| authoring/specs-agents-can-cite | p | Two serialisations, one model | 2 | 44 | 25 | In XML a section is an element named after its anchor, `<retry-policy`… |
| authoring/specs-agents-can-cite | p | Two serialisations, one model | 3 | 27 | 25 | A tool that knows nothing of your vocabulary still finds every rule… |
| authoring/specs-agents-can-cite | p | Two serialisations, one model | 2 | 26 | 25 | Two id registers carry the signal at no cost: an upper-case id… |
| authoring/translate-documentation | outcome |  | 1 | 23 | 20 | the translation has the same files and anchors as the source, `[i18n]`… |
| authoring/translate-documentation | p | What happens | 1 | 46 | 25 | The agent copies the source's page tree, translates the prose of each… |
| authoring/translate-documentation | p | What happens | 3 | 27 | 25 | The site, seeing a `translates` edge from a package in the source's… |
| authoring/translate-documentation | p | Edge cases and rules | 1 | 31 | 25 | Sidecar files inside the source package, `README.ru.md` beside `README.md`, are how specifications… |
| authoring/translate-documentation | prompt |  | 1 | 50 | 20 | Create the package org.acme/notes-flow-docs-ru in packages/notes-flow-docs-ru of the current VibeVM project as… |
| authoring/write-a-feat-or-stack | outcome |  | 1 | 24 | 20 | the feat's manifest requires `ui:page-host`, the stack's manifest provides it, each has… |
| authoring/write-a-feat-or-stack | p | What happens | 1 | 29 | 25 | The agent creates both packages with `vibe init package`, writes the feat's… |
| authoring/write-a-feat-or-stack | p | What happens | 2 | 33 | 25 | When a project later installs the feat, the resolver looks for a… |
| authoring/write-a-feat-or-stack | p | A feat | 1 | 29 | 25 | A feat describes what a feature does for its user, in terms… |
| authoring/write-a-feat-or-stack | p | A stack | 1 | 36 | 25 | A stack is a technology context: it says how the abstract abilities… |
| authoring/write-a-feat-or-stack | p | A stack | 2 | 26 | 25 | Its manifest declares what it provides, `[provides] capabilities = ["ui:page-host@1.0"]`, and its… |
| authoring/write-a-feat-or-stack | p | A stack | 1 | 26 | 25 | A stack may also bind lifecycle contributions in its manifest, so that… |
| authoring/write-a-feat-or-stack | p | Capabilities | 2 | 26 | 25 | A feat requires; a stack provides; the resolver matches them at install… |
| authoring/write-a-feat-or-stack | prompt |  | 1 | 45 | 20 | Create two packages under packages/ in the current VibeVM project: a feat… |
| authoring/write-a-flow | outcome |  | 1 | 32 | 20 | `packages/review-notes/vibe.toml` declares a `flow` package with a boot snippet; the snippet is… |
| authoring/write-a-flow | p | What happens | 2 | 48 | 25 | It then writes three things: the boot snippet, a short instruction the… |
| authoring/write-a-flow | p | The snippet is the expensive part | 1 | 30 | 25 | The consumer decides how your snippet is linked, compiled into the priority… |
| authoring/write-a-flow | prompt |  | 2 | 28 | 20 | It should teach an agent to leave a short REVIEW.md note at… |
| authoring/write-a-lang-package | outcome |  | 1 | 21 | 20 | `packages/sql-style/vibe.toml` declares `kind = "lang"` with a boot snippet; the guide under… |
| authoring/write-a-lang-package | p | What happens | 2 | 46 | 25 | The difference from a flow is the genre, not the mechanics: a… |
| authoring/write-a-lang-package | p | What a lang package is, and is not | 3 | 27 | 25 | A third party may publish its own language discipline in its own… |
| authoring/write-a-lang-package | p | When a language brings tools | 1 | 42 | 25 | A language guide that also ships tools, a checker, a formatter, a… |
| authoring/write-a-lang-package | p | When a language brings tools | 1 | 27 | 25 | The family stem leads every named surface: crates, binaries, skills and the… |
| authoring/write-a-lang-package | p | By hand | 2 | 24 | 20 | Write `vibevm/vibespecs/boot/sql-style.xml` with the rules an agent must follow every time, and… |
| authoring/write-a-lang-package | p | Edge cases and rules | 1 | 31 | 25 | What a language brought that can be run is answered by `vibe`… |
| authoring/write-a-lang-package | p | Edge cases and rules | 1 | 32 | 25 | Before the `lang` kind existed, language guides were stacks; a `stack` package… |
| authoring/write-a-lang-package | prompt |  | 1 | 48 | 20 | Create a lang package org.acme/sql-style in packages/sql-style of the current VibeVM project:… |
| authoring/write-documentation | outcome |  | 1 | 29 | 20 | `packages/notes-flow-docs/vibe.toml` declares `kind = "doc"`, `title`, `abstract` and `[[documents]]`; the page under… |
| authoring/write-documentation | p | What happens | 1 | 28 | 25 | The agent creates the package with the `doc` kind, fills the card,… |
| authoring/write-documentation | p | What happens | 3 | 33 | 25 | The manual is published like any package; the site finds it through… |
| authoring/write-documentation | p | The manifest | 1 | 30 | 25 | A `doc` package must name at least one subject and carry a… |
| authoring/write-documentation | p | The vocabulary of a page | 1 | 35 | 25 | A page that quotes a normative value, a flag, a path, a… |
| authoring/write-documentation | p | The shape of a page | 1 | 28 | 25 | The full writing rules, including the banned words and the limits on… |
| authoring/write-documentation | p | Checking and publishing | 1 | 32 | 25 | `vibe doc check --examples --citations --derived --media --style` runs every check; the… |
| authoring/write-documentation | p | Edge cases and rules | 1 | 34 | 25 | Any group may document any package; the site shows such a manual… |
| authoring/write-documentation | p | Edge cases and rules | 1 | 26 | 25 | A manual describes a version range of its subject through `[[documents]] version`,… |
| authoring/write-documentation | prompt |  | 1 | 50 | 20 | Create a documentation package org.acme/notes-flow-docs in packages/notes-flow-docs of the current VibeVM project,… |
| diagnostics/errors | p | How to read an error | 3 | 28 | 25 | Exit codes are stable for scripts: 1 for a general failure, 3… |
| faq/index | p | Do I commit the dependency tree? | 2 | 33 | 25 | `vibevm/vibedeps/` is committed on purpose, so an agent that clones the repository… |
| faq/index | p | A dependency deep in my tree clashes on a version. How do I fix it? | 2 | 49 | 25 | Then climb the ladder from lightest to heaviest: widen a constraint you… |
| faq/index | p | A registry I used has disappeared. Is my project stuck? | 1 | 33 | 25 | Not while the machine store holds the packages: a stored version is… |
| faq/index | p | Why can I not install the manual into my project? | 2 | 27 | 25 | So a documentation package is warmed into the machine store with `vibe`… |
| glossary/index | p | block number | 1 | 33 | 25 | The ordinal `pNN` every block of a documentation page receives at build… |
| glossary/index | p | boot lane | 1 | 29 | 25 | The ordered reading list an agent follows at the start of a… |
| glossary/index | p | companion | 1 | 30 | 25 | A package tied to another by name for the default case of… |
| glossary/index | p | contribution | 1 | 27 | 25 | A binding of a handler to an extension point, declared as an… |
| glossary/index | p | extension point | 1 | 26 | 25 | A named place in the lifecycle a contribution binds to: a phase,… |
| glossary/index | p | hook | 1 | 26 | 25 | A package's `pre-install` or `post-install` script, run in the package's slot; installing… |
| glossary/index | p | link type | 1 | 30 | 25 | How a dependency's boot snippet enters a consumer's lane: `static`, compiled into… |
| glossary/index | p | managed block | 1 | 32 | 25 | The region between the lines `<vibevm>` and `</vibevm>` at the end of… |
| glossary/index | p | traceability map | 1 | 26 | 25 | `specmap.json`: the generated graph of specification units, tagged code items and the… |
| glossary/index | p | translation | 1 | 26 | 25 | A separate documentation package in another language that names its source in… |
| glossary/index | p | workspace | 1 | 26 | 25 | A repository developing several packages together, declared by a `[workspace]` table listing… |
| howto/install-a-package | outcome |  | 1 | 31 | 20 | `vibe.toml` lists the package under its requirements, `vibe.lock` pins one version with… |
| howto/install-a-package | p | What happens | 2 | 29 | 25 | vibe walks the project's registries in order and asks the first one… |
| howto/install-a-package | p | What happens | 3 | 43 | 25 | It resolves the package's own dependencies with the rest of the project's… |
| howto/install-a-package | p | Asking for a version | 1 | 26 | 25 | A kind prefix, as in `flow:org.vibevm.world/wal`, is optional and acts as a… |
| howto/install-a-package | p | After cloning a project | 2 | 35 | 25 | When neither the manifest nor the lock changed since the last install,… |
| howto/install-a-package | p | Edge cases and rules | 1 | 34 | 25 | A public registry that answers with an authentication error for a missing… |
| howto/install-a-package | prompt |  | 1 | 33 | 20 | Install the package org.vibevm.world/wal into the VibeVM project in the current folder,… |
| howto/publish-a-package | outcome |  | 1 | 28 | 20 | a repository named after the package's coordinate exists in the registry organisation… |
| howto/publish-a-package | p | What happens | 2 | 34 | 25 | After your yes it runs the real command: vibe creates the repository… |
| howto/publish-a-package | p | By hand | 2 | 23 | 20 | Put a publish token where vibe reads it: the environment variable `VIBEVM_PUBLISH_TOKEN`,… |
| howto/publish-a-package | p | Several packages at once | 1 | 42 | 25 | A repository that develops several packages publishes them with `vibe workspace publish`:… |
| howto/publish-a-package | p | Edge cases and rules | 1 | 26 | 25 | `--repo-url` pushes straight to an existing git repository with your local git… |
| howto/publish-a-package | prompt |  | 1 | 39 | 20 | Publish the package in the folder packages/notes to the first registry of… |
| howto/read-documentation-locally | p | What happens | 2 | 32 | 25 | A documentation package is never installed into a project; it is warmed… |
| howto/read-documentation-locally | p | What happens | 3 | 30 | 25 | Then the agent runs `vibe doc serve`: vibe starts a small web… |
| howto/read-documentation-locally | p | By hand | 2 | 25 | 20 | Pick a language with the selector on any page; the reader shows… |
| howto/read-documentation-locally | p | The reader's shell | 2 | 36 | 25 | A `vibe` you built from source carries a plain fallback; on the… |
| howto/read-documentation-locally | prompt |  | 1 | 26 | 20 | Fetch the VibeVM manual, the package org.vibevm.core/vibevm-docs, into the machine store, open… |
| howto/remove-a-package | outcome |  | 1 | 30 | 20 | the requirement is gone from `vibe.toml`, the entry is gone from `vibe.lock`,… |
| howto/remove-a-package | p | What happens | 2 | 29 | 25 | vibe shows what will leave: the requirement line in the manifest, the… |
| howto/remove-a-package | p | Removing derived state without removing packages | 1 | 26 | 25 | Sometimes you want a clean slate rather than a smaller graph: before… |
| howto/remove-a-package | prompt |  | 1 | 23 | 20 | Remove the package org.vibevm.world/wal from the VibeVM project in the current folder… |
| howto/set-up-a-workspace | outcome |  | 1 | 30 | 20 | the root `vibe.toml` carries a `[workspace]` table listing both members, each member… |
| howto/set-up-a-workspace | p | What happens | 1 | 28 | 25 | The agent adds a `[workspace]` table to the root manifest naming the… |
| howto/set-up-a-workspace | p | What happens | 3 | 30 | 25 | Then it runs `vibe install` at the root: vibe discovers the workspace,… |
| howto/set-up-a-workspace | p | One manifest, three roles | 1 | 44 | 25 | Every node has a file named `vibe.toml`, and what the file contains… |
| howto/set-up-a-workspace | p | Members referring to each other | 2 | 28 | 25 | The lock file records such an entry with a source kind of… |
| howto/set-up-a-workspace | prompt |  | 1 | 24 | 20 | Turn the VibeVM project in the current folder into a workspace with… |
| howto/update-packages | p | What happens | 1 | 27 | 25 | The agent first runs `vibe outdated`, which compares every pin in the… |
| howto/update-packages | p | What happens | 2 | 46 | 25 | Then it runs `vibe update --all`: vibe re-resolves the graph, preferring the… |
| howto/update-packages | p | What happens | 3 | 28 | 25 | The agent finishes by reading the lock file's diff back to you:… |
| howto/update-packages | p | The constraint stays where you put it | 3 | 27 | 25 | To move the constraint too, install the new version explicitly with a… |
| howto/update-packages | p | Recovering after a breaking update | 1 | 41 | 25 | If an update leaves the project in a state that no longer… |
| howto/update-packages | prompt |  | 1 | 28 | 20 | In the VibeVM project in the current folder, show me which packages… |
| howto/use-a-private-registry | p | By hand | 2 | 35 | 20 | Choose the authentication regime with `--auth`: `none` for public read, `ssh` for… |
| howto/use-a-private-registry | p | Edge cases and rules | 1 | 30 | 25 | In a script that must notice a private registry being down, pass… |
| howto/use-a-private-registry | prompt |  | 1 | 37 | 20 | Add the private registry named acme at git@github.com:acme-specs as the first registry… |
| howto/work-offline | p | What happens | 1 | 27 | 25 | The agent runs `vibe cache add` for the extra package, which resolves… |
| howto/work-offline | p | What happens | 3 | 28 | 25 | Finally it runs `vibe install --offline`: resolution and fetch are satisfied from… |
| howto/work-offline | p | By hand | 2 | 22 | 20 | Warm the store with a package and its dependencies; inside a project… |
| howto/work-offline | p | A whole team without a network | 1 | 34 | 25 | For a machine that never sees the registry, `vibe registry vendor` writes… |
| howto/work-offline | p | Edge cases and rules | 1 | 30 | 25 | Offline resolution sees the store as of its last refresh: a version… |
| howto/work-offline | prompt |  | 1 | 37 | 20 | Before I go offline, fetch into the machine store everything the VibeVM… |
| lifecycle/build-package-deploy | outcome |  | 1 | 36 | 20 | the plan names the targets in order; after the deploy `vibe deployments`… |
| lifecycle/build-package-deploy | p | What happens | 1 | 27 | 25 | `vibe deploy --plan` runs the whole default lifecycle in planning mode and… |
| lifecycle/build-package-deploy | p | What happens | 2 | 32 | 25 | The real `vibe deploy` then validates, installs, generates, builds, tests, creates, verifies… |
| lifecycle/build-package-deploy | p | Artifacts and targets | 1 | 44 | 25 | The manifest declares what `build` produces and what `package` assembles as artifact… |
| lifecycle/build-package-deploy | p | Artifacts and targets | 2 | 29 | 25 | An artifact or a target may declare the operating systems it applies… |
| lifecycle/build-package-deploy | p | Edge cases and rules | 1 | 32 | 25 | The deploy phase applies packaged artifacts and nothing else: a step that… |
| lifecycle/build-package-deploy | prompt |  | 1 | 35 | 20 | For the VibeVM project in the current folder, show me the deploy… |
| lifecycle/extensions-and-providers | p | Points and contributions | 2 | 45 | 25 | The `phase:` family is the nine phases; the `slot:` family names places… |
| lifecycle/extensions-and-providers | p | Points and contributions | 3 | 26 | 25 | A *contribution* binds a handler to a point and is declared in… |
| lifecycle/extensions-and-providers | p | Providers | 3 | 36 | 25 | At a terminal, vibe can call a configured model provider, the first… |
| lifecycle/phases | p | Seeing before doing | 1 | 38 | 25 | `--plan` reports what a phase run would do and changes nothing; every… |
| lifecycle/scrape | outcome |  | 1 | 26 | 20 | `../product-clean` exists, holds no `vibevm/` directory, no `vibe.toml`, no `vibe.lock` and no… |
| lifecycle/scrape | p | What happens | 1 | 39 | 25 | The agent runs `vibe scrape --plan`, which reads the project's contract, classifies… |
| lifecycle/scrape | p | What happens | 2 | 26 | 25 | Then it runs `vibe scrape --output ../product-clean`, which creates the scraped copy… |
| lifecycle/scrape | p | What happens | 3 | 30 | 25 | The copy is a plain project of its language: no manifest, no… |
| lifecycle/scrape | p | Edge cases and rules | 2 | 29 | 25 | `vibe clean` removes what vibe can regenerate and keeps the relationship; scrape… |
| lifecycle/scrape | prompt |  | 1 | 33 | 20 | Show me the scrape plan for the VibeVM project in the current… |
| model/boot-lane | p | The order of reading | 2 | 33 | 25 | At the end of that file sits a short block vibe maintains,… |
| model/boot-lane | p | The order of reading | 2 | 31 | 25 | It holds the text that must be in front of the agent… |
| model/boot-lane | p | The order of reading | 3 | 27 | 25 | It changes only when the set of packages or their versions change,… |
| model/boot-lane | p | The order of reading | 2 | 52 | 25 | Each entry names a file and says whether it is *static*, to… |
| model/boot-lane | p | Edge cases and rules | 2 | 29 | 25 | Rules are cited by the address of their source document, never by… |
| model/lock-and-store | p | The lock file | 1 | 27 | 25 | `vibe.lock` lists every package in the resolved graph, direct and transitive, with… |
| model/lock-and-store | p | The lock file | 1 | 40 | 25 | The lock file is kept even when derived state is removed: `vibe`… |
| model/lock-and-store | p | The machine store | 1 | 28 | 25 | Every package vibe fetches, for any project, lands in one store under… |
| model/lock-and-store | p | The machine store | 2 | 35 | 25 | A version fetched for one project is available to every other project… |
| model/lock-and-store | p | The machine store | 2 | 34 | 25 | `vibe cache add` fetches a package and everything it depends on without… |
| model/lock-and-store | p | Edge cases and rules | 1 | 30 | 25 | The settings folder, including the store, is `~/.vibe/` on every platform; the… |
| model/lock-and-store | p | Edge cases and rules | 1 | 29 | 25 | The store and the registry clone cache are two different folders: the… |
| model/packages-and-kinds | p | The eight kinds | 1 | 34 | 25 | The kind is metadata about the package, not part of its identity:… |
| model/packages-and-kinds | p | Families and companions | 2 | 44 | 25 | A package's manual is its *companion*, named with the suffix `-docs` in… |
| model/packages-and-kinds | p | Edge cases and rules | 1 | 32 | 25 | A short name without a group, such as `wal`, is accepted on… |
| model/registries | p | The index | 2 | 26 | 25 | So a registry may keep an *index*: a separate repository beside the… |
| model/registries | p | The index | 1 | 35 | 25 | The index location is derived from the registry's address and can be… |
| model/two-trees | p | The founding rule | 1 | 30 | 25 | Think of how a C++ program uses a library: you write `#include`,… |
| model/two-trees | p | Why the copies are committed | 2 | 31 | 25 | The reason is the reader: an agent that clones the repository must… |
| model/two-trees | p | Why the copies are committed | 3 | 33 | 25 | A committed tree makes the whole reading list a set of ordinary… |
| model/two-trees | p | What regenerates, and when | 1 | 36 | 25 | Three things in a project are derived from the manifest and the… |
| model/two-trees | p | What regenerates, and when | 2 | 27 | 25 | `vibe reinstall` rebuilds all three from the lock file and the machine… |
| model/two-trees | p | What regenerates, and when | 2 | 26 | 25 | It is the command for a clean start before a build, and… |
| model/two-trees | p | Edge cases and rules | 1 | 43 | 25 | If you develop packages inside the same repository, their sources live in… |
| model/versions | p | Asking for a version | 2 | 31 | 25 | In the manifest you name a constraint, not a version: `^1.0` means… |
| model/versions | p | Moving the pin | 2 | 31 | 25 | `vibe update` re-resolves and moves the pins, preferring to keep every package… |
| model/versions | p | What a version promises | 3 | 42 | 25 | This is also how vibe treats its own releases: the binary you… |
| model/versions | p | Versions of vibe itself | 1 | 37 | 25 | The program manages its own versions with `vibe self`: `self install` builds… |
| reference/commands | p | Global options | 1 | 37 | 25 | Every command takes `--json` for machine-readable output, `--quiet` for a one-line summary,… |
| reference/lock-file | p | Reading a diff | 2 | 31 | 25 | A changed `content_hash` with the same version is impossible in a healthy… |
| reference/machine-formats | p | One rule for every format | 1 | 49 | 25 | The schemas are JSON Typedef documents under `schemas/` in the source tree;… |
| reference/machine-formats | p | The envelope | 2 | 33 | 25 | Every object carries `ok`, the `command` that produced it, and the report… |
| reference/manifest | p | One file, three roles | 2 | 37 | 25 | The tables present decide the role: `[project]` marks a consumer that is… |
| start/first-project | outcome |  | 1 | 23 | 20 | a folder `hello-vibe` with `vibe.toml`, `vibe.lock`, a `vibevm/` directory, and `CLAUDE.md`, `AGENTS.md`… |
| start/first-project | p | What happens | 1 | 30 | 25 | The agent runs `vibe init hello-vibe`, which creates the folder with a… |
| start/first-project | p | What happens | 2 | 45 | 25 | It then runs `vibe install org.vibevm.world/wal --path hello-vibe`: vibe reads the manifest's… |
| start/first-project | p | What appeared on disk | 1 | 29 | 25 | Open a new agent session in `hello-vibe` and ask it what rules… |
| start/first-project | prompt |  | 1 | 32 | 20 | Create a VibeVM project named hello-vibe in the current folder, install the… |
| start/install-vibe | p | What happens | 2 | 27 | 25 | The script imports the binary into vibe's own managed store under `~/.vibe/opt`,… |
| start/install-vibe | p | By hand / On Windows | 2 | 22 | 20 | Compare the digest of `vibe.exe` with the line in that file before… |
| start/install-vibe | p | By hand / From the source, on any platform | 1 | 24 | 20 | Later, `vibe self install latest` rebuilds from the tip of the main… |
| start/install-vibe | p | Where things go | 1 | 37 | 25 | vibe keeps everything it owns under one folder in your home directory,… |
| start/install-vibe | p | Where things go | 1 | 26 | 25 | The installer never overwrites a running binary and never edits your `PATH`… |
| start/install-vibe | prompt |  | 1 | 24 | 20 | Install vibe on this machine: on Windows from the latest release archive… |
| start/what-a-project-contains | p | The files, one by one | 1 | 28 | 25 | `vibevm/vibepacks/` holds packages this repository develops in place: a project that also… |
| start/what-a-project-contains | p | The boot files | 2 | 32 | 25 | vibe rewrites what is between the two markers and nothing else in… |
| start/what-a-project-contains | p | Edge cases and rules | 1 | 29 | 25 | If you write into `vibevm/vibedeps/` by accident, nothing breaks immediately; the next… |
| start/what-vibevm-is | p | How it works | 3 | 51 | 25 | When you run `vibe install`, vibe resolves versions, fetches the packages once… |
| start/what-vibevm-is | p | How it works | 3 | 26 | 25 | The first file is a short block at the end of your… |

### 3.2 Абзацы длиннее 6 фраз (всего 3)

| страница | где | секция | фраз | начало абзаца |
|---|---|---|---|---|
| agent/how-agents-read-this-manual | p | The procedure the skill teaches | 10 | 1. On an error, read the address the message names, then the page the diagnostics page maps it to. 2… |
| architecture/how-vibe-is-built | p | Five layers | 7 | Read the product bottom-up. *Identity*: a package is a coordinate plus a content fingerprint, and th… |
| architecture/how-vibe-is-built | p | The path of an install | 16 | 1. Discover the workspace root and read the manifests, the lock file, the user configuration, the re… |

Примечание: все 3 абзаца — не «разросшаяся проза», а нумерованные процедуры (`1. … 2. … 3. …`), записанные внутри одного `<p>` без разбивки на отдельные шаги-секции; формально каждый пункт даёт минимум одну фразу, отсюда счёт. Факт, не решение — не переписываю разметку.

## 4. Заголовки

Всего: 12.

| страница | тег секции | title | причина |
|---|---|---|---|
| faq/index | q-commit-vibedeps | Do I commit the dependency tree? | ends-with-? |
| faq/index | q-conflict | A dependency deep in my tree clashes on a version. How do I fix it? | ends-with-? |
| faq/index | q-confirm | Why does install ask me to confirm? | ends-with-? |
| faq/index | q-llm | Does vibe call a language model? | ends-with-? |
| faq/index | q-two-versions | Can two versions of one package coexist in a project? | ends-with-? |
| faq/index | q-registry-gone | A registry I used has disappeared. Is my project stuck? | ends-with-? |
| faq/index | q-pin | How do I pin a package to an exact version? | ends-with-? |
| faq/index | q-edit-lock | Can I edit vibe.lock by hand to silence a hash error? | ends-with-? |
| faq/index | q-why-eight-kinds | Why so many package kinds? | ends-with-? |
| faq/index | q-offline | I am on a plane. What works? | ends-with-? |
| faq/index | q-remove-vibe | The project is done. How do I ship it without vibe? | ends-with-? |
| faq/index | q-docs-install | Why can I not install the manual into my project? | ends-with-? |

Все 12 — вопросительные заголовки на `faq/index.xml` (жанр страницы: вопрос-ответ). Generic-заголовков из списка (Overview/Summary/Conclusion/Next steps/Key takeaways/Introduction) не найдено ни одного. Факт, не решение — является ли вопросительный заголовок FAQ-страницы нарушением по смыслу правила, решает центральная сессия.

## 5. Знаки (!, эмодзи, **жирный** в прозе, >1 тире «—» в абзаце)

Не найдено ни одного случая ни по одной из 4 категорий на всех 44 страницах. Единственные не-ASCII знаки во всём корпусе — «», —, … (3 файла для «»/…, 1 файл (`diagnostics/errors.xml`) для — — все в пределах разрешённой типографики, и все 3 вхождения `—` лежат в разных `<td>`, не в одном абзаце, так что правило «не больше одного тире на абзац» не нарушено нигде.

## 6. Термины до введения (глоссарий)

50 терминов из `glossary/index.xml`. Всего первых вхождений на 43 страницах (кроме глоссария): 499, из них с проблемой: 308 (`used-in-first-paragraph` — термин во вводном абзаце, запрещено вовсе: 69; `first-use-not-explained` — первое вхождение далее без курсива/ссылки/пояснительного оборота: 239).

Ограничение метода (честно фиксирую, чтобы центральная сессия не переоценивала колонку «OK»): пояснительный оборот (`, the …`, `: …`, `that is`, `which is`) ищется по всей фразе, содержащей вхождение термина, а не рядом со словом — если оборот встретился в той же фразе по другой причине, строка помечена как «пояснено», хотя термин не пояснён. Значит реальное число нарушений `first-use-not-explained`, вероятно, выше 239; таблица — нижняя граница, не точное число.

### 6.1 Нарушения (полная таблица)

| страница | термин | нарушение | где | курсив | ссылка | контекст |
|---|---|---|---|---|---|---|
| agent/ask-your-agent | package | первое вхождение не пояснено | p |  |  | e mcp install`, which answers questions about the project's packages and can run the same operations without spawning a…` |
| agent/ask-your-agent | project | первое вхождение не пояснено | p |  |  | ed by `vibe mcp install`, which answers questions about the project's packages and can run the same operations without s… |
| agent/ask-your-agent | provider | первое вхождение не пояснено | p |  |  | son at a terminal, vibe can instead call a configured model provider, and pays for it only when the step actually runs. |
| agent/give-your-agent-the-skill | skill | во вводном абзаце (запрещено) | - | - | - | what vibe is and how to call it. This page installs a small skill into the agent's own folder, and, for agents that supp… |
| agent/give-your-agent-the-skill | agent session | первое вхождение не пояснено | outcome |  |  | te server entry and skill for each detected agent; the next agent session in this project knows the vibe commands and ca… |
| agent/give-your-agent-the-skill | family | первое вхождение не пояснено | p |  |  | om its own code, such as the discipline tools of a language family. `vibe mcp install` registers such servers in the age… |
| agent/give-your-agent-the-skill | package | первое вхождение не пояснено | outcome |  |  | project knows the vibe commands and can query the project's packages |
| agent/give-your-agent-the-skill | store | первое вхождение не пояснено | p |  |  | cs`, arrives the same way once the manual is in the machine store.` |
| agent/how-agents-read-this-manual | anchor | первое вхождение не пояснено | td |  |  | every page with its language, audiences, status and anchors |
| agent/how-agents-read-this-manual | block number | первое вхождение не пояснено | td |  |  | one page as plain Markdown, block numbers included |
| agent/how-agents-read-this-manual | boot lane | первое вхождение не пояснено | p |  |  | Text written for agents is never part of a project's boot lane; this manual is fetched when needed, not read at every se… |
| agent/how-agents-read-this-manual | package | первое вхождение не пояснено | p |  |  | rt exits zero. 4. When choosing among documentations of one package, prefer the one marked official and name the publish… |
| agent/how-agents-read-this-manual | project | первое вхождение не пояснено | p |  |  | Text written for agents is never part of a project's boot lane; this manual is fetched when needed, not read a |
| agent/how-agents-read-this-manual | specification | первое вхождение не пояснено | p |  |  | Rules on a page are quoted from the specification by address and shown in the specification's own language. P |
| agent/how-agents-read-this-manual | store | первое вхождение не пояснено | p |  |  | The same pages live in the machine store once `vibe cache add org.vibevm.core/vibevm-docs` has run. |
| architecture/how-vibe-is-built | family | во вводном абзаце (запрещено) | - | - | - | vibe is one binary built from a family of Rust libraries, each owning one concern: the manifest, r |
| architecture/how-vibe-is-built | manifest | во вводном абзаце (запрещено) | - | - | - | om a family of Rust libraries, each owning one concern: the manifest, resolution, the registry, the workspace on disk, t… |
| architecture/how-vibe-is-built | registry | во вводном абзаце (запрещено) | - | - | - | ies, each owning one concern: the manifest, resolution, the registry, the workspace on disk, the agent surfaces. This pa… |
| architecture/how-vibe-is-built | workspace | во вводном абзаце (запрещено) | - | - | - | ng one concern: the manifest, resolution, the registry, the workspace on disk, the agent surfaces. This page maps the li… |
| architecture/how-vibe-is-built | fact | первое вхождение не пояснено | td |  |  | ackends; the status markup parser and reports; the adoption-facts registry; traceability queries |
| architecture/how-vibe-is-built | lifecycle | первое вхождение не пояснено | td |  |  | the lifecycle |
| architecture/how-vibe-is-built | managed block | первое вхождение не пояснено | p |  |  | rified tree into the store. 5. Build the plan, validate the managed blocks of the instruction files, and ask for confirm… |
| architecture/how-vibe-is-built | MCP server | первое вхождение не пояснено | td |  |  | the MCP server and integration manager; skill projection into agents; thre |
| architecture/how-vibe-is-built | phase | первое вхождение не пояснено | td |  |  | the nine-phase model and chaining; the pure extension registry; surface-ne |
| architecture/how-vibe-is-built | provider | первое вхождение не пояснено | td |  |  | and the quarantined loader of native extensions; the model provider seam; scrape planning |
| architecture/how-vibe-is-built | scrape | первое вхождение не пояснено | td |  |  | tined loader of native extensions; the model provider seam; scrape planning |
| architecture/how-vibe-is-built | skill | первое вхождение не пояснено | td |  |  | the MCP server and integration manager; skill projection into agents; three-level preferences; frontend-a |
| architecture/how-vibe-is-built | specification | первое вхождение не пояснено | td |  |  | specifications |
| architecture/traceability | kind | первое вхождение не пояснено | p |  |  | ents it. `vibe query` filters the map by address, symbol or kind and returns the many nodes that fit. `vibe select --whe…` |
| architecture/traceability | package | первое вхождение не пояснено | p |  |  | `specmap.toml`; the engine itself ships with the discipline package and is vendored, never edited in place. |
| architecture/what-the-lifecycle-epic-delivered | package | во вводном абзаце (запрещено) | - | - | - | Between July and September 2026 vibe grew from a package installer into a build system with an extension machine, a |
| architecture/what-the-lifecycle-epic-delivered | receipt | во вводном абзаце (запрещено) | - | - | - | uild system with an extension machine, a native ABI, deploy receipts and a terminal export. This page tells that route a… |
| architecture/what-the-lifecycle-epic-delivered | feature | первое вхождение не пояснено | p |  |  | ames every commit. Read them as one story rather than eight features. |
| architecture/what-the-lifecycle-epic-delivered | fingerprint (freshness) | первое вхождение не пояснено | td |  |  | ansform positions with owner-scoped activation and per-unit fingerprints, a built-in XML minifier, and the lane analyser… |
| architecture/what-the-lifecycle-epic-delivered | hook | первое вхождение не пояснено | td |  |  | ed and never wipes a tree; a hash gate for mutable sources; hooks that run only on a real change |
| architecture/what-the-lifecycle-epic-delivered | override | первое вхождение не пояснено | p |  |  | ly extension tier, per-language and per-resource preference overrides, cloud sync of user preferences, a built-in infere… |
| architecture/what-the-lifecycle-epic-delivered | package | первое вхождение не пояснено | td |  |  | R8, package, build and deploy |
| architecture/what-the-lifecycle-epic-delivered | provider | первое вхождение не пояснено | td |  |  | R7, providers and agents |
| architecture/what-the-lifecycle-epic-delivered | registry | первое вхождение не пояснено | td |  |  | one pure extension registry below the lifecycle and the workspace, four transform posit |
| architecture/what-the-lifecycle-epic-delivered | specification | первое вхождение не пояснено | p |  |  | lean install` for the clean lifecycle. Each is named in its specification with a compatibility law, so the door stays op…` |
| architecture/what-the-lifecycle-epic-delivered | traceability map | первое вхождение не пояснено | p |  |  | ered stages, each landed as atomic commits with tests and a traceability map, and recorded in a ledger that names every… |
| architecture/what-the-lifecycle-epic-delivered | workspace | первое вхождение не пояснено | td |  |  | one pure extension registry below the lifecycle and the workspace, four transform positions with owner-scoped activation… |
| authoring/ship-tools-and-mcp-servers | package | во вводном абзаце (запрещено) | - | - | - | A package can deliver programs: command-line tools built on install, |
| authoring/ship-tools-and-mcp-servers | lock file | первое вхождение не пояснено | p |  |  | be bin exec`, which resolves the tool through the project's lock file to the artifact of the exact version installed and…` |
| authoring/ship-tools-and-mcp-servers | manifest | первое вхождение не пояснено | outcome |  |  | the manifest carries a `[[binary]]` table; `vibe bin list` shows the too |
| authoring/ship-tools-and-mcp-servers | package | первое вхождение не пояснено | prompt |  |  | In the package packages/notes-tools of the current VibeVM project, declare |
| authoring/ship-tools-and-mcp-servers | project | первое вхождение не пояснено | prompt |  |  | In the package packages/notes-tools of the current VibeVM project, declare the Rust crate crates/notes-check as a binary… |
| authoring/ship-tools-and-mcp-servers | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a package with a Cargo workspace |
| authoring/ship-tools-and-mcp-servers | workspace | первое вхождение не пояснено | needs |  |  | bevm skill installed for your agent; a package with a Cargo workspace at its root and a binary crate; the Rust toolchain… |
| authoring/specs-agents-can-cite | package | во вводном абзаце (запрещено) | - | - | - | The text in a package is only useful to an agent if every rule in it has an addre |
| authoring/specs-agents-can-cite | fact | первое вхождение не пояснено | p |  |  | the freshest installed version. Anchors are section ids and fact ids in one address space, so a rule is cited the same w… |
| authoring/specs-agents-can-cite | project | первое вхождение не пояснено | p |  |  | A specification is written in Markdown or in the project's XML dialect, and both parse into one document model. In X |
| authoring/specs-agents-can-cite | specification | первое вхождение не пояснено | p |  |  | g else: no hallway, no shared memory, no tone of voice. The specification tree is the only channel between them, and a c… |
| authoring/translate-documentation | anchor | во вводном абзаце (запрещено) | - | - | - | package in the same shape as the original: same pages, same anchors, same examples by reference, in another language. Th… |
| authoring/translate-documentation | package | во вводном абзаце (запрещено) | - | - | - | A translation is a separate package in the same shape as the original: same pages, same anchors |
| authoring/translate-documentation | translation | во вводном абзаце (запрещено) | - | - | - | A translation is a separate package in the same shape as the original: sa |
| authoring/translate-documentation | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; the source documentation package |
| authoring/translate-documentation | specification | первое вхождение не пояснено | p |  |  | Rules quoted from a specification stay in the specification's language, marked as such; trans |
| authoring/write-a-feat-or-stack | contribution | первое вхождение не пояснено | p |  |  | A stack may also bind lifecycle contributions in its manifest, so that `vibe build` and `vibe test` in a |
| authoring/write-a-feat-or-stack | fact | первое вхождение не пояснено | p |  |  | at the agent checks after a build; write them as observable facts, not as wishes. |
| authoring/write-a-feat-or-stack | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` at the |
| authoring/write-a-flow | index (of a registry) | первое вхождение не пояснено | p |  |  | is linked, compiled into the priority lane or listed in the index; you may suggest a default in `[boot_snippet]`, and th… |
| authoring/write-a-flow | package | первое вхождение не пояснено | prompt |  |  | Create a flow package org.acme/review-notes in the folder packages/review-notes o |
| authoring/write-a-flow | project | первое вхождение не пояснено | prompt |  |  | s in the folder packages/review-notes of the current VibeVM project. It should teach an agent to leave a short REVIEW.md… |
| authoring/write-a-flow | registry | первое вхождение не пояснено | td |  |  | what the flow is, shown on the registry and the site |
| authoring/write-a-flow | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` at the |
| authoring/write-a-lang-package | package | во вводном абзаце (запрещено) | - | - | - | A lang package teaches an agent how to write in a language or a notation: |
| authoring/write-a-lang-package | kind | первое вхождение не пояснено | p |  |  | and its snippet the way a flow's are written, and sets the kind to `lang`. The difference from a flow is the genre, not… |
| authoring/write-a-lang-package | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` at the |
| authoring/write-documentation | package | во вводном абзаце (запрещено) | - | - | - | A manual for a package is itself a package: it names what it documents, carries a |
| authoring/write-documentation | anchor | первое вхождение не пояснено | td |  |  | the anchor exists |
| authoring/write-documentation | coordinate | первое вхождение не пояснено | p |  |  | es out. The `title` is the display name on every shelf; the coordinate stays the identity, and the publisher is shown be… |
| authoring/write-documentation | index (of a registry) | первое вхождение не пояснено | p |  |  | ual with `vibe cache add`, and the site renders it when the index announces it. |
| authoring/write-documentation | official documentation | первое вхождение не пояснено | p |  |  | lished it under the `-docs` name, shows it as the subject's official documentation. |
| authoring/write-documentation | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` at the |
| authoring/write-documentation | specification | первое вхождение не пояснено | p |  |  | ations` resolves every `rule` address against the subject's specification and fails on any that does not exist. The manu…` |
| diagnostics/errors | contribution | первое вхождение не пояснено | td |  |  | a contribution reported `fail` and the chain stopped |
| diagnostics/errors | coordinate | первое вхождение не пояснено | td |  |  | the coordinate is already in the lock file with the same content |
| diagnostics/errors | git source | первое вхождение не пояснено | td |  |  | the git source names a tag, branch or commit the repository does not have |
| diagnostics/errors | kind | первое вхождение не пояснено | td |  |  | an instruction file has two markers of a kind, an opener without a closer, or a closer first |
| diagnostics/errors | lock file | первое вхождение не пояснено | td |  |  | the coordinate is already in the lock file with the same content |
| diagnostics/errors | manifest | первое вхождение не пояснено | td |  |  | the package's manifest points at a snippet file it does not ship |
| diagnostics/errors | package | первое вхождение не пояснено | td |  |  | a package the lock file pins has no folder on disk and the regenerati |
| diagnostics/errors | phase | первое вхождение не пояснено | td |  |  | one step of a phase failed; the phases before it kept their results |
| diagnostics/errors | project | первое вхождение не пояснено | td |  |  | no project at or above the current folder |
| diagnostics/errors | receipt | первое вхождение не пояснено | td |  |  | a file a receipt owns changed after deployment |
| diagnostics/errors | registry | первое вхождение не пояснено | td |  |  | every configured registry answered that it has no such package |
| diagnostics/errors | store | первое вхождение не пояснено | td |  |  | `--offline` was set and the store does not hold the package |
| faq/index | feature | первое вхождение не пояснено | p |  |  | configured, only for that step and only if you switched the feature on. See [Ask your agent to do the work](../agent/ask… |
| faq/index | kind | первое вхождение не пояснено | p |  |  | pin what it serves, a feature cannot name a framework. Each kind is one genre, and a word that meant two genres was spli… |
| faq/index | lifecycle | первое вхождение не пояснено | p |  |  | vibe can regenerate. See [Remove VibeVM from a project](../lifecycle/scrape.xml). |
| faq/index | manifest | первое вхождение не пояснено | p |  |  | ate` to write the resolved version as an exact pin into the manifest. Without `--exact`, the manifest keeps a range and…` |
| faq/index | mirror | первое вхождение не пояснено | p |  |  | he store. For the long run, `vibe registry vendor` writes a mirror folder of everything the lock file references, which… |
| faq/index | package | первое вхождение не пояснено | p |  |  | he dependency's own constraint must change. One version per package across the whole workspace is the rule, so vibe stop… |
| faq/index | provider | первое вхождение не пояснено | p |  |  | ion for the agent that hosts it, or, at a terminal, calls a provider you configured, only for that step and only if you… |
| faq/index | scrape | первое вхождение не пояснено | p |  |  | regenerate. See [Remove VibeVM from a project](../lifecycle/scrape.xml). |
| faq/index | skill | первое вхождение не пояснено | p |  |  | ly with `vibe doc serve`, and reached by an agent through a skill or by address. See [Read documentation locally](../how… |
| faq/index | workspace | первое вхождение не пояснено | p |  |  | raint must change. One version per package across the whole workspace is the rule, so vibe stops rather than installing… |
| howto/install-a-package | package | во вводном абзаце (запрещено) | - | - | - | You found a package your project should follow. This page adds it to the projec |
| howto/install-a-package | project | во вводном абзаце (запрещено) | - | - | - | You found a package your project should follow. This page adds it to the project, records th |
| howto/install-a-package | coordinate | первое вхождение не пояснено | p |  |  | 1. Install by coordinate. Add `@` and a constraint to ask for a range or an exact ve |
| howto/install-a-package | lock file | первое вхождение не пояснено | p |  |  | range. The manifest keeps the constraint you asked for; the lock file keeps the version you got. |
| howto/install-a-package | manifest | первое вхождение не пояснено | p |  |  | exact` writes the resolved version as an exact pin into the manifest instead of a range. The manifest keeps the constrai…` |
| howto/install-a-package | package | первое вхождение не пояснено | prompt |  |  | Install the package org.vibevm.world/wal into the VibeVM project in the current |
| howto/install-a-package | project | первое вхождение не пояснено | prompt |  |  | Install the package org.vibevm.world/wal into the VibeVM project in the current folder, accept the plan, and tell me whi… |
| howto/install-a-package | registry | первое вхождение не пояснено | p |  |  | A public registry that answers with an authentication error for a missing pac |
| howto/install-a-package | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` in the |
| howto/install-a-package | store | первое вхождение не пояснено | needs |  |  | project's registries, or the package already in the machine store |
| howto/publish-a-package | package | во вводном абзаце (запрещено) | - | - | - | You wrote a package and want others to install it. This page publishes it to a |
| howto/publish-a-package | project | во вводном абзаце (запрещено) | - | - | - | s own repository, tags the version, and checks that a fresh project can install it. |
| howto/publish-a-package | registry | во вводном абзаце (запрещено) | - | - | - | and want others to install it. This page publishes it to a registry as its own repository, tags the version, and checks… |
| howto/publish-a-package | coordinate | первое вхождение не пояснено | outcome |  |  | a repository named after the package's coordinate exists in the registry organisation with a tag for the vers |
| howto/publish-a-package | index (of a registry) | первое вхождение не пояснено | p |  |  | ion. The package is then one more repository the registry's index will pick up. To prove it, the agent creates a scratch… |
| howto/publish-a-package | manifest | первое вхождение не пояснено | p |  |  | ` picks a registry by name; without it the first one in the manifest is used:` |
| howto/publish-a-package | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a publish token for the registry' |
| howto/read-documentation-locally | package | во вводном абзаце (запрещено) | - | - | - | Documentation of the packages you use, including private ones, can be read on your own ma |
| howto/read-documentation-locally | store | во вводном абзаце (запрещено) | - | - | - | othing sent anywhere. This page fetches the manual into the store and opens the reader in a browser. |
| howto/read-documentation-locally | project | первое вхождение не пояснено | p |  |  | vm-docs`. A documentation package is never installed into a project; it is warmed into the machine store together with t…` |
| howto/read-documentation-locally | registry | первое вхождение не пояснено | needs |  |  | ibevm skill installed for your agent; network access to the registry once, or the package already in the store |
| howto/read-documentation-locally | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; network access to the registry on |
| howto/read-documentation-locally | subject | первое вхождение не пояснено | p |  |  | Warming a documentation package warms its subjects too, so the rules a page quotes resolve without a network. |
| howto/read-documentation-locally | translation | первое вхождение не пояснено | p |  |  | reader shows the manual's own language where a page has no translation, and says so. |
| howto/remove-a-package | package | во вводном абзаце (запрещено) | - | - | - | Removing a package takes its text out of the project and out of the agent's re |
| howto/remove-a-package | project | во вводном абзаце (запрещено) | - | - | - | Removing a package takes its text out of the project and out of the agent's reading list, and leaves everything |
| howto/remove-a-package | agent session | первое вхождение не пояснено | p |  |  | it removes them and regenerates the boot files, so the next agent session no longer reads the package's snippet. Your ow… |
| howto/remove-a-package | coordinate | первое вхождение не пояснено | p |  |  | 1. Remove by coordinate; the version is not needed: |
| howto/remove-a-package | package | первое вхождение не пояснено | prompt |  |  | Remove the package org.vibevm.world/wal from the VibeVM project in the current |
| howto/remove-a-package | project | первое вхождение не пояснено | prompt |  |  | Remove the package org.vibevm.world/wal from the VibeVM project in the current folder and confirm that nothing of it rem… |
| howto/remove-a-package | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project in which the package is |
| howto/set-up-a-workspace | lock file | во вводном абзаце (запрещено) | - | - | - | developed together can live in one repository and share one lock file. This page turns a folder into such a workspace an… |
| howto/set-up-a-workspace | package | во вводном абзаце (запрещено) | - | - | - | Several packages developed together can live in one repository and share one |
| howto/set-up-a-workspace | workspace | во вводном абзаце (запрещено) | - | - | - | d share one lock file. This page turns a folder into such a workspace and shows how members refer to each other by path. |
| howto/set-up-a-workspace | kind | первое вхождение не пояснено | p |  |  | irements. The lock file records such an entry with a source kind of `path` and the member's folder relative to the root,… |
| howto/set-up-a-workspace | lock file | первое вхождение не пояснено | prompt |  |  | s documents notes-flow. Run an install and show me that one lock file at the root covers both members. |
| howto/set-up-a-workspace | manifest | первое вхождение не пояснено | p |  |  | The agent adds a `[workspace]` table to the root manifest naming the member paths, and creates each member with `vibe` |
| howto/set-up-a-workspace | registry | первое вхождение не пояснено | p |  |  | A member requires a sibling by path rather than by registry, with a `path` source in its requirements. The lock file re |
| howto/set-up-a-workspace | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` at the |
| howto/update-packages | lock file | во вводном абзаце (запрещено) | - | - | - | on, moves one or all of them forward, and explains what the lock file does during the move. |
| howto/update-packages | package | во вводном абзаце (запрещено) | - | - | - | Packages change. This page shows which of yours have a newer version |
| howto/update-packages | fingerprint (freshness) | первое вхождение не пояснено | p |  |  | t. Each changed entry names the old and the new version and fingerprint. |
| howto/update-packages | index (of a registry) | первое вхождение не пояснено | p |  |  | tdated` needs the registry and, for the fastest answer, its index; without a network it reports what it could not reach…` |
| howto/update-packages | manifest | первое вхождение не пояснено | p |  |  | te moves the pin in the lock file within the constraint the manifest names; it never widens or narrows the constraint. I… |
| howto/update-packages | registry | первое вхождение не пояснено | p |  |  | ares every pin in the lock file with the newest version the registry offers and prints the difference; it changes nothin… |
| howto/update-packages | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` and `v` |
| howto/use-a-private-registry | index (of a registry) | во вводном абзаце (запрещено) | - | - | - | r a whole machine at a private registry, with or without an index, and keeps the public one as a fallback. |
| howto/use-a-private-registry | package | во вводном абзаце (запрещено) | - | - | - | A company can keep its packages in its own place and still use vibe unchanged. This page po |
| howto/use-a-private-registry | project | во вводном абзаце (запрещено) | - | - | - | own place and still use vibe unchanged. This page points a project or a whole machine at a private registry, with or wit… |
| howto/use-a-private-registry | registry | во вводном абзаце (запрещено) | - | - | - | This page points a project or a whole machine at a private registry, with or without an index, and keeps the public one… |
| howto/use-a-private-registry | fingerprint (freshness) | первое вхождение не пояснено | p |  |  | ive address of the same packages, verified against the same fingerprints; use `registry add` for a different source of p… |
| howto/use-a-private-registry | index (of a registry) | первое вхождение не пояснено | p |  |  | A private registry without an index still works; searches skip it and installs clone what they |
| howto/use-a-private-registry | manifest | первое вхождение не пояснено | p |  |  | imary`, which writes a new registry block at the top of the manifest's list. From now on every resolution asks `acme` fi…` |
| howto/use-a-private-registry | mirror | первое вхождение не пояснено | p |  |  | A mirror is not a second registry. Use `vibe registry set-mirror` fo |
| howto/use-a-private-registry | package | первое вхождение не пояснено | p |  |  | ion asks `acme` first and the public registry second, and a package that exists in both comes from `acme`. `vibe registr…` |
| howto/use-a-private-registry | project | первое вхождение не пояснено | prompt |  |  | t@github.com:acme-specs as the first registry of the VibeVM project in the current folder, authenticated over SSH, keep… |
| howto/use-a-private-registry | registry | первое вхождение не пояснено | prompt |  |  | Add the private registry named acme at git@github.com:acme-specs as the first regist |
| howto/use-a-private-registry | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; an SSH key that the private host |
| howto/use-a-private-registry | store | первое вхождение не пояснено | p |  |  | . Keep the variable in your shell profile or your CI secret store, not in the repository. |
| howto/work-offline | store | во вводном абзаце (запрещено) | - | - | - | ng from what the machine already holds. This page warms the store before you leave and installs from it without a networ… |
| howto/work-offline | coordinate | первое вхождение не пояснено | p |  |  | never reached the store is a hard error naming the missing coordinate. |
| howto/work-offline | lock file | первое вхождение не пояснено | p |  |  | registry vendor` writes a folder holding every package the lock file references, ready to be used as a mirror with a `fi… |
| howto/work-offline | mirror | первое вхождение не пояснено | p |  |  | ery package the lock file references, ready to be used as a mirror with a `file://` address on the other side. The finge… |
| howto/work-offline | registry | первое вхождение не пояснено | p |  |  | For a machine that never sees the registry, `vibe registry vendor` writes a folder holding every packa |
| howto/work-offline | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; network access now; a project wit |
| lifecycle/build-package-deploy | project | во вводном абзаце (запрещено) | - | - | - | This page takes a project from source to a running deployment with three commands, sh |
| lifecycle/build-package-deploy | deploy profile | первое вхождение не пояснено | needs |  |  | ect whose manifest declares build and package targets and a deploy profile named `local`; the project's stack tools on t… |
| lifecycle/build-package-deploy | fingerprint (freshness) | первое вхождение не пояснено | p |  |  | `--force` on a lifecycle verb ignores the recorded fingerprints for one run; it does not change what the run means. |
| lifecycle/build-package-deploy | lifecycle | первое вхождение не пояснено | p |  |  | `vibe deploy --plan` runs the whole default lifecycle in planning mode and prints what each phase would do, endin |
| lifecycle/build-package-deploy | manifest | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project whose manifest declares build and package targets and a deploy prof… |
| lifecycle/build-package-deploy | package | первое вхождение не пояснено | needs |  |  | for your agent; a project whose manifest declares build and package targets and a deploy profile named `local`; the proj… |
| lifecycle/build-package-deploy | phase | первое вхождение не пояснено | p |  |  | ole default lifecycle in planning mode and prints what each phase would do, ending with the ordered targets of the profi… |
| lifecycle/build-package-deploy | receipt | первое вхождение не пояснено | outcome |  |  | le with a generation and a status; after the undeploy every receipt-owned resource is gone and the listing shows the pro… |
| lifecycle/build-package-deploy | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project whose manifest declares |
| lifecycle/extensions-and-providers | contribution | во вводном абзаце (запрещено) | - | - | - | ge, or hand a task to an agent. This page explains how such contributions are declared, how a project turns them on, and… |
| lifecycle/extensions-and-providers | lifecycle | во вводном абзаце (запрещено) | - | - | - | A package can plug into the lifecycle: run a step, transform a stage, or hand a task to an agent. |
| lifecycle/extensions-and-providers | package | во вводном абзаце (запрещено) | - | - | - | A package can plug into the lifecycle: run a step, transform a stage, |
| lifecycle/extensions-and-providers | project | во вводном абзаце (запрещено) | - | - | - | is page explains how such contributions are declared, how a project turns them on, and how to see which ones ran and why… |
| lifecycle/extensions-and-providers | family | первое вхождение не пояснено | p |  |  | on points*, strings of the form `family:name`. The `phase:` family is the nine phases; the `slot:` family names places i… |
| lifecycle/extensions-and-providers | feature | первое вхождение не пояснено | p |  |  | ually reaches the point of calling it. Every model-enhanced feature declares whether it is off, assisting or required, a… |
| lifecycle/extensions-and-providers | handler | первое вхождение не пояснено | p |  |  | sform the text an agent will read. A *contribution* binds a handler to a point and is declared in a manifest as an `[[ex…` |
| lifecycle/extensions-and-providers | kind | первое вхождение не пояснено | td |  |  | Kind |
| lifecycle/extensions-and-providers | lifecycle | первое вхождение не пояснено | p |  |  | The lifecycle exposes named *extension points*, strings of the form `fami` |
| lifecycle/extensions-and-providers | manifest | первое вхождение не пояснено | p |  |  | ntribution* binds a handler to a point and is declared in a manifest as an `[[extension]]` table, in a package or in the… |
| lifecycle/extensions-and-providers | package | первое вхождение не пояснено | p |  |  | mpile:` family is the boot compiler's own pipeline, where a package may transform the text an agent will read. A *contri…` |
| lifecycle/extensions-and-providers | phase | первое вхождение не пояснено | p |  |  | of the form `family:name`. The `phase:` family is the nine phases; the `slot:` family names places inside a phase where… |
| lifecycle/extensions-and-providers | project | первое вхождение не пояснено | p |  |  | anifest as an `[[extension]]` table, in a package or in the project itself. |
| lifecycle/phases | workspace | во вводном абзаце (запрещено) | - | - | - | has a fixed order of steps, and vibe names them: check the workspace, produce generated sources, build, test, let an age… |
| lifecycle/phases | contribution | первое вхождение не пояснено | p |  |  | phase whose inputs did not change. Freshness is judged per contribution, so one stale step re-runs alone and its neighbo… |
| lifecycle/phases | fingerprint (freshness) | первое вхождение не пояснено | p |  |  | Every phase run records a fingerprint of the inputs it declared; the next run skips a phase whose |
| lifecycle/phases | lifecycle | первое вхождение не пояснено | p |  |  | vibe has two lifecycles. `clean` has one phase and removes derived state. `default` |
| lifecycle/phases | override | первое вхождение не пояснено | p |  |  | he manifest of the project can switch a contribution off or override it by its id. |
| lifecycle/phases | phase | первое вхождение не пояснено | p |  |  | vibe has two lifecycles. `clean` has one phase and removes derived state. `default` has nine phases in a f |
| lifecycle/phases | project | первое вхождение не пояснено | td |  |  | sources from specifications and prompts, written where the project's stack says |
| lifecycle/phases | specification | первое вхождение не пояснено | td |  |  | derived sources from specifications and prompts, written where the project's stack says |
| lifecycle/scrape | project | во вводном абзаце (запрещено) | - | - | - | When a project is finished and must leave without a trace of the tool that |
| lifecycle/scrape | managed block | первое вхождение не пояснено | outcome |  |  | `vibevm/` directory, no `vibe.toml`, no `vibe.lock` and no managed block in the agent instruction files, and its native… |
| lifecycle/scrape | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; a project with `vibe.toml` and a |
| model/boot-lane | index (of a registry) | во вводном абзаце (запрещено) | - | - | - | s on. The list has a fixed first file, read in full, and an index of the rest. Some entries are read always, some only w… |
| model/boot-lane | project | во вводном абзаце (запрещено) | - | - | - | rdered list of files that vibe computed from everything the project depends on. The list has a fixed first file, read in… |
| model/boot-lane | agent session | первое вхождение не пояснено | p |  |  | An agent session begins with the instruction file its vendor reads, `CLAUDE.` |
| model/boot-lane | manifest | первое вхождение не пояснено | p |  |  | `INDEX.md` is a manifest, not a payload. Each entry names a file and says whether it |
| model/boot-lane | package | первое вхождение не пояснено | p |  |  | front of the agent before anything else, assembled from the packages that asked to be read that way, each package's text… |
| model/boot-lane | skill | первое вхождение не пояснено | p |  |  | ippet by definition; an agent reaches this manual through a skill or by address, when it needs it. |
| model/lock-and-store | fingerprint (freshness) | во вводном абзаце (запрещено) | - | - | - | exactly which package versions your project got, down to a fingerprint of their content, so a teammate installs the same… |
| model/lock-and-store | lock file | во вводном абзаце (запрещено) | - | - | - | The lock file records exactly which package versions your project got, do |
| model/lock-and-store | package | во вводном абзаце (запрещено) | - | - | - | The lock file records exactly which package versions your project got, down to a fingerprint of their c |
| model/lock-and-store | project | во вводном абзаце (запрещено) | - | - | - | The lock file records exactly which package versions your project got, down to a fingerprint of their content, so a team… |
| model/lock-and-store | store | во вводном абзаце (запрещено) | - | - | - | content, so a teammate installs the same bytes. The machine store keeps those bytes once per computer, so a second proje… |
| model/lock-and-store | project | первое вхождение не пояснено | p |  |  | Every package vibe fetches, for any project, lands in one store under your home directory, `~/.vibe/cac` |
| model/lock-and-store | store | первое вхождение не пояснено | p |  |  | Every package vibe fetches, for any project, lands in one store under your home directory, `~/.vibe/cache/`, keyed by th… |
| model/packages-and-kinds | feature | во вводном абзаце (запрещено) | - | - | - | at a package is for before you open it: a way of working, a feature, a technology, a tool, a language guide, an agent se… |
| model/packages-and-kinds | kind | во вводном абзаце (запрещено) | - | - | - | t and the text or tools it delivers. Packages come in eight kinds, and the kind tells you what a package is for before y… |
| model/packages-and-kinds | manifest | во вводном абзаце (запрещено) | - | - | - | verything vibe installs is a package: a folder with a small manifest and the text or tools it delivers. Packages come in… |
| model/packages-and-kinds | package | во вводном абзаце (запрещено) | - | - | - | Everything vibe installs is a package: a folder with a small manifest and the text or tools it de |
| model/packages-and-kinds | family | первое вхождение не пояснено | td |  |  | that says how a feature is built with it, or a bundle of a family's members at one version |
| model/packages-and-kinds | feature | первое вхождение не пояснено | td |  |  | a technology context that says how a feature is built with it, or a bundle of a family's members at one |
| model/packages-and-kinds | index (of a registry) | первое вхождение не пояснено | p |  |  | epted on the command line and resolved through the registry index; it is convenience, and the manifest always records th… |
| model/packages-and-kinds | kind | первое вхождение не пояснено | p |  |  | dress when one is needed: `org.vibevm.world/wal@1.0.0`. The kind is not part of the name. It may be written as a prefix… |
| model/packages-and-kinds | manifest | первое вхождение не пояснено | p |  |  | osed and grows only by an amendment to the specification; a manifest with an unknown kind is rejected rather than guesse… |
| model/packages-and-kinds | package | первое вхождение не пояснено | p |  |  | A package is a project made installable. It has the same layout as a |
| model/packages-and-kinds | project | первое вхождение не пояснено | p |  |  | A package is a project made installable. It has the same layout as a project, with |
| model/packages-and-kinds | registry | первое вхождение не пояснено | p |  |  | `, is accepted on the command line and resolved through the registry index; it is convenience, and the manifest always r…` |
| model/packages-and-kinds | specification | первое вхождение не пояснено | p |  |  | The set is closed and grows only by an amendment to the specification; a manifest with an unknown kind is rejected rathe… |
| model/packages-and-kinds | subject | первое вхождение не пояснено | p |  |  | demand a new manual. The manual says which versions of its subject it describes, and the site picks the newest manual th… |
| model/registries | index (of a registry) | во вводном абзаце (запрещено) | - | - | - | age. A project lists the registries it trusts, in order. An index beside the registry answers searches without cloning a… |
| model/registries | package | во вводном абзаце (запрещено) | - | - | - | A registry is where packages are published: by default a public organisation on GitHub, |
| model/registries | project | во вводном абзаце (запрещено) | - | - | - | ublic organisation on GitHub, one repository per package. A project lists the registries it trusts, in order. An index b… |
| model/registries | registry | во вводном абзаце (запрещено) | - | - | - | A registry is where packages are published: by default a public organi |
| model/registries | coordinate | первое вхождение не пояснено | p |  |  | package is its own git repository named after the package's coordinate. Publishing means pushing a repository and taggin… |
| model/registries | manifest | первое вхождение не пояснено | p |  |  | A project declares its registries in the manifest as an ordered list. Each entry has a local name, the organi |
| model/registries | package | первое вхождение не пояснено | p |  |  | sation, such as `https://github.com/vibespecs`, where every package is its own git repository named after the package's… |
| model/registries | project | первое вхождение не пояснено | p |  |  | A project declares its registries in the manifest as an ordered list. |
| model/registries | registry | первое вхождение не пояснено | p |  |  | A registry is not a server vibe runs. It is a hosting organisation, su |
| model/two-trees | kind | во вводном абзаце (запрещено) | - | - | - | A project keeps two kinds of text apart: the rules your team wrote, and the copies of |
| model/two-trees | package | во вводном абзаце (запрещено) | - | - | - | wrote, and the copies of shared rules that arrived with the packages you installed. Installing a package never edits you… |
| model/two-trees | project | во вводном абзаце (запрещено) | - | - | - | A project keeps two kinds of text apart: the rules your team wrote, a |
| model/two-trees | package | первое вхождение не пояснено | p |  |  | text. Your specifications live in `vibevm/vibespecs/`; the packages you depend on are copied, whole and unchanged, into… |
| model/two-trees | registry | первое вхождение не пояснено | p |  |  | rom the lock file and the machine store, without asking any registry; add `--force` to fetch the package files again fro… |
| model/two-trees | specification | первое вхождение не пояснено | p |  |  | into your files. vibe follows the same rule for text. Your specifications live in `vibevm/vibespecs/`; the packages you… |
| model/two-trees | store | первое вхождение не пояснено | p |  |  | tall` rebuilds all three from the lock file and the machine store, without asking any registry; add `--force` to fetch t…` |
| model/versions | lock file | во вводном абзаце (запрещено) | - | - | - | not a snapshot of files. Your project asks for a range, the lock file pins one number, and an update moves the pin on pu… |
| model/versions | project | во вводном абзаце (запрещено) | - | - | - | is a promise about behaviour, not a snapshot of files. Your project asks for a range, the lock file pins one number, and… |
| model/versions | family | первое вхождение не пояснено | p |  |  | A family of packages that must move together pins its members exactl |
| model/versions | lock file | первое вхождение не пояснено | p |  |  | ckage that satisfies every constraint in the graph, and the lock file records the choice. |
| model/versions | registry | первое вхождение не пояснено | p |  |  | `vibe outdated` reads the lock file and the registry and lists the packages with a newer version available; it c |
| reference/lock-file | lock file | во вводном абзаце (запрещено) | - | - | - | The lock file is written by vibe and committed by you. This page explains |
| reference/lock-file | coordinate | первое вхождение не пояснено | td |  |  | the coordinates the manifests asked for directly; the baseline of the fresh |
| reference/lock-file | feature | первое вхождение не пояснено | td |  |  | the active features and subskills recorded for the package |
| reference/lock-file | fingerprint (freshness) | первое вхождение не пояснено | td |  |  | the fingerprint of the package's shippable tree; the identity; verified on |
| reference/lock-file | kind | первое вхождение не пояснено | td |  |  | the package's kind, for placement and filters |
| reference/lock-file | manifest | первое вхождение не пояснено | p |  |  | .lock` per workspace, at the absolute root, beside the root manifest; members never have their own. `vibe install` and `… |
| reference/lock-file | mirror | первое вхождение не пояснено | td |  |  | me; informational, always the canonical address even when a mirror served |
| reference/lock-file | package | первое вхождение не пояснено | td |  |  | the package's kind, for placement and filters |
| reference/lock-file | project | первое вхождение не пояснено | p |  |  | e is a mirror or a host migration and means nothing for the project. A new entry with `source_kind = "override"` is a pa… |
| reference/lock-file | registry | первое вхождение не пояснено | td |  |  | the name of the registry that answered, from the manifest's list |
| reference/lock-file | subskill | первое вхождение не пояснено | td |  |  | the active features and subskills recorded for the package |
| reference/lock-file | workspace | первое вхождение не пояснено | p |  |  | There is one `vibe.lock` per workspace, at the absolute root, beside the root manifest; members ne |
| reference/machine-formats | feature | первое вхождение не пояснено | p |  |  | A document that no schema describes is a defect, not a feature; the campaign that wrote this manual filed the ones it fo… |
| reference/machine-formats | lifecycle | первое вхождение не пояснено | td |  |  | the lifecycle verbs with `--json` |
| reference/machine-formats | manifest | первое вхождение не пояснено | p |  |  | a report a script parses, a file another language writes, a manifest a browser fetches, is described by a schema and reg… |
| reference/machine-formats | package | первое вхождение не пояснено | td |  |  | the installed packages |
| reference/machine-formats | registry | первое вхождение не пояснено | p |  |  | Typedef documents under `schemas/` in the source tree; the registry is `formats/REGISTRY.toml`, which records for each f… |
| reference/manifest | manifest | во вводном абзаце (запрещено) | - | - | - | The manifest is the one file you write to describe a project or a packag |
| reference/manifest | package | во вводном абзаце (запрещено) | - | - | - | nifest is the one file you write to describe a project or a package: its name, what it depends on, where packages come f… |
| reference/manifest | project | во вводном абзаце (запрещено) | - | - | - | The manifest is the one file you write to describe a project or a package: its name, what it depends on, where packages |
| reference/manifest | feature | первое вхождение не пояснено | td |  |  | optional, additive content sets with a `default` list; features may depend on features |
| reference/manifest | fingerprint (freshness) | первое вхождение не пояснено | td |  |  | ne registry or for any, tried by `priority` and verified by fingerprint |
| reference/manifest | kind | первое вхождение не пояснено | td |  |  | one of the eight kinds; metadata, not identity |
| reference/manifest | mirror | первое вхождение не пояснено | td |  |  | ackage` and `version` of the documentation this translation mirrors` |
| reference/manifest | package | первое вхождение не пояснено | p |  |  | Every node, whether a consumer project, a publishable package or a workspace root, has a file named `vibe.toml`. The tab… |
| reference/manifest | project | первое вхождение не пояснено | p |  |  | Every node, whether a consumer project, a publishable package or a workspace root, has a file name |
| reference/manifest | provider | первое вхождение не пояснено | td |  |  | abstract abilities any provider may satisfy, `namespace:name@constraint` |
| reference/manifest | registry | первое вхождение не пояснено | td |  |  | the card every registry shows |
| reference/manifest | translation | первое вхождение не пояснено | td |  |  | `package` and `version` of the documentation this translation mirrors |
| reference/manifest | workspace | первое вхождение не пояснено | p |  |  | ode, whether a consumer project, a publishable package or a workspace root, has a file named `vibe.toml`. The tables pre… |
| reference/settings-and-environment | contribution | первое вхождение не пояснено | p |  |  | solved value with its origin, `vibe prefs show-origins` the contribution of each layer, and `vibe prefs set` writes one… |
| reference/settings-and-environment | embedded registry | первое вхождение не пояснено | td |  |  | ignores the embedded registry of a source-built vibe |
| reference/settings-and-environment | fingerprint (freshness) | первое вхождение не пояснено | td |  |  | lifecycle fingerprints, compile traces and the agent relay mailbox; machine state, |
| reference/settings-and-environment | index (of a registry) | первое вхождение не пояснено | td |  |  | the index location of one registry; `none` switches its index off |
| reference/settings-and-environment | lifecycle | первое вхождение не пояснено | td |  |  | lifecycle fingerprints, compile traces and the agent relay mailbox; m |
| reference/settings-and-environment | mirror | первое вхождение не пояснено | td |  |  | machine-wide registries, mirrors and overrides, merged after each project's |
| reference/settings-and-environment | override | первое вхождение не пояснено | td |  |  | machine-wide registries, mirrors and overrides, merged after each project's |
| reference/settings-and-environment | package | первое вхождение не пояснено | td |  |  | the machine store of fetched packages, keyed by identity |
| reference/settings-and-environment | project | первое вхождение не пояснено | td |  |  | e-wide registries, mirrors and overrides, merged after each project's |
| reference/settings-and-environment | registry | первое вхождение не пояснено | td |  |  | moves the registry clone cache |
| reference/settings-and-environment | relay | первое вхождение не пояснено | td |  |  | lifecycle fingerprints, compile traces and the agent relay mailbox; machine state, not committed |
| reference/settings-and-environment | store | первое вхождение не пояснено | td |  |  | the machine store of fetched packages, keyed by identity |
| start/first-project | project | во вводном абзаце (запрещено) | - | - | - | A project is a folder your agent works in. This page creates one, add |
| start/first-project | package | первое вхождение не пояснено | prompt |  |  | project named hello-vibe in the current folder, install the package org.vibevm.world/wal into it from the default regist… |
| start/first-project | project | первое вхождение не пояснено | prompt |  |  | Create a VibeVM project named hello-vibe in the current folder, install the package |
| start/first-project | registry | первое вхождение не пояснено | prompt |  |  | l the package org.vibevm.world/wal into it from the default registry, and show me the reading list the agent gets at ses… |
| start/first-project | skill | первое вхождение не пояснено | needs |  |  | the vibevm skill installed for your agent; network access to github.com/vibe |
| start/first-project | store | первое вхождение не пояснено | needs |  |  | github.com/vibespecs, or the package already in the machine store |
| start/index | project | во вводном абзаце (запрещено) | - | - | - | You have a coding agent and a project. VibeVM gives that agent the right text to read before it s |
| start/index | index (of a registry) | первое вхождение не пояснено | p |  |  | tion file, reads one generated file in full, and follows an index for the rest. It never runs vibe to start. When you la… |
| start/index | package | первое вхождение не пояснено | p |  |  | prompt to your agent, or run the four commands by hand. The package you install is a way of working; the agent reads it… |
| start/install-vibe | store | первое вхождение не пояснено | p |  |  | ript. The script imports the binary into vibe's own managed store under `~/.vibe/opt`, marks it active, and adds one fol… |
| start/what-a-project-contains | project | во вводном абзаце (запрещено) | - | - | - | After the first install, a project holds a handful of files you wrote and a larger set that vi |
| start/what-a-project-contains | fingerprint (freshness) | первое вхождение не пояснено | p |  |  | nstalled, including the ones your packages pulled in, and a fingerprint of each package's content. With it, a fresh clon… |
| start/what-a-project-contains | lock file | первое вхождение не пояснено | p |  |  | a teammate needs to reproduce your setup, together with the lock file beside it. |
| start/what-a-project-contains | managed block | первое вхождение не пояснено | p |  |  | The agent finds the boot files through a short managed block at the end of `CLAUDE.md`, `AGENTS.md` and `GEMINI.md`, bet |
| start/what-a-project-contains | package | первое вхождение не пояснено | p |  |  | the first version for you. It names the project, lists the packages it requires with a version range for each, and lists… |
| start/what-a-project-contains | project | первое вхождение не пояснено | p |  |  | `vibe init` writes the first version for you. It names the project, lists the packages it requires with a version range… |
| start/what-a-project-contains | store | первое вхождение не пояснено | p |  |  | state, ignored by git and safe to delete. The machine-wide store of fetched packages is elsewhere, in your home director… |
| start/what-vibevm-is | package | во вводном абзаце (запрещено) | - | - | - | ing nothing about your project. VibeVM fixes that the way a package manager fixes missing libraries: you name what your… |
| start/what-vibevm-is | project | во вводном абзаце (запрещено) | - | - | - | oding agent starts every session knowing nothing about your project. VibeVM fixes that the way a package manager fixes m… |
| start/what-vibevm-is | coordinate | первое вхождение не пояснено | p |  |  | manifest and the text or tools it delivers. A package has a coordinate made of a group and a name, such as `org.vibevm.w…` |
| start/what-vibevm-is | fact | первое вхождение не пояснено | p |  |  | eeds them. Text costs tokens, and vibe is built around that fact. |
| start/what-vibevm-is | manifest | первое вхождение не пояснено | p |  |  | . A project declares which rule sets it follows, in a small manifest file, and vibe assembles from them the text an agen… |

### 6.2 Пояснено корректно (для контроля метода, не нарушения)

| страница | термин | где | курсив | ссылка | оборот-пояснение | контекст |
|---|---|---|---|---|---|---|
| agent/ask-your-agent | relay | p |  |  | да | t: it composes an instruction and parks it in the project's relay mailbox, `.vibe/agentic/command.md`. The agent then ru… |
| agent/ask-your-agent | skill | p |  |  | да | ith a block you can copy into any agent that has the vibevm skill: it names the thing, states the result, and mentions n… |
| agent/give-your-agent-the-skill | kind | p |  |  | да | Packages can declare skills of their own, for any kind of package: a flow that ships a checklist skill, a language |
| agent/give-your-agent-the-skill | MCP server | prompt |  |  | да | ect in the current folder: install the vibevm skill and the MCP server entry, then show me what was written and for whic… |
| agent/give-your-agent-the-skill | project | prompt |  |  | да | very coding agent installed on this machine, for the VibeVM project in the current folder: install the vibevm skill and… |
| agent/give-your-agent-the-skill | skill | prompt |  |  | да | he VibeVM project in the current folder: install the vibevm skill and the MCP server entry, then show me what was writte… |
| agent/how-agents-read-this-manual | index (of a registry) | td |  |  | да | the index: one line per page, the page's first paragraph |
| architecture/how-vibe-is-built | capability | td |  |  | да | y; workspace discovery, materialisation, the computed boot; capability-relative filesystem mutation |
| architecture/how-vibe-is-built | contribution | p |  |  | да | d recorded in the lock file. *Computed boot*: the packages' contributions projected into the two generated files an agen… |
| architecture/how-vibe-is-built | coordinate | p |  |  | да | Read the product bottom-up. *Identity*: a package is a coordinate plus a content fingerprint, and the kind is metadata.… |
| architecture/how-vibe-is-built | fingerprint (freshness) | p |  |  | да | om-up. *Identity*: a package is a coordinate plus a content fingerprint, and the kind is metadata. *Registry*: ordered p… |
| architecture/how-vibe-is-built | index (of a registry) | p |  |  | да | red package sources with mirrors, overrides and an optional index. *Store*: every fetched package version kept once per… |
| architecture/how-vibe-is-built | kind | p |  |  | да | package is a coordinate plus a content fingerprint, and the kind is metadata. *Registry*: ordered package sources with m… |
| architecture/how-vibe-is-built | lock file | p |  |  | да | pied into the project's dependency tree and recorded in the lock file. *Computed boot*: the packages' contributions proj… |
| architecture/how-vibe-is-built | manifest | td |  |  | да | manifests, the lock file, identities and content hashes; the generate |
| architecture/how-vibe-is-built | mirror | p |  |  | да | kind is metadata. *Registry*: ordered package sources with mirrors, overrides and an optional index. *Store*: every fetc… |
| architecture/how-vibe-is-built | override | p |  |  | да | metadata. *Registry*: ordered package sources with mirrors, overrides and an optional index. *Store*: every fetched pack… |
| architecture/how-vibe-is-built | package | p |  |  | да | Read the product bottom-up. *Identity*: a package is a coordinate plus a content fingerprint, and the kind is |
| architecture/how-vibe-is-built | project | p |  |  | да | hine. *Materialisation*: the resolved graph copied into the project's dependency tree and recorded in the lock file. *Co… |
| architecture/how-vibe-is-built | registry | p | да |  | да | nate plus a content fingerprint, and the kind is metadata. *Registry*: ordered package sources with mirrors, overrides a… |
| architecture/how-vibe-is-built | store | p | да |  | да | age sources with mirrors, overrides and an optional index. *Store*: every fetched package version kept once per machine.… |
| architecture/how-vibe-is-built | traceability map | td |  |  | да | e settings home; the maintainer gates: code generation, the traceability map, engine synchronisation, mirroring, the rel… |
| architecture/how-vibe-is-built | workspace | td |  |  | да | the solver seams and cells; plan and apply; workspace discovery, materialisation, the computed boot; capability-r |
| architecture/traceability | anchor | p |  |  | да | into this project's own namespace must land on an existing anchor. The *suspect rule*: when a spec unit's revision bumps… |
| architecture/traceability | content hash | p |  |  | да | cmap.json`: nodes for every spec unit with its revision and content hash, nodes for every tagged code item, and the edge…` |
| architecture/traceability | project | p |  |  | да | ed with a debt id. The *resolve gate*: every edge into this project's own namespace must land on an existing anchor. The… |
| architecture/traceability | specification | p |  |  | да | `cargo xtask specmap` walks the crates for marks and the specification tree for units, and writes `specmap.json`: nodes… |
| architecture/what-the-lifecycle-epic-delivered | contribution | td |  |  | да | e with its verbs and the clean chain, ordered collection of contributions, the context envelope, durable freshness with… |
| architecture/what-the-lifecycle-epic-delivered | deploy profile | td |  |  | да | le skills, the Agent Plugins directory, client projections, deploy profiles with intents, receipts and recovery, a deter… |
| architecture/what-the-lifecycle-epic-delivered | handler | td |  |  | да | velope, durable freshness with `--force`, script and binary handlers, data presets, `vibe extensions` |
| architecture/what-the-lifecycle-epic-delivered | lifecycle | td |  |  | да | R2, the lifecycle engine |
| architecture/what-the-lifecycle-epic-delivered | phase | td |  |  | да | the strict `[[extension]]` grammar, the nine-phase line with its verbs and the clean chain, ordered collection |
| architecture/what-the-lifecycle-epic-delivered | project | td |  |  | да | the project skill binding, the mechanism and artifact grammar, artifact |
| architecture/what-the-lifecycle-epic-delivered | receipt | td |  |  | да | irectory, client projections, deploy profiles with intents, receipts and recovery, a deterministic zip, platform applica… |
| architecture/what-the-lifecycle-epic-delivered | scrape | p |  |  | да | Beside the route, the scrape operation landed as its own contract: the deterministic rem |
| architecture/what-the-lifecycle-epic-delivered | skill | td |  |  | да | the project skill binding, the mechanism and artifact grammar, artifact recor |
| authoring/ship-tools-and-mcp-servers | fingerprint (freshness) | p |  |  | да | tool beside it, and the artifact never enters the package's fingerprint. |
| authoring/ship-tools-and-mcp-servers | kind | p |  |  | да | A package of kind `mcp` delivers a server an agent talks to: one or more `[[m` |
| authoring/specs-agents-can-cite | anchor | p |  |  | да | th inside `vibevm/vibespecs/` without its extension, and an anchor. The version is a feature, never an obligation: absen… |
| authoring/specs-agents-can-cite | boot lane | p |  |  | да | citation target: cite the source document, not the compiled boot lane. |
| authoring/specs-agents-can-cite | coordinate | p |  |  | да | <name>[@<version>]/<path>/<document>#<anchor>`: the package coordinate, an optional version, the document's path inside…` |
| authoring/specs-agents-can-cite | feature | p |  |  | да | cs/` without its extension, and an anchor. The version is a feature, never an obligation: absent, the address resolves a…` |
| authoring/specs-agents-can-cite | package | p |  |  | да | <group>/<name>[@<version>]/<path>/<document>#<anchor>`: the package coordinate, an optional version, the document's path…` |
| authoring/translate-documentation | anchor | prompt |  |  | да | s-flow-docs: mirror its page tree file for file, keep every anchor and block, replace each example with a reference to t… |
| authoring/translate-documentation | kind | p |  |  | да | rees: the same paths, the same anchors, the same number and kinds of blocks; a difference is an error. The site, seeing… |
| authoring/translate-documentation | manifest | prompt |  |  | да | ample with a reference to the source example, and write the manifest with [translates] and the same [[documents]] subjec… |
| authoring/translate-documentation | mirror | prompt |  |  | да | ect as the Russian translation of org.acme/notes-flow-docs: mirror its page tree file for file, keep every anchor and bl… |
| authoring/translate-documentation | package | prompt |  |  | да | Create the package org.acme/notes-flow-docs-ru in packages/notes-flow-docs-ru |
| authoring/translate-documentation | project | prompt |  |  | да | ocs-ru in packages/notes-flow-docs-ru of the current VibeVM project as the Russian translation of org.acme/notes-flow-do… |
| authoring/translate-documentation | subject | prompt |  |  | да | e the manifest with [translates] and the same [[documents]] subject. Run vibe doc check --translations. |
| authoring/translate-documentation | translation | prompt |  |  | да | s-flow-docs-ru of the current VibeVM project as the Russian translation of org.acme/notes-flow-docs: mirror its page tre… |
| authoring/write-a-feat-or-stack | capability | prompt |  |  | да | es a welcome page with acceptance criteria and requires the capability ui:page-host, and a stack org.acme/static-site th… |
| authoring/write-a-feat-or-stack | family | p |  |  | да | The word `stack` also names a family bundle: a package of kind `stack` with nothing but exact pi |
| authoring/write-a-feat-or-stack | feature | p |  |  | да | A feat describes what a feature does for its user, in terms any stack can implement: purpos |
| authoring/write-a-feat-or-stack | kind | p |  |  | да | The word `stack` also names a family bundle: a package of kind `stack` with nothing but exact pins of a language family'… |
| authoring/write-a-feat-or-stack | lifecycle | p |  |  | да | for are realised with one set of tools, and it may bind the lifecycle's build and test phases to that toolchain. Its man… |
| authoring/write-a-feat-or-stack | manifest | outcome |  |  | да | the feat's manifest requires `ui:page-host`, the stack's manifest provides it, |
| authoring/write-a-feat-or-stack | package | prompt |  |  | да | Create two packages under packages/ in the current VibeVM project: a feat org.a |
| authoring/write-a-feat-or-stack | phase | p |  |  | да | et of tools, and it may bind the lifecycle's build and test phases to that toolchain. Its manifest declares what it prov… |
| authoring/write-a-feat-or-stack | project | prompt |  |  | да | Create two packages under packages/ in the current VibeVM project: a feat org.acme/welcome-page that describes a welcome… |
| authoring/write-a-feat-or-stack | specification | outcome |  |  | да | :page-host`, the stack's manifest provides it, each has its specification documents under `vibevm/vibespecs/`, and `vibe… |
| authoring/write-a-feat-or-stack | version constraint | p |  |  | да | t interface: a namespace, a colon, a name, and optionally a version constraint. A feat requires; a stack provides; the r… |
| authoring/write-a-flow | boot snippet | prompt |  |  | да | change it makes, with the date and what changed. Write the boot snippet, the protocol document and the manifest, then ru… |
| authoring/write-a-flow | manifest | prompt |  |  | да | nged. Write the boot snippet, the protocol document and the manifest, then run vibe check on the package. |
| authoring/write-a-lang-package | boot lane | p |  |  | да | an be run is answered by `vibe tools`, not by the kind: the boot lane says which disciplines are installed, the tools re… |
| authoring/write-a-lang-package | boot snippet | prompt |  |  | да | VibeVM project: a guide on how our team writes SQL, with a boot snippet that names the three rules an agent must always… |
| authoring/write-a-lang-package | family | p |  |  | да | ips tools, a checker, a formatter, a type oracle, becomes a family: the guide package `<family>-lang`, a server package… |
| authoring/write-a-lang-package | package | prompt |  |  | да | Create a lang package org.acme/sql-style in packages/sql-style of the current Vib |
| authoring/write-a-lang-package | project | prompt |  |  | да | .acme/sql-style in packages/sql-style of the current VibeVM project: a guide on how our team writes SQL, with a boot sni… |
| authoring/write-a-lang-package | registry | p |  |  | да | s how to write in something, and the kind tells an agent, a registry and the site which is which before the file is open… |
| authoring/write-documentation | boot snippet | p |  |  | да | slation, the documentation it mirrors. It may not declare a boot snippet, a binary or a server: documentation is read, n… |
| authoring/write-documentation | kind | prompt |  |  | да | ocumenting the package org.acme/notes-flow: a manifest with kind doc, a title and an abstract, a [[documents]] entry for… |
| authoring/write-documentation | manifest | prompt |  |  | да | eVM project, documenting the package org.acme/notes-flow: a manifest with kind doc, a title and an abstract, a [[documen… |
| authoring/write-documentation | mirror | p |  |  | да | skill, images and, for a translation, the documentation it mirrors. It may not declare a boot snippet, a binary or a ser… |
| authoring/write-documentation | package | prompt |  |  | да | Create a documentation package org.acme/notes-flow-docs in packages/notes-flow-docs of the |
| authoring/write-documentation | project | prompt |  |  | да | flow-docs in packages/notes-flow-docs of the current VibeVM project, documenting the package org.acme/notes-flow: a mani… |
| authoring/write-documentation | subject | prompt |  |  | да | doc, a title and an abstract, a [[documents]] entry for the subject, and one page explaining what the flow does with a r… |
| authoring/write-documentation | translation | p |  |  | да | nd an `abstract`; it may declare a skill, images and, for a translation, the documentation it mirrors. It may not declar… |
| diagnostics/errors | managed block | p |  |  | да | ailure, 3 for a conflict-shaped refusal such as a malformed managed block, 5 when you declined a plan. |
| diagnostics/errors | mirror | td |  |  | да | tes the lock file remembered: a re-tagged version, a broken mirror or an override |
| diagnostics/errors | override | td |  |  | да | file remembered: a re-tagged version, a broken mirror or an override |
| faq/index | boot snippet | p |  |  | да | e wrong thing early: a documentation package cannot carry a boot snippet, a server package must pin what it serves, a fe… |
| faq/index | coordinate | p |  |  | да | raint with `vibe update`; add an `[[override]]` for the one coordinate with a `reason`; fork and use a git source when t… |
| faq/index | fingerprint (freshness) | p |  |  | да | No. A fingerprint mismatch means the bytes served are not the bytes the lock |
| faq/index | git source | p |  |  | да | e]]` for the one coordinate with a `reason`; fork and use a git source when the dependency's own constraint must change.…` |
| faq/index | lock file | p |  |  | да | stall writes into your repository: the dependency tree, the lock file, the boot files, the managed block in the instruct… |
| faq/index | managed block | p |  |  | да | ry: the dependency tree, the lock file, the boot files, the managed block in the instruction files. The plan shows all o… |
| faq/index | override | p |  |  | да | mbered: a force-pushed tag, a broken mirror or a deliberate override. Find out which, then uninstall and reinstall to re… |
| faq/index | project | p |  |  | да | a range and the lock file holds the pin, which is what most projects want. |
| faq/index | registry | p |  |  | да | holds the packages: a stored version is usable even when no registry lists it any more, and `vibe reinstall` rebuilds th… |
| faq/index | store | p |  |  | да | Not while the machine store holds the packages: a stored version is usable even when no |
| howto/install-a-package | boot snippet | outcome |  |  | да | ee sits under `vibevm/vibedeps/`, and `vibe tree` shows its boot snippet in the reading list |
| howto/install-a-package | fingerprint (freshness) | outcome |  |  | да | requirements, `vibe.lock` pins one version with its content fingerprint, the package's tree sits under `vibevm/vibedeps/…` |
| howto/install-a-package | kind | p |  |  | да | A kind prefix, as in `flow:org.vibevm.world/wal`, is optional and |
| howto/publish-a-package | fingerprint (freshness) | p |  |  | да | already exists is refused, and consumers verify the content fingerprint on every install, so a re-pushed tag is caught r… |
| howto/publish-a-package | package | prompt |  |  | да | Publish the package in the folder packages/notes to the first registry of this |
| howto/publish-a-package | project | prompt |  |  | да | in the folder packages/notes to the first registry of this project, using the publish token already in my environment, t… |
| howto/publish-a-package | registry | prompt |  |  | да | blish the package in the folder packages/notes to the first registry of this project, using the publish token already in… |
| howto/read-documentation-locally | package | prompt |  |  | да | Fetch the VibeVM manual, the package org.vibevm.core/vibevm-docs, into the machine store, open t |
| howto/read-documentation-locally | store | prompt |  |  | да | , the package org.vibevm.core/vibevm-docs, into the machine store, open the local documentation reader, and tell me the… |
| howto/remove-a-package | hook | p |  |  | да | e does not undo what its install script did, if it had one: hook effects are not tracked, by design. |
| howto/remove-a-package | lock file | p |  |  | да | ee and the generated boot files and keeps the manifest, the lock file, everything you wrote and the machine store. `vibe…` |
| howto/remove-a-package | manifest | p |  |  | да | l`. vibe shows what will leave: the requirement line in the manifest, the lock entry, the package's folder in the depend…` |
| howto/remove-a-package | store | p |  |  | да | nifest, the lock file, everything you wrote and the machine store. `vibe clean install` chains the two steps. |
| howto/set-up-a-workspace | coordinate | p |  |  | да | that is never published, and a `[workspace]` table makes it coordinate members. A node cannot be both a package and a pr… |
| howto/set-up-a-workspace | package | prompt |  |  | да | ject in the current folder into a workspace with two member packages under packages/: org.acme/notes-flow and org.acme/n… |
| howto/set-up-a-workspace | project | prompt |  |  | да | Turn the VibeVM project in the current folder into a workspace with two member pack |
| howto/set-up-a-workspace | workspace | prompt |  |  | да | Turn the VibeVM project in the current folder into a workspace with two member packages under packages/: org.acme/notes-… |
| howto/update-packages | lock file | prompt |  |  | да | then update all of them, and summarise what changed in the lock file. |
| howto/update-packages | package | prompt |  |  | да | In the VibeVM project in the current folder, show me which packages have newer versions, then update all of them, and su… |
| howto/update-packages | project | prompt |  |  | да | In the VibeVM project in the current folder, show me which packages have newer ve |
| howto/update-packages | store | p |  |  | да | age that is not being updated, fetches what is new into the store, shows the plan, and on confirmation replaces the pack… |
| howto/work-offline | fingerprint (freshness) | p |  |  | да | `vibe cache check` to verify every stored entry against its fingerprint. Finally it runs `vibe install --offline`: resol… |
| howto/work-offline | package | prompt |  |  | да | ing the VibeVM project in the current folder needs plus the package org.vibevm.world/multi-user-planning, then verify th… |
| howto/work-offline | project | prompt |  |  | да | offline, fetch into the machine store everything the VibeVM project in the current folder needs plus the package org.vib… |
| howto/work-offline | store | prompt |  |  | да | Before I go offline, fetch into the machine store everything the VibeVM project in the current folder needs p |
| lifecycle/build-package-deploy | project | prompt |  |  | да | For the VibeVM project in the current folder, show me the deploy plan for the prof |
| lifecycle/build-package-deploy | provider | p |  |  | да | A deploy profile names its targets in order and the provider that applies each: a folder on this machine, an agent's pro |
| lifecycle/build-package-deploy | specification | p |  |  | да | n agent's project or user configuration, and the genres the specification lists. An installed package may replace a buil… |
| lifecycle/extensions-and-providers | contribution | p | да |  |  | ere a package may transform the text an agent will read. A *contribution* binds a handler to a point and is declared in… |
| lifecycle/extensions-and-providers | extension point | p | да |  |  | The lifecycle exposes named *extension points*, strings of the form `family:name`. The `phase:` family is |
| lifecycle/extensions-and-providers | override | p |  |  | да | s an id, which is also the key a project uses to disable or override it; the same handler may be bound several times und… |
| lifecycle/extensions-and-providers | provider | td |  |  | да | work handed to the hosting agent, or to a configured model provider when a person runs vibe at a terminal |
| lifecycle/extensions-and-providers | registry | p |  |  | да | es and servers the installed packages brought, which is the registry of what a contribution of kind `binary` may name. |
| lifecycle/phases | handler | p |  |  | да | it will run: their id, the point they bind to, the kind of handler, and where they came from. Nothing in the lifecycle r… |
| lifecycle/phases | kind | p |  |  | да | ibutions it will run: their id, the point they bind to, the kind of handler, and where they came from. Nothing in the li… |
| lifecycle/phases | manifest | td |  |  | да | the cheap preflight: the manifest parses, the declared extensions and profiles are well forme |
| lifecycle/phases | package | td |  |  | да | the package install described elsewhere in this manual: resolve, fetch, |
| lifecycle/scrape | lock file | p |  |  | да | he copy is a plain project of its language: no manifest, no lock file, no dependency tree, no boot files, no managed blo… |
| lifecycle/scrape | manifest | p |  |  | да | untouched. The copy is a plain project of its language: no manifest, no lock file, no dependency tree, no boot files, no… |
| lifecycle/scrape | project | prompt |  |  | да | Show me the scrape plan for the VibeVM project in the current folder, then export a scraped copy of it to |
| lifecycle/scrape | scrape | prompt |  |  | да | Show me the scrape plan for the VibeVM project in the current folder, then exp |
| lifecycle/scrape | specification | p |  |  | да | es, no managed block, no source annotations that pointed at specifications. |
| model/boot-lane | boot snippet | p |  |  | да | Every package may contribute one boot snippet: a short text meant to be read at every session start. The |
| model/boot-lane | index (of a registry) | p |  |  | да | s linked: compiled into the priority lane, or listed in the index and read on demand. The project's choice wins over the… |
| model/boot-lane | link type | p |  |  | да | at declares a condition is always a dynamic entry, whatever link type the project asked for: a condition cannot be evalu… |
| model/boot-lane | project | p |  |  | да | ackage declares it in its manifest with a category, and the project's manifest decides how the snippet is linked: compil… |
| model/lock-and-store | fingerprint (freshness) | p |  |  | да | ied by four things: its group, its name, its version, and a fingerprint of every file it contains. The address it was fe… |
| model/lock-and-store | lock file | p |  |  | да | or, a moved repository or a vendored copy never changes the lock file: as long as the bytes are the same, the package is… |
| model/lock-and-store | mirror | p |  |  | да | ed from is written down for information only. That is why a mirror, a moved repository or a vendored copy never changes… |
| model/lock-and-store | package | p |  |  | да | A package version is identified by four things: its group, its name, |
| model/lock-and-store | registry | p |  |  | да | nd transitive, with its exact version, its fingerprint, the registry it came from and how it was resolved. vibe writes i… |
| model/packages-and-kinds | boot snippet | td |  |  | да | commit rules, session notes, review conventions; usually a boot snippet the agent reads every session |
| model/packages-and-kinds | companion | p | да |  | да | Documentation is the exception. A package's manual is its *companion*, named with the suffix `-docs` in the same group,… |
| model/packages-and-kinds | coordinate | p | да |  | да | A package is named by a *coordinate*: a group, a slash, and a name, as in `org.vibevm.world/wal` |
| model/registries | fingerprint (freshness) | p |  |  | да | y published version, the manifest's summary and the content fingerprint. `vibe search` reads the index; a fresh install… |
| model/registries | index (of a registry) | p | да |  | да | s impossible without an account. So a registry may keep an *index*: a separate repository beside the packages that recor… |
| model/registries | lock file | p |  |  | да | ersion is refused, not trusted. Mirrors never appear in the lock file: the canonical address is what gets recorded, so s… |
| model/registries | mirror | p | да |  |  | A *mirror* is another address for the same registry, tried first for |
| model/registries | override | p | да |  |  | An *override* replaces one package with a copy from elsewhere, for a hot |
| model/two-trees | lock file | p |  |  | да | e things in a project are derived from the manifest and the lock file, and vibe rebuilds them on demand: the dependency… |
| model/two-trees | managed block | p |  |  | да | and: the dependency tree, the generated boot files, and the managed block in the agent instruction files. `vibe reinstal…` |
| model/two-trees | manifest | p |  |  | да | Three things in a project are derived from the manifest and the lock file, and vibe rebuilds them on demand: the de |
| model/two-trees | project | p |  |  | да | Three things in a project are derived from the manifest and the lock file, and vibe r |
| model/versions | coordinate | p |  |  | да | .x from 1.0 up, `=1.2.0` means exactly that one, and a bare coordinate means the newest stable release. The resolver pic… |
| model/versions | git source | p |  |  | да | A branch used as a git source is the one exception to pinning by number: its lock entry r |
| model/versions | manifest | p |  |  | да | e first changes when a package breaks compatibility. In the manifest you name a constraint, not a version: `^1.0` means… |
| model/versions | package | p |  |  | да | Package versions follow semantic versioning: three numbers, where t |
| reference/lock-file | index (of a registry) | p |  |  | да | The lock file records no registry index and no mirror: reproducing it needs only the coordinates, t |
| reference/lock-file | lock file | p |  |  | да | An unchanged manifest against an unchanged lock file makes `vibe install` skip the resolver entirely: the lock i |
| reference/machine-formats | index (of a registry) | p |  |  | да | The documents of the index server, the scrape contract and plan, the compiler trace an |
| reference/machine-formats | receipt | td |  |  | да | the deploy intent, the receipts, the inverse plan and the checkpoints |
| reference/machine-formats | scrape | p |  |  | да | The documents of the index server, the scrape contract and plan, the compiler trace and the native ABI ha |
| reference/manifest | contribution | td |  |  | да | `id`, `point`, `handler`, optional selector and config: a contribution to the lifecycle |
| reference/manifest | coordinate | td |  |  | да | one key per required coordinate, the value a constraint string or an inline table with `ver` |
| reference/manifest | lifecycle | td |  |  | да | ndler`, optional selector and config: a contribution to the lifecycle` |
| reference/manifest | manifest | p |  |  | да | Unknown keys are rejected, not ignored: a manifest written for a newer vibe than the one reading it fails to p |
| reference/manifest | skill | td |  |  | да | `name`, `path`, `description`, optional target `agents`: a skill an agent may install |
| reference/manifest | subject | td |  |  | да | `package` and a `version` constraint: a subject this documentation describes; required, repeatable |
| start/first-project | agent session | p |  |  | да | Open a new agent session in `hello-vibe` and ask it what rules it follows: it will n |
| start/first-project | fingerprint (freshness) | p |  |  | да | -vibe/vibe.lock`: it pins the exact version and the content fingerprint. Under `hello-vibe/vibevm/vibedeps/` sits the pa…` |
| start/first-project | lock file | p |  |  | да | o-vibe`, which creates the folder with a manifest, an empty lock file, the two boot files that are yours to edit, and th…` |
| start/first-project | manifest | p |  |  | да | uns `vibe init hello-vibe`, which creates the folder with a manifest, an empty lock file, the two boot files that are yo… |
| start/index | boot lane | p |  | да |  | Read [The boot lane](../model/boot-lane.xml). The agent opens the instruction f |
| start/index | kind | p |  |  | да | the machinery, the pages under *Model* explain packages and kinds, registries, the lock file and versions. To let your a… |
| start/index | lock file | p |  |  | да | s under *Model* explain packages and kinds, registries, the lock file and versions. To let your agent do the work, *Give… |
| start/index | project | p |  | да |  | Follow [Create your first project](first-project.xml). Give the prompt to your agent, or run |
| start/index | skill | p | да |  |  | To let your agent do the work, *Give your agent the vibevm skill* comes next. And whenever a command refuses, *From an e… |
| start/install-vibe | package | p |  |  | да | their launchers under `opt/`, the machine store of fetched packages under `cache/`, registry clones under `registries/`,… |
| start/install-vibe | registry | p |  |  | да | pt/`, the machine store of fetched packages under `cache/`, registry clones under `registries/`, and your settings files…` |
| start/what-a-project-contains | manifest | p | да |  |  | `vibe.toml` is the *manifest*. You write it, or `vibe init` writes the first version for |
| start/what-a-project-contains | override | p |  |  | да | ds the project's foundations, `90-user` holds your personal overrides, and vibe never touches either. Two files there ar… |
| start/what-a-project-contains | registry | p |  |  | да | ources here, and vibe treats the directory as a small local registry. Most projects do not have it. |
| start/what-a-project-contains | specification | p |  |  | да | `vibevm/vibespecs/` is *your* tree: the specifications and rules this project itself writes, in Markdown or in the |
| start/what-vibevm-is | boot lane | p | да |  | да | gs: a lock file that records exactly what arrived, and the *boot lane*, the ordered reading list the agent follows at th… |
| start/what-vibevm-is | feature | p |  |  | да | what to check before pushing, which words mean what, how a feature is described before it is built. People learn these r… |
| start/what-vibevm-is | lock file | p |  |  | да | ine, copies them into the project, and writes two things: a lock file that records exactly what arrived, and the *boot l… |
| start/what-vibevm-is | package | p | да |  | да | The unit vibe installs is a *package*: a folder with a short manifest and the text or tools it d |
| start/what-vibevm-is | project | p |  |  | да | Every serious project has rules that never make it into the code: how to commit, |
| start/what-vibevm-is | registry | p | да |  |  | `org.vibevm.world/wal`, and a version. Packages live in a *registry*, which by default is a public organisation on GitHu… |
| start/what-vibevm-is | store | p |  |  | да | `, vibe resolves versions, fetches the packages once into a store on your machine, copies them into the project, and wri…` |

## 7. Ссылки между страницами `[текст](путь.xml)`

Всего ссылок: 10. Битых: 0.

| страница | текст ссылки | путь | статус |
|---|---|---|---|
| faq/index | Two trees | ../model/two-trees.xml | найден |
| faq/index | Ask your agent to do the work | ../agent/ask-your-agent.xml | найден |
| faq/index | Work offline | ../howto/work-offline.xml | найден |
| faq/index | Remove VibeVM from a project | ../lifecycle/scrape.xml | найден |
| faq/index | Read documentation locally | ../howto/read-documentation-locally.xml | найден |
| start/index | What VibeVM is | what-vibevm-is.xml | найден |
| start/index | Install vibe | install-vibe.xml | найден |
| start/index | Create your first project | first-project.xml | найден |
| start/index | What a project contains | what-a-project-contains.xml | найден |
| start/index | The boot lane | ../model/boot-lane.xml | найден |

## 8. Идентификаторы `example`/`prompt`

Не найдено ни одной проблемы на 44 страницах: все `example id` (59 шт.) и все `prompt id` (19 шт.) уникальны в пределах своей страницы и не пересекаются друг с другом; у каждого `example` есть `run` (59/59); у каждого `prompt` есть хотя бы один `assert` (19/19), атрибут `assert="none"` в корпусе не встречается.

## Расхождения с решениями

- REVIEW: словарь запрещённых слов (P.6) банит `capabilities` целиком, но в этом продукте «capability» — точный технический термин (глоссарий; поля манифеста `[requires]`/`[provides] capabilities`), а не рыхлая маркетинговая лексика, на которую рассчитан общий список — против замысла P.6 (устранение маркетинговой рыхлости), не переписываю список, только фиксирую факт (см. §2).
- Иных расхождений с решениями (D-NN) не обнаружено.

## Открытые вопросы / дефекты пакета

- Дефект пакета: реальный корпус содержит цитаты вида `spec://org.vibevm.core/vibevm-docs/<page>[#<anchor>]` (самоадресация страниц документации друг на друга) — 2 конкретных случая (`agent/how-agents-read-this-manual` -> `model/boot-lane#p7`; `howto/read-documentation-locally` -> `start/what-vibevm-is`) плюс 1 шаблонный (`vibevm-docs/<page>#<anchor>`). Ни один из 3 данных в пакете паттернов резолюции его не покрывает. Оставлено нерезолвленным (`out-of-scope`) по консервативному допущению — правило для этого адреса пакет не даёт.
- 4 вхождения `spec://…` (эллипсис, U+2026) и 2 вхождения `spec://org.acme/...` — это иллюстративные примеры синтаксиса в страницах, которые сами объясняют схему адресации (`authoring/specs-agents-can-cite`, `authoring/write-a-flow`, `agent/how-agents-read-this-manual`, `architecture/traceability`, `authoring/write-documentation`), не настоящие цитаты; см. §1.2. Разметки, отличающей «пример синтаксиса» от «настоящей цитаты», в источнике нет — вывод по контексту вручную (автор страницы, посвящённой самой схеме адресации, использует свою же схему как пример).
- §6 (термины): метод занижает число нарушений `first-use-not-explained` (см. ограничение в §6) — пояснительный оборот ищется по всей фразе, а не рядом со словом. Понадобится точнее для A2.25.
- §3 (длина фраз): разбиение на фразы — эвристика (маскирует точки только внутри `` `код` `` и `(путь-ссылки)`); сокращения вида «т.е.», версии вне кода (не встречены де-факто, но не проверено доказательно) не обрабатывались бы отдельно.
- §1.1: 2 из 3 ненайденных якорей (`PROP-054#WHY-C-ABI` при секции `<why-c-abi>` в нижнем регистре; `PROP-054#WASM` при факте `<WASM-DEFERRED>`) выглядят как настоящие сломанные цитаты (регистр / переименование), не искажение метода — воспроизводимо, см. §1.1.

