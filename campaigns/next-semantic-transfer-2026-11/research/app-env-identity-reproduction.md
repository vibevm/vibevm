# Application environment identity collision — runtime reproduction

Main observed at `926a30f8e53d91da73bdefe860b95d6856a81be3`. Read the named user/spec rules and existing app validation, step_env_name, Run.env and local registry fixture paths. No product edits, test files, commits, worktrees, dependency builds or external network calls. A standalone Python fixture used isolated temporary registry/settings/HOME/cache directories only; recipe output contains synthetic values.

## Observed outcome

All six CLI invocations (plan + actual `install -g` for three cases) exited0. Plans and installation records preserved the two original feature/parameter IDs correctly. The recipe child silently received only one value under their colliding generated environment names, with declaration order deciding which survived.

| Feature/parameter declaration order | Plan selected feature | Child VIBE_FEATURE_A_B | Child VIBE_PARAM_A_B |
|---|---|---:|---|
| a-b, a.b | a-b only | 0 | RIGHT_VALUE (a.b) |
| a.b, a-b | a-b only | 1 | LEFT_VALUE (a-b) |
| a-b, a-c (control) | a-b only | 1 | LEFT_VALUE |

The control additionally receives VIBE_FEATURE_A_C=0 and VIBE_PARAM_A_C=RIGHT_VALUE. In both colliding cases the generated child has no distinct variable for the other ID. Notably, an **unselected** a.b feature overwrites the selected a-b feature with0 when declared later: Run exports every declared feature, not just selected ones.

For both colliding cases the persisted record still contains `features=['a-b']` and `parameters={'a-b':'LEFT_VALUE','a.b':'RIGHT_VALUE'}`. The mismatch occurs at execution projection, not in UI selection, the feature kernel, or parameter retention. Existing success/record tests alone would miss it.

## Exact evidence and binary qualification

Reproducer: `/tmp/nst-app-env-identity-reproduce.py`. Exact argv, exit statuses, selected plan data and observed output paths: `/tmp/nst-app-env-identity-evidence.json`. Retained scratch fixture/output tree: `/tmp/nst-app-env-identity-p2hqb3bg`. Each case contains its manifest/recipe, plan/install stdout/stderr and installed `observed.txt`; no owner files were touched.

Binary: `/fast/git/vibevm/target/debug/vibe`, version `vibe 1.0.11`, size 336952544 bytes, SHA256 `8dc8f4fa559d656031a63dd93bc9d9c1a0be2a6a6c5f780835cf92ec51b0e5ca`. Hash was checked before and after the reproduction and remained identical. Filesystem mtime: `2026-11-04T02:54:25.369240+00:00`. This timestamp precedes the final commit timestamp, which is normal for a binary built from a reviewed dirty snapshot; it does **not** establish stale code. It also does not prove exact-HEAD binary provenance. This is an observed executable reproduction, with a scope-limited current-source correspondence check.

The relevant `steps/params.rs::step_env_name`, `steps/run.rs::Run::new` and `manifest/mechanism/identity.rs::is_portable_token` are unchanged between c48c6d37a and inspected main. The intervening wizard_validation change only adds the option reference for template checking. Thus the current source has the same demonstrated collision path. No claim about unrelated exact-HEAD binary behavior is inferred from this reproduction.

CLI shape per case:

```text
vibe app plan org.example/env-identity --registry <scratch>/registry --offline --unattended --json --feature a-b --set a-b=LEFT_VALUE --set a.b=RIGHT_VALUE
vibe install -g org.example/env-identity --registry <scratch>/registry --offline --unattended --json --feature a-b --set a-b=LEFT_VALUE --set a.b=RIGHT_VALUE
```

The control substitutes a-c for a.b. Environment uses isolated VIBE_SETTINGS, VIBE_REGISTRY_CACHE, VIBEVM_SEARCH_CACHE_DIR, HOME and VIBE_NO_DEFAULT_REGISTRY=1, following existing UserScratch isolation. Existing VIBE_/VIBEVM_ environment inputs are cleared before fixture-specific values are set. The recipe only makes its payload directory and writes four named environment observations; it neither dumps ambient environment nor uses secrets.

## Cause in current owners

- `is_portable_token` admits lowercase alphanumeric IDs containing internal hyphen/dot; `InstallerOptionDecl::validate_features` and parameter validation reject duplicate **literal** IDs but not duplicate projected names.
- `step_env_name` uppercases ASCII and changes both `-` and `.` to `_`.
- `Run::new` maps all declared features to VIBE_FEATURE_* pairs and resolved parameters to VIBE_PARAM_* pairs in declaration order.
- `StepPaths::child_env` retains those pairs; recipe/shell `kinds::environment` applies them with Command::envs, where the later same-name value replaces the earlier one.

The mapping is many-to-one. The shared pure feature kernel is behaving correctly, and globally narrowing its literal ID rules would fix the wrong owner.

## Minimal proposed fix and migration (root decision)

Keep existing ordinary environment spellings. Put one pure environment-key projection/validation helper in the core **application domain**, then have the public vibe-app::step_env_name compatibility entry delegate to it. This avoids copying the mapping into validation or introducing a reverse core→app dependency. The helper should report the emitted key and both original IDs on collision.

Validate injectivity per installer option over **all** feature declarations and separately over **all** parameter declarations. Inactive features still emit0, optional parameters can acquire values later, and exclusivity does not make projected collisions safe. Different installer options are separate scopes. Feature and parameter prefixes are different namespaces, so a feature a-b and parameter a.b alone do not conflict with each other. A runtime preflight check should also protect programmatically assembled plans before the recipe/shell command is constructed; it must not silently choose the first/last winner.

Common installer feature IDs are currently generic literal IDs (nonblank/no-control), including spaces, Unicode, underscores and uppercase. Some are not representable as portable shell environment names; others introduce additional case/underscore aliases. Preserve these IDs in the generic configuration model and in package scopes that do not use this environment projection. At the **installer environment adapter**, require the resulting key to be portable and injective. A possible bounded spelling set for this existing projection is ASCII letters/digits/underscore/hyphen/dot, with the fixed prefix making initial digits safe; actual acceptance belongs to root. Do not invent synthetic encodings or a new JSON environment protocol to disguise an incompatible key. Common parameter IDs already use portable-token grammar but still need collision validation.

Do not apply this restriction globally to ordinary authored `env` step names: durable user-environment operations have their own name rules and are not the feature/parameter identity namespace. Keep the current public spellings for every noncolliding legacy definition.

Migration is a validation change, not a new TOML key/schema: rename one ambiguous local ID (for example a.b→a-dot-b) and update references, profiles, conditions, flags and saved consumer intent explicitly. Existing records retain original IDs/values and can support a named reconfiguration diagnosis; never guess which old ID the single environment value was meant to represent. Package-only generic literal IDs remain valid. New validation may reject previously accepted but ambiguous installer definitions, which should be documented as an intentional fix.

Suggested regression gates: the two orderings above both refuse with both IDs/key before child effects; distinct control keeps its exact environment; collisions are found even on inactive/optional rows; feature-versus-parameter cross-namespace remains valid; common installer unrepresentable IDs get a scoped error while generic/package literal IDs still compile. Native Windows/macOS behavior was not executed; uppercase canonical output makes the proposed uniqueness rule portable without relying on host case sensitivity.
