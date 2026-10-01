# Source declarations for existing Developer Preview 1 projects

This is a source draft for the mutable 1.1.0 `next` line, not the final
Developer Preview 2 migration guide or permission to publish it. The bounded
implementation reviews cover explicit source declarations and selected readers;
the complete documentation, release and native-platform checks remain open.

## Decide whether to declare a source

Existing 1.0.x packages and projects without source declarations remain usable
through their established compatibility paths. Installing 1.1.0 does not add
declarations, convert authored files or require a package-version increment.
Opt in only when you can identify what each source *is*, rather than inferring
its meaning from `.md`, `.xml`, a directory name or parsable contents.

A package declares repeated `[[package.sources]]` rows in its authored
`vibe.toml`. A plain project can opt in with repeated `[[project.sources]]`
rows. Each row selects exactly one package- or project-relative `file`, or a
`directory` default for descendants, and states `media`, `interpretation`,
`role` and `loading`. An exact file wins over matching directory defaults;
otherwise the longest matching directory wins. Incompatible equally specific
rows return a path-specific `NeedsInput` question. A directory row is not a
glob, and it does not by itself prove that every child was observed.

Choose `vibe-spec` or `vibe-doc` only for documents that should carry Vibe
facts or addresses. Ordinary `commonmark` gives structural navigation without
Vibe facts. `xml-data` and `opaque` are resources, including when their names
resemble specification files. `scan` may enumerate any declared source, but
only a selected Vibe interpretation grants its corresponding semantics;
`explicit` does not enroll a resource in automatic scanning. The declaration
records intent, not trust: the reader must still capture bytes and verify its
own source, media and provenance witness.

## Review a recognized old package before applying a change

The finite source-declaration migration recognizes an old `flow` package with
`format = "normal"` and one `[boot_snippet].source`. It asks for the media of
that **exact path** and can plan one TOML-safe declaration while preserving
untouched manifest bytes, comments, spacing and field order. Planning writes
nothing. A `NeedsInput` question or refusal leaves the authored manifest
unchanged; resolve the path-specific meaning yourself before proceeding.
`simple`, fragment and unrecognized old forms have no automatic recipe here.

The existing [explicit manifest migration procedure](migration-application.md)
explains the review, single-file ownership grant, application journal and
inspection/recovery model. Its JSON example is for a **different**
materialization profile: do not copy its profile or recipe values for source
declarations. Apply a source-declaration plan only when its own supported
profile, exact before/after text and identities have been produced and reviewed.
Do not target an upstream package, fixture or historical capture as though it
were your owned authored file.

If you decide against a proposed declaration, stop before application and
leave the manifest unchanged. After an interrupted application, inspect its
retained operation and use only a supported explicit recovery action with the
same authority and operation identity. A completed edit is not automatically
rolled back: keep the original authored bytes and reviewed report, make any
later reversal as a separate owned manifest change, then inspect the resulting
installed state. Removing a descriptor does not itself prove that an existing
materialized slot or generated boot output has been restored.

## What changes after opting in

Only a selected Vibe document enters the Vibe format conversion and address
resolver. Declared ordinary CommonMark, XML data and opaque resources retain
their original bytes in materialization, including invalid or non-UTF-8 XML
data. Optional XML-data structure inspection may fail without erasing the raw
resource. A same-stem CommonMark file and XML-data file can coexist; two Vibe
forms claiming one document address, or two outputs colliding on the target
filesystem, refuse before publication.

The reviewed initial consumers carry this selection through facts adoption and
reporting, requirements queries, boot generation and simple inputs, citations,
`show effective`, `tree`, `check`, and progress scan/mirror/weave. Raw resources
do not acquire Vibe facts, citations or boot directives from their suffix. A
selected source with missing, ambiguous, foreign or changed evidence refuses
instead of silently returning to suffix-based interpretation. Packages and
projects that have not opted in keep their explicit legacy behavior.

An installed descriptor slot may need manual rematerialization from retained
authored sources after a declaration changes; this draft does not claim an
automatic conversion or a general rollback recipe. A transformed source's
original-file digest is not always available from the retained output. A
directory default without a complete host census cannot claim complete fact
coverage. In-place sources without a materialized record have a weaker
live-manifest observation contract, and citation readers without an independent
selected hash may refuse.

## Portability and proof boundary

The implementation has paths for Linux, macOS and Windows. The selected
source/reader tests in the A.1–A.4 reviews ran on GNU/Linux; native macOS and
Windows runtime behavior, including filename aliases, remains to be measured.
Descriptor slot publication uses no-replace operations and recorded identity
checks. Its portable guarantee coordinates cooperating writers and reports
detected collision or drift; it is not unconditional content-CAS against a
process that ignores the protocol. This weaker contract does not prevent
ordinary Linux use. It also does not justify accepting an observed collision.

The [source capture evidence](evidence/m06-a-after-424ed24cd-v2-capture-e1.json)
identifies candidate product source. The [current transition index](transition.json)
still selects an earlier bootstrap scope; an M-06-A scoped transition has not
been accepted. Neither file establishes final documentation, all-platform
execution or release acceptance.
