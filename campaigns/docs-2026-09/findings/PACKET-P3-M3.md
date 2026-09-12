# PACKET-P3-M3 — обязательства: основания, загрузка, воркспейс, резолвер, диалект

##subagent-quiet-clause

Общая часть — `campaigns/docs-2026-09/findings/PACKET-P3-M-COMMON.md`;
прочитай её первой и целиком. Твой номер — `M3`.

Твои документы (только они):

- `vibevm/vibespecs/common/PROP-000.xml`
- `vibevm/vibespecs/common/PROP-006-operating-modes.xml`
- `vibevm/vibespecs/common/PROP-052-directory-layout.xml`
- `vibevm/vibespecs/common/PROP-028-package-families.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-038-hybrid-boot-linking.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-034-transitive-links-boot-graph.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml`
- `vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml`
- `vibevm/vibespecs/modules/vibe-resolver/PROP-017-resolvo-resolver.xml`
- `vibevm/vibespecs/common/PROP-045-xml-spec-sources.xml`
- `vibevm/vibespecs/common/PROP-044-change-native-formats.xml`
- `vibevm/vibespecs/common/PROP-049-snippet-genre.xml`

Подсказка по аудиториям: два дерева, что коммитить, boot-лейн как
результат, воркспейс, версии и разрешение зависимостей глазами оператора —
`user`; диалект спек, адреса, семейства, снипеты, раскладка пакета —
`author`; закон boot-лейна и чтение спек — `agent` (редко); правила
коммитов и алгоритмы резолвера — `dev` или без пометки.
