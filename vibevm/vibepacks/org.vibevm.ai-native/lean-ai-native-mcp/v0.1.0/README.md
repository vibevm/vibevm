# AI-Native Lean over MCP {#root}

`mcp:org.vibevm.ai-native/lean-ai-native-mcp` serves the language family's
checking library through three tools. It exact-pins
`lang:org.vibevm.ai-native/lean-ai-native-lang` at `0.1.0`.

| Tool | Required arguments | Result |
| --- | --- | --- |
| `lean_ai_native_scan` | `carrier` | Declared subject/theorem inventory |
| `lean_ai_native_check` | `carrier`, `subject` | The selected model check and its evidence |
| `lean_ai_native_verify_witness` | `carrier`, `subject`, `witness` | Verification of the saved witness against its subject |

All arguments are strings. `carrier` and `witness` are relative to the
configured consumer project root. `subject` is the exact declared spec
address. Unknown fields, wrong types and missing fields produce an
`invalid_request` reply. Startup configuration cannot be overridden by a tool.

## Run

From this package directory:

```sh
cargo build --locked -p lean-ai-native-mcp
target/debug/lean-ai-native-mcp --path /absolute/consumer/project \
  --toolchain-dir /absolute/lean/distribution
```

`--toolchain-dir` is optional trusted startup configuration. The shared
engine determines toolchain availability; a missing required checker yields
a typed failure. It does not download or install a checker through a tool call.
`--path` defaults to the startup directory, and must name an existing directory.

The package also declares the `lean-ai-native-mcp` binary and the
`lean-ai-native` MCP server for VibeVM registration. The server executable
runs directly from its build output and needs no VibeVM executable on PATH.

## Reply contract

The MCP text block contains the exact JSON `Reply` returned by
`lean-ai-native-check::execute`; `isError` is `!Reply.ok`. CLI and MCP call
the same library. A tool-level refusal remains an MCP result with `isError`
set; unknown tool names and malformed protocol frames use the shared
transport's JSON-RPC errors. Stdout is reserved for protocol frames.

Inventory records declared associations. Model-proof evidence does not
certify a Rust implementation or grant owner acceptance. The first family
version does not install host scanner composition; that integration has its
own acceptance boundary.

## Maintainer checks

```sh
CARGO_TARGET_DIR="$PWD/target" cargo test --locked -p lean-ai-native-mcp
python3 tooling/sync-core.py --check
python3 tooling/sync-engine.py --check
```

The package owns `crates/lean-ai-native-mcp`. Its sibling
`crates/lean-ai-native-check` is copied byte-identically from the matching
language package. `crates/vendor/` carries the neutral MCP transport, specmap
and specmark dependencies from `flow:org.vibevm.ai-native/core-ai-native`.
Change their authored sources, then synchronize the family. The core sync
script is a development operation requiring the source catalog; the shipped
server uses only package-local Cargo dependencies.

`tooling/check-proof-parity.py` runs one actual proof through both compiled
front doors and compares the exact typed replies. Supply `--cli`, `--mcp`,
`--project`, `--toolchain`, and `--subject`; the language package's
`fixtures/model-proof` is a small reproducible project. It is separate from
the transport-only tests above and requires the pinned Lean distribution.

The [surface contract](vibevm/vibespecs/tools/lean-mcp.xml) defines startup
authority, strict arguments and reply parity.
