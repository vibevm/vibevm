# PACKET-P3-M2 — обязательства: lifecycle, код в пакетах, менеджер версий, установка

##subagent-quiet-clause

Общая часть — `campaigns/docs-2026-09/findings/PACKET-P3-M-COMMON.md`;
прочитай её первой и целиком. Твой номер — `M2`.

Твои документы (только они):

- `vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml`
- `vibevm/vibespecs/common/PROP-056-scraped-project-export.xml`
- `vibevm/vibespecs/common/PROP-024-code-bearing-packages.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-053-clean-verb.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-020-install-hooks.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-022-materialization-modes.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml`
- `vibevm/vibespecs/modules/vibe-workspace/PROP-012-managed-redirect-block.xml`
- `vibevm/vibespecs/common/PROP-019-version-manager.xml`

Подсказка по аудиториям: фазы lifecycle, `build`/`package`/`deploy`/`scrape`
как команды, установка, обновление, `clean`, `vibe self` — `user`;
грамматика манифестов сборки и развёртывания, точки расширения, провайдеры,
бинарники и хуки в пакете — `author`; внутреннее устройство инкрементальной
установки и редирект-блока — без пометки.
