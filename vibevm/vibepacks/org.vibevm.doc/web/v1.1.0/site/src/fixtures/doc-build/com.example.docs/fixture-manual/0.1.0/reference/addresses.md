# Addresses {#root}

@status:impl/done @audience:user

[p01] @fact:ADDRESS-SHAPE A page address ends with a slash and the projections lie beside it as files: `…/<document>/`, `…/<document>.md`, `…/<document>.xml`. @status:impl/done

## The language segment {#language}

[p02] The language is the segment after `/doc/`, and the source language carries no segment at all. A translation is served under the source package's coordinate, which is what lets one address become another by changing one segment.

## island {#island}

[p03] A page's content as finished HTML, rendered by the pipeline and inserted whole: the shell parses nothing of it. See [the language segment](/doc/com.example.docs/fixture-manual/latest/reference/addresses/#language).

## resolver {#resolver}

[p04] The one address that knows what a mount carries, asked with a `spec://` citation and answering with the page it names.

