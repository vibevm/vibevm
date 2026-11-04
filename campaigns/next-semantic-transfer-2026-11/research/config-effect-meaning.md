# Config-value effect meaning — contract clarification

Read-only review of prototype `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`, original M-05-B.1/C.2/E.2 task records, governing PROP-003 anchors and actual callers. No builds or product edits. Earlier research reports remain unchanged; this note qualifies their interpretation.

**Conclusion:** the inspected contracts define config-value activation as evaluated, candidate-correct, inspectable and persistable **data**. They do not name a config-file writer, environment mutation or runtime parameter consumer. Absence of such a writer is therefore not, by itself, an implementation defect in these tasks. Creating generic config-writing machinery would expand the contract.

## Exact requirements

All following anchors are in `vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml` at the prototype pin.

- **M05B1-EFFECTS:** “Config effects select declared values; conflicting values refuse expansion.” The surrounding algebra includes dependency activation, forwarding, subskills and config values; it does not say these different effect kinds all perform filesystem writes.
- **M05B1-SAFE-STOP:** the normalized Rust API is available; new authored grammar, schema publication and migration are outside B1. “Cross-package installation integration and acceptance remain owner-controlled; this contract makes no claim of delivery to another package.” Task M-05-B.1 likewise accepts one typed algebra and legacy lowering, not a named configuration writer.
- **M05C2-CONDITIONING:** rejecting a candidate discards its “features, providers, dependency effects, subskills and config values”; “Only selected candidates contribute returned effects.” Its acceptance compares the chosen graph and derived activation effects. This requires correct returned data and no rejected-candidate leakage.
- **M05D1-REQUEST / M05D1-READER:** distinguish authored request from resolved results, and connect the production lock write/read path. **M05D2-DORMANT:** inactive branches emit no config effects; reactivation uses retained authored selections.
- **M05E2-UPDATE-INTENT / M05E2-REPORT:** reevaluate retained intent through B1/B2 and report exact before/after definition, payload and induced graph/effect differences. **M05E2-EXACT-REINSTALL:** replay exact locked selections/payloads without resolving new choices. These requirements do not introduce a config mutation destination.

The contrast is explicit: **M05D2-LOADING-COMPOSITION / M05D2-DELIVERY-CONSUMERS** name real subskill boot/MCP consumers. There is no corresponding named runtime/config-file requirement for the value-map field in this inspected contract family.

## Actual representation and consumers

The source symbol is `FeatureEffect::ConfigValue(LocalFeatureRef)`, not `ConfigEffect`. `FeatureDeclarations.config_values` maps package-local names to `ConfigValue::{Bool,Integer,String}`. Expansion selects those declared values into `FeatureExpansion.config_values`; conflicting merged values refuse atomically.

Actual consumers include:

- `crates/vibe-resolver/src/features/m_05_b_1_cases.rs`: verifies selected `verbose=true` and conflict behavior; configuration/dormancy tests verify inactive maps remain empty and reactivate correctly.
- `crates/vibe-install/src/record.rs::configuration_record`: stores config values inside the selected package's generated effects, bound to request/candidate/content evidence.
- `crates/vibe-orchestrator/src/machine_operation/configuration_choice.rs`: returns typed bool/integer/string config data in the inspected plan's effects and binds it into the prepared result identity.

Thus the minimal intended consumer is **the domain's evaluated configuration result, inspection/diff and exact persistence/replay path**. No returned value authorizes arbitrary key/path writes.

## Consequence for the new campaign

Retain “no runtime writer found” as a source observation, not a missing-feature label. Test selection, conflicting values, dormant branches, rejected-candidate retraction, visible plan data and record/reload/update identity. Do not demand an invented runtime sink to declare those semantics implemented.

External-adapter config reconciliation and environment activation/config edits elsewhere in the historical campaign have their own named targets and effect ownership. They are separate work, not implicit consumers of every ConfigValue key. If PROP-069 later promises applying one value to a specific app parameter or runtime field, define that mapping and test its real consumer then. Until then, expose this capability honestly as configuration data, preserving the distinction from user parameters and execution authority.
