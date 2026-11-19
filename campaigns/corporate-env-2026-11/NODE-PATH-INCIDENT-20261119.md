# Node install with a foreign PATH provider — bounded incident investigation

2026-11-19. Owner reported `vibe install -g node` failed with pre-existing external Node.js on PATH and worked after removing that PATH entry. No assumption about the owner's OS, existing runtime version, invocation context or exact failure is made here. No operator Node installation or PATH was changed.

## Observed Linux controls

First synthetic archive/layout control: same Vibe binary, issued registry alias `node → org.vibevm.node/node`, installer and equivalent empty settings homes; only presence of a foreign `node` directory in child PATH varied. Both installs/uninstalls succeeded. This control is not real upstream-runtime qualification. An initial fixture invocation omitted the required target of `vibe app exec`; its usage refusal was preserved, then the driver was corrected without a product change.

Real control task `bmqde6iyl`, terminal exit 0: canonical supplier checkout revision `f030ee3a83a0db098a3f735733884586065cb235`, exact source template version 1.2.1 (not Node's runtime version), actual official upstream version/digest metadata and `node-v26.13.1-linux-x64.tar.gz`. Full archive SHA-256 `518674de2b621728a5e4486f5fb3b7d2900bb3cc4aa97969edaa463db99f9778` matched official SHASUMS256. The same selected runtime, alias, installer `linux-x64`, binary and clean-home fixture ran with and without a synthetic foreign Node on PATH.

| Observation | No foreign Node | Foreign Node present |
|---|---|---|
| Explicit installation | Exit 0 | Exit 0 |
| Receipt/default | `org.vibevm.node/node@26.13.1` | Same |
| Fresh ordinary shell after normal `.profile` activation | Managed Node v26.13.1 and npm 11.20.0 | Same |
| Managed PATH location | `opt/paths/node/0` → selected payload `bin` | Same |
| `opt/bin/node` | Absent | Absent |
| Old inherited PATH without activation | No Node | Foreign Node still wins |
| Uninstall | Exit 0 | Exit 0; foreign bytes unchanged and resolution restored |

The canonical installer step downloads/unpacks an archive; this selected installer has no build/runtime command step that executes ambient Node. The launcher and freshly activated shell execute the selected payload. `path_dirs = ["bin"]` deliberately uses the managed slot link; absence of `opt/bin/node` is not a missing installation receipt or failure. Existing shell PATH is not retroactively replaced by a subprocess.

Bubblewrap used isolated namespaces/proc, cleared environment, synthetic HOME/settings, read-only exact supplier package/control/input/binaries and only evidence writable. A private loopback remap served official metadata and already verified supplier bytes. No external network or host credentials were available during execution. This is not a public distribution or a test on the owner's machine.

## Disposition and reopening

**Open, not reproduced on this Linux baseline; not fixed.** No production PATH fix was made. Per owner direction, further investigation is nonblocking and bounded: no new platform provisioning, identical retries or owner-attended request. Resume independent CORP work.

Useful reopening evidence is the actual OS/architecture, Vibe binary/source identity, Node supplier/template revision and selected installer/runtime, exact failing command/exit/diagnostics, and observed command winner in the affected shell. Gather only sanitized paths/metadata if a later authorized reproduction exists; do not collect credentials/environment dumps or require removal of a real installation.

Retained private roots: `/data/git/vibevm-node-path-real-20261119T191322Z/` (official metadata/input, exact package execution inputs, `control/probe.py`, `evidence/real-node-ab.json`, `real-node-ab.log`) and the earlier synthetic control under `/data/git/vibevm-corp-qwen-20261119T183921Z/evidence/node-path-ab*`. Required evidence remains protected from cleanup.
