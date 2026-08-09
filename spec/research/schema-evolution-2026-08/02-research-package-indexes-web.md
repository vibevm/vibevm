# Research A — How package-index / repository-metadata formats evolve their schema

**Scope:** data AT REST (files published and read by third-party tools), not an RPC wire.
**All access dates: 2026-08-09** unless stated otherwise.
**Evidence rule used:** every claim carries a verbatim quote + URL. Where no authoritative
answer was found, the entry says **NOT FOUND** and states what was searched. Nothing is
filled in from general knowledge.

---

# §1 Per-subject findings

## 1. crates.io sparse index (Cargo registry index format)

Primary sources used (raw where possible):
- The Cargo Book, *Registry Index* — <https://doc.rust-lang.org/cargo/reference/registry-index.html>
- `cargo-util-schemas` — the authoritative reader/writer struct:
  <https://raw.githubusercontent.com/rust-lang/cargo/master/crates/cargo-util-schemas/src/index.rs>
- RFC 3143 (weak/namespaced features — the RFC that introduced `features2` **and** justified `v`):
  <https://raw.githubusercontent.com/rust-lang/rfcs/master/text/3143-cargo-weak-namespaced-features.md>
- RFC 2789 (sparse index): <https://raw.githubusercontent.com/rust-lang/rfcs/master/text/2789-sparse-index.md>
- PR #9161 *Add schema field and `features2` to the index* — <https://github.com/rust-lang/cargo/pull/9161>

### Q1 — Tagged vs untagged unions

There is **no general discriminator mechanism**. Variant-ish fields are stringly-typed
tags read leniently. The dependency kind is a plain optional string:

> `/// The dependency kind. "dev", "build", and "normal".`
> `pub kind: Option<Cow<'a, str>>,`

— `crates/cargo-util-schemas/src/index.rs`, accessed 2026-08-09.

The book states the same as a defaulting rule, not a closed enum:

> "The dependency kind.
> "dev", "build", or "normal".
> If not specified or `null`, it defaults to "normal"."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

**The recorded trouble case is `features2`, which is precisely an untagged-variant
problem.** The `features` map's *values* silently gained new syntax (`dep:`, `pkg?/feat`)
with no tag, so old readers mis-parsed them. The fix was to move the new-shaped values
into a **separate field** and add a **schema tag** (`v`):

> "This is separated from `features` because versions older than 1.19
> will fail to load due to not being able to parse the new syntax, even
> with a `Cargo.lock` file."

— `crates/cargo-util-schemas/src/index.rs`, accessed 2026-08-09.

This is the single most instructive data point in the whole subject: **an in-place
widening of a value's grammar, without a tag, was a breaking change; the repair was to
split the field and version the entry.**

### Q2 — Absent vs empty vs null

Documented per-field defaults, and — notably — the defaults were **added to the docs only
in Cargo 1.84**, i.e. the rule was under-specified for years:

> "Array of features (as strings) enabled for this dependency.
> Since Cargo 1.84, defaults to `[]` if not specified."

> "Set of features defined for the package.
> Each feature maps to an array of features or dependencies it enables.
> Since Cargo 1.84, defaults to `{}` if not specified."

> "Boolean of whether or not this is an optional dependency.
> Since Cargo 1.84, defaults to `false` if not specified."

> "Boolean of whether or not default features are enabled.
> Since Cargo 1.84, defaults to `true` if not specified."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

`null` is explicitly given the *same* meaning as absent, per field:

> "The target platform for the dependency.
> If not specified or `null`, it is not a target dependency."

> "The URL of the index of the registry where this dependency is
> from as a string. If not specified or `null`, it is assumed the
> dependency is in the current registry."

— same page.

So: **absent == null == the documented default. "Empty" and "not recorded" are NOT
distinguished** in this format. The reader enforces it with serde defaults:

> `#[serde(default)]`
> `pub features: BTreeMap<Cow<'a, str>, Vec<Cow<'a, str>>>,`

> `#[serde(default = "default_true")]`
> `pub default_features: bool,`

— `crates/cargo-util-schemas/src/index.rs`, accessed 2026-08-09.

One field carries the archaeology of an absent-vs-empty mistake being tolerated forever:

> `/// If `true`, Cargo will skip this version when resolving.`
> `///`
> `/// This was added in 2014. Everything in the crates.io index has this set`
> `/// now, so this probably doesn't need to be an option anymore.`
> `pub yanked: Option<bool>,`

— same file, accessed 2026-08-09. The field is *de facto* always present but remains
`Option` because the type can never be narrowed once published.

### Q3 — Closed vocabularies

**NOT FOUND**: any documented "unknown value" fallback or extension convention for enum-like
values in the crates.io index. Searched: the Cargo Book registry-index page (full text,
keywords `unknown`, `reserved`, `extension`), `cargo-util-schemas/src/index.rs`, RFC 2789,
RFC 3143. What exists instead is that enum-ish values are **not modelled as enums at all** —
`kind` is `Option<Cow<str>>`, so an unknown value survives deserialization. The format's
answer to "new kind of thing" is the `v` schema counter (Q5), not per-value tolerance.

### Q4 — Strictness (ignore vs reject unknown fields)

**There is no normative statement in the spec.** The rule is *implementation-defined and
lenient*: the reader struct carries **no `deny_unknown_fields`**, so serde's default —
ignore unknown keys — applies.

Verified mechanically: `grep -c "deny_unknown_fields" crates/cargo-util-schemas/src/index.rs`
→ `0` (accessed 2026-08-09).

The nearest thing to a normative statement is the forward-compatibility paragraph that
opens the format definition:

> "New features are occasionally added, which are only understood starting with the version
> of Cargo that introduced them. Older versions of Cargo may not be able to use packages
> that make use of new features. However, the format for older packages should not change,
> so older versions of Cargo should be able to use them."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

Asymmetry: **writers are constrained, readers are not.** See Q7.

### Q5 — Version field semantics — **the `v` field**

This is the best-documented reader contract found in any subject *for at-rest data*.

> "An unsigned 32-bit integer value indicating the schema version of this entry.
>
> If this is not specified, it should be interpreted as the default of 1.
>
> Cargo (starting with version 1.51) will ignore versions it does not
> recognize. This provides a method to safely introduce changes to index
> entries and allow older versions of cargo to ignore newer entries it
> doesn't understand. Versions older than 1.51 ignore this field, and
> thus may misinterpret the meaning of the index entry.
>
> The current values are:
>
> * 1: The schema as documented here, not including newer additions.
>      This is honored in Rust version 1.51 and newer.
> * 2: The addition of the `features2` field.
>      This is honored in Rust version 1.60 and newer."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

The reader contract is therefore: **higher-than-known `v` ⇒ skip THIS ENTRY (this
package version), not the file, and not an error.** The unit of version-gating is the
*record*, not the document. This is the key structural choice, and it is enabled by the
physical layout — the index is newline-delimited JSON, one record per line:

> "The rest of the index repository contains one file for each package, where the filename is
> the name of the package in lowercase. Each version of the package has a separate line in the
> file."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

NDJSON + per-record version means an unparseable or too-new record costs you exactly that
one package *version*, while every other version of the same package remains resolvable.
A single JSON document with a document-level version would make the same change cost the
whole package.

The source adds a **v3** not yet in the book, and is franker about the cost:

> `/// The schema version for this entry.`
> `///`
> `/// If this is None, it defaults to version `1`. Entries with unknown`
> `/// versions are ignored.`
> `///`
> `/// Version `2` schema adds the `features2` field.`
> `///`
> `/// Version `3` schema adds `artifact`, `bindep_targes`, and `lib` for`
> `/// artifact dependencies support.`
> `///`
> `/// This provides a method to safely introduce changes to index entries`
> `/// and allow older versions of cargo to ignore newer entries it doesn't`
> `/// understand. This is honored as of 1.51, so unfortunately older`
> `/// versions will ignore it, and potentially misinterpret version 2 and`
> `/// newer entries.`
> `///`
> `/// The intent is that versions older than 1.51 will work with a`
> `/// pre-existing `Cargo.lock`, but they may not correctly process `cargo`
> `/// update` or build a lock from scratch. In that case, cargo may`
> `/// incorrectly select a new package that uses a new index schema. A`
> `/// workaround is to downgrade any packages that are incompatible with the`
> `/// `--precise` flag of `cargo update`.`

— `crates/cargo-util-schemas/src/index.rs`, accessed 2026-08-09.
(Note `bindep_targes` — a typo preserved in the shipped doc comment.)

**No major/minor split.** `v` is a single monotonic integer.

**Crucially: adding a field does NOT bump `v`.** `links` (2018), `rust_version` (2023) and
`pubtime` (2025) were all added as plain optional fields with **no `v` bump**; only changes
that would make an old reader *misinterpret existing data* got a bump:

> `/// Native library name this package links to.`
> `///`
> `/// Added early 2018 (see <https://github.com/rust-lang/cargo/pull/4978>),`
> `/// can be `None` if published before then.`
> `pub links: Option<Cow<'a, str>>,`

> `/// Corresponds to `package.rust-version`.`
> `///`
> `/// Added in 2023 (see <https://github.com/rust-lang/crates.io/pull/6267>),`
> `/// can be `None` if published before then or if not set in the manifest.`
> `pub rust_version: Option<RustVersion>,`

— same file, accessed 2026-08-09.

The RFC states the design intent explicitly:

> "The version field is added to help prevent older versions of Cargo from updating to
> newer versions of package that it doesn't understand.
> Cargo, since 1.51, already supports the `"v"` field, and will ignore any entries with a
> `"v"` value greater than 1."

— RFC 3143, accessed 2026-08-09.

And the PR that introduced it:

> "This adds a v field to the index which indicates a format version for an index entry. If
> Cargo encounters a version newer than it understands, it will ignore those entries. This
> makes it safer to make changes to the index entries (such as adding new things), and not
> need to worry about how older cargos will react to it. In particular, this will make it
> safer to run cargo update on older versions if we ever decide to add new things to the
> index. Currently this is not written anywhere, and is intended as a safety guard for the
> future. For now I will leave it undocumented until we actually decide to start using it."

— ehuss, PR #9161 body, commented 2021-02-10, merged by bors 2021-02-22T16:26:28Z;
<https://github.com/rust-lang/cargo/pull/9161>, accessed 2026-08-09. (Verified verbatim
against the raw page HTML, not via summarisation.)

**This is the most actionable sentence in the entire research.** The version field was
shipped **undocumented and unused, purely as a forward-compatibility guard**, four years
(v1 → the first real use at v2) before it was needed. That ordering is forced: a version
gate only protects you if it is already deployed in readers *before* the first change that
needs it. Cargo's own docs record what it costs when it is not — "Versions older than 1.51
ignore this field, and thus may misinterpret the meaning of the index entry." The readers
that predate the guard can never be protected retroactively.

**`config.json` has no version field at all** — its documented keys are only `dl`, `api`,
`auth-required` (<https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed
2026-08-09). The version discipline was applied to records, never to the index-level config.

### Q6 — Field identity and reuse

Fields are identified by JSON key name. **NOT FOUND**: any reserved-name list, or any stated
policy on renaming/removing/reusing a field name, in the Cargo Book or `cargo-util-schemas`.
Searched the registry-index page full text for `reserved`, `rename`, `remove`, `deprecat`;
the only `reserved` hit concerns *package names* ("Rejects reserved names, such as Windows
special filenames like 'nul'"), not fields.

What is documented instead is that **the same concept carries different names in three
sibling formats**, and the doc warns readers about it:

> "Note: The index JSON format has subtle differences from the JSON format of the Publish API
> and cargo metadata.
> If you are using one of those as a source to generate index entries, you are encouraged to
> carefully inspect the documentation differences between them."

> "`req` — The Publish API field is called `version_req`."
> "`vers` — The cargo metadata field is called `version`."
> "`default_features` — The cargo metadata field is called `uses_default_features`."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

Worse, the *same* field name means different things across the three:

> "`registry` — cargo metadata uses a value of null to indicate that the dependency comes
> from crates.io. The index uses a value of null to indicate that the dependency comes from
> the same registry as the index."

> "`name` — When the dependency is renamed in Cargo.toml, the publish API puts the original
> package name in the `name` field and the aliased name in the `explicit_name_in_toml` field.
> The index places the aliased name in the `name` field, and the original package name in the
> `package` field."

— same page. This is an *observed* cost of name reuse across formats, documented as a
hazard rather than prevented by policy.

### Q7 — Round-trip preservation

The rule is stated for writers, and it is the strongest immutability rule found anywhere
in this research:

> "The JSON objects should not be modified after they are added except for the `yanked`
> field whose value may change at any time."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

That is not "preserve unknown fields on rewrite" — it is "**do not rewrite at all**". The
index is append-only per record, with exactly one mutable field. Round-trip preservation is
achieved by forbidding round-trips.

### Q8 — Migration mechanism

Policy is **never break, only add**, plus per-record version-gating for the cases where
adding is not enough:

> "However, the format for older packages should not change, so older versions of Cargo
> should be able to use them."

— <https://doc.rust-lang.org/cargo/reference/registry-index.html>, accessed 2026-08-09.

The *transport* migration (git index → sparse HTTP index) was handled entirely outside the
document schema, by a URL-scheme change (`sparse+https://`) — the per-file JSON payload was
kept identical. RFC 2789, accessed 2026-08-09.

**Regret note on the mechanism itself:** it only works for readers ≥ 1.51, and the design
document says so plainly — see §5.

---

## 2. OCI Image Spec (descriptors, `mediaType`, `schemaVersion`)

Primary sources (raw markdown from the spec repo):
- `descriptor.md`, `manifest.md`, `image-index.md`, `spec.md`, `considerations.md` at
  <https://github.com/opencontainers/image-spec> (`main`, and tags `v1.0.0`, `v1.0.2`,
  `v1.1.0`, `v1.1.1`), all accessed 2026-08-09.
- OCI blog, *OCI Image and Distribution Specs v1.1 Releases*, published 2024-03-13 —
  <https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/>

### Q1 — Tagged vs untagged unions

**OCI is the strongest "always tag" case found.** Every reference to external content is a
descriptor, and the type tag is REQUIRED on the descriptor:

> "- **`mediaType`** *string*
>
>   This REQUIRED property contains the media type of the referenced content.
>   Values MUST comply with [RFC 6838][rfc6838], including the [naming requirements in its
>   section 4.2][rfc6838-s4.2]."

— `descriptor.md`, accessed 2026-08-09.

And descriptors are designed so that sibling entries may legitimately differ in type:

> "When other formats contain multiple descriptors, unless otherwise specified, those
> descriptors are independent of each other, allowing fields like the `mediaType` and the
> algorithm for the `digest` to vary within that external content."

— `descriptor.md`, accessed 2026-08-09.

**Why mandatory:** so a reader can decide whether to parse *before* fetching, and can refuse
to parse safely:

> "Implementations MUST NOT attempt to parse the referenced content if this media type is
> unknown and instead consider the referenced content as arbitrary binary data (e.g.: as
> `application/octet-stream`)."

— `manifest.md`, accessed 2026-08-09.

**The recorded trouble case is the in-band tag being optional.** In the *manifest itself*
(as opposed to the descriptor pointing at it), `mediaType` is only SHOULD:

> "- **`mediaType`** *string*
>
>   This property SHOULD be used and [remain compatible](media-types.md#compatibility-matrix)
>   with earlier versions of this specification and with other similar external formats.
>   When used, this field MUST contain the media type `application/vnd.oci.image.manifest.v1+json`.
>   This field usage differs from the [descriptor](descriptor.md#properties) use of `mediaType`."

— `manifest.md` (`main`), accessed 2026-08-09.

**That wording is itself the result of a reversal.** At tags `v1.0.0` and `v1.0.1` the field
was not recommended at all — it was merely parked:

> "- **`mediaType`** *string*
>
>   This property is *reserved* for use, to [maintain compatibility](media-types.md#compatibility-matrix).
>   When used, this field contains the media type of this document, which differs from the
>   [descriptor](descriptor.md#properties) use of `mediaType`."

— `manifest.md` @ `v1.0.0` and @ `v1.0.1`, both verified 2026-08-09. The promotion from
*reserved* to *SHOULD be used* landed in `v1.0.2` (verified by diffing the three tags on
2026-08-09).

So OCI ended up with a self-describing tag at the *reference* site (mandatory from the start)
and a tag *inside* the document that took three point releases to go from "reserved" to
"recommended" — and still is not required. Readers that fetch a bare document without its
descriptor have to sniff. The lesson is directional: **the in-band type tag kept getting
stronger, never weaker.**

### Q2 — Absent vs empty vs null

OCI's answer is unusual and directly relevant: rather than allow absent, it **invented an
explicit empty value** so that a required field can be present-but-meaningless:

> "In the scenarios were an artifact does not have content for the config or any layers, an
> empty JSON descriptor is recommended as a placeholder to satisfy the requirement that these
> fields exist, while serving as an indication to consuming tools that there is no useful
> content in that descriptor."

— OCI blog 2024-03-13, accessed 2026-08-09. (The sentinel is
`application/vnd.oci.empty.v1+json`, a descriptor whose content is the two bytes `{}`.)

That is: **"empty" was made a first-class tagged value rather than an absence**, precisely
because absence would have been ambiguous to older readers that require the field.

`descriptor.md` also constrains when a reader may *add* data:

> "Implementations MUST NOT populate the `data` field in situations where doing so would
> modify existing content identifiers.
> For example, a registry MUST NOT arbitrarily populate `data` fields within uploaded
> manifests, as that would modify the content identifier of those manifests."

— `descriptor.md`, accessed 2026-08-09.

For the one open-ended map in the format, the rule **is** stated normatively — and it
explicitly makes absent and empty *equivalent*, permitting either:

> "- If there are no annotations then this property MUST either be absent or be an empty map."

> "- While the value MUST be present, it MAY be an empty string."

— `annotations.md`, accessed 2026-08-09.

So OCI's answer to Q2 is: **do not distinguish absent from empty — mandate that they mean
the same thing**, and let writers choose. There is no "unknown/not recorded" third state.

**NOT FOUND**: a general normative rule in image-spec on omitting vs emitting `[]`/`{}` for
*array* fields such as `layers`. Searched `descriptor.md`, `manifest.md`, `image-index.md`,
`considerations.md` for `empty`, `omit`, `null`.

### Q3 — Closed vocabularies

OCI states an **unknown-value tolerance rule repeatedly and normatively**, at every place a
vocabulary appears. This is the densest such evidence in the whole research.

For layer media types:

> "Implementations storing or copying image manifests MUST NOT error on encountering a
> `mediaType` that is unknown to the implementation."

For config media types:

> "Implementations storing or copying image manifests MUST NOT error on encountering a value
> that is unknown to the implementation."

For artifact types:

> "Implementations storing or copying image manifests MUST NOT error on encountering an
> `artifactType` that is unknown to the implementation."

— all `manifest.md`, accessed 2026-08-09.

For index entries:

> "Future versions of the spec MAY use a different mediatype (i.e. a new versioned format).
> An encountered `mediaType` that is unknown to the implementation MUST NOT generate an error."

— `image-index.md`, accessed 2026-08-09.

**Checksum algorithms are the model case for extending a closed set.** The algorithm is a
prefix tag inside the value, the grammar is open, and unknown algorithms MUST pass syntactic
validation:

> "Implementations SHOULD allow digests with unrecognized algorithms to pass validation if
> they comply with the above grammar.
> While `sha256` will only use hex encoded digests, separators in _algorithm_ and
> alphanumerics in _encoded_ are included to allow for extensions.
> As an example, we can parameterize the encoding and algorithm as
> `multihash+base58:QmRZxt2b1FVZPNqd8hsiykDL3TdBDeTSPX9Kv46HmX4Gx8`, which would be
> considered valid but unregistered by this specification."

— `descriptor.md`, accessed 2026-08-09.

With a registry and a mandatory floor:

> "The following algorithm identifiers are currently defined by this specification:
> | `sha256` | [SHA-256](#sha-256) |
> | `sha512` | [SHA-512](#sha-512) |
> | `blake3`  | [BLAKE3](#blake3)   |
>
> If a useful algorithm is not included in the above table, it SHOULD be submitted to this
> specification for registration."

> "Implementations MUST implement SHA-256 digest verification for use in descriptors."

— `descriptor.md`, accessed 2026-08-09. (BLAKE3 is a *later addition* to that table — the
mechanism has actually been exercised.)

### Q4 — Strictness

Normative, and **it was WEAKENED between v1.0.2 and v1.1.0** — see §5.

Current (`main`, and v1.1.0 / v1.1.1):

> "Implementations processing content SHOULD NOT generate an error if they encounter an
> unknown property in a known media type."

— `considerations.md` (`main`), accessed 2026-08-09.

Former (identical text at tags `v1.0.0` and `v1.0.2`, verified 2026-08-09):

> "Implementations that are reading/processing [manifests](manifest.md) or
> [image indexes](image-index.md) MUST NOT generate an error if they encounter an unknown
> property.
> Instead they MUST ignore unknown properties."

— `considerations.md` @ `v1.0.2`.

**Asymmetric by role, not by writer/reader.** v1.1 split one rule into two, addressed at two
different kinds of implementation:

> "Implementations storing or copying content MUST NOT modify or alter the content in a way
> that would change the digest of the content. Examples of these implementations include:
>
> - A [registry implementing the distribution specification][distribution-spec], including
>   local registries, caching proxies
> - An application which copies content to disk or between registries
>
> Implementations processing content SHOULD NOT generate an error if they encounter an
> unknown property in a known media type. Examples of these implementations include:
>
> - A [runtime implementing the runtime specification][runtime-spec]
> - An implementation using OCI to retrieve and utilize artifacts, e.g.: a WASM runtime"

— `considerations.md` (`main`), accessed 2026-08-09.

Note the qualifier added in v1.1: "**in a known media type**". Tolerance of unknown
*properties* is conditional on recognising the *type*. That is the tag doing load-bearing
work: you only forgive what you can locate.

**And the tolerance rule is stronger for the designated extension namespace than for the
schema proper.** Unknown *annotation keys* are still a MUST NOT, never downgraded:

> "- Consumers MUST NOT generate an error if they encounter an unknown annotation key."

— `annotations.md`, accessed 2026-08-09.

Contrast with the schema-level rule quoted above, which is now only SHOULD NOT. OCI
therefore offers a **two-tier extensibility contract**: put speculative data in the
open, reverse-DNS-namespaced annotation map and you get a hard guarantee that readers
will not choke; invent a new top-level property and you get only a soft one.

Compliance is defined only over MUST-class terms:

> "An implementation is not compliant if it fails to satisfy one or more of the MUST, MUST NOT,
> REQUIRED, SHALL, or SHALL NOT requirements for the protocols it implements."

— `spec.md`, accessed 2026-08-09. Which means the v1.0→v1.1 downgrade to SHOULD NOT moved
unknown-property tolerance *out* of the compliance boundary.

### Q5 — Version field semantics — `schemaVersion` is vestigial

> "- **`schemaVersion`** *int*
>
>   This REQUIRED property specifies the image manifest schema version.
>   For this version of the specification, this MUST be `2` to ensure backward compatibility
>   with older versions of Docker. The value of this field will not change. This field MAY be
>   removed in a future version of the specification."

— `manifest.md` (`main`), accessed 2026-08-09; **the identical sentence is already present
at tag `v1.0.2`**, verified 2026-08-09.

This is the sharpest negative result in the research: OCI has a schema-version field, it is
REQUIRED, it is **frozen at a constant inherited from a predecessor format (Docker manifest
schema 2)**, it carries no reader contract at all, and the spec announces it may be deleted.
**Versioning is done by media type instead** — `application/vnd.oci.image.manifest.v1+json`
— i.e. the version lives in the *type tag*, not in a numeric field.

### Q6 — Field identity and reuse

Fields are identified by name; there is an explicit **Reserved** policy for extension fields:

> "### Reserved
>
> Extended _Descriptor_ field additions proposed in other OCI specifications SHOULD first be
> considered for addition into this specification."

— `descriptor.md`, accessed 2026-08-09.

Annotations are the sanctioned open namespace, and its naming policy is fully normative —
this is the most complete field-identity regime found in any subject:

> "- Keys MUST be unique within this map, and best practice is to namespace the keys.
> - Keys SHOULD be named using a reverse domain notation - e.g. `com.example.myKey`.
> - The prefix `org.opencontainers` is reserved for keys defined in Open Container Initiative
>   (OCI) specifications and MUST NOT be used by other specifications and extensions.
> - Keys using the `org.opencontainers.image` namespace are reserved for use in the OCI Image
>   Specification and MUST NOT be used by other specifications and extensions, including other
>   OCI specifications."

— `annotations.md`, accessed 2026-08-09. Note the second reservation: OCI reserved a
sub-namespace **against its own sibling specifications**, not only against third parties.

The rules are binding wherever annotations appear:

> "This OPTIONAL property MUST use the [annotation rules](annotations.md#rules)."

— `descriptor.md`, accessed 2026-08-09.

**NOT FOUND**: any policy explicitly forbidding renaming or re-purposing an existing *schema*
field name. Searched `descriptor.md`, `manifest.md`, `image-index.md`, `considerations.md`,
`spec.md` for `rename`, `reuse`, `repurpose`, `deprecat`.

### Q7 — Round-trip preservation — **mandatory, and enforced by hashing**

> "Implementations storing or copying content MUST NOT modify or alter the content in a way
> that would change the digest of the content."

— `considerations.md` (`main`), accessed 2026-08-09. **This clause did not exist in v1.0.2**
(verified by diffing the two tags on 2026-08-09) — it was added in v1.1.0.

The blog restates it as a design assumption of the whole migration:

> "To facilitate this transition period, changes to the image spec added new fields which
> should be ignored by registries without generating errors or modifying the content of the
> manifest."

— OCI blog 2024-03-13, accessed 2026-08-09.

Preservation is not merely required, it is **structurally unavoidable**: the document's
identity is its digest, so any dropped field yields a different object that no longer matches
the descriptor pointing at it. Content-addressing converts "please preserve unknown fields"
from a policy into a physical law.

### Q8 — Migration mechanism

Additive, with the media type as the escape hatch of last resort:

> "Future versions of the spec MAY use a different mediatype (i.e. a new versioned format)."

— `image-index.md`, accessed 2026-08-09.

And an out-of-band extension namespace on the distribution side:

> "Extensions to the registry API may be added by implementations without requiring a change
> to the OCI distribution specification.
> This allows development and testing of new features, in addition to custom features in a
> registry.
> Extensions are denoted by a leading `_` in their API path, which is invalid for a repository
> name."

> "To prevent conflicts, extensions should register their name with the distribution
> specification."

— OCI blog 2024-03-13, accessed 2026-08-09.

Plus an explicit deprecation *channel*:

> "Registries may now include a warning header in responses.
> This can be used to notify users of issues that are not yet error conditions, including
> deprecation announcements."

— OCI blog 2024-03-13, accessed 2026-08-09.

The v1.1 migration also shipped a **capability-probe + fallback** pattern rather than a
version bump, because old readers could not be upgraded in time:

> "Clients pushing a manifest with a subject field and querying for referrers have an
> additional responsibility to manage and use a fallback tagged Index when the registry does
> not support the referrers API.
> When a registry supports the referrers API, it will return the header OCI-Subject set to the
> digest in the subject field in response to a client pushing a manifest with the subject
> field. Similarly, when querying for referrers, a registry that supports the API will always
> return a 200 response, even if the response is an empty Index, to distinguish between
> registries that have not implemented the API and may respond with a 404 or similar error."

— OCI blog 2024-03-13, accessed 2026-08-09. And the honest cost accounting:

> "The client requirement to manage the fallback tag is subject to race conditions since the
> existing content of the fallback tag must first be queried to add the new entry. The
> fallback tags also clutter the tag listing with entries that are not the images end users
> are searching for."

— same source.


---

## 3. Debian `Packages` / deb822 control format

Primary sources: Debian Policy Manual **v4.7.4.1**
(<https://www.debian.org/doc/debian-policy/ch-controlfields.html>), `deb822(5)` and
`deb-control(5)` / `deb-src-control(5)` / `dsc(5)` / `deb-changes(5)` from **dpkg-dev 1.23.7**,
`sources.list(5)` from **apt 3.3.2**, the `DebianRepository/Format` wiki, and the dpkg and apt
sources on salsa.debian.org (`main`). All accessed 2026-08-09.

*(Fidelity note: a sample of these quotes — Policy §5.1 case-sensitivity, §5.1 empty fields,
§5.7 user-defined fields, §5.1 context-dependence — was independently re-fetched and matched
character-for-character on 2026-08-09.)*

### Q1 — Tagged vs untagged unions

**No per-field type tag. Variant selection is out-of-band — it depends on *which kind of
control file you are reading*.** This is stated normatively:

> "The presence and purpose of a field, and the syntax of its value, may differ between types
> of control files."

— Policy §5.1, <https://www.debian.org/doc/debian-policy/ch-controlfields.html>, accessed
2026-08-09 (verbatim in `deb822(5)` too).

The consequences are exactly the untagged-union failure mode. The *same field name* carries
different arity and different separators depending on context:

> "This field contains a list of files with information about each one. The exact information
> and syntax varies with the context." *(`Files` — 3 columns in `.dsc`, 5 in `.changes`)*

> "This folded field is a list of binary packages. Its syntax and meaning varies depending on
> the control file in which it appears." … "When it appears in a `.changes` file, it lists the
> names of the binary packages being uploaded, separated by whitespace (not commas)."

> "When used inside a `.changes` file, the `Description` field has a different format than in
> source or binary control files."

— Policy §5.6.21, §5.6.19, §5.6.13, accessed 2026-08-09.

And `Architecture` is a four-way untagged union discriminated only by inspecting the word:

> "A unique single word identifying a Debian machine architecture … An architecture wildcard
> identifying a set of Debian machine architectures … `all`, which indicates an
> architecture-independent package. … `source`, which indicates a source package."

— Policy §5.6.8, accessed 2026-08-09.

**Recorded trouble cases:**

*(a) `Format` collided with itself* — the same name means two unrelated things, and Policy had
to say so out loud when it finally documented both:

> "The `Format` field of `.changes` files is now 1.8.  The `Format` field syntax for source
> package `.dsc` files allows a subtype in parentheses, and it is used for a different purpose
> than the `Format` field for `.changes` files."

— Policy upgrading-checklist §10.33, Version 3.9.0, released June 2010,
<https://www.debian.org/doc/debian-policy/upgrading-checklist.html>, accessed 2026-08-09.

*(b) `Rules-Requires-Root` had to have an in-band discriminator retrofitted* so two variants
could share one value slot:

> "A space separated list of keywords described below.  These keywords must always contain a
> forward slash, which sets them apart from the other possible values of `Rules-Requires-Root`."

— Policy §5.6.31, accessed 2026-08-09.

*(c) One field's shape is discriminated by the presence of a **different** field*:

> "As an exception to Policy 5.6.13 (Description), the value of the `Description` field may omit
> the long description if the `Description-md5` field is defined."

— `DebianRepository/Format` wiki, <https://wiki.debian.org/DebianRepository/Format?action=raw>,
accessed 2026-08-09.

*(d) apt silently degrades an unrecognised variant rather than failing*:

> ```c
> _error->Warning("Unknown Multi-Arch type '%.*s' for package '%s'",
>       (int)MultiArch.size(), MultiArch.data(), Package().c_str());
> MA = pkgCache::Version::No;
> ```

— apt `apt-pkg/deb/deblistparser.cc`, `debListParser::ParseMultiArch`,
<https://salsa.debian.org/apt-team/apt/-/raw/main/apt-pkg/deb/deblistparser.cc>, accessed
2026-08-09.

### Q2 — Absent vs empty vs null

**There is no `null`, "empty" is largely illegal, and empty is NOT distinguishable from
"not recorded".** Omission is the canonical encoding of both "no data" and "the default".

> "Empty field values are only permitted in source package template control files
> (`debian/control`). Such fields are ignored."

— Policy §5.1, accessed 2026-08-09.

Writers are normatively told to drop them:

> "These tools are responsible for removing the line breaks from such fields when using fields
> from `debian/control` to generate other control files. They are also responsible for
> discarding empty fields."

— Policy §5.2, accessed 2026-08-09.

An empty list means **omit the field**, stated separately for two fields:

> "If no binary packages are being uploaded, this field will not be present."

— Policy §5.6.19 (`Binary`) and §5.6.13 (`Description` in `.changes`), accessed 2026-08-09.

In one case, absence is the *only* legal way to express the default — the format cannot say
the default out loud:

> "Note that, due to limitations in the archive management software, this value cannot
> currently be specified explicitly in binary package control files.  Instead, the field needs
> to be absent in order to imply its default value and `debhelper` discards it when
> transforming a source package control template."

— Policy §5.6.30 (`Multi-Arch: no`), accessed 2026-08-09.

> "In source package template control files, the `Package-Type` field should be omitted instead
> of giving it a value of `deb`, as this value is assumed for stanzas lacking this field."

— Policy §5.6.28, accessed 2026-08-09.

Confirmed in dpkg: an empty value is a silent no-op, indistinguishable from absence —

> ```c
> f_priority(...)
> {
> 	const char *str = value;
> 	int priority;
>
> 	if (!*value)
> 		return;
> ```

— dpkg `lib/dpkg/fields.c`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/lib/dpkg/fields.c>, accessed 2026-08-09.

### Q3 — Closed vocabularies

**The governing rule Debian arrived at: a new *algorithm* becomes a new *field*; a new *enum
value* is only safe where an unknown-fallback was designed in from the start.**

Checksums — new algorithm = new field, never a new value in an old field:

> "These multiline fields contain a list of files with a checksum and size for each one. Both
> `Checksums-Sha1` and `Checksums-Sha256` have the same syntax and differ only in the checksum
> algorithm used"

— Policy §5.6.24, accessed 2026-08-09. The legacy MD5 `Files` field was **never removed** and
is still mandatory alongside:

> "The list of files in these fields must match the list of files in the `Files` field."

— Policy §5.6.24, accessed 2026-08-09.

Weak algorithms are retired by a *reader* rule instead of by deletion:

> "Clients may not use the MD5Sum and SHA1 fields for security purposes, and must require a
> SHA256 or a SHA512 field."

— `DebianRepository/Format` wiki, accessed 2026-08-09.

Unknown `Checksums-*` fields are silently *not read* at all, because dpkg iterates its own
hardcoded algorithm table rather than the file's fields:

> ```perl
> my $CHECKSUMS = {
>     md5 => { name => 'MD5', … strong => 0, },
>     sha1 => { name => 'SHA-1', … strong => 0, },
>     sha256 => { name => 'SHA-256', … strong => 1, },
> };
> ```
> ```perl
> sub add_from_control {
>     …
>     foreach my $alg (checksums_get_list()) {
>         my $key = "Checksums-$alg";
> ```

— dpkg `scripts/Dpkg/Checksums.pm`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/scripts/Dpkg/Checksums.pm>, accessed
2026-08-09. (No SHA-512 in dpkg's table.)

**Where an unknown-fallback WAS designed in, it works.** `Priority` has an explicit catch-all
that *preserves the unrecognised value verbatim*:

> "The known priorities are required, important, standard, optional, extra, and unknown, but
> other values can be used as well."

— `deb-control(5)`, dpkg-dev 1.23.7,
<https://manpages.debian.org/unstable/dpkg-dev/deb-control.5.en.html>, accessed 2026-08-09.

> ```c
> 	priority = parse_nv(ps, PARSE_NV_FALLBACK, &str, priorityinfos);
> 	…
> 	if (str == NULL) {
> 		pkg->priority = PKG_PRIO_OTHER;
> 		pkg->otherpriority = nfstrsave(value);
> ```

— dpkg `lib/dpkg/fields.c`, accessed 2026-08-09.

Package types and architecture names are likewise open:

> "Simple field containing a word indicating the type of package: `deb` for binary packages and
> `udeb` for micro binary packages. Other types not defined here may be indicated."

— Policy §5.6.28, accessed 2026-08-09.

> "/* Ignore unknown types for forward-compatibility. */"

— dpkg `lib/dpkg/arch.c`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/lib/dpkg/arch.c>, accessed 2026-08-09.

**The best-designed part of the format is extension *inside values*** — namespaced keywords
with a MUST-ignore rule, and positional lists declared open-ended:

> "All tools must ignore keywords under namespaces they do not know or own.  A tool may emit a
> warning, or abort with an error, if it finds unknown keywords in namespaces it owns, but it is
> not required to do this for all keywords in the namespace."

— Policy §5.6.31, accessed 2026-08-09.

> "Other space-separated `keyword=value` items may be introduced in the future, and users of
> this field must ignore items with unknown keywords."

— Policy §5.6.33 (`Git-Tag-Info`), accessed 2026-08-09.

> "Fifth and subsequent space-separated items may be present and parsers must allow them."

— Policy §5.6.27 (`Package-List`), accessed 2026-08-09.

**Counter-example — a new value in an existing field DID break old readers:**

> "APT 1.8 added automatic subkey support and support for disabling it via ! as used by gnupg.
> As APT before 1.8 will choke on this new syntax it should be avoided until all expected users
> have a sufficiently recent client."

— `DebianRepository/Format` wiki (`Signed-By`), accessed 2026-08-09.

### Q4 — Strictness

**NOT FOUND: Debian Policy contains no general normative statement about what a reader must do
with an unknown *field*.** Searched every Policy chapter source (`ch-controlfields.rst`,
`ch-archive.rst`, `ch-source.rst`, `ch-binary.rst`, `ch-relationships.rst`,
`ch-customized-programs.rst`, `upgrading-checklist`) for `unknown field`, `unrecognised`,
`unrecognized`, `must ignore`, `should ignore`. The only hits concern unknown **keywords inside
a value** (quoted in Q3), not unknown fields.

Policy §5.7 is a **writer** rule, not a reader rule — and it is the format's designated
extension mechanism:

> "Additional user-defined fields may be added to the source package template control file. Such
> fields will be ignored, and not copied to (for example) binary or Debian source control files
> or Debian upload changes control files.
> If you wish to add additional unsupported fields to these output files you should use the
> mechanism described here.
> Fields in the source package template control file with names starting `X`, followed by one or
> more of the letters `BCS` and a hyphen `-`, will be copied to the output files. Only the part
> of the field name after the hyphen will be used in the output file. Where the letter `B` is
> used the field will appear in binary package control files, where the letter `S` is used in
> Debian source control files and where `C` is used in Debian upload changes control files."

— Policy §5.7,
<https://www.debian.org/doc/debian-policy/ch-controlfields.html#user-defined-fields>, accessed
2026-08-09.

**The de-facto rule is "tolerate", and it differs per tool** — the same unknown field is
retained by one tool and dropped with a warning by another:

- dpkg's *database* parser **silently retains** unknown fields (`arbs` list); it only errors if
  the name is under 2 characters or duplicated —
  dpkg `lib/dpkg/parse.c`, `pkg_parse_field()`,
  <https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/lib/dpkg/parse.c>, accessed 2026-08-09.
- dpkg's control-file *transformer* **warns and drops**:

> ```perl
>   } elsif (not field_is_allowed_in($field, $from_type)) {
>       warning(g_("unknown information field '%s' in input data in %s"),
>               $field, $from->get_option('name') || g_('control information'));
>   }
> ```

  — dpkg `scripts/Dpkg/Control/FieldsCore.pm`, `field_transfer_single()`, accessed 2026-08-09.
- apt's tag parser **retains** unknown tags in a secondary hash bucket (`Key::Unknown` →
  `BetaIndexes`) so they stay retrievable — apt `apt-pkg/tagfile.cc`, accessed 2026-08-09.

Index readers are explicitly warned to expect undocumented fields:

> "Note that the control file of `.deb` files may contain additional fields not yet documented
> by policy or not yet documented here which then might also be found in this file."

— `DebianRepository/Format` wiki, accessed 2026-08-09.

The `X-` convention has an **acknowledged flaw**: the prefix is *stripped* on output, so
user-defined fields land in the global namespace:

> "Take into account that these user-defined fields will be using the global namespace, which
> might at some point in the future collide with officially recognized fields. To avoid such
> potential situation you can prefix those fields with Private-, such as XB-Private-New-Field."

— `deb-src-control(5)`, dpkg-dev 1.23.7,
<https://manpages.debian.org/unstable/dpkg-dev/deb-src-control.5.en.html>, accessed 2026-08-09.

### Q5 — Version field semantics

**There is no format version on a `Packages`, `Sources`, or `Release` file.** The `Release`
file's `Version:` is the *distribution* release number, not a schema version:

> "The Version field, if specified, shall be the version of the release. This is usually a
> sequence of integers separated by the character `.` (full stop)."

— `DebianRepository/Format` wiki, accessed 2026-08-09.

Four different version-ish things exist, none interchangeable:

1. **`Standards-Version`** — package↔Policy conformance, explicitly *not* parse-affecting:

> "The version number has four components: major and minor version number and major and minor
> patch level. When the standards change in a way that requires every package to change the
> major number will be changed."

— Policy §5.6.11, accessed 2026-08-09.

> "For a package to have an old Standards-Version value is not *itself* a bug, however.  It just
> means that no-one has yet reviewed the package with changes to the standards in mind."

— Policy §3.1.1, accessed 2026-08-09.

2. **`Format` in `.changes`** — a real, semver-shaped schema version with a stated
   compatibility rule:

> "The value of this field declares the format version of the file. The syntax of the field
> value is a version number with a major and minor component. Backward incompatible changes to
> the format will bump the major version, and backward compatible changes (such as field
> additions) will bump the minor version. The current format version is 1.8."

— `deb-changes(5)`, dpkg-dev 1.23.7,
<https://manpages.debian.org/unstable/dpkg-dev/deb-changes.5.en.html>, accessed 2026-08-09.

**NOT FOUND**: any statement of what a reader MUST do on a higher `.changes` `Format`. The
major/minor *writing* rule is stated; the corresponding *reading* rule is not.

3. **`Format` in `.dsc`** — a dispatch key, **hard-rejected** if the major is unknown, and the
   minor is ignored and overwritten:

> ```perl
>  my ($major, $minor, $variant) = $self->{format}->set($format);
>  my $module = "Dpkg::Source::Package::V$major";
>  …
>  if ($@) {
>      error(g_("source package format '%s' is not supported: %s"), $format, $@);
>  }
> ```

— dpkg `scripts/Dpkg/Source/Package.pm`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/scripts/Dpkg/Source/Package.pm>, accessed
2026-08-09.

4. **`dpkg-build-api`** — a *behaviour* API level declared via a build-dependency; the newest and
   most deliberate versioning mechanism in the ecosystem:

> "The source package dpkg build API level, defines a versioned interface for source packages,
> where each API level provides specific behaviors and guarantees. These interfaces can then be
> adopted by packages in a gradual way, and phased out more easily than with global behavior
> changes."

— `dpkg-build-api(7)`,
<https://manpages.debian.org/unstable/dpkg-dev/dpkg-build-api.7.en.html>, accessed 2026-08-09.

### Q6 — Field identity and reuse

**Case-insensitive on read; canonical mixed case on write** — and the index format tightens this
into an explicit reader/writer asymmetry:

> "Field names are not case-sensitive, but it is usual to capitalize the field names using mixed
> case as shown below. Field values are case-sensitive unless the description of the field says
> otherwise."

— Policy §5.1, accessed 2026-08-09 (verbatim in `deb822(5)`).

> "In addition to the rules for control files, field names shall be generated using the case
> defined in this document, that is, code creating repositories shall be case-sensitive, but
> code reading repositories should not be case-sensitive."

— `DebianRepository/Format` wiki, accessed 2026-08-09. **This is Postel's law stated as a
normative rule about a single syntactic property**, and it is one of only two places in this
research where writer-strict/reader-lax is spelled out as policy (the other is Go's
`Parse`/`ParseLax`).

Name syntax and uniqueness:

> "The field name is composed of US-ASCII characters excluding control characters, space, and
> colon (i.e., characters in the ranges U+0021 (`!`) through U+0039 (`9`), and U+003B (`;`)
> through U+007E (`~`), inclusive). Field names must not begin with the comment character
> (U+0023 `#`), nor with the hyphen character (U+002D `-`)."

— Policy §5.1, accessed 2026-08-09.

> "5.1 A control stanza must not contain more than one instance of a particular field name."

— upgrading-checklist §10.33, Version 3.9.0 (June 2010), accessed 2026-08-09. Note that this
elementary constraint was only *written down* in 2010, ~15 years into the format's life.

**Renaming / removing / reusing a field name: NOT FOUND as stated policy. No reserved-name list
exists in Policy.** What exists is de-facto practice in dpkg: renamed fields keep a **permanent
read-side alias** that warns and is **never written back** (writer function `w_null`):

> ```c
> 	/* The following are the obsolete fields that get remapped to their
> 	 * modern forms, while emitting an obsolescence warning. */
> 	{ FIELD("Recommended"),      f_obs_dependency,  w_null,  dep_recommends },
> 	{ FIELD("Optional"),         f_obs_dependency,  w_null,  dep_suggests   },
> 	{ FIELD("Class"),            f_obs_class,       w_null                  },
> 	{ FIELD("Revision"),         f_obs_revision,    w_null                  },
> 	{ FIELD("Package-Revision"), f_obs_revision,    w_null                  },
> 	{ FIELD("Package_Revision"), f_obs_revision,    w_null                  },
> ```

— dpkg `lib/dpkg/parse.c`, accessed 2026-08-09. Still shipping today: `Recommended`→`Recommends`,
`Optional`→`Suggests`, `Class`→`Priority`. **Read forever, write never** is the de-facto rename
protocol.

### Q7 — Round-trip preservation

**NOT FOUND: no normative MUST/SHOULD anywhere** — Policy, `deb822(5)`, `deb-control(5)`, or the
repository-format wiki — requiring a rewriting tool to preserve fields it did not understand.

dpkg's actual behaviour is split three ways:

1. **The status/available database DOES round-trip unknown fields** — parsed into `arbs`, written
   back after all known fields:

> ```c
> 	for (fip = fieldinfos; fip->name; fip++) {
> 		fip->wcall(vb, pkg, pkgbin, fw_printheader, fip);
> 	}
> 	for (afp = pkgbin->arbs; afp; afp = afp->next) {
> 		varbuf_add_arbfield(vb, afp, fw_printheader);
> 	}
> ```

— dpkg `lib/dpkg/dump.c`, `varbuf_stanza()`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/lib/dpkg/dump.c>, accessed 2026-08-09.
Content is preserved; **relative field order is not** — unknown fields are appended at the end.

2. **Obsolete (known-but-retired) fields are deliberately NOT round-tripped** (`w_null`, Q6).

3. **Control-file generators drop unknown fields with a warning** unless `X[SBC]-`-prefixed (Q4).

The Perl deb822 library layer *does* preserve, and documents it:

> "The order in which fields have been set is remembered and is used to be able to dump back the
> same content. The output order can also be overridden if needed.
> You can store arbitrary values in the hash, they will always be properly escaped in the output
> to conform to the syntax of control files."

— dpkg `scripts/Dpkg/Control/HashCore.pm`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/scripts/Dpkg/Control/HashCore.pm>, accessed
2026-08-09.

### Q8 — Migration mechanism

**Dominant strategy: additive, "never break, only add".** No format-wide migration mechanism
exists for `Packages`/`Sources`/`Release`. Decades of pure accretion, from Policy's own
upgrading-checklist (<https://www.debian.org/doc/debian-policy/upgrading-checklist.html>,
accessed 2026-08-09):

| Policy version | Addition |
|---|---|
| 3.9.0 (June 2010) | "The `Checksums-Sha1` and `Checksums-Sha256` fields in `*.dsc` and `*.changes` files are now documented and recommended." |
| 3.9.4 (Aug 2012) | "New `Built-Using` field…" |
| 3.9.5 (Oct 2013) | "`Checksums-Sha1` and `Checksums-Sha256` are now mandatory in `.dsc` files." / "New section documenting the `Package-Type` field" / "New section documenting the `Package-List` field" |
| 4.0.1 (Aug 2017) | "New section documenting the `Testsuite` field in Debian source control files." |
| 4.1.5 (July 2018) | "Document the `Rules-Requires-Root` field." |
| 4.7.3 (Dec 2025) | "New sections documenting the `Git-Tag-Tagger` and `Git-Tag-Info` fields" |

Old readers are an **explicit design constraint**, with graceful degradation prescribed instead
of versioning:

> "Any tool (particularly older versions of them) may be unaware of this field and behave like
> the field was set to `binary-targets`.  The package build must gracefully cope with this and
> produce a semantically equivalent result."

— Policy §5.6.31, accessed 2026-08-09.

**When a break was genuinely needed, five different mechanisms were used — there is no single
one:** (1) versioned dispatch key with hard rejection (`.dsc` `Format`); (2) opt-in
build-dependency-declared API level (`dpkg-build-api`); (3) a **temporary field designed to be
deleted**, purely to sequence a transition —

> "An optional field with a temporary and very specific usecase … The field's purpose is to
> decouple the introduction of indexes like Contents-all from the introduction of Packages-all.
> Support by clients (if they choose to support it at all) is therefore bound to disappear after
> the transition to `all` for all indexes is done."

— `DebianRepository/Format` wiki (`No-Support-for-Architecture-all`), accessed 2026-08-09;
(4) dated deprecation with a removal floor (one-line `sources.list`, §5 R-D1); (5) permanent
read-side aliasing (Q6).

Removals happen, but on ~decade timescales: `Origin` and `Bugs` were added in dpkg 1.7.0
(2000-11-05) and removed in dpkg 1.19.5 (2019-02-23) — dpkg `debian/changelog`,
<https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/debian/changelog>, accessed 2026-08-09.

**A sixth mechanism, and the most transferable one: migrate by changing the FILENAME, so old
readers never open the new format at all.** apt's deb822 sources live in files with a new
extension, and pre-1.1 readers simply skip them:

> "Files in this format have the extension .sources."

> "This is a new format supported by apt itself since version 1.1. Previous versions ignore such
> files with a notice message as described earlier."

— `sources.list(5)`, apt 3.3.2,
<https://manpages.debian.org/unstable/apt/sources.list.5.en.html>, accessed 2026-08-09
(independently re-fetched and verified).

Old and new coexist in the same directory; the reader's file-extension filter *is* the version
check. No in-band version field is needed, and there is no ambiguity for a reader that
understands only one of them.

**Debian also has a delta convention for list-valued fields**, which lets a new field extend a
default without restating it:

> "Multivalue fields like Architectures also have Architectures-Add and Architectures-Remove to
> modify the default value rather than replacing it."

— same source, accessed 2026-08-09. (The one-line format has the equivalent `-=` / `+=`
operators.) This is a direct answer to "how do I let a consumer adjust a list I own without
either of us clobbering the other".

**And a documented instance of extending a format breaking third-party parsers** — the exact
risk profile of an index read by third-party tools:

> "Note that not all options as described below are supported by all apt versions. Note also that
> some older applications parsing this format on their own might not expect to encounter options
> as they were uncommon before the introduction of multi-architecture support."

— same source, accessed 2026-08-09.

---

## 4. Go module proxy (GOPROXY), `go.mod`, `go.sum`

Primary source: *Go Modules Reference* — <https://go.dev/ref/mod>, accessed 2026-08-09.
Parser source: `golang.org/x/mod/modfile/rule.go` —
<https://raw.githubusercontent.com/golang/mod/master/modfile/rule.go>, accessed 2026-08-09.

### Q1 — Tagged vs untagged unions

`go.mod` is a **verb-tagged line format**: every line's first token is its discriminator
(`module`, `go`, `require`, `exclude`, `replace`, `retract`, `tool`, `ignore`, `godebug`,
`toolchain`). The grammar is stated as such:

> "GoMod = { Directive } .
> Directive = ModuleDirective | GoDirective | ToolDirective | IgnoreDirective |
> RequireDirective | ExcludeDirective | ReplaceDirective | RetractDirective ."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

`go.sum` hashes are **algorithm-tagged inside the value**, with a pre-declared upgrade path:

> "The hash column consists of an algorithm name (like h1) and a base64-encoded cryptographic
> hash, separated by a colon (:). Currently, SHA-256 (h1) is the only supported hash algorithm.
> If a vulnerability in SHA-256 is discovered in the future, support will be added for another
> algorithm (named h2 and so on)."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

The parser even keeps its *own* recogniser deliberately loose so that future syntax tweaks
stay detectable:

> `// for forward compatibility.`
> `// (This code has to be work to identify new toolchains even if we tweak the syntax in the future.)`
> `var ToolchainRE = lazyregexp.New(`^default$|^go1($|\.)`)`

— `modfile/rule.go`, accessed 2026-08-09 (typo "has to be work" is in the source).

### Q2 — Absent vs empty vs null

For the proxy `.info` document, one field is required and one optional, with absence given
explicit meaning:

> "The Version field is required and must contain a valid, canonical version (see Versions).
> The $version in the request path does not need to be the same version or even a valid
> version; this endpoint may be used to find versions for branch names or revision identifiers.
> However, if $version is a canonical version with a major version compatible with $module,
> the Version field in a successful response must be the same. The Time field is optional. If
> present, it must be a string in RFC 3339 format."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

For `.mod`, "absent upstream" is normalised into a synthesised minimal document rather than an
error or an empty response:

> "If the module does not have a go.mod file at the requested version, a file containing only
> a module statement with the requested module path must be returned."

— same source.

**NOT FOUND**: a general rule about emitting `[]`/`{}` vs omitting collections in the GOPROXY
JSON. Searched <https://go.dev/ref/mod> full text for `empty`, `omit`, `null`.

### Q3 — Closed vocabularies

Two mechanisms, both explicit:
1. Hash algorithms: a named-prefix scheme with a reserved successor (`h1` → `h2`), quoted in Q1.
2. `go.mod` directives: unknown verbs are *tolerated in dependencies, rejected in your own
   module* — see Q4. That is a role-scoped fallback rather than a value-level one.

### Q4 — Strictness — **the clearest asymmetry found anywhere**

Go parses the *same file format* under two different strictness modes depending on **whose**
file it is. Strict for the main module (unknown directive is an error), lax for dependencies
(unknown directive is silently skipped) — and the reason given is explicitly forward
compatibility:

> `// ParseLax is like Parse but ignores unknown statements.`
> `// It is used when parsing go.mod files other than the main module,`
> `// under the theory that most statement types we add in the future will`
> `// only apply in the main module, like exclude and replace,`
> `// and so we get better gradual deployments if old go commands`
> `// simply ignore those statements when found in go.mod files`
> `// in dependencies.`

— `modfile/rule.go`, accessed 2026-08-09.

> `// The [Parse] and [ParseLax] functions both parse a go.mod file and return an`
> `// abstract syntax tree. ParseLax ignores unknown statements and may be used to`
> `// parse go.mod files that may have been developed with newer versions of Go.`

— `modfile/rule.go` package doc, accessed 2026-08-09.

The implementation, with the allow-list of directives that matter even in a dependency:

> ```go
> func (f *File) add(errs *ErrorList, block *LineBlock, line *Line, verb string, args []string, fix VersionFixer, strict bool) {
> 	// If strict is false, this module is a dependency.
> 	// We ignore all unknown directives as well as main-module-only
> 	// directives like replace and exclude. It will work better for
> 	// forward compatibility if we can depend on modules that have unknown
> 	// statements (presumed relevant only when acting as the main module)
> 	// and simply ignore those statements.
> 	if !strict {
> 		switch verb {
> 		case "go", "module", "retract", "require", "ignore":
> 			// want these even for dependency go.mods
> 		default:
> 			return
> 		}
> 	}
> ```

and in strict mode:

> ```go
> 	switch verb {
> 	default:
> 		errorf("unknown directive: %s", verb)
> ```

— `modfile/rule.go`, accessed 2026-08-09.

**The rule of thumb this encodes: be strict about data you are the authority for; be lax
about data you merely consume.** Not writer-vs-reader — *owner*-vs-*consumer*.

### Q5 — Version field semantics — the `go` directive, and a real tightening

> "A go directive indicates that a module was written assuming the semantics of a given
> version of Go. The version must be a valid Go version, such as 1.14, 1.21rc1, or 1.23.0.
> The go directive sets the minimum version of Go required to use this module. Before Go 1.21,
> the directive was advisory only; now it is a mandatory requirement: Go toolchains refuse to
> use modules declaring newer Go versions."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

So the reader contract on a higher version changed **from "warn and try" to "refuse"**. The
toolchain document is more explicit, and records that **the new gate was backported into
already-released readers**:

> "Before Go 1.21, Go toolchains treated the go line as an advisory requirement: if builds
> succeeded the toolchain assumed everything worked, and if not it printed a note about the
> potential version mismatch. Go 1.21 changed the go line to be a mandatory requirement
> instead. This behavior is partly backported to earlier language versions: Go 1.19 releases
> starting at Go 1.19.13 and Go 1.20 releases starting at Go 1.20.8, refuse to load workspaces
> or modules declaring version Go 1.22 or later."

— <https://go.dev/doc/toolchain>, accessed 2026-08-09.

That backport is the answer to the problem crates.io documented but could not fix
(pre-1.51 Cargo silently misinterpreting `v`): **Go shipped the version check into patch
releases of old readers**, so that old readers would *refuse* rather than *misread*. The
tightening was then made survivable by pairing it with automatic toolchain download:

> "When the go or toolchain line is newer than the bundled toolchain, the go command runs the
> newer toolchain instead. For example, when using the go command bundled with Go 1.21.3 in a
> main module that says go 1.21.9, the go command finds and runs Go 1.21.9 instead. It first
> looks in the PATH for a program named go1.21.9 and otherwise downloads and caches a copy of
> the Go 1.21.9 toolchain. This automatic toolchain switching can be disabled, but in that
> case, for more precise forwards compatibility, the go command will refuse to run in a main
> module or workspace in which the go line requires a newer version of Go."

— <https://go.dev/doc/toolchain>, accessed 2026-08-09.

The blunt statement of the rule:

> "The Go toolchain refuses to load a module or workspace that declares a minimum required Go
> version greater than the toolchain's own version. For example, Go 1.21.2 will refuse to load
> a module or workspace with a go 1.21.3 or go 1.22 line."

— same source. For `go.work`, the same contract is stated as version-directed
interpretation:

> "The go directive indicates the go toolchain version with which the go.work file is intended
> to work. If changes are made to the go.work file format, future versions of the toolchain
> will interpret the file according to its indicated version."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

**The GOPROXY protocol itself has no version number.** NOT FOUND: any protocol version,
negotiation, or capability header for GOPROXY. Searched <https://go.dev/ref/mod> for
"protocol version", "version of the protocol" (0 hits each; "GOPROXY protocol" occurs 11
times, always as the name of the endpoint set). The protocol is versioned only by the fact
that new endpoints (e.g. `@latest`) are declared optional:

> "This endpoint is optional, and module proxies are not required to implement it."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

### Q6 — Field identity and reuse — **an explicit reservation, and the best single quote in the research**

> "More fields may be added in the future, so other names are reserved."

— <https://go.dev/ref/mod>, GOPROXY `$base/$module/@v/$version.info` description, accessed
2026-08-09.

This is the only place in any subject surveyed where a format *pre-emptively reserves the
entire unused name space* of a JSON object. It converts "unknown field" from an
extensibility question into a **conflict-avoidance rule binding third parties**: you may not
invent your own key in that object, because the name might later be spent by the spec.

### Q7 — Round-trip preservation — mandatory, verbatim, for proxies

> "Otherwise, the original, unmodified go.mod file must be returned."

— <https://go.dev/ref/mod>, `$base/$module/@v/$version.mod`, accessed 2026-08-09.

A proxy is a byte-preserving relay, not a re-serializer. Backed by hashing:

> "After downloading a .mod or .zip file, the go command computes a cryptographic hash and
> checks that it matches a hash in the main module's go.sum file."

— same source. Again: preservation enforced by content-addressing, not by good manners.

Note the deliberate exception — the two *mutable* endpoints are explicitly declassified:

> "Note that version lists and version metadata returned for .info requests are not
> authenticated and may change over time."

— same source.

### Q8 — Migration mechanism

Additive-plus-lax-readers (Q4), reserved names (Q6), a pre-registered successor for the one
crypto value that must eventually rotate (Q1). The one genuinely breaking change — the `go`
directive becoming mandatory — was shipped with an automatic remediation (toolchain
switching), not with a migration guide. See §5.

---

---

## 5. npm registry (packument)

Primary sources (raw markdown from the npm registry docs repo):
- `docs/responses/package-metadata.md` —
  <https://raw.githubusercontent.com/npm/registry/master/docs/responses/package-metadata.md>
- `docs/REGISTRY-API.md` —
  <https://raw.githubusercontent.com/npm/registry/master/docs/REGISTRY-API.md>
- npm Docs, `package.json` — <https://docs.npmjs.com/cli/v11/configuring-npm/package-json/>
All accessed 2026-08-09.

### Q1 — Tagged vs untagged unions — the worst case found

npm's metadata is full of **untagged unions**, documented as such, because the packument
copies user-authored `package.json` shapes through verbatim. The `funding` field is the
clearest, with three shapes and no discriminator:

> "`funding`: object containing a URL that provides up-to-date information about ways to help
> fund development of your package, or a string URL, or an array of these"

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

npm's own docs confirm the same shorthand-or-object pattern elsewhere:

> "If you have a single executable, and its name should be the name of the package, then you
> can just supply it as a string." *(on `bin`)*

> "For GitHub, GitHub gist, Bitbucket, or GitLab repositories you can use the same shortcut
> syntax you use for `npm install`" *(on `repository`)*

— <https://docs.npmjs.com/cli/v11/configuring-npm/package-json/>, accessed 2026-08-09.

**The recorded trouble case is `license`.** Historically it was an untagged union of
*string*, *object*, and a sibling plural field `licenses` holding an array of objects. It was
deprecated in favour of a single string form:

> "Some old packages used license objects or a "licenses" property containing an array of
> license objects"

> "Those styles are now deprecated. Instead, use SPDX expressions"

— <https://docs.npmjs.com/cli/v11/configuring-npm/package-json/>, accessed 2026-08-09.

The escape hatch for values outside the closed SPDX vocabulary is itself a string sentinel,
not a new shape:

> "If you are using a license that hasn't been assigned an SPDX identifier, or if you are
> using a custom license, use a string value like this one: { "license": "SEE LICENSE IN
> <filename>" }"

— same page. (See §5 for the deprecation timeline.)

### Q2 — Absent vs empty vs null — the only explicit tri-state found

The abbreviated packument states a general absence rule:

> "The `name`, `version`, and `dist` fields will always be present. The others will be absent
> if they are irrelevant for this package version."

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

and then, for exactly one field, distinguishes **false from unknown**:

> "`_hasShrinkwrap`: `true` if this version is known to have a shrinkwrap that must be used to
> install it; `false` if this version is known not to have a shrinkwrap. If this field is
> undefined, the client must determine through other means if a shrinkwrap exists."

— same source, accessed 2026-08-09.

That is a genuine three-valued field: `true` / `false` / *not recorded*, with a documented
reader obligation in the third case. It is the only such contract found in any subject.

### Q3 — Closed vocabularies

**Checksums are the live case, and npm solved it by adding a new tagged field beside the old
untagged one rather than changing the old field's meaning:**

> "- `shasum`: the SHA-1 sum of the tarball
> - `integrity`: since Apr 2017, string in the format `<hashAlgorithm>-<base64-hash>`, refer
>   the [Subresource Integrity] and [cacache] package for more"

and, listed as a future item in the same list:

> "- (in the future) a SHA-2 512 sum of the tarball"

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

`shasum` is permanently SHA-1 because its name and its untagged value make it unchangeable;
`integrity` carries the algorithm inside the value (SRI form), so it can rotate. Note also
the doc's habit of stamping each field with the date it appeared — `since Apr 2017`,
`since Feb 2018`, `since Apr 2018` — an informal per-field version marker in lieu of a schema
version.

**NOT FOUND**: any documented fallback rule for an unrecognised enum value in the packument.
Searched both npm registry docs files for `unknown`, `unrecognized`, `MUST`, `SHOULD`.

### Q4 — Strictness

**NOT FOUND**: any normative statement (MUST/SHOULD) that readers ignore or reject unknown
fields. Searched `docs/responses/package-metadata.md` and `docs/REGISTRY-API.md` for `MUST`,
`SHOULD`, `unknown`, `ignore`, `additional` — the documents contain no RFC-2119 language at all.

What is documented is that the full packument is an **open object carrying arbitrary
publisher-controlled keys**, which makes reader leniency mandatory in practice:

> "The full version object will also contain any other fields the package publisher chose to
> include in their package.json file for that version."

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

The abbreviated form is the opposite — a closed allow-list:

> "This form of the package metadata exists to provide a smaller payload designed to support
> installation. It contains an allow list of fields from the full metadata set."

— same source, accessed 2026-08-09.

So npm runs **both policies at once on the same data**: an open document for humans/tools, a
curated closed projection for installers. The two are selected by content negotiation (Q5).

A candid admission that no validation was ever applied on the way in:

> "Historically no validation has been performed on those fields; they are generated by
> parsing user-provided data in package.json at publication time."

— `docs/responses/package-metadata.md` (on `human` objects), accessed 2026-08-09.

The documentation has visibly lost the race with the format — one field is documented as:

> "- `directories`:???"

— `docs/REGISTRY-API.md`, accessed 2026-08-09. (Verbatim; that is the entire entry.)

### Q5 — Version field semantics — versioned by media type in the `Accept` header, not by a field

> "The registry responds with a JSON-formatted string containing metadata for the package
> named, either in full or abbreviated form depending on what you request in the `Accept`
> header. If you provide no Accept header, the full document is returned. To request an
> _abbreviated_ document with only the fields required to support installation, set the
> `Accept` header in your request to the following string:
>
> `application/vnd.npm.install-v1+json`"

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

with a documented graceful-degradation idiom built from HTTP q-values:

> "A more typical accept header might request json as a fallback, like this:
>
> `application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*`"

— same source.

**There is no version field inside the document.** NOT FOUND: any `schemaVersion`,
`api-version` or equivalent in the packument. Searched both registry docs files.
**NOT FOUND**: any documented reader contract for encountering a higher version — there is no
`install-v2`, and no stated policy for what a client does if one appeared.

Two structural consequences for an at-rest index: (a) this mechanism **does not exist without
a request** — a file in a git repo has no `Accept` header, so npm's entire versioning strategy
is unavailable to a file-at-rest design; (b) it has been an active interop hazard even where
it does apply — see §5.

### Q6 — Field identity and reuse

Fields are identified by name. **NOT FOUND**: any reserved-prefix convention, or any stated
policy on renaming/removing/reusing field names. Searched both registry docs and the
`package.json` docs page for `reserved`, `rename`, `deprecat`.

Observed convention only: registry-generated fields carry a leading underscore (`_id`, `_rev`,
`_npmUser`, `_npmVersion`, `_nodeVersion`, `_hasShrinkwrap`) — and two of them are undisguised
CouchDB implementation leakage frozen into the public format:

> "- `_id`: the package name, used as an ID in CouchDB
> - `_rev`: the revision number of this version of the document in CouchDB"

— `docs/responses/package-metadata.md`, accessed 2026-08-09. The storage engine's identifiers
became permanent public API.

### Q7 — Round-trip preservation

**NOT FOUND**: any statement about preserving unknown fields on rewrite. Searched both npm
registry docs files. The nearest documented guarantee is about trustworthiness of a
generated subtree, not preservation:

> "The `dist` object is generated by npm and may be relied upon."

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

### Q8 — Migration mechanism

Never-break-only-add, plus dual serving via content negotiation, plus per-field "since
<date>" annotations (Q3). The only *removal* documented is a deprecation with a warning
period rather than a format break (the `licenses` array, §5). **NOT FOUND**: any explicit
migration path or format-version transition document for the packument.


---

## 6. Maven — POM `modelVersion` and `maven-metadata.xml`

**Provenance markers used below:** `[FH]` = fetched and read by me directly; `[SS]` = delivered by
the delegated research stream and not independently re-fetched by me; `[FH✓]` = both. All access
dates 2026-08-09.

**This is the contrast case for the whole research: Maven is the one subject whose reader
REJECTS unknown elements — and it is also the one whose format stopped evolving.**

### Q1 — Tagged vs untagged unions

`type` is an untagged, *derived* discriminator: an open registry key that silently expands into
`(classifier, extension, packaging)`.

`[FH]` POM XSD `maven-4.0.0.xsd`, on `type`:

> "The type of dependency, that will be mapped to a file extension, an optional classifier, and a
> few other attributes. Some examples are `jar`, `war`, `ejb-client` and `test-jar`: see default
> artifact handlers for a list. New types can be defined by extensions, so this is not a complete
> list."

`[FH]` and on `packaging`:

> "Plugins can create their own packaging, and therefore their own packaging types, so this list
> does not contain all possible types."

`[SS]` Maven's own prose names the consequence
(<https://maven.apache.org/repositories/artifacts.html>):

> "This has "interesting" consequences. Consider the artifact
> `org.project:reusable-test-support:1.0:tests:jar`. Maybe surprisingly, a dependency on this
> artifact can be described in two ways:"

> "If the type is not one of these 11 names, then the value of the "type" is used as the
> extension."

**Recorded trouble, still open:** `[SS]` MNG-8097 (created 2024-04-15, **Open**), *"Validate that
each dependency->type is a type registered in an artifact handler"*:

> "Currently often the dependency's type is being set to the extension and the resolution is
> lenient, i.e. if there is no artifact handler defining the value given in `dependency->type`
> resolution transparently uses the type as extension."

`[SS]` MNG-6681 (Fixed 3.6.2) — the *documentation* of the discriminator was itself wrong:

> "Reading that, people understand that type is extension" … "The "While it usually represents the
> extension on the filename of the dependency, that is not always the case." as first explanation
> is totally misleading"

`[SS]` MNG-4654 (Fixed 3.0-alpha-3) — packaging/type conflation shipped wrong file extensions:

> "Maven ignores the artifact handler because it tries to match `project.packaging` with
> `artifactHandler.packaging`. As a result, the project's main artifacts ends up being
> installed/deployed with the file extension "woframework" instead of "jar"."

`[SS]` And two normative Maven pages **disagree** on the same mapping (`artifact-handlers.html`
gives `ejb-client` classifier `client`; `artifacts.html` differs) — unreconciled.

At rest, `maven-metadata.xml` stores only the *expanded* pair and never the tag — `[FH]`
`repository-metadata-1.1.0.xsd`: *"The classifier of the sub-artifact. Each classifier and
extension pair may only appear once."*

### Q2 — Absent vs empty vs null

**Empty collections omitted; scalars equal to their default omitted; absent == empty == default.**
`[FH]` from Maven's own code generator templates (`src/mdo/writer-stax.vm`):

> `if (list != null && !list.isEmpty())` · `if (props != null && !props.isEmpty())` ·
> `if (value != null && !Objects.equals(defaultValue, value))`

One three-state case is modelled, using the empty string as the "unknown" token — `[FH]`
`DependencyScope.java`:

> `UNDEFINED("", false)` — "When no scope is explicitly given, UNDEFINED will be used, but its
> meaning will depend on whether the DependencyCoordinates is used in dependency management…"

`[FH]` **Real Central data contradicts the omit rule** — a present-but-empty `<release></release>`
appears in ASF snapshot metadata. `[SS]` And `validateEnum` short-circuits on `""`
(`if (string == null || string.length() <= 0) { return true; }`), so the empty string is silently
accepted everywhere.

Distinct from every JSON subject here: **absence in a POM means "inherit from the parent
document"**, not "apply a default" — resolution is by document composition (`[FH]`
<https://maven.apache.org/ref/current/maven-model/maven.html>: *"Values from the parent project
will be the default for this project if they are left unspecified."*).

### Q3 — Closed vocabularies

`[FH]` `DefaultModelValidator.validateEnum` emits:

> `"must be one of " + values + " but is '" + string + "'."`

But the **severity was deliberately lowered because third parties invented values** — `[SS]`
Maven 3.9.x source comment:

> ```java
>                     /*
>                      * TODO Extensions like Flex Mojos use custom scopes like "merged", "internal", "external", etc. In
>                      * order to don't break backward-compat with those, only warn but don't error out.
>                      */
>                     validateEnum(prefix, "scope", problems, Severity.WARNING, ...
> ```

`[SS]` A rendered instance (MNG-5072):

> `[WARNING] 'dependencies.dependency.scope' for junit:junit:jar must be one of [provided, compile, runtime, test, system] but is 'test,runtime'. @ line 13, column 11`

`[SS]` …while the *plugin-dependency* variant of the same field is ERROR-severity with a
different list (TUSCANY-3845). **The same concept, two vocabularies, two severities.**

`[FH]` New enum members are **hard-gated on `modelVersion`**:

> ```java
>             // MNG-8750: New dependency scopes are only supported starting with modelVersion 4.1.0
>                             "scope '" + scope + "' is not supported with modelVersion 4.0.0; "
>                                     + "use modelVersion 4.1.0 or remove this scope.",
> ```

`[SS]` *(Caveat carried forward: the `MNG-8750` reference in that comment does not match JIRA —
MNG-8750 is an unrelated issue. Correct id **NOT FOUND**.)*

`[FH]` The scope vocabulary grew 5 → 10 in Maven 4. `[FH]` And one vocabulary **shrank** —
`combine.children` `override → merge`, `combine.self` `append → merge` — requiring a shipped
rewrite tool.

`[FH]` **NOT FOUND**: any scope enumeration in either published XSD — `scope` is `xs:string` in
both 4.0.0 and 4.1.0. The vocabulary lives only in a validator.

### Q4 — Strictness — **normatively REJECT, but scoped by provenance, with a lenient retry**

`[FH]` The throw site, Maven's reader template `src/mdo/reader-stax.vm`:

> ```java
>     private void checkUnknownElement(XMLStreamReader parser, boolean strict) throws XMLStreamException {
>         if (strict) {
>             throw new XMLStreamException("Unrecognised tag: '" + parser.getName() + "'", parser.getLocation(), null);
>         }
>
>         for (int unrecognizedTagCount = 1; unrecognizedTagCount > 0;) {
>             int eventType = nextTag(parser);
>             if (eventType == XMLStreamReader.START_ELEMENT) {
>                 unrecognizedTagCount++;
>             } else if (eventType == XMLStreamReader.END_ELEMENT) {
>                 unrecognizedTagCount--;
>             }
>         }
>     }
> ```

Strictness also covers **attributes** and **namespace** — `[FH]`:

> `throw new XMLStreamException("Unknown attribute '" + attribute + "' for tag '" + tagName + "'", …)`
> `String.format("Unexpected namespace for element '%s': found '%s' but expected '%s'", …)`

And the user-facing framing — `[FH]` Apache Maven wiki, *ModelParseException*
(<https://cwiki.apache.org/confluence/display/MAVEN/ModelParseException>):

> "This error indicates a bad syntax of your POM."
> "The POM contains invalid/unknown elements that Maven does not understand."
> "The POM is not even well-formed XML, e.g. the XML elements are not properly nested."

**Unknown element and malformed XML are the same error class** — Maven treats unrecognised
content as corruption.

**But strictness is scoped by PROVENANCE, and this is the crucial detail** — `[FH]`
`DefaultModelBuilder`: `boolean strict = isBuildRequest();`. `[SS]` In Maven 3.9.x it is literally
file-vs-repository: a local `pomFile` → `Severity.ERROR`; a *downloaded* POM (`pomFile == null`) →
`Severity.WARNING` and the build continues. `[FH]` Maven states the consequence in its own code:

> `// Non-POM projects are consumed as dependencies where unknown elements are ignored…`

**This is the same owner-vs-consumer axis Go chose (§1.4 Q4)** — strict on the document you
author, lenient on documents you merely consume. Maven *depends* on that leniency to publish
4.1.0 POMs into a 4.0.0 world (Q8).

`[SS]` Recorded rejections in the wild: MNG-7837 (`Unrecognised tag: 'project'` for a legal
element inside plugin `<configuration>`); MNG-6036 (open 2016→2023, namespaced config rejected —
reporter: *"It would be awesome if the XML parser would resolve namespaces properly."*); MNG-8666
(2025) — **Maven's own shipped `settings.xml` unreadable by its own builder**:
`Unrecognised tag: 'repositories'`.

`[FH]` Schema-level closure: `xs:any` appears exactly 12× in `maven-4.0.0.xsd`, only inside
`properties`/`configuration`/`reports`/`goals`, always `processContents="skip"`. No `xs:any` at
`Model` level; no `anyAttribute` anywhere.

`[FH]` **`maven-metadata.xml` is always read NON-strict**: `return new MetadataStaxReader().read(input, false);`
`[SS]` MNG-4498: *"We should just warn in case of unreadable metadata."* — **the published index
is lenient even though the authored POM is not.**

**NOT FOUND** (both streams): any maven.apache.org page stating *normatively* that readers reject
unknown elements. The norm exists only in generator code plus closed `xs:all` content models.

### Q5 — Version field semantics

`[FH]` 4.0.0 XSD: `modelVersion` **required** — *"4.0.0 is the only value supported by Maven 3."*
`[FH]` 4.1.0 XSD: **optional** (`minOccurs="0"`), namespace `http://maven.apache.org/POM/4.1.0`.
`[FH]` master `.mdo`: required again, *"`4.0.0` for Maven 3, `4.1.0` or `4.2.0` for Maven 4"* —
*the required/optional discrepancy between the published 4.1.0 XSD and current master is real and
unreconciled.* `[FH]` `KNOWN_MODEL_VERSIONS = List.of(4.0.0, 4.1.0, 4.2.0)`; `maven-4.2.0.xsd` →
**HTTP 404**.

Reader contract on a higher version — `[FH]` master:

> `requestedModel + "' is not supported by this Maven version (" + getMavenVersionString(session) + "). Supported modelVersions are: " + validVersions + ". Building this project requires a newer version of Maven."`

`[SS]` Maven 3.9.x, at `Severity.FATAL`:

> `"of '" + string + "' is newer than the versions supported by this version of Maven: " + values + ". Building this project requires a newer version of Maven."`

`[SS]` A real rendered instance (MNG-7851, input `4.0`):

> `'modelVersion' of '4.0' is newer than the versions supported by this version of Maven: [4.0.0]. Building this project requires a newer version of Maven.`

`[SS]` *(Caveat: no verbatim log line for `4.1.0` under Maven 3 was found. In practice Maven 3
hits `Unrecognised tag: 'subprojects'` at parse time **before** the validator runs — i.e. the
strict reader defeats the version field's own error message.)*

**`modelVersion` is a computed minimum, not a declaration** — `[FH]`:

> `String minVersion = new MavenModelVersion().getModelVersion(model);` … `"the model contains elements that require a model version of " + minVersion,`

and it silently **downgrades an over-declaration**: `[FH]` `MavenModelVersionTest.testV4ModelVersion`
— a model declaring `4.1.0` that uses no 4.1.0 feature returns `"4.0.0"`.

**`maven-metadata.xml`: a version attribute exists, is optional since 1.1.0, and is dead in
practice.** `[FH]` XSD: `<xs:attribute name="modelVersion" type="xs:string" use="optional"/>` —
*"The version of the underlying metadata model."* `[FH]` Real Central files carry neither namespace
nor attribute. `[FH]` My own live fetch of `commons-lang3/maven-metadata.xml` (2026-08-09) confirms
it: the only document-level scalar is `<lastUpdated>20251116125534</lastUpdated>`.

`[SS]` **MNG-4139 (Fixed 2.2.0) — the schema reference was deliberately deleted**:

> "the schema is not used since we don't use a validating parser, and nobody really edits them by
> hand. With network traffic on these files at a premium for downloads, it's best left out to save
> some bytes."

`[SS]` MNG-3125 — the XSD arrived in 2007, *after* the format was already in production, with an
empty description.

### Q6 — Field identity and reuse

`[FH]` Identity = element name **+ namespace**, and the namespace is templated from the version:
`xml.namespace="http://maven.apache.org/POM/${version}"`. **Every model-version bump is a
namespace break.**

`[FH]` Rename policy = add + deprecate + keep. XSD element-name diff 4.0.0 → 4.1.0:
only-in-4.1.0 = `condition, lang, source, sources, stringFiltering, subproject, subprojects,
targetVersion`; **only-in-4.0.0 = none**. `modules` carries
`<xs:documentation source="version">[4.0.0 => 4.1.0]</xs:documentation>` and
`@deprecated Use {@link #subprojects} instead.`, with `@Deprecated(since = "4.0.0")` and an mdo
version range ending at 4.2.0 — i.e. **scheduled for removal two model versions later.**

**NOT FOUND** (both streams): any reserved-name list or written policy on reuse of a retired
element name.

### Q7 — Round-trip preservation — **none, anywhere**

`[FH]` The non-strict skip loop (Q4) **records nothing** — unknown subtrees are consumed and
discarded, not retained. Writers emit only model fields.

`[FH]` `MavenMetadata.merge` reads recessive → merges → writes from the typed model; a
`modelVersion` attribute on an existing remote file is **dropped on the next deploy**.

`[FH]` The consumer POM is a deliberate lossy projection: *"Removes build, mailing lists, issue
management, and other build-specific information"*; `builder.properties(null).reporting(null);`

`[FH]` The only preservation Maven achieves is of *source text*, and it needed a third-party
library — mvnup *"uses domtrip internally for superior formatting preservation"*.

### Q8 — Migration mechanism — **dual artifacts + automatic downgrade + a migration CLI**

Not "never break, only add", and not in-place upgrade. `[SS]` Maven's own site:

> "Maven 4 updates the POM version to 4.1.0 which defines the namespace
> http://maven.apache.org/POM/4.1.0. Version 4.1.0 adds some new elements and attributes, while
> others are marked as deprecated. To not break the ecosystem, this version is only available for
> the build POM (stored in source control), while the consumer POM (available in remote
> repositories like Maven Central) will still use version 4.0.0. Maven generates the consumer POM
> during the build from the build POM."

> "Warning: There are rare situations where Maven 4 will produce a consumer POM based on version
> 4.1.0, for example when condition-based profiles (see below) can't be transformed to version
> 4.0.0. Maven will show a warning in such situations."

`[SS]` The originating issue, MNG-6656 (Fixed, 2019-05-14), states the motive plainly:

> "The pom.xml as we know it has reached it limits, but it is quite hard to do improvements as
> long as the local pom (as part of the sources) is exactly the same as the file being published.
> For the Maven eco system it is important that the published file will still be a model 4.0.0 to
> ensure other projects can still depend on these artifacts."

`[FH]` Plus a hard failure when the projection is impossible (*"cannot be downgraded to model
version 4.0.0 because it contains features that require a newer model version…"*), and a shipped
one-way upgrade tool: `mvnup`, *"If not specified the tool will target model version 4.0.0."* /
*"Model version 4.0.0 is fully compatible with Maven 3, while projects with a model version 4.1.0
can only be built using Maven 4."* — `// Support upgrades: 4.0.0 → 4.1.0, 4.0.0 → 4.2.0, 4.1.0 → 4.2.0`.

**The generalisable pattern: separate the AUTHORED document from the PUBLISHED document, and let
the published one stay on the old schema forever.** That is the same move OCI made under duress
(§1.2 Q8) and the same one NuGet named as policy (§1.8b Q8).


---

## 7. RPM / yum `repomd.xml`

**Provenance markers:** `[FH]` = fetched and read by me directly; `[SS]` = delivered by the
delegated research stream, not independently re-fetched by me; `[FH✓]` = both. All access dates
2026-08-09.

Structurally this is the closest analogue in the study to "a manifest of typed entries published
as files", and it is the **mirror image of Maven**: no version, free-form tags, warn-and-skip
readers — and it never needed a migration mechanism.

My own primary artefact: a **live** `repomd.xml` from EPEL 9
(<https://dl.fedoraproject.org/pub/epel/9/Everything/x86_64/repodata/repomd.xml>, fetched
2026-08-09, saved to `corpus/primary-sources/repomd.xml`).

### Q1 — Tagged vs untagged unions — **mandatory free-form tag as the record's primary key**

`[FH]` Every entry carries an explicit `type`, and the bodies are structurally identical:

> ```xml
> <repomd xmlns="http://linux.duke.edu/metadata/repo" xmlns:rpm="http://linux.duke.edu/metadata/rpm">
>   <revision>1786236202</revision>
>   <data type="primary">
>     <checksum type="sha256">14e40c5facb527535365188388722571d48d9fb479ef4ef2fa20aaeaa648c5c5</checksum>
>     <open-checksum type="sha256">26576acd8ca65bdf43a22662e232af24f599b41ee4981ba1c7da3f188ce510d5</open-checksum>
>     <location href="repodata/14e40c5facb527535365188388722571d48d9fb479ef4ef2fa20aaeaa648c5c5-primary.xml.xz"/>
>     <timestamp>1786236183</timestamp>
>     <size>4514744</size>
>     <open-size>55142100</open-size>
>   </data>
> ```

— live EPEL 9 file. The eight types present: `primary`, `filelists`, `other`, `primary_db`,
`filelists_db`, `other_db`, `group`, `updateinfo`.

`[SS]` The tag is the identity — `// Remove all existing record of the same type` in
`cr_repomd_set_record`.

**But createrepo_c layers two *inferred* discriminators on top**, and both are fragile:

`[SS]` **(a) payload shape from a string suffix**:
`if (g_str_has_suffix((char *) rec->type, "_db")) { … "database_version" … }`

`[SS]` **(b) the tag derived from the filename when unspecified** — `modifyrepo_c(8)`:
*"Specific datatype of the metadata, will be derived from the filename if not specified."* — the
code splits the basename at the first `.`, with no vocabulary check anywhere, only
`g_warning("Record with type \"%s\" already exists in repomd.xml", task->type);`

`[SS]` And a missing tag is only a warning, with the reader **inventing a value**: `val = "unknown";`
— shipped with a regression fixture `<data> <!-- WARNING: Missing data element type -->`.
**Collision hazard: two untagged records both key to `"unknown"` and evict each other.**

### Q2 — Absent vs empty vs null

`[SS]` Omitted when absent, with `NULL` / `-1` sentinels meaning "not recorded":
`if (rec->checksum_open)` · `if (rec->size_open != -1)` ·
`if (rec->checksum_header && rec->size_header != -1)`. There is **no empty form and no way to say
"empty but known"**.

`[SS]` `<revision>` is the one synthesised field:
`// Use the current time if no revision was explicitly specified`.

**NOT FOUND**: any document stating an absent-vs-empty rule. The rule exists only as C code.

### Q3 — Closed vocabularies — **`<data type>` is OPEN; the checksum vocabulary is where it went wrong**

`[FH]` The canonical client-side statement, `dnf5.conf(5)` on `optional_metadata_types`:

> "Note: The list includes only metadata types recognized by DNF. However, a repository's metadata
> may include various other types (e.g., AppStream or metadata stored as databases instead of XML
> files). The special value ``all`` represents all metadata types present in the repository,
> including those unknown to DNF."

`[FH✓]` Zchunk is the textbook additive rollout — Fedora Change, *Upgrade/compatibility impact*:

> "Old-style xz and gzip metadata will continue to be generated by createrepo_c, so old versions of
> Fedora will use the xz/gzip metadata while upgrading to the new version, and then use the new
> zchunk metadata after the upgrade."

`[SS]` Note what the proposal does **not** say: anything about what a reader that does not
understand `*_zck` does. The entire compatibility story is *"we keep emitting the old records
too."* New types are minted by string concatenation, even for third-party records:
`gchar *additional_metadatum_rec_zck_type = g_strconcat(elem_type, "_zck", NULL);`

**The checksum trap — the most important RPM finding.** `[FH]` `createrepo(8)`
(<https://linux.die.net/man/8/createrepo>, header `.TH "createrepo" "8" "2005 Jan 2" "Seth Vidal"`):

> "-s --checksum
> Choose the checksum type used in repomd.xml and for packages in the metadata. The default is now
> "sha256" (if python has hashlib). The older default was "sha", which is actually "sha1", however
> explicitly using "sha1" doesn't work on older (3.0.x) versions of yum, you need to specify "sha"."

and the workarounds, same file `[FH]`:

> "--unique-md-filenames  Include the file's checksum in the metadata filename, helps HTTP caching (default)"
> "--simple-md-filenames  Do not include the file's checksum in the metadata filename."
> "--retain-old-md  Keep around the latest (by timestamp) N copies of the old repodata (so clients with older repomd.xml files can still access it). Default is 0."

Two stacked breakages: the default moved to an algorithm old clients could not compute, **and the
two spellings of the same algorithm are not interchangeable** — old yum accepts `sha`, rejects
`sha1`. `[SS]` The alias survives in code to this day
(`else if (!strcmp(sha_type, "")) return CR_CHECKSUM_SHA;` → `EVP_sha1()`), with the header
admitting *"Note: SHA is just a "nickname" for the SHA1. This is for the compatibility with
original createrepo."*, and is mirrored in librepo and libsolv. `[SS]` When it landed: createrepo
`ChangeLog`, `2009-01-26 Seth Vidal` — *"make sha256 the default checksum type everywhere"*.

### Q4 — Strictness — **readers IGNORE, uniformly across three generations**

`[SS]` createrepo_c: `// We are inside of unknown element` → descend-and-discard;
`cr_xml_parser_warning(pd, CR_XML_WARNING_UNKNOWNTAG, "Unknown element \"%s\"", element);` → warn
and continue.

`[SS]` Original yum — the namespace is **stripped, not checked**, and the dispatch is an
`if/elif` chain with no `else`:

> ```python
> def ns_cleanup(qn):
>     if qn.find('}') == -1: return qn
>     return qn.split('}')[1]
> ```

with only well-formedness fatal: `raise RepoMDError, "Damaged repomd.xml file"`. Every `<data>`
record is retained keyed by its type string whether understood or not.

`[SS]` librepo is the same design; libsolv swallows unknowns silently (its debug print is
`#if 0`'d out) and its element table is *narrower* than createrepo_c's — so everything zchunk
added falls into that path.

`[SS]` **The one hard failure is an unknown *checksum* type**: libsolv
`pool_error(…, "line %d: unknown checksum type: %s", …)`; librepo `LRE_UNKNOWNCHECKSUM`.

**Asymmetry, and it is the exact opposite of Maven's:** the *writer* is strict
(`[SS]` `"Unknown/Unsupported checksum type \"%s\""`), the *reader* is permissive.

**NOT FOUND**: any normative "readers MUST ignore" statement — there is no specification document
to state it in.

### Q5 — Version field semantics — **no schema version; the one schema that exists is a stale, unenforced DTD written by a consumer**

`[FH]` The only document-level scalar in the live file is `<revision>1786236202</revision>` — a
Unix timestamp. `[SS]` And the man pages say it is arbitrary: *"Arbitrary string for a repository
revision."* / *"User-specified revision for this repository."* Observed values differ in **kind**
across distros: `<revision>9-stream</revision>` (CentOS) vs `<revision>1786236202</revision>`
(EPEL 9) — `[SS]`/`[FH]`.

`[FH]` The root element carries two fixed, unversioned namespaces:
`xmlns="http://linux.duke.edu/metadata/repo"`.

`[FH✓]` **A schema does exist, and its provenance is the finding.** `yum/docs/repomd.dtd`:

> "This is DTD for repomd.xml files used by yum.
>
> 	Author:
> 		Miroslav Suchý <msuchy@redhat.com>
>
> 	Date: 2010-01-29"

> ```
> <!ATTLIST data
> 	type (other_db | other | filelists_db | filelists | primary_db | primary | group | prestodelta | group_gz) #REQUIRED
> >
> ```

Written by a **consumer**, five years after the format was already in production, declaring `type`
as a **closed enumeration of nine values**. Reality blew past it immediately (`updateinfo`,
`modules`, every `*_zck`, `filelists-ext`, …), and it lacks `<header-checksum>`, `<header-size>`,
`<repoid>`, `<contenthash>`, `<tags>` entirely. **No implementation validates against it.**
`[SS]` Its origin, the author on the yum list 2009-06-29:

> "BTW - Do you know where is definition of repodata files (repomd.xml, primary.xml...)? I could not
> find DTD file, nor any other documentation of the format."

`[SS]` The only version field anywhere is `<database_version>`, per-record and payload-only,
emitted on a `_db` suffix test. No major/minor split anywhere.

### Q6 — Field identity and reuse

`[SS]` Identity = the `type` string; namespaces are declared but unenforced. A rename happened and
the old name is a **permanent alias**: `gchar *repoid; /*!< OBSOLETE, replaced by contenthash */` —
both still read and written. **Nothing is ever removed.**

`[FH]` The namespace URI outlived its host: `http://linux.duke.edu/metadata/repo` is still on every
RPM repo on the internet — unversioned, therefore unchangeable.

**NOT FOUND**: reserved-name list or reuse policy. Collision avoidance is purely social — with a
failure mode documented in Q7.

### Q7 — Round-trip preservation — **preserved by default, with two silent-loss paths**

`[SS]` The default: *"Keep all additional metadata (not primary, filelists and other xml or sqlite
files, nor their compressed variants) from source repository during update (default)."*;
`.keep_all_metadata = TRUE,`

`[SS]` **Loss path 1 — it is gated on `--update`:**

> ```c
>     if (cmd_options->update && cmd_options->keep_all_metadata &&
>         old_metadata_location && old_metadata_location->additional_metadata)
> ```

A plain `createrepo_c <dir>` regenerates from scratch and third-party records are gone.

`[SS]` **Loss path 2 — "additional" is a hard-coded prefix test, not a registry:**

> ```c
>         else if ( !g_str_has_prefix(record->type, "primary_"   ) &&
>                   !g_str_has_prefix(record->type, "filelists_" ) &&
>                   !g_str_has_prefix(record->type, "filelists-ext_" ) &&
>                   !g_str_has_prefix(record->type, "other_"     ) )
> ```

A third-party record whose type merely *starts with* one of those four prefixes is **silently
dropped on every update, with no warning.** Four undocumented reserved prefixes, discoverable only
by reading C.

Either way, preservation is at **record** granularity — unknown child elements inside a known
`<data>` are dropped.

Separately, the format achieves verification-without-comprehension by content-addressing every
payload filename — `[FH]` `createrepo(8)`: *"--unique-md-filenames Include the file's checksum in
the metadata filename"*, visible in the live file as
`repodata/14e40c5f…c5c5-primary.xml.xz`. **A reader that understood 3 of 8 entries can still
verify exactly the 3 it fetched.**

### Q8 — Migration mechanism — **"never break, only add", because there is no version to negotiate**

`[FH✓]` Every change has been a new type alongside the old (zchunk), or a default flip with restore
flags (`--checksum`, `--simple-md-filenames`, `--retain-old-md`). `[SS]` Deprecation is a slow fade,
never removal — sqlite `*_db` is documented `DEPRECATED` and still shipping in EPEL 9 today, as my
own live fetch confirms.

`[SS]` **And the reason the discipline holds is itself the most interesting finding** — yum list,
2009-06, on a proposal to add a second checksum per `<data>`. Seth Vidal:

> "if we start doing this we run the risk of breaking any of the non-yum depsolvers."

James Antill:

> "I think it's worse than that, for instance does anyone know what 3.0.1 will do? Or 2.4.0?"

**With no version field, nobody could answer what deployed readers would do — so the extension was
not made.** The absence of a version field did not merely fail to help; it exerted a *chilling
effect* on evolution. That is the cost of "no version" stated by the people paying it.


---

## 8a. PyPI Simple API — PEP 629 / PEP 691 / PEP 714

**This is the most directly applicable subject in the entire research**, because PEP 629 designed
its versioning *specifically for repositories served as static files by dumb HTTP servers* — the
same constraint as a JSON index published into a git repository.

Primary sources, all fetched raw and first-hand on 2026-08-09:
- PEP 629 — <https://raw.githubusercontent.com/python/peps/main/peps/pep-0629.rst> (Status: Final;
  created 2020-07-16, accepted 2020-08-20)
- PEP 691 — <https://raw.githubusercontent.com/python/peps/main/peps/pep-0691.rst>
- PEP 714 — <https://raw.githubusercontent.com/python/peps/main/peps/pep-0714.rst> (Status:
  Accepted; created 2023-06-06, resolved 2023-06-27)

### Q1 — Tagged vs untagged unions

**NOT FOUND**: a discriminated-union construct in the Simple API JSON. The format is a fixed set
of typed keys. The *serialization* itself is tagged, however — by media type, and the reference
client dispatches on it and errors on anything unrecognised:

> ```python
>     match content_type:
>         case "application/vnd.pypi.simple.v1+json":
>             handle_v1_json(resp)
>         case "application/vnd.pypi.simple.v1+html" | "text/html":
>             handle_v1_html(resp)
>         case _:
>             raise Exception(f"Unknown content type: {content_type}")
> ```

— PEP 691, accessed 2026-08-09.

### Q2 — Absent vs empty vs null

PEP 691 states shape constraints normatively, and notably forbids the top-level array form:

> "* All JSON responses will *always* be a JSON object rather than an array or other
>   type."

> "* While JSON doesn't natively support an URL type, any value that represents an
>   URL in this API may be either absolute or relative as long as they point to
>   the correct location. If relative, they are relative to the current URL as if
>   it were HTML."

— PEP 691, accessed 2026-08-09.

**NOT FOUND**: an explicit rule on omitting vs emitting `[]`/`{}`, or on distinguishing empty
from unknown. Searched pep-0691.rst for `empty`, `omit`, `null`, `absent`.

The "always an object, never an array" rule is worth lifting on its own: **a top-level array has
nowhere to put `meta`**, so it forecloses ever versioning the document. PEP 691 closed that door
deliberately.

### Q3 — Closed vocabularies

**NOT FOUND** in PEP 691 as a general rule. The relevant live case is hashes, which are carried
as a *map keyed by algorithm name* (`hashes: {"sha256": "..."}`) rather than a single tagged
string — an open key space by construction, consistent with C3 in §3.

### Q4 — Strictness — **normative, unambiguous, and stated as a MUST on readers**

> "* Additional keys may be added to any dictionary objects in the API responses
>   and clients **MUST** ignore keys that they don't understand."

— PEP 691, accessed 2026-08-09.

This is the **clearest single sentence found in the entire research** on unknown-field handling
for at-rest data. Compare OCI, which says the same thing but downgraded it to SHOULD NOT in
v1.1 (§1.2 Q4), and every other subject, which does not say it at all.

Note the symmetry: the same sentence grants writers permission ("may be added") and binds readers
("MUST ignore"). One sentence, both halves of the contract.

### Q5 — Version field semantics — **`meta.api-version`, and the only complete reader contract found**

PEP 691 mandates the field and delegates its semantics:

> "* All JSON responses will have a ``meta`` key, which contains information related to
>   the response itself, rather than the content of the response."

> "* All JSON responses will have a ``meta.api-version`` key, which will be a string that
>   contains the :pep:`629` ``Major.Minor`` version number, with the same fail/warn
>   semantics as defined in :pep:`629`."

— PEP 691, accessed 2026-08-09.

PEP 629 supplies the semantics — **quoted here in full because it is the deliverable of this
research**:

> "When interpreting the repository version:
>
> * Incrementing the major version is used to signal a backwards
>   incompatible change such that existing clients would no longer be
>   expected to be able to meaningfully use the API.
> * Incrementing the minor version is used to signal a backwards
>   compatible change such that existing clients would still be
>   expected to be able to meaningfully use the API."

and the client obligations:

> "Clients interacting with the simple API **SHOULD** introspect each
> response for the repository version, and if that data does not exist
> **MUST** assume that it is version 1.0.
>
> When encountering a major version greater than expected, clients
> **MUST** hard fail with an appropriate error message for the user.
>
> When encountering a minor version greater than expected, clients
> **SHOULD** warn users with an appropriate message.
>
> Clients **MAY** still continue to use feature detection in order to
> determine what features a repository uses."

— PEP 629, accessed 2026-08-09.

Three things no other subject does:
1. **Absent version has a defined meaning** — MUST assume 1.0 — so unversioned legacy data stays
   legal forever.
2. **Major and minor have different, explicitly different reader actions**: hard fail vs warn.
   Nobody else splits the reader's response by component.
3. **Feature detection is explicitly permitted alongside** the version, rather than replaced by it.

And the expectation that the major will never move — the version exists mostly to *disambiguate*:

> "It is expectation of this PEP that the major version will never be
> incremented, and any future major API evolutions would utilize a
> different mechanism for API evolution. However the major version
> is included to disambiguate with future versions (e.g. a hypothetical
> simple api v2 that lived at /v2/, but which would be confusing if the
> repository-version was set to a version >= 2)."

— PEP 629, accessed 2026-08-09.

PEP 691 adds the writer-side rule, and is candid that the boundary is a judgement call:

> "Similar to :pep:`629`, the major version number **MUST** be incremented if any
> changes to the new format would result in no longer being able to expect existing
> clients to meaningfully understand the format.
>
> Likewise, the minor version **MUST** be incremented if features are
> added or removed from the format, but existing clients would be expected to continue
> to meaningfully understand the format.
>
> Changes that would not result in existing clients being unable to meaningfully
> understand the format and which do not represent features being added or removed
> may occur without changing the version number.
>
> This is intentionally vague, as this PEP believes it is best left up to future PEPs
> that make any changes to the API to investigate and decide whether or not that
> change should increment the major or minor version."

— PEP 691, accessed 2026-08-09.

Note the third clause: **a change that neither adds nor removes a feature does not bump anything
at all.** That matches crates.io's practice (§3 C2) but here it is written down.

Multi-serialization rule, relevant if you ever publish the same data two ways:

> "Future versions of the API may add things that can only be represented in a subset
> of the available serializations of that version. All serializations version numbers,
> within a major version, **SHOULD** be kept in sync, but the specifics of how a
> feature serializes into each format may differ, including whether or not that feature
> is present at all."

> "It is the intent of this PEP that the API should be thought of as URL endpoints that
> return data, whose interpretation is defined by the version of that data, and then
> serialized into the target serialization format."

— PEP 691, accessed 2026-08-09.

### Q6 — Field identity and reuse

Keys are identified by name. **NOT FOUND**: a reserved-prefix convention or a stated rename
policy in PEP 691. PEP 714, however, *performed* a rename and legislated exactly how readers and
writers handle both names during the overlap — see Q8 and §5.

### Q7 — Round-trip preservation

**NOT FOUND**: any statement requiring a rewriting client or mirror to preserve unknown keys.
Searched pep-0691.rst and pep-0629.rst. Note that PEP 629's rejection of header-based versioning
(§5) was motivated by keeping mirrors *byte-dumb*, which achieves preservation as a side effect —
a mirror that copies files verbatim cannot drop anything.

### Q8 — Migration mechanism

Three distinct mechanisms, all exercised:

1. **Minor bump + MUST-ignore** for ordinary additions (Q4, Q5).
2. **Dual serialization with content negotiation** (HTML + JSON) — with the fallbacks made
   explicit, and with static-file-friendly alternatives (a URL parameter) offered because
   negotiation is not always available.
3. **Rename the key** when an old reader hard-fails on it — PEP 714, with a precisely specified
   overlap period:

> "The :pep:`658` metadata, when used in the :pep:`691` JSON representation of the
> Simple API, **MUST** be emitted using the key ``core-metadata``, with the
> supported values remaining the same."

> "To support clients that used the previous key names, the HTML representation
> **MAY** also be emitted using the ``data-dist-info-metadata``, and if it does
> so it **MUST** match the value of ``data-core-metadata``."

> "Clients consuming the JSON representation of the Simple API **MUST** read the
> :pep:`658` metadata from the key ``core-metadata`` if it is present. They
> **MAY** optionally use the legacy ``dist-info-metadata`` key if it is present
> but ``core-metadata`` is not."

— PEP 714, accessed 2026-08-09.

That is a complete, reusable rename protocol: **new name is MUST-read and MUST-write; old name is
MAY-write and MAY-read; if both are written they MUST agree.** And the graceful-degradation
claim is stated rather than assumed:

> "There is a minor compatibility break in this PEP, in that clients that currently
> correctly handle the existing metadata keys will not automatically understand
> the newer metadata keys, but they should degrade gracefully, and simply act
> as if the :pep:`658` metadata does not exist."

— PEP 714, accessed 2026-08-09.

---

## 8b. NuGet service index (`index.json`)

*Coverage note: delivered by delegated research as a corrections/delta report; the base Q1–Q8
sweep was not recovered. What follows is the material that was delivered, with the agent's own
confidence markings preserved. Full raw text is at
`corpus/raw-subagent-nuget-delta.md`.*

### Q1 / Q3 — `@type` as discriminator, and a closed vocabulary that failed silently for six years

NuGet's index is a list of resources each tagged with `@type`, matched by **exact string**, with
the version baked into the tag value (`PackageBaseAddress/3.0.0`, `SearchQueryService/3.5.0`).
The designer's own diagnosis of why that was a mistake — NuGet/Home#4206, *"Support for v3
versioned services"*, `emgarten`, opened 2017-01-05 (metadata verified independently):

> "The previous model used hardcoded type strings which made it impossible for a server to
> optimize or support or new feature without breaking older clients unless the client added a
> new type."

> "The above scenarios work today in 3.5.0 but cannot be enabled without breaking older clients.
> Having this feature will make it possible to do the server work at a later date instead of when
> client work begins on a new version.
>
> It would also make it possible for 3rd party servers to support new features instead of being
> limited to what nuget.org supports."

**Versioning inside the discriminator value couples server capability to client release cadence** —
the opposite of what it looks like it does. The consequence, live in 2026: `SearchFilter.PackageTypes`

> "has been silently ignored by nuget.org and other compliant V3 sources for as long as the
> property has existed publicly"

— NuGet.Client PR #7395, merged 2026-06-10 (single-sourced; corroborated by the shipped code and
`Strings.resx` read first-hand by the delegated agent). The fix converts a silent wrong answer
into a thrown exception — i.e. **the repair for a silent compatibility failure is itself a
breaking change.**

The open design question sat unanswered for six years — NuGet/Home#8915, `zivkan`, 2020-07-20:

> "From an API design point of view, how should NuGet.Protocol behave when the server implements
> an old version of the NuGet HTTP protocol and doesn't support packageType filtering?" …
> "Silently ignore the issue, make the request anyway, and return results that may not be the
> package type requested?" … "API design is hard :("

### Q3 (cont.) / Q6 — a **two-tier vocabulary**: documented keys are permanent, undocumented keys are disposable

nuget.org's published policy, per the delegated report:

> "If a resource is not documented here, we *strongly* recommend that you do not take a dependency
> on them. We may remove or change the behavior of these undocumented resources"

and removals have in fact happened — `MetricsService`, `TotalStats/3.0.0-rc` and the bare
`SearchGalleryQueryService` appear in 2015/2016 Wayback captures and are absent from the live
index (verified independently by the agent against `web.archive.org/…id_/` raw captures and a
live fetch, 2026-08-09).

**Saying in writing which keys are contractual and which are yours to churn is cheap, and it is
the only mechanism found that lets a published vocabulary shrink.**

### Q5 — the version field moved twice, and once *backwards*

Verified sequence: `3.0.0-rc.1` (Feb 2015) → `3.0.0-beta.1` (Jan 2016) → `3.0.0` (by Aug 2018).
Under SemVer 2.0.0 precedence `3.0.0-beta.1 < 3.0.0-rc.1`, so **the declared schema version went
down**, and nothing noticed — the client only ever compares `version.Major != 3`.

**A version field that no reader fully evaluates is not a contract, it is decoration.**

### Q8 — breaking change ⇒ **new URL**, never a new major in the same document

NuGetGallery#5403, *"NuGet V3 API service index moving to 3.0.0!"*, `joelverhagen`, 2018-02-05
(metadata verified independently):

> "Non-breaking changes to the schema can be introduced and will be signaled by a minor version
> change in the existing document" … "Any future breaking changes will not be made to the
> existing V3 service index. They would be introduced in a different service index URL."

Directly comparable to Debian's new-file-extension trick (§1.3 Q8) and PEP 629's parenthetical
about "a hypothetical simple api v2 that lived at /v2/". **For a git-hosted index a new path is
nearly free, which makes this the cheapest available answer to "what if we need a real break".**

### Q7 — the reader **normalises rather than preserves**

Per the delegated report on NuGet's System.Text.Json converter test table: non-string array
members are silently dropped, a wrong-typed `@type` degrades to `[]` with no error, and on write
a single-element array is re-emitted as a bare string. **A NuGet-shaped round-trip is lossy and
shape-changing by design.**

### Q2, Q4 — **NOT FOUND** (not recovered from the delegated research)

The base sweep covering absent-vs-empty and the normative strictness rule was not delivered.
Observed evidence only: Azure Artifacts ships four vendor-private `@type` values (`VssBaseUrl`,
`VssFeedId`, `VssQualifiedFeedViewId`, `AzureDevOpsProjectId`) that no documented client
understands, which means unknown entries are tolerated in practice.

---

## 8c. Homebrew JSON API

Sources: <https://formulae.brew.sh/docs/api/>, accessed 2026-08-09.

**Q1–Q8: NOT FOUND, essentially across the board.** The documentation describes the endpoints
(`https://formulae.brew.sh/api/formula.json`, `.../cask.json`, and per-item
`.../api/formula/${FORMULA}.json`) but contains **no statement about API versioning, stability
guarantees, deprecation policy, breaking-change policy, unknown-field handling, or warnings to
third-party consumers**, and no mention of an API v3.

Searched: the API docs page above for versioning/stability/deprecation language. This is a
genuine and notable negative result rather than a gap in searching — **a widely-consumed JSON API
published as static files with no documented compatibility contract at all.** Treat Homebrew as an
example of the default outcome when nobody writes the contract down, not as a design to copy.

*(The delegated deep-dive on Homebrew did not return in time; a follow-up should check
Homebrew/brew GitHub discussions for the JSON API v3 migration, which was outside what the
published docs disclose.)*


---

# §2 Cross-subject table

Split into two tables for width. **Bold** = normatively stated (MUST/SHOULD in a spec).
*Italic* = observed implementation behaviour, not specified. "—" = NOT FOUND.

## Table A

| | **crates.io sparse index** | **OCI image-spec** | **Debian deb822 / Packages** | **Go GOPROXY / go.mod** |
|---|---|---|---|---|
| **Q1 Tagged unions** | No general tag. `kind` is a loose string. `features2` exists *because* an untagged value-grammar widening broke readers | **Strongest: `mediaType` REQUIRED on every descriptor**; MUST NOT parse unknown type. In-band manifest `mediaType` only SHOULD (was "reserved" pre-1.0.2) | **No tag — variant chosen by which FILE you're in.** `Files`/`Binary`/`Description` change arity+separator by context; `Format` collides with itself | Verb-tagged lines (first token = discriminator). `go.sum` hashes tagged `h1:` |
| **Q2 Absent/empty/null** | absent == `null` == documented default. Defaults only written down in **Cargo 1.84** | **Absent and empty MUST be equivalent** ("MUST either be absent or be an empty map"). Invented `application/vnd.oci.empty.v1+json` sentinel so a REQUIRED field can be meaningless | **No `null`.** Empty "only permitted in templates… Such fields are ignored"; writers MUST discard empty. For `Multi-Arch: no`, absence is the *only* legal encoding | `.info`: `Version` required, `Time` optional. Missing `go.mod` upstream ⇒ **synthesise** a minimal one, not an error |
| **Q3 Closed vocabularies** | — (no fallback documented; enums aren't modelled as enums, so unknown values survive) | **Open by grammar**: unknown digest algorithms SHOULD pass validation; registry + MUST-implement floor (sha256). BLAKE3 added later — mechanism exercised | **New algorithm ⇒ new FIELD** (`Checksums-Sha256`), never a new value. `Priority` has a real catch-all (`PKG_PRIO_OTHER`, value preserved verbatim). Counter-case: `Signed-By`'s new syntax "will choke" old apt | `h1` → `h2` pre-reserved for the SHA-256 break |
| **Q4 Strictness** | — no statement. *serde default: no `deny_unknown_fields` ⇒ ignores* | **Only subject with a normative rule** — and it was **weakened MUST NOT → SHOULD NOT** in v1.1, plus narrowed to "in a known media type". Unknown *annotation keys* still **MUST NOT** error | — **NOT FOUND in Policy.** *De-facto tolerance, split by tool*: dpkg DB retains; dpkg generator warns+drops; apt retains in a side bucket | **Role-scoped**: strict on your OWN `go.mod` (`unknown directive` = error), **lax on dependencies'** — explicitly "for forward compatibility" |
| **Q5 Version field** | **`v`, per-RECORD** (NDJSON line). Absent ⇒ 1. **Higher ⇒ skip that entry**, not error. Single integer, no major/minor. Shipped undocumented years before first use | `schemaVersion` **frozen at 2**, "value will not change", "MAY be removed". Real version lives in the **media type** | **None** on `Packages`/`Sources`/`Release`. Four unrelated version-ish things exist; `.dsc` `Format` is a hard-reject dispatch key | **None for GOPROXY.** `go` directive: **advisory → MANDATORY in 1.21**, and the check was **backported into 1.19.13/1.20.8** |
| **Q6 Field identity** | By name. — no reserved list, no rename policy. Same name means different things across index / publish API / `cargo metadata` (documented as a hazard) | By name. **Reserved**: `org.opencontainers` prefix, reverse-DNS annotations, reserved even against sibling OCI specs | **Case-insensitive read, canonical case write** (stated as policy). — no rename policy; *de-facto: permanent read-side aliases, writer `w_null` — read forever, write never* | **"More fields may be added in the future, so other names are reserved."** — the whole unused key space pre-claimed |
| **Q7 Round-trip** | **Forbidden to rewrite at all**: "should not be modified after they are added except for the `yanked` field" | **MUST NOT alter content in a way that changes the digest** (clause ADDED in v1.1; absent at v1.0.2). Enforced physically by content-addressing | — no rule. *dpkg splits 3 ways: DB retains (but reorders to end); obsolete fields dropped; generators warn+drop* | **"the original, unmodified go.mod file must be returned"**, backed by `go.sum` |
| **Q8 Migration** | Never break, only add. "the format for older packages should not change". Transport migration (git→sparse) done by URL scheme, payload unchanged | Additive; new media type as last resort; `_`-prefixed API extensions; **warning header** as a deprecation channel; capability-probe + fallback tag | Additive for decades. **Six mechanisms**, incl. **new FILE EXTENSION** (`.sources`) old readers skip, and **dated removal floors** ("not before 2029") | Additive + lax readers + reserved names. The one real break shipped with **automatic toolchain download** as remediation |

## Table B

| | **npm packument** | **Maven POM / metadata** | **RPM `repomd.xml`** | **PyPI Simple (PEP 629/691/714)** |
|---|---|---|---|---|
| **Q1 Tagged unions** | **Worst case.** `funding` = object \| string \| array, no tag; `bin`/`repository` shorthand forms; `license` was string\|object + plural `licenses` array ⇒ **deprecated** | Fixed XML schema, no polymorphic fields. — no recorded inferred-variant incident | **Cleanest tagged design**: `<data type="primary">` on every entry; bodies structurally identical. New kinds (`primary_db`, `updateinfo`, `group`) added alongside old | — no union construct. Serialization tagged by media type; reference client **errors on unknown content type** |
| **Q2 Absent/empty/null** | **Only explicit tri-state found**: `_hasShrinkwrap` true/false/**undefined ⇒ "client must determine through other means"**. Otherwise "absent if irrelevant" | XML: absence means **inherit from parent POM**, not "default" — resolved by document composition | Entry present-with-full-body or absent entirely; no empty form | **"always a JSON object rather than an array"** (so `meta` always has a home). — no []/{} rule |
| **Q3 Closed vocabularies** | **New algorithm ⇒ new FIELD beside the old**: `shasum` (SHA-1, frozen forever) + `integrity` (`<alg>-<b64>`). Fields stamped "since Apr 2017" etc. | — (unknown *values* not verified; given Q4, assume closed) | **Best-documented enum break anywhere**: default sha1→sha256 broke old yum, AND `sha` vs `sha1` are **not interchangeable spellings of the same algorithm** | Hashes are a **map keyed by algorithm name** — open by construction |
| **Q4 Strictness** | — **no RFC-2119 language at all.** Full packument is an open object carrying arbitrary publisher keys; abbreviated form is a closed allow-list | **REJECTS unknown elements.** "The POM contains invalid/unknown elements that Maven does not understand" — same error class as malformed XML | — no spec located. *Observed: 8 data types coexist; partial readers work, so unknowns are skipped* | **"Additional keys may be added… and clients MUST ignore keys that they don't understand."** Clearest sentence in the research |
| **Q5 Version field** | **None in the document.** Version lives in the **`Accept` header** (`application/vnd.npm.install-v1+json`) — unavailable at rest. No v2 ever | `modelVersion` **required, exactly one legal value (4.0.0)**, ~2 decades. `maven-metadata.xml` has **no version at all** — only `<lastUpdated>` timestamp | **No schema version.** `<revision>` is a Unix timestamp. Only signal is an unversioned XML namespace | **`meta.api-version`, Major.Minor. Absent ⇒ MUST assume 1.0. Major higher ⇒ MUST hard fail. Minor higher ⇒ SHOULD warn.** Only complete reader contract found |
| **Q6 Field identity** | By name. — no reserved prefix/rename policy. `_id`/`_rev` are **CouchDB internals frozen into public API** | By element name + namespace (namespace encodes model version). — no rename policy | By `type` attribute value. — no policy; *`sha` vs `sha1` proves values are frozen once matched on* | By name. — no reserved prefix in 691, but **PEP 714 legislated a full rename protocol** |
| **Q7 Round-trip** | — no statement. "`dist` object is generated by npm and may be relied upon" | — moot: unknown content is rejected before preservation arises | **N/A by construction** — regenerated wholesale; safety comes from `(type, checksum, location)` triples + content-addressed filenames | — no statement. *Header-versioning was rejected partly to keep mirrors byte-dumb, which preserves as a side effect* |
| **Q8 Migration** | Never break, only add; dual serving by content negotiation; deprecation with warning period | — **NOT FOUND.** Observable fact: `modelVersion` frozen 4.0.0 for ~20 years. **Strict readers ⇒ the format stops evolving** | Additive at row level; **writer-side flags** (`--checksum sha`, `--simple-md-filenames`) so publishers can emit legacy-compatible output | Minor bump + MUST-ignore; dual serialization; **rename the key** (PEP 714) when old readers hard-fail |

### NuGet (partial — delta only; see §1.8b)

| Q | Answer |
|---|---|
| Q1/Q3 | `@type` exact-string match, **version baked into the tag value**; no unknown-value fallback ⇒ `SearchQueryService/3.5.0` **silently ignored for six years**; the fix was itself a breaking change |
| Q3/Q6 | **Two-tier vocabulary**: documented types permanent, undocumented types explicitly disposable ("We may remove or change the behavior of these undocumented resources") — and removals really happened |
| Q5 | `version` moved `3.0.0-rc.1` → `3.0.0-beta.1` → `3.0.0` — **a SemVer regression nobody noticed**, because clients only check `Major != 3` |
| Q7 | Reader **normalises rather than preserves**: array `@type` re-emitted as bare string; malformed degrades to `[]` silently |
| Q8 | **Breaking change ⇒ new service-index URL**, never a new major in the same document |
| Q2/Q4 | NOT FOUND (base sweep not recovered) |

### Homebrew

**Q1–Q8: NOT FOUND across the board.** No documented versioning, stability, deprecation, or
unknown-field policy exists in the published API docs. A genuine negative result.


---

# §3 Convergences — where all or most subjects agree

These are the load-bearing findings. Each is stated with the subjects that support it and at
least one anchoring quote already given in §1.

## C1. "Never break, only add" is universal — and it is a WRITER discipline, not a reader one

Every subject surveyed follows additive-only evolution for its at-rest metadata. Nobody
operates a scheduled breaking-change cadence.

- crates.io: *"the format for older packages should not change, so older versions of Cargo
  should be able to use them."*
- Debian: three decades of pure accretion in the Policy upgrading-checklist, with
  `Files`/MD5 still mandatory alongside `Checksums-Sha256` a decade after it was superseded.
- Go: new directives added continuously; `.mod` bytes must be returned unmodified.
- OCI: `subject`, `artifactType`, `data`, BLAKE3 all added inside `v1.x`.
- npm: fields stamped "since Apr 2017", "since Feb 2018", "since Apr 2018"; nothing removed.

**The consequence people underestimate:** because the rule is enforced on writers, the format
accumulates fields that are permanently optional and permanently ambiguous. Cargo's own source
says the quiet part out loud about `yanked`: *"This was added in 2014. Everything in the
crates.io index has this set now, so this probably doesn't need to be an option anymore."* It is
still an `Option`. **You never get to narrow a type after publication.**

## C2. Adding a FIELD does not bump the version; only a change that would make an old reader MISREAD existing data does

This is the sharpest and most transferable rule found, and it is consistent everywhere a
version exists at all.

- crates.io is explicit: `links` (2018), `rust_version` (2023) and `pubtime` (2025) were added
  with **no `v` bump**. `v` went to 2 only for `features2` — because the *grammar of values
  inside an existing field* widened, which made old readers fail rather than ignore.
- OCI added `subject`/`artifactType` with no `schemaVersion` change (it is frozen at 2).
- Debian adds fields freely with no version anywhere; the `.dsc` `Format` key changes only when
  the *unpacking algorithm* changes.
- Go adds directives freely; the `go` line gates *language semantics*, not field presence.

**Corollary:** a version number that increments on every field addition is not a compatibility
signal, it is a changelog — and it forces readers to reject data they could have read fine.

## C3. Checksum/hash values are ALWAYS self-describing — and the one counter-example is frozen forever

Five independent designs, one answer: carry the algorithm with the value, either as an in-value
prefix or as a per-algorithm field name.

| Subject | Form |
|---|---|
| OCI | `sha256:…`, `blake3:…` — prefix inside the value, open grammar |
| Go | `h1:…`, with `h2` pre-reserved for the SHA-256 break |
| npm | `integrity: "<hashAlgorithm>-<base64-hash>"` (SRI) |
| Debian | per-algorithm *field*: `Checksums-Sha1`, `Checksums-Sha256` |
| RPM | `<checksum type="sha256">` — an attribute on the element |

The counter-example proves the rule: npm's `shasum` is a bare, untagged SHA-1. It could never be
migrated. npm added `integrity` *beside* it and both are published to this day. Debian's `Files`
(MD5) is the same story — still mandatory, retired only by a *reader* rule ("Clients may not use
the MD5Sum and SHA1 fields for security purposes").

**You cannot change what an untagged value means. You can only add a tagged one next to it and
carry both forever.**

## C4. Untagged unions are the single most reliable source of regret

Every subject that allowed a field to hold more than one shape without a discriminator later
paid for it, and the payment is always the same: a *new field* or a *new tag*, never a fix in
place.

- crates.io: `features` values widened to a new grammar ⇒ split into `features2` + `v: 2`.
- npm: `license` was string-or-object plus a plural sibling `licenses` ⇒ deprecated in favour of
  a single string form; `funding`/`bin`/`repository` remain untagged unions today.
- Debian: the same field name means different arity and different separators in different files
  (`Files`, `Binary`, `Description`); `Format` collides with itself across two file types;
  `Rules-Requires-Root` had to have a discriminator (`/` in the token) *retrofitted*.
- OCI is the exception that shows the payoff of the opposite choice — `mediaType` is REQUIRED on
  every descriptor, and the spec is able to say *"Implementations MUST NOT attempt to parse the
  referenced content if this media type is unknown."*

**A reader that must guess the shape cannot safely be lenient, because it cannot tell "unknown
variant" from "malformed known variant."**

## C5. Unknown fields are ignored rather than rejected — but almost nobody says so normatively

The behaviour is near-universal; the *specification* of the behaviour is rare.

| Subject | Normative rule? |
|---|---|
| OCI | **Yes** — the only one. SHOULD NOT error on unknown property (was MUST NOT); MUST NOT error on unknown annotation key |
| Debian | **No** — NOT FOUND in Policy; de-facto tolerance, and it differs per tool (dpkg's DB retains; dpkg's generator warns and drops; apt retains) |
| crates.io | **No** — no statement; serde defaults to ignoring (no `deny_unknown_fields`) |
| Go | **Partly** — role-scoped and stated in code comments, not the spec |
| npm | **No** — the registry docs contain no RFC-2119 language at all |

**The practical reading: leniency you did not write down is leniency you do not have.** OCI wrote
it down and *still* got burned, because registries validated anyway (§5 R1). Everyone else is
relying on a convention that no third-party implementer ever agreed to.

## C6. Absent means "the default"; almost nobody models "unknown / not recorded"

- crates.io: absent == `null` == documented default, per field.
- Debian: no `null` exists; empty is illegal or ignored; omission encodes both "no data" and
  "the default" — and for `Multi-Arch: no` omission is the *only* legal encoding.
- OCI: mandates that absent and empty be equivalent — *"MUST either be absent or be an empty
  map."*
- Go: `.info`'s `Time` is optional; absence carries no separate meaning.

The lone exception is npm's `_hasShrinkwrap`, which is genuinely three-valued with a documented
reader obligation for the third state. **If you need "we don't know", you must design it in
deliberately — no format gets it by accident, and retrofitting it is impossible because absence
already means something.**

## C7. Extension namespaces are reserved, prefixed, and policed

Every mature subject carved out a region where third parties may write, and reserved the rest.

- OCI: reverse-DNS annotation keys; `org.opencontainers` reserved against third parties *and*
  against OCI's own sibling specs.
- Debian: `X`+`[BCS]`+`-` user-defined fields — with the acknowledged flaw that the prefix is
  *stripped on output*, dumping user fields into the global namespace, mitigated by a hand-rolled
  `Private-` convention.
- Go: the strongest statement found — *"More fields may be added in the future, so other names
  are reserved."* The unused name space is pre-claimed in its entirety.
- OCI distribution: extension API paths marked by a leading `_`, "invalid for a repository name",
  with a request to register names centrally.

**Two-tier vocabularies recur:** OCI gives annotations a *harder* tolerance guarantee (MUST NOT
error) than the schema proper (SHOULD NOT error); NuGet distinguishes documented resource types
(permanent) from undocumented ones it reserves the right to remove. Saying in writing which keys
are contractual and which are yours to churn is cheap and repeatedly reinvented.

## C8. Round-trip preservation is only reliably achieved when content-addressing makes it physically necessary

- OCI: *"Implementations storing or copying content MUST NOT modify or alter the content in a way
  that would change the digest of the content."* The manifest's identity **is** its hash, so
  dropping an unknown field produces an object no descriptor points at.
- Go: *"the original, unmodified go.mod file must be returned"*, backed by `go.sum` verification.
- Debian: no normative rule; behaviour is split three ways *within dpkg alone* — the status DB
  round-trips unknown fields (but reorders them to the end), obsolete fields are deliberately
  dropped, and control-file generators warn and drop.
- crates.io sidesteps the question entirely by forbidding rewrites: *"The JSON objects should not
  be modified after they are added except for the `yanked` field."*
- NuGet is the negative case: its reader **normalises rather than preserves** — array `@type` is
  re-emitted as a bare string, malformed values degrade to `[]` silently.

**"Please preserve fields you don't understand" is not enforceable as a request. It is
enforceable as a hash.**

## C9. The version signal, where it exists, is attached to the smallest possible unit

crates.io versions the **record** (`v` on each NDJSON line), not the file. A too-new record costs
exactly one package version; every sibling version stays resolvable. Debian's `.dsc` `Format`
versions one *document*. OCI's real version lives in the media type of each *object*.

Nobody versions "the whole index". The coarser the unit, the more a single new feature costs.

## C10. Deprecation is done by redefinition and calendars, never by deletion

- Debian: `Priority: extra` was not removed — it was **redefined as an alias** ("should be
  treated as equivalent to `optional`"). The one-line `sources.list` has a published removal
  floor: *"deprecated and may eventually be removed, but not before 2029."*
- dpkg keeps **permanent read-side aliases** for renamed fields (`Recommended`→`Recommends`) with
  the writer function set to `w_null` — **read forever, write never**.
- OCI added a *warning header* channel specifically so deprecations can be announced without
  becoming errors.
- npm deprecated the `licenses` array with a warning period, not a break.

**The rename protocol that actually works: keep reading the old name forever, stop writing it
immediately, and publish a date.**

## C11. A tag's SEMANTICS freeze the moment a third party matches on it — even though its VALUE SET stays open

The two RPM findings side by side make this unusually crisp:

- **Adding new `<data type>` values** — `primary_db`, `updateinfo`, `group`, `prestodelta`,
  `modules`, `filelists-ext`, every `*_zck` — happened repeatedly over twenty years and **broke
  nobody**.
- **Changing what the existing value `group` meant** (uncompressed → compressed) **broke RHEL 7.**

The same lesson appears twice more, from unrelated ecosystems:
- RPM's `sha` vs `sha1`: *renaming a value that denotes the identical algorithm* is a breaking
  change, because the identifier string is the contract.
- OCI's withdrawn artifact manifest: a *new* media type was rejected by deployed readers precisely
  because dispatch on a discriminator is almost always a closed match.

**Practical rule: keep the discriminator's value set open and append-only, and treat every
published value's meaning — and its exact spelling — as immutable forever.**

## C12. Strict readers do not protect a format; they immobilise it

The one subject that rejects unknown elements (Maven) is the one whose schema did not move for
~20 years, and it says so about itself: *"Thus, the Maven POM syntax became fixed, unable to
change."* Its eventual escape was not a stricter schema but **a second document** — the consumer
POM, frozen at 4.0.0 forever, generated from an authored POM that is allowed to evolve.

The mirror image is equally instructive: RPM, with *no* version field at all, was chilled in the
other direction — *"does anyone know what 3.0.1 will do? Or 2.4.0?"* — and dropped a needed
extension because nothing let it reason about deployed readers.

**Both extremes stop evolution. What actually keeps a format moving is the middle: lenient
readers PLUS a version signal that says when leniency is not enough** — which is precisely
crates.io's `v` and PyPI's `meta.api-version`.

## C13. The separation that keeps recurring: the AUTHORED document is not the PUBLISHED document

Reached independently, under duress, by at least four subjects:

| Subject | Authored | Published |
|---|---|---|
| Maven | build POM (4.1.0, evolving) | consumer POM (4.0.0, frozen) — generated at build time |
| npm | `package.json` (open, publisher-controlled) | abbreviated packument (closed allow-list) |
| crates.io | `Cargo.toml` + publish API payload | index entry (different field names, extra fields, immutable) |
| Debian | `debian/control` template (may carry `X…-` fields, empty fields) | `Packages` / `.dsc` (empty fields discarded, `X` prefix stripped) |

In every case the published form is **narrower, more stable, and derived**. The index is a
*projection*, not a copy — which is what makes it possible to keep the published contract still
while the authored format moves.


---

# §4 Divergences — where they genuinely disagree, and why

Each divergence below tracks a real difference in the format's situation, not taste. That is the
useful part: it tells you which subject's answer transfers to *your* situation.

## D1. WHERE the version lives — and it tracks who controls the readers

Five different answers, and each is right for its own deployment shape:

| Subject | Version lives in | Works because |
|---|---|---|
| crates.io | an in-band field on each record (`v`) | one reader implementation (Cargo), auto-updating, and the guard was deployed years before first use |
| OCI | the **media type string** (`…manifest.v1+json`) | content is fetched through a descriptor that already carries the type; the numeric `schemaVersion` is dead |
| npm | the **HTTP `Accept` header** | only meaningful in a request/response setting — unavailable at rest |
| Debian | the **filename/extension** (`.list` vs `.sources`) | old readers simply never open the new files; both coexist in one directory |
| Go (GOPROXY) | **nowhere** | the protocol is endpoint-shaped; new capabilities become new optional endpoints |

**For a file-at-rest index read by third-party tools, two of these are unavailable outright**
(npm's header negotiation; and OCI's descriptor-carried type unless you also publish descriptors).
The two that transfer cleanly are crates.io's in-band per-record field and Debian's
new-filename trick — and they compose: a new filename for a shape change, an in-band field for a
semantic change.

## D2. Strictness is scoped by three incompatible axes

Nobody disagrees that leniency is needed; they disagree about *who* must be lenient.

- **Go scopes by ROLE (owner vs consumer).** Strict on the main module's own `go.mod` (unknown
  directive is an error), lax on dependencies' — *"we get better gradual deployments if old go
  commands simply ignore those statements when found in go.mod files in dependencies."*
- **OCI scopes by OPERATION (processing vs storing).** Processors SHOULD NOT error on unknown
  properties; storers/copiers MUST NOT alter bytes.
- **Debian scopes by TOOL, accidentally.** The same unknown field is retained by dpkg's database,
  retained by apt's tag parser, and dropped-with-a-warning by dpkg's control generator. Nobody
  designed this; it is the residue of three implementations and no normative rule.

Go's axis is the one that fits a published index: **you are strict about the file you are the
authority for, and lenient about everyone else's.** OCI's axis is the one that fits a
cache/mirror. Debian's is a warning about what happens with no rule at all.

## D3. Whether a too-new record is SKIPPED or FATAL — and it tracks whether the data is a set or a scalar

- crates.io **skips** the record and keeps going. Correct, because the file is a *set* of package
  versions and the resolver can pick another one.
- Go **refuses to build** on a too-new `go` directive. Correct, because the module version is a
  *scalar* — there is no alternative to fall back to, and proceeding would silently mis-compile.
- Debian's `.dsc` `Format` is **fatal** for the same reason — the value selects the unpacking
  algorithm; guessing is not an option.

The rule that generalises: **skip is safe when the record is one of many interchangeable
candidates; it is unsafe when the record is the answer.** Cargo's own docs record the cost of
getting this wrong at the margin — an old reader that skips a too-new entry may then resolve a
*different, wrong* version rather than reporting a problem, and RFC 3143 enumerates five distinct
bad outcomes.

## D4. Whether the version gate can be retrofitted — and it tracks reader population

- **Go retrofitted it.** The mandatory `go`-line check was backported into Go 1.19.13 and 1.20.8
  so already-shipped readers would *refuse* rather than *misread*.
- **crates.io could not.** *"This is honored as of 1.51, so unfortunately older versions will
  ignore it, and potentially misinterpret version 2 and newer entries."*

The difference is not competence, it is population: Go ships one toolchain that users patch;
crates.io's readers include arbitrary old Cargo binaries and third-party crates. **The more your
readers look like "arbitrary third-party tools", the more the guard must be present from day
one, because you will never get to add it later.**

NuGet's 2018 service-index bump shows the middle case: it was affordable only because the
downstream reader population was *countable* — the announcement @-mentioned eight named
implementers by hand and slipped the date twice in public.

## D5. New TYPE vs new FIELD — the risk is wildly asymmetric, and only OCI has paid the bill publicly

Debian, crates.io and npm add fields routinely with no incident. OCI added a new *manifest media
type*, watched producers adopt it in release candidates, and had to withdraw it:

> "A new manifest type is inherently non-portable to older registries since registries parse
> manifests and reject unknown manifest media types."

The asymmetry is structural: an unknown *field* lands inside a document the reader has already
decided to parse; an unknown *type* is evaluated *before* parsing, by dispatch logic that is
almost always a closed match. **Adding to a discriminator's value set is a breaking change in
practice even where the spec says it is not.**

## D6. Whether the format's own vocabulary is open or closed at the value level

Genuinely opposed positions:

- **Debian `Priority` is open, with the unknown value preserved verbatim** — dpkg parses with
  `PARSE_NV_FALLBACK`, stores `PKG_PRIO_OTHER` plus `otherpriority = nfstrsave(value)`. Package
  types likewise: *"Other types not defined here may be indicated."*
- **OCI digest algorithms are open by grammar** — *"Implementations SHOULD allow digests with
  unrecognized algorithms to pass validation if they comply with the above grammar"* — with a
  registry for the ones that matter and a MUST-implement floor of SHA-256.
- **NuGet resource `@type` is closed by exact string match**, with no fallback — which is why
  `SearchQueryService/3.5.0` was *silently ignored for six years* and the eventual fix had to
  convert silence into an exception.
- **Debian's `Signed-By` is the counter-case within Debian itself**: a new value in an existing
  field genuinely broke old apt — *"As APT before 1.8 will choke on this new syntax it should be
  avoided until all expected users have a sufficiently recent client."*

The pattern: openness at the value level works **only where it was designed in from the
beginning** (a fallback branch, an open grammar, a catch-all bucket). Bolting it on later does
not help, because the readers that need it are the old ones.

## D7. Documentation as a compatibility surface — and two subjects show it failing

crates.io and OCI document their formats to a level a third party can implement. Two do not, and
it shows:

- npm's own registry doc contains a field documented as **`directories`:???**, empty "Filtering"
  and "Errors" sections, and the admission *"Historically no validation has been performed on
  those fields."*
- Debian's `Multi-Arch` ran in production for ~12 years before Policy documented it (bug filed
  2014-05-30, fixed in `debian-policy/4.7.4.0`), while apt's parser silently degraded unknown
  values to `No` — a wrong answer, not an error.

**Undocumented-but-load-bearing is the worst quadrant:** third parties implement against
observed behaviour, and then the observed behaviour becomes the spec whether you like it or not.
NuGet's `/Versioned` mechanism is the same failure from the other direction — designed
specifically to decouple server capability from client release cadence, and then never
documented, so nothing uses it.


---

# §5 Regret signals — decisions reversed or publicly regretted

Ordered roughly by transferability to a JSON index published into a git repository and read by
third-party tools. Every entry carries the quote that establishes it.


### R22. PyPI: a key was shipped, an old client HARD-FAILED on it, and the spec had to be changed to route around the broken readers — PEP 714

**The single most instructive incident in this research**, because the failure was exactly
"third-party tool reads our published file and explodes".

The setup:

> "Unfortunately, PyPI did not support :pep:`658` until just recently, which released with
> a bug where the ``dist-info-metadata`` key from :pep:`658` was incorrectly named in the JSON
> representation, to be ``data-dist-info-metadata``. However, when
> attempting to fix that bug, it was discovered that pip *also* had a
> bug, where any use of ``dist-info-metadata`` in the JSON representation would cause pip to hard
> fail with an exception."

The trap:

> "The bug in pip has existed since at least ``v22.3``, which means that it has
> been released for approximately 8 months, long enough to have been pulled into
> Python releases, downstream Linux releases, baked into containers, virtual
> environments, etc.
>
> This puts us in an awkward position of having a bug on PyPI that cannot be fixed
> without breaking pip, due to a bug in pip, but that version of pip is old enough
> to have been widely deployed. To make matters worse, a version of pip that is
> broken in this way cannot install *anything* from PyPI once it fixes its bug,
> including installing a new, fixed version of pip."

The three options and the choice:

> "1. Do not change the spec, fix the bug in pip, wait some amount of time, then
>    fix the bug in PyPI, breaking anyone using an unfixed pip such that they
>    cannot even install a new pip from PyPI.
> 2. Do the same as (1), but special case PyPI so it does not emit the :pep:`658`
>    metadata for pip, even if it is available. …
> 3. Change the spec to avoid the key that pip can't handle currently, allowing
>    PyPI to emit that key and a new version of pip to be released to take
>    advantage of that key.
>
> This PEP chooses (3), but goes a little further and also renames the key in the
> HTML representation."

And the explicit acknowledgement that this violates their own norms:

> "Typically we do not change specs because of bugs that only affect one particular
> implementation, unless the spec itself is at fault, which isn't the case here:
> the spec is fine and these are just genuine bugs in pip and PyPI."

The cost of *not* renaming:

> "Without some reasonable mitigation strategy, we would have to wait until those versions of pip
> are no longer in use on PyPI, which would likely be 5+ years from now."

Rejected mitigation 1 — special-case the one broken client:

> "This PEP rejects this idea because while the simple command that only upgrades
> pip would work, if the user included *anything* else in that command to upgrade
> then the command would go back to failing, which we consider to be still too
> large of a breakage.
>
> Additionally, while this bug happens to be getting exposed right now with PyPI,
> it is really a bug that would happen with any :pep:`691` repository that
> correctly exposed the :pep:`658` metadata. This would mean that every repository
> would have to carry this special case for pip."

Rejected mitigation 2 — serve different bytes to different clients:

> "This PEP rejects this idea because supporting ``User-Agent`` detection is too
> difficult to implement in a reasonable way.
>
> 1. On PyPI we rely heavily on caching the Simple API in our CDN. If we varied
>    the responses based on ``User-Agent``, then our CDN cache would have an
>    explosion of cache keys for the same content…"

— all PEP 714, <https://raw.githubusercontent.com/python/peps/main/peps/pep-0714.rst>, Status
Accepted, created 2023-06-06, resolved 2023-06-27; fetched raw and accessed 2026-08-09.

**Four transferable lessons, all first-hand:**
1. "Readers MUST ignore unknown keys" is worth exactly as much as the readers' actual compliance.
   PEP 691 *had* that rule. pip violated it. The rule did not help.
2. Once a broken reader is in distros and containers, your remediation window is **5+ years**.
3. The only fix that works against a hard-failing reader is **a key name it has never heard of** —
   renaming is the escape hatch, and it works *because* the old reader ignores absent keys.
4. Per-client behaviour (User-Agent, special-casing) was rejected partly for **caching** reasons —
   the same reason a static file index cannot vary its bytes per reader.

### R23. PyPI: header-based versioning was considered and REJECTED **specifically to keep static file hosting viable**

The most decision-relevant sentence in the entire research for a git-hosted index:

> "Using a Header
> --------------
>
> Instead of baking this information into the actual HTML, an
> alternative would be to use a HTTP header. This idea was
> considered and ultimately was rejected because it would make
> mirrors have to start modifying headers instead of being able
> to operate as a "dumb" HTTP server of files."

And URL-based versioning was rejected for a different, equally sharp reason:

> "Using an URL
> ------------
>
> Another traditional mechanism for versioning APIs is to bake it
> into the URL, something like ``/1.0/simple/`` or so. This works
> well for major version changes where olders clients are not
> expected to be capable of continuing to use it, but it is not
> well suited to minor version bumps, particularly when the version
> numbers can be viewed as largely advisory for end users."

— PEP 629, <https://raw.githubusercontent.com/python/peps/main/peps/pep-0629.rst>, Status Final,
created 2020-07-16; fetched raw and accessed 2026-08-09.

**This is npm's mechanism (§1.5 Q5) being rejected on exactly the grounds that make it unusable
for files at rest — and npm's own ecosystem then proved the rejection right** (private registries
returning 406, §5 R9). Two independent ecosystems, opposite choices, and the one that chose the
header is the one whose users had to build q-value fallbacks.

The motivating incident, stated in PEP 629's Rationale:

> "An example of a scenario where this happened was the phasing in of
> python-requires metadata, while existing clients could still successfully
> use the repository, they were lacking the ability to understand this new
> piece of data which would have informed their behavior to select a better
> file for end users."

**Note what the problem was: not a crash, but a silently worse answer.** Old clients "could still
successfully use the repository" while quietly choosing wrong files. That is the failure mode a
version field exists to make visible.

### R24. RPM: changing a checksum default broke old clients — and the two SPELLINGS of the same algorithm were not interchangeable

> "-s --checksum
> Choose the checksum type used in repomd.xml and for packages in the metadata. The default is
> now "sha256" (if python has hashlib). The older default was "sha", which is actually "sha1",
> however explicitly using "sha1" doesn't work on older (3.0.x) versions of yum, you need to
> specify "sha"."

— `createrepo(8)`, <https://linux.die.net/man/8/createrepo>, accessed 2026-08-09 (first-hand).

Two stacked breakages: (a) the default moved to an algorithm old clients could not compute; (b)
**`sha` and `sha1` name the same algorithm and are not interchangeable to a reader** — a publisher
who tidied the label broke clients without changing a single hashed byte.

**Generalisation: in a closed vocabulary, the identifier string IS the contract, independent of
what it denotes. Cosmetic renaming of a value is a breaking change.**

### R25. Maven: strict readers froze the format for two decades

> "modelVersion — String — Declares to which version of project descriptor this POM conforms to:
> 4.0.0 is the only value supported by Maven 3."

— <https://maven.apache.org/ref/current/maven-model/maven.html>, accessed 2026-08-09.

> "The POM contains invalid/unknown elements that Maven does not understand."

— Apache Maven wiki, *ModelParseException*,
<https://cwiki.apache.org/confluence/display/MAVEN/ModelParseException>, accessed 2026-08-09.

Not a quoted *confession* of regret, but the strongest structural evidence in the set: the one
format that rejects unknown elements is also the one whose schema version has not moved in ~20
years. **Strictness did not protect the format; it immobilised it.** Every other subject here
added fields freely and survived.

### R26. NuGet: versioning inside the discriminator was designed to decouple client and server, and did the opposite

> "The previous model used hardcoded type strings which made it impossible for a server to
> optimize or support or new feature without breaking older clients unless the client added a
> new type."

— NuGet/Home#4206, `emgarten`, 2017-01-05 (metadata verified independently; body quote from the
delegated sweep, raw issue body). The replacement mechanism (`/Versioned` + `clientVersion`) was
then **never documented**, so nothing adopted it — and six years later:

> "As a result, `SearchFilter.PackageTypes` has been silently ignored by nuget.org and other
> compliant V3 sources for as long as the property has existed publicly."

— NuGet.Client PR #7395, merged 2026-06-10 (single-sourced; corroborated by shipped code read
first-hand by the delegated agent).

**A closed vocabulary with no unknown-value fallback does not fail loudly — it fails silently for
years, and the eventual fix is itself a breaking change.**

### R27. NuGet: a schema version that went BACKWARDS, undetected

`3.0.0-rc.1` (Feb 2015) → `3.0.0-beta.1` (Jan 2016) → `3.0.0` (by Aug 2018). Under SemVer 2.0.0
precedence `3.0.0-beta.1 < 3.0.0-rc.1`. Nothing detected it because the client only ever evaluates
`version.Major != 3`. (Verified independently by the delegated agent against Wayback raw captures
plus a live fetch, 2026-08-09.)

**A version field no reader fully evaluates is decoration, not a contract — and it will rot
without anyone noticing.**

### R28. RPM: **the only fatal break in twenty years was redefining what an EXISTING tag value MEANS** — adding new values never broke anyone

This is, on the evidence assembled here, the single most important regret in the corpus.
`[FH✓]` Fedora Change "createrepo_c 1.0.0":

> "When adding groups.xml to repodata createrepo_c currently adds two variants to repomd.xml. The
> specified file as is, uncompressed, with the type "group" and also a compressed variant with type
> "group_XX", where XX is compression suffix. This is atypical and unexpected. We propose to include
> just one variant of groups.xml using specified compression and repomd.xml type "group". This is
> not compatible with yum in RHEL 7."

> "Repodata created with createrepo_c's new defaults will not be compatible with fedora < 30 and
> RHEL < 8.4 (including RHEL 7, 6). This is mainly due to zstd compression but old YUM also cannot
> handle the changed mdtype of groups (it is compatible only with uncompressed - "group" and
> compressed - "group_XX" not compressed - "group")."

— <https://fedoraproject.org/wiki/Changes/createrepo_c_1.0.0>, accessed 2026-08-09.

Set that against twenty years of `primary_db`, `updateinfo`, `group`, `prestodelta`, `modules`,
`filelists-ext` and every `*_zck` type being **added** without incident. The asymmetry is total:

- **Adding a new value to an open discriminator: safe, done dozens of times, never broke anyone.**
- **Changing the invariant attached to an existing value: broke a supported enterprise distro.**

Readers had encoded "type `group` means *uncompressed*" as an unstated assumption. Nothing in the
format said so, and nothing could have stopped them. **A tag's semantics are frozen the moment a
third party matches on it.**

### R29. Maven: the freeze is **self-diagnosed on Maven's own website**

`[SS]` The clearest statement anywhere of what a strict, namespace-bound published format costs:

> "This made Maven more than a tool; it became a whole ecosystem with many dependencies on the POM,
> especially the Maven Central repository, other build tools, and IDEs. Thus, any change in the
> POM's schema forces each participant of the ecosystem to either adopt the change or drop support.
> Thus, the Maven POM syntax became fixed, unable to change."

and Hervé Boutemy, quoted on the same page:

> "With the Maven build schema preserved in amber, we can't evolve much: we'll stay forever with
> Maven 3 minor releases, unable to implement improvements that we imagine will require seriously
> updating the POM schema…"

`[FH]` The structural cause is one line in the model descriptor:
`xml.namespace="http://maven.apache.org/POM/${version}"` — **the namespace is templated from the
version, so every model-version bump is a namespace break.**

`[SS]` And the argument is **still live in July 2025** — dev@maven.apache.org, *"[DISCUSS] Maven 4.0
Namespace Strategy - Breaking Change Timing Discussion"* (Guillaume Nodet):

> "*However*, this represents a *major breaking change*." / "Should we introduce a major breaking
> change in the 4.x series, or wait for 5.0?"

with Boutemy replying:

> "wow, it feels strange to have that discussion in 2025 after 20 years of practice :)" …
> "changing the practice for POM = the most sensitive place - may be risky"

**Twenty years of not being able to change a schema, and the discussion is still about whether it
is safe to try.**

### R30. Maven: a twenty-year-old wish-list still sitting in the model source

`[FH]` The `Improvements:` header of `maven.mdo`, on `master` today:

> "o use enums where appropriate (eg dependency scope)"
> "o validators: there could be several levels of validation…"
> "o annotation mechanism so that changes to the model can be accurately tracked."

All three are precisely the properties under decision in this research; all three remain open.

### R31. RPM: with no version field, nobody could say what deployed readers would do — so a needed extension was simply **not made**

`[SS]` yum mailing list, June 2009, on adding a second checksum per `<data>` record. Seth Vidal:

> "if we start doing this we run the risk of breaking any of the non-yum depsolvers."

James Antill:

> "I think it's worse than that, for instance does anyone know what 3.0.1 will do? Or 2.4.0?"

The extension was dropped. **This is the cost of "no version field" stated by the people paying
it**: not a broken reader, but a *chilling effect* — you cannot reason about a change you cannot
gate, so you stop proposing changes. It is the exact complement to R29 (Maven froze because its
version gate was too strict) and it lands in the same place from the opposite direction.

### R32. RPM: "we preserve what we don't understand" is a trap without a published reserved-prefix registry

`[SS]` Preservation is gated on `--update`, and "additional metadata" is decided by a hard-coded
`g_str_has_prefix` test against `primary_`, `filelists_`, `filelists-ext_`, `other_`. A third-party
record whose type merely *starts with* one of those four strings is **silently dropped on every
regeneration, with no warning**. The four reserved prefixes are documented nowhere; they are
discoverable only by reading C.

**If you promise to preserve unknown entries, you must publish the exact rule that decides which
entries are "unknown" — otherwise the rule is a hidden reserved namespace.**

### R33. RPM: an untyped scalar drifted int→float, and the reader had to keep the narrow parse deliberately

`[SS]` In-code note in yum's parser:

> "# NOTE: This will fail on float timestamps, this is required for compatibility. Fix is to not
> generate float timestamps in repomd.xml."

Some producer began emitting a float where readers expected an integer; the reader chose to keep
failing rather than widen, because widening would itself change behaviour. **The exact failure mode
of "the schema is prose": nothing ever said which numeric type `<timestamp>` was.**

### R34. Maven: a closed vocabulary the schema never actually closed, enforced at two different severities

`[FH]` `scope` is `xs:string` in **both** published XSDs — the enumeration exists only inside a
validator. `[SS]` And its severity was deliberately lowered because third-party extensions had
already invented values:

> "TODO Extensions like Flex Mojos use custom scopes like "merged", "internal", "external", etc. In
> order to don't break backward-compat with those, only warn but don't error out."

while the *plugin-dependency* variant of the same field is ERROR-severity against a different list
(`[SS]` TUSCANY-3845). **One concept, two vocabularies, two severities, neither in the schema.**

### R35. Maven: `maven-metadata.xml` — the merge arbiter is a string comparison, and real published files have contained the literal `"null"`

`[SS]` `lastUpdated` is an `xs:string` `yyyyMMddHHmmss` compared with `String.compareTo`, with
defensive handling for the literal `"null"` — meaning real deployed metadata has contained
`<lastUpdated>null</lastUpdated>`. `[SS]` MNG-8106 (Fixed): overlapping group/artifact directory
roles cause outright data loss — *"Affected maven versions will simply drop "the other" metadata"*.
`[SS]` MNG-7375 (**Open**): invalid metadata *published on an ASF repository* NPEs the merge.
`[SS]` MRESOLVER-363: two Maven components disagree on the key name for `lastUpdated`.

**A published index whose ordering key is an unvalidated string is a published index that will
eventually contain `"null"`.**

### R1. OCI: a new manifest media type was specified, adopted, then **withdrawn** — because deployed readers reject unknown types

The single best-documented reversal found. OCI specified
`application/vnd.oci.artifact.manifest.v1+json`, shipped it in release candidates, saw
producers adopt it, and then **deleted it before 1.1.0 final**.

> "Notably absent from this release is a new type of manifest dedicated to artifacts.
> This was removed from the release candidates because of a combination of portability
> concerns and a lack of added value. A new manifest type is inherently non-portable to older
> registries since registries parse manifests and reject unknown manifest media types.
> This would typically be handled by defining a new type, upgrading the various consumer tools
> and registries to be ready, and later upgrade the tooling that produces artifacts after time
> had passed to allow a controlled upgrade.
> In practice, OCI saw a notable uptake of the new manifest by content producers in the very
> early release candidates, long before registries and other consumers had the opportunity to
> upgrade. While debating whether and how OCI could phase a rollout of a new manifest type,
> the alternative of reusing the existing image manifest was revisited.
> The image manifest was already being used by artifact producers, including projects like
> Helm and Sigstore.
> By adding the artifactType field to the image manifest, and defining the empty JSON
> descriptor, all of the use cases that previously required a new artifact manifest could be
> retrofitted into the existing image manifest. Without any requirements that necessitated the
> new manifest type, and no easy solution to the portability concerns, the difficult decision
> was made to remove the new manifest type from the release candidates.
> Future work in OCI may revisit a new manifest type by focusing on use cases and capabilities
> that cannot be provided with the existing manifests."

— OCI blog, *OCI Image and Distribution Specs v1.1 Releases*, published 2024-03-13,
<https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/>,
accessed 2026-08-09.

The removal PR is *Remove artifact manifest* — <https://github.com/opencontainers/image-spec/pull/999>
(title verified 2026-08-09). The empirical study that preceded it is
<https://github.com/opencontainers/image-spec/issues/1025>, titled:

> "Fact finding: Registry & client support for Image Manifest type artifacts"

— accessed 2026-08-09; the issue is closed as not planned. (I could not extract the issue's
result tables verbatim through the fetch tooling; **the decision rationale I rely on is the
blog quote above, which is authoritative and first-party.**)

**Why it matters here.** The whole point of a `mediaType`/kind discriminator is that readers
can route on it — but that same discriminator is exactly what a strict reader validates
against a closed list. OCI's own extensibility rules said readers must not error on unknown
values; real registries errored anyway. **A tolerance rule that third-party implementations
have not yet shipped is not a tolerance rule.** The recovery was to make the new thing a
*new optional field inside the existing type* rather than a *new type*.

### R2. OCI: the unknown-property rule was **weakened from MUST to SHOULD**, and narrowed

At `v1.0.0` and `v1.0.2`:

> "Implementations that are reading/processing [manifests](manifest.md) or
> [image indexes](image-index.md) MUST NOT generate an error if they encounter an unknown
> property.
> Instead they MUST ignore unknown properties."

At `v1.1.0`, `v1.1.1` and `main`:

> "Implementations processing content SHOULD NOT generate an error if they encounter an
> unknown property in a known media type."

— `considerations.md` at tags `v1.0.0`, `v1.0.2`, `v1.1.0`, `v1.1.1`, `main`; all fetched and
diffed 2026-08-09 from `https://raw.githubusercontent.com/opencontainers/image-spec/<tag>/considerations.md`.

Three changes at once: **MUST NOT → SHOULD NOT** (moving the rule out of the compliance
boundary, per `spec.md`'s definition of compliance over MUST-class terms only); the added
qualifier **"in a known media type"**; and the deletion of the flat imperative "Instead they
MUST ignore unknown properties."

In the same edit the spec **gained** a hard rule aimed at a different audience — content
preservation, which did not exist at `v1.0.2`:

> "Implementations storing or copying content MUST NOT modify or alter the content in a way
> that would change the digest of the content."

**Reading of the reversal:** the strong "everyone must ignore unknown properties" rule was
unenforceable against registries that validate. It was replaced by a split: *processors*
get a soft ignore-rule, *storers/copiers* get a hard don't-touch rule. The obligation moved
from "understand leniently" to "**preserve exactly**". I could not identify the specific PR;
the file's commit history lists a candidate — "spec: clarify descriptor, align with de facto
artifact usage", 2023-05-04 — but I did not verify its diff, so the PR attribution is
**NOT FOUND** and only the tag-to-tag diff is asserted.

### R3. OCI: `schemaVersion` is a dead field the spec would like to delete

> "This REQUIRED property specifies the image manifest schema version.
> For this version of the specification, this MUST be `2` to ensure backward compatibility
> with older versions of Docker. The value of this field will not change. This field MAY be
> removed in a future version of the specification."

— `manifest.md`, present identically at `v1.0.2` and `main`, accessed 2026-08-09.

A REQUIRED integer schema-version field, frozen forever at a constant inherited from a
predecessor format, with no reader contract, publicly marked for possible removal. The real
versioning moved into the media type string. **This is the strongest available evidence that
a bare numeric `schemaVersion` with no defined reader behaviour is worse than useless — it
occupies the name and teaches readers nothing.**

### R4. OCI: the manifest's own type tag had to be promoted from "reserved" to "SHOULD be used"

`v1.0.0`/`v1.0.1`:

> "This property is *reserved* for use, to [maintain compatibility](media-types.md#compatibility-matrix)."

`v1.0.2` onward:

> "This property SHOULD be used and [remain compatible](media-types.md#compatibility-matrix)
> with earlier versions of this specification and with other similar external formats.
> When used, this field MUST contain the media type `application/vnd.oci.image.manifest.v1+json`."

— `manifest.md` at tags `v1.0.0`, `v1.0.1`, `v1.0.2`, verified 2026-08-09.

Self-description inside the document was initially treated as optional decoration; it had to
be strengthened once documents started travelling without their descriptors.

### R5. crates.io: the `v` guard was shipped **undocumented and unused, years early** — and the readers older than it can never be protected

> "Currently this is not written anywhere, and is intended as a safety guard for the future.
> For now I will leave it undocumented until we actually decide to start using it."

— ehuss, PR #9161, 2021-02-10, <https://github.com/rust-lang/cargo/pull/9161>, accessed
2026-08-09 (verified against raw page HTML).

The cost of not having done it earlier is stated in the shipped source:

> `/// This provides a method to safely introduce changes to index entries`
> `/// and allow older versions of cargo to ignore newer entries it doesn't`
> `/// understand. This is honored as of 1.51, so unfortunately older`
> `/// versions will ignore it, and potentially misinterpret version 2 and`
> `/// newer entries.`

— `crates/cargo-util-schemas/src/index.rs`, accessed 2026-08-09.

RFC 3143 enumerates the five distinct failure modes this produces in the wild — an unusually
honest piece of design documentation:

> "This means that running `cargo update` with a version older than 1.51 (published
> 2021-03-25) may not work correctly when updating a package that starts using the new syntax.
> This can have any of the following behaviors:
>
> 1. It will update to the new version and work just fine if nothing actually uses the new
>    feature syntax.
> 2. It will skip the package if something requires one of the new features.
> 3. It will update and successfully build, but build with the wrong features (because the new
>    features aren't enabled correctly).
> 4. It will update and the build will fail, because a new feature that is required isn't
>    enabled.
> 5. The update will fail if a matching version can't be found, since the required features
>    aren't available."

and concedes the user-facing consequence:

> "The errors that versions of Cargo older than 1.51 may generate when trying to use a
> dependency using the new syntax can be confusing."

— RFC 3143, accessed 2026-08-09.

### R6. crates.io: an alternative — negotiating format support with the registry — was built and then **abandoned as unnecessary**

> "A new publish API could be added (endpoint `api/v2/crates/new`) to ensure that Cargo is not
> speaking to a registry that does not understand the new syntax.
> This was pursued in [PR #9111](https://github.com/rust-lang/cargo/pull/9111), but it was
> considered not necessary.
> [crates.io] is the only registry that can support older versions of Cargo.
> Other registries that don't support the new syntax may reject publishing with the new syntax
> (if they perform validation), or they may accept it (if the don't validate), in which case it
> should just work.
> The `"v"` field addition is only necessary for Cargo versions between 1.51 and whenever this
> is stabilized, and most use cases of other registries are generally expected to have stricter
> control over which versions of Cargo are in use."

— RFC 3143, accessed 2026-08-09 (typo "if the don't validate" is in the source).

A version-negotiation endpoint was written, then dropped in favour of the cheap in-band
per-record version tag. **Negotiation lost to a field.** Directly relevant to a file-at-rest
index, where negotiation is not even available.

### R7. crates.io: an internal review caught the version-skip interacting badly with caching

> "if Cargo version 3 reads a cache file from Cargo version 2, I think that's the case I was
> worried about? (where version 2 didn't cache any version 3 entries)"

— alexcrichton, review comment on PR #9161, <https://github.com/rust-lang/cargo/pull/9161>,
accessed 2026-08-09. *(Quote obtained via page-summarisation tooling, not verified against
raw HTML — treat as lower-confidence than the other quotes in this document.)*

The hazard: a reader that **skips** records it does not understand and then **caches** the
filtered result poisons the cache for a later, more capable reader. Skip-unknown and
cache-derived-artefacts do not compose unless the cache is keyed by reader version.

### R8. Go: the `go` directive was **tightened from advisory to mandatory** — and the check was backported into already-released readers

> "Before Go 1.21, Go toolchains treated the go line as an advisory requirement: if builds
> succeeded the toolchain assumed everything worked, and if not it printed a note about the
> potential version mismatch. Go 1.21 changed the go line to be a mandatory requirement
> instead. This behavior is partly backported to earlier language versions: Go 1.19 releases
> starting at Go 1.19.13 and Go 1.20 releases starting at Go 1.20.8, refuse to load workspaces
> or modules declaring version Go 1.22 or later."

— <https://go.dev/doc/toolchain>, accessed 2026-08-09.

Also stated in the modules reference:

> "Before Go 1.21, the directive was advisory only; now it is a mandatory requirement: Go
> toolchains refuse to use modules declaring newer Go versions."

— <https://go.dev/ref/mod>, accessed 2026-08-09.

This is the mirror image of R5. Cargo documented that its pre-guard readers can never be
fixed; Go **went back and fixed them**, by shipping the version check in patch releases of
two already-shipped major versions. It is the only instance found of a project retrofitting
a forward-compatibility gate into readers already in the field. It was affordable only
because Go controls its own single reader implementation and ships patch releases people
actually install — a condition that does not hold for a format read by arbitrary third-party
tools.

### R9. npm: version-by-`Accept`-header became an interop hazard

> "Some private package registries, or at least an older version of at least one of them, do
> not understand this header, and return HTTP 406 Not Acceptable responses."

> "The registry specification recommends that `Accept: application/vnd.npm.install-v1+json;
> q=1.0, application/json; q=0.8, */*` be used to provide a more tolerant fallback for
> non-compliant registries."

> "the fallback path leads to potentially 10s MBs of unnecessary payload for non-conforming
> registries, but the alternative (as far as I can tell so far) is that they don't work at all."

— tchetwin, oven-sh/bun issue #341, *"Some private registries give 406 for
`Accept: application/vnd.npm.install-v1+json` header"*, 2022-07-07,
<https://github.com/oven-sh/bun/issues/341>, accessed 2026-08-09.

And the mechanism silently regressing on npm's own client:

> "when running Npm install with the latest version Npm does not send any more the
> `accept: 'application/vnd.npm.install-v1+json'` header."

> "as a result, we are getting the full metadata and not the abbreviated metadta"

— npm/cli issue #7529, *"[BUG] Npm install not sending accept:
'application/vnd.npm.install-v1+json' header"*, opened 2024-05-15,
<https://github.com/npm/cli/issues/7529>, accessed 2026-08-09 (typo in original).

**Reading:** npm put its format version in a request header. Third-party servers that did not
implement it returned a hard error rather than degrading, so every client had to carry a
q-value fallback and be prepared to download the 10 MB variant. The version signal is
invisible in the artefact itself.

### R10. npm: an untagged union that had to be deprecated, and a hash field that could never be fixed

`license` was string-or-object, with a plural sibling `licenses` holding an array of objects:

> "Some old packages used license objects or a "licenses" property containing an array of
> license objects"

> "Those styles are now deprecated. Instead, use SPDX expressions"

— <https://docs.npmjs.com/cli/v11/configuring-npm/package-json/>, accessed 2026-08-09.

And `shasum` — an untagged SHA-1 — could not be migrated in place; a *new* field with the
algorithm carried inside the value was added beside it, and both are published forever:

> "- `shasum`: the SHA-1 sum of the tarball
> - `integrity`: since Apr 2017, string in the format `<hashAlgorithm>-<base64-hash>`"

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

### R11. npm: storage-engine identifiers became permanent public API

> "- `_id`: the package name, used as an ID in CouchDB
> - `_rev`: the revision number of this version of the document in CouchDB"

— `docs/responses/package-metadata.md`, accessed 2026-08-09.

And the documentation lost the race with the format:

> "- `directories`:???"

— `docs/REGISTRY-API.md`, accessed 2026-08-09 (verbatim, entire entry).

### R12. Debian: the deb822 sources format was shipped, **disabled mid-flight, field-renamed, and re-shipped under a new file extension**

The strongest reversal *chain* found in any subject — a format that was published, retracted,
renamed and republished:

> "rename \"Dist:\" in deb822 style sources.list to \"Suite:\"" / "rename URL to Uri in
> deb822-sources"

— apt 0.9.14.3~exp2, 2014-01-16.

> "set APT::Sources::Use-Deb822=false until the format is fully finalized"

— apt 0.9.14.3~exp5, 2014-01-24.

> "bring back deb822 sources.list entries as .sources"

— apt 1.1~exp9.

— all from apt `debian/changelog`,
<https://salsa.debian.org/apt-team/apt/-/raw/main/debian/changelog>, accessed 2026-08-09.

The lesson is about *sequencing*: the format was exposed to users before its field names were
settled, and the only clean exit was to turn it off, rename the fields, and reintroduce it
under a different filename so that old and new could not be confused.

### R13. Debian: the legacy one-line `sources.list` is being retired — with a **dated removal floor**, not a version bump

> "This format is deprecated and may eventually be removed, but not before 2029."

> "It is intended to make this format gradually the default format, deprecating the previously
> described one-line-style format, as it is easier to create, extend and modify for humans and
> machines alike especially if a lot of sources and/or options are involved."

— `sources.list(5)`, apt 3.3.2,
<https://manpages.debian.org/unstable/apt/sources.list.5.en.html>, accessed 2026-08-09.

> "Deprecate legacy one-line-style format harder. Removal not before 2029."

— apt 2.9.26, 2025-01-28, apt `debian/changelog`, accessed 2026-08-09.

**A calendar date, published years ahead, is Debian's migration mechanism.** Not a schema
version, not negotiation — a promise about when readers may stop supporting the old shape.

### R14. Debian: `Priority: extra` deprecated — the distinction was admitted to be arbitrary and unmaintained

> "*This priority is deprecated.* Use the `optional` priority instead. This priority should be
> treated as equivalent to `optional`.
> The `extra` priority was previously used for packages that conflicted with other packages and
> packages that were only likely to be useful to people with specialized requirements. However,
> this distinction was somewhat arbitrary, not consistently followed, and not useful enough to
> warrant the maintenance effort."

— Policy §2.5, <https://www.debian.org/doc/debian-policy/ch-archive.html#priorities>, accessed
2026-08-09.

> "Yes, I think the will in the project to maintain \"optional\" separate from \"extra\" is gone."

— Jonathan Nieder, 2014-08-25, <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=759260>,
accessed 2026-08-09.

Note the *retirement protocol*: the value was not removed from the vocabulary, it was
**redefined as an alias** ("should be treated as equivalent to `optional`"). Old data keeps
parsing; new data stops being produced. It took ~3 years from proposal to landing in Policy
4.0.1.0.

### R15. Debian: a field specified too broadly accidentally applied to the whole archive

> "Based on discussion in debian-devel, the current Built-Using description would imply that it
> had to be present for, say, the code from libgcc incorporated into every binary build.  The
> description should be modified to be clear that it is only mandatory if there are licensing
> reasons why we have to be aware of the original source."

— Russ Allbery, 2012-09-20.

> "The basic problem that we're trying to solve is that nearly every package in Debian
> incorporates code from gcc and/or libc into the resulting binary.  So, currently, Policy says
> that basically every package in the archive needs Built-Using.  This obviously isn't what we
> want to have happen."

— Russ Allbery, 2013-09-23. Both <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=688251>,
accessed 2026-08-09.

Fixed in Policy 4.1.3 (Dec 2017); the *over*correction then required a second field —
"Dpkg::Control::Fields: Add Static-Built-Using field support." (dpkg 1.21.3, 2022-03-24, dpkg
`debian/changelog`, accessed 2026-08-09). **One under-specified field became two fields and a
decade of argument.**

### R16. Debian: `DM-Upload-Allowed` — a field added, then obsoleted, forcing the invention of an "Obsolete fields" section

> "now that the implementation changed …, I propose the following patch to obsolete the
> DM-Upload-Allowed field. This patch creates a new subsection for obsoleted fields."

— <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=679326>, accessed 2026-08-09.

> "5.6.25, 5.8.1 The `DM-Upload-Allowed` field is obsolete. Permissions are now granted via
> `dak-commands` files."

— upgrading-checklist, Version 3.9.5, October 2013, accessed 2026-08-09.

> "The following fields have been obsoleted and may be found in packages conforming with previous
> versions of the Policy."

— Policy §5.8, accessed 2026-08-09. The format had no place to put a retired field until one was
needed; **a graveyard section is itself a schema-evolution feature.**

### R17. Debian: a derived key was validated too strictly, then loosened twice

> "If the value is specified, it must be a hex MD5 digest and must consist solely of the digits
> 012345679, and the lowercase characters abcdef. If the value contains any other character, such
> as uppercase characters, the behaviour is unspecified."
> "Since APT 1.4, any 32-byte string is supported. Starting with APT 2.7.7, the key is no longer
> validated at all and may be any string that matches up with the one in the translation file."

— `DebianRepository/Format` wiki (`Description-md5`), accessed 2026-08-09.

A field whose *name* claims an algorithm ("md5") ended up as an opaque correlation key with no
validation at all. **The name outlived its meaning** — a direct argument against encoding an
algorithm in a field name.

### R18. Debian: two relation operators that silently meant the opposite of what they looked like — tolerated for ~20 years

> "The deprecated forms \`<' and \`>' were used to mean earlier/later or equal, rather than
> strictly earlier/later, so they should not appear in new packages (though \`dpkg' still supports
> them)."

— quoted from Policy 7.1 in <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=663918>
(2012-03-14, proposing "should not" → "must not"), accessed 2026-08-09.

Tightened only in Policy 4.0.0 — "The deprecated relations `<` and `>` now must not be used." —
and dpkg *still* warns rather than rejects. **A value whose meaning is a footgun cannot be
withdrawn once published; it can only be discouraged.**

### R19. Debian: optional-index probing caused real operational damage

> "floods APT repositories without i18n/Translation-* files with unneeded requests -- please
> avoid requesting those unless they are known to be present"

> "As you can see -- those often constitute the MAJORITY of the logged transactions to the
> server, leading to wasted bandwith and log files \"pollution\" and disk space waste."

— Yaroslav Halchenko, 2014-05-07, <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=747321>,
accessed 2026-08-09 (typo in original).

**The cost of "absence is discovered by trying".** When a format leaves optional files
undeclared, readers probe, and probing at scale is a denial-of-service against your own mirrors.
An explicit manifest of what exists (RPM's `repomd.xml`, §1.7) is the counter-design.

### R20. Debian: documentation debt as a first-class failure — `Multi-Arch` ran in production for over a decade before being specified

> "the use of \"Multi-Arch:\" field (and so the values: \"same\", \"foreign\" etc.) in
> debian/control file should probably be documented in Debian Policy."

— Bob Bib, 2014-05-30, <https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=749826>, accessed
2026-08-09; the bug page records it fixed in `debian-policy/4.7.4.0` (2026-03-31).

Twelve years from "please document the field everyone already depends on" to a specification.
Meanwhile third-party readers had to reverse-engineer it — and apt's parser degrades unknown
values to `No` (§1.3 Q1), which is a *silent wrong answer*, not an error.

### R21. Debian: even the terminology was reversed

> "The stanzas somtimes used to be referred to as paragraphs, but that caused confusion with text
> paragraphs in prose, so it is now considered a discouraged term."

— Policy §5.1 footnote, accessed 2026-08-09 (typo "somtimes" in original).


---

# §6 Re-fetch list

Every URL used, with access date and — where knowable — the version or date of the document
itself, plus a confidence marker. **Confidence reflects how the text reached this document:**
*raw source fetched by me* > *HTML fetched and tag-stripped locally by me* > *delegated agent,
independently re-verified* > *delegated agent, single-sourced* > *page-summarisation tooling*.



## crates.io / Cargo

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://doc.rust-lang.org/cargo/reference/registry-index.html> | Cargo Book, stable channel (content includes `pubtime`, so ≥ Cargo 1.9x) | 2026-08-09 | High — fetched as HTML, tags stripped locally |
| <https://raw.githubusercontent.com/rust-lang/cargo/master/crates/cargo-util-schemas/src/index.rs> | `master` branch HEAD | 2026-08-09 | High — raw source |
| <https://raw.githubusercontent.com/rust-lang/rfcs/master/text/3143-cargo-weak-namespaced-features.md> | RFC 3143, final text | 2026-08-09 | High — raw source |
| <https://raw.githubusercontent.com/rust-lang/rfcs/master/text/2789-sparse-index.md> | RFC 2789, final text | 2026-08-09 | High — raw source |
| <https://github.com/rust-lang/cargo/pull/9161> | opened 2021-02-10, merged 2021-02-22T16:26:28Z | 2026-08-09 | High for PR body (verified against raw HTML); **Low** for the alexcrichton review comment (summarisation tooling only) |
| <https://github.com/rust-lang/cargo/pull/9111> | referenced only, not fetched | — | Cited via RFC 3143's description of it |

*Note:* `https://raw.githubusercontent.com/rust-lang/cargo/master/src/doc/src/reference/registry-index.md`
returns **404** — the Cargo book source is not at that path on `master`. Use the rendered
doc.rust-lang.org page, or `crates/cargo-util-schemas/src/index.rs` for the authoritative
struct.

## OCI Image Spec

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| `https://raw.githubusercontent.com/opencontainers/image-spec/main/{descriptor,manifest,image-index,spec,considerations,annotations}.md` | `main` branch HEAD | 2026-08-09 | High — raw source |
| `https://raw.githubusercontent.com/opencontainers/image-spec/v1.0.0/{manifest,considerations}.md` | tag `v1.0.0` (2017-07) | 2026-08-09 | High — raw source |
| `https://raw.githubusercontent.com/opencontainers/image-spec/v1.0.1/manifest.md` | tag `v1.0.1` | 2026-08-09 | High — raw source |
| `https://raw.githubusercontent.com/opencontainers/image-spec/v1.0.2/{manifest,considerations}.md` | tag `v1.0.2` | 2026-08-09 | High — raw source |
| `https://raw.githubusercontent.com/opencontainers/image-spec/v1.1.0/considerations.md` | tag `v1.1.0` (released 2024-02-15) | 2026-08-09 | High — raw source |
| `https://raw.githubusercontent.com/opencontainers/image-spec/v1.1.1/considerations.md` | tag `v1.1.1` | 2026-08-09 | High — raw source |
| <https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/> | published 2024-03-13 | 2026-08-09 | High — fetched as HTML, tags stripped locally |
| <https://github.com/opencontainers/image-spec/pull/999> | "Remove artifact manifest" | 2026-08-09 | Title only |
| <https://github.com/opencontainers/image-spec/issues/1025> | "Fact finding: Registry & client support for Image Manifest type artifacts"; closed as not planned | 2026-08-09 | Title only — result tables not extracted |

*Note:* the GitHub REST API (`api.github.com`) rate-limited unauthenticated requests during this
session, so the PR/commit that performed the MUST→SHOULD change was **not** pinned. The
tag-to-tag diff is the asserted evidence.

## Debian / deb822

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://www.debian.org/doc/debian-policy/ch-controlfields.html> | Policy **v4.7.4.1** | 2026-08-09 | High — 4 quotes independently re-fetched and matched exactly |
| <https://www.debian.org/doc/debian-policy/ch-archive.html> | Policy v4.7.4.1 | 2026-08-09 | Medium-high — delegated fetch |
| <https://www.debian.org/doc/debian-policy/upgrading-checklist.html> | Policy v4.7.4.1 | 2026-08-09 | Medium-high — delegated fetch |
| `https://salsa.debian.org/dbnpolicy/policy/-/raw/master/policy/*.rst` | git master; `debian/changelog` head 4.7.4.2 UNRELEASED (2026-05-28) | 2026-08-09 | Medium-high — delegated fetch |
| <https://manpages.debian.org/unstable/dpkg-dev/deb822.5.en.html> and `deb-control.5`, `deb-src-control.5`, `dsc.5`, `deb-changes.5`, `dpkg-build-api.7` | dpkg-dev **1.23.7** (unstable) | 2026-08-09 | Medium-high — delegated fetch |
| <https://manpages.debian.org/unstable/apt/sources.list.5.en.html> | apt **3.3.2** (unstable) | 2026-08-09 | High — independently re-fetched and verified |
| <https://wiki.debian.org/DebianRepository/Format?action=raw> | wiki, self-described "work in progress"; **no version number** | 2026-08-09 | Medium — a wiki, not a spec; treat as descriptive |
| `https://salsa.debian.org/dpkg-team/dpkg/-/raw/main/{lib/dpkg/parse.c,dump.c,fields.c,arch.c, scripts/Dpkg/**}` | git `main` HEAD | 2026-08-09 | Medium-high — delegated fetch |
| `https://salsa.debian.org/apt-team/apt/-/raw/main/{apt-pkg/deb/deblistparser.cc,apt-pkg/tagfile.cc,debian/changelog,debian/NEWS}` | git `main` HEAD | 2026-08-09 | Medium-high — delegated fetch |
| `https://bugs.debian.org/cgi-bin/bugreport.cgi?bug={749826,688251,679326,759260,747321,663918}` | bug threads, dates in text | 2026-08-09 | Medium-high — delegated fetch |

*Note:* `git.dpkg.org` serves an anti-bot challenge to `curl`; the salsa.debian.org mirror was
used instead.

## Go modules / GOPROXY

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://go.dev/ref/mod> | *Go Modules Reference*, live (content includes the `ignore` and `tool` directives, so ≥ Go 1.24) | 2026-08-09 | High — fetched as HTML, tags stripped locally |
| <https://go.dev/doc/toolchain> | *Go Toolchains*, live | 2026-08-09 | High — same method |
| <https://raw.githubusercontent.com/golang/mod/master/modfile/rule.go> | `golang.org/x/mod` `master` HEAD | 2026-08-09 | High — raw source |

## npm registry

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://raw.githubusercontent.com/npm/registry/master/docs/responses/package-metadata.md> | `master` HEAD; **undated document** | 2026-08-09 | High — raw source |
| <https://raw.githubusercontent.com/npm/registry/master/docs/REGISTRY-API.md> | `master` HEAD; **incomplete document** (empty "Filtering"/"Errors" sections, `directories:???`) | 2026-08-09 | High — raw source |
| <https://docs.npmjs.com/cli/v11/configuring-npm/package-json/> | npm CLI **v11** docs | 2026-08-09 | Medium — via summarisation tooling |
| <https://github.com/oven-sh/bun/issues/341> | opened 2022-07-07 by tchetwin | 2026-08-09 | High — verified against raw HTML |
| <https://github.com/npm/cli/issues/7529> | opened 2024-05-15 by yuvalojfrog | 2026-08-09 | Medium — via summarisation tooling |


## Maven

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://maven.apache.org/ref/current/maven-model/maven.html> | Maven model reference, "current" | 2026-08-09 | High — first-hand |
| <https://maven.apache.org/xsd/maven-4.0.0.xsd> | "Generated by Modello 2.7.0" | 2026-08-09 | High — first-hand (delegated + re-verified) |
| <https://maven.apache.org/xsd/maven-4.1.0.xsd> | `targetNamespace=…/POM/4.1.0` | 2026-08-09 | High — first-hand |
| <https://maven.apache.org/xsd/maven-4.2.0.xsd> | **HTTP 404** | 2026-08-09 | High — negative result |
| <https://maven.apache.org/xsd/repository-metadata-1.1.0.xsd> | Modello 2.7.0 | 2026-08-09 | High. Note `…/xsd/metadata-1.1.0.xsd` is **404** |
| <https://repo1.maven.org/maven2/org/apache/commons/commons-lang3/maven-metadata.xml> | live; `<lastUpdated>20251116125534</lastUpdated>` | 2026-08-09 | High — fetched by me, saved to corpus |
| <https://cwiki.apache.org/confluence/display/MAVEN/ModelParseException> | Apache Maven wiki | 2026-08-09 | Medium — via summarisation tooling |
| `apache/maven` `master`: `api/maven-api-model/src/main/mdo/maven.mdo`, `src/mdo/reader-stax.vm`, `src/mdo/writer-stax.vm`, `impl/maven-impl/.../DefaultModelBuilder.java`, `DefaultModelValidator.java`, `api/maven-api-core/.../DependencyScope.java`, `impl/maven-core/.../DefaultConsumerPomBuilder.java` | branch HEAD | 2026-08-09 | Medium-high — delegated first-hand |
| `apache/maven` `maven-3.9.x`: `DefaultModelBuilder.java`, `DefaultModelValidator.java`, `metadata.mdo` | branch HEAD | 2026-08-09 | Medium — delegated, single-sourced |
| <https://maven.apache.org/whatsnewinmaven4.html>, `…/repositories/artifacts.html`, `…/tools/mvnup.html` | Last Published 2026-08-09 | 2026-08-09 | Medium — delegated |
| JIRA `issues.apache.org/jira/browse/…`: MNG-6656, MNG-8097, MNG-6681, MNG-4654, MNG-5072, MNG-4139, MNG-3125, MNG-8106, MNG-7375, MNG-4498, MNG-7837, MNG-6036, MNG-8666, MNG-7851, TUSCANY-3845, MRESOLVER-363 | issue dates in text | 2026-08-09 | Medium — delegated, single-sourced |
| lists.apache.org dev@maven thread `1xmnwgt8q3owsjt8pl2nm4rt5q84zdfc` ("[DISCUSS] Maven 4.0 Namespace Strategy", 2025-07-22) | mailing list | 2026-08-09 | Medium — delegated, single-sourced |

**Known unresolved discrepancies (do not paper over):** (a) `modelVersion` is `minOccurs="0"` in the
published 4.1.0 XSD but `<required>true</required>` in current master `.mdo`; (b) master
`metadata.mdo` declares `xml.namespace` while 3.9.x declares `xsd.namespace` — if both readings
hold, Maven 4 will emit a namespace on `maven-metadata.xml` that Maven 3 never did; the 3.9.x file
was not re-fetched; (c) the `MNG-8750` id in a master source comment does not match that JIRA
issue.

## RPM / repomd

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://dl.fedoraproject.org/pub/epel/9/Everything/x86_64/repodata/repomd.xml> | live EPEL 9, `<revision>1786236202</revision>` | 2026-08-09 | High — fetched by me, saved to corpus |
| <https://linux.die.net/man/8/createrepo> | `createrepo(8)`, header `.TH "createrepo" "8" "2005 Jan 2" "Seth Vidal"` | 2026-08-09 | High — first-hand, verified verbatim |
| `rpm-software-management/yum` `docs/repomd.dtd` | dated **2010-01-29**, author Miroslav Suchý | 2026-08-09 | High — delegated first-hand + independently re-fetched |
| <https://fedoraproject.org/wiki/Changes/createrepo_c_1.0.0> | Fedora Change page | 2026-08-09 | High — delegated first-hand |
| <https://fedoraproject.org/wiki/Changes/Zchunk_Metadata> | F30, updated 2019-02-25 | 2026-08-09 | High — delegated first-hand |
| `rpm-software-management/createrepo_c` `master`: `doc/createrepo_c.8.in`, `doc/modifyrepo_c.8`, `src/repomd.{h,c}`, `xml_dump_repomd.c`, `xml_parser_repomd.c`, `checksum.{h,c}`, `cmd_parser.c`, `locate_metadata.c` | branch HEAD | 2026-08-09 | Medium-high — delegated first-hand |
| `yum` `yum/repoMDObject.py`, `yum/misc.py`; librepo `repomd.c`/`checksum.c`; libsolv `ext/repo_repomdxml.c` | branch HEAD | 2026-08-09 | Medium — delegated |
| dnf5 `doc/dnf5.conf.5.rst` | branch HEAD | 2026-08-09 | Medium-high — delegated first-hand |
| lists.baseurl.org pipermail: `rpm-metadata/2009-April.txt`, `yum/2009-June.txt` | mailing list archives | 2026-08-09 | Medium — delegated, single-sourced |
| bugzilla.redhat.com 1129901 (2014-08-13) | bug | 2026-08-09 | Medium — delegated, single-sourced |

**Negative results worth keeping:** no XSD/RNG/RNC for `repomd.xml` exists (createrepo_c tree of 410
paths contains none; libzypp/yum/yum-metadata-parser paths probed and 404'd). Only the 2010 DTD
exists, and nothing validates against it.

## PyPI / PEPs

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://raw.githubusercontent.com/python/peps/main/peps/pep-0629.rst> | PEP 629, **Status: Final**, created 2020-07-16, accepted 2020-08-20 | 2026-08-09 | High — raw source, first-hand |
| <https://raw.githubusercontent.com/python/peps/main/peps/pep-0691.rst> | PEP 691 | 2026-08-09 | High — raw source, first-hand |
| <https://raw.githubusercontent.com/python/peps/main/peps/pep-0714.rst> | PEP 714, **Status: Accepted**, created 2023-06-06, resolved 2023-06-27 | 2026-08-09 | High — raw source, first-hand |

*(PEP 700 and PEP 658 were **not** fetched. Claims about the `api-version` bump to 1.1 are
therefore NOT made in this document.)*

## NuGet

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://github.com/NuGet/Home/issues/4206> | opened 2017-01-05, `emgarten` | 2026-08-09 | Metadata verified independently; body quote delegated |
| <https://github.com/NuGet/NuGetGallery/issues/5403> | 2018-02-05, `joelverhagen` | 2026-08-09 | Metadata + policy sentences verified independently |
| `web.archive.org/web/20150209215913id_/https://api.nuget.org/v3/index.json` and `…/20160127072820id_/…` | Wayback raw captures | 2026-08-09 | Verified independently by delegated agent |
| NuGet.Client PR #7395; NuGet/Home #8915, #14936; NuGet.Client PR #7297 | 2020–2026 | 2026-08-09 | **Single-sourced**; PR #7297 review-comment wording could not be re-rendered |

**Not recovered:** the base NuGet Q1–Q8 sweep. Q2 and Q4 for NuGet are NOT FOUND in this document.

## Homebrew

| URL | Doc version / date | Accessed | Confidence |
|---|---|---|---|
| <https://formulae.brew.sh/docs/api/> | undated | 2026-08-09 | Medium — via summarisation tooling; **negative result** (no versioning/stability/deprecation language at all) |

**Not researched:** Homebrew/brew GitHub discussions on the JSON API v3 migration.


---

# §7 Independent second-opinion review (GLM-5.2, via `claudez`)

Requested by the owner: the full corpus above was handed to a **separate GLM-5.2 worker** running as
an independent reviewer (not a continuation of this session), with instructions to judge the design
variants for the actual decision, challenge the corpus, flag evidence-quality problems, say what is
missing, and write its own conclusion.

Run on 2026-08-09 via `claudez -p` (Claude Code against GLM-5.2 on the z.ai gateway); input was
`glm-input.txt` (instruction block + this document, 214,872 chars); raw output preserved at
`glm-review-output.md`. **It answered in Russian; reproduced verbatim, unedited.**

Its four substantive disagreements with the corpus, summarised in English for the record:

1. **Git refs are a missed versioning lever.** §4 D1 lists only in-band fields and Debian's
   new-filename trick as transferable; the reviewer argues **git tags/branches/commits** are a third
   and closer one — an old reader pinned to an old ref sees old bytes forever, with no in-band
   mechanism at all.
2. **§3 C8 undersells the substrate.** Content-addressing is presented as effortful (OCI digests,
   `go.sum`); in git it is **free** — blobs are SHA-addressed and "don't rewrite" is just
   no-force-push. The reviewer says this should be central, not a footnote.
3. **A real internal tension the corpus does not resolve.** §3 C2 praises crates.io for *not*
   bumping `v` on field additions, while §1.8a praises PEP 691 for *requiring* a minor bump when
   features are added or removed. Those are opposite policies, both endorsed. The reviewer ranks
   **a single monotonic per-record integer above Major.Minor**, on the grounds that PEP 629's
   "minor higher ⇒ SHOULD warn" is close to decoration for a silent resolver — the same trap §5 R27
   identifies at NuGet.
4. **§3 C12 is weaker than stated.** "Strict readers immobilise a format" rests on `[SS]` quotes
   plus a correlation-to-causation step; Maven Central's dominance and ordinary risk-aversion would
   explain the freeze equally well. R31 (RPM's chilling effect) is convergent evidence from the
   opposite direction, which strengthens the *direction* — but the claim should have been hedged.

And four gaps it names: **git specifics**; **index authenticity/signing** (the corpus covers
integrity of referenced payloads but never asks how a reader knows the index bytes are genuine);
**"silent wrong answer" deserves to be a headline principle** rather than being scattered across
D3/R23/features2/apt-`Multi-Arch`; and **JSON numeric-type drift** (R33's int→float, but JSON has one
number type) plus the concrete unasked question of whether to publish a JSON Schema and whether it
sets `additionalProperties: false` — which would recreate Maven's freeze.

*These are the reviewer's positions, not this document's findings. §1–§6 are unchanged; nothing
below has been folded back into them.*

---

# Независим обзор: Разработка JSON-индекса пакетов в Git

## Суть проблемы

Основные выводы корпуса верны: **lenient readers + наличие сигнала версии с первого дня** — это рабочая связка, а extremes (строгий Maven, zero-version RPM) останавливают эволюцию. Я бы не стал менять этот вердикт. Однако я отличаюсь от корпуса в трех существенных аспектах: (1) я считаю Major.Minor PEP 629 *переоцененным* и ранжирую crates.io с одним целочисленным значением выше; (2) корпус *недооценивает* тот факт, что git является носителем (substrate) — несколько его «механизмов обеспечения (enforcers)» предоставляются бесплатно; и (3) наиболее важное утверждение в нем (C12, анти-вывод Maven) опирается на единичные источники (single-sourced) и корреляционно-причинный вывод, которые он преподносит слишком уверенно. Ниже приведены мои рассуждения.

## Оценка осей (axis-by-axis), с моим собственным ранжированием

**Сигнал версии — где он находится.** Корпус (§D1) называет два варианта, которые «передаются в состоянии покоя»: in-band per-record поле crates.io и трюк Debian с новым именем файла. Он упускает третий, который *ближе всего к нашей ситуации*: **git refs (tags/branches/commits)**. Git-коммит — это неизменяемый снимок состояния с контентной адресацией (content-addressed snapshot); более старые читатели, привязанные к более старому коммиту или ветке, навсегда видят старые байты, без каких-либо действий in-band. Это лежит в основе новой схемы Debian (R12) / нового пути NuGet (Q8), но *бесплатно и нативно*. Мое ранжирование доступных рычагов для git-индекса: (1) in-band per-record поле, (2) git ref / путь, (3) новый media type — только если вы также публикуете descriptors. Заголовок npm и descriptor-carried тип OCI недоступны (§D1), и корпус прав в этом.

**Major/Minor vs одно целое число vs его отсутствие.** Здесь я сознательно расхожусь с корпусом, который называет контракт `meta.api-version` PEP 629 «единственным полным контрактом для чтения» (§1.8a Q5). Разделяйте PEP 629 на две части: *(a)* «отсутствует ⇒ предполагаем 1.0» и «слишком высокий major ⇒ критический сбой (hard fail)» — это не подлежит обсуждению и блестяще; *(b)* «слишком высокий minor ⇒ предупреждение» — это, по сути, декорация. В §C2 (следствие) сам корпус предупреждает, что «номер версии, увеличивающийся при каждом добавлении поля… заставляет читателей отклонять данные, которые они могли бы нормально прочитать» — однако он хвалит PEP 691 за обязательное увеличение minor при добавлении *или удалении* функции (§1.8a Q5), что и есть такое поведение. Хуже того, PEP 629 буквально ожидает, что major никогда не будет увеличиваться. Таким образом, Minor в реальном мире — это журнал изменений (changelog), который ничего не требует от читателя — ровно та ловушка R27 NuGet («поле версии, которое ни один читатель полностью не вычисляет, — это декорация»). Мое ранжирование: **одно монотонно возрастающее целое число на запись (crates.io `v`) > Major.Minor > его отсутствие.** Если вам действительно нужно различать «warn» и «fail», целое число делает это: «выше моего поддерживаемого максимума ⇒ skip». Минорное предупреждение PEP 629 почти не имеет практической пользы для silent resolver'ов.

**Действие при слишком новой версии.** Формулировка корпуса в §D3 верна и хорошо аргументирована: skip безопасен, когда запись является одной из многих взаимозаменяемых кандидатов; критический сбой (fatal), когда это сам ответ. Для индекса пакетов, где запись — это одна версия среди многих, **skip-the-record** — это правильный выбор. Предостережение, которое корпус поднимает (R7), а затем упускает в рекомендациях: skip + cache = отравление (poisoning). Таким образом, skip должен быть *шумным (log)*, а не молчаливым — производный артефакт никогда не должен кэшироваться так, будто пропущенная запись никогда не существовала.

**Обязательные теги vs untagged.** Доказательства против untagged объединений (§C4: features2, npm `license`, Debian `Format` self-collision) убедительны, и я согласен. Но корпус констатирует, не разрешая, реальное противоречие: тег обеспечивает безопасность, *пока он не используется для маршрутизации (matched for dispatch)* — в этот момент его набор значений и семантика замораживаются (§C11, R28, R1). OCI пришлось *отозвать (withdraw)* манифест артефакта именно потому, что новый тип — это закрытое совпадение (closed match) (R1). Мой синтез для плоской JSON-записи: **тег на уровне сериализации/типа записи, наборы значений остаются открытыми и только добавляются (append-only), и для изменчивости внутри записи предпочитайте отдельные опциональные поля, а не полиморфные значения** (это урок features2, вывернутый наизнанку). Не повторяйте ошибку OCI, делая «новый вариант» ≡ «новый тип».

**Строгость.** Maven froze-for-20-years (§C12/R25/R29) — это контрпример; расслабленность (leniency) — это норма. Но верный вывод для *неуправляемых сторонних читателей* немного отличается от формулировки «будь lenient»: **вы не можете предполагать leniency ИЛИ строгость, поэтому проектируйте так, чтобы leniency не имела значения** — никогда не помещайте load-bearing данные за полем, которое строгий читатель мог бы отклонить.

**Словари.** Открытый набор значений + добавление (append-only) + неизменное значение-за-токен — это правильный выбор, подтвержденный контрпримерами (R24 sha/sha1, R26 NuGet closed `@type`, Debian `Signed-By`). Я бы добавил: используйте self-describing значения (форма SRI `sha256:…`) и никогда не помещайте алгоритм в *имя* поля (R17, `Description-md5`).

**Absent/empty/null.** Для *кураторского* индекса (в отличие от npm, где публикуют пользователи), вам редко нужно состояние «не записано». Сделайте **absent == default, никогда не генерируйте `null` как нечто отличное от absent, опускайте пустые коллекции** — и для нескольких булевых значений, где `false` ≠ неизвестно, заранее смоделируйте tri-state. Археология `yanked` (по-прежнему `Option` спустя десятилетие, §1.1 Q2) — это предупреждение: вы никогда не сможете сузить тип после публикации.

**Round-trip.** §C8 прав, что content-addressing — это единственное, что действительно *обеспечивает (enforces)* сохранение — но корпус представляет это так, будто это тяжелая работа (OCI digests, `go.sum`). В git это **бесплатно**: blob хранятся с контентной адресацией по SHA. «Запретить перезапись» crates.io (§1.1 Q7) — это политика git-no-force-push. Это должно быть центральным элементом, а не сноской.

## Возражения против корпуса

**Внутреннее противоречие, которое он не замечает.** §C2 хвалит crates.io за *отсутствие* повышения `v` при добавлении полей. §1.8a/§2 хвалит PEP 691 за обязательное повышение minor при добавлении или удалении функции. Это противоположные политики, обе названы «правильными». Для git-индекса crates.io честнее: бамп = «вы неправильно прочитаете старые данные», а не «что-то изменилось». Повышение minor при каждом добавлении превращает номер в журнал изменений, от которого читатель почти ничего не требует.

**C12 (строгие читатели замораживают формат) — это самый важный вывод корпуса, и он слабее, чем кажется.** Анти-паттерн Maven — это то, что мотивирует все «не будь строгим». Но он опирается на единичные источники (single-sourced) ([SS]) цитаты — Boutemy «preserved in amber», тред DISCUSS за июль 2025 — и *корреляционно-причинный* шаг: Maven заморожен *и* строг; корпус делает вывод, что строгость *вызвала* заморозку. Однако доминирование Maven Central и естественное избегание риска сами по себе объяснили бы это. C12 — это правдоподобный вывод, а не доказанный; R31 (отсутствие версии RPM → chilling effect) дает конвергентное, противоположное подтверждение, что укрепляет направление, но корпус должен был сформулировать это с меньшей уверенностью.

**§D1 упускает git refs** как рычаг переноса (и §C8 упускает, что git *является* content-addressing). Для корпуса подложка (substrate) — это «тупое файловое хранилище», как статический HTTP в PEP 629. Но git богаче, и несколько рекомендаций меняются, если принять это во внимание.

## Проблемы с качеством доказательной базы

- **Вывод «строгие читатели замораживают» (C12/R25/R29)** частично опирается на [SS] + корреляцию; относитесь к нему как к направлению, а не как к факту.
- **«Молча игнорируется в течение шести лет» (R26, NuGet `SearchQueryService/3.5.0`)** — это *единственный источник* (single-sourced) из PR #7395, хотя и подтверждается shipped-кодом. Это важный сигнал сожаления (regret signal); воспринимайте его как достоверный, но не тривиальный.
- **Кэш-отравляющая цитата alexcrichton (R7)** — это явно *самый низкий уровень доверия* в корпусе (инструмент суммаризации), но поддерживает «передаваемый урок». Урок здрав, независимо от конкретной цитаты.
- **NOT FOUND в основном дисциплинированы.** Один пункт, который стоит зажать: OCI Q6 «политика переименования/повторного использования NOT FOUND» используется так, как будто отсутствие правила является значимым — это просто означает, что OCI его не записала. И Maven R35 `"null"`-in-`lastUpdated` ([SS]) — это яркая, но слабо доказанная деталь, используемая для общего правила о строковых ключах упорядочивания; не позволяйте ей нести слишком большую нагрузку.
- Корпус честно флагует свое собственное отсутствие (NuGet Q2/Q4, Homebrew со всех сторон, PEP 658/700 не получены). Это настоящие пробелы, а не переборы.

## Чего не хватает

1. **Специфика git** — tags/branches, immutable refs, signed commits/tags, content-addressed blobs. Самый большой пробел по отношению к *фактическому* решению.
2. **Аутентичность индекса.** Корпус покрывает целостность *указанных payloads* (§C3), но никогда не спрашивает: «Откуда читатель знает, что байты индекса подлинны?» — подписывание, ротация ключей, защита от подмены. Для индекса пакетов, который читают сторонние инструменты, это упущение, влияющее на безопасность.
3. **Принцип «молчаливого неверного ответа» (silent-wrong-answer).** Это *разбросано* (D3, R23, результат #3 features2, apt `Multi-Arch→No`), но заслуживает того, чтобы быть заголовком: **спроектируйте так, чтобы любое слишком-новое состояние приводило к сбою или явному skip, никогда — к молчаливому неверному результату.** Это, возможно, самое важное правило, и оно растеряно по всему тексту.
4. **Дрейф числового типа (R33: int→float в RPM)** упоминается вскользь, но это конкретная JSON-риск: JSON имеет один числовой тип; int-vs-float и целочисленные границы неоднозначны и *будут* кусать schema-first читателей. Заслуживает собственного правила (фиксированная точность; ISO-8601 timestamps вместо epoch; числа в виде строк, если важна точность).
5. **JSON Schema как инструмент для читателей.** Опубликовать схему или нет? `additionalProperties: false` воссоздаст заморозку Maven; `true` — либерально. Реальная конкретная развилка, которую корпус никогда не ставит перед нами.

## Моя рекомендация

**Принять.** Одна запись на файл (или NDJSON на пакет), чтобы изоляция версий/сбоев каждой записи была бесплатной (§C9/D3). **Per-record целочисленное поле версии с первого дня, отсутствует ⇒ 1, слишком высокое ⇒ skip-и-лог** — не warn, не silent. Authored-vs-published split (§C13): генерируйте опубликованный индекс из более богатого источника и заморозьте опубликованную форму. Self-describing checksums и content-addressed filenames, чтобы частичные читатели могли проверять то, что они получили (RPM §1.7). Reserved extension namespace (`x-` / reverse-DNS) с остальным пространством имен предварительно заявленным, как у Go «другие имена зарезервированы» (§1.4 Q6). Absent == default, без `null`-vs-absent; tri-state только там, где это действительно необходимо. Используйте git: отсутствие перезаписи дает сохранение (§C8) бесплатно; signed tags дают аутентичность индекса; новый путь/ref — это дешевый рычаг «реального разрыва».

**Не принимать.** Голый `schemaVersion` без контракта для чтения (OCI R3). Заголовок/URL-переговоры (npm R9; PEP 629 R23 явно отвергает это для состояния покоя). Увеличение версии при каждом добавлении поля (превращается в журнал изменений, §C2). Полиморфизм без тегов (§C4) и косметическое переименование значений (R24). Алгоритм в имени поля (R17). Major.Minor с нагрузкой на minor — если вы сохраните формат PEP 629, обращайтесь с major как с контрактом, а с minor — как с информационным.

## То, что нужно сделать правильно в первый день (невозможно внедрить задним числом)

**Опубликуйте per-record шлюз версии и однострочное правило «MUST игнорировать неизвестные ключи» при запуске, до первого изменения схемы.** Оба свойства являются свойствами читателей, которыми вы не управляете и не можете обновить. R5/D4 однозначны: шлюз защищает только читателей, которые уже знают о нем — pre-1.51 Cargo навсегда не защищен, а Go удалось внедрить только потому, что он контролирует ровно один toolchain и отправляет патчи, которые люди действительно устанавливают. Вы контролируете ни одного стороннего читателя. Компаньон — норма «игнорировать неизвестные ключи» — страдает от той же асимметрии: OCI записал её как MUST NOT, реестры все равно валидировали (R1/R2), а *единственный* выход, когда старый читатель жестко падает (hard-fails), — это имя ключа, о котором он никогда не слышал (PEP 714, R22) — исправление, которое стоило пятилетнего окна. Упустите это предварительное условие при запуске, и первое настоящее изменение схемы появится в поле до того, как это сделает шлюз, без средства исправления для читателей, которые уже отправлены.
