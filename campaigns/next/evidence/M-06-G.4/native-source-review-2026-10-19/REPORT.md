# M-06-G.4 continuation — independent source review

## Verdict

No concrete P1/P2 found in this bounded six-file source candidate. All6 current file hashes/lengths match SOURCE-FREEZE; WORKER.patch covers exactly those6 paths, and all11 target hunks match current bytes. Published report/patch SHA-256 match PROVENANCE; Root's reverse-check0 is recorded separately. Recommend preserving/committing this useful source candidate and obtaining the planned actual source-SHA-bound Windows evidence. **This is not G4 acceptance:** both original native usability/positive-negative proof obligations remain open; campaign count stays Root-reported113/222.

## Native ABI and lifetime

The Windows-only method uses the held Project directory handle under `&self`; it neither transfers nor closes it. The FFI uses system calling convention, HANDLE as raw pointer, DWORD32/LPDWORD outputs and BOOL32, with Kernel32 linkage. Only the initialized component-limit output is requested; other optional outputs are null with zero sizes. Failure immediately returns last_os_error; a zero bound is unavailable, never a guessed limit. Component units are WCHAR/UTF-16 excluding NUL, distinct from total path length. These points match [Microsoft's function contract](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew). The function itself does not establish total-path limits or support for every remote volume.

## Materialization behavior

The existing real reconciliation caller invokes preflight before inspecting/migrating/creating the slot, placing/removing payloads or writing a record. Windows prospective components use the nearest existing no-follow Project parent; failed/missing/length parents can be traversed toward an actual held ancestor. A successful query checks every prospective suffix component, including absent parents, record and staging names. This preserves existing no-follow/ownership/package/wire-coordinate laws and never silently shortens a name. Record/payload staging names match the incumbent tempfile prefixes and configured8-character random suffix length.

Metadata refusal identifies exact destination/component/stage, measured UTF-16 length, queried volume bound/deficit and practical shorter-coordinate/filename or volume remedy. It correctly says root shortening cannot repair an oversized individual component and invents no native failure code. The existing native OS-length path still supplies actual code and units when known, names unavailable query/total limit honestly, and keeps later-I/O partial-state uncertainty. POSIX code does not acquire a Windows260 limit or global disabling condition.

PROP-007 adds exactly MATERIALIZATION-COMPONENT-LIMIT; no old law is deleted or weakened. No PROP-009/035 semantic change is introduced.

## What the fixtures establish, and what remains open

The public fixture constructs a real source package, Workspace and ResolvedDep and invokes the normal install composition. Windows source includes a long-root positive path; a queried-volume legal ASCII L creation versus actual native L+1 failure and no-created-entry census; then a parser-consistent oversized group.name with a short physical source must refuse before dependency parent/slot/record/payload creation while preserving source and foreign bytes. An explicit shorter coordinate then installs selected bytes. The common short-root collision/owned cleanup/retry case preserves foreign neighbors. Linux's existing native length refusal remains independently exercised.

The published worker report records only3 Linux passes/0fail/775filtered, scoped formatting initial failure then corrected pass, and no Windows compilation/execution. The two Windows-only branches are source-reviewed, NOT reported as passed. Query availability, actual numeric L/control OS code, long-root success, pre-effect install refusal and retry must come from the actual Windows run. No guessed numeric ceiling, complete rollback guarantee, macOS proof, real release qualification or final acceptance is inferred.

## Narrow CI route

The workflow triggers only push to1.1.0 for the narrow affected paths, runs one windows-2025 job with a45minute bound, records source SHA/runner image/toolchain, executes one filtered library family and preserves its actual exit. It uses read-only contents, SHA-pinned checkout/upload actions, disabled persisted checkout credentials and always-uploaded named native evidence. No secrets, package publication, release, manual dispatch, install/config change or unrelated suite is introduced. Source syntax/pinned declarations do not prove the action revision resolves, toolchain is available or CI executes successfully; Root must inspect actual SHA-bound CI/log/JSON after its ordinary mirror push.

Reviewer performed no test/build, repository/source/spec/Git mutation, live API/runtime/config/model action or worker interference. Only REPORT/RESULT in this review directory were written. Direct extra source reads were narrow existing preflight caller/staging-call sites; Microsoft primary documentation was read only for the referenced native ABI/units contract. Root owns commit/mirror and final evidence acceptance.
