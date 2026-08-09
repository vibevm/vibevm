# How large consumer-scale systems let old clients survive server-side schema changes

**Research date / access date for every source below: 2026-08-09.**
Every claim carries a verbatim quote, a URL, and that access date. Where an authoritative
answer could not be found, the finding is marked **NOT FOUND** with the searches performed.
Primary = the organisation's own engineering blog, docs, spec, or source code.
Secondary = third-party blog, summary, or commentary (labelled and treated as weak).

---

## §0 The one-line answer, stated up front

The mobile playbook is not one technique. It is **three techniques stacked**, and they are
load-bearing in this order:

1. **The server knows its clients.** It can measure which app versions are live, which
   fields they request, and it can decide when a shape is safe to stop serving.
2. **The client is a compiled artifact the vendor built**, so the vendor can mandate
   defensive decoding in its own codegen and can, at the limit, refuse to serve an old build.
3. **Only then** does the "additive-only, versionless schema" story work.

Layer 1 and layer 2 are exactly what a JSON file published into a git repository and read
by arbitrary third-party tools does not have. The detailed verdict is §4.2: of seventeen
practices found, **six transfer fully, three transfer only in weakened form, and eight do not
transfer at all**.

**But the conclusion is not "freeze the format."** None of the organisations studied froze
anything — Badoo ships nine releases a week, Kubernetes changes its API quarterly, Meta ships
continuously while supporting three years of old app builds. What they bought was not stasis but
**the ability to change without needing permission from consumers they cannot reach**, and most
of what they paid for it *is* purchasable at rest. **§4.4 is the constructive section**: how to
keep a git-published JSON format genuinely evolvable, assembled only from practices that survive
the translation. Its load-bearing ideas are (a) stamp every artifact, because an unversioned
format cannot evolve, it can only accrete — Discord's frozen default is the published proof;
(b) Kubernetes' split between *stop writing* an old shape (cheap, do it whenever) and *stop
reading* one (never); (c) guarantee lossless round-tripping via stable field identity, as Thrift
and Iceberg do; (d) grow closed vocabularies by widening into a new field rather than adding in
place; and (e) gate the diff in CI, which is what buys the confidence to move fast — the two
organisations that let themselves stay versionless internally, LinkedIn and Uber, are exactly
the two with a hard CI gate.

**The one thing that genuinely does not transfer, and has no workaround:** you will never know
who is still reading the old shape. Netflix removes a deprecated field "once the stats show that
a deprecated field is no longer used"; that evidence does not exist for a published file. So
transition windows are chosen by judgement, and (b) above is what makes choosing wrong
survivable instead of fatal.

---

# §1 Per-subject findings

## 1.1 Meta / Facebook

### 1.1.1 Thrift — the original whitepaper (PRIMARY)

Source: Mark Slee, Aditya Agarwal, Marc Kwiatkowski, *"Thrift: Scalable Cross-Language
Services Implementation"*, Facebook, 2007.
URL: https://thrift.apache.org/static/files/thrift-20070401.pdf — accessed 2026-08-09.
(Text extracted locally from the PDF; section numbers as printed.)

The design goal is stated in §1 and it explicitly includes **data at rest**:

> "Versioning. For robust services, the involved datatypes must provide a mechanism for
> versioning themselves. Specifically, it should be possible to add or remove fields in an
> object or alter the argument list of a function without any interruption in service (or,
> worse yet, nasty segmentation faults)."

§5 opening — note the second sentence, which is the single most relevant sentence in the
whole paper for the data-at-rest question:

> "Thrift is robust in the face of versioning and data definition changes. This is critical
> to enable staged rollouts of changes to deployed services. The system must be able to
> support reading of old data from log files, as well as requests from out-of-date clients
> to new servers, and vice versa."

**Q1 — version or versionless?** Versionless at the IDL level. There is no protocol version
number in the payload; identity is carried per-field:

> "Versioning in Thrift is implemented via field identifiers. The field header for every
> member of a struct in Thrift is encoded with a unique field identifier. The combination of
> this field identifier and its type specifier is used to uniquely identify the field."

> "The Thrift definition language supports automatic assignment of field identifiers, but it
> is good programming practice to always explicitly specify field identifiers."

**Q2 — who is tolerant?** Explicitly the **reader**, and the mechanism is self-delimiting
types:

> "When data is being deserialized, the generated code can use these identifiers to properly
> identify the field and determine whether it aligns with a field in its definition file. If
> a field identifier is not recognized, the generated code can use the type specifier to skip
> the unknown field without any error. Again, this is possible due to the fact that all
> datatypes are self delimiting."

> "When an unexpected field is encountered, it can be safely ignored and discarded. When an
> expected field is not found, there must be some way to signal to the developer that it was
> not present. This is implemented via an inner isset structure inside the defined objects."

> "When a reader receives a struct, it should check for a field being set before operating
> directly on it."

The four-case analysis (§5.3) is quoted in full because it is the clearest published
statement of who bears which risk:

> "1. Added field, old client, new server. In this case, the old client does not send the new
> field. The new server recognizes that the field is not set, and implements default behavior
> for out-of-date requests.
> 2. Removed field, old client, new server. In this case, the old client sends the removed
> field. The new server simply ignores it.
> 3. Added field, new client, old server. The new client sends a field that the old server
> does not recognize. The old server simply ignores it and processes as normal.
> 4. Removed field, new client, old server. This is the most dangerous case, as the old server
> is unlikely to have suitable default behavior implemented for the missing field. It is
> recommended that in this situation the new server be rolled out prior to the new clients."

Note that all four cases concern **fields**. The paper contains **no** case analysis for
enum value growth. That omission is not accidental — see §3.

§5.4 contains the one piece of genuinely file-oriented advice in the paper:

> "For example, if we wished to add some new checksumming or error detection to the
> TFileTransport, we could simply add a version header into the data it writes to the file in
> such a way that it would still accept old log files without the given header."

**Q3 — enums.** NOT FOUND in the whitepaper. The paper never discusses enum evolution. What
*is* published is that Thrift's generated code historically **crashes** on unknown enum
values — see §1.1.2 and §3.

**Q5 — deprecation lifecycle.** NOT FOUND in the whitepaper.
**Q6 — CI checking.** NOT FOUND in the whitepaper.

### 1.1.2 Thrift enums in practice — the published bug reports (PRIMARY, issue trackers)

Microsoft `thrifty` (a Thrift implementation for Android/Kotlin), issue #84, opened by
maintainer Ben Bader. URL: https://github.com/microsoft/thrifty/issues/84 — accessed 2026-08-09.

> "clients receiving a `Baz` with the new enum value will crash. This is because the generated
> `Foo.findByValue` method returns null, and the adapter will attempt to set the field with
> that null value."

> "It seems wrong to attempt to overwrite a default value with `null`. Conversely, it also
> seems wrong to silently ignore incomprehensible data."

Apache Thrift THRIFT-5392, "Thrift Enums should generate forward compatible enum like code".
URL: https://www.mail-archive.com/dev@thrift.apache.org/msg50731.html — accessed 2026-08-09.
Allen George (Apache Thrift committer) states the design constraint plainly:

> "to support forward-compatibility, you have to have the ability to create enum variants
> without a named value and encode them onto the wire."

This is the crux of §3: forward compatibility for a closed vocabulary requires the
**decoded representation to be able to hold a value the code does not know**. A plain
language-level enum cannot do that.

### 1.1.3 GraphQL — Meta's own stated rationale (PRIMARY)

Source: Lee Byron, *"GraphQL: A data query language"*, Engineering at Meta, 2015-09-14.
URL: https://engineering.fb.com/2015/09/14/core-infra/graphql-a-data-query-language/ —
accessed 2026-08-09.

**Q1 — version or versionless?** Versionless, and Meta says why. The full "Version free"
paragraph, verified by two independent fetches on 2026-08-09:

> "**Version free:** The shape of the returned data is determined entirely by the client's
> query, so servers become simpler and easy to generalize. When you're adding new product
> features, additional fields can be added to the server, leaving existing clients unaffected.
> When you're sunsetting older features, the corresponding server fields can be deprecated but
> continue to function. This gradual, backward-compatible process removes the need for an
> incrementing version number."

Read the second sentence carefully: *"additional fields can be added to the server, leaving
existing clients unaffected."* The mechanism is **not** that the client tolerates the new
field — it is that the client **never receives it**, because it did not name it. That
distinction is the hinge of §4.

**The two numbers that matter** — these are the strongest published evidence anywhere that
versionless evolution works at consumer scale, and they are Meta's own:

> "We still support three years of released Facebook applications on the same version of our
> GraphQL API."

> "GraphQL powers almost all data-fetching in our mobile applications, serving millions of
> requests per second from nearly 1,000 shipped application versions."

The mobile motivation:

> "When we built Facebook's mobile applications, we needed a data-fetching API powerful enough
> to describe all of Facebook, yet simple enough to be easy to learn and use by our product
> developers."

**Q2 — who is tolerant?** Structurally, the *query* is the tolerance mechanism: an old client
sends an old query and receives exactly the old shape, because it named the fields it wanted.
Additive server changes are invisible to it. This is a genuinely different mechanism from
"the reader skips what it does not know" — the reader never receives what it did not ask for.
This is the single most important architectural observation in this whole report, and §4
turns on it.

**Q4 — forced upgrade.** NOT FOUND. Meta publishes no statement in this post about
force-upgrading clients. The "three years / 1,000 versions" figure is evidence *against*
routine force-upgrade for the main app. Searched: engineering.fb.com for GraphQL +
versionless + mobile app old versions; searched for Lee Byron statements on old app versions
(the reactiflux Q&A transcript at
https://raw.githubusercontent.com/reactiflux/q-and-a/master/lee-byron_facebook-graphql.md,
accessed 2026-08-09, contains **no** discussion of versioning, deprecation, or legacy app
support — verified by fetch).

### 1.1.4 Facebook Graph API — the dated-version scheme (PRIMARY)

URL: https://developers.facebook.com/docs/graph-api/guides/versioning/ — accessed 2026-08-09.

Note the sharp contradiction with §1.1.3: Meta's **internal** mobile API is versionless, and
Meta's **external third-party** API is explicitly versioned with a hard clock. Same company,
opposite answer, and the difference is precisely whether the consumer is controlled.

**Q1 — version or versionless?** Explicitly versioned.
**Q5 — deprecation lifecycle. Published duration: two years.**

> "Each version will remain for at least 2 years from release giving you a solid timeline for
> how long your app will remain working."

The failure mode is a **silent downgrade**, not an error:

> "For APIs, once a version is no longer usable, any calls made to it will be defaulted to the
> next oldest, usable version."

> "An unversioned call uses the version set in the app dashboard **Upgrade API Version** card
> under **Settings > Advanced**."

There is an explicit carve-out that voids the two-year guarantee:

> "Facebook does reserve the right to make changes in any API in a short period of time for
> issues related to security or privacy."

**Q2 / Q3 for the Graph API.** NOT FOUND. Fetched
https://developers.facebook.com/docs/graph-api/changelog/breaking-changes/ (accessed
2026-08-09); it is an index of dated changelog entries with **no** definition of a breaking
change and **no** normative statement about client tolerance or enum growth.

---

## 1.2 GraphQL as a movement

### 1.2.1 The versionless claim, in the official docs (PRIMARY)

graphql.org returns HTTP 403 to automated fetches. The identical text was obtained from the
site's own source repository and from a public mirror of the same page; both are quoted with
their URLs.

Source: graphql.github.io source repo, `faq/best-practices`.
URL: https://raw.githubusercontent.com/graphql/graphql.github.io/source/src/pages/faq/best-practices.mdx
— accessed 2026-08-09.

> "There's nothing that will prevent a GraphQL service from being versioned like any other
> REST API. That said, GraphQL avoids versioning by design."

> "GraphQL only returns the data that's explicitly requested. This means that you can add new
> features (and all the associated types and fields) without creating a breaking change or
> bloating results for existing queries."

Source: the schema-design page in the same repo, and the mirrored `learn/best-practices` page.
URLs: https://raw.githubusercontent.com/graphql/graphql.github.io/source/src/pages/learn/schema-design.mdx
and http://chentsulin.github.io/graphql.github.io/learn/best-practices/ — both accessed 2026-08-09.

> "GraphQL takes a strong opinion on avoiding versioning by providing the tools for the
> continuous evolution of a GraphQL schema."

> "GraphQL only returns the data that's explicitly requested, so new capabilities can be added
> via new types and new fields on those types without creating a breaking change."

> "This has led to a common practice of always avoiding breaking changes and serving a
> versionless API."

Nullability is presented as a deliberate partial-failure mechanism — relevant because it is
the only place GraphQL puts tolerance on the *reader*:

> "In a GraphQL type system, every field is _nullable_ by default."

> "By defaulting every field to _nullable_, any of these reasons may result in just that field
> returned 'null' rather than having a complete failure for the request."

### 1.2.2 The deprecation mechanism (PRIMARY, the spec)

spec.graphql.org returns 403; the spec source was fetched from the specification repository.
URL: https://raw.githubusercontent.com/graphql/graphql-spec/main/spec/Section%203%20--%20Type%20System.md
— accessed 2026-08-09.

> "The `@deprecated` _built-in directive_ is used within the type system definition language
> to indicate deprecated portions of a GraphQL service's schema, such as deprecated fields on
> a type, arguments on a field, input fields on an input type, values of an enum type, or
> directives. Deprecations include a reason for why it is deprecated, which is formatted using
> Markdown syntax (as specified by CommonMark)."

```graphql
directive @deprecated(
  reason: String! = "No longer supported"
) on FIELD_DEFINITION | ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | ENUM_VALUE | DIRECTIVE_DEFINITION
```

**Q5 — deprecation lifecycle.** The mechanism carries **no duration and no expiry**.
`@deprecated` takes a `reason: String` and nothing else. There is no `removalDate`, no
`supportedUntil`, no machine-readable clock anywhere in the directive. This is a real and
under-appreciated gap: the GraphQL spec provides a way to *say* something is deprecated and
no way to say *until when*.

The enum result-coercion rule, which constrains the **server** only:

> "GraphQL services must return one of the defined set of possible values. If a reasonable
> coercion is not possible they must raise an _execution error_."

> "Enums are not references for a numeric value, but are unique values in their own right.
> They may serialize as a string: the name of the represented value."

Note what this does and does not say. It binds the server to *its own current schema*. It
says nothing about a client holding an *older* schema. That gap is §3.

### 1.2.3 The criticisms (mixed)

**PRIMARY — the reference tooling's own classification.** `graphql-js` ships the canonical
breaking-change detector. URL:
https://raw.githubusercontent.com/graphql/graphql-js/16.x.x/src/utilities/findBreakingChanges.ts
— accessed 2026-08-09.

`BreakingChangeType` includes `VALUE_REMOVED_FROM_ENUM`. `DangerousChangeType` — a separate,
weaker category — includes `VALUE_ADDED_TO_ENUM`. So the reference implementation
does **not** call adding an enum value breaking; it invents a third category for it.

**PRIMARY — graphql-inspector, the widely used CI tool.** URL:
https://raw.githubusercontent.com/graphql-hive/graphql-inspector/master/packages/core/src/diff/changes/enum.ts
— accessed 2026-08-09. The reason string is the most quotable sentence in this entire report:

> "Adding an enum value may break existing clients that were not programming defensively
> against an added case when querying an enum."

Criticality: `CriticalityLevel.Dangerous` when the enum already existed;
`CriticalityLevel.NonBreaking` only when the enum is brand new. Removal is
`CriticalityLevel.Breaking`:

> "Removing an enum value will cause existing queries that use this enum value to error."

**PRIMARY — GitHub's public GraphQL API policy.** URL:
https://docs.github.com/en/graphql/overview/breaking-changes — accessed 2026-08-09.
GitHub adopts the same three-way split *as published policy*, not just as tooling:

> "**Breaking:** Changes that will break existing queries to the GraphQL API. For example,
> removing a field would be a breaking change."

> "**Dangerous:** Changes that won't break existing queries but could affect the runtime
> behavior of clients. Adding an enum value is an example of a dangerous change."

**Q5 for GitHub — a real published duration and a real published schedule:**

> "We'll announce upcoming breaking changes at least three months before making changes to the
> GraphQL schema, to give integrators time to make the necessary adjustments."

> "Changes go into effect on the first day of a quarter (January 1st, April 1st, July 1st, or
> October 1st)."

> "For example, if we announce a change on January 15th, it will be made on July 1st."

The page carries a dated forward schedule (entries observed for 2026-04-01, 2026-07-01,
2026-10-01 and 2027-01-01), i.e. the clock is published as data, not prose.

**SECONDARY (weak) — Nicolas Charpentier, "GraphQL Enums Are Unsafe", 2023-09-26.** URL:
https://charpeni.com/blog/graphql-enums-are-unsafe — accessed 2026-08-09. Independent
practitioner blog, not an organisation's engineering blog. Used only to characterise the
failure mode, which matches the primary sources above:

> "the frontend would hopefully end up with a type-error because the function/component
> doesn't know how to handle `D`, but in most cases where we weren't defensive enough, it would
> crash with an unexpected newly added value"

> "if the frontend isn't defensive enough, it will crash"

**PRIMARY (discussion) — graphql-spec issue #175, "Whole API versioning".** URL:
https://github.com/graphql/graphql-spec/issues/175 — accessed 2026-08-09. The filer's
argument against the versionless claim:

> "in real use, you are likely to make mistakes when designing your api, have requirements
> change over time, or need to make changes for other reasons."

