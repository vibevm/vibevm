# ADV-RELEASE-ATTEST-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The release workflow's build job attests the smoked bundle, fragment and bootstrap of every matrix target with a SHA-pinned actions/attest-build-provenance step placed after the smoke and before retention, with id-token/attestations write granted to the build job only; a new xtask test reads the workflow file and proves the placement and the permission scope.

**Write.** `.github/workflows/release-distributions.yml`, `xtask/src/dist/mod.rs`, `xtask/src/dist/workflow_tests.rs`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/start/install-vibe.xml`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0/vibevm/vibespecs/start/install-vibe.xml`, `CHANGELOG.md`

**Check.** `cargo test -p xtask --bin xtask workflow_tests 2>&1 | grep -F 'test result: ok. 2 passed'`

**Test.** xtask/src/dist/workflow_tests.rs: dist::workflow_tests::build_job_attests_smoked_bundle_and_bootstrap_before_retention and dist::workflow_tests::attestation_permissions_belong_to_the_build_job_only (new file; today the filter `workflow_tests` runs 0 xtask bin tests, so the grep for 'ok. 2 passed' fails before the work)

**Read first.**
- .github/workflows/release-distributions.yml: the `build:` job (its `permissions:` block, the smoke step(s) that call smoke-bootstrap, and the step `- name: Retain the native bundle as a workflow artifact` with its `path:` globs dist/vibevm-* and dist/vibe-bootstrap-*); also the `prepare`, `upload`, `finalize` jobs and the top-level `permissions:`
- xtask/src/dist/mod.rs: the trailing `#[cfg(test)] #[path = "credential_tests.rs"] mod credential_tests;` wiring to copy
- xtask/src/dist/build.rs: fns bundle_asset_name (`vibevm-{version}-{target}.zip`), fragment_asset_name (`vibevm-{version}-{target}.fragment.json`), bootstrap_asset_name (`vibe-bootstrap-{target}` plus `.exe` on x86_64-pc-windows-msvc); this is why the two subject globs cover all three subjects
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/start/install-vibe.xml: the `<edge-cases>` element and its `PROP-019#ACE-ACCEPTED` rule line
- vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0/vibevm/vibespecs/start/install-vibe.xml: the `<edge-cases>` element and its `PROP-019#ACE-ACCEPTED` rule line
- CHANGELOG.md: the `## [Unreleased]` section, the bullet group right after the **Upgrade notes.** block

