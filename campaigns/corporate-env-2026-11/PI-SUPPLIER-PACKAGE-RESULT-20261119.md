# Pi supplier package — isolated Linux x64 lifecycle

2026-11-19. Architect reviewed and accepted the actual isolated lifecycle as a bounded package result. This is not online installation proof, corporate redistribution clearance, cross-platform qualification or admission to the existing npm-layout native instruction carrier.

## Exact inputs

Canonical source-only package: `vibevm/vibepacks/org.vibevm.agents/pi/v1.5.2/vibe.toml`, SHA-256 `18c1a6330abc6282d44fa7efe3c7d2b4562445cb7cbde450a5d3e93e221526b5`. It selects the official pinned supplier URL and digest, `tar_gz`, `payload_root = "pi"`, executable `$app_root/pi` and public command `pi`. No npm/Node prerequisite is invented for the standalone archive.

Official repository `earendil-works/pi`, release `v1.5.2`, release ID `440309680`, published 2026-11-10. Linux x64 asset `731292534`, `pi-linux-x64.tar.gz`: 58,922,566 bytes, SHA-256 `811c6f2c9e7aaee252b5404252e556fb713aa209c1f7f924226aa77d4ef70cb5`. Full acquired bytes match the official asset size and digest. Read-only tar inventory found 234 regular-file/directory entries, no links; `pi/pi` and the complete resources/native subtree are preserved, not just a copied executable.

The exact-tag source root license is MIT, SHA-256 `0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48`. No LICENSE/NOTICE/SBOM member was found in this archive. Root licensing is not complete third-party notice or compiled-runtime clearance; corporate offline redistribution remains unapproved. No archive is included in this repository or a corporate drop.

## Actual execution

Task `bcrq39255`, terminal exit 0:

| Check | Observed outcome |
|---|---|
| Install exact package/installer | Exit 0; `org.vibevm.agents/pi@1.5.2`, `linux-x64-standalone` receipt |
| Installed public command `pi --version` | Exit 0, stdout `1.5.2` |
| Repeat installation | Exit 0 |
| Uninstall | Exit 0; owned payload and public-command proxy absent, installation index empty |
| Corrupted expected digest | Expected exit 1; no new installed receipt, payload or public-command proxy |

The manifest keeps its official origin. A private fixture-local digest-keyed remap served the independently acquired bytes over isolated loopback; this is not an online supplier request or approved redistributable offline drop. Bubblewrap used `unshare-all`, `clearenv`, private proc, tmpfs HOME/TMP, read-only system/control/registry/input/binary mounts and only its evidence directory writable. No host authentication, real provider/model request, external network or normal user installation was available. Only fixture-owned processes were used.

Vibe binary SHA-256 `602dd4a4f4119df8e9e8eebd7b5cde2fc71a277c6b1334c3584a9c8e151b59ff`; proxy SHA-256 `87c17af288fd0887f462a3299deb6d92df338ca9a70d15e5f3e9149f0ff255f7`. Product source was unchanged since `dfc544f91`; builder HEAD before integration was `7dc9436c11b0b0168e90276653dc119a38f33f62`, including the accepted separate synthetic root-member test.

Private evidence root: `/data/git/vibevm-corp-pi-20261119T182623Z/`. Retained `metadata/release.json`, `evidence/archive-inventory.json`, `control/probe.py`, exact candidate registry, `evidence/steps.json`, `evidence/installed-receipt.json` and `evidence/native-lifecycle.log`. Terminal log SHA-256 `161111e011efe765ebd64280a350590d0bb40552c2156dab96a73ec6dad9e373`.

## Remaining cells

Official metadata lists Linux/macOS/Windows x64 and arm64 direct assets, but only Linux x64 was acquired and executed here. Remaining platform/architecture installations are UNRUN. Offline redistribution/component notices, online supplier acquisition through Vibe, interactive conversation, proxy routing, AI setup and native instruction-carrier layout admission remain separate. Neither this receipt nor the retained historical npm runtime proves those outcomes. Six-agent CORP completion is not claimed.