**NOT FOUND:** a verbatim statement from Lee Byron or another GraphQL spec editor defending
the no-versioning decision in his own words. Searched: graphql-spec issues #175 and #134
(both fetched, accessed 2026-08-09 — #134's comment thread did not render for the fetcher and
#175 only *links* to his comment without reproducing it); the reactiflux Q&A transcript
(fetched, contains nothing on versioning); web searches for Lee Byron + versioning + talk
transcripts. The claim that Meta deliberately avoids versioning **is** primary-sourced from
§1.1.3; the individual-attributed defence of it is not.

**SECONDARY, could not verify:** Marc-André Giroux, "How Should We Version GraphQL APIs?",
https://productionreadygraphql.com/blog/2019-11-06-how-should-we-version-graphql-apis/ —
fetch returned truncated content on 2026-08-09 and no quotes could be extracted. Listed for
re-fetch only; **no claim in this report rests on it.**

---

## 1.3 Google

Full detail with all verbatim quotes is in the Google subsection below. Structural note
first: `cloud.google.com/apis/design/compatibility` and `.../versioning` no longer exist as
independent documents — both 301-redirect into the AIP corpus
(→ https://google.aip.dev/180 and → https://google.aip.dev/185 respectively, verified
2026-08-09). AIP is now the single authoritative Google source.

**Q1 — hybrid: explicitly versioned at major level only, versionless within it.**
URL: https://google.aip.dev/185 — accessed 2026-08-09.

> "All Google API interfaces **must** provide a _major version number_, which is encoded at the
> end of the protobuf package, and included as the first part of the URI path for REST APIs."

> "However, unlike in traditional semantic versioning, Google APIs **must not** expose minor or
> patch version numbers. For example, Google APIs use `v1`, not `v1.0`, `v1.1`, or `v1.4.2`.
> From a user's perspective, major versions are updated in place with minor/patch equivalent
> changes, and users receive new functionality without migration."

> "A new major version of an API **must not** depend on a previous major version of the same
> API."

> "Different versions of the same API **must** be able to work at the same time within a single
> client application for a reasonable transition period."

URL: https://google.aip.dev/181 — accessed 2026-08-09.

> "When breaking changes become necessary, the API producer **should** create the next major
> version of the API, and start a deprecation clock on the existing version."

**Q2 — the burden is normatively on the SERVER.** URL: https://google.aip.dev/180 — accessed
2026-08-09.

> "Existing client code **must not** be broken by a service updating to a new minor or patch
> release. Old clients **must** be able to work against newer servers (with the same major
> version number)."

> "1. Source compatibility: Code written against a previous version **must** compile against a
> newer version, and successfully run with a newer version of the client library.
> 2. Wire compatibility: Code written against a previous version **must** be able to communicate
> correctly with a newer server. In other words, not only are inputs and outputs compatible, but
> the serialization and deserialization expectations continue to match.
> 3. Semantic compatibility: Code written against a previous version **must** continue to receive
> what most reasonable developers would expect."

An unusually strong clause — compatibility extends to *undocumented* behaviour:

> "Code will often depend on API behavior and semantics, _even when such behavior is not
> explicitly supported or documented_. Therefore, APIs **must not** change visible behavior or
> semantics in ways that are likely to break reasonable user code, as such changes will be seen
> as breaking by those users."

**There is no normative "clients MUST ignore unknown fields" rule anywhere in AIP-180.** The
full section list of AIP-180 was enumerated (Guidance; Adding components; Removing or renaming
components; Moving components between files; Moving into oneofs; Changing the type of fields;
Changing string length; Changing resource names; Semantic changes; Changing value format or
construction; Default values must not change; Serializing defaults; Further reading;
Rationale; Changelog) and contains no tolerant-reader clause. Client tolerance is *assumed*
to be a free property of the protobuf wire format.

The scope carve-out — this is directly relevant to the data-at-rest question, because it is
Google stating exactly which assumption its own guidance rests on:

> "This guidance assumes that APIs are intended to be called from a range of consumers,
> written in multiple languages and with no control over how and when consumers update. Any
> API which has a more limited scope (for example, an API which is only called by client code
> written by the same team as the API producer, or deployed in a way which can enforce updates)
> should carefully consider its own compatibility requirements."

And an explicit humility clause:

> "**Important:** It is not always clear whether a change is compatible or not. The guidance
> here **should** be treated as indicative, rather than as a comprehensive list of every possible
> change."

The prohibitions (all AIP-180, accessed 2026-08-09):

> "Existing components (interfaces, methods, messages, fields, enums, or enum values) **must
> not** be removed from existing APIs in the same major version."
> "**Important:** Renaming a component is semantically equivalent to 'remove and add'."
> "Existing fields and messages **must not** have their type changed, even if the new type is
> wire-compatible, because type changes alter generated code in a breaking way."
> "Changing the default value is considered breaking and **must not** be done."
> "APIs **must not** change the expected format or algorithm used to construct the value of an
> existing field"

**Q3 — enums: see §3, where Google's position is dissected.** The key quotes:

AIP-180 (accessed 2026-08-09):

> "In general, new components (interfaces, methods, messages, fields, enums, or enum values)
> **may** be added to existing APIs in the same major version."
> "For enum values specifically, be aware that it is possible that user code does not handle new
> values gracefully."
> "Enum values **may** be freely added to enums which are only used in request messages."
> "Enums that are used in response messages or resources and which are expected to receive new
> values **should** document this."
> "Enum values still **may** be added in this situation; however, appropriate caution **should**
> be used."

AIP-216 (states), https://google.aip.dev/216 — accessed 2026-08-09. This is the most candid
paragraph Google publishes on the subject:

> "Even though adding states to an existing states enum _can_ break existing user code, adding
> states is not considered a breaking change."
> "Consider a state with only two values: `ACTIVE` and `DELETED`. A user may add code that checks
> `if state == ACTIVE`, and in the else cases simply assumes the resource is deleted. If the API
> later adds a new state for another purpose, that code will break."
> "We ultimately can not control this behavior, but API documentation **should** actively
> encourage users to code against state enums with the expectation that they may receive new
> values in the future."

AIP-126, https://google.aip.dev/126 — accessed 2026-08-09. The **actual** mitigation is a
budget, not a mechanism:

> "Enums can be more accessible and readable than strings or booleans in many cases, but they do
> add overhead when they change. Therefore, enums **should** receive new values infrequently.
> While the definition of 'infrequently' may change based on individual use cases, a good rule of
> thumb is no more than once a year. For enums that change frequently, the API **should** use a
> string and document the format."

> "For enumerated values where the set of allowed values changes frequently, APIs **should** use
> a `string` field instead, and **must** document the allowed values."

> "Enums **should** document whether the enum is frozen or they expect to add values in the
> future."

> "The first value of the enum **should** be the name of the enum itself followed by the suffix
> `_UNSPECIFIED`."

> "An exception to this rule is if there is a clearly useful zero value. In particular, if an
> enum needs to present an `UNKNOWN`, it is usually clearer and more useful for it to be a zero
> value rather than having both."

AIP-216 further specifies that the zero value means *unset*, not *unrecognised*:

> "Resources **should not** provide an unspecified state to users, and this value **should not**
> actually be used."

**Q4 — forced upgrade: Google provides no true force-update mechanism.**
URL: https://developer.android.com/guide/playcore/in-app-updates — accessed 2026-08-09.

> "Immediate updates are fullscreen UX flows that require the user to update and restart the app
> in order to continue using it."
> "This UX flow is best for cases where an update is critical to the core functionality of your
> app."

But the same corpus instructs developers to handle refusal, which proves it is not
enforceable. URL: https://developer.android.com/guide/playcore/in-app-updates/kotlin-java —
accessed 2026-08-09:

> "Your app should be able to handle cases where a user declines the update or cancels the
> download."
> "If possible, let the user continue without the update and prompt them again later."
> "If your app can't function without the update, consider displaying an informative message
> before restarting the update flow or prompting the user to close the app."

Google's own recommended fallback for an app that cannot function without the update is
**to ask the user to close it**. There is no server-side minimum-version enforcement and no
kill switch in the platform.

**NOT FOUND:** any Google-provided server-side mechanism to block an outdated client or
mandate a minimum app version. Searched: developer.android.com, support.google.com,
android-developers.googleblog.com for force update / minimum version / server enforce /
declined. Only developer-community forum threads (SECONDARY, non-authoritative) discuss it,
and they conclude the developer must build it.

**Q5 — deprecation lifecycle: 12 months, contractually.**
URL: https://cloud.google.com/terms/ §1.4(e) — accessed 2026-08-09.

Verified by a second independent fetch on 2026-08-09; the complete sentence is:

> "Google will notify Customer at least 12 months before: (i) discontinuing any Service (or
> associated material functionality) unless Google replaces such discontinued Service or
> functionality with a materially similar Service or functionality; or (ii) significantly
> modifying a Customer-facing Google API in a backwards-incompatible manner."

