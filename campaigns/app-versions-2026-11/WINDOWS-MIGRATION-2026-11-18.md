# APP-VERSIONS native Windows migration — 2026-11-18

## Actual native result

A current-source Windows AMD64 binary built offline on the owner's designated
Windows host successfully migrated the real local npm package
`accept-npm-fixture@1.0.0` from Node20.18.0 to Node22.11.0. Both Node runtimes were
installed through the real unchanged archive installer and independently executed.
The fixture was physically present only in the old runtime before migration; the
old adapter's `observed` list had to contain its exact name/version. Migration
reported `ok:true`, `ecosystem:npm` outcome `done`; the fixture was then physically
present under the new runtime's own npm prefix. The active default remained20.18.0.
The four npm/corepack package.json hashes (two per runtime) remained unchanged.

This is real Windows execution, not a Linux test, cross compilation, empty adapter
list or successful wrapper exit substituted for a structured outcome.

## Two demonstrated defects and bounded repairs

1. Node template revision1.2.0 declares four `.sh` ecosystem recipes without an
   interpreter. Windows host-default execution selects cmd. Both Node installations
   and runtime execution succeeded, but extras listing returned exit0 with a
   structured `observed_failed` and recorded adapter `outcome:failed`. The candidate
   revision1.2.1 explicitly selects `posix_sh` for all four existing recipes. It
   requires a compatible POSIX shell on Windows; the test used existing Git Bash.
   No extension-based interpreter guessing or host-default change was introduced.
2. That package correction alone still yielded an empty list despite a physically
   installed fixture. Ordinary `app exec` and npm itself saw the fixture. The shared
   migration/extras owner constructed PATH from proxies and a hardcoded payload
   `bin`, omitting Windows Node's declared `path_dirs = ['.']`. It now adds the exact
   selected installation's directories through the existing `paths::session_dirs`
   owner. Proxies/bin retain their priority, declared directories are deduplicated
   before inherited PATH, and the selected session choice is preserved. No global
   slot-default lookup, new resolver or public Exposure API change was added.

A public Linux regression installs two providers with payload-root commands and no
public command proxies. Export must execute version1, import version2, and the
active default must not change. The fixture asserts the authored path replacement
and absence of public proxies, preventing a vacuous success through the old `bin`.

## Source and binary binding

Base tracked source snapshot: `ad05957055fef365a5d9a2305b4b96a875d92903`, plus the
following runtime repairs (final remote and local file hashes agree):

| File | SHA256 |
| --- | --- |
| `crates/vibe-app/src/steps/env.rs` | `a8d8d38742f8c27d11546c5b0e1703b1683c0558715b331a969efb01c3da55a3` |
| `crates/vibe-app/src/migrate/run.rs` | `ddb2087e4aef023c5299e6fc23ae18640e7bdccef3d93fb34cb22d953414c7c1` |
| Native Windows `vibe.exe` | `45bf9eaa446f855c348a8c577e13bcbdb9887ef757c7e8136cf990cd9d58f8cc` |
| Native Windows `vibe-app-proxy.exe` | `af3e2c4486da3bda47a6f6e1158f4f137c26b84e6fa8b5b8b2e6d5b578db2f1f` |

Node Windows ZIPs were explicitly authorized, obtained separately and verified
against the existing pinned SHASUMS documents. No mirror lock changed:

-20.18.0 ZIP:29,602,979bytes,
 `f5cea43414cc33024bbe5867f208d1c9c915d6a38e92abeee07ed9e563662297`.
-22.11.0 ZIP:34,188,182bytes,
 `905373a059aecaf7f48c1ce10ffbd5334457ca00f678747f19db5ea7d256c236`.

## Isolation incident and restoration

The established separate Windows test root was reused through a **new** child;
old checkouts and prior runs were not overwritten. Build used existing toolchains,
a new target, offline Cargo and enabled incremental compilation. Scenario HOME,
USERPROFILE, APPDATA, LOCALAPPDATA, settings, caches and install root were isolated.
The npm fixture registry listened only on loopback. Existing nvm4w was excluded from
the scenario process PATH.

However, initial normal application installation still persisted two test-owned
path entries in the ordinary user's PATH. Isolation by HOME/settings alone was
insufficient for the existing Windows persister. This was measured, not concealed.
Only those two owned entries were removed, and the result had to match the saved
original PATH hash **before** restoration was applied. Final checks prove the
original user PATH, machine PATH, ordinary Node command/version and profile hashes
match their baseline. The final migration uses the established test-only environment
registry owner rather than the ordinary user environment. Do not describe this as
"the ordinary PATH was never modified."

## Retained local evidence

- `/fast/tmp/app-versions-base-native-build-20261118.log`
- `/fast/tmp/app-versions-base-unchanged-adapter-outcome-20261118.log`
- `/fast/tmp/app-versions-base-cmd-recipe-diagnosis-20261118.log`
- `/fast/tmp/app-versions-base-list-diagnosis-20261118.log`
- `/fast/tmp/app-versions-base-build-path-order-final-20261118.log`
- `/fast/tmp/app-versions-base-migration-isolated-final-20261118.log`
- `/fast/tmp/app-versions-windows-migration-verified-receipt-20261118.json`
- `/fast/tmp/app-versions-base-owned-path-restoration-final-20261118.log`
- `/fast/tmp/app-versions-base-final-subject-host-20261118.log`

The earlier cold-cache offline attempts and insufficient interpreter-only candidate
remain failed/precondition evidence. None is relabeled a successful migration.
Local package/core commits and integration do not authorize tags, external package
publication or a release. Final Linux regression/strict/full-scenario outcomes are
recorded separately on their actual terminal subjects.
