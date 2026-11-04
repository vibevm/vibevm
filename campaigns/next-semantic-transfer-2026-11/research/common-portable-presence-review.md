# Portable presence review at accepted c5662ee42

**Recommendation: a new, closed, typed portable representation with field omission meaning `None` is sufficient for the inspected minimal transfer state. Explicit presence tags are not inherently required.** My earlier recommendation “TOML has no null, therefore tag presence” was too broad. It identified a real incompatibility with reusing canonical required-nullable readers unchanged, but not an unavoidable semantic need for another tag on every optional value.

This is read-only design evidence at immutable MAIN `c5662ee420973d006d0b2f019496f58b65eb28a8`, not implementation, chosen protocol/API or executed JSON/TOML proof. Current accepted F12 includes the proxy amendment; the removed candidate was not opened. Source captures and SHA256 ledger are under `/tmp/nst-portable-presence-capture`; structured conclusions are in the companion JSON. Standing 90-user/PROP-044/PROP-069 bytes match the previously fully read copies. The two prior design reports were reused, including root's appended closure/path/completion rulings; that audit was not repeated.

## Actual codec boundary

Current `portable.rs::parse_portable` decodes apps/1 directly into generated `AppsManifest`, using JSON when trimmed text starts with `{`, otherwise TOML, then checks the exact protocol. `portable_toml` calls `toml::to_string`. CLI export serializes the same typed set as JSON or TOML. Current rich export/import still refuses unrepresentable configuration; existing support is not a proof that a rich record can be embedded unchanged.

Canonical common wire intentionally differs from authored TOML:

- Generated `ConfigurationSelection.members` has `#[serde(deserialize_with = "...required_nullable::deserialize")]` and no missing-field default. It is `Option<Vec<String>>` in memory. Canonical JSON `{intent:"exact",members:null}` and `{intent:"exact",members:[]}` are distinct; missing `members` is rejected by the existing vocabulary test source.
- Authored `vibe_core::manifest::ConfigurationSelection.members` is the **same semantic `Option<Vec<String>>`**, but has `#[serde(default, skip_serializing_if = "Option::is_none")]`. Thus the product already has a concrete omission=None representation. `ConfigurationGroupDecl.defaults/fallback` use that same omission treatment.
- `vocabulary::read_selection`, `read_input` and `definition::read_definition` transfer typed values directly, without serializer roundtrips. They are the semantic conversion owners to reuse.

A new portable format may intentionally use authored-style omission while leaving the current input/record JSON grammar, generated readers and required-nullable tests untouched. Passing TOML directly to unchanged canonical types cannot be assumed to roundtrip: an omitted required-nullable field is rejected on read, irrespective of whether a particular TOML serializer omits it or errors on serialization. No serializer behavior was executed here.

## Exact nullable paths in transferable state

Paths below use the existing `ConfigurationRecord` layout as the reference, not a proposed new envelope. `{id}` denotes one map entry. All nullable collections here are `Option<Vec<String>>`, not nullable list elements.

| Existing path | Semantic alternatives | New portable omission rule |
|---|---|---|
| `evaluation.input.intent.selections.{id}.members` | `None` = omitted answer **inside a present selection object**; `Some([])` = explicit empty; `Some(ids)` = explicit members | Omit only `members` for None; keep the selection object and its required `intent`. Emit `members=[]` for Some(empty). |
| `definition.profiles[].selections.{id}.members` | Same distinction for an authored profile answer | Same local field rule; never erase a profile's selection entry because members is None. |
| `definition.groups[mode=choice].defaults` | No declared default versus declared empty/default members | Omit None; emit empty/nonempty array when Some. Both are separately framed by the semantic fingerprint. |
| `definition.groups[mode=closure].fallback` | No fallback versus explicit empty/nonempty fallback set | Same. **Fallback is an optional array, not a scalar ID.** |
| `definition.parameters[].default` | No public authored default versus a tagged bool/integer/text value | Omit None; retain the complete tagged value, including false, zero or empty string. |
| `evaluation.parameters[privacy=public].source` | No source value versus typed lexical authored source | Omit None inside the retained parameter record. Preserve id/type/origin and all present values. |
| `evaluation.parameters[privacy=public].value` | No realization versus realized value | Usually exclude target-local realization from the authoritative portable state; if source realization is included diagnostically, omission=None is representable. It must not become target execution authority. |