Note that the 12-month clock covers **both** shutdown and backwards-incompatible modification.
Pre-GA is excluded (https://cloud.google.com/terms/deprecation — accessed 2026-08-09):

> "Any versions, features, or functionality of the Services below labeled 'Early Access',
> 'Alpha', or 'Beta' are excluded from the Deprecation Policy."

Engineering guidance, AIP-185 and AIP-181 — accessed 2026-08-09:

> "The beta channel's functionality **may** be removed after it has been deprecated for a
> sufficient period; we recommend 180 days."
> "An alpha release **may** be shut down at any time, while a beta release **should** allow users
> a reasonable transition period; we recommend 180 days."
> "Beta components **should** be time-boxed and promoted to stable if no issues are found in the
> specified timeframe … a good rule of thumb is 90 days."

> "**Important:** Making an in-place breaking change in a stable API is considered an extreme
> course of action, and should be treated with equal or greater gravity as creating a new major
> version. For example, at Google, this requires the approval of the API Governance team."

**Q6 — CI compatibility checking: Google publishes NO officially supported gate.** This is a
firm negative, verified structurally. The API Linter's rules directory was enumerated via the
GitHub API (https://api.github.com/repos/googleapis/api-linter/contents/rules — accessed
2026-08-09); the listing contains `aip0121 … aip0235, aip4232, internal` and **`aip0180` is
absent**. `aip0126` and `aip0216` exist but enforce naming style, not evolution safety.

> "Not every piece of AIP guidance is able to be expressed as lint rules"
> "The linter should be used as a useful tool, but not as a substitute for reading and
> understanding API guidance."
> — https://linter.aip.dev/ — accessed 2026-08-09

A breaking-change detector exists but is disclaimed:

> "This repository contains the source code of a breaking change detector in proto level, which
> takes API proto definition files and detects the unintended breaking changes in minor versions
> updates."
> "This is not an officially supported Google project."
> — https://github.com/googleapis/proto-breaking-change-detector — accessed 2026-08-09

Repo state via GitHub API (accessed 2026-08-09): not archived, 23 stars, 38 open issues, no
description, last push 2026-05-20. Live but marginal.

---

## 1.4 Kubernetes — the counter-authority (PRIMARY)

Kubernetes is not in the original subject list but earns a section, because it is the one
large-scale published corpus that (a) flatly contradicts Google on the enum question and
(b) governs **data at rest**, since Kubernetes objects are persisted in etcd and must be
readable by later code. That combination makes it the closest published analogue to the
data-at-rest case in §4.

URL: https://raw.githubusercontent.com/kubernetes/community/main/contributors/devel/sig-architecture/api_changes.md
— accessed 2026-08-09.

**Q3 — the enum ruling, and it is the opposite of Google's:**

The complete paragraph, from the section **"Backward compatibility gotchas"**, verified verbatim
by two independent fetches on 2026-08-09:

> "Enumerated values cause similar challenges. Adding a new value to an enumerated set is *not* a
> compatible change. Clients which assume they know how to handle all possible values of a given
> field will not be able to handle the new values. However, removing a value from an enumerated set
> *can* be a compatible change, if handled properly (treat the removed value as deprecated but
> allowed). For enumeration-like fields that expect to add new values in the future, such as
> `reason` fields, document that expectation clearly in the API field description in the first
> release the field is made available, and describe how clients should treat an unknown value.
> Clients should treat such sets of values as potentially open-ended."

**Q2 — six normative compatibility rules, quoted in full:**

> "1. Any API call (e.g. a structure POSTed to a REST endpoint) that succeeded before your change
> must succeed after your change.
> 2. Any API call that does not use your change must behave the same as it did before your change.
> 3. Any API call that uses your change must not cause problems (e.g. crash or degrade behavior)
> when issued against an API servers that do not include your change.
> 4. It must be possible to round-trip your change (convert to different API versions and back)
> with no loss of information.
> 5. Existing clients need not be aware of your change in order for them to continue to function
> as they did previously, even when your change is in use.
> 6. It must be possible to rollback to a previous version of API server that does not include
> your change and have no impact on API objects which do not use your change."

Definition of compatible:

> "does not change existing semantics, including: the semantic meaning of default values *and
> behavior*; interpretation of existing API types, fields, and values; which fields are required
> and which are not; mutable fields do not become immutable; valid values do not become invalid;
> explicitly invalid values do not become valid"

**Q1 / Q5 — explicit versions with published durations.**
URL: https://kubernetes.io/docs/reference/using-api/deprecation-policy/ — accessed 2026-08-09.

> "**Rule #1: API elements may only be removed by incrementing the version of the API group.**"
> "Once an API element has been added to an API group at a particular version, it can not be
> removed from that version or have its behavior significantly changed, regardless of track."

> "**Rule #2: API objects must be able to round-trip between API versions in a given release
> without information loss**, with the exception of whole REST resources that do not exist in some
> versions."

**Rule #4a**, verified verbatim by a second fetch on 2026-08-09:

> "**Rule #4a: API lifetime is determined by the API stability level**
> - GA API versions may be marked as deprecated, but must not be removed within a major version of
>   Kubernetes
> - Beta API versions are deprecated no more than 9 months or 3 minor releases after introduction
>   (whichever is longer), and are no longer served 9 months or 3 minor releases after deprecation
>   (whichever is longer)
> - Alpha API versions may be removed in any release without prior deprecation notice"

Enumerated values are held to the same standard as resources:

> "As with whole REST resources and fields thereof, a constant value which was supported in API
> v1 must exist and function until API v1 is removed."

**The data-at-rest clause** — the single most transferable sentence found in this research:

> "no API versions that have been persisted to storage may be removed. Serving REST endpoints for
> those versions may be disabled (subject to the deprecation timelines in this document), but the
> API server must remain capable of decoding/converting previously persisted data from storage."

Read that carefully. Kubernetes draws exactly the distinction §4 needs: you may stop *serving*
an old version, but you may never stop *reading* it, because the data outlived the endpoint.

---

## 1.5 Stripe — the explicit-versioning counter-example (PRIMARY)

Included because it is the best-documented large-scale system that chose the **opposite** of
the versionless strategy, and because its mechanism (pin the version at write time) is one of
the few that survives translation to data at rest.

URL: https://stripe.com/blog/api-versioning — accessed 2026-08-09.

> "rolling versions that are named with the date they're released (for example, `2017-05-24`)"

> "The first time a user makes an API request, their account is automatically pinned to the most
> recent version available, and from then on, every API call they make is assigned that version
> implicitly."

> "To date, we've maintained compatibility with every version of our API since the company's
> inception in 2011."

**Q5 — the published duration is, in effect, forever.** No sunset is stated.

URL: https://docs.stripe.com/upgrades — accessed 2026-08-09. The published list of
backward-compatible changes, verbatim and complete:

> "Stripe considers the following changes to be backward-compatible:
> - Adding new API resources.
> - Adding new optional request parameters to existing API methods.
> - Adding new properties to existing API responses.
> - Changing the order of properties in existing API responses.
> - Changing the length or format of opaque strings, such as object IDs, error messages, and
>   other human-readable strings.
> - Adding new event types.
>   - Make sure that your webhook listener gracefully handles unfamiliar event types."

Two observations, both load-bearing:

1. **Adding an enum value is NOT on Stripe's backward-compatible list.** Stripe's list is
   materially narrower than Google's. The one closed-vocabulary growth case it does bless —
   new event types — comes with an explicit client-side obligation attached in the same
   bullet.
2. **"Make sure that your webhook listener gracefully handles unfamiliar event types"** is the
   clearest normative tolerant-reader instruction found in any vendor's docs. It is stated
   *at the point of the change that requires it*, not in a general principles section.

> "Each major release, such as Basil, includes changes that aren't backward-compatible with
> previous releases. Upgrading to a new major release can require updates to existing code. Each
> monthly release includes only backward-compatible changes, and uses the same name as the last
> major release. You can safely upgrade to a new monthly release without breaking any existing
> code."

---

## 1.6 IETF — the normative statements nobody in the mobile world cites (PRIMARY)

These two RFCs are the only genuinely *normative*, standards-track answers to Q2 found in
this research, and they point in opposite directions from each other. That tension is
itself a finding.

### RFC 6709, "Design Considerations for Protocol Extensions"
URL: https://www.rfc-editor.org/rfc/rfc6709.html — accessed 2026-08-09.

§4.2, on reserved fields — this is the clearest published statement that **strictness is the
bug**, not the safety measure:

> "It is good practice to specify the value to be inserted in such a field by the sender
> (typically zero) and the action to be taken by the receiver when seeing some other value
> (typically no action)."

> "A common mistake of inexperienced protocol implementers is to think that 'MBZ' means that
> it's their software's job to verify that the value of the field is zero on reception and reject
> the packet if not. This is a mistake, and such software will fail when it encounters future
> versions of the protocol where these previously reserved fields are given new defined meanings."

> "Similarly, protocols should carefully specify how receivers should react to unknown extensions
> (headers, TLVs, etc.), such that failures occur only when that is truly the intended outcome."

The actual formulation of the must-ignore rule, verified against the plain-text RFC
(https://www.rfc-editor.org/rfc/rfc6709.txt — accessed 2026-08-09), §4.2:

> "'MBZ', to be read as, 'Must Be Zero on transmission, Must Be Ignored on reception.'"

§4.7, "Handling of Unknown Extensions" — the opening statement:

> "IETF protocols have utilized several techniques for the handling of unknown extensions. One
> technique (often used for vendor-specific extensions) is to specify that unknown extensions be
> 'silently discarded'."

and, on when tolerance is *wrong*:

> "In order to ensure that a recipient supports an extension, a recipient encountering an unknown
> extension may be required to explicitly reject it and to return an error, rather than ignoring
> the unknown extension and proceeding with the remainder of the message."

**§A.3, on TLS, is the concrete example of the granularity principle** — the same protocol
being tolerant in one place and strict in another, deliberately:

> "Implementations are supposed to ignore unknown record types but to reject unknown handshake
> messages."

§4.1 on version negotiation:

> "Protocols generally do not need any version-negotiation mechanism more complicated than the
> mechanisms described here."

### RFC 9413, "Maintaining Robust Protocols"
URL: https://www.rfc-editor.org/rfc/rfc9413.html — accessed 2026-08-09.

This is the IETF formally walking back Postel's Law, and it is the strongest published
counter-argument to "just make the reader tolerant":

The complete Abstract, verified verbatim against the plain-text RFC
(https://www.rfc-editor.org/rfc/rfc9413.txt — accessed 2026-08-09):

> "The main goal of the networking standards process is to enable the long-term interoperability
> of protocols. This document describes active protocol maintenance, a means to accomplish that
> goal. By evolving specifications and implementations, it is possible to reduce ambiguity over
> time and create a healthy ecosystem.
>
> The robustness principle, often phrased as 'be conservative in what you send, and liberal in
> what you accept', has long guided the design and implementation of Internet protocols. However,
> it has been interpreted in a variety of ways. While some interpretations help ensure the health
> of the Internet, others can negatively affect interoperability over time. When a protocol is
> actively maintained, protocol designers and implementers can avoid these pitfalls."

All three of the following verified verbatim against the plain-text RFC on 2026-08-09:

> "However, an interpretation that advocates for tolerating unexpected inputs is no longer
> considered best practice in all scenarios."

> "Time and experience show that negative consequences to interoperability accumulate over time if
> implementations silently accept faulty input."

> "Tolerating unexpected input instead conceals problems, making it harder, if not impossible, to
> fix them later."

The document describes a "pathological feedback cycle" in which tolerated errors become
entrenched, buggy behaviour becomes "de facto standard", and implementers are forced to be
"bug-for-bug compatible". Its §5.1 is titled **"Virtuous Intolerance"** (the phrase appears as
a section heading, not in body text); the body states the idea as:

> "Choosing to generate fatal errors for unspecified conditions instead of attempting error
> recovery can ensure that faults receive attention. This intolerance can be harnessed to reduce
> occurrences of aberrant implementations."

Coupled with:

> "Protocol designers are strongly encouraged to continue to maintain and evolve protocol
> specifications beyond their initial inception and definition."

**The tension worth internalising:** RFC 6709 says *ignore what you do not understand or you
will break on future versions*. RFC 9413 says *ignoring what you do not understand hides bugs
and ossifies the protocol*. Both are right, and they are reconciled by **granularity**: be
tolerant of things the format explicitly designated as extension points, and strict about
everything else. A format that does not mark its extension points forces the reader to guess,
and both failure modes then become available at once.

### OpenID Connect Core 1.0 — the normative MUST-IGNORE for a JSON document
URL: https://openid.net/specs/openid-connect-core-1_0.html — accessed 2026-08-09. Included
because it is the clearest *normative* tolerant-reader rule found for a JSON payload, and
because of the asymmetry it exposes.

§2, on the ID Token (a JSON object):

> "Any Claims used that are not understood MUST be ignored."

§3.1.3.3:

> "Clients SHOULD ignore unrecognized response parameters."

§3.1.2.1:

> "Scope values used that are not understood by an implementation SHOULD be ignored."

**Now the asymmetry, and it is the whole enum problem in one specification.** For `display`
and `prompt` — which are closed vocabularies, i.e. enums — the same spec refuses to mandate
anything, in §3.1.2.6:

> "If an OP receives a display value outside the set defined above that it does not understand,
> it MAY return an error or it MAY ignore it."

> "If an OP receives a prompt value outside the set defined above that it does not understand, it
> MAY return an error or it MAY ignore it."

A mature, heavily deployed, standards-track JSON specification therefore says **MUST ignore**
for an unknown *member* and **MAY error or MAY ignore** for an unknown *enum value*. The
unknown-field problem is considered solved and is legislated; the unknown-enum-value problem
is considered unsolvable and is left to the implementer. That is not an oversight — it recurs
in every corpus surveyed, and §3 is about why.

---

## 1.7 Data-at-rest precedents (PRIMARY) — what formats actually do when they cannot negotiate

Gathered specifically to answer Q7 with evidence rather than opinion. These are the published
practices of formats that are **written as files and read by consumers the publisher does not
control** — the actual shape of the questioner's problem.

### npm `package-lock.json` — an explicit integer version in the file
URL: https://docs.npmjs.com/cli/v10/configuring-npm/package-lock-json — accessed 2026-08-09.

> "No version provided: an 'ancient' shrinkwrap file from a version of npm prior to npm v5."
> "`1`: The lockfile version used by npm v5 and v6."
> "`2`: The lockfile version used by npm v7 and v8. Backwards compatible to v1 lockfiles."
> "`3`: The lockfile version used by npm v9 and above. Backwards compatible to npm v7."

And the tolerant-reader rule, stated for a **file**:

> "npm will always attempt to get whatever data it can out of a lockfile, even if it is not a
> version that it was designed to support."

This is the closest published analogue to the questioner's case, and note what it does: it
uses an **explicit version integer**, not versionless additive evolution. The format that
most resembles "JSON in a git repo read by tools I do not control" chose the *opposite* of
the mobile playbook.

### Avro object container files — embed the writer's schema in the file
Source: Martin Kleppmann, "Schema evolution in Avro, Protocol Buffers and Thrift",
https://martin.kleppmann.com/2012/12/05/schema-evolution-in-avro-protocol-buffers-thrift.html
— accessed 2026-08-09. (Author's own technical blog; the author later wrote *Designing
Data-Intensive Applications*. Treat as **strong secondary** — expert-authored, not an
organisation's official position.)

> "In real life, data is always in flux. The moment you think you have finalised a schema,
> someone will come up with a use case that wasn't anticipated, and wants to 'just quickly add a
> field.'"

> "Although you need to know the exact schema with which the data was written (the writer's
> schema), that doesn't have to be the same as the schema the consumer is expecting (the reader's
> schema)."

> "Object container files handle this case: they just include the schema once at the beginning of
> the file, and the rest of the file can be decoded with that schema."

Contrast with RPC, in the same source:

> "it's probably too much overhead to send the schema with every request and response"

The mechanism is: **the file carries its own schema**. Reader/writer reconciliation then
happens locally, with no negotiation and no live server. This is the only technique found
that fully solves the data-at-rest problem, and it costs bytes in every file.

### schema.org — a published vocabulary read by uncontrolled third parties
URL: https://schema.org/docs/howwework.html — accessed 2026-08-09. This is structurally the
closest match to the questioner's case of any *organisation* studied: a vocabulary published
as data, consumed by arbitrary tools worldwide, with no ability to negotiate or force-upgrade.

Verified verbatim by two independent fetches on 2026-08-09:

> "It is exceptionally rare for a property, type or enumerated value to be deleted/removed
> without leaving it in the system as 'supersededBy' another."

**Note that the rule names *enumerated value* explicitly, alongside property and type.**
schema.org is the one organisation in this study whose consumer population most closely matches
the questioner's — arbitrary, uncounted, uncontactable third-party tools — and its policy on
closed-vocabulary members is the strictest found anywhere: never remove, always tombstone with
a forwarding pointer. It says nothing about never *adding*, which is consistent with everyone
else's silence, but the never-remove half is stated more firmly here than in any API corpus.

> "Consumers of schema.org data can generally rely on schema.org term meanings not changing
> dramatically; however term definitions often evolve gradually over time, to accommodate new
> usage scenarios or to improve usability."

> "For general use, publishers and consumers are encouraged to use the latest release and to use
> simple non-versioned schema.org URLs such as 'https://schema.org/Place' in structured data
> applications."

> "However there are settings in which more precise versioning is important."

> "Each release has a name that is assigned upon publication (e.g. '2.1')."

> "Schema.org also provides dated snapshots of each release, including both human and machine
> readable definitions of the schema.org core vocabulary."

Note the hybrid: **unversioned URLs for general use, dated snapshots for when you need
precision, and a hard never-delete rule with an explicit `supersededBy` redirect.** The
never-delete-plus-tombstone rule is the practice that transfers most cleanly.

---

## 1.8 Q6 in depth — the compatibility-checking tools (PRIMARY)

This is the question with the most concrete, verifiable answer, so it gets full treatment.
The decisive axis is not "does it exist" — several do — but **what each tool needs in order to
work**: a live server, an observable client population, or nothing but two schema files.

### 1.8.1 Buf — `buf breaking` (Protobuf). Pure static diff.
URL: https://buf.build/docs/breaking/ — accessed 2026-08-09.

> "`buf breaking` compares the current version of your Protobuf schema against a past version
> and reports any changes that would break clients, servers, or the code generated from those
> schemas."

The four categories form a strictness ladder:

> "**FILE:** Detects breakage to generated source code on a per-file basis."
> "**PACKAGE:** Detects breakage to generated source code on a per-package basis."
> "**WIRE_JSON:** Detects breakage to the binary wire format or JSON encoding."
> "**WIRE:** Detects breakage to the binary wire format only."
> "Passing a stricter category implies passing every looser one: schemas that pass `FILE` also
> pass `PACKAGE`, `WIRE_JSON`, and `WIRE`."

**The enum answer, and it is a negative finding: buf has NO rule that forbids ADDING an enum
value, at any strictness level.** The complete set of enum-value rules is
`ENUM_VALUE_NO_DELETE`, `ENUM_VALUE_NO_DELETE_UNLESS_NAME_RESERVED`,
`ENUM_VALUE_NO_DELETE_UNLESS_NUMBER_RESERVED`, `ENUM_VALUE_SAME_NAME`. No `NO_ADD` variant
exists. (https://buf.build/docs/breaking/rules/ — accessed 2026-08-09.)

Selected rules verbatim from that page (accessed 2026-08-09):

> `ENUM_VALUE_NO_DELETE` (FILE, PACKAGE): "This checks that no enum value is deleted. Deleting
> an enum value results in the corresponding value or field being deleted from the generated
> source code, which could be referenced."

> `ENUM_VALUE_NO_DELETE_UNLESS_NUMBER_RESERVED` (WIRE_JSON, WIRE): "This checks that no enum
> value is deleted without reserving the number. Though deleting an enum value isn't directly a
> wire-breaking change, reusing these numbers in the future is likely to result in bugs."

> `ENUM_SAME_TYPE` (FILE, PACKAGE): "This checks that an enum doesn't change from open to closed
> or vice versa, because whether an enum is open or closed can impact code generation. Enums in
> `proto2` files are closed, which means that unrecognized values result in the field being unset
> (the actual value is stored with other unrecognized fields). Enums in `proto3` files are open,
> which means that values not defined in the schema are accepted."

> `ENUM_VALUE_SAME_NAME` (FILE, PACKAGE, WIRE_JSON): "This checks that a given enum value has the
> same name for each enum value number. For example You can't change `FOO_ONE = 1` to
> `FOO_TWO = 1`. Doing so results in potential JSON incompatibilities and broken source code."

**Needs no live server and no client telemetry.** `--against` accepts a local git ref, a
remote git URL, a BSR module, or an archive (https://buf.build/docs/breaking/usage/ — accessed
2026-08-09). Default category when unconfigured is `FILE`. A server-side variant exists
(https://buf.build/docs/bsr/checks/breaking/ — accessed 2026-08-09): "the BSR enforces one of
two breaking-change rule sets on commits that try to advance the default label", and "On
`buf push`, the BSR's policy always wins".

### 1.8.2 Protobuf proto2 — the counter-ruling that undoes "adding an enum value is safe"
URL: https://protobuf.dev/programming-guides/proto2/ — accessed 2026-08-09.

> "A second issue with required fields appears when someone adds a value to an enum. In this
> case, the unrecognized enum value is treated as if it were missing, which also causes the
> required value check to fail."

> "Because the default value for enums is the first defined enum value, take care when adding a
> value to the beginning of an enum value list."

Compare to https://protobuf.dev/programming-guides/proto3/ — accessed 2026-08-09:

> "Adding additional values to an enum is safe."

> "Be aware that client code may treat them differently when the message is deserialized: for
> example, unrecognized proto3 enum values will be preserved in the message, but how this is
> represented when the message is deserialized is language-dependent."

**And the conformance table on https://protobuf.dev/programming-guides/enum/ (accessed
2026-08-09) records that C++, C#, Java, Kotlin, Go, JSPB, Ruby and Dart "all have known
conformance gaps" on open/closed enum handling.** The spec's ruling is not necessarily what
your runtime does.

Protobuf's own best-practices page states the framing assumption behind all of this, and it is
worth quoting because it is the assumption a git-published file *also* satisfies:
URL: https://protobuf.dev/best-practices/dos-donts/ — accessed 2026-08-09.

> "Clients and servers are never updated at exactly the same time - even when you try to update
> them at the same time. One or the other may get rolled back. Don't assume that you can make a
> breaking change and it'll be okay because the client and server are in sync."

And the closed-enum consequence, stated plainly:

> "When new values are added to an enum, old clients will see the field as unset and the getter
> will return the default value or the first-declared value if no default exists."

> "It may be tempting to declare this default as a semantically meaningful value but as a general
> rule, do not, to aid in the evolution of your protocol as new enum values are added over time."

Two adjacent rules from the same page that transfer well to file formats:

> "**Do Reserve Numbers for Deleted Enum Values** … When you delete an enum value that's no longer
> used, reserve its number so that no one accidentally re-uses it in the future."

> "**Don't Use Booleans for Something That Has Two States Now, but Might Have More Later** … The
> future flexibility of using an enum is often worth it, even if it only has two values when it is
> first introduced."

### 1.8.3 Confluent Schema Registry — compatibility modes
URL: https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
— accessed 2026-08-09.

> BACKWARD: "consumers using the new schema can read data produced with the last schema"
> FORWARD: "data produced with a new schema can be read by consumers using the last schema"
> FULL: "schemas are both backward and forward compatible"

Transitive variants check "against all previously registered schemas" rather than only the
latest. The operationally decisive passage, quoted in full:

> "The configured compatibility type has an implication on the order for upgrading client
> applications … Depending on the compatibility type:
> - `BACKWARD` or `BACKWARD_TRANSITIVE`: there is no assurance that consumers using older schemas
>   can read data produced using the new schema. Therefore, upgrade all consumers before you start
>   producing new events.
> - `FORWARD` or `FORWARD_TRANSITIVE`: there is no assurance that consumers using the new schema
>   can read data produced using older schemas. Therefore, first upgrade all producers to using the
>   new schema and make sure the data already produced using the older schemas are not available to
>   consumers, then upgrade the consumers.
> - `FULL` or `FULL_TRANSITIVE`: there are assurances that consumers using older schemas can read
>   data produced using the new schema and that consumers using the new schema can read data
>   produced using older schemas. Therefore, you can upgrade the producers and consumers
>   independently.
> - `NONE`: compatibility checks are disabled. Therefore, you need to be cautious about when to
>   upgrade clients."

**Read the FORWARD bullet against the data-at-rest case.** Confluent's own prescription for
forward compatibility includes *"make sure the data already produced using the older schemas
are not available to consumers"* — i.e. delete or hide the old data. In a git repository that
option does not exist; history is the point.

**Q3 for Confluent — NOT FOUND, and this is a substantive negative.** Both the Platform and
Cloud schema-evolution pages were fetched and searched for the string "enum"
(https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
and https://docs.confluent.io/cloud/current/sr/fundamentals/schema-evolution.html — both
accessed 2026-08-09); **the word does not appear on either page.** Confluent's per-format
tables concern fields, union/oneof variants, and scalar widening. For Avro it delegates to
the Avro spec's Schema Resolution section.

**Needs a server only optionally.** `schema-registry:test-compatibility` is "used to read
schemas from the local file system and test them for compatibility against the Schema Registry
servers"; `schema-registry:test-local-compatibility` "tests compatibility of a local schema with
other existing local schemas during development and testing phases" —
"Before the addition of `schema-registry:test-local-compatibility`, if you wanted to check
compatibility of a new schema you had to connect to the Schema Registry."
(https://docs.confluent.io/platform/current/schema-registry/develop/maven-plugin.html —
accessed 2026-08-09.) **No client telemetry required in any mode.**

**Gap closed by the issue tracker (PRIMARY).** Confluent's own engineer states the ruling that
the docs omit. URL: https://github.com/confluentinc/schema-registry/issues/601 — accessed
2026-08-09. Ewen Cheslack-Postava (Confluent), 2017-08-04:

> "Currently if you add a new value to an enum and register the updated schema against a subject
> with forward compatibility set, it will pass the compatibility check."

> "However, this is an incompatible change since data written with the new enum symbol would not
> be readable by the earlier schemas."

> "SR just relies on the Avro compatibility checks"

So: **adding an enum symbol IS a forward-incompatible change, and Confluent's checker used to
let it through anyway.** That is a compatibility gate silently failing open on precisely the
question under study.

> ⚠️ **LOW CONFIDENCE — RE-FETCH BY HAND.** Two independent fetches of the Confluent
> schema-evolution page returned *inconsistent* per-format compatibility tables (the Protobuf
> "Remove union/oneof variant" row differed between fetches). The BACKWARD/FORWARD definitions
> and the upgrade-ordering block above were stable across both fetches and are safe to rely on.
> The per-format tables are **not** transcribed here for that reason.

### 1.8.4 Apache Avro — the only spec with a first-class enum evolution mechanism
URL: https://avro.apache.org/docs/1.12.0/specification/ (identical text at .../1.11.1/) —
accessed 2026-08-09.

> "_default_: A default value for this enumeration, used during resolution when the reader
> encounters a symbol from the writer that isn't defined in the reader's schema (optional). The
> value provided here must be a JSON string that's a member of the symbols array."

The resolution rule, verbatim — this is the sharpest single sentence in the whole enum
question:

> "if both are enums: if the writer's symbol is not present in the reader's enum and the reader
> has a default value, then that value is used, otherwise **an error is signalled**."

> "A reader of Avro data, whether from an RPC or a file, can always parse that data because the
> original schema must be provided" with the data.

Verified against the reference implementation (PRIMARY, source code):
https://raw.githubusercontent.com/apache/avro/main/lang/java/avro/src/main/java/org/apache/avro/SchemaCompatibility.java
— accessed 2026-08-09. The incompatibility constant is `MISSING_ENUM_SYMBOLS`, and
`checkReaderEnumContainsAllWriterEnumSymbols()` implements exactly the spec rule: it computes
`writer.getEnumSymbols() − reader.getEnumSymbols()`, and if that set is non-empty it returns
`compatible()` **only if** `reader.getEnumDefault() != null` and the reader's symbol list
contains that default; otherwise `incompatible(MISSING_ENUM_SYMBOLS, …)`.

**The operational fact that matters most in this entire report:** the escape hatch must be
present in the **old** artifact. Avro's enum `default` protects a reader only if that reader's
schema *already declared it* before the new symbol appeared. No later schema change can
retrofit protection onto readers already in the wild. Enum `default` landed in Avro 1.9.0 via
AVRO-1340 (https://issues.apache.org/jira/browse/AVRO-1340 — ASF JIRA, project record;
accessed 2026-08-09), whose stated motivation was that "it was difficult to use enums because
you could never add an enum value and keep old readers compatible."

**Avro is explicitly a data-at-rest format.** Object Container Files contain "a schema, and all
objects stored in the file must be written according to that schema", with "the schema of
objects stored in the file, as JSON data" as required header metadata (1.12.0 spec, accessed
2026-08-09). The writer's schema travels with the data. This is the structural difference from
every API-shaped tool in this section.

### 1.8.5 JSON Schema — NOT FOUND, and the gap is real
- https://json-schema.org/understanding-json-schema/reference/enum — accessed 2026-08-09:
  "The `enum` keyword is used to restrict a value to a fixed set of values. It must be an array
  with at least one element, where each element is unique." The page carries **no** evolution,
  versioning, or compatibility guidance.
- https://json-schema.org/blog/posts/future-of-json-schema — accessed 2026-08-09: the
  compatibility guarantees discussed are for **the specification itself**, not for user schemas
  — "In this case, 'stable' means that there will be strict backward and forward compatibility
  requirements that must be followed for any change." The page does **not** define rules for
  evolving a user's schema compatibly with previously written data.

**NOT FOUND: any normative JSON Schema rule on whether adding or removing an `enum` value is a
compatible change.** Searched: the json-schema.org enum reference, the future-of-JSON-Schema
post, and web searches for JSON Schema evolution/backward-compatibility/versioning official
docs. **JSON Schema has no notion of a writer's schema versus a reader's schema at all** —
there is only validation of an instance against one schema. This is directly and unhappily
relevant to a JSON-files-in-git case: the format ecosystem the questioner is actually in is the
one ecosystem with no published evolution model.

### 1.8.6 Apollo GraphOS schema checks — the tool that needs an observable client population
URL: https://www.apollographql.com/docs/graphos/platform/schema-management/checks — accessed
2026-08-09.

> "Operations checks use your graph's historical client operation data to determine whether any
> clients would be negatively affected by the proposed schema changes."
> "Operations checks run against a maximum of 10,000 distinct operations."

**The decisive quote**, from
https://www.apollographql.com/docs/graphos/platform/schema-management/checks/run — accessed
2026-08-09:

> "If GraphOS has no operation metrics to compare against, all potentially dangerous schema
> changes result in a failed check."

Prerequisites: "your supergraph is sending operation metrics to GraphOS"; default window is
the last seven days. So Apollo **requires a live cloud service and a live, instrumented client
population**, and without traffic it degrades to fail-closed — it cannot produce a useful
static verdict.

The breaking/non-breaking list
(https://www.apollographql.com/docs/graphos/platform/schema-management/checks/reference —
accessed 2026-08-09) classifies `VALUE_REMOVED_FROM_ENUM` as breaking —
"A value was removed from an enum used by at least one operation" — and
`VALUE_ADDED_TO_ENUM` as **non-breaking**. Note the qualifier recurring in every breaking rule:
**"used by at least one operation."** Apollo's verdicts are traffic-conditional by
construction; a removed enum value with zero observed usage is not breaking under Apollo.

### 1.8.7 oasdiff (OpenAPI) — the only tool that gets enum variance right per position
URLs: https://github.com/oasdiff/oasdiff and https://www.oasdiff.com/docs/breaking-changes —
both accessed 2026-08-09.

> "Command-line tool to compare and detect breaking changes in OpenAPI specs."

> oasdiff judges changes "against the API contract your OpenAPI definition declares, not against
> what a particular server happens to accept."

It ships **separate** checks for enum-value addition and removal on the request side and the
response side — `request-property-enum-value-added`, `request-property-enum-value-removed`,
`response-property-enum-value-added`, `response-property-enum-value-removed`,
`request-parameter-enum-value-added`/`-removed`, `response-mediatype-enum-value-removed`, and
others. That decomposition is variance-correct: **adding to a response enum and adding to a
request enum are opposite risks**, and oasdiff is the only tool surveyed that models them
separately. It also honours `x-extensible-enum`, an explicit "this enum is open" marker, with
dedicated checks (`request-property-x-extensible-enum-value-removed`, etc.). Pure static; no
server, no telemetry.

### 1.8.8 Apache Iceberg — schema evolution over immutable files
URL: https://raw.githubusercontent.com/apache/iceberg/main/docs/docs/evolution.md — accessed
2026-08-09. Included because it is the other genuinely data-at-rest system found.

> "Iceberg schema updates are **metadata changes**, so no data files need to be rewritten to
> perform the update."

> "Iceberg guarantees that **schema evolution changes are independent and free of side-effects**,
> without rewriting files:
> 1. Added columns never read existing values from another column.
> 2. Dropping a column or field does not change the values in any other column.
> 3. Updating a column or field does not change values in any other column.
> 4. Changing the order of columns or fields in a struct does not change the values associated
>    with a column or field name."

Iceberg achieves this with **unique field IDs** — the same trick as Thrift/protobuf field
numbers, applied to columnar storage. Note that **Iceberg's supported-change list contains no
enum concept at all**; there is no enum type in the Iceberg schema model. A system designed
from scratch for evolvable data at rest simply declined to have closed vocabularies.

---

## 1.9 LinkedIn — Rest.li. The most explicit doctrine anyone has published (PRIMARY)

This is the single most valuable subject in the study. LinkedIn is the only organisation found
that has written a *dedicated essay* on the enum question, and its conclusion is the opposite
of Google's.

### 1.9.1 Q3 — the enum essay, quoted at length
URL: https://linkedin.github.io/rest.li/modeling/compatibility_check — accessed 2026-08-09.
The section heading is literally **"Why is Adding to an Enum Considered Backwards
Incompatible?"**

> "Many developers are surprised that adding to an enum is considered a backwards incompatible
> change."

> "But, while Rest.li is designed with features to make it easier to add symbols to enums, it
> cannot possibly guarantee that adding a enum symbols is backward compatible."

The mechanism LinkedIn generates — note that, like Apollo, this only works because LinkedIn
writes the client's codegen:

> "To make it easier to add values in the enum data schema, java enum classes generated by
> Rest.li that correspond to enum data schema always contains a special '$UNKNOWN' symbol.
> Whenever Rest.li deserializes enum data that contains a symbol that is not present in the java
> enum, Rest.li maps it to '$UNKNOWN'. When the enum is accessed via accessor implemented by a
> data template, the accessor will return the new symbol as the java '$UNKNOWN' symbol. This gives
> readers of the enum the opportunity to check if the enum is '$UNKNOWN', and if it is, handle is
> in the best possible way."

**And then the sentence that should be tattooed on this whole research question:**

> "However, it's still not possible to guarantee backward compatibility, even with the '$UNKNOWN'
> symbol available. It's possible that clients did not handle the '$UNKNOWN' symbol in the best
> possible way, and even if they did **it may be that they cannot do anything other than fail if
> they encounter a enum symbol they do not recognize**. In many practical applications, it is not
> feasible to assess how all clients have been coded to handle new enum symbols, particularly when
> there are many clients. In such cases, adding a new enum symbol might break a unknown number of
> clients."

> "It's true that there may be well controlled use case were an enum is used only by a single
> client and server that are maintained by the same developers… But if additional clients might be
> added in the future, it is still risky to get in the habit of adding enum symbols 'as-if' they
> are backward compatible changes."

**The prescription, and it is directly applicable to the data-at-rest case:**

> "Given all these potential issues with adding a enum symbol, it's important to think of adding
> enum symbols as backward incompatible. If a new symbols is to be added, a migration strategy for
> adding the enum symbol(s) must be performed just as for any other backward incompatible change.
> Note that this is only possible when all clients are known and it is possible to coordinate
> changes with them. **If this is not the case, one should consider making a backward compatible
> change (such as adding a new optional field containing a new enum field with more symbols) and
> supporting the existing clients, with the existing enum symbols, indefinitely.**"

Read that last clause again. LinkedIn's published advice for the case where **you do not know
your clients and cannot coordinate with them** — which is exactly the questioner's case — is:
**do not grow the enum. Add a new parallel field, and serve the old vocabulary forever.**
That is the only concrete, primary-sourced prescription found anywhere for the uncontrolled-
consumer scenario.

**And LinkedIn explicitly calls out that this gets worse for data at rest:**

> "While unknown symbols can be deserialized by older Rest.li consumers (because rest.li does not
> require the schema to de-serialize), it doesn't work for data persisted as Avro. Any attempt to
> deserialize an avro record containing the new enumeration value with an older schema lacking that
> enum will fail."

Verified in source (PRIMARY, code) —
https://raw.githubusercontent.com/linkedin/rest.li/master/data/src/main/java/com/linkedin/data/template/DataTemplateUtil.java
— accessed 2026-08-09:

```java
public static final String UNKNOWN_ENUM = "$UNKNOWN";
```
`stringToEnum` attempts `Enum.valueOf(targetClass, value)`, falls back to
`Enum.valueOf(targetClass, UNKNOWN_ENUM)`, and only then throws `TemplateOutputCastException`.

### 1.9.2 The undocumented third compatibility tier — a notable finding
The published docs list four levels. The source has **five**. URL:
https://raw.githubusercontent.com/linkedin/rest.li/master/restli-tools/src/main/java/com/linkedin/restli/tools/idlcheck/CompatibilityLevel.java
— accessed 2026-08-09:

```java
public enum CompatibilityLevel { OFF, IGNORE, WIRE_COMPATIBLE, BACKWARDS, EQUIVALENT; ... }
```
> "The order of the members are critical. Least requirement member comes first."

And in `CompatibilityInfo.java` (same tree, accessed 2026-08-09), **`ENUM_VALUE_ADDED` is the
sole member of the `WIRE_COMPATIBLE` level**, described as:

> "Old readers can deserialize changes serialized by new writers, but may not be able to handle
> them correctly."

`CompatibilityMessage.java` (accessed 2026-08-09) carries the same judgement in its `Impact`
enum, and note that it is the *only* impact constructed with `false` (non-error):

```java
  /** New reader is incompatible with old writer. */   BREAKS_NEW_READER(true),
  /** Old reader is incompatible with new writer. */   BREAKS_OLD_READER(true),
  /** New enum value added, which is wire compatible change. However, old readers may not be
      able to handle it. */                            ENUM_VALUE_ADDED(false);
```

**LinkedIn found that "adding an enum value" fit neither *compatible* nor *incompatible*, and
invented a third category containing only that one change.** That is independent
corroboration, from production code, of exactly the tension GitHub named "dangerous" and that
Google and Kubernetes resolve in opposite directions.

> ⚠️ Caveat carried from the research: the source-level ordering and classification are
> verified, but the exact pass/fail wiring under each level was not confirmed by executing a
> build, and it was not verified that the Gradle plugin accepts `wire_compatible` as a property
> value. Treat the *existence and description* of the tier as solid, the *runtime behaviour* as
> unconfirmed.

### 1.9.3 Q6 — the compatibility checker, and Q1/Q2/Q5
Q6 (https://linkedin.github.io/rest.li/modeling/compatibility_check and
https://linkedin.github.io/rest.li/setup/gradle — both accessed 2026-08-09):

> "**equivalent** - If the check is run in equivalent mode, no changes to Resources or schemas
> will pass. **backwards** - Changes that are considered backwards compatible will pass, otherwise,
> changes will fail. **ignore** - The compatibility checker is run, but all changes will pass…
> **off** - The compatibility checker will not be run at all."

> "By default, the compatibility checker will be run on backwards compatibility mode."

> "If you are running a continuous integration environment on a Rest.li project, you will want to
> run your compatibility checker on `equivalent`."

The honest escape hatch, published in the same page:

> "You are always free to ignore backwards-incompatible change messages if you know that the
> change will not cause problems, or are willing to take steps to ensure that it will not."

Q1 — **versionless internally, versioned externally, and LinkedIn says the unversioned public
model failed**
(https://www.linkedin.com/blog/engineering/marketing/under-the-hood-how-we-built-api-versioning-for-linkedin-market
— accessed 2026-08-09):

> "we were releasing breaking changes almost monthly with different sunset dates – making it hard
> for developers to test and plan their roadmap without a predictable release schedule."

> "Unversioned APIs also blocked customers from accessing the latest features and caused internal
> challenges with new feature development."

Q2 — the server is made bilingual and deployed first
(https://linkedin.github.io/rest.li/Rest_li-2_x-upgrade-instructions — accessed 2026-08-09):

> "Deploy your server. Since Rest.li servers running Rest.li 2.x can understand the 1.x protocol
> this is safe to do."

Q5 — a published duration for the public API
(https://learn.microsoft.com/en-us/linkedin/marketing/versioning — accessed 2026-08-09):

> "LinkedIn Marketing API Program publishes new versions monthly, and those versions are supported
> for a minimum of one (1) year."

> "An error response is returned when the version header is deprecated (e.g., 202401)."

**Note the shape, which recurs across every subject:** versionless where the organisation can
*gate* the change (internal, CI-enforced), versioned where it cannot (external partners).

---

## 1.10 Netflix — deprecation by observed usage, and eviction as normal (PRIMARY)

**Q5 — the most interesting deprecation policy found, because it has no clock at all.**
Tejas Shikhare, 2020-12-11,
https://netflixtechblog.com/how-netflix-scales-its-api-with-graphql-federation-part-2-bbe71aaec44a
— accessed 2026-08-09:

> "We have a deprecation workflow in place for evolving the schema. We've leveraged GraphQL's
> deprecation feature and also track usage stats for every field in the schema. **Once the stats
> show that a deprecated field is no longer used, we can make a backward incompatible change to
> remove the field from the schema.**"

This is the right design when you cannot force clients to upgrade — and it is precisely the
design that **requires an observable client population**. It is unavailable at rest (§4).

**Q4 — Netflix treats forced obsolescence as routine, and publishes it to users.**
https://help.netflix.com/en/node/112425, /119807 and /295469825389156 — all accessed 2026-08-09:

> "Unfortunately, Netflix will no longer be available on this device after (DATE)."

> "This current app version is no longer supported. Please upgrade the OS and App version.
> (R39-1)"

> "Please update your device. This version is no longer supported by Netflix. 5072"

Support ends when "a device can no longer get necessary updates from its manufacturer or support
new features."

The device-diversity motivation, Daniel Jacobson, 2012-07-09,
https://netflixtechblog.com/embracing-the-differences-inside-the-netflix-api-redesign-15fd8b3dc49d
— accessed 2026-08-09:

> "Netflix's streaming service is available on more than 800 different device types, almost all of
> which receive their content from our private APIs."

> "supporting these myriad device types with an OSFA API, while successful, is not optimal for the
> API team, the UI teams or Netflix streaming customers."

> "its emphasis is to make it convenient for the API provider, not the API consumer."

Protobuf-side instinct, same conclusion as everyone else
(https://netflixtechblog.com/practical-api-design-at-netflix-part-1-using-protobuf-fieldmask-35cfdc606518
— accessed 2026-08-09):

> "Never rename fields when FieldMask is used. This is the simplest solution, but it's not always
> possible"
> "Deprecate old and create a new field instead of renaming."

**Q3 — NOT FOUND.** No Netflix engineering post or DGS documentation addresses enum growth.
What *is* verifiable is a tolerant default in the DGS client's Jackson configuration
(https://raw.githubusercontent.com/Netflix/dgs-framework/master/graphql-dgs-client/src/main/kotlin/com/netflix/graphql/dgs/client/GraphQLResponse.kt
— accessed 2026-08-09): `enable(DeserializationFeature.READ_UNKNOWN_ENUM_VALUES_USING_DEFAULT_VALUE)`.
That flag requires an enum constant annotated `@JsonEnumDefaultValue`, and **no evidence was
found that dgs-codegen generates one** — checked https://netflix.github.io/dgs/generating-code-from-schema/
(no enum-growth or backward-compatibility guidance). So the tolerance is configured but
possibly unarmed. Searched additionally: dgs-codegen issue #19 (about schema default rendering,
not unknown values) and dgs-framework discussion #1042 (maintainer acknowledges the mapper
should be configurable; says nothing about enum growth).

**Q6 — NOT FOUND as a hard refusal.** Federation Part 2 describes a schema registry, CI/CD
integration and a schema working group, but does **not** describe rejecting breaking changes.

**NOT FOUND:** an explicit Netflix *engineering* statement of the form "we cannot update these
devices". Searched netflixtechblog.com for "cannot be updated" / "never be updated" / "unable
to update", device-certification posts, the TV UI deployment post, the Android TV scaling post,
Federation Parts 1–2, and the Android backend-swap post. The *consumer-facing* help pages above
are the primary evidence, and they show eviction rather than indefinite support.

---

## 1.11 Uber — the clearest mobile-specific CI gate found (PRIMARY)

**Q6 — the single best quote in the study on gating mobile schema changes.**
https://www.uber.com/blog/architecture-api-gateway/ — accessed 2026-08-09 (corroborated
identical at the /en-IN/ path):

> "All of Uber's mobile apps generate services and models based on the Thrift IDL to interact with
> the server. A CI job fetches all of the endpoint IDL from the gateway and runs a custom
> code-generation for the various models… **Any backward-incompatible change to an endpoint schema
> is prevented by a CI job that runs against the generated code review.**"

**Q1 — versionless, stated unusually bluntly.**
https://raw.githubusercontent.com/uber/idl/master/README.md — accessed 2026-08-09:

> "There should only be one version of the world. Your company runs at a single version of each
> service in production… treated as a single versioned collection."
> "You cannot pick and choose which services to update. This is intentional."

And on the protobuf side
(https://raw.githubusercontent.com/uber/prototool/dev/style/README.md — accessed 2026-08-09):

> "**Your API as a whole should not need semantic versioning - one of the core promises of Protobuf
> is forwards and backwards compatibility, and this should extend to your code as well.**"

> "Breaking changes should never be made in stable packages… Both wire-incompatible and
> source-code-incompatible changes are considered breaking changes."

**Q3 — Uber's two transports disagree with each other, and the mobile-facing one is the strict
one.** Binary Thrift accepts anything
(https://raw.githubusercontent.com/thriftrw/thriftrw-go/dev/gen/enum.go — accessed 2026-08-09):
`FromWire` simply casts `w.GetI32()` with no validation, and carries the comment
`// TODO(abg) define an error type in the library for unrecognized enums.`

JSON-by-name does not
(https://raw.githubusercontent.com/uber/zanzibar/master/docs/thrift.md — accessed 2026-08-09):

> "A thrift `enum` is a JSON string. The string value must be one of the enum names defined in the
> thrift `enum` declaration."

with `UnmarshalText` returning `fmt.Errorf("unknown enum value %q for %q: %v", ...)`.

Uber separately states "All mobile to server communications were primarily in HTTP/JSON"
(https://www.uber.com/blog/gatewayuberapi/ — accessed 2026-08-09). **Uber never draws the
conclusion that this makes enum growth unsafe for installed apps; that inference is mine, and
is flagged as inference, not quotation.**

Uber's `INVALID = 0` is about *unset*, not *unknown* — same as Google's `_UNSPECIFIED`:

> "All enum values must have a 0 `INVALID` value."
> "The invalid value carries no semantic meaning… if a value can be purposefully unset… there
> should be a `UNSET` value as the 1 value."

**Q4 — NOT FOUND (primary).** No Uber engineering post on forced upgrades or minimum app
version. Uber's published strategy is the opposite: "By shipping the older version of the app
along with the major rewrite, we can adjust the variables for the rollout or fall back to an app
that has a proven record of stability."
(https://www.uber.com/blog/carbon-dual-binary-mobile-app/ — accessed 2026-08-09.)

**SECONDARY (weak, and the page itself says answers were "edited down and summarized")** —
Gergely Orosz, former Uber engineering manager,
https://bitrise.io/blog/post/q-and-a-on-building-apps-at-scale-part-1 — accessed 2026-08-09:

> "the problem we had at Uber — we had force updates in place, we never tested it too much, but we
> never used it. Because every time when we were about to use it, the business looked at it and
> said like, 'oh, there's still one and a half percent of our users using the older Uber app'… and
> this means, like, $100 million per year in revenue."

If accurate, this is the most honest published account of why force-upgrade is a theoretical
tool rather than a practical one at consumer scale. Labelled SECONDARY; do not rely on it alone.

**Q5 — NOT FOUND.** The mechanism is published ("Instead of making a breaking change, rely on
deprecation of types"; "Do not use the `reserved` keyword in messages or enums. Instead, rely on
the `deprecated` option") but no duration is. The post titled "API **Lifecycle** Management
Platform" was fetched twice targeting lifecycle/deprecation/sunset/retire and returned none of
those terms.

⚠️ `uber/prototool` is **archived / read-only as of 2026-03-04**.

---

## 1.12 Airbnb — server-driven UI, and a conspicuous silence (PRIMARY)

**Q2 — the most explicit normative statement that the burden is on the SERVER.**
Ryan Brooks, 2021-06-29,
https://medium.com/airbnb-engineering/a-deep-dive-into-airbnbs-server-driven-ui-system-842244c5f5
— accessed 2026-08-09:

> "That's essentially what SDUI does — we pass both the UI and the data together, and **the client
> displays it agnostic of the data it contains**."

> "Everything from the screen's layout, how sections are arranged in that layout, the data
> displayed in each section, and even the actions taken when users interact with sections is
> controlled by a single backend response across our web, iOS, and Android apps."

> "GP provides many 'core' section components… meant to be configurable, styleable, and **backward
> compatible from the backend** so we can adapt to any feature's use case."

**But the tolerance is conditional, and Airbnb says so.** The conditional clause is load-bearing
and is truncated by most search engines. Ryan Brooks (`rbro112`), Airbnb,
https://github.com/MobileNativeFoundation/discussions/discussions/47 — accessed 2026-08-09:

> "Mobile has always had a versioning problem, as we well know users don't always update their
> apps. Depending on the SDUI system, it's common to be able to launch features back to previous
> releases with no client code changes needed **assuming the response is supported**."

And they concede the safety net leaks:

> "Given the backend can change a response instantly (and dynamically for different content),
> ensuring clients can support these ever-changing responses is challenging. We leverage screenshot
> testing, E2E testing & robust mocking, but **that still doesn't catch everything**."

**Q1 — one shared schema; Airbnb never uses the word "versionless":**

> "The key decision that helped us make our server-driven UI system scalable was to use a single,
> shared GraphQL schema for Web, iOS, and Android apps — i.e., we're using the same schema for
> handling responses and generating strongly typed data models across all of our platforms."

**Q3 — NOT FOUND, and it is the most conspicuous gap in the entire study.** Verified by
exact-phrase check against the full raw article text: the strings **"unknown"** and
**"deprecat"** are **absent from the SDUI deep-dive entirely** — despite the architecture
resting on two open-ended growth axes that the same article documents:

> "In GraphQL schema, GP sections are a **union of all possible section types**."
> "One important concept to touch on is `SectionComponentType`." / "`SectionComponentType` controls
> _how_ a section's data model is rendered."

The nearest thing to a mechanism is a runtime registry with **no stated miss behaviour**:

> "We leverage a plugin system on a per-feature basis to pull in section renderers at runtime. On
> Android specifically, we accomplish this through Dagger multibindings."

⚠️ **Integrity note:** several third-party blogs claim Airbnb "returns a fallback component" or
"never crashes on unknown components." **No Airbnb source for this was found. Do not attribute
it to Airbnb.** The research also caught search-engine-fabricated quotes attributed to the
Airbnb blog (a "versioning problem" sentence that actually appears, with different wording, in
a GitHub comment); those were discarded after exact-phrase verification against raw article text.

**Q4 — NOT FOUND.** No Airbnb statement on force-upgrade or minimum-version gating. Their
framing is that needing a release is the *defect SDUI removes*, and the stated pain is
**measurement**, not crashes:

> "Finally, mobile has a versioning problem. Each time we need to add new features to our listing
> page, we need to release a new version of our mobile apps for users to get the latest experience.
> **Until users update, we have few ways to determine if users are using or responding well to these
> new features.**"

**Q5 — NOT FOUND for mobile.** The Thrift-side rule has no clock: "Schema field deprecation
should only happen after we are sure the fields are no longer used."
(https://medium.com/airbnb-engineering/building-services-at-airbnb-part-4-23c95e428064 —
accessed 2026-08-09.) The only published duration points the *other* way — at the partner,
not the publisher (https://www.airbnb.com/help/article/3418 — accessed 2026-08-09):

> "(v) implementing all mandatory API features within 6 months of their release."

**Q6 — YES for Thrift, NOT for the mobile GraphQL/SDUI contract** (same Part-4 post, accessed
2026-08-09):

> "Static API Schema Validation is a tool we build to automatically detect bad API schema changes"
> "Now with static schema validation, we are able to **detect bad API changes and prevent them
> before code merge** (e.g., field type change, field id change)."

with a motivating incident stated outright:

> "Adding a new data field broke the listing availability service's API xxx before the
> corresponding service code changes got deployed."

**BLOCKED, not NOT-FOUND:** Airbnb's partner-API versioning policy at
https://developer.withairbnb.com/docs/homes/versioning 302-redirects to a login wall
(`oauth.readme.io`) — accessed 2026-08-09. It is the highest-value unresolved target in the
study.

---

## 1.13 Twitter / X (PRIMARY)

**Fetch note:** `blog.x.com` and `blog.twitter.com` return **HTTP 403** to every fetcher
(Cloudflare JS challenge); `developer.x.com` returns **HTTP 402 Payment Required**. All Twitter
primaries below were recovered from Wayback raw (`…id_/`) snapshots via curl. Quotes marked
byte-verified were extracted from raw bytes with tags stripped locally.

### 1.13.1 Q1 / Q5 — Twitter versions explicitly, and publishes the clock
URL: `https://web.archive.org/web/20200924190056id_/https://developer.twitter.com/en/docs/twitter-api/versioning`
— accessed 2026-08-09 (byte-verified).

> "We are introducing a versioning strategy in our efforts to build a stable and reliable Twitter
> API. Developers can know when to expect changes to Twitter's public APIs and be given time to
> migrate to new versions."

> "Versioning for the Twitter API will be represented by version numbers declared in the route path
> for our endpoints: `https://api.twitter.com/2/tweets`"

> "We aim to release major versions of the public API as necessary no more than every 12 months. A
> major version will be released when breaking (outlined below) changes are introduced in the API…
> Non-breaking changes will be additive and rolled out to the most recent version when ready,
> requiring no work on a developer's end"

**Q5 — the published durations:**

> "As soon as a new version is released, the previous version will be marked as deprecated.
> Versions will remain in a deprecated state for one year, after which they will be retired. In
> effect, any version will be available for at least two years overall, including their deprecation
> period. Any calls made to versions after they are retired will fail."

The **breaking** list, verbatim: "Addition of a new required parameter / Removal of an existing
endpoint / Removal of any field in the response (either required or optional) / Removal of a
query parameter / Restructuring of the input or output format… / Changing the name or data type
of an existing input parameter or output value / Changing the name of a field / Changing the
resource name / Changing a response code / Changing error types / Changes to existing
authorization scopes".

The **non-breaking** list, verbatim: "Addition of a new endpoint / Addition of a new optional
parameter / Addition of a new response field / Reordering of fields / Changing text in error
messages / Availability of new scopes / 'Nulling' of fields".

**Q3 — NOT FOUND, and the absence is itself the finding.** *Enum or field-value growth appears
in neither list.* Twitter enumerates eleven breaking changes and seven non-breaking ones, and
never rules on adding a value to a closed vocabulary. Two entries in the non-breaking list —
"Addition of a new response field" and "Reordering of fields" — silently require client
tolerance that is never stated as an obligation.

### 1.13.2 Q4 — the v1 → v1.1 forced migration, with dates
All byte-verified from Wayback, accessed 2026-08-09.

"Changes coming in Version 1.1 of the Twitter API", 2012-08-16
(`https://web.archive.org/web/20240416135831id_/https://blog.twitter.com/developer/en_us/a/2012/changes-coming-to-twitter-api`):

> "When we release version 1.1 of the API we will simultaneously announce the deprecation of v1.0.
> From the day of the release, developers will have six months to migrate applications from v1.0
> to v1.1."

> "In version 1.1, we will require every request to the API to be authenticated."

The clauses that killed third-party clients:

> "If your application displays Tweets to users, and it doesn't adhere to our Display Requirements,
> we reserve the right to revoke your application key."

> "If your application already has more than 100,000 individual user tokens, you'll be able to
> maintain and add new users to your application until you reach 200% of your current user token
> count… Once you reach 200% of your current user token count, you'll be able to maintain your
> application to serve your users, but you will not be able to add additional users without our
> permission."

"API v1 Retirement: Final Dates", 2013-03-29
(`https://web.archive.org/web/20240226063445id_/…/api-v1-retirement-final-dates`):

> "The Twitter REST API v1 will officially retire on Tuesday, May 7, 2013."

> "We will hold another blackout test on April 16, 2013 beginning at 23:00 UTC"

> "Authenticated & unauthenticated requests to api.twitter.com/1/* will receive HTTP 410 Gone. Use
> API v1.1 instead."

"API v1 Retirement is Complete", 2013-06-11
(`https://web.archive.org/web/20231211182010id_/…/api-v1-is-retired`):

> "Today, we are retiring API v1 and fully transitioning to API v1.1."
> "Based on the blackout tests and looking at the numbers, we can see that the vast majority of
> applications have transitioned to API v1.1."

**Timeline:** announced 2012-08-16 → released 2012-09-05 → 6-month window → blackout tests
(rehearsals for the kill) → target 2013-05-07 → actual completion 2013-06-11. Announced window
6 months, elapsed ~9 months, terminating in `HTTP 410 Gone`. **A forced migration, executed,
with public rehearsals.**

### 1.13.3 Q1 — the fields/expansions rationale: a NEGATIVE finding
Twitter built the same sparse-fieldset mechanism GraphQL uses, but **published no
schema-evolution rationale for it.** The most that exists
(`https://web.archive.org/web/20240303153815id_/https://developer.twitter.com/en/docs/twitter-api/data-dictionary/using-fields-and-expansions`
— accessed 2026-08-09, byte-verified):

> "This simplicity, along with the fields and expansions parameters, enable you to request only
> those fields you require, depending on your use case."

**NOT FOUND: any Twitter statement connecting fields/expansions to schema evolution, forward
compatibility, or client tolerance.** This is the striking contrast of the whole study —
**Twitter built GraphQL's mechanism and drew none of GraphQL's conclusions, shipping an explicit
`/2/` version with a one-year deprecation clock alongside it.** Badoo built the same mechanism
and *did* draw the conclusion ("There is no v2", §1.14).

**NOT RETRIEVABLE:** "Introducing a new and improved Twitter API" (July 2020), the actual v2
announcement. Wayback holds only 301 redirects for every URL variant on both `blog.twitter.com`
and `blog.x.com`; live is 403. No verbatim text was obtained and none is paraphrased here.

### 1.13.4 Q3 for Twitter's *internal* stack — Scrooge, and the default throws
URL: `https://raw.githubusercontent.com/twitter/scrooge/develop/scrooge-core/src/main/scala/com/twitter/scrooge/ThriftEnum.scala`
— accessed 2026-08-09 (byte-verified). Doc comments from Twitter's own Thrift codegen:

> "Base class for unknown enum items. The implementations are used for backward compatibility
> during enum update at producer."

> "Find the enum by its integer value, as defined in the Thrift IDL. **Throws NoSuchElementException
> exception if the value is not found**" — `def apply(value: Int): T`

> "Find the enum by its integer value… **If the value is not found it returns a special enum unknown
> value** of type T that extends EnumItemUnknown type. In particular this allows ignoring new values
> added to an enum in the IDL on the producer side when the consumer was not updated."
> — `def getOrUnknown(value: Int): T`

`CHANGELOG.rst` (byte-verified, v3.18.0): "scrooge: Support ignoring unknown enum ids."

**The tolerance is opt-in and the default throws.** That is the same posture found in Jackson,
kotlinx.serialization, Moshi and Swift (§3.2a) — and Twitter never surfaced any of it to public
API consumers.

**NOT FOUND:** any Twitter blog post or design doc on Thrift/Scrooge schema evolution. The
`twitter.github.io/scrooge/Semantics.html` page covers required/optional and default values and
contains no text on schema evolution, compatibility, passthrough fields, or enum handling.

---

## 1.14 Badoo / Bumble — NOT "NOT FOUND". Richly documented, and directly on point (PRIMARY)

The brief anticipated a null result here. It is wrong: Badoo has published more concrete
operational detail on this exact problem than most of the larger companies. The decisive move
was fetching the **authored markdown** from `github.com/badoo/techblog` (the blog's source
repository) rather than the Medium-redirected rendering.

### 1.14.1 Q1 — Badoo explicitly rejects protocol versioning, and says why
Source: Ivan Biryukov (Mobile Architect) & Orene Gauthier (Head of Mobile Engineering),
"Crazy Agile API", dated 2016-05-11.
URL: `https://raw.githubusercontent.com/badoo/techblog/master/_posts/2016-05-04-crazy-agile-api.markdown`
— accessed 2026-08-09 (byte-verified). Rendered at
`https://medium.com/bumble-tech/crazy-agile-api-5130be6f5b06`.

The protocol and its scale:

> "Our Badoo API is a set of data structures (messages) and values (enum values) that the client
> and the server send to each other. It is written in Google protobuf definitions and stored in a
> separate git repository."

> "- 450 messages, 2665 fields
> - 135 enums, 2096 values
> - 125 features flags that can be controlled from server
> - 165 functionality flags. We call them supported features"

The rejection, verbatim:

> "**Protocol level** — This approach is widely used for slow-changing public APIs. When new version
> of protocol are released, all the clients are suppose to start using it instead of the old one. We
> can't use it as different client platforms have different sets of features implemented… So if the
> client needs to implement feature D, it will also have to upgrade feature B to B', which might be
> not needed at the moment. **At Badoo we never used this versioning approach.**"

> "Our protocol is shared between our server and our 5 client platforms. As our clients release a
> new version each week (resulting in ~20 app versions per month, all of which can behave
> differently and use different parts of the protocol), we can't just create a different protocol
> version for every app release. Such protocol versioning will require server to support thousands
> of various combinations of apps behaviours, which is far not ideal."

### 1.14.2 Q2 — the client declares, the server adapts. Stated normatively.

> "A better option—the one we decided implement— would be for each client to declare at the start
> which versions of the protocol bits they support. This allows the server to be client agnostic
> when it comes to feature support and just rely on the list of supported features provided by the
> client."

> "Clients that support it send the server a SUPPORTS_WHATS_NEW flag. The server then knows that it
> can send What's New messages to the client and that they be displayed correctly."

> "On a side note we always use the optional fields. This gives us the flexibility to deprecate
> fields."

### 1.14.3 Q3 — Badoo's answer to the enum question is PREVENTION, and they diagrammed it
Source: Konstantin Yakushev, "Versioning strategy for a complex internal API", Nordic APIs
Platform Summit, deck published 2016-11-11.
URL: `https://www.slideshare.net/BadooDev/versioning-strategy-for-a-complex-internal-api`
— accessed 2026-08-09 (slide transcript extracted from raw HTML; byte-verified).

**This deck is the single most on-point artifact in the entire study — it literally draws the
unknown-enum failure.** Scale and the version tail carried:

> "API / Evolving since 2010 / RPC-style / non-restful protobuf-based / 570 commands / 1200 classes
> / 9 releases each week"

> "Badoo versions — ~ 5 last versions / last iOS 7 version / ~ 10 last versions / last Android 2.x
> version / ~ 2 last versions / last WP7 version"

The thesis slides:

> "Typical versioning — 1) http://api.example.com/orders 2) Collect nice-to-have breaking changes
> 3) Announce new version with all of them 4) http://api.example.com/v2/orders 5) Slowly deprecate v1"
> "The least important step" *(annotation pointing at step 4, the URL)*
> "There is no v2"
> "Continuous versioning"

**The enum/type-growth failure, drawn as slides 29–33:**

> "Badoo has 34 types of banners"

Slide 29 — the failure state:
> "user_list_request: { fields: [banners] }
> user_list: { banners: [**&lt;unknown banner&gt;**] }"

Slide 30 — the rejected fix (versioning the field name):
> "user_list_request: { fields: [banners_v24] }"

Slide 31 — the adopted fix (client enumerates what it knows):
> "user_list_request: { fields: [banners], supported_banners: [EXTRA_SHOWS] }
> user_list: { banners: [&lt;extra shows banner&gt;] }"

Slide 33 — the rule, stated normatively:
> "Problem: similar structures of different types
> **Release a new thing on server whenever. Make clients send supported types explicitly.**"

And for behavioural change:
> "Problem: business logic changes
> **Do changes behind version flag. Make client control those flags.**"

> "Problem: simultaneous release on clients
> **Negotiate feature with server. Once you see that enough clients support it, launch.**"

The five-rule summary:
> "Continuous versioning — 0. Add new fields for new features 1. Have a list of supported things
> 2. Cover changes with a change flag 3. Let server control enabling and disabling 4. Create
> supersets of APIs for experimenting"

Results: "On practice — 257 feature flags / 161 negotiable features". Closing vanity URL:
`http://no-v2.kojo.ru`.

**Note precisely what Badoo's answer is: the client sends an inventory of the vocabulary it
understands, and the server promises never to send anything outside it.** This is not tolerance;
it is *negotiation*. It is the most robust answer found anywhere — and it is 100% dependent on
there being a request in which the client can state its inventory. It is the practice that
transfers *least* to a file (§4).

### 1.14.4 Q6 — the adoption dashboard, and Q4 — both upgrade modes
Slide 61, verbatim:

> "Dashboard
> 4.44 4.43 4.42 4.41 3.57
> VIDEOS_IN_PHOTOS + +
> BUTTONS_ARRAY + + + + +
> ALL_DATES_ARE_UTC"

A per-app-version × per-change-flag adoption matrix. `ALL_DATES_ARE_UTC` has no `+` in any
column — a flag not yet adopted anywhere. This is how Badoo decides when a flag can be retired.
**It is pure A2 (observability); it does not exist at rest.**

Q4 — consecutive slides read simply:
> "Suggest upgrading"
> "Force upgrading"

Badoo has both, presented as a normal matched pair with no apology.

### 1.14.5 Q5 — no published duration, and Badoo explains why
From "Crazy Agile API" (byte-verified):

> "For public API the deadline is usually set and an old part stops working on this date. At Badoo
> this is not always possible as tasks to implement new features often have a much higher priority
> then removing old features. Thus we have a 3 stage process for that."

> "During the second stage, all the clients should remove deprecated protocol usage from their
> code. At this point server can't remove code as some older versions of apps can still be in
> production."

> "During the last stage when all clients have removed their code and no production versions left
> that use the protocol, it can then be removed from server code and protocol itself."

The Russian original on Habr (`https://habr.com/ru/company/badoo/blog/305888/` — accessed
2026-08-09) contains a clause absent from the English text, making the premise explicit:

> "на сервере нельзя удалять этот код ещё достаточно долго — **не все пользователи обновляют свои
> приложения быстро**"
> ("on the server this code cannot be removed for a long time yet — not all users update their apps
> quickly")

**NOT FOUND for Badoo:** (a) any statement of what a client does when prevention fails and it
*does* receive an unknown enum value — the deck's answer is prevention, never recovery;
(b) any published deprecation duration (explicitly rejected as impossible); (c) any automated
CI schema-compatibility gate — the gates are human protocol review and the adoption dashboard.
Queries run are listed in §5.

⚠️ **SECONDARY, do not cite as Badoo's words:** `https://nordicapis.com/continuous-versioning-strategy-for-internal-apis/`
(Bill Doerrfeld, 2017-03-14) summarises the same talk and asserts Badoo "never had a breaking
change… since 2010." That phrase could not be byte-verified against anything Yakushev said.

---

## 1.15 Published post-mortems where a schema/config change broke deployed consumers

This was the hardest target and it produced the most directly transferable evidence in the
report — because the best cases are not API-versioning incidents at all. **They are cases where
a data file changed shape and already-deployed readers could not cope.** That is exactly the
questioner's failure mode.

### 1.15.1 ★ Cloudflare, 2025-11-18 — the closest published analogue to the data-at-rest case
URL: https://blog.cloudflare.com/18-november-2025-outage/ — accessed 2026-08-09 (byte-verified).
Author: Matthew Prince.

The causal chain is: **a schema/permissions change → a generated data file gains extra rows →
the file exceeds a hardcoded limit compiled into already-deployed consumers → global crash.**

> "it was triggered by a change to one of our database systems' permissions which caused the
> database to output multiple entries into a 'feature file' used by our Bot Management system. That
> feature file, in turn, doubled in size. The larger-than-expected feature file was then propagated
> to all the machines that make up our network."

> "The software had a limit on the size of the feature file that was below its doubled size. That
> caused the software to fail."

The schema change looked entirely benign:

> "Since users already have implicit access to underlying tables in r0, we made a change at 11:05 to
> make this access explicit, so that users can see the metadata of these tables as well."

> "Unfortunately, there were assumptions made in the past, that the list of columns returned by a
> query like this would only include the 'default' database"

The limit that had never been approached:

> "the Bot Management system has a limit on the number of machine learning features that can be used
> at runtime. Currently that limit is set to 200, well above our current use of ~60 features."
> "When the bad file with more than 200 features was propagated to our servers, this limit was hit —
> resulting in the system panicking."
> "thread fl2_worker_thread panicked: called Result::unwrap() on an Err value"

**Version skew changed the failure MODE, not merely its presence** — this is the single most
instructive sentence for anyone reasoning about mixed-vintage readers:

> "Customers deployed on the new FL2 proxy engine, observed HTTP 5xx errors. Customers on our old
> proxy engine, known as FL, did not see errors, but bot scores were not generated correctly,
> resulting in all traffic receiving a bot score of zero. Customers that had rules deployed to block
> bots would have seen large numbers of false positives."

The old reader "kept working" in the worst possible way: silently, wrongly, and confidently.

**A partially rolled-out schema change produced flapping, not clean failure:**

> "Bad data was only generated if the query ran on a part of the cluster which had been updated. As
> a result, every five minutes there was a chance of either a good or a bad set of configuration
> files being generated and rapidly propagated across the network."

The remediation Cloudflare committed to is the whole lesson in one line:

> "Hardening ingestion of Cloudflare-generated configuration files in the same way we would for
> user-generated input"

> "Today was Cloudflare's worst outage since 2019."

### 1.15.2 ★ Google Cloud, 2025-06-12 — blank fields in replicated data crash deployed binaries
URL: https://status.cloud.google.com/incidents/ow5i3PPK96RduMcb1SsW — accessed 2026-08-09
(byte-verified).

> "On May 29, 2025, a new feature was added to Service Control for additional quota policy checks.
> This code change and binary release went through our region by region rollout, but the code path
> that failed was never exercised during this rollout due to needing a policy change that would
> trigger the code… Without the appropriate error handling, the null pointer caused the binary to
> crash."

> "This policy data contained unintended blank fields… This pulled in blank fields for this
> respective policy change and exercised the code path that hit the null pointer causing the
> binaries to go into a crash loop. This occurred globally given each regional deployment."

**Latent code plus activating data, separated by two weeks.** The commitments made afterwards
are directly reusable as design rules:

> "We will modularize Service Control's architecture, so the functionality is isolated and **fails
> open**. Thus, if a corresponding check fails, Service Control can still serve API requests."

> "We will audit all systems that consume globally replicated data. Regardless of the business need
> for near instantaneous consistency of the data globally…, data replication needs to be propagated
> incrementally with sufficient time to validate and detect issues."

### 1.15.3 Cloudflare, 2019-07-02 — why config changes bypass the safety rails code enjoys
URL: https://blog.cloudflare.com/details-of-the-cloudflare-outage-on-july-2-2019/ — accessed
2026-08-09 (byte-verified). The most quotable passage on the structural double standard:

> "The SOP for a rule change specifically allows it to be pushed globally. This is very different
> from all the software we release at Cloudflare where the SOP first pushes software to an internal
> dogfooding network point of presence (PoP)…, then to a small number of customers in an isolated
> location, followed by a push to numerous customers and finally to the world."

> "However, in this case, that speed meant that a change to the rules went global in seconds."

> "The SOP allowed a non-emergency rule change to go globally into production without a staged
> rollout."

### 1.15.4 Fastly, 2021-06-08 — a *valid* config triggers a latent bug
URL: https://www.fastly.com/blog/summary-of-june-8-outage — accessed 2026-08-09 (byte-verified).

> "We experienced a global outage due to an undiscovered software bug that surfaced on June 8 when
> it was triggered by a **valid** customer configuration change."
> "On May 12, we began a software deployment that introduced a bug that could be triggered by a
> specific customer configuration under specific circumstances."
> "Early June 8, a customer pushed a **valid** configuration change that included the specific
> circumstances that triggered the bug, which caused 85% of our network to return errors."

The word "valid" carries the lesson: schema-legal and still fatal. Passing your own validator is
not evidence that deployed readers will cope.

### 1.15.5 Slack, 2021-02-24 — "how we broke your Slack app"
URL: https://web.archive.org/web/20230129150309/https://api.slack.com/changelog/2021-02-24-how-we-broke-your-slack-app
— accessed 2026-08-09 (byte-verified; live page is an SPA shell).

> "Hello! You are here because three monumental things changed on the Slack platform today,
> February 24, 2021."

> "These deprecation and retirements are rolling out gradually on February 24, 2021. **Your apps or
> integrations may work fine in one workspace but break in another.**"

> "We retired every Web API method in the channels.*, im.*, mpim.*, and groups.* namespaces.
> Requests to these methods now return a `method_deprecated` error."

### 1.15.6 Concrete enum-growth incidents (PRIMARY, issue trackers)

**Google broke its own official client library by adding an enum value.** URL:
https://github.com/protocolbuffers/protobuf/issues/16857 — accessed 2026-08-09 (byte-verified):

> "After releasing a new version of the Google Ads API (v16_1)… we added one more new enum value to
> `CriterionType`. But when a user uses that enum class to parse a new value (`LIFE_EVENT`), it
> failed with `Enum Google\Ads\GoogleAds\V16\Enums\CriterionTypeEnum\CriterionType has no name
> defined for value 41`"

> "**What did you expect to see** — Based on this doc, it should just parses the value without any
> issues. I'd expect it to represent a new value with `UNKNOWN` and all methods in that enum class
> should just work."

**Real end-user crashes from a server-delivered enum, with stack trace.** URL:
https://github.com/facebook/facebook-android-sdk/pull/544 (merged 2019-02-06) — accessed
2026-08-09 (byte-verified):

> "Your current SDK crashes all the time for real users:"

```
java.lang.IllegalArgumentException:
   at java.lang.Enum.valueOf (Enum.java:257)
   at com.facebook.appevents.codeless.internal.EventBinding$MappingMethod.valueOf (EventBinding.java:154)
   at com.facebook.appevents.codeless.internal.EventBinding.getInstanceFromJson (EventBinding.java:82)
   at com.facebook.appevents.codeless.CodelessMatcher$ViewMatcher.run (CodelessMatcher.java:224)
```

> "Looks like this comes from invalid/unexpected values delivered by your codeless configuration
> JSON data."

**Server-delivered JSON config → `Enum.valueOf` → uncaught `IllegalArgumentException` → hard
crash in production.** This is the exact requested pattern, from Meta's own SDK.

**The standard mitigation, already enabled, still failing.** URL:
https://github.com/joelittlejohn/jsonschema2pojo/issues/728 — accessed 2026-08-09
(byte-verified):

> "If I add a new value to an existing Enum and start returning it from my service any client who
> has not upgraded to the latest version of the JAR containing the new Enum will start getting
> exceptions during deserialization **even if the DeserializationFeature.READ_UNKNOWN_ENUM_VALUES_AS_NULL
> has been set to enabled**."

### 1.15.7 Q4 — forced upgrade is published as routine, without apology
Slack (https://slack.com/help/articles/1500001836081-Slacks-deprecation-schedule — accessed
2026-08-09, byte-verified):

> "To allow for innovation and keep customer data safe and secure, Slack updates its system
> requirements every six months in May and November."

> "When a Slack desktop app version is no longer supported, you'll receive a notification prompting
> you to upgrade your app version to continue using Slack. **You won't be able to access Slack until
> you upgrade.**"

Published durations: desktop/mobile apps "12-18 months from release"; iOS "2.5-3 years from
release"; Android "4.5-5 years from release".

Signal (https://support.signal.org/hc/en-us/articles/5109141421850-Supporting-Older-Operating-Systems
— accessed 2026-08-09, byte-verified):

> "you'll be able to open the Signal app but **you won't be able to send or receive messages or calls
> until you've upgraded** the operating system."

Valve/Steam shows the opposite discipline — a protocol change gated on *observed* adoption
(https://steamcommunity.com/discussions/forum/14/2974028351344359625/, FletcherDunnValve,
2020-12-07 — accessed 2026-08-09, byte-verified):

> "When a client queries a server, it will begin by sending an A2S_INFO packet, **formatted exactly
> as before**."

> "**Since not all players may have an updated Steam client and understand this handshake, it is not
> recommend to enable this at this time, except for testing. We will post again when the vast
> majority of users are running clients that understand the new protocol**, and enabling the new
> protocol is safe."

### 1.15.8 ★ Discord — the strongest revealed preference in the study
URL: https://docs.discord.com/developers/reference — accessed 2026-08-09 (byte-verified).

> "Some API and Gateway versions are now non-functioning, and are labeled as discontinued in the
> table below for posterity. Trying to use these versions will fail and return 400 Bad Request."

> "Omitting the version number from the route will route requests to the current default version
> (marked below)."

The raw table markup places the ✓ Default marker on **version 6, whose Status is `Deprecated`**;
every other row's Default cell is empty. **Discord's unversioned default route has been frozen
on a deprecated version for years, because moving it would break clients that never named a
version.** Compatibility of already-deployed consumers outranks pointing the default at current.
That is the single most transferable governance decision found — and it is the argument for
requiring a version stamp rather than allowing an unversioned default at all.

---

# §2 CROSS-SUBJECT TABLE

Rows = questions, columns = subjects, split into two tables for width. `NF` = NOT FOUND
(searched, no authoritative answer — see the named subsection for what was searched).
Every cell is backed by a verbatim quote in §1.

## 2.1 Consumer platforms with mobile clients

| | **Meta / Facebook** | **Twitter / X** | **Badoo / Bumble** | **Netflix** | **LinkedIn** | **Uber** | **Airbnb** |
|---|---|---|---|---|---|---|---|
| **Q1 Version or versionless** | **Both.** Internal mobile GraphQL "version free"; external Graph API explicitly dated-versioned | **Versioned.** `/2/` in path; v1→v1.1 was a hard migration | **Versionless, emphatically.** "There is no v2" / "At Badoo we never used this versioning approach" | Versionless; evolution by deprecation | **Both.** Versionless internal (CI-gated); versioned external after the unversioned model failed | **Versionless.** "There should only be one version of the world"; "should not need semantic versioning" | One shared schema, no version stated; never says "versionless" |
| **Q2 Who is tolerant** | Neither — **the client never receives what it did not ask for** | Server constrained; client tolerance of added/reordered fields assumed but never stated | **Client declares, server adapts.** `supported_features` / `supported_banners` | Server (BFF → federated graph) | **Server**, deployed first and bilingual | Codec, symmetric; server is app-version-aware | **Backend, explicitly.** "the client displays it agnostic of the data it contains" |
| **Q3 Enum growth** | NF for Graph API. Internally Thrift codegen historically **crashed** | **NF — in neither the breaking nor non-breaking list.** Internally Scrooge `apply` throws; `getOrUnknown` opt-in | **Prevention, not recovery.** Client sends `supported_banners`; unknown banner shown as the failure state | NF. DGS client enables Jackson's unknown-enum default; no evidence codegen arms it | ⭐ **"Think of adding enum symbols as backward incompatible."** `$UNKNOWN` generated but "cannot guarantee" | Binary Thrift permissive; **JSON-by-name strict**. No UNKNOWN convention | **NF** — "unknown" absent from the SDUI article entirely, on a union+enum architecture |
| **Q4 Forced upgrade** | NF; "three years"/"~1,000 versions" is evidence against | ⭐ **Yes, executed** — blackout tests → `HTTP 410 Gone` | **Yes, both** — "Suggest upgrading" / "Force upgrading" | ⭐ **Normal** — devices EOL'd when they "can no longer get necessary updates" | External: deprecated version header errors | NF (secondary: built it, never used it — "$100 million per year") | **NF** — old installs are reached, never evicted |
| **Q5 Deprecation clock** | **2 years** (Graph API) | **1 yr deprecated / ≥2 yrs total**; v1→v1.1 was 6 months announced, ~9 actual | **None, and says why** — "not always possible" | **None — usage-gated** ("once the stats show… no longer used") | **1 year minimum** (public API) | NF | NF (mobile) |
| **Q6 CI gate** | NF | NF | Human review + **adoption dashboard** (version × flag matrix) | Governance + linting, **no published refusal** | ⭐ **Yes** — 4–5 levels, `equivalent` in CI | ⭐ **Yes** — "prevented by a CI job"; `prototool break check` | Thrift only; **not** for the mobile GraphQL/SDUI contract |

## 2.2 Governance corpora, formats, and counter-examples

| | **Google AIP** | **Kubernetes** | **Stripe** | **GitHub GraphQL** | **GraphQL (movement)** | **Thrift (2007 paper)** | **Discord / Slack** |
|---|---|---|---|---|---|---|---|
| **Q1** | **Hybrid** — major version in path/package, "must not expose minor or patch"; versionless within | Explicit API groups; "elements may only be removed by incrementing the version" | **Dated rolling versions, account-pinned**; compatible "with every version… since 2011" | Versionless schema + published breaking-change schedule | **Versionless by design** — "GraphQL avoids versioning by design" | Versionless; identity via **field IDs** | Discord: numbered, and the **unversioned default is frozen on a deprecated version** |
| **Q2** | ⭐ **Server, normatively** — "Old clients **must** be able to work against newer servers". **No** tolerant-reader rule exists | **Six normative rules**; "Existing clients need not be aware of your change" | Server, forever (pinning). Client told to "gracefully handle unfamiliar event types" | Server | Neither — the query is the mechanism | ⭐ **Reader** — "the generated code can use the type specifier to skip the unknown field without any error" | Slack: **client must upgrade or lose access** |
| **Q3** | **Not breaking** — but AIP-216 admits "can break existing user code". Real mitigation is a **budget**: "no more than once a year" | ⭐ **"Adding a new value to an enumerated set is *not* a compatible change."** Direct contradiction of Google | **Not on the compatible list.** Nearest: "gracefully handles unfamiliar event types" | ⭐ **"Dangerous"** — a third category: "won't break existing queries but could affect runtime behavior" | `VALUE_ADDED_TO_ENUM` = *dangerous*: "may break existing clients that were not programming defensively" | **NF** — the paper's four-case analysis covers fields only | NF |
| **Q4** | ⭐ **No true mechanism.** Play's "immediate" update is declinable; fallback is "prompt the user to close the app" | N/A | No | No | No | N/A | ⭐ **Routine, unapologetic** — "You won't be able to access Slack until you upgrade" |
| **Q5** | **12 months** contractually; beta 180 days; "in-place breaking change… requires the approval of the API Governance team" | **GA: never within a major.** Beta: 9 months / 3 releases. Alpha: no notice | **None published — effectively forever** | **3 months minimum**, quarterly effective dates, published forward schedule | **None. `@deprecated` has a reason and no duration** | NF | Slack: 12–18 mo apps, 4.5–5 yrs Android. Discord: no durations |
| **Q6** | ⭐ **NO official gate.** `aip0180` absent from the linter; detector "is not an officially supported Google project" | Round-trip tests | NF | NF | `buf`, `oasdiff`, `graphql-inspector` (static); **Apollo requires client telemetry and fails closed without it** | N/A | NF |

## 2.3 The three patterns worth naming

1. **Versionless is bought, not free.** Every organisation that stays versionless pays for it with
   something the questioner's case cannot supply: a query (Meta, GraphQL), a negotiation handshake
   (Badoo), a hard CI gate (LinkedIn, Uber), or an observable client population (Netflix).
2. **The same company versions when it loses control of the consumer.** Meta: versionless
   internally, dated versions externally. LinkedIn: versionless internally, versioned externally
   *after publicly stating the unversioned model failed*. This is the strongest single signal in
   the study, and it points directly at the data-at-rest verdict.
3. **Nobody solved the enum question.** Four organisations independently invented a *third*
   category for it (GitHub "dangerous", graphql-js `DangerousChangeType`, LinkedIn
   `WIRE_COMPATIBLE`, Google's request/response asymmetry) rather than call it compatible or
   incompatible.

---

# §3 THE ENUM QUESTION, IN DEPTH

This is the question the research answers most clearly, and the answer is worse than the
folklore suggests.

## 3.1 The headline: the two most respected corpora flatly contradict each other

**Google says adding an enum value is not a breaking change.**
https://google.aip.dev/216 — accessed 2026-08-09:

> "Even though adding states to an existing states enum _can_ break existing user code, adding
> states is not considered a breaking change."

**Kubernetes says adding an enum value is not a compatible change.**
https://raw.githubusercontent.com/kubernetes/community/main/contributors/devel/sig-architecture/api_changes.md
— accessed 2026-08-09:

> "Adding a new value to an enumerated set is *not* a compatible change. Clients which assume
> they know how to handle all possible values of a given field will not be able to handle the new
> values."

**LinkedIn agrees with Kubernetes, in a document written specifically to correct the opposite
belief.** https://linkedin.github.io/rest.li/modeling/compatibility_check — accessed
2026-08-09, under the heading *"Why is Adding to an Enum Considered Backwards Incompatible?"*:

> "Many developers are surprised that adding to an enum is considered a backwards incompatible
> change."

> "it's important to think of adding enum symbols as backward incompatible"

These are not addressing different questions. They are the same question, answered in
opposite directions, by three of the most carefully governed API corpora in existence — two of
which (Kubernetes, Rest.li) went to the trouble of writing down *why the intuitive answer is
wrong*, and one of which (Google) states the intuitive answer while conceding in the same
paragraph that it breaks real code. **There is no industry consensus on the sharpest question
here.** Anyone who tells you there is has read only one of the three documents.

A fourth position exists and is arguably the most honest: **invent a third category.** GitHub
publishes it as policy — "Adding an enum value is an example of a dangerous change" — the
reference `graphql-js` implementation encodes it as `DangerousChangeType.VALUE_ADDED_TO_ENUM`,
and LinkedIn independently arrived at the same place *in production code*, creating a
`WIRE_COMPATIBLE` compatibility level whose sole member is `ENUM_VALUE_ADDED`, commented:

> "New enum value added, which is wire compatible change. However, old readers may not be able to
> handle it."
> — https://raw.githubusercontent.com/linkedin/rest.li/master/data/src/main/java/com/linkedin/data/schema/compatibility/CompatibilityMessage.java
> — accessed 2026-08-09

Four organisations, working independently, could not fit "added an enum value" into either
bucket. That is the strongest available evidence that the question is genuinely ill-posed under
a binary compatible/incompatible model.

What reconciles the positions is a difference in *who absorbs the cost*, not a difference in
fact. All agree the old client can break. Google chooses to ship anyway and discharge the
obligation through documentation; Kubernetes and LinkedIn choose to call it incompatible and
force a migration. Google's own text concedes the mechanism it is choosing not to use:

> "We ultimately can not control this behavior, but API documentation **should** actively
> encourage users to code against state enums with the expectation that they may receive new
> values in the future."

And LinkedIn states the limit of even the best mechanism, which is the sentence that should
govern any decision made on the strength of this report:

> "it may be that they cannot do anything other than fail if they encounter a enum symbol they do
> not recognize"
> — https://linkedin.github.io/rest.li/modeling/compatibility_check — accessed 2026-08-09

## 3.2 What actually happens on the wire, per format

The behaviour is entirely determined by whether the format's enums are **open** (the decoded
value can hold a number/string the code does not know) or **closed** (it cannot).

| Format | Old reader receives a new enum value | Loud or silent? |
|---|---|---|
| proto3 / editions binary (**open**) | "Open enums will parse the value `2` and store it directly in the field." Accessor "will report the field as being _set_". Round-trips intact. | Neither — it works |
| proto2 binary (**closed**) | "Closed enums will parse the value `2` and store it in the message's unknown field set." Accessor reports the field **unset**, returns the default. | **Silent** — worst case |
| proto2 `repeated` closed enum | Unknown values go to the unknown-field set and on reserialize are appended, "**but not in their original place in the list**" — `[0,2,1,2]` reads as `[0,1]` and rewrites as `[0,1,2,2]` | **Silent reordering** |
| ProtoJSON | **Unspecified by the spec.** Default in major implementations is to **throw**. | **Loud crash** |
| Avro | "if the writer's symbol is not present in the reader's enum and the reader has a default value, then that value is used, **otherwise an error is signalled**" | **Loud** unless `default` pre-declared |
| GraphQL (typed clients) | Codegen-dependent. Apollo Kotlin synthesises an `UNKNOWN__` case; exhaustive `switch`/`when` in hand-written TS unions crashes. | Varies |
| Thrift (historic codegen) | `findByValue` returns null; "clients receiving a `Baz` with the new enum value will crash" | **Loud crash** |
| JSON Schema | No writer/reader schema concept exists. `enum` is a closed set constraint; a validator rejects the new value. | **Loud rejection** |

Sources for the above, all accessed 2026-08-09: https://protobuf.dev/programming-guides/enum/;
https://avro.apache.org/docs/1.12.0/specification/; https://github.com/microsoft/thrifty/issues/84;
https://json-schema.org/understanding-json-schema/reference/enum.

**The ProtoJSON gap deserves separate emphasis, because it is the one that catches people.**
ProtoJSON serialises enums by *name*
(https://protobuf.dev/programming-guides/json/ — accessed 2026-08-09):

> "The name of the enum value as specified in proto is used."
> "Parsers accept both enum names and integer values."

but the specification does not say what a parser does with a name it does not recognise.
Protobuf's own tracker documents this as a known, acknowledged specification gap —
https://github.com/protocolbuffers/protobuf/issues/7392 (status: closed, label
`documentation`) — accessed 2026-08-09:

> "Need specification for how to parse unrecognized enums in JSON"
> "when it sees an unrecognized enum value that has been serialized to a JSON string, it throws
> an exception"
> "Could someone please provide a specification for what a protobuf library should do when it sees
> an unrecognized Enum value serialized in a JSON string?"
> "when this field, which is totally ignored, gets a new enum value, the v12.5 version of the app
> breaks"

So the widely repeated maxim "adding an enum value is safe" is **true for proto3 binary and
false for JSON**, and every implementation requires an explicit opt-in flag
(`protojson.UnmarshalOptions.DiscardUnknown` in Go, `JsonFormat.Parser.ignoringUnknownFields()`
in Java, `ignoreUnknownFields` in protobuf-es) to make it true. **If the transport is JSON,
enum growth is breaking by default.**

## 3.2a ★ Every mainstream JSON deserializer throws by default. Tolerance is opt-in everywhere.

This is the most operationally important finding in the report, and it holds across four
independent language ecosystems plus Twitter's own internal codegen. All accessed 2026-08-09.

| Library | Default on unknown enum value | The opt-in |
|---|---|---|
| **Jackson** (Java) | throws | `READ_UNKNOWN_ENUM_VALUES_AS_NULL` — "Feature is disabled by default." |
| **kotlinx.serialization** | throws | `coerceInputValues` — "`false` by default." |
| **Moshi** (Kotlin/Java) | throws | `EnumJsonAdapter.withUnknownFallback(...)` |
| **Swift `Codable`** | `dataCorrupted` decoding error | hand-written `init(from:)` |
| **Swift enums (language)** | traps at runtime | `@unknown default` |
| **Twitter Scrooge** | `apply` "Throws NoSuchElementException" | `getOrUnknown` |
| **Apollo Kotlin** | **tolerant** — `UNKNOWN__` generated automatically | (opt-*out*) |
| **LinkedIn Rest.li** | **tolerant** — `$UNKNOWN` generated automatically | (opt-*out*) |

Sources: Jackson `DeserializationFeature` javadoc
(https://fasterxml.github.io/jackson-databind/javadoc/2.13/com/fasterxml/jackson/databind/DeserializationFeature.html):

> "Feature that allows unknown Enum values to be parsed as null values. If disabled, unknown Enum
> values will throw exceptions. … **Feature is disabled by default.**"

kotlinx.serialization `coerceInputValues`
(https://kotlinlang.org/api/kotlinx.serialization/kotlinx-serialization-json/kotlinx.serialization.json/-json-builder/coerce-input-values.html):

> "Enables coercing incorrect JSON values in the following cases: … Property type is an enum type,
> but JSON value contains an unknown enum member." … "`false` by default."

Moshi `EnumJsonAdapter` (raw source, byte-verified) — the default path:
```kotlin
if (!useFallbackValue) {
  val name = reader.nextString()
  throw JsonDataException("Expected one of ${nameStrings.toList()} but was $name at path ${reader.path}")
}
```

kotlinx.serialization issue #3071 (open, byte-verified):
> "Currently, kotlinx.serialization fails with an exception when deserializing an enum value that is
> not present in the target enum class. This makes it difficult to handle backward/forward
> compatibility when enums evolve (e.g., when a server adds a new enum constant not yet known by the
> client)."

And issue #1303 documents something worse than a clean error: "if an unknown enum value is
encountered, the deserializer throws an `ArrayIndexOutOfBoundsException` on the JVM instead of a
`SerializationException`."

Swift codegen, the same failure (https://github.com/swagger-api/swagger-codegen/issues/7304):
> "Later on I decide to add another vehicle type in my backend… my app version was released earlier
> will not be able to deserialize Vehicle. It will throw an error even if the type is not
> 'required':" → `dataCorrupted(… "Cannot initialize ModelType from invalid String value Something")`

**Swift took this seriously enough to change the language.** SE-0192, "Handling Future Enum
Cases" (https://raw.githubusercontent.com/swiftlang/swift-evolution/main/proposals/0192-non-exhaustive-enums.md
— byte-verified):

> "Currently, adding a new case to an enum is a source-breaking change, something that's at odds
> with Apple's established process for evolving APIs. This proposal aims to distinguish between enums
> that are _frozen_ (meaning they will never get any new cases) and those that are _non-frozen,_ and
> to ensure that clients handle any future cases when dealing with the latter."

> "It's well-established that many enums need to grow new cases in new versions of a library… This
> all implies that library authors *must* have a way to add new cases to enums without breaking
> binary compatibility."

> "A program will trap at run time if an unknown enum case is actually encountered."

**The conclusion for anyone publishing data that third parties decode:** the *default* behaviour
of the tooling your consumers most likely use is **to fail**. You cannot assume tolerance; the
industry's defaults are against you. The two libraries in the table that are tolerant by default
(Apollo Kotlin, Rest.li) are both cases where the *schema publisher also wrote the client's code
generator*. Where the publisher does not control codegen, the default is a crash.

## 3.3 Does anyone mandate an UNKNOWN fallback? Essentially no.

This was the specific question, and the answer is a clear negative across every corpus
examined.

- **Google does not.** `_UNSPECIFIED = 0` means *unset*, not *unrecognised*. AIP-126 (accessed
  2026-08-09): "The first value of the enum **should** be the name of the enum itself followed by
  the suffix `_UNSPECIFIED`." An `UNKNOWN` sentinel is offered as an explicit *optional exception*:
  "An exception to this rule is if there is a clearly useful zero value." And AIP-216 says the
  zero value "**should not** actually be used." No mandate.
- **GraphQL does not.** The spec constrains the server only — "GraphQL services must return one
  of the defined set of possible values" — and says nothing about a client on an older schema.
- **OpenID Connect explicitly declines to**, for exactly this case: unknown `display`/`prompt`
  values → "it MAY return an error or it MAY ignore it" (§3.1.2.6, accessed 2026-08-09).
- **Kubernetes comes closest, and it is only a SHOULD on documentation**: "document that
  expectation clearly in the API field description in the first release the field is made
  available, and describe how clients should treat an unknown value. Clients should treat such
  sets of values as potentially open-ended."
- **Avro mandates nothing but *provides* the only real mechanism** — the reader-side enum
  `default`. And its constraint is brutal: it must already be in the old reader's schema.
- **Stripe issues the closest thing to a normative client instruction**, and only for event
  types: "Make sure that your webhook listener gracefully handles unfamiliar event types."
  (https://docs.stripe.com/upgrades — accessed 2026-08-09.)

**Where the UNKNOWN fallback actually gets mandated is in codegen, not in specs.** Apollo
Kotlin synthesises an `UNKNOWN__` case into every generated enum precisely so that old clients
survive server-side additions; the community request to *remove* it
(https://github.com/apollographql/apollo-kotlin/issues/6243 — accessed 2026-08-09) exists
because developers find the extra `when` branch annoying. That is the shape of the real-world
solution: **a vendor who controls the client's code generator can impose tolerance that no
specification is willing to require.** Note the precondition — *controls the client's code
generator*.

**Verified from Apollo Kotlin's own code generator (PRIMARY, source code).** URL:
https://raw.githubusercontent.com/apollographql/apollo-kotlin/main/libraries/apollo-compiler/src/main/kotlin/com/apollographql/apollo/compiler/codegen/kotlin/schema/EnumAsEnumBuilder.kt
— accessed 2026-08-09. The generated KDoc strings are:

> "Auto generated constant for unknown enum values"

> "Returns the [%T] that represents the specified [rawValue]. Note: unknown values of [rawValue]
> will return [UNKNOWN__]. You may want to update your schema instead of calling this function
> directly."

`safeValueOf` returns the matching entry, or `UNKNOWN__` when `withUnknown` is true, or throws
when it is false. So the tolerance is real, it is generated automatically into every enum, and
it is opt-out-able — which is exactly the level of control a vendor has over its own client and
does not have over anyone else's.

**NOT FOUND:** a verbatim maintainer statement of the *rationale* for `UNKNOWN__` in a
maintainer's own words. Fetched https://github.com/apollographql/apollo-kotlin/issues/6243
(accessed 2026-08-09); the page carried the feature request to remove it but no maintainer
reply explaining the original design decision. The behaviour is primary-sourced from the
generator above; the stated reasoning is not.

## 3.3a The one published prescription for uncontrolled consumers

Only one organisation in the study wrote down what to do when **you do not know your clients
and cannot coordinate with them**. It is LinkedIn, and it is worth isolating because it is
the exact case at hand.
https://linkedin.github.io/rest.li/modeling/compatibility_check — accessed 2026-08-09:

> "If a new symbols is to be added, a migration strategy for adding the enum symbol(s) must be
> performed just as for any other backward incompatible change. **Note that this is only possible
> when all clients are known and it is possible to coordinate changes with them. If this is not the
> case, one should consider making a backward compatible change (such as adding a new optional
> field containing a new enum field with more symbols) and supporting the existing clients, with
> the existing enum symbols, indefinitely.**"

Unpacked, the prescription is:
1. **Do not grow the existing enum.** Ever, once uncontrolled readers exist.
2. **Add a new optional field** carrying the wider vocabulary alongside the old one.
3. **Keep populating the old field with the old vocabulary indefinitely**, choosing the least-bad
   legacy value for records whose true value is new.
4. Old readers keep reading the old field and never see a symbol they don't know. New readers
   read the new field.

The cost is permanent duplication and a lossy projection into the legacy vocabulary. The
benefit is that it is the only strategy on this list that does not require the reader to have
done anything in advance. **Everything else in §3.4 requires foresight the old artifact may not
have had; this requires foresight only from the publisher.**

LinkedIn also states, in the same document, that this gets strictly worse once the data is
persisted rather than transported:

> "While unknown symbols can be deserialized by older Rest.li consumers (because rest.li does not
> require the schema to de-serialize), it doesn't work for data persisted as Avro. Any attempt to
> deserialize an avro record containing the new enumeration value with an older schema lacking that
> enum will fail."

## 3.4 The three real mitigations, ranked by whether they work

1. **Don't use a closed vocabulary at all.** This is Google's actual load-bearing advice, and
   it is the only one that cannot fail. AIP-126 (accessed 2026-08-09): "enums **should** receive
   new values infrequently … a good rule of thumb is no more than once a year. For enums that
   change frequently, the API **should** use a string and document the format." Iceberg, designed
   for evolvable data at rest, has no enum type in its model at all.
2. **Make the decoded representation open before you need it** — proto3 open enums, Avro's enum
   `default`, OpenAPI's `x-extensible-enum`, Apollo's `UNKNOWN__`. Every one of these is
   **retroactive-defence-only**: it protects a reader only if that reader already had it. You
   cannot retrofit tolerance onto artifacts already in the wild. This is the single most
   important operational fact in this report.
3. **Document that the set is open and hope.** This is what Google and Kubernetes both fall back
   on. It is not a mechanism; it is a request. It works to the extent your consumers read your
   docs, which for anonymous third-party tooling is approximately not at all.

## 3.5 The generalisation

The unknown-*field* problem is solved everywhere and legislated in several places
(Thrift's self-delimiting skip, protobuf's unknown-field set, OIDC's "MUST be ignored", RFC
6709's MBZ warning). The unknown-*enum-value* problem is solved nowhere and legislated
nowhere.

The reason is structural, and Allen George of Apache Thrift stated it exactly
(https://www.mail-archive.com/dev@thrift.apache.org/msg50731.html — accessed 2026-08-09):

> "to support forward-compatibility, you have to have the ability to create enum variants
> without a named value and encode them onto the wire."

An unknown field can be skipped because it is *addressed* — it has an identifier the reader
can step over without understanding. An unknown enum value cannot be skipped, because it is
not a container; it is the value itself, and the reader must *hold* it in a typed slot that,
by construction, enumerates only known values. Every fix amounts to widening that slot in
advance. There is no way to widen it after the fact.

---

# §4 WHAT TRANSFERS TO DATA AT REST, AND WHAT DOES NOT

## 4.1 The four properties the mobile playbook silently assumes

Before the table, name the assumptions, because every verdict below follows mechanically from
which ones the target case violates. The case in question is: **JSON files published into a
git repository, read later by arbitrary third-party tools that cannot be version-negotiated
with, cannot be force-upgraded, and cannot be observed.**

| # | Assumption | Holds for mobile? | Holds for JSON-in-git? |
|---|---|---|---|
| A1 | **The reader asks.** There is a request in which the consumer states what it wants. | Yes | **No** — the file is written once, read later, unilaterally |
| A2 | **The publisher can see consumers.** Traffic, versions, field usage are measurable. | Yes | **No** |
| A3 | **The publisher can reach consumers.** Force-upgrade, kill-switch, minimum-version gate. | Partly | **No** |
| A4 | **The artifact is transient.** Yesterday's response is gone; only the current shape matters. | Yes | **No** — the artifact is the deliverable and it persists |

A1 is the one people miss, and it is the load-bearing one for GraphQL specifically. Meta's
"nearly 1,000 shipped application versions" works because **each old client sends its old query
and gets exactly the shape it named.** A file cannot do that. A file is a response with no
request. Every additive change lands in the reader's lap whether it wants it or not.

A4 is the one that inverts the direction of compatibility. API guidance is overwhelmingly about
**backward** compatibility — new server, old client, and the server is the thing that changes.
Data at rest needs **forward** compatibility — old reader, new data — which is the direction
Confluent explicitly warns is unassured, and whose only published remedy is to hide the new
data from old readers:

> "`FORWARD` or `FORWARD_TRANSITIVE`: there is no assurance that consumers using the new schema
> can read data produced using older schemas. Therefore, first upgrade all producers to using the
> new schema and **make sure the data already produced using the older schemas are not available
> to consumers**, then upgrade the consumers."
> — https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
> — accessed 2026-08-09

In a git repository you cannot make old data unavailable. History is the product.

## 4.2 The verdict table

Verdicts are deliberately harsh. A practice **transfers** only if it works with no live server,
no telemetry, and no ability to contact the consumer.

| Practice (and who publishes it) | Transfers? | Reasoning |
|---|---|---|
| **Never remove or rename a field; additive-only growth** — Google AIP-180 "must not be removed"; Thrift §5.3; Kubernetes Rule #1 | **YES — fully** | Pure property of the artifact. Needs no server, no negotiation, no observation. This is the one practice that is *more* important at rest than in an API, because there is no version bump available as an escape hatch. |
| **Never reuse an identifier; tombstone what you delete** — protobuf `reserved 2, 3;`; schema.org `supersededBy`; Kubernetes "constant value … must exist and function until API v1 is removed" | **YES — fully** | Static discipline over the schema's own history. schema.org is the proof case: a vocabulary consumed by uncontrolled third parties worldwide that never deletes. |
| **Self-describing artifacts: the file carries its own schema/version** — Avro object container files; npm `lockfileVersion`; Thrift §5.4 "add a version header into the data it writes to the file" | **YES — fully, and it is the only complete solution** | This is the *substitute* for negotiation. Avro can resolve reader-vs-writer with no server precisely because "the original schema must be provided" with the data. Note that the format most like the target case — `package-lock.json`, a JSON file in a git repo read by tools of many vintages — chose an **explicit version integer**, not versionless evolution. |
| **Tolerant reader: ignore unknown members** — OIDC "MUST be ignored"; RFC 6709 §4.2 MBZ; protobuf unknown-field set | **PARTIAL — you can mandate it, you cannot enforce it** | The *rule* transfers perfectly and costs nothing to state. But you have no way to verify third-party tools obey it, and no way to find out when they don't (A2 fails). Publish it as a normative requirement in the format spec, then design as if half your readers ignore the requirement. |
| **Explicit version field / pinning at write time** — Stripe account pinning; npm `lockfileVersion`; Kubernetes API groups | **PARTIAL → mostly YES** | Stripe's *mechanism* (pin the account, serve old shapes forever) needs a server. But its *idea* — the artifact records which contract it was written against — transfers completely and is exactly what `lockfileVersion` does. The part that does not transfer is Stripe's ability to keep a translation layer running; at rest you must instead keep the old shape *readable*, which is cheaper. |
| **Deprecate-but-keep-serving** — GraphQL `@deprecated`; Meta "deprecated but continue to function" | **PARTIAL** | Marking a field deprecated in a published schema transfers. What does not transfer is the *lifecycle*: `@deprecated` carries a reason and **no duration**, and at rest there is no moment at which you learn it is safe to stop. In practice "deprecate" at rest collapses into "keep forever", i.e. into row 1. |
| **Versionless / additive-only-forever, no version number** — graphql.org "GraphQL avoids versioning by design"; Meta "removes the need for an incrementing version number" | **NO** | Depends entirely on A1. GraphQL is versionless *because the client names its fields*; the server never sends an unrequested shape. A file has no query. Strip A1 away and "versionless" degrades to "unversioned", which is strictly worse than versioned — the reader cannot even tell which contract it is holding. **The single most commonly mis-transferred practice in this whole corpus.** |
| **Force-upgrade / minimum-version gate / kill-switch** | **NO** | A3 fails absolutely. Worth noting it barely holds *for mobile either*: Google's strongest mechanism is declinable, and Google's own documented fallback for an app that cannot function without the update is to *"prompt the user to close the app"* (https://developer.android.com/guide/playcore/in-app-updates/kotlin-java — accessed 2026-08-09). |
| **Traffic-driven safety checks** — Apollo GraphOS operations checks | **NO** | A2 fails absolutely. Apollo's rules are literally phrased "used by at least one operation", and with no metrics "all potentially dangerous schema changes result in a failed check". With zero observability the tool has no signal and degrades to refusing everything. |
| **Time-boxed deprecation windows** — Meta 2 years; GitHub 3 months + quarterly; Google 12 months; Kubernetes 9 months / 3 releases | **NO (as written); YES only as a promise you can never collect on** | Every published window measures *time since announcement*, and assumes the consumer sees announcements (A3) and that you learn when the last old client leaves (A2). Neither holds. A file written in 2026 may first be read in 2031 by a tool pinned in 2027. **Kubernetes is the one corpus that draws the right distinction**: you may stop *serving* an old version, but "the API server must remain capable of decoding/converting previously persisted data from storage." Adopt that split; discard the clocks. |
| **Stop serving an old shape once usage hits zero** | **NO** | Requires A2. There is no observable zero. |
| **Server-side translation / conversion layer** — Kubernetes round-trip conversion; Stripe version-shim | **NO for the publisher; YES only if relocated into the reader** | Requires a live server that sees the request. At rest the only equivalent is shipping a migration tool and hoping consumers run it — which is A3 again. |
| **Compatibility gating in CI (static)** — `buf breaking`, `oasdiff`, `graphql-inspector`, Avro `SchemaCompatibility`, Confluent `test-local-compatibility` | **YES — fully** | The one piece of enforcement that survives intact. All of these are pure schema-vs-schema diffs needing no server and no telemetry. Confluent even added a local goal specifically to remove the server dependency. **If exactly one practice is adopted from this report, adopt this one** — it is the only mechanism that catches the mistake while it is still cheap. |
| **UNKNOWN/open-enum fallback in generated clients** — Apollo `UNKNOWN__`; proto3 open enums; `x-extensible-enum` | **NO — you cannot impose it on third-party readers** | This works for Apollo because Apollo *writes the client's decoder*. You do not write your consumers' decoders. What **does** transfer is the negative: knowing this, do not create closed vocabularies you intend to grow (§4.3). |
| **Enum-growth budget: "no more than once a year", else use a documented string** — AIP-126 | **YES — fully, and it is the most valuable single line in the corpus** | Pure design-time discipline. No server, no telemetry, no consumer contact. Google's own load-bearing mitigation turns out to be the one that survives translation completely intact. |
| **Parallel-field enum growth: freeze the old enum, add a new optional field with the wider vocabulary, populate both forever** — LinkedIn Rest.li | **YES — fully. The single most directly applicable practice found.** | Explicitly prescribed by LinkedIn *for the case where clients are unknown and uncoordinated*. Requires nothing of the reader and nothing of a server. Costs permanent duplication and a lossy legacy projection. This is the answer to "what do I actually do when I must add a value." |
| **Usage-gated deprecation: remove only once telemetry shows zero use** — Netflix | **NO** | The correct design when clients cannot be force-upgraded, and completely unavailable at rest: A2 fails. Netflix can do this *because* it sees every field access. A published file is read invisibly. |
| **Server-driven UI: move the decision to the server so the client needs no schema knowledge** — Airbnb GP/SDUI | **NO** | The most extreme form of "let the server absorb everything", and it inverts to nothing at rest — there is no server in the loop at read time. Note that even Airbnb qualifies it: features launch to old releases "assuming the response is supported", and their testing "still doesn't catch everything." |

## 4.3 The blunt summary

Of seventeen practices, **six transfer fully** (additive-only; tombstoning; self-describing
artifacts; static CI gating; the enum budget; LinkedIn's parallel-field enum growth), **three
transfer partially** (tolerant-reader as an unenforceable norm; explicit version stamping;
deprecation marking without a clock), and **eight do not transfer at all** (versionless
evolution, force-upgrade, traffic-driven checks, timed deprecation windows, usage-gated
removal, server-side translation, generated UNKNOWN fallbacks, server-driven UI).

**A useful sorting rule falls out of the table:** a practice transfers if and only if it is a
property of *the artifact or of the publisher's own discipline*. Every practice that is a
property of *the relationship* between publisher and consumer — negotiation, observation,
coercion, translation — dies on contact with a file in a git repository.

## 4.4 How to keep the format EVOLVABLE — the constructive answer

§4.2 reads as a list of prohibitions. It should not be mistaken for "freeze the format forever."
None of the organisations studied froze anything: Meta ships continuously, Badoo ships nine
releases a week, Kubernetes changes its API every quarter. **What they bought was not stasis, it
was the ability to change without asking permission from consumers they cannot reach.** The
research supports a specific, buildable design for that, assembled from the practices that
survived §4.2. Every element below is primary-sourced above.

### 4.4.1 Stamp every artifact — this is the enabling move, not a concession
npm's `lockfileVersion` is the closest published analogue to the case at hand, and it is an
explicit integer at the top of a JSON file in a git repository. **Discord supplies the negative
proof**: its unversioned default route has been frozen on a *deprecated* version for years,
because there is no safe way to move a default that consumers never named. A format without a
version stamp cannot evolve; it can only accrete.

The stamp is what converts every later decision from "will this break someone?" into "which
readers does this affect?" — a question you can answer statically. Corollaries from the corpus:
- Make the stamp **mandatory**, not defaulted. Discord's frozen default is the cost of optional.
- Consider carrying the **schema itself**, not just a number, as Avro object container files do
  ("they just include the schema once at the beginning of the file"). That is the only technique
  found that fully removes the need to negotiate.

### 4.4.2 Separate "stop writing" from "stop reading" — Kubernetes' distinction
This is the single most useful governance idea in the study, and it dissolves most of the
apparent tension:

> "no API versions that have been persisted to storage may be removed. Serving REST endpoints for
> those versions may be disabled…, but the API server must remain capable of decoding/converting
> previously persisted data from storage."
> — https://kubernetes.io/docs/reference/using-api/deprecation-policy/ — accessed 2026-08-09

Applied here: **you may stop emitting an old shape whenever you like. You may never stop being
able to read one.** That splits a single frightening decision into one cheap decision (change
what you write, today) and one permanent but small obligation (keep the reader for the old
shape). Reader code is cheap; it is a pure function with no runtime cost when unused, and it can
be covered by golden fixtures forever.

### 4.4.3 Round-tripping as the real invariant
Kubernetes Rule #2 is what makes aggressive change safe:

> "API objects must be able to round-trip between API versions in a given release without
> information loss"

If old→new→old is lossless, you can restructure freely, because any consumer's view can be
reconstructed. Iceberg states the same guarantee for files and gets it from **stable field IDs**
rather than names — "Iceberg guarantees that schema evolution changes are independent and free
of side-effects, without rewriting files." Renaming becomes free once identity is an ID rather
than a name. That is worth designing in early; it cannot be retrofitted.

### 4.4.4 Grow closed vocabularies by widening, never by adding in place
This is the one place where the research is genuinely restrictive, and LinkedIn published the
workaround for precisely the uncontrolled-consumer case:

> "one should consider making a backward compatible change (such as adding a new optional field
> containing a new enum field with more symbols) and supporting the existing clients, with the
> existing enum symbols, indefinitely."

So the enum is not frozen — the *old field* is. New vocabulary lands in a new field; the legacy
field keeps carrying a best-effort legacy value. Cheaper still, and endorsed by Google:

> "For enums that change frequently, the API **should** use a string and document the format."
> "enums **should** document whether the enum is frozen or they expect to add values in the future."

**Declaring openness up front is free and permanently valuable.** Kubernetes says the same:
"document that expectation clearly in the API field description in the first release the field is
made available." A reader that was told on day one "this set is open" has no excuse; a reader
that was told nothing will write an exhaustive switch.

### 4.4.5 Designate extension points explicitly — the RFC 6709 lesson
RFC 6709 and RFC 9413 disagree about tolerance in general but reconcile on granularity: TLS
"ignore unknown record types but… reject unknown handshake messages." Mark, in the spec, which
parts of the document are open for growth and which are closed. Readers can then be tolerant
exactly where tolerance is correct and strict everywhere else — which is also the only way to get
RFC 9413's benefit ("Tolerating unexpected input instead conceals problems") without paying its
cost.

### 4.4.6 Gate it in CI — the enforcement that survives
`buf breaking`, `oasdiff`, `graphql-inspector` and Avro's `SchemaCompatibility` are all pure
schema-vs-schema diffs requiring no server and no telemetry. **This is what buys the freedom to
move fast.** Note the pattern from §1.9–1.11: LinkedIn and Uber are the two organisations that
let themselves stay versionless internally, and they are the two with a hard CI gate. The gate is
not a brake on evolution; it is the thing that makes confident evolution possible.

Set the level deliberately, as LinkedIn does, and keep the escape hatch honest:

> "You are always free to ignore backwards-incompatible change messages if you know that the change
> will not cause problems, or are willing to take steps to ensure that it will not."

### 4.4.7 When you must make a genuinely breaking change
The corpus converges on: **publish both shapes for a window, then stop writing the old one.**
That is Stripe's pinning, Kubernetes' dual-version serving and Twitter's blackout tests, all
reduced to what a file publisher can actually do:
1. Bump the stamp and emit the new shape alongside the old (dual-write) — the at-rest form of
   "serve both versions".
2. Announce it in the artifact itself, since you cannot reach consumers any other way. Nobody in
   this study had to solve announcement-without-a-channel; it is the one place where a file
   publisher is strictly worse off than any API vendor, and the mitigation is to make the data
   self-announcing (a `deprecated`/`supersededBy` marker travelling in the file, as schema.org
   does with terms).
3. Ship a converter as a first-class artifact, and keep it forever. This is the substitute for
   the server-side translation layer that does not transfer.
4. Stop emitting the old shape. Keep reading it indefinitely (§4.4.2).

**The honest limit, stated plainly:** at step 2 you have no way to know whether anyone is still
reading the old shape. Netflix removes a field "once the stats show that a deprecated field is no
longer used"; you will never have those stats. So the transition window is chosen by judgement,
not by evidence — and the reader-side obligation from §4.4.2 is what makes choosing wrong
survivable rather than fatal.

**The suspicion in the brief is correct, and sharper than stated.** Most of the mobile playbook
depends not merely on the server knowing its clients, but on the *client having asked a
question*. GraphQL — the single most-cited success story for versionless evolution, with Meta's
own "three years of released Facebook applications" and "nearly 1,000 shipped application
versions" as evidence — achieves it through a mechanism a file fundamentally cannot have. Meta
proves the opposite point too, and proves it against itself: **the same company that runs a
versionless internal API runs an explicitly versioned external one with a hard two-year clock**,
and the only difference is whether it controls the consumer. When Meta faces third-party
consumers it does not control, Meta versions.

The practices that do survive are not the famous ones. They are the boring static disciplines —
never remove, never reuse, stamp the artifact, gate the diff in CI, and do not build closed
vocabularies you intend to grow. Notably, every format actually designed for data at rest
converged on these independently: Avro embeds the writer's schema, Iceberg uses field IDs and
has no enum type at all, npm puts an integer version at the top of the file, and schema.org
never deletes a term.

---

# §5 RE-FETCH LIST

**Every URL below was accessed 2026-08-09.** Labels: **P** = primary (the organisation's own
docs, blog, spec, or source code) · **S** = secondary (third-party blog or summary; weak) ·
★ = load-bearing for a headline claim · ⚠ = reliability caveat, re-verify before publishing.

## 5.1 Meta / Facebook · Thrift
| # | URL | Label |
|---|---|---|
| 1 | https://engineering.fb.com/2015/09/14/core-infra/graphql-a-data-query-language/ | **P ★** (double-fetched) |
| 2 | https://thrift.apache.org/static/files/thrift-20070401.pdf | **P ★** (PDF; text extracted locally with pypdf) |
| 3 | https://developers.facebook.com/docs/graph-api/guides/versioning/ | **P ★** (double-fetched) |
| 4 | https://developers.facebook.com/docs/graph-api/changelog/breaking-changes/ | **P** — index only, no definitions |
| 5 | https://github.com/microsoft/thrifty/issues/84 | **P** (issue tracker) |
| 6 | https://www.mail-archive.com/dev@thrift.apache.org/msg50731.html | **P** (THRIFT-5392) |
| 7 | https://raw.githubusercontent.com/reactiflux/q-and-a/master/lee-byron_facebook-graphql.md | **P** — contains nothing on versioning |
| 8 | https://github.com/facebook/facebook-android-sdk/pull/544 | **P ★** (real crash + stack trace) |

## 5.2 GraphQL as a movement
| # | URL | Label |
|---|---|---|
| 9 | https://raw.githubusercontent.com/graphql/graphql.github.io/source/src/pages/faq/best-practices.mdx | **P** — graphql.org itself 403s |
| 10 | https://raw.githubusercontent.com/graphql/graphql.github.io/source/src/pages/learn/schema-design.mdx | **P** |
| 11 | http://chentsulin.github.io/graphql.github.io/learn/best-practices/ | **S** — mirror, used only to corroborate #10 |
| 12 | https://web.archive.org/web/20230601000000/https://graphql.org/learn/best-practices/ | **P ★** (archived; live page dropped the Versioning section) |
| 13 | https://graphql.org/learn/governance-versioning/ | **P** |
| 14 | https://raw.githubusercontent.com/graphql/graphql-spec/main/spec/Section%203%20--%20Type%20System.md | **P ★** (`@deprecated`, enum coercion) |
| 15 | https://raw.githubusercontent.com/graphql/graphql-js/16.x.x/src/utilities/findBreakingChanges.ts | **P ★** (`DangerousChangeType`) |
| 16 | https://raw.githubusercontent.com/graphql-hive/graphql-inspector/master/packages/core/src/diff/changes/enum.ts | **P ★** ("programming defensively") |
| 17 | https://docs.github.com/en/graphql/overview/breaking-changes | **P ★** |
| 18 | https://github.com/graphql/graphql-spec/issues/175 · /issues/134 | **P** — #134's comment thread did not render |
| 19 | https://charpeni.com/blog/graphql-enums-are-unsafe | **S** |
| 20 | https://productionreadygraphql.com/blog/2019-11-06-how-should-we-version-graphql-apis/ | **S ⚠** — fetch truncated; **no claim rests on it** |
| 21 | https://raw.githubusercontent.com/apollographql/apollo-kotlin/main/libraries/apollo-compiler/src/main/kotlin/com/apollographql/apollo/compiler/codegen/kotlin/schema/EnumAsEnumBuilder.kt | **P ★** (`UNKNOWN__`) |
| 22 | https://github.com/apollographql/apollo-kotlin/issues/6243 | **P** — no maintainer rationale present |

## 5.3 Google
| # | URL | Label |
|---|---|---|
| 23 | https://google.aip.dev/180 (raw: `github.com/aip-dev/google.aip.dev/…/0180.md`) | **P ★** |
| 24 | https://google.aip.dev/181 · /185 | **P ★** |
| 25 | https://google.aip.dev/126 | **P ★** (enum budget) — retrieved in full |
| 26 | https://google.aip.dev/216 | **P ★** (the candid enum paragraph) |
| 27 | https://protobuf.dev/programming-guides/enum/ | **P ★** (open vs closed) |
| 28 | https://protobuf.dev/programming-guides/proto3/ · /proto2/ | **P ★** |
| 29 | https://protobuf.dev/best-practices/dos-donts/ | **P ★** (note: `/programming-guides/dos-donts/` redirects here) |
| 30 | https://protobuf.dev/programming-guides/json/ | **P** (ProtoJSON) |
| 31 | https://github.com/protocolbuffers/protobuf/issues/7392 | **P ★** (JSON unknown-enum spec gap) |
| 32 | https://github.com/protocolbuffers/protobuf/issues/16857 | **P ★** (Google broke its own client) |
| 33 | https://cloud.google.com/terms/ §1.4(e) | **P ★** (double-fetched; 12 months) |
| 34 | https://cloud.google.com/terms/deprecation | **P** |
| 35 | https://developer.android.com/guide/playcore/in-app-updates · /kotlin-java | **P ★** (Q4) |
| 36 | https://linter.aip.dev/ · https://api.github.com/repos/googleapis/api-linter/contents/rules | **P ★** (proves `aip0180` absent) |
| 37 | https://github.com/googleapis/proto-breaking-change-detector | **P** ("not an officially supported Google project") |
| — | **Redirects, do not cite separately:** `cloud.google.com/apis/design/compatibility` → 301 → aip.dev/180; `…/design/versioning` → 301 → aip.dev/185 | |

## 5.4 Twitter / X
**Fetch note:** `blog.x.com` / `blog.twitter.com` → **HTTP 403** (Cloudflare JS challenge) to
WebFetch *and* curl-with-browser-UA. `developer.x.com` → **HTTP 402**. `web.archive.org` is
blocked to WebFetch but reachable via curl. Use Wayback raw (`…id_/`) snapshots.

| # | URL | Label |
|---|---|---|
| 38 | `https://web.archive.org/web/20200924190056id_/https://developer.twitter.com/en/docs/twitter-api/versioning` | **P ★** (v2 policy + durations) |
| 39 | `https://web.archive.org/web/20240416135831id_/https://blog.twitter.com/developer/en_us/a/2012/changes-coming-to-twitter-api` | **P ★** (six months) |
| 40 | `https://web.archive.org/web/20231205015116id_/…/current-status-api-v1-1` | **P** |
| 41 | `https://web.archive.org/web/20240226063445id_/…/api-v1-retirement-final-dates` | **P ★** (`HTTP 410 Gone`) |
| 42 | `https://web.archive.org/web/20231211182010id_/…/api-v1-is-retired` | **P** |
| 43 | `https://web.archive.org/web/20240303153815id_/…/using-fields-and-expansions` | **P** (the weak rationale) |
| 44 | `https://web.archive.org/web/20220707142143id_/…/migrate/whats-new` · `…/early-access` | **P** |
| 45 | https://docs.x.com/x-api/fundamentals/fields · /expansions · /x-ads-api/fundamentals/versioning | **P** (live) |
| 46 | https://raw.githubusercontent.com/twitter/scrooge/develop/scrooge-core/src/main/scala/com/twitter/scrooge/ThriftEnum.scala | **P ★** (`apply` throws) |
| 47 | https://raw.githubusercontent.com/twitter/scrooge/develop/CHANGELOG.rst | **P** |
| 48 | https://groups.google.com/g/twitter-development-talk/c/ahbvo3VTIYI | **P** (Snowflake `id_str`) |
| — | **NOT RETRIEVABLE:** "Introducing a new and improved Twitter API" (July 2020) — Wayback holds only 301s for every URL variant; live 403. **No verbatim text obtained; none paraphrased.** | |

## 5.5 Badoo / Bumble
| # | URL | Label |
|---|---|---|
| 49 | https://raw.githubusercontent.com/badoo/techblog/master/_posts/2016-05-04-crazy-agile-api.markdown | **P ★** (authored source) |
| 50 | https://medium.com/bumble-tech/crazy-agile-api-5130be6f5b06 | **P** (rendered form of #49) |
| 51 | https://www.slideshare.net/BadooDev/versioning-strategy-for-a-complex-internal-api | **P ★★** (the unknown-banner slides) |
| 52 | https://habr.com/ru/company/badoo/blog/305888/ | **P** (Russian original; extra clause) |
| 53 | https://api.github.com/repos/badoo/techblog/contents/_posts?ref=master | **P** (post index) |
| 54 | https://nordicapis.com/continuous-versioning-strategy-for-internal-apis/ | **S ⚠** — its "never had a breaking change" claim is **unverified; do not cite as Badoo's words** |
| — | **Dead:** `tech.badoo.com/*` → 301 → `badoo.com/` (dead); `badootech.badoo.com/*` → 301 → `medium.com/bumble-tech`. The 2014 JSConf EU post source is a 674-byte stub with no prose. | |

## 5.6 LinkedIn · Netflix · Uber · Airbnb
| # | URL | Label |
|---|---|---|
| 55 | https://linkedin.github.io/rest.li/modeling/compatibility_check | **P ★★** (double-fetched; the enum essay) |
| 56 | https://linkedin.github.io/rest.li/setup/gradle · /Rest_li-2_x-upgrade-instructions · /spec/protocol | **P** |
| 57 | `raw.githubusercontent.com/linkedin/rest.li/master/data/src/main/java/com/linkedin/data/schema/compatibility/CompatibilityChecker.java` | **P ★** (the real rules) |
| 58 | `…/compatibility/CompatibilityMessage.java` · `…/idlcheck/CompatibilityLevel.java` · `…/idlcheck/CompatibilityInfo.java` | **P ★** (`WIRE_COMPATIBLE`) ⚠ runtime wiring unverified |
| 59 | `…/data/template/DataTemplateUtil.java` | **P ★** (`$UNKNOWN`) |
| 60 | https://learn.microsoft.com/en-us/linkedin/marketing/versioning | **P ★** (1-year minimum) |
| 61 | https://www.linkedin.com/blog/engineering/marketing/under-the-hood-how-we-built-api-versioning-for-linkedin-market | **P ★** |
| 62 | https://netflixtechblog.com/how-netflix-scales-its-api-with-graphql-federation-part-2-bbe71aaec44a | **P ★** (double-fetched; usage-gated deprecation) |
| 63 | https://netflixtechblog.com/embracing-the-differences-inside-the-netflix-api-redesign-15fd8b3dc49d | **P** (800 device types) |
| 64 | https://netflixtechblog.com/practical-api-design-at-netflix-part-1-using-protobuf-fieldmask-35cfdc606518 · /safe-updates-of-client-applications-at-netflix-1d01c71a930c | **P** |
| 65 | https://raw.githubusercontent.com/Netflix/dgs-framework/master/graphql-dgs-client/src/main/kotlin/com/netflix/graphql/dgs/client/GraphQLResponse.kt | **P ★** |
| 66 | https://help.netflix.com/en/node/112425 · /119807 · /295469825389156 | **P ★** (Q4) |
| 67 | https://www.uber.com/blog/architecture-api-gateway/ | **P ★★** (the CI-gate quote) |
| 68 | https://raw.githubusercontent.com/uber/prototool/dev/style/README.md · /docs/breaking.md | **P ★** ⚠ repo archived 2026-03-04 |
| 69 | https://raw.githubusercontent.com/uber/idl/master/README.md | **P ★** ("one version of the world") |
| 70 | https://raw.githubusercontent.com/uber/zanzibar/master/docs/thrift.md · `thriftrw-go/dev/gen/enum.go` | **P ★** |
| 71 | https://bitrise.io/blog/post/q-and-a-on-building-apps-at-scale-part-1 | **S ⚠** (page states answers were "edited down and summarized") |
| 72 | https://medium.com/airbnb-engineering/a-deep-dive-into-airbnbs-server-driven-ui-system-842244c5f5 | **P ★** |
| 73 | https://github.com/MobileNativeFoundation/discussions/discussions/47 | **P ★** (richest Airbnb source) |
| 74 | https://medium.com/airbnb-engineering/building-services-at-airbnb-part-4-23c95e428064 | **P ★** (their CI gate) |
| 75 | https://www.airbnb.com/help/article/3418 | **P** (6-month partner clause) |
| — | **BLOCKED:** `developer.withairbnb.com/docs/homes/versioning` → 302 → login wall. Highest-value unresolved target. | |
| — | **Fetch techniques:** `netflixtechblog.com` 307s twice through `medium.com/m/global-identity-2` — follow both hops. Medium mangles under WebFetch; prefix `https://r.jina.ai/` for clean markdown + exact-phrase checks. | |

## 5.7 Compatibility-checking tools and formats
| # | URL | Label |
|---|---|---|
| 76 | https://buf.build/docs/breaking/ · /rules/ · /usage/ · /bsr/checks/breaking/ | **P ★** (note `/breaking/overview` redirects) |
| 77 | https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html | **P ⚠ RE-FETCH BY HAND** — per-format tables were **inconsistent across two fetches**; definitions + upgrade-ordering block were stable |
| 78 | https://docs.confluent.io/cloud/current/sr/fundamentals/schema-evolution.html | **P** — the word "enum" does not appear |
| 79 | https://docs.confluent.io/platform/current/schema-registry/develop/maven-plugin.html | **P ★** (local vs server goals) |
| 80 | https://github.com/confluentinc/schema-registry/issues/601 | **P ★** (Confluent engineer: enum addition *is* forward-incompatible) |
| 81 | https://avro.apache.org/docs/1.12.0/specification/ (identical at 1.11.1) | **P ★★** (enum `default`, "an error is signalled", object container files) |
| 82 | `raw.githubusercontent.com/apache/avro/main/lang/java/avro/src/main/java/org/apache/avro/SchemaCompatibility.java` | **P ★** (`MISSING_ENUM_SYMBOLS`) |
| 83 | https://issues.apache.org/jira/browse/AVRO-1340 | **P** (ASF project record) |
| 84 | https://json-schema.org/understanding-json-schema/reference/enum · /blog/posts/future-of-json-schema | **P** — **no evolution model exists** |
| 85 | https://www.apollographql.com/docs/graphos/platform/schema-management/checks · /checks/run · /checks/reference | **P ★★** ("no operation metrics… result in a failed check") |
| 86 | https://the-guild.dev/graphql/inspector/docs/commands/diff · /essentials/diff | **P** |
| 87 | https://github.com/oasdiff/oasdiff · https://www.oasdiff.com/docs/breaking-changes | **P ★** (per-position enum checks) |
| 88 | https://raw.githubusercontent.com/apache/iceberg/main/docs/docs/evolution.md | **P ★** (metadata-only evolution) |
| 89 | Jackson `DeserializationFeature` javadoc · kotlinx `coerceInputValues` · Moshi `EnumJsonAdapter.kt` · kotlinx issues #3071, #1303 | **P ★** (defaults throw) |
| 90 | `raw.githubusercontent.com/swiftlang/swift-evolution/main/proposals/0192-non-exhaustive-enums.md` | **P ★** |
| 91 | https://github.com/joelittlejohn/jsonschema2pojo/issues/728 · https://github.com/swagger-api/swagger-codegen/issues/7304 | **P ★** |

## 5.8 Governance corpora, standards, and data-at-rest precedents
| # | URL | Label |
|---|---|---|
| 92 | `raw.githubusercontent.com/kubernetes/community/main/contributors/devel/sig-architecture/api_changes.md` (also `master`) | **P ★★** (double-fetched; the enum ruling) |
| 93 | https://kubernetes.io/docs/reference/using-api/deprecation-policy/ | **P ★★** (double-fetched; Rules #1/#2/#4a, the storage clause) |
| 94 | https://stripe.com/blog/api-versioning · https://docs.stripe.com/upgrades | **P ★** (pinning; the compatible-changes list) |
| 95 | https://www.rfc-editor.org/rfc/rfc9413.txt (and .html) | **P ★** (abstract + §5.1 verified verbatim) |
| 96 | https://www.rfc-editor.org/rfc/rfc6709.txt (and .html) | **P ★** (MBZ, §4.7, §A.3 TLS) |
| 97 | https://openid.net/specs/openid-connect-core-1_0.html | **P ★** (MUST-ignore vs MAY-error asymmetry) |
| 98 | https://docs.npmjs.com/cli/v10/configuring-npm/package-lock-json | **P ★★** (`lockfileVersion` — closest analogue) |
| 99 | https://schema.org/docs/howwework.html | **P ★★** (double-fetched; never delete an enumerated value) |
| 100 | https://martin.kleppmann.com/2012/12/05/schema-evolution-in-avro-protocol-buffers-thrift.html | **S** (strong — expert-authored, not an org's position) |
| 101 | https://docs.discord.com/developers/reference | **P ★★** (default frozen on a deprecated version) |
| 102 | https://slack.com/help/articles/1500001836081-Slacks-deprecation-schedule · https://support.signal.org/hc/en-us/articles/5109141421850-Supporting-Older-Operating-Systems | **P ★** (Q4 as routine) |

## 5.9 Post-mortems
| # | URL | Label |
|---|---|---|
| 103 | https://blog.cloudflare.com/18-november-2025-outage/ | **P ★★** (closest analogue to data-at-rest) |
| 104 | https://status.cloud.google.com/incidents/ow5i3PPK96RduMcb1SsW | **P ★★** |
| 105 | https://blog.cloudflare.com/details-of-the-cloudflare-outage-on-july-2-2019/ | **P ★** (config bypasses staged rollout) |
| 106 | https://www.fastly.com/blog/summary-of-june-8-outage | **P ★** ("valid" config) |
| 107 | https://web.archive.org/web/20230129150309/https://api.slack.com/changelog/2021-02-24-how-we-broke-your-slack-app | **P ★** (live page is an SPA shell) |
| 108 | https://about.roblox.com/newsroom/2022/01/roblox-return-to-service-10-28-10-31-2021 | **P** |
| 109 | https://steamcommunity.com/discussions/forum/14/2974028351344359625/ | **P ★** (adoption-gated protocol change) |
| 110 | https://github.com/bitnami/charts/issues/7264 · https://github.com/Azure/AKS/issues/1205 · https://cert-manager.io/docs/releases/upgrading/ingress-class-compatibility/ | **P** (K8s removal breaking deployed controllers) |
| 111 | https://livefront.com/writing/dont-live-with-regret-build-a-kill-switch-into-your-mobile-app/ | **S** (consultancy, not an operator) |

## 5.10 Explicit NOT-FOUND register
Recorded so the gaps are not silently re-filled later:

1. **Meta Q3/Q4 for the Graph API** — no definition of a breaking change, no enum guidance, no
   force-upgrade statement. Searched: the versioning guide, the breaking-changes changelog index,
   engineering.fb.com, the Lee Byron reactiflux transcript.
2. **A Lee Byron (or spec-editor) verbatim defence of no-versioning** — spec issues #175/#134 link
   to it without reproducing it; the comment thread did not render.
3. **Twitter on enum growth** — absent from *both* the breaking and non-breaking lists.
4. **Twitter's stated rationale for fields/expansions as an evolution mechanism** — only
   "simplicity"/"use case". The v2 announcement post is unretrievable.
5. **Badoo on unknown-enum *recovery*** (as opposed to prevention), any deprecation duration, or
   any automated CI gate.
6. **Netflix on enum growth**, and any Netflix *engineering* statement that devices cannot be
   updated (only consumer-facing help pages show eviction).
7. **Uber Q4 and Q5** — no primary statement on forced upgrade or deprecation duration.
8. **Airbnb Q3, Q4, Q5 (mobile)** — "unknown" and "deprecat" are absent from the SDUI article
   entirely. Third-party claims that Airbnb "returns a fallback component" have **no Airbnb
   source; do not repeat them.**
9. **Confluent on enum add/remove per format** — the word "enum" does not appear on either
   schema-evolution page; the gap is closed only by issue #601.
10. **Any normative JSON Schema rule on enum evolution** — none exists; JSON Schema has no
    writer-schema/reader-schema concept at all.
11. **Google-published CI gate for AIP-180** — `aip0180` is absent from the linter's rule tree.
12. **Apollo Kotlin's stated rationale for `UNKNOWN__`** — the behaviour is source-verified; the
    reasoning in a maintainer's words is not.
13. **WhatsApp's official supported-versions policy** — the FAQ URL tried returned 404 and only
    secondary tech-news coverage was found; **not pursued further and not used.**
