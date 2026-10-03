# Static XML consumer: normal and simple packages

The static boot compiler resolves the `#use` and `#source` closure of a
`format = "normal"` package before emitting the consumer's `STATIC.xml`. A
`#use` of a section pulls its top-level anchored ancestor; a `#source` brings
the addressed source document into the contract merge by matching anchors.
The XML consumer test uses an umbrella and two statically required normal
packages to check the cross-package closure and that a missing address
refuses compilation before boot artifacts are replaced. The present fixture
exposes a remaining whole-lane duplication: when a package is both reached
through `#use` and separately required as a static entry, its text appears in
both contributions. In the three-entry fixture the resulting lane contains
the umbrella body once, the first package's body twice, and the second
package's contract and source bodies three times each. The once-only
conformance test is ignored by default under owner question B-224 and remains
reproducible with `--ignored`; its assertion is not a passing result.

`format = "simple"` has different semantics: static compilation carries the
selected snippet as text, so a literal `#use` is not an unresolved-link bug.
`#embed` is expanded during materialization and static compilation. These
behaviors are separately pinned by `render_static_normal_differs_from_simple_on_the_same_file`
and `render_static_expands_an_embed_directive` in the workspace tests.

An authored project boot file named by a `kind = "static"` entry in generated
`INDEX.md` is read directly at session start. It is an authored file alongside
the generated artifacts, not a normal package snippet passed through the
static compiler. `STATIC.xml` itself is generated compiler output and is read
first when present.

`#source` is a document-scoped declaration. The directive scanner recognizes
it at the start of any unmasked line, including after a section heading. The
normal compiler discovers and merges all source directives in their written
order; it does not require the directive to be top-level or the last line of a
file. Fenced and commented text is excluded by the scanner.

These boundaries follow PROP-035 §§3, 7–8 and PROP-009 §2.3. The scanner and
compiler worklist/merge implementation establish the placement rule.