There is no separate valid semantic “field absent” state in those Option fields once decoded. Required-nullable JSON uses presence as an **admission requirement**, not as a third in-memory meaning alongside None and Some. Changing only the new portable admission to omission=None does not collapse those semantic alternatives.

Other optional fields in transferable source/definition data already have ordinary Option semantics: `input.intent.profile`; feature `description` and `parent`; group `description`; parameter `description`, `min`, `max`; profile `description`. Their absence can remain None. Do not conflate absent profile with whichever profile is marked `default`: the declaration's default is UI preselection, and `normalize_input` selects a profile only from `input.profile`.

Nullable/optional positions in **derived evaluation**, generally not needed in the minimal transfer projection:

- `evaluation.normalizedConsumers[].answers.{id}.members`: same selection representation, but normalized profile-overlay consumers are recomputed from original, not substituted for it.
- `evaluation.controls[].selection`: nullable projected selection; recompute controls/active/dormant presentation on B.
- `evaluation.problems[kind=feature].reason`: nullable diagnostic reason; not an authoritative transfer field.
- Optional `evaluation.resolved` and `featureResult`, plus nested diagnostics/review copies of definitions: do not export them as target readiness. Ready source records require resolved and empty questions/problems, but B needs fresh evaluation.

No inspected transferable collection has nullable **elements**, no transfer map has a direct nullable **value** requiring a placeholder, and no inspected field is `Option<Option<T>>`. Existing maps hold nonnullable typed values or selection objects whose own members may be absent. This is why omission of a field within a retained object works without introducing another union.

## Counterexamples and proposed spellings

These snippets describe proposed new portable semantics only; they are not current canonical JSON or executed test results.

### Parent-map presence must survive

`normalize_input` clones the selected profile's selections, then inserts **every explicit input selection entry**, even one with members=None. Suppose profile `p` supplies group `mode` = `["a"]`:

| Original input | Target normalization meaning |
|---|---|
| No `selections.mode` entry | Retain profile's answer `["a"]`. |
| Present `mode={intent="exact"}` | Override profile with an omitted answer. Defaults/questions now follow policy; do not silently restore profile answer. |
| Present `mode={intent="exact",members=[]}` | Explicit empty answer. Cardinality may accept it or produce a problem; it is not omission. |
| Present `mode={intent="additive",members=[]}` | An explicit additive contribution, still distinct from exact and from omitted intent. |

Portable JSON for the second and third cases could be:

```json
{"selections":{"mode":{"intent":"exact"}}}
{"selections":{"mode":{"intent":"exact","members":[]}}}
```

Corresponding TOML fragments:

```toml
selections = { mode = { intent = "exact" } }
```

```toml
selections = { mode = { intent = "exact", members = [] } }
```

Both avoid null while retaining all necessary states. A normalizer that removes the whole `mode` entry, invents one for every declared group, or fills members from profile/defaults is wrong. Do not drop an inactive group's original selection because the current projection marks it dormant; it may become active later.

### Empty defaults and values must survive

For a group allowing an empty selection, no `defaults` and `defaults=[]` are different retained definition meanings; `semantic_fingerprint` encodes an optional marker before the set. `fallback=[]` must likewise survive. A general “remove empty fields” pass would lose this distinction even though a correct Option::is_none omission rule does not.

A parameter with no default differs from `{kind="text",value=""}`, `{kind="bool",value=false}` and `{kind="integer",value="0"}`. Keep the value discriminator and mandatory `value`. Omitting an entire typed value or replacing it with `{}` is not valid None encoding. The `origin={kind="absent"}` and `origin={kind="default"}` objects are real distinct union variants, not optional objects to prune because they have no payload fields.

### No unavoidable three-state field was found

I found **no current minimal full-snapshot field** for which explicit presence tags are necessary to encode all legitimate semantics. A future patch API with “missing = leave unchanged, null = clear, value = set” would genuinely need three states in TOML, such as a tagged edit action. That is not the current complete original-input snapshot contract and must not be invented to justify wrappers here. Missing required root/basis/definition/input objects should be rejected, not treated as None or default-created.

## Scalar, collection and union fidelity

