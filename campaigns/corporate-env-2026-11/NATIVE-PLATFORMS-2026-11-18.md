# Native installer platform evidence — 2026-11-18

## Exact subject

Runtime source: `576e6a8623c10087c9ad5a0730754d6ba9d61be6`.
Later architectural review integration `7775fa33e` changes documentation only.
Both native builds below use the same committed source, existing native toolchains,
offline Cargo and enabled incremental compilation. No external release is implied.

## Final completed builds and workflows

| Platform | Installer SHA256 | Release ZIP SHA256 | Actual native workflow |
| --- | --- | --- | --- |
| Windows x64 MSVC | `c9b2318b2065ae919d14e97d22bce3fc013cb3f1dc64479666aec9e9a5f0aad7` | `14c6f989920e85db3e64676a755b35a940042e2321e3de6b04c7de6a2dbbb777` | Seven cases PASS; elevated SSH token, **not** standard-user acceptance |
| macOS Intel | `7e7858e2487d40a52313a27bad43063b73cd776e08eb6602eb42deebbcc44b12` | `4b6cbb78c35767db74c4dfcf761c02a4389e3561fec6b9784955666cd453bdf8` | Seven cases PASS under ordinary UID501 |

Windows release compilation completed in17m07s; macOS in34m15s. Each local
admission receipt verifies the canonical bundle manifest's source/target and the
size/SHA256 of every component and the packaged source archive. Only rebuilt owned
assets were transferred back; unchanged third-party drops were not downloaded.

The seven actual cases on each platform are preflight without publication,fresh
installation,re-run preserving a foreign settings file,explicit no-PATH opt-out,
conflicting homes refusal,invalid profile refusal and corrupt ZIP refusal. Tests
use a space-containing kit from an unrelated working directory. All three runtime
components and the source tree must exist after installation. Automatic user PATH
and byte-preserving no-PATH behavior are measured through the platform's existing
persister,with isolated HOME on macOS and the established test-only registry owner
on Windows. External shell/archive tools are absent from the process PATH.

The generic invalid profile carries a deliberately private-looking sentinel. The
actual debug log must omit it while retaining the failed stage. Redirected output
must contain no animation escapes. These are executable assertions,not claims
inferred from installer exit status alone.

## OOM recovery and remaining boundaries

The first final coherent Linux gate was interrupted during workspace tests when
the session was killed by an owner-reported OOM. It has **no terminal verdict**.
The resumed complete gate uses process-local `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1`; incremental remains enabled. The running source is frozen.
The older diagnostic panel's three failed targets and one lint are retained as
failures; their focused repairs passed before this final source was committed.

Linux final release/native proof remain pending until the resumed full canonical
panel succeeds. No current Linux PASS or whole-distribution completion is inferred
from the two completed platforms. Windows ordinary-user runtime verification also
remains pending: the existing SSH session is elevated. Lowering an own token did
not yield a running test child; that is not a standard-user proof and no UAC,
window-station ACL,account or credential change was used to manufacture one.
A single normal-terminal verification command is prepared on the designated host.

## Private delivery integrity

After OOM,all5758 existing private corporate files were independently hashed
against the pre-update baseline:zero changed,zero missing. The main kit has not
been updated and `~/corp-upd-2` has not been created at this evidence boundary.
Independent script fallbacks and their helper retain their original hashes.

Final delivery must admit Linux assets and the remaining gates,then update only
reviewed owned files. Preserve private profiles,admin state,mirrors and unchanged
third-party payloads. Assemble the necessary offline-transfer delta rather than
recopying the complete distribution. There is no external publication authorization.

## Machine-local evidence

- `/fast/tmp/corp-native-final-windows-build-verified-20261118.log`
- `/fast/tmp/corp-final-delivery-576e6a862/windows/ADMISSION.json`
- `/fast/tmp/corp-final-delivery-576e6a862/windows/NATIVE-FUNCTIONAL.json`
- `/fast/tmp/corp-native-macos-final-oom-recovered-outcome-20261118.log`
- `/fast/tmp/corp-final-delivery-576e6a862/macos/ADMISSION.json`
- `/fast/tmp/corp-final-delivery-576e6a862/macos/NATIVE-FUNCTIONAL.json`
- `/fast/tmp/corp-native-final-coherent-self-check-oom-recovery-20261118.log`
- `/fast/tmp/corp-baseline-after-oom-verification-20261118.json`
- `/fast/tmp/corp-fallback-preservation-20261118.json`

Local absolute paths identify evidence,not shipped inputs. No private profile,
credential or actual corporate endpoint is reproduced in this report.
