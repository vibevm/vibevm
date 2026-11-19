# Qwen Code standalone — isolated Linux x64 package lifecycle

2026-11-19. Architect independently accepted the candidate manifest and actual five-step lifecycle. Source-only package: `vibevm/vibepacks/org.vibevm.agents/qwen/v0.28.0/vibe.toml`, SHA-256 `a8b4879a425563d9d2e8caf01cb024ee829bc468589292ccc275fbdfc8bf3823`. No upstream binary is committed or published.

## Selection and layout

Official `QwenLM/qwen-code` CLI release `v0.28.0`, ID `448525214`, non-prerelease. GitHub's generic latest-release endpoint instead returned SDK `sdk-typescript-v0.1.35`; that is not the CLI and was excluded. Preview OpenTUI assets were also excluded. Exact-tag README offers standalone installation by default, with npm/Homebrew alternatives. npm package `@qwen-code/qwen-code@0.28.0` requires Node >=22.0.0, but that optional route does not impose an ambient Node/npm prerequisite on this standalone package.

Asset `758752401`, `qwen-code-linux-x64.tar.gz`, 92,809,778 bytes, SHA-256 `51b56eb1fffc104651016cf440e5afffc93c021e1a48fe2641060fbd9202abfb`. Entire privately acquired bytes match official metadata. Read-only inventory found 7,448 regular-file/directory members and no links. Preserve `payload_root = "qwen-code"` with its `bin`, `lib`, bundled `node` and native resources. The shipped `bin/qwen` resolves its own root and explicitly executes `node/bin/node` with `lib/cli-entry.js`; no global npm installation or runtime substitution is used.

The archive includes root/library licenses, generated-UI third-party notices and numerous bundled component licenses. Exact-tag root LICENSE SHA-256 `55367b61ccd2a016a0159ad886bd66a3ee6cb5e873d0c75c803c897dd245b075`. Root NOTICE request returned HTTP 404, not proof no notices are required. Presence of bundled notices is not a completed transitive/composition review; corporate offline redistribution remains separately unapproved.

## Actual results

Task `bdk2thr0i`, terminal exit 0:

- Install selected `org.vibevm.agents/qwen@0.28.0`, installer `linux-x64-standalone`, with matching owned receipt and preserved bundled runtime/resources: exit 0.
- Actual installed public `qwen --version`: exit 0, stdout `0.28.0`, empty stderr.
- Repeat installation: exit 0.
- Uninstall: exit 0, owned payload/public proxy absent and installation index empty.
- Invalid expected digest: expected exit 1, no new receipt/payload/proxy.

The manifest keeps the official pinned URL and digest. The isolated fixture used a private digest-keyed loopback remap to the independently acquired archive, not an actual online Vibe download or a redistributed corporate drop. Bubblewrap used `unshare-all`, `clearenv`, private proc, tmpfs HOME/TMP, read-only system/input/control/registry/binary mounts and only fixture evidence writable. No external network, host auth, normal user state or model/provider call was available.

Private evidence: `/data/git/vibevm-corp-qwen-20261119T183921Z/`, including release/npm/source metadata, full archive inventory, exact candidate manifest, `control/probe.py`, `evidence/steps.json`, `installed-receipt.json` and `native-lifecycle.log`. Terminal log SHA-256 `00f4465ea68f36e7bad1ff9e5ddeb15fb04037496d5e0e7ef21fbe622f8256f5`. Vibe binary SHA-256 `602dd4a4f4119df8e9e8eebd7b5cde2fc71a277c6b1334c3584a9c8e151b59ff`; proxy SHA-256 `87c17af288fd0887f462a3299deb6d92df338ca9a70d15e5f3e9149f0ff255f7`. Product source unchanged since `dfc544f91`.

## Remaining scope

Other platforms/architectures, online Vibe acquisition, interactive/native carrier qualification, optional npm closure and operator remediation, proxy/API setup and corporate redistribution remain separate. Version-command success does not prove those journeys. The shared explicit npm self-install/defer outcome is a subsequent implementation atom, not fulfilled by this standalone package or a missing-command refusal. No six-agent CORP completion is claimed.
