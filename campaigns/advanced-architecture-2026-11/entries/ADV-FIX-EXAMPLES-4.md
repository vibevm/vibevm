# ADV-FIX-EXAMPLES-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe doc check --examples` over the English manual exits 0 on a fresh checkout: every example matches or is skipped with a true reason in examples/deferred.toml.

**Write.** `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/**/*.xml`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/deferred.toml`, `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/hello-deploy/example.toml`

**Check.** `cargo run --quiet -p vibe-cli -- doc check --examples --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0`

**Test.** No Rust test; the manual's examples are the golden tests. The acceptance criterion is the check's exit code: 1 today, 0 after. The counts are informative only and can differ by machine (they depend on the caller's environment and installed agent clients). On the machine this was planned on, today prints `41 matched, 27 differ, 7 skipped`; after, 0 differ and 0 failed, with 11 skipped = the 6 existing deferred rows that stay (start/install-vibe.xml#windows-install, howto/publish-a-package.xml#publish-dry-run and #publish, howto/read-documentation-locally.xml#cache-add-docs, lifecycle/scrape.xml#scrape-plan and #scrape-output) plus the 5 new rows below; howto/read-documentation-locally.xml#doc-serve is matched from a newly captured golden.

**Read first.**
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/deferred.toml
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/examples/hello-deploy/example.toml
- vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/read-documentation-locally.xml:15-27
- crates/vibe-doc/src/examples/fixture.rs:150-260
- crates/vibe-cli/src/cli/doc.rs:488-505

**Steps.**
1. Run the check and keep its output: it must exit 1.
2. deferred.toml: delete the `doc-serve` entry. Replace the `cache-add-docs` entry's `captured` with "release" and its `reason` with "the example fixtures' local registry carries only the small packages the other examples install, never this manual; the output is captured on a machine whose registry serves the published `org.vibevm.core/vibevm-docs`". Append five entries, each `captured = "blocked"`: page "agent/give-your-agent-the-skill.xml" ids "mcp-status" and "mcp-install" with reason "the output lists the agent clients installed on the machine that runs it (agent detection reads the real user home), which the example sandbox does not replace"; page "architecture/traceability.xml" ids "explain" and "select", and page "authoring/specs-agents-can-cite.xml" id "explain", with reason "runs in the source checkout and needs `.vibe/specmap-authority.json`, which only `vibe install` writes and a fresh clone lacks; its output also cites live page line numbers". Keep every other entry byte-identical.
3. In howto/read-documentation-locally.xml change the doc-serve example's `fixture="docs-store"` to `fixture="empty"` (no fixture named docs-store exists). Leave its `<expect></expect>` empty. Append to examples/hello-deploy/example.toml: a comment line `# The deploy plan names the host operating system; the page shows one.` then `[[normalize.replace]]` with `pattern = 'on `(?:windows|linux|macos)`$'` and `with = 'on `<OS>`'`.
4. Re-bless exactly the listed examples, one command per address in the decisions list: `cargo run --quiet -p vibe-cli -- doc check --examples --accept --force --only '<page>#<id>' --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0` (a non-zero exit from these runs is expected and not an error). Then capture doc-serve with the same command but `--accept` without `--force` and `--only 'howto/read-documentation-locally.xml#doc-serve'`.
5. Run `git diff` over the manual and read it: a new golden must not contain `/home/`, `/Users/`, `C:/`, `AppData` or `VIBEVM_HOME=/`. If one does, put that example's golden back to its previous text by editing the page by hand (copy the old text from `git diff`) and append a deferred.toml row for it with `captured = "blocked"` and a reason naming the machine path it leaked. Do the same (append a deferred.toml row, captured = "blocked", reason = the first differing line the check printed) for any example the final check still reports as differing that is not in the decisions list. These rows are entries in deferred.toml; this entry's own list state is not changed by them.
6. Run the check until it exits 0.

**Decisions already made.**
- Addresses to re-bless (output restyle, the new `registry` JSON field, the user-scope fixture registry, and the fixed package run): agent/give-your-agent-the-skill.xml#skill-list; authoring/write-a-flow.xml#init-package and #check; authoring/write-a-lang-package.xml#init-lang; howto/remove-a-package.xml#uninstall; howto/use-a-private-registry.xml#registry-add; howto/work-offline.xml#cache-add; lifecycle/build-package-deploy.xml#package and #deploy-plan; lifecycle/extensions-and-providers.xml#extensions; lifecycle/phases.xml#deploy-plan; lifecycle/scrape.xml#scrape-contract; model/registries.xml#registry-list; reference/machine-formats.xml#list-json; start/first-project.xml#init and #check; tutorials/markdown-and-xml.xml#init, #init-package, #install-contract, #install-split, #install-xml.
- `--only` is a substring match on `<page>#<id>`: `tutorials/markdown-and-xml.xml#init` also runs `#init-package` on that page, which is listed too, so that overlap is harmless. No listed address is a prefix of an unlisted id on the same page (checked).
- The publish, scrape and windows-install deferrals stay exactly as they are.
- Only the English manual (vibevm-docs/v1.0.0) is in scope.
- Re-blessing is done by the product's own `--accept --force --only`, never by a blanket `--accept --force` over the whole manual and never by writing new golden text by hand; hand edits are allowed only to put a golden back to its previous text.
- Do not use git to revert anything; restore old golden text by editing the page.

**Not in this entry.** Rewriting the prose around re-blessed examples, touching vibevm-docs-ru, changing product output to match old goldens, or a blanket `--accept --force` without `--only`.

**Depends on.** ADV-FIX-EXAMPLES-1, ADV-FIX-EXAMPLES-2, ADV-FIX-EXAMPLES-3

Entries that write the same file run one after another, never in parallel.