- Preserve `policy`, `defaultFeatures`, `features`, `disabledFeatures`, `selections` and public `parameters` as the original request. Do not infer `defaultFeatures` from the effective feature set. The canonical input makes these containers/booleans required; the new portable representation can keep them required even when empty, avoiding accidental default insertion.
- Preserve declared features/groups/profiles/parameters/config-values and their required arrays/maps. TOML supports empty arrays and empty tables; use an explicit empty map/table where required. Group keys containing dots or punctuation must be quoted, not reinterpreted as nested TOML keys. Do not discard array ordering or set-normalize transport merely because a fingerprint later sorts a semantic set.
- Reuse existing closed discriminators: value `kind=bool|integer|text`, group `mode=choice|closure`, origin `kind=explicit|profile|default|absent|legacy-effective`, privacy and effects/guards. TOML tables/arrays of tables can carry these fields. No arbitrary JSON-in-string or untyped map escape is needed.
- All signed i64 **value** payloads remain canonical decimal strings in `ConfigurationValueInteger.value`; parameter `min`/`max` are optional canonical strings too. `vocabulary::integer` parses i64 then checks exact `to_string` equality, rejecting overflow, `-0`, `+1`, leading zeros and whitespace. Preserve `"-9223372036854775808"`/`"9223372036854775807"` as strings across JSON and TOML. Do not blindly serialize the authored `ParameterValue::Integer` as a JSON number merely to reuse its serde implementation.
- Path lexical source is the typed text value plus parameter type/origin. Preserve its exact decoded string through TOML escaping; do not normalize separators, absolutize, expand environment variables or resolve against importer cwd during transport. A source-host absolute realization is optional historical context, never a requirement that A's path exists on B. Existing record admission uses host-native absolute-path checks, so importing a portable historical basis is not identical to loading a ready local record.

## Small historical state that must travel

The evaluator/record readers identify the needed state directly:

| Must retain as historical transfer evidence | Why current consumers need it |
|---|---|
| Complete original `evaluation.input` | `input::edit` reuses it during maintenance; normalization overlays explicit entries on a profile. Preserve dormant, omitted/empty, exact/additive, disabled and policy distinctions. |
| Typed historical `definition` plus its semantic fingerprint (`evaluation.definition`) | `record::validate` reconstructs definition and checks the fingerprint; `evaluate` compares old/current basis; review presents changed typed rows. Keep a full typed snapshot as the minimal safe existing-owner reuse unless a narrower projection is separately proved sufficient. Cosmetic labels need not enter the hash but remain useful for review. |
| Prior `evaluation.reliance` | `evaluate` compares new reliance with old and deliberately retains certain historical automatic dependencies until explicit review/replacement. It is not safe to assume current default-use output alone reproduces that history. |
| Public parameter id/type/origin/lexical `source` | `parameters::resolve` distinguishes Explicit/Profile/Default/Absent and uses source+origin equality for path realization reuse. Do not turn a target rebind into an authored Explicit override. |
| Secret id and value-free origin/previous explicit provision | `needs_resupply = !replacement && old.provided`. Even an optional previously explicit secret must be supplied again when consumed on B. Installed common records validate `provided` against Explicit origin and require `resupply=false`; that stored false is **not** permission to skip target resupply. Derive B's current requirement rather than copying a stale boolean. Legacy-effective uncertainty stays explicit in the legacy transport branch. |
| Exact selected source package/option context already required by the parent design | Historical basis belongs to that source selection; target option reassessment is separate. This review does not redesign dependency or cohort identity. |

Transport should omit runtime secret values/hashes. Authored public default literals remain part of the public typed definition, including the existing public secret-default declaration model; do not erase them as if they were ephemeral answers.

Recompute on B: normalized consumers, active controls/projections, effective default-use rows, implied feature closure, current questions/problems, readiness/resolved/data, canonical host realizations and secret-resupply questions. Historical effective result may be optional read-only review context but cannot authorize B. Do not call the existing evaluator with `replacement=true` merely because the imported file contains complete input: replacement can bind current defaults directly, whereas transfer must compare historical basis and reliance. Likewise `reviewed=true` must come from the target review, not a portable boolean claiming prior acceptance.

