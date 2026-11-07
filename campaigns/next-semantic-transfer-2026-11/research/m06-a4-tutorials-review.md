# A4 init/tutorial correction — ready for root review

Scope: documentation only, three repository files changed. No code/configuration values, app/prototype paths, derived dependencies, specmap, or other workers' files edited. No Git mutations, build, Cargo, Conform, runtime init, or tests run. Full A4 remains partial; this report is not acceptance.

## Corrections

- Authored EN and RU `v1.0.0/vibevm/vibespecs/tutorials/markdown-and-xml.xml`: replaced the false claim that `--link` is ignored with supported `--link static` / `--link dynamic` and the omitted-flag `dynamic` default. Kept the tutorial's existing manual manifest edit and all example commands, expected output, citations and links.
- Both pages distinguish omitted manifest `format` => `simple` from `vibe init package` scaffold default `normal`. No default changed.
- Root `vibe.toml`: replaced stale `packages/`, future self-hosting and obsolete init-registry-default commentary with current `vibevm/vibepacks/` and a statement that these registry blocks configure this project's remote resolution. Configuration values untouched.

## Observed source evidence

- `crates/vibe-cli/src/commands/init/package.rs:83-97`: doc/remap refuse `--link`; other nested packages use `args.link.unwrap_or_default().as_str()` and pass it to scaffold generation. `tool_manifest_text` writes that link and `fields.format`.
- `crates/vibe-cli/src/cli/init.rs:16-30`: InitLink default Dynamic, supported Static/Dynamic spellings. At 107 the CLI documents package format default normal.
- `crates/vibe-cli/src/commands/init/prompts.rs:52`: omitted format produces `normal`.
- `/tmp/nst-g3-rules.md`, selected PROP-035 FORMAT-SIMPLE/FORMAT-NORMAL: omitted package manifest format is Simple; Normal is opt-in.
- `crates/vibe-cli/tests/cli_init_kinds.rs:284-315`: existing `the_nested_form_honours_link_in_a_new_and_an_existing_project` tests explicit static, explicit dynamic, omitted dynamic, and scaffold Normal. Read only, not run.
- Canonical `PROP-072-boot-observability.xml`: source interpretation does not determine Package Simple/Normal or boot linkage. No semantic expansion inferred here.
- Campaign `m06-a4-residual-consumers.md` identifies these exact stale tutorial/comment claims and explicitly excludes an init rewrite.

## Pending verification

Root can run the existing focused fixture after acquiring the build/test lease:

```sh
cargo test -p vibe-cli --test cli_init_kinds the_nested_form_honours_link_in_a_new_and_an_existing_project -- --exact
```

No new prose-mirroring test added. The first syntax/hash script invocation failed with exit 127 because `python` is absent. Retried with `python3`: both edited tutorial documents parsed successfully (exit 0, `XML parse OK: 2 authored tutorial files`). These outcomes are recorded in the hash JSON; that check is syntax-only, not tutorial runtime acceptance. Changes are frozen for root review.

## Root acceptance

Accepted docs-only correction in `839fdd0c34e0dd96361cf82c5f74b3e77d9ac789`. Root verified all3file hashes, unchanged parsed root TOML values and unchanged XML element/attribute topology. Exact existing init link/default fixture passed1test (11filtered); both tutorial XML files parse. No runtime/default/manifest-format change. Remaining tutorial assertions were not revalidated by this narrow correction. Evidence: local context evidence/2026-11-07/m06-a4-tutorials/init-test.log. Full A4 remains partial.