**Steps.**
1. Sequencing: this entry runs after ADV-RELEASE-SMOKE-1 has landed, and one after another with every other ADV-RELEASE entry that writes .github/workflows/release-distributions.yml (aarch64 target, published-release guard). It also writes CHANGELOG.md, so it runs one after another with any other entry that writes CHANGELOG.md. Before starting, confirm that the `build:` job contains at least one line with the text `smoke-bootstrap`. If it does not, stop and report to the lead that the smoke prerequisite is missing.
2. Anchor guard: before editing, confirm that each of these anchors is present byte for byte: (1) the build job's three-line permissions block quoted in the next step; (2) the line `      - name: Retain the native bundle as a workflow artifact` inside the `build:` job; (3) the line `    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-019#ACE-ACCEPTED"/>` exactly once in each of the en and ru install-vibe.xml files; (4) exactly one line in CHANGELOG.md beginning `- **You can now tell vibe to install the version your artifact drop actually holds.**` inside `## [Unreleased]`. If any anchor is missing, changed, or occurs a different number of times (for example, because an earlier entry rewrote the build permissions or renamed or moved the retain step), stop and report to the lead which anchor differs. Do not adapt the anchor or guess a placement.
3. In .github/workflows/release-distributions.yml, `build:` job: replace the job's permissions block, which today reads exactly
```
    permissions:
      contents: read
    strategy:
```
with
```
    permissions:
      contents: read
      id-token: write
      attestations: write
    strategy:
```
Do not touch the top-level `permissions:` or the prepare/upload/finalize jobs.
4. In the same `build:` job, insert this step immediately before the line `      - name: Retain the native bundle as a workflow artifact` (6-space indent on `- name`, exact bytes, no `if:`):
```
      - name: Attest build provenance of the smoked bundle and bootstrap
        uses: actions/attest-build-provenance@4d101475d8b20a2381f78447822ac1eab6504dd8 # v4.2.2
        with:
          subject-path: |
            dist/vibevm-*
            dist/vibe-bootstrap-*

```
The trailing blank line separates it from the retain step, matching the file's step spacing.
5. Create xtask/src/dist/workflow_tests.rs (target well under 150 lines). It reads the workflow with `std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/release-distributions.yml"))` and splits it with `str::lines()`, which also strips CR on a Windows CRLF checkout. Add a private helper `fn job_lines<'a>(lines: &[&'a str], job: &str) -> Vec<&'a str>`. It returns the lines after the line equal to `format!("  {job}:")` up to (not including) the next line that starts with exactly two spaces followed by a non-space character, or EOF. It panics with a message naming the job when the header is absent.
6. Test `build_job_attests_smoked_bundle_and_bootstrap_before_retention`, on job_lines("build"): (a) exactly one line whose trim() equals `uses: actions/attest-build-provenance@4d101475d8b20a2381f78447822ac1eab6504dd8 # v4.2.2` (index attest). (b) The index of the line whose trim() equals `- name: Retain the native bundle as a workflow artifact` (index retain). (c) The last index of a line containing `smoke-bootstrap` (assert it exists, with the message `build job has no smoke-bootstrap step; the smoke prerequisite must land first`). (d) Assert smoke < attest < retain. (e) Between the `- name:` line just above attest and the next `- name:` line, no line has a trim() that starts with `if:`. (f) The first line after attest whose trim() equals `subject-path: |` is followed by two lines whose trim() equals `dist/vibevm-*` and then `dist/vibe-bootstrap-*`.
7. Test `attestation_permissions_belong_to_the_build_job_only`. (a) In job_lines("build"), find the first line equal to `    permissions:`. Collect the following lines that start with 6 spaces and a non-space 7th char, stopping at the first line with less indent. Their trim() values, sorted, must equal ["attestations: write", "contents: read", "id-token: write"]. (b) For each job in ["prepare", "upload", "finalize"], no line of job_lines contains `id-token` or `attestations`. (c) No line before the line equal to `jobs:` contains `id-token` or `attestations`.
8. Wire the module by appending to xtask/src/dist/mod.rs, after the existing credential_tests wiring:
```

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;
```
The test file needs no `use super::*;`. Import only std items it uses, so clippy --all-targets -D warnings stays clean, and leave no unused helper.
9. In vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/start/install-vibe.xml, insert this line immediately after the line `    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-019#ACE-ACCEPTED"/>`. That line occurs once in the file.
```
    <p>Each release bundle, platform fragment and bootstrap built by the release workflow also carries a GitHub build-provenance attestation; releases built before attestations were added have none. If you want proof that a downloaded file was built by the release workflow of `github.com/vibevm/vibevm`, not only that its digest matches the release manifest, run `gh attestation verify vibe-bootstrap-x86_64-unknown-linux-gnu --repo vibevm/vibevm` with the GitHub CLI, naming the file you downloaded. This check is optional and needs network access to GitHub; the installers do not run it and keep verifying the digest only, so offline and mirrored installs work as before.</p>
```
10. In vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0/vibevm/vibespecs/start/install-vibe.xml, insert this line immediately after the same line `    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-019#ACE-ACCEPTED"/>`, which also occurs once there.
```
    <p>Каждый релизный бандл, фрагмент платформы и бутстрап, собранный релизным workflow, также несёт аттестацию происхождения сборки GitHub (build provenance); у релизов, собранных до её появления, её нет. Если вам нужно доказательство, что скачанный файл собран релизным workflow репозитория `github.com/vibevm/vibevm`, а не только что его дайджест совпадает с манифестом релиза, выполните в GitHub CLI `gh attestation verify vibe-bootstrap-x86_64-unknown-linux-gnu --repo vibevm/vibevm`, подставив имя скачанного файла. Эта проверка необязательна и требует сетевого доступа к GitHub; установщики её не выполняют и по-прежнему проверяют только дайджест, поэтому офлайн-установки и установки с зеркала работают как раньше.</p>
```
11. In CHANGELOG.md, inside `## [Unreleased]`, insert this line immediately before the line that begins `- **You can now tell vibe to install the version your artifact drop actually holds.**`. It is the first bullet of the group after the Upgrade notes block. Do not add an Upgrade notes bullet.
```
- **Release files can be traced to the workflow that built them.** Every release bundle, bootstrap and platform fragment now carries a GitHub build-provenance attestation, so `gh attestation verify <file> --repo vibevm/vibevm` shows that the file was built by this repository's release workflow from the selected commit, not only that its digest matches the release manifest. The check is optional: the installers still verify the SHA-256 against the manifest only, so offline and mirrored installs are unchanged.
```
12. Run the check, then `cargo fmt --all` and `cargo clippy -p xtask --all-targets -- -D warnings`. Run `wc -l xtask/src/dist/mod.rs xtask/src/dist/workflow_tests.rs`: both must stay at or below 600 (mod.rs is 234 today).