There is one concrete consumer-boundary change to plan if the portable projection is minimal: the current evaluator takes `previous: Option<&ConfigurationRecord>`, while that local record requires a ready `resolved` result. Do not manufacture dummy resolved/current-control fields just to fit that signature. Expose a narrow borrowed historical-basis view of the fields actually consumed (definition/hash, original, reliance and parameter history), with normal local records providing that view too, or explicitly retain a separately validated richer historical projection. This is a bounded evaluator adapter decision, not a second semantic model or serializer framework. Old feature-result text used by `review::summary` can be retained as diagnostic context or recomputed using the pure historical feature/choice model; do not rerun source-host path existence checks to obtain it.

`record::validate` is structural admission, not a full replay of all historical evaluation. It verifies definition/fingerprint, original normalization, ready/result consistency and parameter metadata; it does not turn an untrusted imported record into authenticated history or target readiness. The transport should validate the historical projection against its own declared invariants, then use the normal evaluator for target decisions.

## Concrete approaches

| Approach | Reuse and cost | Assessment |
|---|---|---|
| Explicit tagged presence at every nullable field | Extra `omitted`/`present` types and converters around existing Option fields; canonical JSON stays unchanged | Sound if specified, but unnecessary for inspected full-snapshot semantics. Use only when a real third state or nullable element/map-value must be represented. |
| **New typed portable omission representation** | Reuse primitive enums, typed value/origin/effect/feature structures; adapt Selection, group defaults/fallback, parameter default and optional historical source. Containers referencing adapted types need small typed mirrors/conversions. | Recommended. Separate wire grammar, same domain meaning. No generic serializer framework or alternate evaluator. Required containers/discriminators stay required; skip only None. |
| Narrow documented normalization into canonical types | An exact allowlist can insert null at the known required-nullable field positions **inside already present correctly typed objects**, then use canonical strict admission | Possible but easier to get wrong: must never create missing map entries/parent objects, traverse arbitrary extension objects or erase empty values. Generic serde_json::Value acceptance/string payload is not the contract. A typed adapter is clearer; normalization is only justified if it reduces real duplication and has the same closed validation boundary. |

Reuse canonical primitive types directly where their serde shape already fits TOML. Do not reuse entire `ConfigurationDefinition/Profile/Input` containers unchanged if they recursively reference canonical required-nullable selections; their nested reader still rejects omission. Conversely, do not duplicate the semantic model: explicit From/TryFrom adapters can construct the existing typed common values and invoke `read_input`/`read_definition` and the existing validators. Reusing the authored Rust structs as an intermediate is legitimate, but reusing their entire on-wire serde dialect would also change naming, defaults, enum encodings and integer representation; that is not a free drop-in replacement for the proposed canonical-shaped portable projection.

For JSON in the new format, decide explicitly whether literal null is a None alias or rejected. Omission is sufficient either way; there is no semantic need to distinguish the two. Ordinary Serde `Option<T>` accepts missing and often explicit null, so **do not promise null rejection merely by moving a field to optionalProperties or adding a default**. The chosen schema and actual reader must agree. Permitting null as a documented JSON-only alias while always exporting omission avoids inventing a third state; stricter rejection would need a concrete admission check. This does not change existing canonical reader behavior.

## Validation boundary and focused discriminators

Recommended boundary: version-dispatch the new portable envelope; decode its closed typed shape; map only local omission fields into existing Option values; validate original and historical-definition references/fingerprint/parameter classification; retain old basis/reliance; evaluate target selection with target path binding and ephemeral secrets; persist the existing normal target record only after resolution/review. Unknown fields/variants, absent required parents and missing union payloads refuse. No current format number or CLI spelling is assigned here.

Before implementation acceptance, test proposed JSON and TOML equivalence for: absent group key versus present omitted members with a profile; exact/additive empty and nonempty answers; dormant selections later activated; absent versus empty defaults/fallback; absent versus false/zero/empty-string defaults and source values; optional profile and origin variants; i64 boundaries/invalid spellings; quoted map keys; lexical paths with backslashes; and changed definition/new reliance on import requiring review. Also prove canonical input still rejects missing required-nullable `members`, and portable decoding does not invent it for a missing selection object. These are proposed discriminators, not executed PASS results.

Root still chooses envelope/API, exact minimal history projection and JSON-null alias policy. The semantic evidence supports omission-based typed transport without weakening current canonical JSON or adding a general serialization framework.
