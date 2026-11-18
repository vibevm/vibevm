# Native corporate installer — integrated architectural review

## Scope and accepted implementation boundary

APP-VERSIONS is technically accepted separately in its campaign review. CORP
implementation `1e45189521b8840d242ce2dfc2a16ffbc24af86a` and conform integration
`890bcf1f059bc95f2fd0fbbd9c4ceb9dfccf392a` are reviewed and locally integrated.
This is not final distribution acceptance.

The lightweight Rust installer reuses canonical distribution manifests, bounded
hashing/safe ZIP extraction and the verified native vibe child for full-bundle
import and site application. Same verified file handles are consumed for ZIP and
companion staging. Existing independent script fallbacks are retained. Default
user PATH activation and explicit no-path use the existing native owner; machine
PATH/elevation are not required by product design. Contradictory CLI/profile homes
refuse rather than creating inconsistent persistent configuration.

## Verified evidence so far

- Bootstrap validation/shared helpers: passing tests; later classified batch includes
  compiled public doctests. Complete xtask classification recovery:341PASS.
- Conform:141existing,0new,no baseline update; bootstrap is gated,kit composition
  root is the explicit ambient-input boundary.
- Integrated self-apply:24PASS; native cache targeted group:6PASS.
- Linux basic native workflow:5PASS with empty external-tools PATH,space-containing
  kit/unrelated cwd,dry-run,fresh full3components+source,re-run preserving foreign
  file,invalid profile and corrupted ZIP refusal. Default PATH writes isolated HOME
  shell startup files; debug-file private URL is absent and failure stage retained.
  This uses an older bundle as a fixture,not the final freshly rebuilt distribution.
- Packaging target-selection test explicitly includes all4products including installer.
- Windows candidate890 release build succeeds;7native workflow cases pass on its
  actual source/hash-bound installer/bundle. SSH token is elevated,so this is NOT
  ordinary-user runtime acceptance. Own-token lowering experiments did not produce
  a running child; DLL_INIT_FAILED before main is not an installer failure.
- Windows standard-user verification is prepared for final assets; no security-policy,
  account or credential changes are authorized or used as a workaround.

## First integrated canonical panel — diagnostic FAIL

Actual log `/fast/tmp/corp-native-full-self-check-20261118.log` terminates101:
format PASS14s; workspace tests FAIL101/1817s; strict Clippy FAIL101/55s;
host check PASS121s;1MiB startup PASS;conform PASS11s;codegen PASS6s;wire diff PASS19s.
Only3workspace targets fail:offline app short-name registry lookup,offline local
monorepo acquisition,and version-bound index golden projection. Clippy identifies
one collapsible-if in the installer home-conflict guard.

Sources were changed for repairs during later keep-going steps. Therefore this is
mixed-subject diagnostic evidence,not a frozen-source final panel. Observer exit0
only means it saw the terminal marker. Wire diff is allowed pre-public reporting
of the changed corpus generator,not a claim of unchanged bytes.

## Finite repair scope under execution

- Existing native registry owner must permit cold filesystem-only capture offline,
  while filtering remote top-level candidates and refusing remote recursive gitlinks
  before contact. No globally weakened offline posture or new resolver.
- Public HTTPS gitlink regression must assert the intended offline refusal,zero TCP
  contacts and no lock publication; unrelated fixture failures must not pass it.
- Golden repomd is regenerated through journal replay/project/WriteCtx. Exact diff
  is only generator1.0.11→1.0.12; journal/other files unchanged. No hand normalization.
- Installer let-chain rewrite preserves behavior without lint suppression.

Builder executes targeted tests,then freezes the repaired commit and reruns the
coherent canonical gate. Candidate890 artifacts are not final delivery. Native
Windows/Linux/macOS matching final bundles and installers still require verification.

## Delivery acceptance remains open

Preserve actual5758-file private corp baseline and unchanged52third-party payloads.
Only after reviewed final assets:stage/update owned files in ~/corp,retain independent
scripts/profiles/admin,then generate only necessary transfer files in ~/corp-upd-2.
Verify hashes,relative actions,backup/replay and unchanged unrelated bytes. No private
profile contents enter committed records. No external publication or remote corporate
application is implied. Ordinary-user Windows criterion remains explicit until a
real non-elevated test runs; do not hide it behind user-directory installation.
