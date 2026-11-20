# Hermes public interpreter prerequisites — bounded qualification

Contract `CORP-HERMES-INTERPRETER-QUALIFY-01`, architect `320420bd048ccd7ef40f6eeefc320d829df95100`, integrated `34148611e`. Test/report only; parked guarded-default implementation and its r3 RED remain unaccepted and untouched by this atom. APP-VERSIONS accepted baseline is not reopened. No CORP/NST completion.

## Actual operation and source

`qualify-hermes-interpreter.py` invokes the ordinary public `install -g org.vibevm.agents/hermes --registry /world/registry --offline --unattended --json` on the retained accepted baseline CLI SHA-256 `2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e`. It copies only the actual canonical Hermes v0.23.2 manifest and placement recipe into the local fixture registry. Their source/archive identity is unchanged from `HERMES-ARCHIVE-RESULT-20261119.md`; no supplier archive, wheel, pip, uv or supplier executable is needed or executed for these refusal worlds.

Every command runs under bwrap filesystem/PID/network isolation, cleared environment, synthetic HOME and settings path containing spaces. No ordinary auth/config/socket is mounted. Missing-Python worlds overlay `/usr/bin` with a private empty mount containing only the existing system shell; no counterfeit Python or production-policy switch is used. Retained binary and explicit proxy are read-only mounts. New output root is exclusive; prior evidence is never overwritten.

## Executed results

Private evidence: `/data/git/vibevm-keyring-storage-20261120T0100Z/hermes-prerequisite-public-r2/`, complete `steps.json` and `summary.json`; outer log `hermes-prerequisite-public-r2.log`. Final runner exited0 with three worlds and five actual public commands after adding exact accepted manifest/recipe SHA-256 checks and recording proxy identity. Initial r1 also passed but predates those explicit source assertions; its receipts remain retained, not substituted for r2.

| Subject | Actual result | Qualified boundary |
|---|---|---|
| Actual Hermes declaration, absent `/usr/bin/python3` | exit2, `runtime_remediation_refused`, exact missing-system-Python diagnostic | System search refusal before target app work/index/launcher/payload paths; package-resolution caches are not claimed absent |
| Actual Hermes declaration, read-only host system binaries in user namespace | exit2, `runtime_remediation_refused`, protected-system-owned executable diagnostic | Provenance refusal only; **not** version/ABI incompatibility evidence |
| Separate synthetic archive with no Python declaration | install0 → public app run0 with exact synthetic marker → uninstall0 | Existing archive installation works in the same missing-Python runner; exact installation receipt exists before uninstall, empty index and absent public proxy afterward |

The synthetic control shares the local public acquisition/installer route and isolated runtime, not the Hermes supplier payload or system-Python requirement. It is not a Hermes positive, library-composition positive or proof that Hermes can run without Python. Earlier library-composition resource evidence remains separate.

## Explicitly unrun and next route

ABI incompatibility and witness drift before application are **UNRUN**: the admitted isolated runner fails system-ownership before the capability probe, so it cannot supply the required trusted positive interpreter witness. No policy weakening, foreign interpreter execution or host credential access was attempted. Normal public Hermes positive installation remains UNRUN. No redistribution/platform/provider claim follows.

Per the architect contract, the blocked discriminator does not justify repeated runner workarounds: proceed to the independent existing npm missing/defer/cancel pre-effects route under a finite follow-up. Exact-source independent review of this script/report is still required. No product code or schema changed; no Cargo rebuild or full host gate is claimed for this script-only qualification.
