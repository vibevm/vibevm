# ADV-FIX-PUBDRY-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe registry publish <slot> --dry-run` against a GitHub registry with no publish token exits 0 and prints `repository presence: not checked (no publish token for github.com)`, and its `--json` report carries `"repo_presence": "unchecked"`.

**Write.** `crates/vibe-cli/src/commands/registry/publish.rs`, `crates/vibe-cli/src/cli/registry.rs`, `crates/vibe-cli/tests/cli_registry_publish_dry_run.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_registry_publish_dry_run`

**Test.** New file crates/vibe-cli/tests/cli_registry_publish_dry_run.rs (starts with `mod common;` and `use common::UserScratch;`). Fixture built in a tempdir: project dir with vibe.toml `[project]\nname='publisher'\nversion='0.1.0'\n[[registry]]\nname='acme'\nurl='https://github.com/acme'\n`; package dir with vibe.toml `[package]\ngroup='org.acme'\nname='notes'\nkind='flow'\nversion='0.1.0'\n` plus one file `payload` containing `hello\n`. Every run uses `user.vibe()` with `.env("VIBE_NO_DEFAULT_REGISTRY","1")`, `.env_remove("VIBEVM_PUBLISH_TOKEN")`, `.env_remove("VIBEVM_PUBLISH_TOKEN_GITHUB")`, `.env("HTTPS_PROXY","http://127.0.0.1:9")`, `.env("https_proxy","http://127.0.0.1:9")`, args `registry publish <package> --path <project>` plus the test's extra args. (1) `dry_run_without_token_reports_presence_not_checked`: extra `--dry-run`; asserts exit success, stdout contains `repository presence: not checked (no publish token for github.com)`, stdout contains `would push to `https://github.com/acme/` and `tag `v0.1.0``, stdout does not contain `Loaded publish token`. (2) `dry_run_without_token_json_reports_repo_presence_unchecked`: global `--json` placed before `registry`, extra `--dry-run`; asserts exit success and the parsed stdout JSON has `repo_presence == "unchecked"`, `dry_run == true`, `created_repo == false`, `host == "github.com"`, `tag == "v0.1.0"`. (3) `publish_without_token_still_refuses`: no `--dry-run`; asserts exit failure and stderr contains `no token available for host `github.com``.

**Read first.**
- crates/vibe-cli/src/commands/registry/publish.rs:1-410
- crates/vibe-cli/src/commands/registry/publish/layout.rs:125-142
- crates/vibe-cli/src/registry.rs:30-40
- crates/vibe-cli/src/cli/registry.rs:570-580
- crates/vibe-cli/tests/cli_registry_layout_publish.rs:1-90
- crates/vibe-publish/src/token.rs:140-165

**Steps.**
1. In publish.rs, add to `struct PublishReport` right after `dry_run: bool,` the field `repo_presence: &'static str,` with the doc comment `/// Whether the host was asked if the repository exists: "exists", "absent" or "unchecked" (dry-run without a publish token).`
2. In `run_publish`, replace the block from `let token = load_token_for_host(&host).context("loading publish token")?;` through `let creator = creator_for_url(...)...?;` with: a `match load_token_for_host(&host)` giving `Some(token)` on Ok, `None` on `Err(vibe_publish::PublishError::AuthMissing { .. }) if args.dry_run`, and `return Err(anyhow::Error::new(error).context("loading publish token"))` on any other Err; then `let probed = token.is_some();` and `let creator: Box<dyn vibe_publish::RepoCreator> = match token { Some(token) => { <the existing `Loaded publish token from …` ctx.step unchanged>; creator_for_url(&registry_section.url, org_segment, token).map_err(|e| anyhow!("{e}"))? } None => { ctx.step(&format!("repository presence: not checked (no publish token for {host})")); Box::new(crate::registry::direct_git_creator(layout::package_repo_url(registry_section, &source_dir)?)) } };` (add a one-line comment: a dry-run without a token plans without the host API).
3. After `outcome` is computed, add `let repo_presence = if !probed { "unchecked" } else if outcome.created_repo { "absent" } else { "exists" };` and set `repo_presence` in the `PublishReport` literal.
4. In the text branch, wrap the existing `ctx.step("{} repository `{}` on `{}`" …)` so it runs only when `probed`; when `probed && outcome.dry_run`, first print `ctx.step("repository presence: checked (read-only API call)")`. Leave the summary lines unchanged.
5. In crates/vibe-cli/src/cli/registry.rs, change only the doc comment of `RegistryPublishArgs::dry_run` (line ~576) from `Describe what would happen but make no API calls or pushes.` to `Describe what would happen and push nothing. With a publish token, checks repository presence with one read-only API call; without one, reports presence as not checked.` Do not touch the other three `--dry-run` doc comments in that file.
6. Create crates/vibe-cli/tests/cli_registry_publish_dry_run.rs with the three tests described, then run the check.

**Decisions already made.**
- Product change is in vibe-cli only; crates/vibe-publish/src/orchestrator.rs is NOT changed: with a token the read-only repo_exists probe stays (design decision), without a token the CLI routes through the existing DirectRepoCreator short-circuit, which never calls a host API.
- Unchecked-path repo URL is `layout::package_repo_url(registry_section, &source_dir)` = `<registry url>/<naming repo name>.git`; DirectRepoCreator derives host `github.com` from it.
- Exact text lines: `repository presence: not checked (no publish token for <host>)` and `repository presence: checked (read-only API call)`; `<host>` is the `host` variable already computed from the registry URL.
- JSON field name `repo_presence`, values `exists` | `absent` | `unchecked`, always present on the GitHub/GitVerse-API report (`PublishReport`); `DirectPublishReport`, `PublishStubReport` and the layout reports are unchanged.
- Non-dry-run behaviour is unchanged: AuthMissing still refuses a real publish with the same error.
- The GitVerse stub branch earlier in run_publish is untouched.
- Grepping the old doc-comment string will also hit vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/maintenance/surface/1.0.0.json (four occurrences) and the B-133 entry of BACKLOG.md: leave both untouched.
- Keep publish.rs under 600 lines.
- Commit message: `fix(registry): let publish --dry-run plan without a publish token` with a body saying a dry-run against an API host no longer needs a token, reports repository presence as not checked, and the JSON report gains repo_presence.

**Not in this entry.** Editing schemas/registry_publish_report.jtd.json and regenerating vibe-wire (that schema already lacks submodules, index_hook and index_checkout and is not used by the CLI); reordering repo_exists in vibe-publish/orchestrator.rs or fixing its doc comments; re-recording the CLI surface snapshot vibevm-docs/v1.0.0/maintenance/surface/1.0.0.json (it is the recorded 1.0.0 history, not a live mirror); closing or editing BACKLOG.md B-133 (the lead closes it, not this entry).

Entries that write the same file run one after another, never in parallel.
