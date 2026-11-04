# F09 — Core configuration meaning and direct-input changes

Architectural integration atom L0, after the complete222-task map. This is the independently usable core foundation of the frozen selected-installer candidate; it does not activate its new formats, persistence, app operations or CLI.

## Responsibility and dependency boundary

ConfigurationDecl::semantic_fingerprint validates the declaration and computes the versioned typed meaning recipe documented in formats/hash_recipes/configuration-semantic-1.toml. It includes feature relations/visibility/defaults/effects, group kind/cardinality/guards/default presence, parameter types/constraints/privacy/defaults, profile answers and config-value data. Presentation labels/order and the default-profile UI marker are excluded. Canonical set/map ordering, typed values and framed lengths prevent presentation changes or ambiguous concatenation from changing/confusing the intended meaning. Authored public definition literals are data, not supplied operation secrets.

ChoiceModel::prepare_feature_delta delegates to the existing FeatureModel preparation rule and returns direct input IDs before implication closure. Editing an original request must not silently replace it with its inferred effective feature set. No second solver, execution owner or grammar is added.

The five core source/test paths and one recipe depend only on existing core declarations/models and existing sha2. No Cargo dependency, wire/app/CLI change, new environment runtime or schema activation is needed. This independence was checked by root and a separate read-only dependency review, then by compiling/testing the slice in main rather than relying on the full candidate's green panel.

## Acceptance evidence and limits

Exact six-file hashes match the reviewed frozen candidate; source copies used fresh mtimes. Main core configuration selection65/65 and semantic_fingerprint doctest1/1 passed. The four semantic tests include empty/nonempty fixed goldens and discriminators for omitted/empty/exact/additive/profile/typed values/guards/effects/rules versus presentation-only changes. Core clippy (lib/tests, no deps, warnings denied), workspace fmt, core conform0findings, diff and specmap --check passed. Map9067units/4783tagged items/4023edges,0suspects/0gated orphans/0unresolved host edges,43known warnings.

This accepts the pure foundation only. Default review receipts, stored execution continuity and public selected-installer behavior remain later layers; the broad APP-CONFIGURATION-BASIS fact is not marked fully implemented from this method alone. No new old-task completion count follows, and no native Windows/macOS or full package/configuration consumer acceptance is claimed. FMT-014 records the additive API/recipe boundary.
