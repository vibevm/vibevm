# M-16-B.1 selected verification

Observed selected cases at the bound local 1.1.0 checkout; each named command exited 0. Raw output is retained byte-for-byte. These are affected owner checks, not a workspace-wide or native macOS/Windows proof.

| Owning check | Passed | Raw log SHA-256 |
| --- | ---: | --- |
| `wire` | 3 | `d1075a846370d3b1b718951b1de188d3ade43e3c19f140ddaa1557a4defba5cc` (`machine-operation-b1-wire.log`) |
| `extension-core` | 4 | `be04130d49db7a0651812d5581eec23ac01f61005f1d86df7fd73bf1c63af230` (`machine-operation-b1-extension-core.log`) |
| `extension-world` | 5 | `46b3fe5bcff81b187853ebea739006c306a09fdbed54ef90385956d9101d289e` (`machine-operation-b1-extension-world.log`) |
| `extension-discovery` | 1 | `d637dc1291d01c01790825d79813b102d380b82a42576e64389683721800dac2` (`machine-operation-b1-extension-discovery.log`) |
| `service` | 4 | `aa10ee21eea0a7f76398e4267f786111a946282b60d9e322c110df8e8357664e` (`machine-operation-b1-service.log`) |
| `publication` | 3 | `ec3ec305c90f8f2929e0252fd1fc4b3ae5eb47e34d0179feef41425e294ec3c2` (`machine-operation-b1-publication.log`) |
| `mcp-corpus` | 7 | `720b4edfaf21d044829662298e711e657fa1cf05e34b8423a6c4c1df0dfcfb26` (`machine-operation-b1-mcp-corpus.log`) |
| `mcp-machine` | 7 | `d8e87e16f3b24817434a0c1668fdaa404767ea71ec678151e7fbd68f6d663828` (`machine-operation-b1-mcp-machine.log`) |
| `query` | 2 | `6182c2f0f099d1ae496c409b3d6ab4ea39cdad97347e754d975451d555c2e551` (`machine-operation-b1-query.log`) |
| `coverage` | 4 | `b31150e388c930e4b2d12c17ed174eace9cdf30f9e974e9c690519499b2140a8` (`machine-operation-b1-coverage.log`) |
| `native-cli` | 1 | `7dcb1e609859d4492df739b3a62e9de7f5e2b4b158386361febdb55d4361afed` (`machine-operation-b1-native-cli.log`) |
| `native-mcp` | 1 | `7a7fdc5c65ce5d62cc92f142ab548fa2bd66e6b3bbeaf8fc0463e78a90e1d451` (`machine-operation-b1-native-mcp.log`) |

The generated wire reader covers descriptor and all six envelope arms, unknown members, and required-nullable omission. Core/selected-world checks cover authored extension schema and typed declared/absent projection. Service and MCP checks cover the one admitted pure planner, strict Input admission, owner replay of Result/Error, diagnostic retention of invalid originals, and explicit Event nonpublication. Coverage checks retain 214 operation IDs and 29 model IDs as gaps except for the reviewed MCP planner binding. Native registration comparisons checked the actual CLI and MCP owner inventories; those commands do not execute their handlers.

All other domain handlers remain extraction gaps; declared content hashes are not physical source-byte authentication. No broad mutation run, package publication, public Git push, final DP2, or all-platform native execution is claimed.
