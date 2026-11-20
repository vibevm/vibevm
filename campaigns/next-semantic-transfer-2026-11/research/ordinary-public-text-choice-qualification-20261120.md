# Ordinary string and choice parameters — qualification

Contract NST-ORDINARY-PUBLIC-TEXT-CHOICE-QUALIFY-01 (`36da3a9ddc7a5997d6f5a9f4eea56cc7196d63e6`, integrated `3382738a3`), after accepted integer implementation `cbfd4d354`. Test/report only: no product/schema/preview changes or parent promotion.

## Nine distinct parameter worlds and scalar control

Each fresh project has one required public parameter on an exact compulsory ordinary local flow, constant explicit feature/choice flags, no profile/default/secret/path/automatic reliance. String is separate from a declared choice parameter; neither is a feature-choice group. Choice values alpha/beta have display names Friendly Alpha/Friendly Beta.

| Type/input | Actual normalization | Public install |
| --- | --- | --- |
| String omitted | Absent parameter | Unsupported ordinary bool-only gate |
| String empty literal | Text("") retained | Same unsupported gate |
| String surrounding spaces | Text("  literal value  ") unchanged | Same unsupported gate |
| String numeric0 | Invalid typed input | `not a string value` |
| Choice omitted | Absent parameter | Unsupported ordinary bool-only gate |
| Choice alpha | Text("alpha") | Same unsupported gate |
| Choice beta | Text("beta") | Same unsupported gate |
| Choice other | Invalid declared value | `other` is not one of alpha, beta |
| Choice Friendly Alpha | Label is not a value | `Friendly Alpha` is not one of alpha, beta |

Source definition parsing, original typed maps and normalization are checked independently before invoking the actual public install in every world. Empty is a valid present string at this layer, not omission. Surrounding spaces remain literal; display labels are not silently converted into choice values. Type/value errors retain configuration.parameters.text through existing candidate incompatibility/unsatisfiable diagnostics.

All nine text/choice operations refuse before publication: unchanged semantic project state, no lock, parent/base/extra slots or effects log; authored project and complete registry bytes remain unchanged. Coordination paths are printed separately; semantic snapshots deliberately exclude that namespace. An additional fresh accepted Integer(0) control installs, projects through selected_parameters and cold replays unchanged. No text/choice success or projection support is inferred from that scalar control.

## Missing owner and bounded recommendation

Valid present text/choice inputs and missing values first reach `PackageConfiguration::evaluate_parts` in `crates/vibe-core/src/manifest/configuration/package.rs`, with exact public refusal `only explicitly supplied required public bool parameters without defaults are connected`. This is an unsupported-operation result, not typed missing-value delivery for these kinds. The accepted integer branch is separate and does not remove this refusal.

Shared normalization already validates literal string and declared choice values. The smallest proposed delivery extends the existing ordinary parameter agreement/projection owner for one required public string or choice without default/profile, retaining literal spelling and actual consumer maps, distinct missing diagnostics and actual old reader/writer evidence. Do not create another choice kernel, coerce labels, or extend text to secret/path handling. Separate architectural contract and acceptance required.

## Evidence

Selector: `default_review::public_text_and_choice_parameters_literal_values_qualification`. Initial nine-world plus integer-control run PASS; independent architect run also passed that exact pair. Final assertions pin wrong-type, unknown-value and display-label reasons separately instead of merely parameter path. Exact final two-file source/CLI/fixture and runtime/lint/format hashes are sealed in the candidate manifest; independent acceptance required.

Finite python3 -I bwrap runtime clears environment and isolates filesystem/process/network with synthetic HOME/settings and exact read-only current/test/system artifacts. No host homes/auth stores/credentials/sockets/network/provider inference. No NST/all-kind/USER-PROJECT/REMAINDER/IDE completion follows from conservative refusals.