**Decisions already made.**
- Action and pin: `actions/attest-build-provenance@4d101475d8b20a2381f78447822ac1eab6504dd8 # v4.2.2`. This is the commit that tags v4 and v4.2.2 point to, from git ls-remote on 2026-11-22. It follows the repo's convention of a full-SHA `uses:` with a `# vN` comment.
- Subjects: exactly the two globs `dist/vibevm-*` and `dist/vibe-bootstrap-*`, the same set the retain step uploads. They cover vibevm-<version>-<target>.zip, vibevm-<version>-<target>.fragment.json and vibe-bootstrap-<target>[.exe]. The vibe-installer-<target> file that build.rs also writes is neither retained nor attested. The en/ru docs paragraphs and the CHANGELOG bullet all name the same three kinds (bundle, platform fragment, bootstrap).
- The attest step has no `if:`. It runs on every matrix target and on build-only dispatches (upload=false), so a build-only run can be verified with `gh attestation verify`.
- Repository visibility: GitHub artifact attestations need a public repository (or GitHub Enterprise Cloud). github.com/vibevm/vibevm is treated as public: README.md lists it as a public source mirror, and distribution/install/install.sh and install.ps1 download releases anonymously from `https://github.com/${VIBEVM_RELEASE_REPOSITORY}/releases`, which only works for a public repository. That is why the step carries no visibility `if:`. If the first real workflow run fails the attest step with a plan or visibility error, that is an owner question, not something this entry adapts.
- Permissions: the build job gets `id-token: write` and `attestations: write` beside its existing `contents: read`. The top-level permissions and the prepare/upload/finalize jobs stay unchanged.
- Keyless OIDC/Sigstore attestation only: no signing key, no secret, no new workflow input.
- Installers (distribution/install/*) are not changed and do not verify attestations, because offline and corporate-mirror installs cannot reach Sigstore.
- The guard test is text-based and line-oriented in xtask. No YAML parser dependency is added. The test is identified by the step's exact `uses:` line and by the retain step's exact name.
- The test locates the smoke by the substring `smoke-bootstrap`, the script name the smoke capability fixed (distribution/release/smoke-bootstrap.sh / .ps1), so it does not depend on the smoke steps' display names or count.
- Every edit is anchored on exact bytes verified on 2026-11-22; a missing or changed anchor stops the entry (step 2) instead of being adapted.
- Docs: one paragraph inserted right after the ACE-ACCEPTED rule line in `<edge-cases>` of the en and ru install pages, and one CHANGELOG feature bullet. No spec anchor or #[spec] marker is involved.

**Not in this entry.** Not in this entry: attestation verification in the installers, xtask dist finalize/upload, or any other job; a YAML dependency or a general workflow-lint framework; changes to the smoke steps, matrix, inputs or other jobs' permissions; switching to actions/attest or adding SBOM attestations; dispatching a real workflow run (an owner step after merge).

**Depends on.** ADV-RELEASE-SMOKE-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
