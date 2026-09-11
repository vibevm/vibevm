# WORKER-REPORT-PP-C3 — решения по инвентарю legacy-документации

Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C3.md`. Скрипт: `campaigns/docs-2026-09/findings/PP-C3-dispose.py`.

## Счётчики по решениям

| решение | количество |
| --- | --- |
| перенесён | 957 |
| перенесён (справочник) | 93 |
| снят: устарело | 80 |
| нужен автор | 54 |
| **всего** | **1184** |

## Переоценки `неизвестно`

Правило: `target/debug/vibe.exe <проба> --help`, только `--help`. 6 строк с меткой `неизвестно` относятся к типу `cmd` и прошли переоценку; остальные `неизвестно` (тип `path`) идут по общему порядку без переоценки.

| строка | файл | утверждение | проба | exit code | результат |
| --- | --- | --- | --- | --- | --- |
| 25 | `docs/ALPHA-NOTES.md` | `vibe <command> --help` | `<command>` | 2 | снят: устарело (не существует) |
| 40 | `docs/architecture.md` | `vibe <command> --help` | `<command>` | 2 | снят: устарело (не существует) |
| 265 | `docs/commands/mcp-install.md` | `vibe <subcmd> --help` | `<subcmd>` | 2 | снят: устарело (не существует) |
| 925 | `docs/guides/agent-mcp-quickstart-opencode.md` | `vibe …` | `…` | 2 | снят: устарело (не существует) |
| 979 | `docs/lockfile-format.md` | `vibe <version>` | `<version>` | 2 | снят: устарело (не существует) |
| 1010 | `docs/README.md` | `vibe <command> --help` | `<command>` | 2 | снят: устарело (не существует) |

## Неоднозначный выбор страницы (несколько кандидатов)

Строка попадает сюда, только когда несколько РАЗНЫХ страниц делят лучший (выигравший) уровень предпочтения — `model`/`howto`/`reference` = 0, прочие разделы = 1, `glossary`/`faq` = 2 — то есть уровень сам не решил выбор и потребовался вторичный тай-брейк (не `edge-cases`, затем алфавит по странице). Страницы, совпавшие только на худшем уровне, здесь не перечисляются — уровень уже их отклонил.

Выбор — чистая функция (`тип`, `утверждение`): одно и то же утверждение даёт одно и то же неоднозначное множество страниц независимо от того, в каком файле `docs/…` оно встретилось. Ниже — уникальные ситуации, с числом затронутых строк инвентаря и их номерами.

215 уникальных неоднозначных (тип, утверждение) из 645 затронутых строк.

| тип | утверждение | выбрано | отклонённые кандидаты того же уровня | строк | номера строк |
| --- | --- | --- | --- | --- | --- |
| path | `vibe.toml` | reference/manifest#root | howto/install-a-package#root; howto/publish-a-package#root; howto/remove-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; howto/use-a-private-registry#root; howto/work-offline#root; model/packages-and-kinds#a-package | 39 | 35, 46, 65, 86, 151, 175, 232, 276, … |
| flag | `--json` | reference/commands#global | reference/machine-formats#root | 35 | 119, 131, 162, 202, 222, 250, 291, 315, … |
| cmd | `vibe install` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 34 | 13, 34, 48, 67, 88, 124, 139, 171, … |
| flag | `--quiet` | reference/commands#global | howto/install-a-package#root; howto/read-documentation-locally#root; howto/remove-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; howto/use-a-private-registry#root | 31 | 21, 120, 132, 163, 203, 223, 251, 292, … |
| flag | `--path` | howto/update-packages#by-hand | howto/install-a-package#by-hand; howto/publish-a-package#by-hand; howto/remove-a-package#by-hand; howto/use-a-private-registry#by-hand; howto/work-offline#by-hand; model/boot-lane#root; model/packages-and-kinds#root; model/registries#root; model/two-trees#root; model/versions#root; reference/machine-formats#root | 30 | 12, 114, 127, 148, 177, 218, 234, 275, … |
| path | `vibe.lock` | reference/lock-file#root | howto/install-a-package#root; howto/remove-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; model/lock-and-store#the-lock-file | 20 | 14, 32, 212, 281, 321, 581, 618, 664, … |
| flag | `--unattended` | reference/commands#global | reference/machine-formats#the-envelope; reference/settings-and-environment#variables | 17 | 122, 134, 165, 205, 225, 249, 314, 343, … |
| section | `[[override]]` | reference/manifest#sources | reference/lock-file#package-entries | 16 | 388, 406, 558, 578, 591, 630, 644, 674, … |
| flag | `--invoked-by` | reference/commands#global | reference/machine-formats#the-envelope; reference/settings-and-environment#variables | 15 | 121, 133, 164, 204, 224, 253, 294, 365, … |
| flag | `--offline` | howto/work-offline#root | model/lock-and-store#offline; reference/commands#global; reference/settings-and-environment#variables | 14 | 33, 123, 130, 166, 187, 221, 367, 634, … |
| cmd | `vibe update` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 13 | 36, 111, 213, 282, 662, 759, 813, 837, … |
| path | `vibedeps/` | model/two-trees#the-rule | howto/install-a-package#root; howto/remove-a-package#root | 13 | 31, 47, 63, 87, 104, 210, 663, 887, … |
| cmd | `vibe registry publish` | howto/publish-a-package#root | reference/machine-formats#the-documents | 12 | 62, 79, 97, 386, 414, 422, 457, 506, … |
| flag | `--assume-yes` | howto/remove-a-package#by-hand | howto/install-a-package#by-hand; howto/set-up-a-workspace#by-hand; howto/update-packages#by-hand; howto/work-offline#root | 11 | 178, 248, 313, 342, 600, 667, 748, 765, … |
| cmd | `vibe init` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 11 | 140, 264, 400, 557, 576, 676, 786, 904, … |
| cmd | `vibe check` | howto/remove-a-package#root | howto/install-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; model/packages-and-kinds#a-package | 10 | 60, 125, 172, 230, 727, 744, 779, 991, … |
| section | `[package]` | reference/lock-file#package-entries | howto/set-up-a-workspace#root; reference/manifest#one-file | 9 | 45, 64, 85, 423, 798, 829, 880, 1068, … |
| cmd | `vibe list` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 8 | 216, 725, 742, 757, 922, 963, 1017, 1119 |
| cmd | `vibe mcp install` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 8 | 231, 272, 286, 304, 331, 358, 937, 1050 |
| cmd | `vibe install <pkgref>` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 7 | 508, 888, 926, 982, 1058, 1113, 1162 |
| cmd | `vibe registry sync` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 7 | 402, 421, 543, 579, 637, 653, 966 |
| cmd | `vibe cache` | howto/update-packages#recovery | howto/publish-a-package#edge-cases; howto/read-documentation-locally#root; howto/remove-a-package#edge-cases; howto/work-offline#root; model/lock-and-store#root | 6 | 98, 215, 660, 758, 778, 1020 |
| cmd | `vibe registry list` | howto/use-a-private-registry#root | model/registries#root | 6 | 401, 403, 555, 577, 614, 651 |
| cmd | `vibe uninstall` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 6 | 37, 214, 745, 964, 1016, 1118 |
| path | `~/.vibe/` | reference/settings-and-environment#the-folder | howto/publish-a-package#root; howto/read-documentation-locally#the-shell; howto/use-a-private-registry#machine-wide; howto/work-offline#switching-it-on; model/lock-and-store#the-store; model/registries#what-a-registry-is | 6 | 39, 432, 494, 529, 804, 1089 |
| cmd | `vibe mcp serve` | agent/ask-your-agent#two-transports | agent/give-your-agent-the-skill#what-happens | 5 | 269, 271, 302, 357, 906 |
| cmd | `vibe show` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 5 | 137, 228, 710, 743, 1022 |
| cmd | `vibe tree` | model/boot-lane#root | howto/install-a-package#root; howto/remove-a-package#root; reference/machine-formats#the-documents | 5 | 138, 229, 726, 728, 1023 |
| path | `vibevm/SKILL.md` | agent/give-your-agent-the-skill#what-happens | authoring/write-a-flow#layout | 5 | 261, 296, 318, 352, 918 |
| flag | `--exact` | howto/install-a-package#constraints | howto/update-packages#the-constraint | 4 | 179, 767, 849, 1169 |
| flag | `--version` | start/index#step-2 | start/install-vibe#root; start/what-vibevm-is#root | 4 | 156, 782, 911, 1146 |
| cmd | `vibe cache clean` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 4 | 19, 38, 105, 755 |
| cmd | `vibe registry redirect` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 4 | 467, 493, 507, 1088 |
| cmd | `vibe registry redirect-sync` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 4 | 446, 498, 536, 1100 |
| cmd | `vibe reinstall` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 4 | 661, 958, 965, 1129 |
| flag | `--auth-required` | howto/use-a-private-registry#edge-cases | howto/install-a-package#edge-cases | 3 | 186, 768, 1041 |
| path | `config.toml` | howto/work-offline#switching-it-on | reference/settings-and-environment#the-folder | 3 | 238, 295, 351 |
| cmd | `vibe install flow:wal` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 3 | 905, 1138, 1167 |
| cmd | `vibe list --overrides` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 3 | 823, 861, 996 |
| cmd | `vibe registry` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 3 | 640, 708, 1021 |
| cmd | `vibe registry redirect <pkgref> --to <url>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 3 | 460, 505, 510 |
| cmd | `vibe registry redirect-sync <pkgref>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 3 | 449, 492, 527 |
| cmd | `vibe registry redirect-update` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 3 | 468, 531, 1101 |
| cmd | `vibe search` | model/packages-and-kinds#the-kinds | model/registries#the-index | 3 | 658, 685, 1018 |
| path | `~/.vibe/cache/` | model/lock-and-store#the-store | reference/settings-and-environment#the-folder | 3 | 30, 99, 211 |
| flag | `--package` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 2 | 17, 107 |
| section | `[workspace]` | howto/set-up-a-workspace#root | reference/manifest#one-file | 2 | 826, 883 |
| cmd | `vibe cache check` | howto/update-packages#recovery | howto/work-offline#root; model/lock-and-store#the-store | 2 | 15, 109 |
| cmd | `vibe cache clean --all` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 2 | 18, 118 |
| cmd | `vibe list --json` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 2 | 1002, 1080 |
| cmd | `vibe mcp` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#root | 2 | 355, 1024 |
| cmd | `vibe mcp install --auto --scope user` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 2 | 254, 930 |
| cmd | `vibe registry redirect flow:internal-helper \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 2 | 533, 1090 |
| cmd | `vibe registry redirect flow:internal-secret \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 2 | 535, 1096 |
| cmd | `vibe registry redirect flow:legacy-pinned \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 2 | 534, 1093 |
| cmd | `vibe registry redirect-sync flow:internal-helper` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 2 | 465, 1104 |
| cmd | `vibe registry redirect-update flow:internal-helper \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 2 | 501, 1102 |
| cmd | `vibe reinstall --force` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 2 | 959, 1131 |
| cmd | `vibe show <pkgref>` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 2 | 526, 1079 |
| cmd | `vibe show subskills` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 2 | 284, 715 |
| cmd | `vibe uninstall <pkgref>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 2 | 863, 1128 |
| path | `~/.vibe/registries/` | reference/settings-and-environment#the-folder | model/lock-and-store#edge-cases | 2 | 1044, 1145 |
| flag | `--plain` | howto/remove-a-package#by-hand | howto/install-a-package#by-hand; model/boot-lane#root | 1 | 731 |
| section | `[project]` | reference/manifest#one-file | howto/set-up-a-workspace#one-manifest | 1 | 882 |
| path | `crates/vibe-core/src/manifest` | reference/manifest#root | howto/install-a-package#constraints; howto/publish-a-package#by-hand; howto/remove-a-package#what-happens; howto/set-up-a-workspace#what-happens; howto/update-packages#the-constraint; howto/use-a-private-registry#what-happens; model/boot-lane#the-order; model/packages-and-kinds#the-kinds; model/registries#what-a-registry-is; model/two-trees#what-regenerates; model/versions#asking; reference/lock-file#where; reference/machine-formats#the-rule | 1 | 885 |
| path | `crates/vibe-core/src/manifest/` | reference/manifest#root | howto/install-a-package#constraints; howto/publish-a-package#by-hand; howto/remove-a-package#what-happens; howto/set-up-a-workspace#what-happens; howto/update-packages#the-constraint; howto/use-a-private-registry#what-happens; model/boot-lane#the-order; model/packages-and-kinds#the-kinds; model/registries#what-a-registry-is; model/two-trees#what-regenerates; model/versions#asking; reference/lock-file#where; reference/machine-formats#the-rule | 1 | 884 |
| cmd | `vibe cache [OPTIONS] <COMMAND>` | howto/update-packages#recovery | howto/publish-a-package#edge-cases; howto/read-documentation-locally#root; howto/remove-a-package#edge-cases; howto/work-offline#root; model/lock-and-store#root | 1 | 100 |
| cmd | `vibe cache add <PACKAGES>...` | howto/read-documentation-locally#what-happens | howto/publish-a-package#edge-cases; howto/work-offline#what-happens; model/lock-and-store#the-store | 1 | 103 |
| cmd | `vibe cache add org.vibevm.world/wal --path ./my-project` | howto/read-documentation-locally#what-happens | howto/publish-a-package#edge-cases; howto/work-offline#what-happens; model/lock-and-store#the-store | 1 | 113 |
| cmd | `vibe cache check --repair` | howto/update-packages#recovery | howto/work-offline#root; model/lock-and-store#the-store | 1 | 110 |
| cmd | `vibe cache check --repair --path ./my-project` | howto/update-packages#recovery | howto/work-offline#root; model/lock-and-store#the-store | 1 | 115 |
| cmd | `vibe cache clean --older-than 90` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 1 | 116 |
| cmd | `vibe cache clean --package org.vibevm.world/wal` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 1 | 16 |
| cmd | `vibe cache clean --package org.vibevm.world/wal@1.0.0` | howto/update-packages#recovery | howto/remove-a-package#edge-cases | 1 | 117 |
| cmd | `vibe cache list` | howto/read-documentation-locally#root | howto/work-offline#root; model/lock-and-store#the-store | 1 | 102 |
| cmd | `vibe check --json` | howto/remove-a-package#root | howto/install-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; model/packages-and-kinds#a-package | 1 | 136 |
| cmd | `vibe check --path ./my-project --wal-max-age-hours 48` | howto/remove-a-package#root | howto/install-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; model/packages-and-kinds#a-package | 1 | 135 |
| cmd | `vibe check [OPTIONS]` | howto/remove-a-package#root | howto/install-a-package#root; howto/set-up-a-workspace#root; howto/update-packages#root; model/packages-and-kinds#a-package | 1 | 126 |
| cmd | `vibe init --no-registry` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 1141 |
| cmd | `vibe init --path .` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 11 |
| cmd | `vibe init --path <project>` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 1157 |
| cmd | `vibe init <group> <project-name>` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 143 |
| cmd | `vibe init <group>/<package>` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 144 |
| cmd | `vibe init <project-name>` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 142 |
| cmd | `vibe init [OPTIONS] [POSITIONAL]...` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 141 |
| cmd | `vibe init group <group> [path]` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 147 |
| cmd | `vibe init group org.example ./hello-vibe` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 170 |
| cmd | `vibe init org.example hello-vibe` | howto/use-a-private-registry#machine-wide | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; reference/machine-formats#the-documents | 1 | 168 |
| cmd | `vibe install --auth-required` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1042 |
| cmd | `vibe install --help` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 201 |
| cmd | `vibe install --json` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1048 |
| cmd | `vibe install --offline --path ./my-project` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 208 |
| cmd | `vibe install --unattended --auth-required flow:internal-helper` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1043 |
| cmd | `vibe install <kind>:<name>` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1136 |
| cmd | `vibe install [OPTIONS] [PACKAGES]...` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 176 |
| cmd | `vibe install feat:<name>` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 42 |
| cmd | `vibe install flow:<name>` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 66 |
| cmd | `vibe install flow:<name> --registry ./path/to/your/dir` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 80 |
| cmd | `vibe install flow:experimental \` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 842 |
| cmd | `vibe install flow:fork \` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 844 |
| cmd | `vibe install flow:internal-helper --assume-yes` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1031 |
| cmd | `vibe install flow:internal-helper \` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 839 |
| cmd | `vibe install flow:internal-helper@^0.1` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1106 |
| cmd | `vibe install flow:org.vibevm.world/wal` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 944 |
| cmd | `vibe install flow:secret \` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 846 |
| cmd | `vibe install flow:wal --assume-yes` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1174 |
| cmd | `vibe install flow:wal --exact --assume-yes` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1177 |
| cmd | `vibe install flow:wal@^0.1 --assume-yes` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1176 |
| cmd | `vibe install flow:wal@^0.1 --exact --assume-yes` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1178 |
| cmd | `vibe install flow:wal@^0.2` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1181 |
| cmd | `vibe install flow:wal@^0.2 --assume-yes # now sees v0.2.x if upstream tagged it` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 599 |
| cmd | `vibe install flow:wal@^0.3` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 838 |
| cmd | `vibe install org.vibevm.world/wal` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 207 |
| cmd | `vibe install stack:<name>` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 82 |
| cmd | `vibe install tool:example --git https://example.com/example.git --tag v1.0.0` | howto/install-a-package#what-happens | howto/publish-a-package#by-hand; howto/set-up-a-workspace#what-happens; howto/use-a-private-registry#edge-cases; howto/work-offline#root; model/packages-and-kinds#root; model/registries#edge-cases; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 209 |
| cmd | `vibe list --json --path ./my-project` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 1 | 227 |
| cmd | `vibe list --kind flow --verbose` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 1 | 226 |
| cmd | `vibe list [OPTIONS]` | reference/machine-formats#root | howto/install-a-package#by-hand; howto/publish-a-package#root; howto/update-packages#root; model/packages-and-kinds#root; model/two-trees#root | 1 | 217 |
| cmd | `vibe mcp <command> --help` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#root | 1 | 362 |
| cmd | `vibe mcp [OPTIONS] <COMMAND>` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#root | 1 | 356 |
| cmd | `vibe mcp install --agent opencode --scope project --what skill` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 257 |
| cmd | `vibe mcp install --agent opencode --scope user --what both` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 255 |
| cmd | `vibe mcp install --auto` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 256 |
| cmd | `vibe mcp install --auto --dry-run` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 260 |
| cmd | `vibe mcp install --auto --scope user --invoked-by manual-bootstrap` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 914 |
| cmd | `vibe mcp install --dry-run` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 297 |
| cmd | `vibe mcp install --help` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 369 |
| cmd | `vibe mcp install --scope both --auto` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 258 |
| cmd | `vibe mcp install --scope project --what skill --agent opencode --invoked-by opencode` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 927 |
| cmd | `vibe mcp install [--path <dir>]` | authoring/ship-tools-and-mcp-servers#servers | agent/ask-your-agent#two-transports; agent/give-your-agent-the-skill#what-happens | 1 | 239 |
| cmd | `vibe mcp serve --path ./my-project` | agent/ask-your-agent#two-transports | agent/give-your-agent-the-skill#what-happens | 1 | 371 |
| cmd | `vibe mcp serve [--path <dir>]` | agent/ask-your-agent#two-transports | agent/give-your-agent-the-skill#what-happens | 1 | 274 |
| cmd | `vibe outdated` | model/versions#root | howto/update-packages#root | 1 | 924 |
| cmd | `vibe registry <command> --help` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 656 |
| cmd | `vibe registry [OPTIONS] <COMMAND>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 641 |
| cmd | `vibe registry add: `private` registered (2 total registries).` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 390 |
| cmd | `vibe registry list                                # human-readable` | howto/use-a-private-registry#root | model/registries#root | 1 | 417 |
| cmd | `vibe registry list --json \| jq '.registries[] \| select(.adapter == null)'   # registries without publish support` | howto/use-a-private-registry#root | model/registries#root | 1 | 419 |
| cmd | `vibe registry list --json \| jq '.registries[] \| {name, url, auth}'` | howto/use-a-private-registry#root | model/registries#root | 1 | 1054 |
| cmd | `vibe registry list --json \| jq '.registries[].name'    # registry names` | howto/use-a-private-registry#root | model/registries#root | 1 | 418 |
| cmd | `vibe registry list --json \| jq '.registries[].token_env'` | howto/use-a-private-registry#root | model/registries#root | 1 | 1051 |
| cmd | `vibe registry list --quiet                        # one-line summary` | howto/use-a-private-registry#root | model/registries#root | 1 | 420 |
| cmd | `vibe registry list [--path <dir>]` | howto/use-a-private-registry#root | model/registries#root | 1 | 408 |
| cmd | `vibe registry list: 2 registries, 3 mirrors, 1 override.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 413 |
| cmd | `vibe registry list: <N> registries, <M> mirrors, <K> overrides.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 412 |
| cmd | `vibe registry publish "$pkg_dir" --json` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 444 |
| cmd | `vibe registry publish ./path/to/your/feat-package` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 61 |
| cmd | `vibe registry publish ./path/to/your/flow-package` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 78 |
| cmd | `vibe registry publish ./path/to/your/stack-package` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 96 |
| cmd | `vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 441 |
| cmd | `vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --dry-run` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 439 |
| cmd | `vibe registry publish ./vibevm/vibepacks/org.vibevm.world/wal/v1.0.0 --registry corporate` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 442 |
| cmd | `vibe registry publish <source> [--registry <name>] [--path <project>]` | howto/publish-a-package#root | reference/machine-formats#the-documents | 1 | 425 |
| cmd | `vibe registry redirect <pkgref> --to ...` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 496 |
| cmd | `vibe registry redirect-sync flow:internal-helper --dry-run` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 466 |
| cmd | `vibe registry redirect-update <pkgref>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 470 |
| cmd | `vibe registry redirect-update flow:internal-helper --clear-description` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 502 |
| cmd | `vibe registry redirect-update flow:internal-secret \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 504 |
| cmd | `vibe registry redirect-update flow:legacy \` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 503 |
| cmd | `vibe registry redirect-update … --dry-run --json` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 499 |
| cmd | `vibe registry remove` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 537 |
| cmd | `vibe registry remove mirror` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 554 |
| cmd | `vibe registry remove mirror   <OF> <URL>        # remove a [[mirror]]` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 542 |
| cmd | `vibe registry remove mirror <OF> <URL>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 548 |
| cmd | `vibe registry remove registry <NAME>` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 544 |
| cmd | `vibe registry remove registry <NAME>            # remove a [[registry]]` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 541 |
| cmd | `vibe registry remove: 0 mirrors remain.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 550 |
| cmd | `vibe registry remove: 1 registry remain.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 549 |
| cmd | `vibe registry set-mirror: 2 total mirrors configured.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 569 |
| cmd | `vibe registry sync                      # pull new tags` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 1 | 598 |
| cmd | `vibe registry sync                      # refresh everything in the lockfile` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 1 | 595 |
| cmd | `vibe registry sync --json \| jq '.refreshed \| length'   # how many were refreshed` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 1 | 596 |
| cmd | `vibe registry sync --quiet              # one-line summary` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 1 | 597 |
| cmd | `vibe registry sync [--path <dir>]` | reference/machine-formats#the-documents | howto/work-offline#edge-cases | 1 | 582 |
| cmd | `vibe registry sync: <N> refreshed, <K> skipped.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 587 |
| cmd | `vibe registry test: 1/4 reachable.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 610 |
| cmd | `vibe registry test: <ok>/<total> reachable.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 609 |
| cmd | `vibe registry vendor: <N> vendored, <K> skipped.` | howto/use-a-private-registry#root | howto/publish-a-package#root; howto/work-offline#air-gapped; model/registries#root; reference/machine-formats#the-documents | 1 | 628 |
| cmd | `vibe reinstall --assume-yes` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 1 | 679 |
| cmd | `vibe reinstall --force --assume-yes` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 1 | 680 |
| cmd | `vibe reinstall [<path>] [--force] [--assume-yes]` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 1 | 665 |
| cmd | `vibe reinstall packages/flow-wal` | howto/update-packages#recovery | model/two-trees#what-regenerates; reference/machine-formats#the-documents | 1 | 681 |
| cmd | `vibe search --purl pkg:cargo/serde` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 705 |
| cmd | `vibe search [OPTIONS] --purl <PURL>` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 687 |
| cmd | `vibe search [OPTIONS] <QUERY...>` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 686 |
| cmd | `vibe search wal --kind flow --registry vibespecs` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 704 |
| cmd | `vibe search wal --no-cache --limit 50` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 706 |
| cmd | `vibe search write ahead log` | model/packages-and-kinds#the-kinds | model/registries#the-index | 1 | 703 |
| cmd | `vibe show <command> --help` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 1 | 717 |
| cmd | `vibe show [OPTIONS] <COMMAND>` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 1 | 711 |
| cmd | `vibe show effective` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 1 | 712 |
| cmd | `vibe show features` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 1 | 714 |
| cmd | `vibe show purls` | howto/remove-a-package#what-happens | reference/settings-and-environment#edge-cases | 1 | 716 |
| cmd | `vibe tree --console` | model/boot-lane#root | howto/install-a-package#root; howto/remove-a-package#root; reference/machine-formats#the-documents | 1 | 741 |
| cmd | `vibe tree --json --path ./my-project` | model/boot-lane#root | howto/install-a-package#root; howto/remove-a-package#root; reference/machine-formats#the-documents | 1 | 740 |
| cmd | `vibe tree --plain` | model/boot-lane#root | howto/install-a-package#root; howto/remove-a-package#root; reference/machine-formats#the-documents | 1 | 739 |
| cmd | `vibe tree [OPTIONS]` | model/boot-lane#root | howto/install-a-package#root; howto/remove-a-package#root; reference/machine-formats#the-documents | 1 | 729 |
| cmd | `vibe uninstall <pkgref> && vibe install <pkgref>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 1122 |
| cmd | `vibe uninstall <pkgref> && vibe install <pkgref>@<version>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 1117 |
| cmd | `vibe uninstall <root>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 894 |
| cmd | `vibe uninstall <transitive>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 895 |
| cmd | `vibe uninstall [OPTIONS] <PACKAGE>` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 746 |
| cmd | `vibe uninstall flow:atomic-commits` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 1004 |
| cmd | `vibe uninstall flow:wal` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 1005 |
| cmd | `vibe uninstall flow:wal --path ./my-project` | howto/remove-a-package#what-happens | reference/machine-formats#the-documents | 1 | 754 |
| cmd | `vibe update --all` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 9 |
| cmd | `vibe update --all --assume-yes` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 775 |
| cmd | `vibe update --all --offline --path ./my-project` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 776 |
| cmd | `vibe update --all [OPTIONS]` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 761 |
| cmd | `vibe update --prune` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 1006 |
| cmd | `vibe update [OPTIONS] [PACKAGES]...` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 760 |
| cmd | `vibe update flow:wal` | howto/update-packages#root | howto/install-a-package#edge-cases; model/versions#moving; reference/lock-file#where; reference/machine-formats#the-documents | 1 | 774 |
| path | `vibevm/auth` | howto/set-up-a-workspace#what-happens | howto/install-a-package#edge-cases; howto/use-a-private-registry#root; reference/manifest#package-table; reference/settings-and-environment#variables | 1 | 827 |
| path | `vibevm/internal` | start/what-a-project-contains#who-writes-what | architecture/what-the-lifecycle-epic-delivered#the-route | 1 | 852 |
| path | `vibevm/vibepacks/` | howto/publish-a-package#root | model/two-trees#edge-cases | 1 | 443 |

## Строки «нужен автор», по файлу `docs/...`

### `docs/ALPHA-NOTES.md` (1)

- строка 24, раздел «Known alpha limitations (2026-08-20)», тип `section`, метка `живо`: `[key]` — поле/секция `[key]` не встречается ни на одной странице руководства

### `docs/authoring-feat.md` (1)

- строка 55, раздел «Manifest: `vibe.toml`», тип `section`, метка `живо`: `[writes]` — поле/секция `[writes]` не встречается ни на одной странице руководства

### `docs/authoring-flow.md` (1)

- строка 68, раздел «Anatomy of a flow package», тип `section`, метка `живо`: `[writes]` — поле/секция `[writes]` не встречается ни на одной странице руководства

### `docs/authoring-stack.md` (1)

- строка 91, раздел «Manifest: `vibe.toml`», тип `section`, метка `живо`: `[writes]` — поле/секция `[writes]` не встречается ни на одной странице руководства

### `docs/commands/init.md` (1)

- строка 146, раздел «Usage», тип `section`, метка `живо`: `[path]` — поле/секция `[path]` не встречается ни на одной странице руководства

### `docs/commands/mcp-install.md` (1)

- строка 262, раздел «Output (JSON)», тип `section`, метка `живо`: `mcp_servers` — поле/секция `mcp_servers` не встречается ни на одной странице руководства

### `docs/commands/mcp-serve.md` (2)

- строка 278, раздел «Tools exposed», тип `section`, метка `живо`: `files_written` — поле/секция `files_written` не встречается ни на одной странице руководства
- строка 279, раздел «Tools exposed», тип `section`, метка `живо`: `read_subskill` — поле/секция `read_subskill` не встречается ни на одной странице руководства

### `docs/commands/mcp-uninstall.md` (2)

- строка 317, раздел «Removal contract», тип `section`, метка `живо`: `mcp_servers` — поле/секция `mcp_servers` не встречается ни на одной странице руководства
- строка 319, раздел «Removal contract», тип `section`, метка `живо`: `[mcp_servers]` — поле/секция `[mcp_servers]` не встречается ни на одной странице руководства

### `docs/commands/registry-publish.md` (2)

- строка 436, раздел «Pipeline», тип `section`, метка `живо`: `[dependencies]` — поле/секция `[dependencies]` не встречается ни на одной странице руководства
- строка 440, раздел «Examples», тип `path`, метка `живо`: `vibevm/vibepacks/org.vibevm.world/wal/v1.0.0` — путь `vibevm/vibepacks/org.vibevm.world/wal/v1.0.0` не встречается ни на одной странице руководства

### `docs/commands/registry-redirect-sync.md` (3)

- строка 448, раздел «`vibe registry redirect-sync` — mirror target tags into a stub», тип `section`, метка `живо`: `pinned_ref` — поле/секция `pinned_ref` не встречается ни на одной странице руководства
- строка 459, раздел «Authentication», тип `section`, метка `живо`: `[redirect]` — поле/секция `[redirect]` не встречается ни на одной странице руководства
- строка 462, раздел «Pipeline», тип `section`, метка `живо`: `ref_policy` — поле/секция `ref_policy` не встречается ни на одной странице руководства

### `docs/commands/registry-redirect-update.md` (4)

- строка 485, раздел «Flags», тип `section`, метка `живо`: `[redirect]` — поле/секция `[redirect]` не встречается ни на одной странице руководства
- строка 486, раздел «Flags», тип `section`, метка `живо`: `pinned_ref` — поле/секция `pinned_ref` не встречается ни на одной странице руководства
- строка 490, раздел «Flags», тип `section`, метка `живо`: `target_url` — поле/секция `target_url` не встречается ни на одной странице руководства
- строка 491, раздел «Flags», тип `section`, метка `живо`: `ref_policy` — поле/секция `ref_policy` не встречается ни на одной странице руководства

### `docs/commands/registry-redirect.md` (1)

- строка 525, раздел «Flags», тип `section`, метка `живо`: `[redirect]` — поле/секция `[redirect]` не встречается ни на одной странице руководства

### `docs/commands/workspace-publish.md` (1)

- строка 800, раздел «What a published copy carries», тип `section`, метка `живо`: `[origin]` — поле/секция `[origin]` не встречается ни на одной странице руководства

### `docs/faq/version-conflicts.md` (2)

- строка 814, раздел «TL;DR», тип `section`, метка `живо`: `[patch]` — поле/секция `[patch]` не встречается ни на одной странице руководства
- строка 816, раздел «TL;DR», тип `section`, метка `живо`: `[workspace.versions]` — поле/секция `[workspace.versions]` не встречается ни на одной странице руководства

### `docs/git-source-dependencies.md` (2)

- строка 830, раздел «Git-source dependencies — whole-repo-as-package», тип `section`, метка `живо`: `[dependencies]` — поле/секция `[dependencies]` не встречается ни на одной странице руководства
- строка 851, раздел «Resolution order», тип `section`, метка `живо`: `[patch]` — поле/секция `[patch]` не встречается ни на одной странице руководства

### `docs/glossary.md` (11)

- строка 868, раздел «boot artifacts», тип `section`, метка `живо`: `[[entry]]` — поле/секция `[[entry]]` не встречается ни на одной странице руководства
- строка 874, раздел «capability», тип `path`, метка `живо`: `crates/vibe-core/src/capability_ref.rs` — путь `crates/vibe-core/src/capability_ref.rs` не встречается ни на одной странице руководства
- строка 876, раздел «content_hash», тип `path`, метка `живо`: `crates/vibe-registry/src/lib.rs` — путь `crates/vibe-registry/src/lib.rs` не встречается ни на одной странице руководства
- строка 891, раздел «pkgref (package reference)», тип `path`, метка `живо`: `crates/vibe-core/src/package_ref.rs` — путь `crates/vibe-core/src/package_ref.rs` не встречается ни на одной странице руководства
- строка 892, раздел «plan (install pipeline stage)», тип `path`, метка `живо`: `crates/vibe-install/src/lib.rs` — путь `crates/vibe-install/src/lib.rs` не встречается ни на одной странице руководства
- строка 893, раздел «resolve (install pipeline stage)», тип `path`, метка `живо`: `crates/vibe-resolver/src/lib.rs` — путь `crates/vibe-resolver/src/lib.rs` не встречается ни на одной странице руководства
- строка 898, раздел «`[package]` (manifest table)», тип `path`, метка `живо`: `crates/vibe-core/src/manifest/package.rs` — путь `crates/vibe-core/src/manifest/package.rs` не встречается ни на одной странице руководства
- строка 899, раздел «vibe.toml», тип `section`, метка `живо`: `[active]` — поле/секция `[active]` не встречается ни на одной странице руководства
- строка 900, раздел «vibe.toml», тип `section`, метка `живо`: `[llm]` — поле/секция `[llm]` не встречается ни на одной странице руководства
- строка 901, раздел «vibe.toml», тип `section`, метка `живо`: `[origin]` — поле/секция `[origin]` не встречается ни на одной странице руководства
- строка 902, раздел «vibe.toml», тип `path`, метка `живо`: `crates/vibe-core/src/manifest/project.rs` — путь `crates/vibe-core/src/manifest/project.rs` не встречается ни на одной странице руководства

### `docs/guides/agent-mcp-quickstart-opencode.md` (1)

- строка 931, раздел «Acceptance checklist», тип `section`, метка `живо`: `read_subskill` — поле/секция `read_subskill` не встречается ни на одной странице руководства

### `docs/loading-model.md` (4)

- строка 951, раздел «Materialised `vibedeps/` — vibevm's», тип `section`, метка `живо`: `[writes]` — поле/секция `[writes]` не встречается ни на одной странице руководства
- строка 952, раздел «Generated artifacts: `INLINE.md` and `INDEX.md`», тип `section`, метка `живо`: `[[entry]]` — поле/секция `[[entry]]` не встречается ни на одной странице руководства
- строка 953, раздел «Generated artifacts: `INLINE.md` and `INDEX.md`», тип `path`, метка `неизвестно`: `vibedeps/stack-rust/2.1.0/boot/rust.md` — путь `vibedeps/stack-rust/2.1.0/boot/rust.md` не встречается ни на одной странице руководства
- строка 955, раздел «Link types — `inline`, `static`, `dynamic`», тип `section`, метка `живо`: `[activation]` — поле/секция `[activation]` не встречается ни на одной странице руководства

### `docs/lockfile-format.md` (4)

- строка 968, раздел «`vibe.lock` — schema reference», тип `path`, метка `живо`: `crates/vibe-core/src/manifest/lockfile.rs` — путь `crates/vibe-core/src/manifest/lockfile.rs` не встречается ни на одной странице руководства
- строка 978, раздел «Top-level shape», тип `section`, метка `живо`: `deny_unknown_fields` — поле/секция `deny_unknown_fields` не встречается ни на одной странице руководства
- строка 995, раздел «`[[package]]` entries», тип `section`, метка `живо`: `files_written` — поле/секция `files_written` не встречается ни на одной странице руководства
- строка 999, раздел «Identity model», тип `path`, метка `живо`: `crates/vibe-install/src/lib.rs` — путь `crates/vibe-install/src/lib.rs` не встречается ни на одной странице руководства

### `docs/README.md` (1)

- строка 1025, раздел «Machine-readable inventory», тип `path`, метка `живо`: `SITE-MANIFEST.toml` — путь `SITE-MANIFEST.toml` не встречается ни на одной странице руководства

### `docs/registry-redirect.md` (6)

- строка 1063, раздел «Marker file», тип `section`, метка `живо`: `[redirect]` — поле/секция `[redirect]` не встречается ни на одной странице руководства
- строка 1064, раздел «Marker file», тип `section`, метка `живо`: `target_url` — поле/секция `target_url` не встречается ни на одной странице руководства
- строка 1065, раздел «Marker file», тип `section`, метка `живо`: `ref_policy` — поле/секция `ref_policy` не встречается ни на одной странице руководства
- строка 1066, раздел «Marker file», тип `section`, метка `живо`: `pinned_ref` — поле/секция `pinned_ref` не встречается ни на одной странице руководства
- строка 1069, раздел «Resolver behaviour», тип `section`, метка `живо`: `target_ref` — поле/секция `target_ref` не встречается ни на одной странице руководства
- строка 1073, раздел «Lockfile», тип `section`, метка `живо`: `via_redirect` — поле/секция `via_redirect` не встречается ни на одной странице руководства

### `docs/troubleshooting.md` (1)

- строка 1133, раздел «`package declares a [boot_snippet].source `…` that does not exist in the package`», тип `section`, метка `живо`: `[writes]` — поле/секция `[writes]` не встречается ни на одной странице руководства

### `docs/version-syntax.md` (1)

- строка 1171, раздел «Two-file model», тип `path`, метка `живо`: `Cargo.lock` — путь `Cargo.lock` не встречается ни на одной странице руководства
